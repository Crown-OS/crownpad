use std::collections::HashSet;

use crownui::prelude::Cx;

use super::{drag::Origin, folder_view, state::Pad};
use crate::library::Item;

pub fn click(cx: &mut Cx, pad: Pad, item: &Item, origin: Origin) {
    let modifiers = pad.modifiers.with(cx, |modifiers| *modifiers);
    if (modifiers.ctrl_key || modifiers.meta_key) && origin == Origin::Grid {
        pad.selection.update(cx, |selection| {
            if !selection.remove(item) {
                selection.insert(item.clone());
            }
        });
        return;
    }
    if pad.selection.with(cx, |selection| !selection.is_empty()) {
        pad.selection.set(cx, HashSet::new());
    }
    open(cx, pad, item);
}

pub fn open(cx: &mut Cx, pad: Pad, item: &Item) {
    match item {
        Item::App(id) => pad.launch(cx, id),
        Item::Folder(folder) => folder_view::open(cx, pad, *folder),
    }
}

pub fn open_selection(cx: &mut Cx, pad: Pad) {
    let selected: Vec<Item> = pad
        .selection
        .with(cx, |selection| selection.iter().cloned().collect());
    let apps: Vec<_> = pad.library.with(cx, |library| {
        selected
            .iter()
            .flat_map(|item| match item {
                Item::App(id) => vec![id.clone()],
                Item::Folder(folder) => library
                    .folder(*folder)
                    .map(|folder| folder.apps.clone())
                    .unwrap_or_default(),
            })
            .collect()
    });
    for id in &apps {
        pad.launch(cx, id);
    }
}
