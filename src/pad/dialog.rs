use crownui::{
    kit::{
        ButtonOptions, ButtonVariant, SliderOptions, ToggleItem, button, setting_row, slider,
        toggle, toggle_group,
    },
    prelude::{
        AnyView, Color, Cx, Listeners, Styled, View, WHITE, hstack, keyed, text, text_field,
        vstack, zstack,
    },
};

use super::{backdrop, folder_view, state::Pad};
use crate::{
    catalog::DesktopId,
    settings::{Backdrop, Browsing, MAX_GRID_EXTENT, Reopen},
    uninstall::{self, Package},
};

const CARD_WIDTH: f32 = 440.0;
const CARD_RADIUS: f32 = 24.0;
const SCRIM: Color = Color::new(0.0, 0.0, 0.0, 0.32);

#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    Settings,
    Rename(DesktopId),
    Uninstall {
        id: DesktopId,
        name: String,
        package: Package,
    },
    Unremovable(String),
}

pub fn ask_uninstall(cx: &mut Cx, pad: Pad, id: DesktopId) {
    let found = pad.library.with(cx, |library| {
        library
            .app(&id)
            .map(|app| (app.entry().path.clone(), app.name().to_owned()))
    });
    let Some((path, name)) = found else {
        return;
    };
    let lookup = async move {
        tokio::task::spawn_blocking(move || uninstall::owner(&path))
            .await
            .ok()
            .flatten()
    };
    cx.spawn(lookup, move |cx, package| {
        let dialog = match package {
            Some(package) => Dialog::Uninstall { id, name, package },
            None => Dialog::Unremovable(name),
        };
        pad.dialog.set(cx, Some(dialog));
    });
}

pub fn close(cx: &mut Cx, pad: Pad) {
    pad.dialog.set(cx, None);
    pad.editing.set(cx, None);
}

pub fn dialog_view(pad: Pad) -> impl View {
    keyed(
        move |runtime| pad.dialog.get(runtime),
        move |cx, dialog| match dialog {
            None => vstack(()).display_none().boxed(),
            Some(dialog) => {
                let body = match dialog {
                    Dialog::Settings => settings(cx, pad),
                    Dialog::Rename(_) => rename(pad),
                    Dialog::Uninstall { name, package, .. } => confirm_uninstall(pad, name, package),
                    Dialog::Unremovable(name) => message(pad, format!("{name} was not installed by a package manager crownpad knows, so it can't be uninstalled here.")),
                };
                frame(cx, pad, body)
            }
        },
    )
    .absolute()
    .inset(0.0)
    .pointer_events_none()
}

fn frame(cx: &mut Cx, pad: Pad, body: AnyView) -> AnyView {
    let (blur, tint) = backdrop::tray_tint(pad.dark(cx));
    let card = vstack(body)
        .w(CARD_WIDTH)
        .p(24.0)
        .gap(16.0)
        .rounded(CARD_RADIUS)
        .bg(Color {
            a: tint.a.max(0.6),
            ..tint
        })
        .backdrop_blur(blur)
        .pointer_events_auto()
        .on_click(|_, event| event.stop_propagation())
        .enter(|style| style.scale(0.92).opacity(0.0));
    zstack(card)
        .absolute()
        .inset(0.0)
        .items_center()
        .justify_center()
        .bg(SCRIM)
        .pointer_events_auto()
        .on_click(move |cx, event| {
            event.stop_propagation();
            close(cx, pad);
        })
        .boxed()
}

fn heading(label: impl Into<String>) -> impl View {
    text(label.into())
        .text_size(18.0)
        .font_semibold()
        .text_color(WHITE)
}

fn rename(pad: Pad) -> AnyView {
    vstack((
        heading("Rename App"),
        text_field(pad.edit_text, "Name").on_submit(move |cx, _| {
            folder_view::commit_rename(cx, pad);
            pad.dialog.set(cx, None);
        }),
        hstack((
            button("Cancel").on_click(move |cx, _| close(cx, pad)),
            button("Rename")
                .variant(ButtonVariant::Primary)
                .on_click(move |cx, _| {
                    folder_view::commit_rename(cx, pad);
                    pad.dialog.set(cx, None);
                }),
        ))
        .gap(8.0)
        .justify_end(),
    ))
    .gap(16.0)
    .boxed()
}

fn confirm_uninstall(pad: Pad, name: String, package: Package) -> AnyView {
    let detail = format!(
        "This removes the package “{}” and its files from the system.",
        package.name()
    );
    vstack((
        heading(format!("Uninstall {name}?")),
        text(detail).text_color(Color::new(1.0, 1.0, 1.0, 0.75)),
        hstack((
            button("Cancel").on_click(move |cx, _| close(cx, pad)),
            button("Uninstall")
                .variant(ButtonVariant::Destructive)
                .on_click(move |cx, _| {
                    close(cx, pad);
                    let package = package.clone();
                    let removal = async move {
                        tokio::task::spawn_blocking(move || uninstall::remove(&package)).await
                    };
                    cx.spawn(removal, move |cx, result| {
                        match result {
                            Ok(Ok(())) => {}
                            Ok(Err(error)) => log::warn!("uninstall: {error}"),
                            Err(error) => log::warn!("uninstall task: {error}"),
                        }
                        pad.rescan(cx);
                    });
                }),
        ))
        .gap(8.0)
        .justify_end(),
    ))
    .gap(16.0)
    .boxed()
}

fn message(pad: Pad, body: String) -> AnyView {
    vstack((
        text(body).text_color(WHITE),
        hstack(
            button("OK")
                .variant(ButtonVariant::Primary)
                .on_click(move |cx, _| close(cx, pad)),
        )
        .justify_end(),
    ))
    .gap(16.0)
    .boxed()
}

fn settings(cx: &mut Cx, pad: Pad) -> AnyView {
    let current = pad.settings.get(cx);
    let rows = cx.signal(f32::from(current.rows));
    let columns = cx.signal(f32::from(current.columns));
    let browsing = cx.signal(usize::from(current.browsing == Browsing::Scroll));
    let material = cx.signal(usize::from(current.backdrop == Backdrop::Thick));
    let reopen = cx.signal(usize::from(current.reopen != Reopen::LastPage));
    let app_labels = cx.signal(current.app_labels);
    let folder_labels = cx.signal(current.folder_labels);
    let click_to_close = cx.signal(current.click_empty_to_close);
    let hidden_in_search = cx.signal(current.hidden_in_search);
    let extent = |value: f32| value.round().clamp(2.0, f32::from(MAX_GRID_EXTENT)) as u8;
    let apply = move |cx: &mut Cx| {
        let (rows, columns) = (extent(rows.get(cx)), extent(columns.get(cx)));
        let browsing = if browsing.get(cx) == 1 {
            Browsing::Scroll
        } else {
            Browsing::Paged
        };
        let backdrop = if material.get(cx) == 1 {
            Backdrop::Thick
        } else {
            Backdrop::Thin
        };
        let reopen = if reopen.get(cx) == 1 {
            Reopen::FirstPage
        } else {
            Reopen::LastPage
        };
        let flags = (
            app_labels.get(cx),
            folder_labels.get(cx),
            click_to_close.get(cx),
            hidden_in_search.get(cx),
        );
        pad.update_settings(cx, |settings| {
            settings.rows = rows;
            settings.columns = columns;
            settings.browsing = browsing;
            settings.backdrop = backdrop;
            settings.reopen = reopen;
            (
                settings.app_labels,
                settings.folder_labels,
                settings.click_empty_to_close,
                settings.hidden_in_search,
            ) = flags;
        });
        pad.dialog.set(cx, None);
    };
    let max = f32::from(MAX_GRID_EXTENT);
    vstack((
        heading("Launchpad Settings"),
        setting_row("Rows", slider(rows).range(2.0, max).step(1.0)),
        setting_row("Columns", slider(columns).range(2.0, max).step(1.0)),
        setting_row(
            "Browse",
            toggle_group(
                [ToggleItem::text("Pages"), ToggleItem::text("Scroll")],
                browsing,
            ),
        ),
        setting_row(
            "Backdrop",
            toggle_group(
                [ToggleItem::text("Thin"), ToggleItem::text("Thick")],
                material,
            ),
        ),
        setting_row(
            "Reopen on",
            toggle_group(
                [
                    ToggleItem::text("Last Page"),
                    ToggleItem::text("First Page"),
                ],
                reopen,
            ),
        ),
        setting_row("App names", toggle(app_labels)),
        setting_row("Folder names", toggle(folder_labels)),
        setting_row("Click empty space to close", toggle(click_to_close)),
        setting_row("Hidden apps in search", toggle(hidden_in_search)),
        hstack((
            button("Cancel").on_click(move |cx, _| close(cx, pad)),
            button("Done")
                .variant(ButtonVariant::Primary)
                .on_click(move |cx, _| apply(cx)),
        ))
        .gap(8.0)
        .justify_end(),
    ))
    .gap(12.0)
    .boxed()
}
