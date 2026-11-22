use std::collections::HashMap;

use super::{Item, Library};
use crate::catalog::DesktopId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Name,
    DateAdded,
    LastOpened,
    Frequency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Start,
    End,
}

type SortKey = (u64, String);

impl Library {
    pub fn arrange(&mut self, order: Order) {
        self.rearrange(|library, item| {
            let rank = match order {
                Order::Name => 0,
                Order::DateAdded => {
                    u64::MAX
                        - library
                            .members(item)
                            .map(|id| library.installed_at(id))
                            .max()
                            .unwrap_or(0)
                }
                Order::LastOpened => {
                    u64::MAX
                        - library
                            .members(item)
                            .map(|id| library.last_opened(id))
                            .max()
                            .unwrap_or(0)
                }
                Order::Frequency => {
                    u64::MAX
                        - library
                            .members(item)
                            .map(|id| u64::from(library.launches(id)))
                            .sum::<u64>()
                }
            };
            (rank, library.item_name(item).to_lowercase())
        });
    }

    pub fn arrange_by_hue(&mut self, hues: &HashMap<DesktopId, f32>) {
        self.rearrange(|library, item| {
            let hue = library.members(item).find_map(|id| hues.get(id));
            let rank = hue.map_or(u64::MAX, |hue| (hue.rem_euclid(360.0) * 100.0) as u64);
            (rank, library.item_name(item).to_lowercase())
        });
    }

    pub fn ungroup_all(&mut self) {
        let folders: Vec<_> = self.layout.folders.iter().map(|folder| folder.id).collect();
        for folder in folders {
            self.dissolve(folder);
        }
    }

    pub fn folders_to(&mut self, edge: Edge) {
        let items: Vec<Item> = self.layout.pages.drain(..).flatten().collect();
        let (folders, apps): (Vec<Item>, Vec<Item>) =
            items.into_iter().partition(|item| item.folder().is_some());
        let ordered = match edge {
            Edge::Start => folders.into_iter().chain(apps).collect(),
            Edge::End => apps.into_iter().chain(folders).collect(),
        };
        self.layout.pages = vec![ordered];
        self.commit();
    }

    pub fn reset(&mut self, now: u64) {
        self.layout.pages.clear();
        self.layout.folders.clear();
        self.reconcile(now);
        self.commit();
    }

    pub fn item_name(&self, item: &Item) -> &str {
        match item {
            Item::App(id) => self.apps.get(id).map_or("", |app| app.name()),
            Item::Folder(id) => self
                .layout
                .folder(*id)
                .map_or("", |folder| folder.name.as_str()),
        }
    }

    fn rearrange(&mut self, key: impl Fn(&Self, &Item) -> SortKey) {
        let mut items: Vec<Item> = self.layout.pages.drain(..).flatten().collect();
        items.sort_by_cached_key(|item| key(self, item));
        self.layout.pages = vec![items];
        self.commit();
    }

    fn members<'a>(&'a self, item: &'a Item) -> impl Iterator<Item = &'a DesktopId> {
        let folder_apps = item
            .folder()
            .and_then(|id| self.layout.folder(id))
            .map(|folder| folder.apps.iter());
        item.app()
            .into_iter()
            .chain(folder_apps.into_iter().flatten())
    }

    fn installed_at(&self, id: &DesktopId) -> u64 {
        self.apps.get(id).map_or(0, |app| app.entry().installed_at)
    }

    fn last_opened(&self, id: &DesktopId) -> u64 {
        self.layout
            .usage
            .get(id)
            .map_or(0, |usage| usage.last_opened)
    }

    fn launches(&self, id: &DesktopId) -> u32 {
        self.layout.usage.get(id).map_or(0, |usage| usage.launches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::testing::{app, library};

    #[test]
    fn frequency_puts_the_most_launched_first_and_breaks_ties_by_name() {
        let mut library = library(&["a", "b", "c"], 30);
        library.record_launch(&DesktopId::new("c"), 5);
        library.record_launch(&DesktopId::new("c"), 6);
        library.record_launch(&DesktopId::new("b"), 7);
        library.arrange(Order::Frequency);
        assert_eq!(library.pages(), [vec![app("c"), app("b"), app("a")]]);
    }

    #[test]
    fn last_opened_puts_the_most_recent_first() {
        let mut library = library(&["a", "b"], 30);
        library.record_launch(&DesktopId::new("a"), 5);
        library.record_launch(&DesktopId::new("b"), 9);
        library.arrange(Order::LastOpened);
        assert_eq!(library.pages(), [vec![app("b"), app("a")]]);
    }

    #[test]
    fn arranging_repaginates() {
        let mut library = library(&["c", "b", "a"], 2);
        library.arrange(Order::Name);
        assert_eq!(library.pages(), [vec![app("a"), app("b")], vec![app("c")]]);
    }

    #[test]
    fn folders_can_be_moved_to_either_edge() {
        let mut library = library(&["a", "b", "c", "d"], 30);
        let Some(folder) = library.group(&[app("c"), app("d")]) else {
            panic!("no folder");
        };
        library.folders_to(Edge::Start);
        assert_eq!(
            library.pages(),
            [vec![Item::Folder(folder), app("a"), app("b")]]
        );
        library.folders_to(Edge::End);
        assert_eq!(
            library.pages(),
            [vec![app("a"), app("b"), Item::Folder(folder)]]
        );
    }

    #[test]
    fn hue_sorts_colourless_icons_last() {
        let mut library = library(&["a", "b", "c"], 30);
        let hues = HashMap::from([(DesktopId::new("a"), 200.0), (DesktopId::new("c"), 10.0)]);
        library.arrange_by_hue(&hues);
        assert_eq!(library.pages(), [vec![app("c"), app("a"), app("b")]]);
    }
}
