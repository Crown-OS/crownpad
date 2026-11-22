use std::collections::HashSet;

use crownui::prelude::Cx;

use super::{
    activate,
    dialog::{self, Dialog},
    folder_view,
    state::{Editing, Pad},
};
use crate::{
    catalog::DesktopId,
    dock, launch,
    library::{Edge, FolderId, Item, Slot},
    settings::{Browsing, SortMode},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Open(Item),
    OpenOnNonDefaultGpu(DesktopId),
    ShowInFiles(DesktopId),
    AddToDock(DesktopId),
    RenameApp(DesktopId),
    RemoveFromFolder(FolderId, DesktopId),
    MoveToFolder(DesktopId, FolderId),
    MoveToPage(Item, usize),
    Hide(Vec<DesktopId>),
    Uninstall(DesktopId),
    RenameFolder(FolderId),
    Dissolve(FolderId),
    OpenSelection,
    Group(Vec<Item>),
    ClearSelection,
    Sort(SortMode),
    Browse(Browsing),
    ArrangeByHue,
    UngroupAll,
    FoldersFirst,
    FoldersLast,
    Reset,
    Unhide(DesktopId),
    Settings,
}

pub fn apply(cx: &mut Cx, pad: Pad, action: Action) {
    match action {
        Action::Open(item) => activate::open(cx, pad, &item),
        Action::OpenOnNonDefaultGpu(id) => pad.launch_on_non_default_gpu(cx, &id),
        Action::ShowInFiles(id) => {
            if let Some(path) = desktop_path(cx, pad, &id) {
                launch::reveal(&path);
            }
        }
        Action::AddToDock(id) => {
            if let Some(path) = desktop_path(cx, pad, &id)
                && let Err(error) = dock::pin(&path)
            {
                log::warn!("pin {id} to the dock: {error}");
            }
        }
        Action::RenameApp(id) => {
            folder_view::begin_rename(cx, pad, Editing::App(id.clone()));
            pad.dialog.set(cx, Some(Dialog::Rename(id)));
        }
        Action::RemoveFromFolder(folder, id) => manual_edit(cx, pad, |library| {
            let last = library.pages().len().saturating_sub(1);
            library.take_out_of_folder(folder, &id, Slot::new(last, usize::MAX));
        }),
        Action::MoveToFolder(id, folder) => manual_edit(cx, pad, |library| {
            let from = library.layout().locate(&Item::App(id.clone()));
            let onto = library.layout().locate(&Item::Folder(folder));
            if let (Some(from), Some(onto)) = (from, onto) {
                library.merge(from, onto);
            }
        }),
        Action::MoveToPage(item, page) => {
            manual_edit(cx, pad, |library| library.move_to_page(&item, page))
        }
        Action::Hide(ids) => {
            pad.selection.set(cx, HashSet::new());
            pad.edit_library(cx, |library| library.hide(&ids));
        }
        Action::Uninstall(id) => dialog::ask_uninstall(cx, pad, id),
        Action::RenameFolder(folder) => {
            folder_view::open(cx, pad, folder);
            folder_view::begin_rename(cx, pad, Editing::Folder(folder));
        }
        Action::Dissolve(folder) => pad.edit_library(cx, |library| library.dissolve(folder)),
        Action::OpenSelection => activate::open_selection(cx, pad),
        Action::Group(items) => {
            pad.selection.set(cx, HashSet::new());
            let folder = manual_edit(cx, pad, |library| library.group(&items));
            if let Some(folder) = folder {
                folder_view::open(cx, pad, folder);
                folder_view::begin_rename(cx, pad, Editing::Folder(folder));
            }
        }
        Action::ClearSelection => pad.selection.set(cx, HashSet::new()),
        Action::Sort(mode) => pad.update_settings(cx, |settings| settings.sort = mode),
        Action::Browse(browsing) => {
            pad.update_settings(cx, |settings| settings.browsing = browsing)
        }
        Action::ArrangeByHue => {
            let hues = pad.hues.with(cx, Clone::clone);
            manual_edit(cx, pad, |library| library.arrange_by_hue(&hues));
        }
        Action::UngroupAll => manual_edit(cx, pad, |library| library.ungroup_all()),
        Action::FoldersFirst => manual_edit(cx, pad, |library| library.folders_to(Edge::Start)),
        Action::FoldersLast => manual_edit(cx, pad, |library| library.folders_to(Edge::End)),
        Action::Reset => manual_edit(cx, pad, |library| library.reset(super::state::now())),
        Action::Unhide(id) => pad.edit_library(cx, |library| library.unhide(&id)),
        Action::Settings => pad.dialog.set(cx, Some(Dialog::Settings)),
    }
}

fn manual_edit<R>(
    cx: &mut Cx,
    pad: Pad,
    change: impl FnOnce(&mut crate::library::Library) -> R,
) -> R {
    if pad
        .settings
        .with(cx, |settings| settings.sort != SortMode::Manual)
    {
        pad.update_settings(cx, |settings| settings.sort = SortMode::Manual);
    }
    pad.edit_library(cx, change)
}

fn desktop_path(cx: &mut Cx, pad: Pad, id: &DesktopId) -> Option<std::path::PathBuf> {
    pad.library.with(cx, |library| {
        library.app(id).map(|app| app.entry().path.clone())
    })
}
