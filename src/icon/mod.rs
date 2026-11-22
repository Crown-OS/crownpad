mod decode;
mod hue;
mod lookup;

pub use hue::dominant_hue;

#[derive(Clone)]
pub struct Pixels {
    pub edge: u32,
    pub rgba: Vec<u8>,
}

pub fn load(icon: Option<&str>, edge: u32) -> Option<Pixels> {
    let file = icon
        .and_then(|name| lookup::find(name, edge))
        .or_else(|| lookup::find(lookup::FALLBACK, edge))?;
    decode::decode(&file, edge)
        .inspect_err(|error| log::warn!("icon {}: {error:#}", file.display()))
        .ok()
}
