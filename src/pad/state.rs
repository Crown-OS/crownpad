use std::{
    collections::{HashMap, HashSet},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crownui::{
    ext::{ImageId, Modifiers},
    kit::{Theme, ThemeMode},
    prelude::{Capabilities, Cx, Runtime, Signal, Size, Store},
};

use super::{
    dialog::Dialog,
    drag::DragState,
    geometry::Geometry,
    gesture::Gesture,
    icons::IconLoads,
    menu::Menu,
    motion::Motion,
    paging::WheelPaging,
    tiles::{self, Tile},
    visibility::HIDDEN,
};
use crate::{
    catalog::{self, AppEntry, DesktopId},
    ipc::VisibilityReport,
    launch,
    library::{FolderId, Item, Layout, Library},
    settings::PadSettings,
    util::non_default_gpu::GpuPreference,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editing {
    App(DesktopId),
    Folder(FolderId),
}

#[derive(Clone, Copy)]
pub struct Pad {
    pub settings: Signal<PadSettings>,
    pub library: Store<Library>,
    pub revision: Signal<u64>,
    pub tiles: Signal<Vec<Tile>>,
    pub page_count: Signal<usize>,
    pub icons: Signal<HashMap<DesktopId, ImageId>>,
    pub hues: Store<HashMap<DesktopId, f32>>,
    pub loads: Store<IconLoads>,
    pub surface: Signal<Size>,
    pub scale: Signal<f32>,
    pub capabilities: Signal<Capabilities>,
    pub theme: Signal<Theme>,
    pub reveal: Signal<f32>,
    pub motion: Store<Motion>,
    pub shown: Signal<bool>,
    pub page: Signal<usize>,
    pub scroll: Signal<f32>,
    pub query: Signal<String>,
    pub highlight: Signal<Option<Item>>,
    pub selection: Signal<HashSet<Item>>,
    pub folder: Signal<Option<FolderId>>,
    pub drag: Signal<DragState>,
    pub menu: Signal<Option<Menu>>,
    pub dialog: Signal<Option<Dialog>>,
    pub gesture: Signal<Gesture>,
    pub resume_page: Store<Option<usize>>,
    pub editing: Signal<Option<Editing>>,
    pub edit_text: Signal<String>,
    pub modifiers: Store<Modifiers>,
    pub wheel: Store<WheelPaging>,
    pub report: Store<VisibilityReport>,
    pub hidden_since: Store<Option<Instant>>,
}

impl Pad {
    pub fn new(
        cx: &mut Cx,
        settings: PadSettings,
        library: Library,
        report: VisibilityReport,
        theme: Signal<Theme>,
    ) -> Self {
        Self {
            settings: cx.signal(settings),
            library: cx.store(library),
            revision: cx.signal(0),
            tiles: cx.signal(Vec::new()),
            page_count: cx.signal(1),
            icons: cx.signal(HashMap::new()),
            hues: cx.store(HashMap::new()),
            loads: cx.store(IconLoads::default()),
            surface: cx.signal(Size::ZERO),
            scale: cx.scale_factor(),
            capabilities: cx.platform_capabilities(),
            theme,
            reveal: cx.signal(0.0),
            motion: cx.store(Motion::default()),
            shown: cx.signal(false),
            page: cx.signal(0),
            scroll: cx.signal(0.0),
            query: cx.signal(String::new()),
            highlight: cx.signal(None),
            selection: cx.signal(HashSet::new()),
            folder: cx.signal(None),
            drag: cx.signal(DragState::default()),
            menu: cx.signal(None),
            dialog: cx.signal(None),
            gesture: cx.signal(Gesture::Idle),
            resume_page: cx.store(None),
            editing: cx.signal(None),
            edit_text: cx.signal(String::new()),
            modifiers: cx.store(Modifiers::default()),
            wheel: cx.store(WheelPaging::default()),
            report: cx.store(report),
            hidden_since: cx.store(None),
        }
    }

    pub fn geometry(self, runtime: &mut Runtime) -> Geometry {
        let surface = self.surface.get(runtime);
        let (shape, browsing) = self
            .settings
            .with(runtime, |settings| (settings.grid(), settings.browsing));
        Geometry::new(surface, shape, browsing)
    }

    pub fn dark(self, runtime: &mut Runtime) -> bool {
        self.theme
            .with(runtime, |theme| theme.mode == ThemeMode::Dark)
    }

    pub fn on_screen(self, runtime: &mut Runtime) -> bool {
        self.shown.get(runtime) || self.reveal.get(runtime) > HIDDEN
    }

    pub fn is_searching(self, runtime: &mut Runtime) -> bool {
        self.query.with(runtime, |query| !query.trim().is_empty())
    }

    pub fn edit_library<R>(self, cx: &mut Cx, change: impl FnOnce(&mut Library) -> R) -> R {
        let result = self.library.update(cx, change);
        self.revision.update(cx, |revision| *revision += 1);
        self.refresh(cx);
        self.save(cx);
        result
    }

    pub fn refresh(self, cx: &mut Cx) {
        tiles::arrange(cx, self);
        super::icons::sync(cx, self);
    }

    pub fn update_settings(self, cx: &mut Cx, change: impl FnOnce(&mut PadSettings)) {
        let mut settings = self.settings.get(cx);
        change(&mut settings);
        if let Err(error) = crownconfig::save_section(PadSettings::SECTION, &settings) {
            log::warn!("save the launchpad settings: {error}");
        }
        self.adopt_settings(cx, settings);
    }

    pub fn adopt_settings(self, cx: &mut Cx, settings: PadSettings) {
        let capacity = settings.grid().capacity();
        let order = settings.sort.order();
        self.settings.set(cx, settings);
        self.edit_library(cx, |library| {
            library.set_capacity(capacity);
            if let Some(order) = order {
                library.arrange(order);
            }
        });
    }

    pub fn rescan(self, cx: &mut Cx) {
        let sources = self
            .settings
            .with(cx, |settings| settings.app_sources.clone());
        let scan = async move {
            tokio::task::spawn_blocking(move || {
                let mut dirs = catalog::default_dirs();
                dirs.extend(sources);
                catalog::scan(&dirs)
            })
            .await
            .unwrap_or_default()
        };
        cx.spawn(scan, move |cx, entries| {
            let order = self.settings.with(cx, |settings| settings.sort.order());
            self.edit_library(cx, |library| {
                library.refresh(entries, now());
                if let Some(order) = order {
                    library.arrange(order);
                }
            });
        });
    }

    pub fn save(self, cx: &mut Cx) {
        let saved = self.library.update(cx, |library| {
            library
                .take_dirty()
                .then(|| crownconfig::save_section(Layout::SECTION, library.layout()))
        });
        if let Some(Err(error)) = saved {
            log::warn!("save the launchpad layout: {error}");
        }
    }

    pub fn launch(self, cx: &mut Cx, id: &DesktopId) {
        self.start(cx, id, launch::launch);
    }

    pub fn launch_on_non_default_gpu(self, cx: &mut Cx, id: &DesktopId) {
        self.start(cx, id, |entry| {
            launch::launch_on(entry, GpuPreference::NonDefault)
        });
    }

    fn start(self, cx: &mut Cx, id: &DesktopId, spawn: impl FnOnce(&AppEntry)) {
        let entry = self
            .library
            .with(cx, |library| library.app(id).map(|app| app.entry().clone()));
        let Some(entry) = entry else {
            return;
        };
        spawn(&entry);
        self.library
            .update(cx, |library| library.record_launch(id, now()));
        super::visibility::set(cx, self, false);
    }

    pub fn dismiss_transient(self, cx: &mut Cx) {
        self.query.set(cx, String::new());
        self.highlight.set(cx, None);
        self.selection.set(cx, HashSet::new());
        self.folder.set(cx, None);
        self.menu.set(cx, None);
        self.dialog.set(cx, None);
        self.editing.set(cx, None);
        self.resume_page.update(cx, |resume| *resume = None);
        self.refresh(cx);
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
