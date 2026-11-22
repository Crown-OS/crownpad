use std::collections::HashMap;

use crownui::{
    ext::ImageId,
    prelude::{Cx, Runtime},
};

use super::state::Pad;
use crate::{
    catalog::DesktopId,
    library::{Item, Library},
    search::Query,
};

pub const FOLDER_PREVIEW: usize = 9;

#[derive(Debug, Clone, PartialEq)]
pub enum Art {
    App(Option<ImageId>),
    Folder(Vec<Option<ImageId>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub item: Item,
    pub page: usize,
    pub index: usize,
    pub label: String,
    pub art: Art,
    pub ghost: bool,
}

impl Tile {
    pub fn key(&self) -> Item {
        self.item.clone()
    }
}

pub fn arrange(cx: &mut Cx, pad: Pad) {
    let query = pad.query.with(cx, |query| Query::new(query));
    let include_hidden = pad.settings.with(cx, |settings| settings.hidden_in_search);
    let drag = pad.drag.get(cx);
    let pages = pad.library.with(cx, |library| {
        if !query.is_empty() {
            let results: Vec<Item> = library
                .search(&query, include_hidden)
                .into_iter()
                .map(Item::App)
                .collect();
            results
                .chunks(library.capacity().max(1))
                .map(<[Item]>::to_vec)
                .collect()
        } else if let Some((item, gap)) = drag.preview() {
            library.preview_move(item, gap)
        } else {
            library.pages().to_vec()
        }
    });
    let tiles = {
        let runtime: &Runtime = cx;
        pad.icons.with_untracked(runtime, |icons| {
            pad.library.with(runtime, |library| {
                build(library, &pages, icons, drag.lifted.as_ref())
            })
        })
    };
    let count = pages.len().max(1);
    if pad.tiles.with(cx, |current| *current != tiles) {
        pad.tiles.set(cx, tiles);
    }
    if pad.page_count.get(cx) != count {
        pad.page_count.set(cx, count);
    }
    if pad.page.get(cx) >= count {
        pad.page.set(cx, count - 1);
    }
    if !query.is_empty() {
        let first = pages.first().and_then(|page| page.first()).cloned();
        let highlighted = pad.highlight.get(cx);
        let present = highlighted
            .as_ref()
            .is_some_and(|item| pages.iter().flatten().any(|candidate| candidate == item));
        if !present {
            pad.highlight.set(cx, first);
        }
    }
}

pub fn build(
    library: &Library,
    pages: &[Vec<Item>],
    icons: &HashMap<DesktopId, ImageId>,
    lifted: Option<&Item>,
) -> Vec<Tile> {
    pages
        .iter()
        .enumerate()
        .flat_map(|(page, items)| {
            items
                .iter()
                .enumerate()
                .map(move |(index, item)| (page, index, item))
        })
        .map(|(page, index, item)| Tile {
            item: item.clone(),
            page,
            index,
            label: library.item_name(item).to_owned(),
            art: art(library, item, icons),
            ghost: lifted == Some(item),
        })
        .collect()
}

fn art(library: &Library, item: &Item, icons: &HashMap<DesktopId, ImageId>) -> Art {
    match item {
        Item::App(id) => Art::App(icons.get(id).copied()),
        Item::Folder(id) => Art::Folder(
            library
                .folder(*id)
                .into_iter()
                .flat_map(|folder| folder.apps.iter())
                .take(FOLDER_PREVIEW)
                .map(|app| icons.get(app).copied())
                .collect(),
        ),
    }
}
