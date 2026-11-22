use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use super::{AppEntry, DESKTOP_EXTENSION, DesktopId, entry::Environment};
use crate::util::{locale, xdg};

const MAX_DEPTH: usize = 4;

pub fn default_dirs() -> Vec<PathBuf> {
    xdg::application_dirs()
}

pub fn scan(dirs: &[PathBuf]) -> Vec<AppEntry> {
    let locale = locale::current();
    let desktops: Vec<String> = env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|desktop| !desktop.is_empty())
        .map(str::to_owned)
        .collect();
    let environment = Environment {
        locale: locale.as_deref(),
        desktops: &desktops,
    };
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    for dir in dirs {
        let mut files = Vec::new();
        collect(dir, dir, 0, &mut files);
        for (id, path) in files {
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(entry) = read(id, path, &environment) {
                entries.push(entry);
            }
        }
    }
    entries
}

fn collect(root: &Path, dir: &Path, depth: usize, files: &mut Vec<(DesktopId, PathBuf)>) {
    let Ok(listing) = fs::read_dir(dir) else {
        return;
    };
    for path in listing.filter_map(Result::ok).map(|item| item.path()) {
        if path.is_dir() {
            if depth < MAX_DEPTH {
                collect(root, &path, depth + 1, files);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == DESKTOP_EXTENSION)
            && let Some(id) = path
                .strip_prefix(root)
                .ok()
                .and_then(DesktopId::from_relative)
        {
            files.push((id, path));
        }
    }
}

fn read(id: DesktopId, path: PathBuf, environment: &Environment<'_>) -> Option<AppEntry> {
    let bytes = fs::read(&path).ok()?;
    let installed_at = fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());
    let mut entry = AppEntry::parse(id, path, bytes, environment)?;
    entry.installed_at = installed_at;
    Some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Installed(PathBuf);

    impl Installed {
        fn new(name: &str, files: &[(&str, &str)]) -> Self {
            let dir = env::temp_dir().join(format!("crownpad-scan-{name}-{}", std::process::id()));
            for (file, text) in files {
                let path = dir.join(file);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                fs::write(path, text).ok();
            }
            Self(dir)
        }
    }

    impl Drop for Installed {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    const APP: &str = "[Desktop Entry]\nType=Application\nName=App\nExec=app\n";

    #[test]
    fn earlier_directories_shadow_later_ones_and_subdirectories_are_scanned() {
        let user = Installed::new("user", &[("app.desktop", APP)]);
        let system = Installed::new(
            "system",
            &[("app.desktop", APP), ("vendor/tool.desktop", APP)],
        );
        let entries = scan(&[user.0.clone(), system.0.clone()]);
        let ids: Vec<_> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(entries.len(), 2);
        assert!(ids.contains(&"vendor-tool.desktop"));
        let app = entries
            .iter()
            .find(|entry| entry.id.as_str() == "app.desktop");
        assert_eq!(app.map(|entry| entry.path.starts_with(&user.0)), Some(true));
    }
}
