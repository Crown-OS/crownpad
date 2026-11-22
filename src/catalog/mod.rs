mod category;
mod desktop_id;
mod entry;
mod exec;
mod scan;

pub use category::folder_name;
pub use desktop_id::DesktopId;
pub use entry::AppEntry;
pub use exec::command;
pub use scan::{default_dirs, scan};

pub const DESKTOP_EXTENSION: &str = "desktop";
