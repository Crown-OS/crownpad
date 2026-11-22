use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::library::Order;

pub const MAX_GRID_EXTENT: u8 = 15;
const MIN_GRID_EXTENT: u8 = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Browsing {
    #[default]
    Paged,
    Scroll,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortMode {
    #[default]
    Manual,
    Name,
    DateAdded,
    LastOpened,
    Frequency,
}

impl SortMode {
    pub fn order(self) -> Option<Order> {
        match self {
            Self::Manual => None,
            Self::Name => Some(Order::Name),
            Self::DateAdded => Some(Order::DateAdded),
            Self::LastOpened => Some(Order::LastOpened),
            Self::Frequency => Some(Order::Frequency),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backdrop {
    Thin,
    #[default]
    Thick,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reopen {
    #[default]
    LastPage,
    FirstPage,
    FirstPageAfter(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PadSettings {
    pub rows: u8,
    pub columns: u8,
    pub browsing: Browsing,
    pub sort: SortMode,
    pub backdrop: Backdrop,
    pub app_labels: bool,
    pub folder_labels: bool,
    pub reopen: Reopen,
    pub click_empty_to_close: bool,
    pub hidden_in_search: bool,
    pub app_sources: Vec<PathBuf>,
}

impl Default for PadSettings {
    fn default() -> Self {
        Self {
            rows: 5,
            columns: 7,
            browsing: Browsing::Paged,
            sort: SortMode::Manual,
            backdrop: Backdrop::Thick,
            app_labels: true,
            folder_labels: true,
            reopen: Reopen::LastPage,
            click_empty_to_close: true,
            hidden_in_search: false,
            app_sources: Vec::new(),
        }
    }
}

impl PadSettings {
    pub const SECTION: &'static str = "crownpad";

    pub fn grid(&self) -> GridShape {
        let clamp = |extent: u8| extent.clamp(MIN_GRID_EXTENT, MAX_GRID_EXTENT);
        GridShape {
            rows: clamp(self.rows),
            columns: clamp(self.columns),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridShape {
    pub rows: u8,
    pub columns: u8,
}

impl GridShape {
    pub fn capacity(self) -> usize {
        usize::from(self.rows) * usize::from(self.columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_file_keeps_the_other_defaults() {
        let settings: PadSettings =
            ron::from_str("(rows: 4, browsing: Scroll)").unwrap_or_default();
        assert_eq!(settings.rows, 4);
        assert_eq!(settings.browsing, Browsing::Scroll);
        assert_eq!(settings.columns, PadSettings::default().columns);
    }

    #[test]
    fn grid_is_clamped_to_the_supported_range() {
        let settings = PadSettings {
            rows: 40,
            columns: 0,
            ..PadSettings::default()
        };
        assert_eq!(
            settings.grid(),
            GridShape {
                rows: 15,
                columns: 2
            }
        );
        assert_eq!(settings.grid().capacity(), 30);
    }
}
