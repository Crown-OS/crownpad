use std::time::{Duration, Instant};

use crownui::prelude::{Cx, KeyboardInteractivity};

use super::{motion, state::Pad};
use crate::settings::Reopen;

pub const HIDDEN: f32 = 0.001;

pub fn set(cx: &mut Cx, pad: Pad, visible: bool) {
    if visible {
        show(cx, pad);
    } else {
        hide(cx, pad);
    }
}

fn show(cx: &mut Cx, pad: Pad) {
    if pad.shown.get(cx) {
        return;
    }
    if returns_home(cx, pad) {
        pad.page.set(cx, 0);
        pad.scroll.set(cx, 0.0);
    }
    pad.shown.set(cx, true);
    pad.rescan(cx);
    cx.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    report(cx, pad, true);
    if pad.motion.update(cx, |motion| motion.reveal.aim(1.0)) {
        motion::animate(cx, pad);
    }
}

fn hide(cx: &mut Cx, pad: Pad) {
    if !pad.shown.get(cx) {
        return;
    }
    pad.shown.set(cx, false);
    pad.dismiss_transient(cx);
    pad.hidden_since
        .update(cx, |since| *since = Some(Instant::now()));
    cx.set_keyboard_interactivity(KeyboardInteractivity::None);
    report(cx, pad, false);
    pad.save(cx);
    if pad.motion.update(cx, |motion| motion.reveal.aim(0.0)) {
        motion::animate(cx, pad);
    }
}

fn returns_home(cx: &mut Cx, pad: Pad) -> bool {
    let reopen = pad.settings.with(cx, |settings| settings.reopen);
    let since = pad.hidden_since.with(cx, |since| *since);
    match reopen {
        Reopen::LastPage => false,
        Reopen::FirstPage => true,
        Reopen::FirstPageAfter(seconds) => {
            since.is_none_or(|since| since.elapsed() >= Duration::from_secs(seconds))
        }
    }
}

fn report(cx: &mut Cx, pad: Pad, visible: bool) {
    pad.report.with(cx, |report| report.set(visible));
}
