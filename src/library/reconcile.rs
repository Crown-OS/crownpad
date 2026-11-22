use std::collections::HashSet;

use super::{Item, Library};
use crate::catalog::DesktopId;

impl Library {
    pub(super) fn reconcile(&mut self, now: u64) {
        let before = self.layout.clone();
        let apps = &self.apps;
        let layout = &mut self.layout;

        layout.hidden.retain(|id| apps.contains_key(id));
        layout.aliases.retain(|id, _| apps.contains_key(id));
        let hidden: HashSet<DesktopId> = layout.hidden.iter().cloned().collect();
        let mut placed: HashSet<DesktopId> = HashSet::new();
        let mut placeable = |id: &DesktopId| {
            apps.contains_key(id) && !hidden.contains(id) && placed.insert(id.clone())
        };

        for folder in &mut layout.folders {
            folder.apps.retain(&mut placeable);
        }
        let mut seen_folders = HashSet::new();
        for page in &mut layout.pages {
            page.retain(|item| match item {
                Item::App(id) => placeable(id),
                Item::Folder(id) => seen_folders.insert(*id),
            });
        }
        layout.folders.retain(|folder| !folder.apps.is_empty());
        let unreferenced: Vec<Item> = layout
            .folders
            .iter()
            .filter(|folder| !seen_folders.contains(&folder.id))
            .map(|folder| Item::Folder(folder.id))
            .collect();
        for item in unreferenced {
            layout.append(item);
        }

        let mut newcomers: Vec<&DesktopId> = apps
            .keys()
            .filter(|id| !hidden.contains(*id) && !placed.contains(*id))
            .collect();
        newcomers.sort_unstable_by_key(|id| apps.get(*id).map(|app| app.name().to_lowercase()));
        for id in newcomers {
            layout.first_seen.entry(id.clone()).or_insert(now);
            layout.append(Item::App(id.clone()));
        }

        self.collapse_small_folders();
        self.layout.normalize(self.capacity);
        self.dirty |= self.layout != before;
    }

    pub(super) fn collapse_small_folders(&mut self) {
        let layout = &mut self.layout;
        let folders = &layout.folders;
        for page in &mut layout.pages {
            page.retain_mut(|item| {
                let Item::Folder(id) = item else {
                    return true;
                };
                let apps = folders
                    .iter()
                    .find(|folder| folder.id == *id)
                    .map(|folder| folder.apps.as_slice());
                match apps {
                    Some([only]) => {
                        *item = Item::App(only.clone());
                        true
                    }
                    Some([]) | None => false,
                    Some(_) => true,
                }
            });
        }
        layout.folders.retain(|folder| folder.apps.len() > 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{
        Folder, FolderId, Layout,
        testing::{app, entry},
    };

    #[test]
    fn first_run_places_every_app_by_name() {
        let library = Library::new(
            vec![entry("b", "Beta"), entry("a", "alpha")],
            Layout::default(),
            30,
            7,
        );
        assert_eq!(library.pages(), [vec![app("a"), app("b")]]);
        assert_eq!(
            library.layout().first_seen.get(&DesktopId::new("a")),
            Some(&7)
        );
    }

    #[test]
    fn uninstalled_apps_vanish_and_new_ones_are_appended() {
        let stored = Layout {
            pages: vec![vec![app("gone"), app("b")]],
            ..Layout::default()
        };
        let library = Library::new(vec![entry("a", "A"), entry("b", "B")], stored, 30, 0);
        assert_eq!(library.pages(), [vec![app("b"), app("a")]]);
    }

    #[test]
    fn folders_left_with_one_app_collapse_into_it() {
        let stored = Layout {
            pages: vec![vec![Item::Folder(FolderId(1))]],
            folders: vec![Folder {
                id: FolderId(1),
                name: "Games".to_owned(),
                apps: vec![DesktopId::new("a"), DesktopId::new("gone")],
            }],
            ..Layout::default()
        };
        let library = Library::new(vec![entry("a", "A")], stored, 30, 0);
        assert_eq!(library.pages(), [vec![app("a")]]);
        assert!(library.layout().folders.is_empty());
    }

    #[test]
    fn hidden_apps_are_not_placed() {
        let stored = Layout {
            hidden: vec![DesktopId::new("a")],
            ..Layout::default()
        };
        let library = Library::new(vec![entry("a", "A"), entry("b", "B")], stored, 30, 0);
        assert_eq!(library.pages(), [vec![app("b")]]);
    }

    #[test]
    fn unchanged_layout_is_not_dirty() {
        let mut first = Library::new(vec![entry("a", "A")], Layout::default(), 30, 0);
        assert!(first.take_dirty());
        let mut second = Library::new(vec![entry("a", "A")], first.layout().clone(), 30, 0);
        assert!(!second.take_dirty());
    }
}
