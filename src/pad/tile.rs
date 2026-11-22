use crownui::{
    ext::ImageId,
    prelude::{
        AnyView, Color, Cx, Listeners, Point, Runtime, Signal, Size, Styled, View, WHITE, bind,
        cover_image, keyed, rgba, text, vstack, zstack,
    },
};

use super::{
    activate, backdrop,
    drag::{self, Origin},
    geometry::{LABEL_GAP, LABEL_HEIGHT},
    menu,
    state::Pad,
    tiles::{Art, Tile},
};
use crate::library::Item;

pub const ICON_CORNER: f32 = 0.225;
const LABEL_SIZE: f32 = 12.5;
const PRESSED_SCALE: f32 = 0.92;
const FOLDER_COLUMNS: usize = 3;
const HIGHLIGHT: Color = Color::new(1.0, 1.0, 1.0, 0.22);
const PLACEHOLDER: u32 = 0xFF_FF_FF_2E;

pub fn tile(cx: &mut Cx, pad: Pad, tile: Signal<Tile>, origin: Origin) -> impl View + use<> {
    let item = tile.with(cx, Tile::key);
    let position = move |runtime: &mut Runtime| {
        let geometry = pad.geometry(runtime);
        let (page, index) = tile.with(runtime, |tile| (tile.page, tile.index));
        let cell = geometry.cell_origin(page, index);
        Point::new(cell.x, cell.y - geometry.grid_top())
    };
    let cell = move |runtime: &mut Runtime| pad.geometry(runtime).cell;
    let icon_size = move |runtime: &mut Runtime| pad.geometry(runtime).icon;
    let shows_label = {
        let folder = matches!(item, Item::Folder(_));
        move |runtime: &mut Runtime| {
            pad.settings.with(runtime, |settings| {
                if folder {
                    settings.folder_labels
                } else {
                    settings.app_labels
                }
            })
        }
    };
    let highlighted = {
        let item = item.clone();
        move |runtime: &mut Runtime| {
            pad.highlight
                .with(runtime, |highlight| highlight.as_ref() == Some(&item))
                || pad
                    .selection
                    .with(runtime, |selection| selection.contains(&item))
        }
    };
    let merging = {
        let item = item.clone();
        move |runtime: &mut Runtime| {
            pad.drag
                .with(runtime, |drag| drag.onto.as_ref() == Some(&item))
        }
    };

    let art = keyed(
        move |runtime| {
            let art = tile.with(runtime, |tile| tile.art.clone());
            (art, icon_size(runtime).to_bits(), pad.dark(runtime))
        },
        move |_, (art, size, dark)| art_view(&art, f32::from_bits(size), dark),
    )
    .rounded(bind(move |runtime| icon_size(runtime) * ICON_CORNER))
    .scale(bind(
        move |runtime| if merging(runtime) { 1.12 } else { 1.0 },
    ))
    .active(|style| style.scale(PRESSED_SCALE));

    let label = text(bind(move |runtime| {
        tile.with(runtime, |tile| tile.label.clone())
    }))
    .text_size(LABEL_SIZE)
    .text_color(WHITE)
    .text_center()
    .truncate()
    .w(bind(move |runtime| cell(runtime).width - 8.0))
    .h(LABEL_HEIGHT)
    .hidden(bind(move |runtime| !shows_label(runtime)));

    let halo = vstack(())
        .absolute()
        .w(bind(move |runtime| icon_size(runtime) + 16.0))
        .h(bind(move |runtime| icon_size(runtime) + 16.0))
        .rounded(bind(move |runtime| {
            (icon_size(runtime) + 16.0) * ICON_CORNER
        }))
        .bg(HIGHLIGHT)
        .hidden(bind(move |runtime| !highlighted(runtime)));

    let clicked = item.clone();
    let menu_item = item.clone();
    let lifted = item.clone();
    zstack((halo, vstack((art, label)).gap(LABEL_GAP).items_center()))
        .absolute()
        .left(bind(move |runtime| position(runtime).x))
        .top(bind(move |runtime| position(runtime).y))
        .w(bind(move |runtime| cell(runtime).width))
        .h(bind(move |runtime| cell(runtime).height))
        .items_center()
        .justify_center()
        .opacity(bind(move |runtime| {
            if tile.with(runtime, |tile| tile.ghost) {
                0.0
            } else {
                1.0
            }
        }))
        .enter(|style| style.scale(0.6).opacity(0.0))
        .exit(|style| style.scale(0.6).opacity(0.0))
        .on_pointer_down(move |cx, event| {
            pad.modifiers
                .update(cx, |modifiers| *modifiers = event.modifiers)
        })
        .on_click(move |cx, event| {
            event.stop_propagation();
            activate::click(cx, pad, &clicked, origin);
        })
        .on_context_menu(move |cx, event| {
            event.stop_propagation();
            let corner = position(cx);
            let top = pad.geometry(cx).grid_top();
            let anchor = Point::new(
                corner.x + event.position.x,
                corner.y + top + event.position.y,
            );
            menu::open_for_item(cx, pad, &menu_item, anchor);
        })
        .on_drag_start(move |cx, event| {
            event.stop_propagation();
            drag::lift(cx, pad, &lifted, event.position, origin);
        })
}

pub fn art_view(art: &Art, size: f32, dark: bool) -> AnyView {
    match art {
        Art::App(image) => app_art(*image, size),
        Art::Folder(previews) => folder_art(previews, size, dark),
    }
}

fn app_art(image: Option<ImageId>, size: f32) -> AnyView {
    match image {
        Some(image) => cover_image(image, Size::new(size, size), 0.0)
            .size(size)
            .boxed(),
        None => vstack(())
            .size(size)
            .rounded(size * ICON_CORNER)
            .bg(rgba(PLACEHOLDER))
            .boxed(),
    }
}

fn folder_art(previews: &[Option<ImageId>], size: f32, dark: bool) -> AnyView {
    let (blur, tint) = backdrop::tray_tint(dark);
    let padding = size * 0.12;
    let gap = size * 0.05;
    let mini = (size - 2.0 * padding - gap * (FOLDER_COLUMNS - 1) as f32) / FOLDER_COLUMNS as f32;
    let rows: Vec<AnyView> = previews
        .chunks(FOLDER_COLUMNS)
        .map(|row| {
            let cells: Vec<AnyView> = row.iter().map(|image| app_art(*image, mini)).collect();
            crownui::prelude::hstack(cells).gap(gap).boxed()
        })
        .collect();
    vstack(rows)
        .size(size)
        .p(padding)
        .gap(gap)
        .rounded(size * ICON_CORNER)
        .bg(Color {
            a: tint.a.max(0.35),
            ..tint
        })
        .backdrop_blur(blur)
        .boxed()
}
