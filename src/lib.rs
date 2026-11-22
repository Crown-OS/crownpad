pub mod catalog;
pub mod dock;
pub mod icon;
pub mod ipc;
pub mod launch;
pub mod library;
mod pad;
pub mod protocol;
pub mod search;
pub mod settings;
pub mod uninstall;
mod util;

pub use pad::root;

use anyhow::{Context, Result};
use crownui::{
    AppConfig, kit,
    prelude::{Anchor, KeyboardInteractivity, Layer, LayerOptions, TRANSPARENT},
};

use library::{Layout, Library};
use settings::PadSettings;

const NAMESPACE: &str = "crownpad";
const FALLBACK_SIZE: (u32, u32) = (1280, 800);

pub fn app() -> Result<()> {
    let remote = ipc::serve()?;
    util::non_default_gpu::probe();
    let settings: PadSettings = crownconfig::load_section(PadSettings::SECTION);
    let layout: Layout = crownconfig::load_section(Layout::SECTION);
    let mut dirs = catalog::default_dirs();
    dirs.extend(settings.app_sources.iter().cloned());
    let library = Library::new(
        catalog::scan(&dirs),
        layout,
        settings.grid().capacity(),
        pad_now(),
    );
    crownui::run(pad::root(settings, library, remote), app_config())
        .context("the launchpad stopped")
}

pub fn app_config() -> AppConfig {
    let layer = LayerOptions::new(
        NAMESPACE,
        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
    )
    .layer(Layer::Overlay)
    .exclusive_zone(-1)
    .keyboard(KeyboardInteractivity::None);
    AppConfig::new("CrownPad")
        .app_id(NAMESPACE)
        .size(FALLBACK_SIZE.0, FALLBACK_SIZE.1)
        .clear_color(TRANSPARENT)
        .accessibility(false)
        .font(kit::inter_font().to_vec(), Some(kit::INTER_FAMILY))
        .layer(layer)
}

fn pad_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
