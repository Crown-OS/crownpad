mod actions;
mod activate;
mod backdrop;
mod dialog;
mod dots;
mod drag;
mod folder_view;
mod geometry;
mod gesture;
mod icons;
mod keys;
mod menu;
mod motion;
mod paging;
mod probe;
mod remote;
mod search_bar;
mod state;
mod sync;
mod tile;
mod tiles;
mod visibility;

use crownui::{
    kit,
    prelude::{
        Curve, Listeners, Runtime, Styled, TRANSPARENT, Vector, View, bind, component, for_each,
        keyed, vstack, zstack,
    },
};

use crate::{
    ipc::Remote,
    library::Library,
    settings::{Browsing, PadSettings},
};
use drag::Origin;
use state::Pad;
use tiles::Tile;

const PEAK_SCALE: f32 = 1.08;

pub fn root(settings: PadSettings, library: Library, remote: Remote) -> impl View {
    component(move |cx| {
        let theme = kit::provide_configured_theme(cx);
        let Remote { requests, report } = remote;
        let pad = Pad::new(cx, settings, library, report, theme);
        cx.request_animation_frame(move |cx, _| {
            remote::follow(cx, pad, requests);
            sync::follow(cx, pad);
        });
        pad.refresh(cx);
        zstack((
            probe::size_probe(pad.surface).absolute().inset(0.0),
            icon_loader(pad),
            body(pad),
        ))
        .pointer_events_none()
        .on_key_down(move |cx, event| keys::down(cx, pad, event))
    })
}

fn icon_loader(pad: Pad) -> impl View {
    keyed(
        move |runtime| {
            let laid_out = pad.surface.get(runtime).width > 0.0;
            laid_out.then(|| icons::edge(runtime, pad))
        },
        move |cx, edge| {
            if let Some(edge) = edge {
                icons::reload(cx, pad, edge);
            }
            vstack(()).display_none()
        },
    )
}

fn body(pad: Pad) -> impl View {
    let content = zstack((search_bar::search_bar(pad), viewport(pad), dots::dots(pad)))
        .absolute()
        .inset(0.0)
        .opacity(bind(move |runtime| pad.reveal.get(runtime).clamp(0.0, 1.0)))
        .scale(bind(move |runtime| {
            PEAK_SCALE - (PEAK_SCALE - 1.0) * pad.reveal.get(runtime).clamp(0.0, 1.0)
        }));
    zstack((
        backdrop(pad),
        content,
        folder_view::folder_view(pad),
        gesture::band_view(pad),
        menu::menu_view(pad),
        dialog::dialog_view(pad),
    ))
    .absolute()
    .left(0.0)
    .top(0.0)
    .right(0.0)
    .bottom(bind(move |runtime| drag::dock_band(runtime, pad)))
    .pointer_events_auto()
    .hidden(bind(move |runtime| !pad.on_screen(runtime)))
    .on_click(move |cx, _| {
        if pad
            .settings
            .with(cx, |settings| settings.click_empty_to_close)
            && pad.selection.with(cx, |selection| selection.is_empty())
        {
            visibility::set(cx, pad, false);
        } else {
            pad.selection.set(cx, Default::default());
        }
    })
    .on_context_menu(move |cx, event| {
        event.stop_propagation();
        menu::open_empty(cx, pad, event.position);
    })
    .on_wheel(move |cx, event| paging::wheel(cx, pad, event))
    .on_pointer_down(move |cx, event| {
        pad.modifiers
            .update(cx, |modifiers| *modifiers = event.modifiers)
    })
    .on_drag_start(move |cx, event| gesture::start(cx, pad, event))
    .on_drag(move |cx, event| gesture::moved(cx, pad, event))
    .on_drag_end(move |cx, event| gesture::end(cx, pad, event))
    .on_drag_over(move |cx, event| drag::over(cx, pad, event))
    .on_drag_leave(move |cx, _| drag::leave(cx, pad))
    .on_drop(move |cx, event| drag::drop(cx, pad, event))
}

fn backdrop(pad: Pad) -> impl View {
    vstack(())
        .absolute()
        .inset(0.0)
        .material(bind(move |runtime| {
            let backdrop = pad.settings.with(runtime, |settings| settings.backdrop);
            backdrop::material(backdrop, pad.reveal.get(runtime), pad.dark(runtime))
        }))
        .bg(bind(move |runtime| {
            if pad.capabilities.get(runtime).materials {
                return TRANSPARENT;
            }
            let backdrop = pad.settings.with(runtime, |settings| settings.backdrop);
            let solid = backdrop::solid(backdrop, pad.dark(runtime));
            crownui::prelude::Color {
                a: solid.a * pad.reveal.get(runtime),
                ..solid
            }
        }))
}

fn viewport(pad: Pad) -> impl View {
    let offset = move |runtime: &mut Runtime| {
        let geometry = pad.geometry(runtime);
        match geometry.browsing {
            Browsing::Paged => {
                let swipe = pad.gesture.get(runtime).swipe_offset();
                Vector::new(
                    swipe - pad.page.get(runtime) as f32 * geometry.page_width(),
                    0.0,
                )
            }
            Browsing::Scroll => Vector::new(0.0, -pad.scroll.get(runtime)),
        }
    };
    let strip = for_each(pad.tiles, Tile::key, move |cx, tile| {
        tile::tile(cx, pad, tile, Origin::Grid)
    })
    .absolute()
    .inset(0.0)
    .translate(bind(offset))
    .transition(Curve::Spring(motion::spring()));
    zstack(strip)
        .absolute()
        .left(0.0)
        .right(0.0)
        .top(bind(move |runtime| pad.geometry(runtime).grid_top()))
        .h(bind(move |runtime| pad.geometry(runtime).viewport_height()))
        .overflow_hidden()
}
