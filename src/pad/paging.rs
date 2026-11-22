use std::time::{Duration, Instant};

use crownui::{ext::WheelEvent, prelude::Cx};
use input::WheelDelta;

use super::state::Pad;
use crate::settings::Browsing;

const LINE_HEIGHT: f32 = 40.0;
const PIXEL_THRESHOLD: f32 = 60.0;
const GESTURE_GAP: Duration = Duration::from_millis(240);

#[derive(Default)]
pub struct WheelPaging {
    accumulated: f32,
    last: Option<Instant>,
    turned: bool,
}

impl WheelPaging {
    fn feed(&mut self, amount: f32, notched: bool) -> isize {
        let now = Instant::now();
        if self
            .last
            .is_none_or(|last| now.duration_since(last) > GESTURE_GAP)
        {
            self.accumulated = 0.0;
            self.turned = false;
        }
        self.last = Some(now);
        if self.turned {
            return 0;
        }
        self.accumulated += amount;
        if notched || self.accumulated.abs() >= PIXEL_THRESHOLD {
            self.turned = true;
            return self.accumulated.signum() as isize;
        }
        0
    }
}

pub fn wheel(cx: &mut Cx, pad: Pad, event: &WheelEvent) {
    let delta = event.delta.to_pixels(LINE_HEIGHT);
    match pad.settings.with(cx, |settings| settings.browsing) {
        Browsing::Scroll => scroll_by(cx, pad, delta.y),
        Browsing::Paged => {
            let amount = if delta.x.abs() > delta.y.abs() {
                delta.x
            } else {
                delta.y
            };
            let notched = matches!(event.delta, WheelDelta::Lines(_));
            let step = pad.wheel.update(cx, |paging| paging.feed(amount, notched));
            if step != 0 {
                turn(cx, pad, step);
            }
        }
    }
}

pub fn turn(cx: &mut Cx, pad: Pad, step: isize) {
    let page = pad.page.get(cx);
    go_to(cx, pad, page.saturating_add_signed(step));
}

pub fn go_to(cx: &mut Cx, pad: Pad, page: usize) {
    let last = pad.page_count.get(cx).saturating_sub(1);
    let page = page.min(last);
    if pad.page.get(cx) != page {
        pad.page.set(cx, page);
    }
}

pub fn scroll_by(cx: &mut Cx, pad: Pad, delta: f32) {
    let limit = {
        let geometry = pad.geometry(cx);
        let items = pad.tiles.with(cx, Vec::len);
        geometry.scroll_limit(items)
    };
    let scroll = (pad.scroll.get(cx) + delta).clamp(0.0, limit);
    pad.scroll.set(cx, scroll);
}

pub fn reveal(cx: &mut Cx, pad: Pad, page: usize, index: usize) {
    match pad.settings.with(cx, |settings| settings.browsing) {
        Browsing::Paged => go_to(cx, pad, page),
        Browsing::Scroll => {
            let geometry = pad.geometry(cx);
            let cell = geometry.cell_origin(page, index);
            let centred = cell.y
                - geometry.grid_top()
                - (geometry.viewport_height() - geometry.cell.height) / 2.0;
            let current = pad.scroll.get(cx);
            scroll_by(cx, pad, centred - current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_touchpad_gesture_turns_one_page() {
        let mut paging = WheelPaging::default();
        let steps: Vec<isize> = (0..10).map(|_| paging.feed(20.0, false)).collect();
        assert_eq!(steps.iter().sum::<isize>(), 1);
    }

    #[test]
    fn a_wheel_notch_turns_at_once_in_its_direction() {
        let mut paging = WheelPaging::default();
        assert_eq!(paging.feed(-40.0, true), -1);
    }
}
