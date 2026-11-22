use crownui::prelude::{
    Color, Cx, Listeners, Runtime, Signal, Styled, View, WHITE, bind, component, for_each, show,
    text, text_field, vstack, zstack,
};

use super::{
    activate, backdrop,
    drag::{self, Origin, PAD_MIME},
    state::{Editing, Pad},
    tile::{ICON_CORNER, art_view},
    tiles::Art,
};
use crate::{
    catalog::DesktopId,
    library::{FolderId, Item},
};

const MAX_COLUMNS: usize = 6;
const MAX_ROWS: usize = 4;
const TITLE_SIZE: f32 = 28.0;
const TRAY_RADIUS: f32 = 32.0;
const TRAY_PADDING: f32 = 28.0;
const SCRIM: Color = Color::new(0.0, 0.0, 0.0, 0.18);

#[derive(Debug, Clone, PartialEq)]
struct Member {
    id: DesktopId,
    name: String,
    image: Option<crownui::ext::ImageId>,
}

pub fn open(cx: &mut Cx, pad: Pad, folder: FolderId) {
    pad.folder.set(cx, Some(folder));
    pad.highlight.set(cx, None);
}

pub fn close(cx: &mut Cx, pad: Pad) {
    if pad
        .editing
        .with(cx, |editing| matches!(editing, Some(Editing::Folder(_))))
    {
        commit_rename(cx, pad);
    }
    pad.folder.set(cx, None);
}

pub fn commit_rename(cx: &mut Cx, pad: Pad) {
    let Some(editing) = pad.editing.get(cx) else {
        return;
    };
    let name = pad.edit_text.get(cx);
    pad.editing.set(cx, None);
    pad.edit_library(cx, |library| match editing {
        Editing::Folder(folder) => library.rename_folder(folder, &name),
        Editing::App(id) => library.rename_app(&id, &name),
    });
}

pub fn begin_rename(cx: &mut Cx, pad: Pad, editing: Editing) {
    let current = pad.library.with(cx, |library| match &editing {
        Editing::Folder(folder) => library.folder(*folder).map(|folder| folder.name.clone()),
        Editing::App(id) => library.app(id).map(|app| app.name().to_owned()),
    });
    pad.edit_text.set(cx, current.unwrap_or_default());
    pad.editing.set(cx, Some(editing));
}

pub fn folder_view(pad: Pad) -> impl View {
    show(
        bind(move |runtime| pad.folder.get(runtime).is_some()),
        move || tray(pad),
        || vstack(()).display_none(),
    )
}

fn tray(pad: Pad) -> impl View {
    component(move |cx| {
        let members: Signal<Vec<Member>> = cx.signal(Vec::new());
        cx.effect(move |runtime| {
            let Some(folder) = pad.folder.get(runtime) else {
                return;
            };
            pad.revision.get(runtime);
            pad.icons.with(runtime, |_| ());
            let shared: &Runtime = runtime;
            let found = pad.icons.with_untracked(shared, |icons| {
                pad.library.with(shared, |library| {
                    library
                        .folder(folder)
                        .into_iter()
                        .flat_map(|folder| folder.apps.iter())
                        .filter_map(|id| library.app(id))
                        .map(|app| Member {
                            id: app.id().clone(),
                            name: app.name().to_owned(),
                            image: icons.get(app.id()).copied(),
                        })
                        .collect::<Vec<_>>()
                })
            });
            if members.with(runtime, |current| *current != found) {
                members.set(runtime, found);
            }
        });
        let columns = move |runtime: &mut Runtime| {
            let count = members.with(runtime, Vec::len);
            let grid = pad
                .settings
                .with(runtime, |settings| usize::from(settings.grid().columns));
            count.clamp(3, MAX_COLUMNS.min(grid).max(3))
        };
        let cell = move |runtime: &mut Runtime| pad.geometry(runtime).cell;
        let grid = for_each(
            members,
            |member: &Member| member.id.clone(),
            move |cx, member| member_tile(cx, pad, member),
        )
        .flex_row()
        .flex_wrap()
        .w(bind(move |runtime| {
            cell(runtime).width * columns(runtime) as f32
        }));
        let tray = vstack((title(pad), grid))
            .items_center()
            .gap(20.0)
            .p(TRAY_PADDING)
            .w(bind(move |runtime| {
                cell(runtime).width * columns(runtime) as f32 + 2.0 * TRAY_PADDING
            }))
            .max_h(bind(move |runtime| {
                cell(runtime).height * MAX_ROWS as f32 + TITLE_SIZE * 2.0 + 2.0 * TRAY_PADDING
            }))
            .overflow_scroll()
            .rounded(TRAY_RADIUS)
            .bg(bind(move |runtime| {
                backdrop::tray_tint(pad.dark(runtime)).1
            }))
            .backdrop_blur(bind(move |runtime| {
                backdrop::tray_tint(pad.dark(runtime)).0
            }))
            .on_click(|_, event| event.stop_propagation())
            .on_drag_over(|_, event| {
                if event.offers(PAD_MIME) {
                    event.accept(PAD_MIME);
                }
                event.stop_propagation();
            })
            .on_drop(move |cx, event| {
                event.stop_propagation();
                settle_drag(cx, pad);
            })
            .enter(|style| style.scale(0.5).opacity(0.0))
            .exit(|style| style.scale(0.5).opacity(0.0));
        zstack(tray)
            .absolute()
            .inset(0.0)
            .items_center()
            .justify_center()
            .bg(SCRIM)
            .on_click(move |cx, event| {
                event.stop_propagation();
                close(cx, pad);
            })
            .on_drag_over(move |cx, event| {
                if event.offers(PAD_MIME) && pad.drag.with(cx, |drag| drag.from_folder.is_some()) {
                    pad.folder.set(cx, None);
                }
            })
    })
}

fn title(pad: Pad) -> impl View {
    let renaming = move |runtime: &mut Runtime| {
        let folder = pad.folder.get(runtime);
        pad.editing.with(runtime, |editing| matches!((editing, folder), (Some(Editing::Folder(editing)), Some(open)) if *editing == open))
    };
    show(
        bind(renaming),
        move || {
            text_field(pad.edit_text, "Folder name")
                .text_size(TITLE_SIZE)
                .text_center()
                .w(360.0)
                .on_submit(move |cx, _| commit_rename(cx, pad))
        },
        move || {
            text(bind(move |runtime| {
                let folder = pad.folder.get(runtime);
                pad.revision.get(runtime);
                pad.library.with(runtime, |library| {
                    folder
                        .and_then(|folder| library.folder(folder))
                        .map(|folder| folder.name.clone())
                        .unwrap_or_default()
                })
            }))
            .text_size(TITLE_SIZE)
            .font_semibold()
            .text_color(WHITE)
            .on_click(move |cx, event| {
                event.stop_propagation();
                if let Some(folder) = pad.folder.get(cx) {
                    begin_rename(cx, pad, Editing::Folder(folder));
                }
            })
        },
    )
}

fn member_tile(cx: &mut Cx, pad: Pad, member: Signal<Member>) -> impl View + use<> {
    let id = member.with(cx, |member| member.id.clone());
    let icon_size = move |runtime: &mut Runtime| pad.geometry(runtime).icon;
    let cell = move |runtime: &mut Runtime| pad.geometry(runtime).cell;
    let art = crownui::prelude::keyed(
        move |runtime| {
            (
                member.with(runtime, |member| member.image),
                icon_size(runtime).to_bits(),
            )
        },
        move |_, (image, size)| art_view(&Art::App(image), f32::from_bits(size), true),
    )
    .rounded(bind(move |runtime| icon_size(runtime) * ICON_CORNER));
    let label = text(bind(move |runtime| {
        member.with(runtime, |member| member.name.clone())
    }))
    .text_size(12.5)
    .text_color(WHITE)
    .text_center()
    .truncate()
    .w(bind(move |runtime| cell(runtime).width - 8.0))
    .hidden(bind(move |runtime| {
        !pad.settings.with(runtime, |settings| settings.app_labels)
    }));
    let folder = pad.folder.get(cx);
    let clicked = Item::App(id.clone());
    let lifted = Item::App(id.clone());
    let target = id.clone();
    let menu_item = Item::App(id);
    vstack((art, label))
        .gap(8.0)
        .items_center()
        .justify_center()
        .w(bind(move |runtime| cell(runtime).width))
        .h(bind(move |runtime| cell(runtime).height))
        .opacity(bind(move |runtime| {
            let lifted = pad.drag.with(runtime, |drag| drag.lifted.clone());
            if lifted.as_ref() == Some(&Item::App(member.with(runtime, |member| member.id.clone())))
            {
                0.3
            } else {
                1.0
            }
        }))
        .on_click(move |cx, event| {
            event.stop_propagation();
            activate::open(cx, pad, &clicked);
        })
        .on_context_menu(move |cx, event| {
            event.stop_propagation();
            super::menu::open_for_item(cx, pad, &menu_item, event.position);
        })
        .on_drag_start(move |cx, event| {
            event.stop_propagation();
            if let Some(folder) = folder {
                drag::lift(cx, pad, &lifted, event.position, Origin::Folder(folder));
            }
        })
        .on_drop(move |cx, event| {
            event.stop_propagation();
            if let Some(folder) = folder {
                reorder_onto(cx, pad, folder, &target);
            }
        })
}

fn reorder_onto(cx: &mut Cx, pad: Pad, folder: FolderId, target: &DesktopId) {
    let lifted = pad
        .drag
        .with(cx, |drag| drag.lifted.as_ref().and_then(Item::app).cloned());
    settle_drag(cx, pad);
    let Some(lifted) = lifted else {
        return;
    };
    pad.edit_library(cx, |library| {
        let apps = &library.folder(folder)?.apps;
        let from = apps.iter().position(|id| *id == lifted)?;
        let to = apps.iter().position(|id| id == target)?;
        library.reorder_in_folder(folder, from, to);
        Some(())
    });
}

fn settle_drag(cx: &mut Cx, pad: Pad) {
    pad.drag.set(cx, drag::DragState::default());
    pad.refresh(cx);
}
