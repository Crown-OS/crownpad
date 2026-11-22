use serde::{Deserialize, Serialize};

use crate::catalog::DesktopId;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct FolderId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    App(DesktopId),
    Folder(FolderId),
}

impl Item {
    pub fn app(&self) -> Option<&DesktopId> {
        match self {
            Self::App(id) => Some(id),
            Self::Folder(_) => None,
        }
    }

    pub fn folder(&self) -> Option<FolderId> {
        match self {
            Self::App(_) => None,
            Self::Folder(id) => Some(*id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slot {
    pub page: usize,
    pub index: usize,
}

impl Slot {
    pub const fn new(page: usize, index: usize) -> Self {
        Self { page, index }
    }
}
