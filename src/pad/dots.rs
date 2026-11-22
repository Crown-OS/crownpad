use crownui::prelude::{Listeners, Styled, View, bind, component, for_each, hstack, rgba, vstack};

use super::{paging, state::Pad};
use crate::settings::Browsing;

const DOT: f32 = 8.0;
const BOTTOM: f32 = 30.0;

pub fn dots(pad: Pad) -> impl View {
    component(move |cx| {
        let indices = cx.signal(Vec::new());
        cx.effect(move |runtime| {
            let count = pad.page_count.get(runtime);
            let wanted: Vec<usize> = (0..count).collect();
            if indices.with(runtime, |current| *current != wanted) {
                indices.set(runtime, wanted);
            }
        });
        let row = for_each(
            indices,
            |index: &usize| *index,
            move |cx, index| {
                let index = index.get(cx);
                vstack(())
                    .size(DOT)
                    .rounded(DOT / 2.0)
                    .bg(bind(move |runtime| {
                        let active = pad.page.get(runtime) == index;
                        if active {
                            rgba(0xFF_FF_FF_F0)
                        } else {
                            rgba(0xFF_FF_FF_59)
                        }
                    }))
                    .pointer_events_auto()
                    .on_click(move |cx, event| {
                        event.stop_propagation();
                        paging::go_to(cx, pad, index);
                    })
            },
        )
        .flex_row()
        .gap(DOT);
        hstack(row)
            .absolute()
            .left(0.0)
            .right(0.0)
            .bottom(BOTTOM)
            .justify_center()
            .hidden(bind(move |runtime| {
                let scrolling = pad
                    .settings
                    .with(runtime, |settings| settings.browsing == Browsing::Scroll);
                scrolling || pad.page_count.get(runtime) < 2
            }))
    })
}
