use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crownui::{
    ext::{DragOverEvent, DropEvent},
    prelude::{Cx, DragIcon, DragOutcome, DragSource, Point, Runtime},
};

use crowndock::settings::{DockSettings, Metrics};

use super::{geometry::Target, paging, state::Pad, visibility};
use crate::{
    icon,
    library::{FolderId, Item, Slot},
    settings::{Browsing, SortMode},
    util::uri::{URI_LIST, file_paths, uri_list},
};

pub const PAD_MIME: &str = "application/x-crownpad-item";
const EDGE: f32 = 56.0;
const FLIP_DELAY: Duration = Duration::from_millis(550);
/// How long an app held at the top edge waits before the pad steps aside, so
/// it can be dropped on the desktop underneath.
const ASIDE_DELAY: Duration = Duration::from_millis(550);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Grid,
    Folder(FolderId),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DragState {
    pub lifted: Option<Item>,
    pub from_folder: Option<FolderId>,
    pub gap: Option<Slot>,
    pub onto: Option<Item>,
    pub flip: Option<(isize, u64)>,
    /// When the lifted app came to rest at the top edge.
    pub aside: Option<Instant>,
    pub dock_band: f32,
}

impl DragState {
    pub fn preview(&self) -> Option<(&Item, Slot)> {
        Some((self.lifted.as_ref()?, self.gap?))
    }

    pub fn is_active(&self) -> bool {
        self.lifted.is_some()
    }
}

pub fn dock_band(runtime: &mut Runtime, pad: Pad) -> f32 {
    pad.drag.with(runtime, |drag| drag.dock_band)
}

fn measure_dock() -> f32 {
    let dock: DockSettings = crownconfig::load_section(DockSettings::SECTION);
    Metrics::new(&dock).surface_height() as f32
}

pub fn lift(cx: &mut Cx, pad: Pad, item: &Item, grip: Point, origin: Origin) {
    let (path, icon_name) = pad.library.with(cx, |library| {
        let app = item.app().and_then(|id| library.app(id)).or_else(|| {
            let folder = library.folder(item.folder()?)?;
            library.app(folder.apps.first()?)
        });
        (
            item.app()
                .and_then(|id| library.app(id))
                .map(|app| app.entry().path.clone()),
            app.and_then(|app| app.entry().icon.clone()),
        )
    });
    let size = pad.geometry(cx).icon;
    let edge = super::icons::edge(cx, pad);
    let icon = icon::load(icon_name.as_deref(), edge).map(|pixels| DragIcon {
        width: pixels.edge,
        height: pixels.edge,
        rgba: pixels.rgba,
        logical_width: size as u32,
        logical_height: size as u32,
        hotspot: grip,
    });
    let mut items = vec![(PAD_MIME.to_owned(), serialize(item))];
    if let Some(path) = &path {
        items.push((URI_LIST.to_owned(), uri_list(path)));
    }
    if !cx.start_drag(DragSource { items, icon, ..DragSource::default() }, move |cx, outcome| {
        land(cx, pad, outcome)
    }) {
        return;
    }
    let gap = pad
        .library
        .with(cx, |library| library.layout().locate(item));
    pad.drag.set(
        cx,
        DragState {
            lifted: Some(item.clone()),
            from_folder: match origin {
                Origin::Grid => None,
                Origin::Folder(folder) => Some(folder),
            },
            gap,
            dock_band: measure_dock(),
            ..DragState::default()
        },
    );
    pad.highlight.set(cx, None);
    pad.refresh(cx);
}

pub fn over(cx: &mut Cx, pad: Pad, event: &mut DragOverEvent) {
    if pad.is_searching(cx) {
        return;
    }
    let own = pad.drag.with(cx, DragState::is_active);
    let mime = match (own, event.offers(PAD_MIME), event.offers(URI_LIST)) {
        (true, true, _) => PAD_MIME,
        (false, _, true) => URI_LIST,
        _ => return,
    };
    event.accept(mime);
    if pad.folder.get(cx).is_some() {
        return;
    }
    if own {
        edge_flip(cx, pad, event.position);
        step_aside(cx, pad, event.position);
    }
    let (gap, onto) = target(cx, pad, event.position);
    let onto = onto.filter(|_| own);
    let changed = pad
        .drag
        .with(cx, |drag| drag.gap != gap.or(drag.gap) || drag.onto != onto);
    if changed {
        pad.drag.update(cx, |drag| {
            drag.gap = gap.or(drag.gap);
            drag.onto = onto;
        });
        pad.refresh(cx);
    }
}

pub fn leave(cx: &mut Cx, pad: Pad) {
    if pad.drag.with(cx, |drag| {
        drag.onto.is_some() || drag.flip.is_some() || drag.aside.is_some()
    }) {
        pad.drag.update(cx, |drag| {
            drag.onto = None;
            drag.flip = None;
            drag.aside = None;
        });
    }
    if !pad.drag.with(cx, DragState::is_active) {
        pad.drag.update(cx, |drag| drag.gap = None);
    }
}

pub fn drop(cx: &mut Cx, pad: Pad, event: &DropEvent) {
    if event.mime_type == URI_LIST {
        file_back(cx, pad, file_paths(&event.data).collect());
        return;
    }
    if event.mime_type != PAD_MIME {
        return;
    }
    let drag = pad.drag.get(cx);
    pad.drag.set(cx, DragState::default());
    let Some(lifted) = drag.lifted else {
        return;
    };
    switch_to_manual(cx, pad);
    pad.edit_library(cx, |library| {
        let target_slot = drag
            .onto
            .as_ref()
            .and_then(|onto| library.layout().locate(onto));
        match (drag.from_folder, lifted.app(), target_slot) {
            (Some(folder), Some(app), Some(onto)) => {
                let end = library.pages().get(onto.page).map_or(0, Vec::len);
                library.take_out_of_folder(folder, app, Slot::new(onto.page, end));
                if let (Some(from), Some(onto)) = (
                    library.layout().locate(&lifted),
                    library.layout().locate(drag.onto.as_ref()?),
                ) {
                    library.merge(from, onto);
                }
            }
            (Some(folder), Some(app), None) => {
                let last_page = library.pages().len().saturating_sub(1);
                let to = drag.gap.unwrap_or(Slot::new(last_page, usize::MAX));
                library.take_out_of_folder(folder, app, to);
            }
            (None, _, Some(onto)) => {
                let from = library.layout().locate(&lifted)?;
                library.merge(from, onto);
            }
            (None, _, None) => {
                let from = library.layout().locate(&lifted)?;
                library.move_item(from, drag.gap?);
            }
            (Some(_), None, _) => {}
        }
        Some(())
    });
}

/// Apps dropped in from elsewhere — an icon dragged off the desktop — move to
/// where they were let go. Apps the pad does not list are left alone.
fn file_back(cx: &mut Cx, pad: Pad, paths: Vec<PathBuf>) {
    let gap = pad.drag.with(cx, |drag| drag.gap);
    pad.drag.set(cx, DragState::default());
    let Some(gap) = gap else {
        return;
    };
    switch_to_manual(cx, pad);
    pad.edit_library(cx, |library| {
        for path in &paths {
            let Some(id) = library
                .apps()
                .find(|app| app.entry().path == *path)
                .map(|app| app.id().clone())
            else {
                continue;
            };
            if let Some(from) = library.layout().locate(&Item::App(id)) {
                library.move_item(from, gap);
            }
        }
        Some(())
    });
}

/// Holding a lifted app at the top edge hides the pad, keeping the drag, so it
/// can land on the desktop below.
fn step_aside(cx: &mut Cx, pad: Pad, position: Point) {
    let resting = pad.drag.with(cx, |drag| drag.aside);
    if position.y >= EDGE {
        if resting.is_some() {
            pad.drag.update(cx, |drag| drag.aside = None);
        }
        return;
    }
    if resting.is_some() {
        return;
    }
    let since = Instant::now();
    pad.drag.update(cx, |drag| drag.aside = Some(since));
    cx.spawn(
        async move { tokio::time::sleep(ASIDE_DELAY).await },
        move |cx, ()| {
            if pad.drag.with(cx, |drag| drag.aside == Some(since)) {
                pad.drag.update(cx, |drag| drag.aside = None);
                visibility::set(cx, pad, false);
            }
        },
    );
}

fn land(cx: &mut Cx, pad: Pad, outcome: DragOutcome) {
    log::debug!("drag ended: {outcome:?}");
    if pad.drag.with(cx, DragState::is_active) {
        pad.drag.set(cx, DragState::default());
        pad.refresh(cx);
    }
}

fn target(cx: &mut Cx, pad: Pad, position: Point) -> (Option<Slot>, Option<Item>) {
    let geometry = pad.geometry(cx);
    let browsing = geometry.browsing;
    let page = pad.page.get(cx);
    let scroll = pad.scroll.get(cx);
    let capacity = geometry.capacity();
    let point = match browsing {
        Browsing::Paged => position,
        Browsing::Scroll => Point::new(position.x, position.y + scroll),
    };
    let tiles = pad.tiles.get(cx);
    let shown: Vec<_> = tiles
        .iter()
        .filter(|tile| browsing == Browsing::Scroll || tile.page == page)
        .collect();
    let Some(found) = geometry.target_at(point, shown.len()) else {
        return (None, None);
    };
    let ghost = shown.iter().position(|tile| tile.ghost);
    match found {
        Target::Onto(index) => {
            let tile = shown[index];
            let mergeable = !tile.ghost
                && pad.drag.with(cx, |drag| {
                    drag.lifted
                        .as_ref()
                        .is_some_and(|lifted| lifted.app().is_some())
                });
            (None, mergeable.then(|| tile.item.clone()))
        }
        Target::Before(index) => {
            let adjusted = index - usize::from(ghost.is_some_and(|ghost| ghost < index));
            let slot = match browsing {
                Browsing::Paged => Slot::new(page, adjusted),
                Browsing::Scroll => Slot::new(adjusted / capacity, adjusted % capacity),
            };
            (Some(slot), None)
        }
    }
}

fn edge_flip(cx: &mut Cx, pad: Pad, position: Point) {
    let width = pad.surface.with(cx, |surface| surface.width);
    let paged = pad
        .settings
        .with(cx, |settings| settings.browsing == Browsing::Paged);
    let direction = if !paged {
        0
    } else if position.x < EDGE {
        -1
    } else if position.x > width - EDGE {
        1
    } else {
        0
    };
    let pending = pad
        .drag
        .with(cx, |drag| drag.flip.map(|(direction, _)| direction));
    if direction == 0 {
        if pending.is_some() {
            pad.drag.update(cx, |drag| drag.flip = None);
        }
        return;
    }
    if pending == Some(direction) {
        return;
    }
    let generation = pad.drag.with(cx, |drag| {
        drag.flip.map_or(0, |(_, generation)| generation) + 1
    });
    pad.drag
        .update(cx, |drag| drag.flip = Some((direction, generation)));
    cx.spawn(
        async move { tokio::time::sleep(FLIP_DELAY).await },
        move |cx, ()| {
            let current = pad
                .drag
                .with(cx, |drag| drag.flip == Some((direction, generation)));
            if !current {
                return;
            }
            let last = pad.page_count.get(cx);
            let page = pad.page.get(cx);
            if direction > 0 && page + 1 >= last {
                let lifted_alone = pad.tiles.with(cx, |tiles| {
                    tiles
                        .iter()
                        .filter(|tile| tile.page == page)
                        .all(|tile| tile.ghost)
                });
                if !lifted_alone {
                    pad.drag
                        .update(cx, |drag| drag.gap = Some(Slot::new(last, 0)));
                    pad.refresh(cx);
                }
            }
            paging::turn(cx, pad, direction);
            pad.drag.update(cx, |drag| drag.flip = None);
        },
    );
}

fn switch_to_manual(cx: &mut Cx, pad: Pad) {
    if pad
        .settings
        .with(cx, |settings| settings.sort != SortMode::Manual)
    {
        pad.update_settings(cx, |settings| settings.sort = SortMode::Manual);
    }
}

fn serialize(item: &Item) -> Vec<u8> {
    match item {
        Item::App(id) => format!("app:{id}").into_bytes(),
        Item::Folder(id) => format!("folder:{}", id.0).into_bytes(),
    }
}
