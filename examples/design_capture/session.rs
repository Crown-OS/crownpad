//! One headless launchpad on a 1920×1080 output, shown through its IPC channel and driven by
//! real pointer and keyboard input.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use capture::{CaptureApp, Carrier};
use crownpad::{
    catalog::{self, DesktopId},
    icon::Pixels,
    ipc::{Remote, VisibilityReport},
    library::{Layout, Library},
    settings::PadSettings,
};
use crownui::{
    ext::{Key, KeyboardEvent, Modifiers, NamedKey, PointerEvent},
    prelude::{Capabilities, DndActions, Point},
};
use input::{PhysicalKey, PointerButton};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use windowing::WindowEvent;

use crate::tasks::{Pump, threaded};

pub const OUTPUT: (f32, f32) = (1920.0, 1080.0);
const LAYOUT_TIME: u64 = 1_762_819_200;
const FRAME_MS: u64 = 16;
const POLL: Duration = Duration::from_millis(8);
const QUIET: Duration = Duration::from_millis(350);
const MOST_PUMPING: Duration = Duration::from_secs(15);
const DRAG_STEPS: usize = 8;
pub const PAD_MIME: &str = "application/x-crownpad-item";
const URI_LIST: &str = "text/uri-list";

/// Mirrors the pad's grid geometry for a 5×7 grid on the output.
const COLUMNS: usize = 7;
const CELL: (f32, f32) = (168.0, 180.0);
const GRID_ORIGIN: (f32, f32) = (372.0, 108.0);

pub const CTRL: Modifiers = Modifiers {
    ctrl_key: true,
    ..Modifiers::NONE
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Glass {
    /// The compositor blurs the desktop behind the pad, as on CrownOS.
    Composited,
    /// No compositor blur: the nearly solid backdrop.
    Solid,
}

const MENU_ROW: &str = "kit_menu_row";

#[derive(Debug, Clone, Copy)]
pub enum MenuRow {
    Nth(usize),
    Last,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icons {
    Decoded,
    /// Icon decoding never finishes: every tile shows its placeholder.
    Loading,
}

type IconBatch = Vec<(DesktopId, Pixels, Option<f32>)>;

pub struct Pad {
    pub app: CaptureApp,
    pump: Pump,
    icons: Icons,
    _visibility: UnboundedSender<bool>,
}

/// The centre of the first-page grid cell at `index`.
pub fn cell_center(index: usize) -> Point {
    let (row, column) = (index / COLUMNS, index % COLUMNS);
    Point::new(
        GRID_ORIGIN.0 + (column as f32 + 0.5) * CELL.0,
        GRID_ORIGIN.1 + (row as f32 + 0.5) * CELL.1,
    )
}

impl Pad {
    pub fn open(settings: PadSettings, layout: Layout, glass: Glass, icons: Icons) -> Result<Self> {
        crownconfig::save_section(PadSettings::SECTION, &settings)
            .context("save the fixture launchpad settings")?;
        let library = Library::new(
            catalog::scan(&settings.app_sources),
            layout,
            settings.grid().capacity(),
            LAYOUT_TIME,
        );
        let (visibility, requests) = unbounded_channel();
        let remote = Remote {
            requests,
            report: VisibilityReport::default(),
        };
        let mut app = capture::mount_app(
            crownpad::root(settings, library, remote),
            &crownpad::app_config(),
            Some(OUTPUT),
        )?;
        let composited = glass == Glass::Composited;
        app.ui().set_platform_capabilities(Capabilities {
            materials: composited,
            layer_shell: true,
            ..Capabilities::default()
        });
        app.ui().set_emulate_materials(composited);
        let (executor, pump) = threaded()?;
        app.ui().set_task_executor(executor);
        let mut pad = Self {
            app,
            pump,
            icons,
            _visibility: visibility.clone(),
        };
        pad.pump();
        visibility
            .send(true)
            .context("the launchpad stopped listening")?;
        pad.pump();
        pad.expect("Search")?;
        Ok(pad)
    }

    /// Hands finished background work (scans, icon decoding, lookups) back to the pad until it
    /// has been quiet for a while, then settles the animations.
    pub fn pump(&mut self) -> &mut Self {
        let started = Instant::now();
        let mut last_finished = started;
        while started.elapsed() < MOST_PUMPING && last_finished.elapsed() < QUIET {
            for (id, output) in self.pump.drain() {
                last_finished = Instant::now();
                if self.icons == Icons::Loading && (*output).is::<IconBatch>() {
                    continue;
                }
                self.app.ui().complete_task(id, output);
            }
            self.app.advance_ms(FRAME_MS);
            std::thread::sleep(POLL);
        }
        self.app.settle();
        self
    }

    /// Fails unless the semantic tree mentions `needle`: the drive reached its state.
    pub fn expect(&mut self, needle: &str) -> Result<&mut Self> {
        let snapshot = self.app.semantics_snapshot();
        if !snapshot.contains(needle) {
            bail!("expected {needle:?} on screen, found:\n{snapshot}");
        }
        Ok(self)
    }

    /// Keeps pumping until `needle` shows up: for slow lookups such as the package owner query.
    pub fn wait_for(&mut self, needle: &str) -> Result<&mut Self> {
        let started = Instant::now();
        while started.elapsed() < MOST_PUMPING && !self.app.semantics_snapshot().contains(needle) {
            self.pump();
        }
        self.expect(needle)
    }

    pub fn expect_absent(&mut self, needle: &str) -> Result<&mut Self> {
        let snapshot = self.app.semantics_snapshot();
        if snapshot.contains(needle) {
            bail!("expected {needle:?} gone, found:\n{snapshot}");
        }
        Ok(self)
    }

    fn pointer(
        &mut self,
        at: Point,
        button: Option<PointerButton>,
        modifiers: Modifiers,
    ) -> PointerEvent {
        PointerEvent {
            button,
            modifiers,
            timestamp: self.app.inline().now(),
            ..PointerEvent::mouse(at)
        }
    }

    pub fn click_at(&mut self, at: Point, modifiers: Modifiers) -> &mut Self {
        self.app.hold_modifiers(modifiers);
        let down = self.pointer(at, Some(PointerButton::Primary), modifiers);
        self.app.deliver(WindowEvent::PointerDown(down));
        let up = self.pointer(at, Some(PointerButton::Primary), modifiers);
        self.app.deliver(WindowEvent::PointerUp(up));
        self.app.hold_modifiers(Modifiers::NONE);
        self.pump()
    }

    pub fn right_click_at(&mut self, at: Point) -> &mut Self {
        let down = self.pointer(at, Some(PointerButton::Secondary), Modifiers::NONE);
        self.app.deliver(WindowEvent::PointerDown(down));
        let up = self.pointer(at, Some(PointerButton::Secondary), Modifiers::NONE);
        self.app.deliver(WindowEvent::PointerUp(up));
        self.pump()
    }

    /// Clicks the open menu's action row picked by `row` (headers and separators skipped); the
    /// whole pad is one accessible button, so rows are found by layout rather than by name.
    pub fn click_menu_row(&mut self, row: MenuRow) -> Result<&mut Self> {
        let rows: Vec<Point> = self
            .app
            .ui()
            .inspect_layout()
            .nodes
            .into_iter()
            .filter(|node| {
                node.path
                    .rsplit(" > ")
                    .next()
                    .is_some_and(|last| last.starts_with(MENU_ROW))
            })
            .map(|node| {
                let [x, y, width, height] = node.rect;
                Point::new(x + width / 2.0, y + height / 2.0)
            })
            .collect();
        let picked = match row {
            MenuRow::Nth(index) => rows.get(index),
            MenuRow::Last => rows.last(),
        };
        let at = *picked.with_context(|| format!("no menu row {row:?} among {}", rows.len()))?;
        Ok(self.click_at(at, Modifiers::NONE))
    }

    /// Presses the primary button at `from` and moves to `to` without releasing.
    pub fn press_and_move(&mut self, from: Point, to: Point, modifiers: Modifiers) -> &mut Self {
        self.app.hold_modifiers(modifiers);
        let down = self.pointer(from, Some(PointerButton::Primary), modifiers);
        self.app.deliver(WindowEvent::PointerDown(down));
        for step in 1..=DRAG_STEPS {
            let share = step as f32 / DRAG_STEPS as f32;
            let at = Point::new(
                from.x + (to.x - from.x) * share,
                from.y + (to.y - from.y) * share,
            );
            self.app.advance_ms(FRAME_MS);
            let moved = self.pointer(at, None, modifiers);
            self.app.deliver(WindowEvent::PointerMove(moved));
        }
        self.pump()
    }

    pub fn release(&mut self, at: Point) -> &mut Self {
        let up = self.pointer(at, Some(PointerButton::Primary), Modifiers::NONE);
        self.app.deliver(WindowEvent::PointerUp(up));
        self.app.hold_modifiers(Modifiers::NONE);
        self.pump()
    }

    /// The pad's own drag (an app it lifted) hovers over `at`.
    pub fn carry_over(&mut self, at: Point) -> &mut Self {
        self.app
            .drag_enter(at.x, at.y, &[PAD_MIME, URI_LIST], DndActions::COPY_MOVE);
        self.app.drag_over(at.x, at.y);
        self.pump()
    }

    pub fn key(&mut self, key: Key, text: Option<String>, modifiers: Modifiers) -> &mut Self {
        self.app.deliver(WindowEvent::KeyDown(KeyboardEvent {
            key,
            code: PhysicalKey(0),
            text,
            repeat: false,
            modifiers,
        }));
        self.pump()
    }

    pub fn named(&mut self, key: NamedKey, modifiers: Modifiers) -> &mut Self {
        self.key(Key::Named(key), None, modifiers)
    }

    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for character in text.chars() {
            self.key(
                Key::Character(character),
                Some(character.to_string()),
                Modifiers::NONE,
            );
        }
        self
    }

    pub fn capture(
        &mut self,
        carrier: &mut Carrier,
        group: &str,
        name: &str,
        description: &str,
    ) -> Result<()> {
        carrier.capture_state(&mut self.app, group, name, description)?;
        Ok(())
    }
}
