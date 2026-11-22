use super::{Folder, FolderId, Item, Layout, Library, Slot};
use crate::catalog::{DesktopId, folder_name};

impl Library {
    pub fn move_item(&mut self, from: Slot, to: Slot) {
        if from == to {
            return;
        }
        if let Some(item) = self.layout.remove(from) {
            self.layout.insert(to, item);
            self.commit();
        }
    }

    pub fn preview_move(&self, item: &Item, to: Slot) -> Vec<Vec<Item>> {
        let mut layout = Layout {
            pages: self.layout.pages.clone(),
            ..Layout::default()
        };
        if let Some(from) = layout.locate(item) {
            layout.remove(from);
        }
        layout.insert(to, item.clone());
        layout.normalize(self.capacity);
        layout.pages
    }

    pub fn move_to_page(&mut self, item: &Item, page: usize) {
        let Some(from) = self.layout.locate(item) else {
            return;
        };
        if from.page == page {
            return;
        }
        if let Some(item) = self.layout.remove(from) {
            let end = self.layout.pages.get(page).map_or(0, Vec::len);
            self.layout.insert(Slot::new(page, end), item);
            self.commit();
        }
    }

    pub fn merge(&mut self, from: Slot, onto: Slot) -> Option<FolderId> {
        let Some(Item::App(dragged)) = self.layout.item(from).cloned() else {
            return None;
        };
        let folder = match self.layout.item(onto)?.clone() {
            Item::Folder(id) => {
                self.layout.folder_mut(id)?.apps.push(dragged);
                id
            }
            Item::App(target) if target != dragged => {
                let id = self.layout.next_folder_id();
                let name = self.default_folder_name(&[&target, &dragged]);
                self.layout.folders.push(Folder {
                    id,
                    name,
                    apps: vec![target, dragged],
                });
                self.layout.pages[onto.page][onto.index] = Item::Folder(id);
                id
            }
            Item::App(_) => return None,
        };
        self.layout.remove(from);
        self.commit();
        Some(folder)
    }

    pub fn group(&mut self, items: &[Item]) -> Option<FolderId> {
        let anchor = items
            .iter()
            .filter_map(|item| self.layout.locate(item))
            .min()?;
        let mut apps = Vec::new();
        for item in items {
            match item {
                Item::App(id) => apps.push(id.clone()),
                Item::Folder(id) => apps.extend(
                    self.layout
                        .folder(*id)
                        .into_iter()
                        .flat_map(|folder| folder.apps.iter().cloned()),
                ),
            }
        }
        if apps.len() < 2 {
            return None;
        }
        let id = self.layout.next_folder_id();
        let members: Vec<&DesktopId> = apps.iter().collect();
        let name = self.default_folder_name(&members);
        self.layout.pages[anchor.page].insert(anchor.index, Item::Folder(id));
        for page in &mut self.layout.pages {
            page.retain(|item| !items.contains(item));
        }
        let grouped: Vec<FolderId> = items.iter().filter_map(Item::folder).collect();
        self.layout
            .folders
            .retain(|folder| !grouped.contains(&folder.id));
        self.layout.folders.push(Folder { id, name, apps });
        self.commit();
        Some(id)
    }

    pub fn reorder_in_folder(&mut self, folder: FolderId, from: usize, to: usize) {
        let Some(apps) = self
            .layout
            .folder_mut(folder)
            .map(|folder| &mut folder.apps)
        else {
            return;
        };
        if from == to || from >= apps.len() {
            return;
        }
        let app = apps.remove(from);
        apps.insert(to.min(apps.len()), app);
        self.dirty = true;
    }

    pub fn take_out_of_folder(&mut self, folder: FolderId, app: &DesktopId, to: Slot) {
        let Some(members) = self
            .layout
            .folder_mut(folder)
            .map(|folder| &mut folder.apps)
        else {
            return;
        };
        let before = members.len();
        members.retain(|member| member != app);
        if members.len() == before {
            return;
        }
        self.layout.insert(to, Item::App(app.clone()));
        self.collapse_small_folders();
        self.commit();
    }

    pub fn dissolve(&mut self, folder: FolderId) {
        let Some(slot) = self.layout.locate(&Item::Folder(folder)) else {
            return;
        };
        let apps = self
            .layout
            .folder(folder)
            .map(|folder| {
                folder
                    .apps
                    .iter()
                    .cloned()
                    .map(Item::App)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.layout.pages[slot.page].splice(slot.index..=slot.index, apps);
        self.layout
            .folders
            .retain(|candidate| candidate.id != folder);
        self.commit();
    }

    pub fn rename_folder(&mut self, folder: FolderId, name: &str) {
        let name = name.trim();
        if let Some(folder) = self.layout.folder_mut(folder)
            && !name.is_empty()
            && folder.name != name
        {
            folder.name = name.to_owned();
            self.dirty = true;
        }
    }

    pub fn rename_app(&mut self, id: &DesktopId, name: &str) {
        let Some(app) = self.apps.get_mut(id) else {
            return;
        };
        let name = name.trim();
        let alias = (!name.is_empty() && name != app.entry().name).then(|| name.to_owned());
        if alias.as_deref() == self.layout.aliases.get(id).map(String::as_str) {
            return;
        }
        match &alias {
            Some(alias) => self.layout.aliases.insert(id.clone(), alias.clone()),
            None => self.layout.aliases.remove(id),
        };
        app.set_alias(alias);
        self.dirty = true;
    }

    pub fn hide(&mut self, ids: &[DesktopId]) {
        for page in &mut self.layout.pages {
            page.retain(|item| item.app().is_none_or(|id| !ids.contains(id)));
        }
        for folder in &mut self.layout.folders {
            folder.apps.retain(|id| !ids.contains(id));
        }
        for id in ids {
            if !self.layout.hidden.contains(id) {
                self.layout.hidden.push(id.clone());
            }
        }
        self.collapse_small_folders();
        self.commit();
    }

    pub fn unhide(&mut self, id: &DesktopId) {
        let before = self.layout.hidden.len();
        self.layout.hidden.retain(|hidden| hidden != id);
        if self.layout.hidden.len() != before && self.apps.contains_key(id) {
            self.layout.append(Item::App(id.clone()));
            self.commit();
        }
    }

    pub fn record_launch(&mut self, id: &DesktopId, now: u64) {
        let usage = self.layout.usage.entry(id.clone()).or_default();
        usage.launches = usage.launches.saturating_add(1);
        usage.last_opened = now;
        self.dirty = true;
    }

    fn default_folder_name(&self, members: &[&DesktopId]) -> String {
        let categories = members
            .iter()
            .filter_map(|id| self.apps.get(*id))
            .map(|app| app.entry().categories.as_slice());
        folder_name(categories).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::testing::{app, library};

    #[test]
    fn moving_within_a_page_reorders_it() {
        let mut library = library(&["a", "b", "c"], 30);
        library.move_item(Slot::new(0, 0), Slot::new(0, 2));
        assert_eq!(library.pages(), [vec![app("b"), app("c"), app("a")]]);
    }

    #[test]
    fn inserting_into_a_full_page_pushes_its_last_item_over() {
        let mut library = library(&["a", "b", "c", "d"], 2);
        library.move_item(Slot::new(1, 1), Slot::new(0, 0));
        assert_eq!(
            library.pages(),
            [vec![app("d"), app("a")], vec![app("b"), app("c")]]
        );
    }

    #[test]
    fn dropping_an_app_on_another_makes_a_folder_in_its_place() {
        let mut library = library(&["a", "b", "c"], 30);
        let Some(folder) = library.merge(Slot::new(0, 2), Slot::new(0, 0)) else {
            panic!("no folder");
        };
        assert_eq!(library.pages(), [vec![Item::Folder(folder), app("b")]]);
        let members = library.folder(folder).map(|folder| folder.apps.clone());
        assert_eq!(
            members,
            Some(vec![DesktopId::new("a"), DesktopId::new("c")])
        );
    }

    #[test]
    fn taking_the_second_last_app_out_dissolves_the_folder() {
        let mut library = library(&["a", "b", "c"], 30);
        let Some(folder) = library.merge(Slot::new(0, 1), Slot::new(0, 0)) else {
            panic!("no folder");
        };
        library.take_out_of_folder(folder, &DesktopId::new("b"), Slot::new(0, 2));
        assert_eq!(library.pages(), [vec![app("a"), app("c"), app("b")]]);
        assert!(library.folder(folder).is_none());
    }

    #[test]
    fn dissolving_spreads_the_apps_where_the_folder_was() {
        let mut library = library(&["a", "b", "c"], 30);
        let Some(folder) = library.group(&[app("b"), app("c")]) else {
            panic!("no folder");
        };
        library.dissolve(folder);
        assert_eq!(library.pages(), [vec![app("a"), app("b"), app("c")]]);
    }

    #[test]
    fn hiding_removes_from_folders_too() {
        let mut library = library(&["a", "b", "c"], 30);
        library.group(&[app("a"), app("b")]);
        library.hide(&[DesktopId::new("a")]);
        assert_eq!(library.pages(), [vec![app("b"), app("c")]]);
        library.unhide(&DesktopId::new("a"));
        assert_eq!(library.pages(), [vec![app("b"), app("c"), app("a")]]);
    }

    #[test]
    fn renaming_back_to_the_original_clears_the_alias() {
        let mut library = library(&["a"], 30);
        let id = DesktopId::new("a");
        library.rename_app(&id, "Mine");
        assert_eq!(library.app(&id).map(|app| app.name()), Some("Mine"));
        library.rename_app(&id, "A");
        assert!(library.layout().aliases.is_empty());
    }

    #[test]
    fn preview_shows_the_move_without_making_it() {
        let library = library(&["a", "b", "c"], 30);
        let preview = library.preview_move(&app("c"), Slot::new(0, 0));
        assert_eq!(preview, [vec![app("c"), app("a"), app("b")]]);
        assert_eq!(library.pages(), [vec![app("a"), app("b"), app("c")]]);
    }

    #[test]
    fn move_to_page_appends_to_that_page() {
        let mut library = library(&["a", "b", "c"], 2);
        library.move_to_page(&app("a"), 1);
        assert_eq!(library.pages(), [vec![app("b")], vec![app("c"), app("a")]]);
    }
}
