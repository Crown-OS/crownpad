use crownui::{
    kit::icons,
    prelude::{Color, Listeners, Styled, View, WHITE, bind, hstack, icon, text},
};

use super::{
    backdrop,
    geometry::{SEARCH_HEIGHT, SEARCH_TOP, SEARCH_WIDTH},
    state::Pad,
};

const PLACEHOLDER: Color = Color::new(1.0, 1.0, 1.0, 0.6);
const GLYPH_SIZE: f32 = 14.0;
const SEARCH_SVG_OPEN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"##;

pub fn search_bar(pad: Pad) -> impl View {
    let glyph = icon(
        format!("{SEARCH_SVG_OPEN}{}</svg>", icons::SEARCH),
        PLACEHOLDER,
    )
    .size(GLYPH_SIZE);
    let label = text(bind(move |runtime| {
        let query = pad.query.get(runtime);
        if query.is_empty() {
            "Search".to_owned()
        } else {
            query
        }
    }))
    .text_size(14.0)
    .truncate()
    .text_color(bind(move |runtime| {
        if pad.query.with(runtime, String::is_empty) {
            PLACEHOLDER
        } else {
            WHITE
        }
    }));
    hstack((glyph, label))
        .absolute()
        .top(SEARCH_TOP)
        .left(bind(move |runtime| {
            (pad.surface.get(runtime).width - SEARCH_WIDTH) / 2.0
        }))
        .w(SEARCH_WIDTH)
        .h(SEARCH_HEIGHT)
        .px(12.0)
        .gap(8.0)
        .items_center()
        .rounded(SEARCH_HEIGHT / 2.0)
        .border(1.0)
        .border_color(Color::new(1.0, 1.0, 1.0, 0.16))
        .bg(bind(move |runtime| {
            backdrop::tray_tint(pad.dark(runtime)).1
        }))
        .backdrop_blur(bind(move |runtime| {
            backdrop::tray_tint(pad.dark(runtime)).0
        }))
        .on_click(|_, event| event.stop_propagation())
}
