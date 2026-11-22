use std::path::PathBuf;

use super::{Item, Layout, Library};
use crate::{
    catalog::{AppEntry, DesktopId},
    util::non_default_gpu::GpuPreference,
};

pub fn entry(id: &str, name: &str) -> AppEntry {
    AppEntry {
        id: DesktopId::new(id),
        path: PathBuf::from(format!("/apps/{id}")),
        name: name.to_owned(),
        icon: None,
        exec: Some(name.to_lowercase()),
        terminal: false,
        gpu: GpuPreference::Default,
        categories: Vec::new(),
        window_class: None,
        installed_at: 0,
    }
}

pub fn app(id: &str) -> Item {
    Item::App(DesktopId::new(id))
}

pub fn library(ids: &[&str], capacity: usize) -> Library {
    let entries = ids.iter().map(|id| entry(id, &id.to_uppercase())).collect();
    Library::new(entries, Layout::default(), capacity, 0)
}
