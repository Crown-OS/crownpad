use crownconfig::SectionWatch;
use crownui::prelude::Cx;

use super::state::Pad;
use crate::settings::PadSettings;

pub fn follow(cx: &mut Cx, pad: Pad) {
    let initial = pad.settings.get(cx);
    next(
        cx,
        pad,
        crownconfig::watch_section(PadSettings::SECTION, initial),
    );
}

fn next(cx: &mut Cx, pad: Pad, mut watch: SectionWatch<PadSettings>) {
    let edited = async move {
        let settings = watch.changed().await;
        (watch, settings)
    };
    cx.spawn(edited, move |cx, (watch, settings)| {
        if pad.settings.with(cx, |current| *current != settings) {
            pad.adopt_settings(cx, settings);
        }
        next(cx, pad, watch);
    });
}
