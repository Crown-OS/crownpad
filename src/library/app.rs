use crate::{
    catalog::{AppEntry, DesktopId},
    search::{Query, SearchKey},
};

#[derive(Debug, Clone)]
pub struct App {
    entry: AppEntry,
    alias: Option<String>,
    name_key: SearchKey,
    alias_key: Option<SearchKey>,
}

impl App {
    pub fn new(entry: AppEntry, alias: Option<&str>) -> Self {
        let mut app = Self {
            name_key: SearchKey::new(&entry.name),
            entry,
            alias: None,
            alias_key: None,
        };
        app.set_alias(alias.map(str::to_owned));
        app
    }

    pub fn id(&self) -> &DesktopId {
        &self.entry.id
    }

    pub fn entry(&self) -> &AppEntry {
        &self.entry
    }

    pub fn name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.entry.name)
    }

    pub fn score(&self, query: &Query) -> Option<u32> {
        let by_name = query.score(&self.name_key);
        let by_alias = self.alias_key.as_ref().and_then(|key| query.score(key));
        by_name.max(by_alias)
    }

    pub(super) fn set_alias(&mut self, alias: Option<String>) {
        self.alias_key = alias.as_deref().map(SearchKey::new);
        self.alias = alias;
    }
}
