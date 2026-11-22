use std::path::Path;

use crate::{
    catalog::{self, AppEntry},
    util::{non_default_gpu::GpuPreference, process::spawn_detached},
};

const TERMINAL_LAUNCHER: &str = "xdg-terminal-exec";
const OPENER: &str = "xdg-open";

pub fn launch(entry: &AppEntry) {
    launch_on(entry, entry.gpu);
}

pub fn launch_on(entry: &AppEntry, gpu: GpuPreference) {
    let Some(mut argv) = entry.exec.as_deref().and_then(catalog::command) else {
        log::warn!("{} has nothing to run", entry.path.display());
        return;
    };
    if entry.terminal {
        argv.insert(0, TERMINAL_LAUNCHER.to_owned());
    }
    spawn_detached(argv, gpu);
}

pub fn reveal(path: &Path) {
    let folder = path.parent().unwrap_or(path);
    spawn_detached(
        vec![OPENER.to_owned(), folder.to_string_lossy().into_owned()],
        GpuPreference::Default,
    );
}
