use std::collections::{HashMap, HashSet};

use crownui::prelude::{Cx, Runtime};

use super::state::Pad;
use crate::{
    catalog::DesktopId,
    icon::{self, Pixels},
};

#[derive(Default)]
pub struct IconLoads {
    edge: u32,
    attempted: HashSet<DesktopId>,
}

pub fn edge(runtime: &mut Runtime, pad: Pad) -> u32 {
    let icon = pad.geometry(runtime).icon;
    let scale = pad.scale.get(runtime);
    (icon * scale).round().max(1.0) as u32
}

pub fn reload(cx: &mut Cx, pad: Pad, edge: u32) {
    let released: Vec<_> = pad
        .icons
        .with(cx, |icons| icons.values().copied().collect());
    pad.icons.set(cx, HashMap::new());
    for image in released {
        cx.release_image(image);
    }
    pad.loads.update(cx, |loads| {
        loads.edge = edge;
        loads.attempted.clear();
    });
    pad.refresh(cx);
}

pub fn sync(cx: &mut Cx, pad: Pad) {
    let edge = pad.loads.with(cx, |loads| loads.edge);
    if edge == 0 {
        return;
    }
    let installed: HashMap<DesktopId, Option<String>> = pad.library.with(cx, |library| {
        library
            .apps()
            .map(|app| (app.id().clone(), app.entry().icon.clone()))
            .collect()
    });
    let mut gone = Vec::new();
    pad.icons.update(cx, |icons| {
        icons.retain(|id, image| {
            let keep = installed.contains_key(id);
            if !keep {
                gone.push(*image);
            }
            keep
        });
    });
    for image in gone {
        cx.release_image(image);
    }
    let missing: Vec<(DesktopId, Option<String>)> = pad.loads.update(cx, |loads| {
        loads.attempted.retain(|id| installed.contains_key(id));
        installed
            .into_iter()
            .filter(|(id, _)| loads.attempted.insert(id.clone()))
            .collect()
    });
    if !missing.is_empty() {
        load(cx, pad, missing, edge);
    }
}

fn load(cx: &mut Cx, pad: Pad, missing: Vec<(DesktopId, Option<String>)>, edge: u32) {
    let decode = async move {
        tokio::task::spawn_blocking(move || {
            missing
                .into_iter()
                .filter_map(|(id, name)| {
                    let pixels = icon::load(name.as_deref(), edge)?;
                    let hue = icon::dominant_hue(&pixels.rgba);
                    Some((id, pixels, hue))
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default()
    };
    cx.spawn(decode, move |cx, decoded| {
        if pad.loads.with(cx, |loads| loads.edge) != edge {
            return;
        }
        for (id, pixels, hue) in decoded {
            show(cx, pad, id, pixels, hue);
        }
        super::tiles::arrange(cx, pad);
    });
}

fn show(cx: &mut Cx, pad: Pad, id: DesktopId, pixels: Pixels, hue: Option<f32>) {
    if let Some(hue) = hue {
        pad.hues.update(cx, |hues| hues.insert(id.clone(), hue));
    }
    let image = cx.register_image(pixels.edge, pixels.edge, pixels.rgba);
    let mut replaced = None;
    pad.icons
        .update(cx, |icons| replaced = icons.insert(id, image));
    if let Some(previous) = replaced {
        cx.release_image(previous);
    }
}
