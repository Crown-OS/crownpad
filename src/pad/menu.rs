use crownui::{
    kit::{context_menu, menu_header, menu_item, menu_item_selected, menu_separator},
    prelude::{AnyView, Cx, Listeners, Point, Styled, View, keyed, vstack, zstack},
};

use super::{actions::Action, state::Pad};
use crate::{
    catalog::DesktopId,
    library::{FolderId, Item},
    settings::{Browsing, SortMode},
    util::non_default_gpu::{self, GpuPreference},
};

const MENU_WIDTH: f32 = 340.0;
const ROW_HEIGHT: f32 = 34.0;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuTarget {
    App {
        id: DesktopId,
        folder: Option<FolderId>,
    },
    Folder(FolderId),
    Selection(Vec<Item>),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub anchor: Point,
    pub target: MenuTarget,
}

enum Row {
    Header(String),
    Separator,
    Action {
        label: String,
        action: Action,
        selected: bool,
    },
}

pub fn open_for_item(cx: &mut Cx, pad: Pad, item: &Item, anchor: Point) {
    let selected = pad.selection.with(cx, |selection| {
        selection.contains(item) && selection.len() > 1
    });
    let target = if selected {
        MenuTarget::Selection(
            pad.selection
                .with(cx, |selection| selection.iter().cloned().collect()),
        )
    } else {
        match item {
            Item::App(id) => MenuTarget::App {
                id: id.clone(),
                folder: pad.folder.get(cx),
            },
            Item::Folder(folder) => MenuTarget::Folder(*folder),
        }
    };
    pad.menu.set(cx, Some(Menu { anchor, target }));
}

pub fn open_empty(cx: &mut Cx, pad: Pad, anchor: Point) {
    pad.menu.set(
        cx,
        Some(Menu {
            anchor,
            target: MenuTarget::Empty,
        }),
    );
}

pub fn menu_view(pad: Pad) -> impl View {
    keyed(
        move |runtime| pad.menu.get(runtime),
        move |cx, menu| match menu {
            Some(menu) => overlay(cx, pad, menu),
            None => vstack(()).display_none().boxed(),
        },
    )
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}

fn overlay(cx: &mut Cx, pad: Pad, menu: Menu) -> AnyView {
    let rows = rows(cx, pad, &menu.target);
    let surface = pad.surface.get(cx);
    let height = rows.len() as f32 * ROW_HEIGHT;
    let left = menu.anchor.x.min(surface.width - MENU_WIDTH - 8.0).max(8.0);
    let top = menu.anchor.y.min(surface.height - height - 8.0).max(8.0);
    let views: Vec<AnyView> = rows
        .into_iter()
        .map(|row| match row {
            Row::Header(label) => menu_header(label).boxed(),
            Row::Separator => menu_separator().boxed(),
            Row::Action {
                label,
                action,
                selected,
            } => {
                let item = if selected {
                    menu_item_selected(label)
                } else {
                    menu_item(label)
                };
                item.on_click(move |cx, event| {
                    event.stop_propagation();
                    pad.menu.set(cx, None);
                    super::actions::apply(cx, pad, action.clone());
                })
                .boxed()
            }
        })
        .collect();
    let panel = context_menu(views)
        .absolute()
        .left(left)
        .top(top)
        .max_h(surface.height - 16.0)
        .overflow_scroll()
        .pointer_events_auto()
        .on_click(|_, event| event.stop_propagation())
        .enter(|style| style.scale(0.92).opacity(0.0));
    zstack(panel)
        .absolute()
        .inset(0.0)
        .pointer_events_auto()
        .on_click(move |cx, event| {
            event.stop_propagation();
            pad.menu.set(cx, None);
        })
        .on_context_menu(move |cx, event| {
            event.stop_propagation();
            pad.menu.set(cx, None);
        })
        .boxed()
}

/// Only where opening it normally would not already pick that GPU.
fn offers_non_default_gpu(cx: &mut Cx, pad: Pad, id: &DesktopId) -> bool {
    non_default_gpu::is_known()
        && pad.library.with(cx, |library| {
            library
                .app(id)
                .is_some_and(|app| app.entry().gpu == GpuPreference::Default)
        })
}

fn rows(cx: &mut Cx, pad: Pad, target: &MenuTarget) -> Vec<Row> {
    let action = |label: &str, action: Action| Row::Action {
        label: label.to_owned(),
        action,
        selected: false,
    };
    let settings = pad.settings.get(cx);
    let paged = settings.browsing == Browsing::Paged;
    let pages = pad.page_count.get(cx);
    let mut rows = Vec::new();
    match target {
        MenuTarget::App { id, folder } => {
            rows.push(action("Open", Action::Open(Item::App(id.clone()))));
            if offers_non_default_gpu(cx, pad, id) {
                rows.push(action(
                    "Open with Discrete GPU",
                    Action::OpenOnNonDefaultGpu(id.clone()),
                ));
            }
            rows.push(action("Show in Files", Action::ShowInFiles(id.clone())));
            rows.push(action("Add to Dock", Action::AddToDock(id.clone())));
            rows.push(Row::Separator);
            rows.push(action("Rename", Action::RenameApp(id.clone())));
            if let Some(folder) = folder {
                rows.push(action(
                    "Remove from Folder",
                    Action::RemoveFromFolder(*folder, id.clone()),
                ));
            } else {
                let folders: Vec<(FolderId, String)> = pad.library.with(cx, |library| {
                    let mut folders: Vec<_> = library
                        .layout()
                        .folders
                        .iter()
                        .map(|folder| (folder.id, folder.name.clone()))
                        .collect();
                    folders.sort_by_key(|folder| folder.1.to_lowercase());
                    folders
                });
                if !folders.is_empty() {
                    rows.push(Row::Header("Move to Folder".to_owned()));
                    rows.extend(folders.into_iter().map(|(folder, name)| {
                        action(&name, Action::MoveToFolder(id.clone(), folder))
                    }));
                }
                if paged {
                    move_to_page(&mut rows, Item::App(id.clone()), pages);
                }
            }
            rows.push(Row::Separator);
            rows.push(action("Hide", Action::Hide(vec![id.clone()])));
            rows.push(action("Uninstall…", Action::Uninstall(id.clone())));
        }
        MenuTarget::Folder(folder) => {
            rows.push(action("Open", Action::Open(Item::Folder(*folder))));
            rows.push(action("Rename", Action::RenameFolder(*folder)));
            rows.push(action("Dissolve Folder", Action::Dissolve(*folder)));
            if paged {
                move_to_page(&mut rows, Item::Folder(*folder), pages);
            }
        }
        MenuTarget::Selection(items) => {
            let count = items.len();
            rows.push(action(
                &format!("Open {count} Items"),
                Action::OpenSelection,
            ));
            rows.push(action(
                "New Folder with Selection",
                Action::Group(items.clone()),
            ));
            let apps: Vec<DesktopId> = items.iter().filter_map(Item::app).cloned().collect();
            if !apps.is_empty() {
                rows.push(action(
                    &format!("Hide {} Apps", apps.len()),
                    Action::Hide(apps),
                ));
            }
            rows.push(action("Deselect All", Action::ClearSelection));
        }
        MenuTarget::Empty => {
            rows.push(Row::Header("Sort By".to_owned()));
            for (label, mode) in [
                ("Custom", SortMode::Manual),
                ("Name", SortMode::Name),
                ("Date Added", SortMode::DateAdded),
                ("Last Opened", SortMode::LastOpened),
                ("Most Used", SortMode::Frequency),
            ] {
                rows.push(Row::Action {
                    label: label.to_owned(),
                    action: Action::Sort(mode),
                    selected: settings.sort == mode,
                });
            }
            rows.push(Row::Header("Browse".to_owned()));
            for (label, browsing) in [("Pages", Browsing::Paged), ("Scroll", Browsing::Scroll)] {
                rows.push(Row::Action {
                    label: label.to_owned(),
                    action: Action::Browse(browsing),
                    selected: settings.browsing == browsing,
                });
            }
            rows.push(Row::Header("Reorganize".to_owned()));
            rows.push(action("By Icon Colour", Action::ArrangeByHue));
            rows.push(action("Ungroup All Folders", Action::UngroupAll));
            rows.push(action("Folders First", Action::FoldersFirst));
            rows.push(action("Folders Last", Action::FoldersLast));
            rows.push(action("Reset Layout", Action::Reset));
            let hidden: Vec<(DesktopId, String)> = pad.library.with(cx, |library| {
                library
                    .hidden()
                    .iter()
                    .filter_map(|id| Some((id.clone(), library.app(id)?.name().to_owned())))
                    .collect()
            });
            if !hidden.is_empty() {
                rows.push(Row::Header("Hidden Apps".to_owned()));
                rows.extend(
                    hidden
                        .into_iter()
                        .map(|(id, name)| action(&format!("Show {name}"), Action::Unhide(id))),
                );
            }
            rows.push(Row::Separator);
            rows.push(action("Settings…", Action::Settings));
        }
    }
    rows
}

fn move_to_page(rows: &mut Vec<Row>, item: Item, pages: usize) {
    rows.push(Row::Header("Move to Page".to_owned()));
    rows.extend((0..=pages).map(|page| Row::Action {
        label: if page == pages {
            "New Page".to_owned()
        } else {
            format!("Page {}", page + 1)
        },
        action: Action::MoveToPage(item.clone(), page),
        selected: false,
    }));
}
