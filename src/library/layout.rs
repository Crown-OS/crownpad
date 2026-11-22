use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{FolderId, Item, Slot};
use crate::catalog::DesktopId;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Folder {
    pub id: FolderId,
    pub name: String,
    pub apps: Vec<DesktopId>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub launches: u32,
    pub last_opened: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub pages: Vec<Vec<Item>>,
    pub folders: Vec<Folder>,
    pub hidden: Vec<DesktopId>,
    pub aliases: BTreeMap<DesktopId, String>,
    pub usage: BTreeMap<DesktopId, Usage>,
    pub first_seen: BTreeMap<DesktopId, u64>,
}

impl Layout {
    pub const SECTION: &'static str = "crownpad-layout";

    pub fn item(&self, slot: Slot) -> Option<&Item> {
        self.pages.get(slot.page)?.get(slot.index)
    }

    pub fn locate(&self, item: &Item) -> Option<Slot> {
        self.pages.iter().enumerate().find_map(|(page, items)| {
            let index = items.iter().position(|candidate| candidate == item)?;
            Some(Slot::new(page, index))
        })
    }

    pub fn folder(&self, id: FolderId) -> Option<&Folder> {
        self.folders.iter().find(|folder| folder.id == id)
    }

    pub(super) fn folder_mut(&mut self, id: FolderId) -> Option<&mut Folder> {
        self.folders.iter_mut().find(|folder| folder.id == id)
    }

    pub(super) fn next_folder_id(&self) -> FolderId {
        FolderId(
            self.folders
                .iter()
                .map(|folder| folder.id.0)
                .max()
                .map_or(1, |max| max + 1),
        )
    }

    pub(super) fn remove(&mut self, slot: Slot) -> Option<Item> {
        let page = self.pages.get_mut(slot.page)?;
        (slot.index < page.len()).then(|| page.remove(slot.index))
    }

    pub(super) fn insert(&mut self, slot: Slot, item: Item) {
        if slot.page >= self.pages.len() {
            self.pages.push(vec![item]);
            return;
        }
        let page = &mut self.pages[slot.page];
        page.insert(slot.index.min(page.len()), item);
    }

    pub(super) fn append(&mut self, item: Item) {
        match self.pages.last_mut() {
            Some(page) => page.push(item),
            None => self.pages.push(vec![item]),
        }
    }

    pub(super) fn normalize(&mut self, capacity: usize) {
        let capacity = capacity.max(1);
        let mut page = 0;
        while page < self.pages.len() {
            if self.pages[page].len() > capacity {
                let overflow = self.pages[page].split_off(capacity);
                if page + 1 == self.pages.len() {
                    self.pages.push(Vec::new());
                }
                self.pages[page + 1].splice(0..0, overflow);
            }
            page += 1;
        }
        self.pages.retain(|items| !items.is_empty());
        if self.pages.is_empty() {
            self.pages.push(Vec::new());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str) -> Item {
        Item::App(DesktopId::new(name))
    }

    #[test]
    fn overflow_spills_onto_the_next_page_and_empty_pages_vanish() {
        let mut layout = Layout {
            pages: vec![vec![app("a"), app("b"), app("c")], vec![], vec![app("d")]],
            ..Layout::default()
        };
        layout.normalize(2);
        assert_eq!(
            layout.pages,
            vec![vec![app("a"), app("b")], vec![app("c")], vec![app("d")]]
        );
    }

    #[test]
    fn inserting_past_the_last_page_opens_a_new_one() {
        let mut layout = Layout {
            pages: vec![vec![app("a")]],
            ..Layout::default()
        };
        layout.insert(Slot::new(5, 0), app("b"));
        assert_eq!(layout.pages.len(), 2);
        assert_eq!(layout.locate(&app("b")), Some(Slot::new(1, 0)));
    }

    #[test]
    fn layout_round_trips_through_ron() {
        let layout = Layout {
            pages: vec![vec![app("a.desktop"), Item::Folder(FolderId(3))]],
            folders: vec![Folder {
                id: FolderId(3),
                name: "Games".to_owned(),
                apps: vec![DesktopId::new("b.desktop")],
            }],
            ..Layout::default()
        };
        let text = ron::to_string(&layout).unwrap_or_default();
        let parsed: Layout = ron::from_str(&text).unwrap_or_default();
        assert_eq!(parsed, layout);
    }
}
