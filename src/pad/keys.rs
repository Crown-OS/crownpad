use crownui::{
    ext::{Key, KeyboardEvent, NamedKey},
    prelude::{Cx, Event},
};

use super::{activate, dialog, folder_view, paging, state::Pad, tiles::arrange, visibility};
use crate::settings::Browsing;

pub fn down(cx: &mut Cx, pad: Pad, event: &mut Event<KeyboardEvent>) {
    let escape = matches!(event.key, Key::Named(NamedKey::Escape));
    if pad.dialog.with(cx, Option::is_some) {
        if escape {
            dialog::close(cx, pad);
            event.prevent_default();
        }
        return;
    }
    if pad.editing.with(cx, Option::is_some) {
        if escape {
            pad.editing.set(cx, None);
            event.prevent_default();
        }
        return;
    }
    if pad.menu.with(cx, Option::is_some) {
        if escape {
            pad.menu.set(cx, None);
            event.prevent_default();
        }
        return;
    }
    let modifiers = event.modifiers;
    let command = modifiers.ctrl_key || modifiers.meta_key;
    match &event.key {
        Key::Named(NamedKey::Escape) => escape_one_level(cx, pad),
        Key::Named(NamedKey::Enter) => {
            if let Some(item) = pad.highlight.get(cx) {
                activate::open(cx, pad, &item);
            } else if pad.selection.with(cx, |selection| !selection.is_empty()) {
                activate::open_selection(cx, pad);
            }
        }
        Key::Named(NamedKey::ArrowLeft) if command => paging::turn(cx, pad, -1),
        Key::Named(NamedKey::ArrowRight) if command => paging::turn(cx, pad, 1),
        Key::Named(NamedKey::ArrowUp) if command => paging::scroll_by(cx, pad, f32::NEG_INFINITY),
        Key::Named(NamedKey::ArrowDown) if command => paging::scroll_by(cx, pad, f32::INFINITY),
        Key::Named(NamedKey::ArrowLeft) => step_highlight(cx, pad, -1),
        Key::Named(NamedKey::ArrowRight) => step_highlight(cx, pad, 1),
        Key::Named(NamedKey::ArrowUp) => step_rows(cx, pad, -1),
        Key::Named(NamedKey::ArrowDown) => step_rows(cx, pad, 1),
        Key::Named(NamedKey::Backspace) => edit_query(cx, pad, |query| {
            query.pop();
        }),
        Key::Character(character) if command && *character == ',' => {
            pad.dialog.set(cx, Some(dialog::Dialog::Settings));
        }
        _ => {
            let typed = event.text.clone().filter(|text| {
                !command
                    && !modifiers.alt_key
                    && text.chars().all(|character| !character.is_control())
            });
            match typed {
                Some(text) if !text.is_empty() => {
                    edit_query(cx, pad, |query| query.push_str(&text))
                }
                _ => return,
            }
        }
    }
    event.prevent_default();
}

fn escape_one_level(cx: &mut Cx, pad: Pad) {
    if pad.folder.get(cx).is_some() {
        folder_view::close(cx, pad);
    } else if pad.is_searching(cx) {
        edit_query(cx, pad, String::clear);
    } else if pad.selection.with(cx, |selection| !selection.is_empty()) {
        pad.selection.set(cx, Default::default());
    } else {
        visibility::set(cx, pad, false);
    }
}

fn edit_query(cx: &mut Cx, pad: Pad, change: impl FnOnce(&mut String)) {
    let before = pad.query.get(cx);
    pad.query.update(cx, change);
    let searching = pad.is_searching(cx);
    if before.trim().is_empty() && searching {
        let page = pad.page.get(cx);
        pad.resume_page.update(cx, |resume| *resume = Some(page));
        pad.page.set(cx, 0);
    }
    if !searching {
        pad.highlight.set(cx, None);
        if let Some(page) = pad.resume_page.update(cx, Option::take) {
            pad.page.set(cx, page);
        }
    }
    arrange(cx, pad);
}

fn step_rows(cx: &mut Cx, pad: Pad, direction: isize) {
    let columns = pad
        .settings
        .with(cx, |settings| isize::from(settings.grid().columns));
    step_highlight(cx, pad, direction * columns);
}

fn step_highlight(cx: &mut Cx, pad: Pad, step: isize) {
    if pad.folder.get(cx).is_some() {
        return;
    }
    let tiles = pad.tiles.get(cx);
    if tiles.is_empty() {
        return;
    }
    let current = pad.highlight.get(cx);
    let position = current.and_then(|item| tiles.iter().position(|tile| tile.item == item));
    let next = match position {
        Some(position) => position.saturating_add_signed(step).min(tiles.len() - 1),
        None => {
            let page = pad.page.get(cx);
            let scroll = pad
                .settings
                .with(cx, |settings| settings.browsing == Browsing::Scroll);
            tiles
                .iter()
                .position(|tile| scroll || tile.page == page)
                .unwrap_or(0)
        }
    };
    let tile = &tiles[next];
    pad.highlight.set(cx, Some(tile.item.clone()));
    paging::reveal(cx, pad, tile.page, tile.index);
}
