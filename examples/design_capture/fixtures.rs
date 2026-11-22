//! A believable set of installed apps (desktop entries under the design dir, Colloid icons) and
//! the launchpad layout that arranges them, so no real install or layout ever shows.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crownpad::{
    catalog::DesktopId,
    library::{Folder, FolderId, Item, Layout},
    settings::PadSettings,
};

const ICONS: &str = "/usr/share/icons/Colloid/apps/scalable";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    /// The user's own applications directory: nothing a package manager owns.
    Home,
    /// A flatpak export, which crownpad can uninstall.
    Flatpak,
}

/// `(desktop file stem, name, Colloid icon, where it is installed)`.
type Entry = (&'static str, &'static str, &'static str, Source);

const APPS: &[Entry] = &[
    ("crownos-files", "Files", "org.gnome.Nautilus", Source::Home),
    ("firefox", "Firefox", "firefox", Source::Home),
    (
        "crownos-terminal",
        "Terminal",
        "org.gnome.Terminal",
        Source::Home,
    ),
    (
        "crownappstore",
        "App Store",
        "system-software-install",
        Source::Home,
    ),
    (
        "crownos-settings",
        "Settings",
        "preferences-system",
        Source::Home,
    ),
    (
        "crownos-process-monitor",
        "Process Monitor",
        "utilities-system-monitor",
        Source::Home,
    ),
    ("crownos-camera", "Camera", "cheese", Source::Home),
    (
        "crownos-preview",
        "Preview",
        "org.gnome.Loupe",
        Source::Home,
    ),
    (
        "org.gnome.Calendar",
        "Calendar",
        "org.gnome.Calendar",
        Source::Home,
    ),
    (
        "org.gnome.Weather",
        "Weather",
        "org.gnome.Weather",
        Source::Home,
    ),
    ("org.gnome.Maps", "Maps", "org.gnome.Maps", Source::Home),
    ("org.gnome.Music", "Music", "org.gnome.Music", Source::Home),
    (
        "org.gnome.Photos",
        "Photos",
        "org.gnome.Photos",
        Source::Home,
    ),
    (
        "code",
        "Visual Studio Code",
        "visual-studio-code",
        Source::Home,
    ),
    ("dev.zed.Zed", "Zed", "zed", Source::Home),
    (
        "md.obsidian.Obsidian",
        "Obsidian",
        "obsidian",
        Source::Flatpak,
    ),
    (
        "com.getpostman.Postman",
        "Postman",
        "postman",
        Source::Flatpak,
    ),
    ("docker-desktop", "Docker Desktop", "docker", Source::Home),
    ("figma-linux", "Figma", "figma-linux", Source::Home),
    (
        "org.mozilla.Thunderbird",
        "Thunderbird",
        "thunderbird",
        Source::Flatpak,
    ),
    (
        "org.telegram.desktop",
        "Telegram",
        "telegram",
        Source::Flatpak,
    ),
    (
        "com.discordapp.Discord",
        "Discord",
        "discord",
        Source::Flatpak,
    ),
    (
        "org.signal.Signal",
        "Signal",
        "signal-desktop",
        Source::Flatpak,
    ),
    ("com.slack.Slack", "Slack", "slack", Source::Flatpak),
    ("com.spotify.Client", "Spotify", "spotify", Source::Flatpak),
    (
        "libreoffice-writer",
        "LibreOffice Writer",
        "libreoffice-writer",
        Source::Home,
    ),
    (
        "libreoffice-calc",
        "LibreOffice Calc",
        "libreoffice-calc",
        Source::Home,
    ),
    (
        "libreoffice-impress",
        "LibreOffice Impress",
        "libreoffice-impress",
        Source::Home,
    ),
    (
        "org.gnome.Evince",
        "Document Viewer",
        "org.gnome.Evince",
        Source::Home,
    ),
    ("org.gnome.Notes", "Notes", "org.gnome.Notes", Source::Home),
    (
        "com.bitwarden.desktop",
        "Bitwarden",
        "bitwarden",
        Source::Flatpak,
    ),
    ("org.videolan.VLC", "VLC", "vlc", Source::Flatpak),
    (
        "com.obsproject.Studio",
        "OBS Studio",
        "com.obsproject.Studio",
        Source::Flatpak,
    ),
    ("chromium", "Chromium", "chromium", Source::Home),
    ("org.gnome.Boxes", "Boxes", "org.gnome.Boxes", Source::Home),
    (
        "org.gnome.Podcasts",
        "Podcasts",
        "org.gnome.Podcasts",
        Source::Flatpak,
    ),
    (
        "transmission-gtk",
        "Transmission",
        "transmission",
        Source::Home,
    ),
    (
        "org.gnome.Epiphany",
        "Web",
        "org.gnome.Epiphany",
        Source::Home,
    ),
    (
        "org.gnome.Contacts",
        "Contacts",
        "org.gnome.Contacts",
        Source::Home,
    ),
    (
        "org.gnome.Extensions",
        "Extensions",
        "org.gnome.Extensions",
        Source::Home,
    ),
];

const UTILITIES: &[Entry] = &[
    (
        "org.gnome.Calculator",
        "Calculator",
        "org.gnome.Calculator",
        Source::Home,
    ),
    (
        "org.gnome.clocks",
        "Clocks",
        "org.gnome.clocks",
        Source::Home,
    ),
    (
        "org.gnome.Characters",
        "Characters",
        "org.gnome.Characters",
        Source::Home,
    ),
    (
        "org.gnome.TextEditor",
        "Text Editor",
        "org.gnome.TextEditor",
        Source::Home,
    ),
    (
        "org.gnome.Console",
        "Console",
        "org.gnome.Console",
        Source::Home,
    ),
    (
        "org.gnome.baobab",
        "Disk Usage",
        "org.gnome.baobab",
        Source::Home,
    ),
    (
        "org.gnome.DiskUtility",
        "Disks",
        "org.gnome.DiskUtility",
        Source::Home,
    ),
    (
        "org.gnome.Screenshot",
        "Screenshot",
        "org.gnome.Screenshot",
        Source::Home,
    ),
    (
        "org.gnome.font-viewer",
        "Fonts",
        "org.gnome.font-viewer",
        Source::Home,
    ),
];

const CREATIVE: &[Entry] = &[
    ("org.blender.Blender", "Blender", "blender", Source::Flatpak),
    ("org.gimp.GIMP", "GIMP", "gimp", Source::Flatpak),
    (
        "org.inkscape.Inkscape",
        "Inkscape",
        "inkscape",
        Source::Flatpak,
    ),
    ("org.kde.krita", "Krita", "org.kde.krita", Source::Flatpak),
    (
        "org.kde.kdenlive",
        "Kdenlive",
        "org.kde.kdenlive",
        Source::Flatpak,
    ),
    ("darktable", "darktable", "darktable", Source::Home),
    ("audacity", "Audacity", "audacity", Source::Home),
];

const GAMES: &[Entry] = &[
    ("steam", "Steam", "steam", Source::Home),
    ("net.lutris.Lutris", "Lutris", "lutris", Source::Flatpak),
    (
        "com.heroicgameslauncher.hgl",
        "Heroic Games Launcher",
        "heroic",
        Source::Flatpak,
    ),
    (
        "minecraft-launcher",
        "Minecraft Launcher",
        "minecraft",
        Source::Home,
    ),
    (
        "org.libretro.RetroArch",
        "RetroArch",
        "retroarch",
        Source::Flatpak,
    ),
    (
        "net.supertuxkart.SuperTuxKart",
        "SuperTuxKart",
        "supertuxkart",
        Source::Flatpak,
    ),
    ("com.play0ad.zeroad", "0 A.D.", "0ad", Source::Flatpak),
    ("org.gnome.Chess", "Chess", "org.gnome.Chess", Source::Home),
    (
        "org.gnome.Sudoku",
        "Sudoku",
        "org.gnome.Sudoku",
        Source::Home,
    ),
    ("org.gnome.Mines", "Mines", "org.gnome.Mines", Source::Home),
    ("gnome-mahjongg", "Mahjongg", "gnome-mahjongg", Source::Home),
];

const HIDDEN: Entry = (
    "avahi-discover",
    "Avahi Zeroconf Browser",
    "preferences-system",
    Source::Home,
);

pub const UTILITIES_FOLDER: FolderId = FolderId(1);
pub const CREATIVE_FOLDER: FolderId = FolderId(2);
pub const GAMES_FOLDER: FolderId = FolderId(3);

/// Where each folder sits among the top-level apps on the first page.
const FOLDER_SLOTS: [(usize, FolderId); 3] = [
    (13, UTILITIES_FOLDER),
    (20, CREATIVE_FOLDER),
    (27, GAMES_FOLDER),
];

pub struct Fixtures {
    home: PathBuf,
    flatpak: PathBuf,
}

impl Fixtures {
    /// Writes the desktop entries under `root` and points the XDG dirs at an empty folder, so
    /// the launchpad's rescans find nothing but the fixtures named in its settings.
    pub fn new(root: &Path) -> Result<Self> {
        let empty = root.join("empty");
        let fixtures = Self {
            home: root.join("home/applications"),
            flatpak: root.join("flatpak/exports/share/applications"),
        };
        for dir in [&empty, &fixtures.home, &fixtures.flatpak] {
            std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        }
        let every = APPS
            .iter()
            .chain(UTILITIES)
            .chain(CREATIVE)
            .chain(GAMES)
            .chain(std::iter::once(&HIDDEN));
        for (stem, name, icon, source) in every {
            let dir = match source {
                Source::Home => &fixtures.home,
                Source::Flatpak => &fixtures.flatpak,
            };
            write_entry(dir, stem, name, &format!("{ICONS}/{icon}.svg"))?;
        }
        // Safety: set before any thread starts, so nothing reads the environment concurrently.
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &empty);
            std::env::set_var("XDG_DATA_DIRS", &empty);
        }
        Ok(fixtures)
    }

    pub fn settings(&self) -> PadSettings {
        PadSettings {
            app_sources: vec![self.home.clone(), self.flatpak.clone()],
            ..PadSettings::default()
        }
    }
}

fn write_entry(dir: &Path, stem: &str, name: &str, icon: &str) -> Result<()> {
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nIcon={icon}\nExec=true\nTerminal=false\n"
    );
    let path = dir.join(format!("{stem}.desktop"));
    std::fs::write(&path, entry).with_context(|| format!("write {}", path.display()))
}

fn app(stem: &str) -> DesktopId {
    DesktopId::new(format!("{stem}.desktop"))
}

fn folder(id: FolderId, name: &str, members: &[Entry]) -> Folder {
    Folder {
        id,
        name: name.to_owned(),
        apps: members.iter().map(|(stem, ..)| app(stem)).collect(),
    }
}

/// The arranged library: three folders on a full first page, the rest on page two, one app
/// hidden.
pub fn layout() -> Layout {
    let mut items: Vec<Item> = APPS.iter().map(|(stem, ..)| Item::App(app(stem))).collect();
    for (slot, folder) in FOLDER_SLOTS {
        items.insert(slot, Item::Folder(folder));
    }
    Layout {
        pages: vec![items],
        folders: vec![
            folder(UTILITIES_FOLDER, "Utilities", UTILITIES),
            folder(CREATIVE_FOLDER, "Creative", CREATIVE),
            folder(GAMES_FOLDER, "Games", GAMES),
        ],
        hidden: vec![app(HIDDEN.0)],
        ..Layout::default()
    }
}

/// The first-page index of `stem` in [`layout`].
pub fn index_of(stem: &str) -> Option<usize> {
    let wanted = Item::App(app(stem));
    layout()
        .pages
        .first()?
        .iter()
        .position(|item| *item == wanted)
}

pub fn folder_index(id: FolderId) -> Option<usize> {
    let wanted = Item::Folder(id);
    layout()
        .pages
        .first()?
        .iter()
        .position(|item| *item == wanted)
}
