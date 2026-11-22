use std::collections::HashSet;

use crownui::{
    ext::DragEvent,
    prelude::{Color, Cx, Point, Rect, Styled, View, bind, vstack},
};

use super::{drag::DragState, paging, state::Pad};
use crate::settings::Browsing;

const SWIPE_DISTANCE_SHARE: f32 = 0.18;
const SWIPE_VELOCITY: f32 = 650.0;
const BAND_FILL: Color = Color::new(1.0, 1.0, 1.0, 0.12);
const BAND_BORDER: Color = Color::new(1.0, 1.0, 1.0, 0.45);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Gesture {
    #[default]
    Idle,
    Swipe(f32),
    Band {
        start: Point,
        end: Point,
    },
}

impl Gesture {
    pub fn swipe_offset(self) -> f32 {
        match self {
            Self::Swipe(offset) => offset,
            _ => 0.0,
        }
    }

    fn band(self) -> Option<Rect> {
        match self {
            Self::Band { start, end } => Some(Rect::from_xywh(
                start.x.min(end.x),
                start.y.min(end.y),
                (end.x - start.x).abs(),
                (end.y - start.y).abs(),
            )),
            _ => None,
        }
    }
}

pub fn start(cx: &mut Cx, pad: Pad, event: &DragEvent) {
    if pad.drag.with(cx, DragState::is_active) {
        return;
    }
    let modifiers = pad.modifiers.with(cx, |modifiers| *modifiers);
    let gesture = if modifiers.ctrl_key || modifiers.shift_key || modifiers.meta_key {
        Gesture::Band {
            start: event.position,
            end: event.position,
        }
    } else {
        Gesture::Swipe(0.0)
    };
    pad.gesture.set(cx, gesture);
}

pub fn moved(cx: &mut Cx, pad: Pad, event: &DragEvent) {
    match pad.gesture.get(cx) {
        Gesture::Idle => {}
        Gesture::Swipe(_) => match pad.settings.with(cx, |settings| settings.browsing) {
            Browsing::Paged => pad.gesture.set(cx, Gesture::Swipe(event.total.x)),
            Browsing::Scroll => paging::scroll_by(cx, pad, -event.delta.y),
        },
        Gesture::Band { start, .. } => pad.gesture.set(
            cx,
            Gesture::Band {
                start,
                end: event.position,
            },
        ),
    }
}

pub fn end(cx: &mut Cx, pad: Pad, event: &DragEvent) {
    let gesture = pad.gesture.get(cx);
    pad.gesture.set(cx, Gesture::Idle);
    match gesture {
        Gesture::Idle => {}
        Gesture::Swipe(offset) => {
            let width = pad.surface.with(cx, |surface| surface.width);
            let flung = event.velocity.x.abs() > SWIPE_VELOCITY;
            let dragged = offset.abs() > width * SWIPE_DISTANCE_SHARE;
            if flung || dragged {
                let direction = if flung { event.velocity.x } else { offset };
                paging::turn(cx, pad, if direction < 0.0 { 1 } else { -1 });
            }
        }
        Gesture::Band { .. } => select_within(cx, pad, gesture.band()),
    }
}

fn select_within(cx: &mut Cx, pad: Pad, band: Option<Rect>) {
    let Some(band) = band else {
        return;
    };
    let geometry = pad.geometry(cx);
    let page = pad.page.get(cx);
    let offset = match geometry.browsing {
        Browsing::Paged => Point::new(page as f32 * geometry.page_width(), 0.0),
        Browsing::Scroll => Point::new(0.0, pad.scroll.get(cx)),
    };
    let selected: HashSet<_> = pad.tiles.with(cx, |tiles| {
        tiles
            .iter()
            .filter(|tile| {
                let cell = geometry.cell_origin(tile.page, tile.index);
                let rect = Rect::from_xywh(
                    cell.x - offset.x,
                    cell.y - offset.y,
                    geometry.cell.width,
                    geometry.cell.height,
                );
                rect.intersection(band).is_some()
            })
            .map(|tile| tile.item.clone())
            .collect()
    });
    pad.selection
        .update(cx, |selection| selection.extend(selected));
}

pub fn band_view(pad: Pad) -> impl View {
    let band = move |runtime: &mut crownui::prelude::Runtime| pad.gesture.get(runtime).band();
    vstack(())
        .absolute()
        .left(bind(move |runtime| {
            band(runtime).map_or(0.0, |rect| rect.origin.x)
        }))
        .top(bind(move |runtime| {
            band(runtime).map_or(0.0, |rect| rect.origin.y)
        }))
        .w(bind(move |runtime| {
            band(runtime).map_or(0.0, |rect| rect.size.width)
        }))
        .h(bind(move |runtime| {
            band(runtime).map_or(0.0, |rect| rect.size.height)
        }))
        .rounded(6.0)
        .bg(BAND_FILL)
        .border(1.0)
        .border_color(BAND_BORDER)
        .hidden(bind(move |runtime| band(runtime).is_none()))
}
