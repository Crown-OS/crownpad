//! Captures every launchpad state for the CrownOS-Design Figma file. Run both appearances with
//! `../crownui/capture/scripts/capture-app.sh .`; output lands in `$CROWN_DESIGN_DIR`.

mod fixtures;
mod session;
mod tasks;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use capture::Carrier;
use crownpad::settings::{Browsing, PadSettings};
use crownui::{
    ext::{Key, Modifiers, NamedKey},
    prelude::Point,
};

use fixtures::{
    CREATIVE_FOLDER, Fixtures, GAMES_FOLDER, UTILITIES_FOLDER, folder_index, index_of, layout,
};
use session::{CTRL, Glass, Icons, MenuRow, Pad, cell_center};

const GRID: &str = "Apps grid";
const GLASS: &str = "Glass";
const SEARCH: &str = "Search";
const SELECTION: &str = "Selection";
const FOLDERS: &str = "Folders";
const MENUS: &str = "Menus";
const DIALOGS: &str = "Dialogs";
const DRAG: &str = "Drag & drop";
const FALLBACK: &str = "Loading & empty";

/// A point in the margin left of the grid: empty space on the pad.
const EMPTY_SPACE: Point = Point::new(180.0, 560.0);
/// The first app in an open six-column folder tray, centred on the output.
const FIRST_FOLDER_MEMBER: Point = Point::new(540.0, 480.0);
/// "Rename" in an app's menu: after Open, Show in Files and Add to Dock.
const APP_RENAME_ROW: MenuRow = MenuRow::Nth(3);
/// "Rename" in a folder's menu, after Open.
const FOLDER_RENAME_ROW: MenuRow = MenuRow::Nth(1);
/// Music's cell once Camera, six places before it, is lifted out of the grid.
const MUSIC_WHILE_CAMERA_LIFTED: usize = 10;

fn slot(stem: &str) -> Result<Point> {
    index_of(stem)
        .map(cell_center)
        .with_context(|| format!("{stem} is not on the first page"))
}

fn folder_slot(id: crownpad::library::FolderId) -> Result<Point> {
    folder_index(id)
        .map(cell_center)
        .with_context(|| format!("folder {id:?} is not on the first page"))
}

struct Capturer<'a> {
    fixtures: &'a Fixtures,
    carrier: Carrier,
}

impl Capturer<'_> {
    fn pad(&self) -> Result<Pad> {
        Pad::open(
            self.fixtures.settings(),
            layout(),
            Glass::Composited,
            Icons::Decoded,
        )
    }

    fn pad_with(&self, change: impl FnOnce(&mut PadSettings)) -> Result<Pad> {
        let mut settings = self.fixtures.settings();
        change(&mut settings);
        Pad::open(settings, layout(), Glass::Composited, Icons::Decoded)
    }

    fn grid(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        pad.capture(&mut self.carrier, GRID, "First page", "The launchpad over the desktop: search field, a 5×7 page of apps and folders, page dots")?;
        pad.named(NamedKey::ArrowRight, CTRL);
        pad.capture(
            &mut self.carrier,
            GRID,
            "Second page",
            "Ctrl+→ (or a swipe, wheel or dot) turned to the partly filled second page",
        )?;
        drop(pad);

        let mut pad = self.pad_with(|settings| settings.browsing = Browsing::Scroll)?;
        pad.named(NamedKey::ArrowDown, CTRL);
        pad.capture(
            &mut self.carrier,
            GRID,
            "Scroll browsing",
            "Browse: Scroll — one continuous grid, here scrolled to its end (Ctrl+↓); no page dots",
        )?;
        drop(pad);

        let mut pad = self.pad_with(|settings| {
            settings.app_labels = false;
            settings.folder_labels = false;
        })?;
        pad.capture(
            &mut self.carrier,
            GRID,
            "Icons only",
            "App and folder names turned off in Settings",
        )?;
        Ok(())
    }

    fn glass(&mut self) -> Result<()> {
        let mut pad = Pad::open(
            self.fixtures.settings(),
            layout(),
            Glass::Solid,
            Icons::Decoded,
        )?;
        pad.capture(
            &mut self.carrier,
            GLASS,
            "Without blur",
            "The compositor cannot blur: the backdrop is drawn as a nearly solid tint",
        )?;
        Ok(())
    }

    fn search(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        pad.type_text("te");
        pad.expect("te")?;
        pad.capture(&mut self.carrier, SEARCH, "Search results", "Typing filters every app (folder members too); the best match is highlighted for Enter")?;
        pad.type_text("lescope");
        pad.capture(
            &mut self.carrier,
            SEARCH,
            "No matches",
            "Nothing matches \u{201c}telescope\u{201d}: an empty grid under the query",
        )?;
        Ok(())
    }

    fn selection(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        pad.named(NamedKey::ArrowRight, Modifiers::NONE)
            .named(NamedKey::ArrowRight, Modifiers::NONE)
            .named(NamedKey::ArrowDown, Modifiers::NONE);
        pad.capture(
            &mut self.carrier,
            SELECTION,
            "Keyboard highlight",
            "Arrow keys move a highlight halo through the grid",
        )?;
        drop(pad);

        let mut pad = self.pad()?;
        let start = Point::new(250.0, 300.0);
        let end = cell_center(16);
        pad.press_and_move(start, end, CTRL);
        pad.capture(
            &mut self.carrier,
            SELECTION,
            "Selection band",
            "Ctrl-drag from empty space draws a band over the apps it will select",
        )?;
        pad.release(end);
        pad.capture(
            &mut self.carrier,
            SELECTION,
            "Multiple selection",
            "Apps under the band (or Ctrl-clicked) wear the selection halo",
        )?;
        pad.right_click_at(cell_center(9));
        pad.expect("New Folder with Selection")?;
        pad.capture(
            &mut self.carrier,
            MENUS,
            "Selection menu",
            "Right-click on a selected app: open, group, hide or deselect the whole selection",
        )?;
        Ok(())
    }

    fn folders(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        pad.click_at(folder_slot(UTILITIES_FOLDER)?, Modifiers::NONE);
        pad.expect("Calculator")?;
        pad.capture(
            &mut self.carrier,
            FOLDERS,
            "Open folder",
            "A folder opens as a glass tray over the dimmed grid",
        )?;
        pad.right_click_at(FIRST_FOLDER_MEMBER);
        pad.expect("Remove from Folder")?;
        pad.capture(
            &mut self.carrier,
            MENUS,
            "App in folder menu",
            "Right-click on an app inside a folder",
        )?;
        drop(pad);

        let mut pad = self.pad()?;
        pad.click_at(folder_slot(GAMES_FOLDER)?, Modifiers::NONE);
        pad.expect("Mahjongg")?;
        pad.capture(
            &mut self.carrier,
            FOLDERS,
            "Large folder",
            "Eleven games wrap onto a second row of the tray",
        )?;
        drop(pad);

        let mut pad = self.pad()?;
        pad.right_click_at(folder_slot(CREATIVE_FOLDER)?);
        pad.expect("Dissolve Folder")?;
        pad.capture(
            &mut self.carrier,
            MENUS,
            "Folder menu",
            "Right-click on a folder: open, rename, dissolve or move it to a page",
        )?;
        pad.click_menu_row(FOLDER_RENAME_ROW)?;
        pad.expect("Blender")?;
        pad.capture(
            &mut self.carrier,
            FOLDERS,
            "Renaming folder",
            "The tray title turns into a name field (also by clicking the title)",
        )?;
        Ok(())
    }

    fn menus(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        pad.right_click_at(slot("crownos-files")?);
        pad.expect("Move to Folder")?;
        pad.capture(&mut self.carrier, MENUS, "App menu", "Right-click on an app: open, reveal, dock, rename, move to a folder or page, hide, uninstall")?;
        pad.click_menu_row(APP_RENAME_ROW)?;
        pad.expect("Rename App")?;
        pad.capture(
            &mut self.carrier,
            DIALOGS,
            "Rename app",
            "A custom name for the app, kept in the layout",
        )?;
        drop(pad);

        let mut pad = self.pad()?;
        pad.right_click_at(EMPTY_SPACE);
        pad.expect("Sort By")?;
        pad.capture(
            &mut self.carrier,
            MENUS,
            "Background menu",
            "Right-click on empty space: sorting, browsing, reorganizing, hidden apps and Settings",
        )?;
        Ok(())
    }

    fn dialogs(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        pad.key(Key::Character(','), Some(",".to_owned()), CTRL);
        pad.expect("Launchpad Settings")?;
        pad.capture(&mut self.carrier, DIALOGS, "Settings", "Ctrl+, (or Settings… in the background menu): grid size, browsing, backdrop and labels")?;
        drop(pad);

        let mut pad = self.pad()?;
        pad.right_click_at(slot("com.spotify.Client")?);
        pad.click_menu_row(MenuRow::Last)?;
        pad.wait_for("Uninstall Spotify?")?;
        pad.capture(
            &mut self.carrier,
            DIALOGS,
            "Uninstall confirmation",
            "Uninstall… on a Flatpak app names the package it removes",
        )?;
        drop(pad);

        let mut pad = self.pad()?;
        pad.right_click_at(slot("dev.zed.Zed")?);
        pad.click_menu_row(MenuRow::Last)?;
        pad.wait_for("be uninstalled here")?;
        pad.capture(
            &mut self.carrier,
            DIALOGS,
            "Cannot uninstall",
            "An app no known package manager owns cannot be removed here",
        )?;
        Ok(())
    }

    fn drag(&mut self) -> Result<()> {
        let mut pad = self.pad()?;
        let lifted = slot("crownos-camera")?;
        let gap = cell_center(16);
        let gap = Point::new(gap.x - 70.0, gap.y);
        pad.press_and_move(
            lifted,
            Point::new(lifted.x + 40.0, lifted.y + 60.0),
            Modifiers::NONE,
        );
        pad.carry_over(gap);
        pad.capture(&mut self.carrier, DRAG, "Reordering", "Camera lifted and carried (the compositor draws its icon): the grid opens a gap where it will land")?;
        pad.carry_over(cell_center(MUSIC_WHILE_CAMERA_LIFTED));
        pad.capture(
            &mut self.carrier,
            DRAG,
            "Making a folder",
            "Held over the middle of Music, the target app swells: dropping makes a folder",
        )?;
        Ok(())
    }

    fn fallbacks(&mut self) -> Result<()> {
        let mut pad = Pad::open(
            self.fixtures.settings(),
            layout(),
            Glass::Composited,
            Icons::Loading,
        )?;
        pad.capture(
            &mut self.carrier,
            FALLBACK,
            "Icons loading",
            "Placeholder squircles until the icons are decoded",
        )?;
        drop(pad);

        let mut pad = self.pad_with(|settings| settings.app_sources.clear())?;
        pad.expect_absent("Files")?;
        pad.capture(
            &mut self.carrier,
            FALLBACK,
            "No apps",
            "Nothing installed: the search field over an empty pad",
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    if std::env::var_os("CROWN_CONFIG_DIR").is_none() {
        bail!("run through capture-app.sh: the launchpad saves its settings into CROWN_CONFIG_DIR");
    }
    let design_dir = std::env::var_os("CROWN_DESIGN_DIR")
        .map(PathBuf::from)
        .context("CROWN_DESIGN_DIR is not set")?;
    let fixtures = Fixtures::new(&design_dir.join(".fixtures"))?;
    let mut capturer = Capturer {
        fixtures: &fixtures,
        carrier: Carrier::new(
            "CrownPad",
            "crownpad: the fullscreen launchpad overlay (1920×1080 output) with its app grid, folders, search, selection, menus, dialogs and drag states, captured headlessly from the current source.",
        ),
    };
    capturer.grid()?;
    capturer.glass()?;
    capturer.search()?;
    capturer.selection()?;
    capturer.folders()?;
    capturer.menus()?;
    capturer.dialogs()?;
    capturer.drag()?;
    capturer.fallbacks()?;
    let path = capturer.carrier.write(None)?;
    println!("{}", path.display());
    capture::finish()
}
