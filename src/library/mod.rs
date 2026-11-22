mod app;
mod arrange;
mod edit;
mod item;
mod layout;
mod reconcile;
#[cfg(test)]
mod testing;

pub use app::App;
pub use arrange::{Edge, Order};
pub use item::{FolderId, Item, Slot};
pub use layout::{Folder, Layout, Usage};

use std::collections::HashMap;

use crate::{
    catalog::{AppEntry, DesktopId},
    search::Query,
};

#[derive(Debug, Clone)]
pub struct Library {
    apps: HashMap<DesktopId, App>,
    layout: Layout,
    capacity: usize,
    dirty: bool,
}

impl Library {
    pub fn new(entries: Vec<AppEntry>, layout: Layout, capacity: usize, now: u64) -> Self {
        let mut library = Self {
            apps: HashMap::new(),
            layout,
            capacity,
            dirty: false,
        };
        library.refresh(entries, now);
        library
    }

    pub fn refresh(&mut self, entries: Vec<AppEntry>, now: u64) {
        self.apps = entries
            .into_iter()
            .map(|entry| {
                let alias = self.layout.aliases.get(&entry.id).map(String::as_str);
                (entry.id.clone(), App::new(entry, alias))
            })
            .collect();
        self.reconcile(now);
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        if self.capacity != capacity {
            self.capacity = capacity;
            self.commit();
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn pages(&self) -> &[Vec<Item>] {
        &self.layout.pages
    }

    pub fn app(&self, id: &DesktopId) -> Option<&App> {
        self.apps.get(id)
    }

    pub fn apps(&self) -> impl Iterator<Item = &App> {
        self.apps.values()
    }

    pub fn folder(&self, id: FolderId) -> Option<&Folder> {
        self.layout.folder(id)
    }

    pub fn hidden(&self) -> &[DesktopId] {
        &self.layout.hidden
    }

    pub fn is_hidden(&self, id: &DesktopId) -> bool {
        self.layout.hidden.contains(id)
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn search(&self, query: &Query, include_hidden: bool) -> Vec<DesktopId> {
        let mut matches: Vec<(u32, &App)> = self
            .apps
            .values()
            .filter(|app| include_hidden || !self.is_hidden(app.id()))
            .filter_map(|app| Some((app.score(query)?, app)))
            .collect();
        matches.sort_unstable_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.name().cmp(right.name()))
        });
        matches
            .into_iter()
            .map(|(_, app)| app.id().clone())
            .collect()
    }

    fn commit(&mut self) {
        self.layout.normalize(self.capacity);
        self.dirty = true;
    }
}
