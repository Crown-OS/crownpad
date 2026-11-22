use std::path::PathBuf;

use freedesktop_entry_parser::{AttrSelector, Entry};

use super::DesktopId;
use crate::util::{locale, non_default_gpu::GpuPreference};

const SECTION: &str = "Desktop Entry";
const APPLICATION: &str = "Application";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    pub id: DesktopId,
    pub path: PathBuf,
    pub name: String,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub terminal: bool,
    pub gpu: GpuPreference,
    pub categories: Vec<String>,
    pub window_class: Option<String>,
    pub installed_at: u64,
}

pub struct Environment<'a> {
    pub locale: Option<&'a str>,
    pub desktops: &'a [String],
}

impl AppEntry {
    pub fn parse(
        id: DesktopId,
        path: PathBuf,
        bytes: Vec<u8>,
        environment: &Environment<'_>,
    ) -> Option<Self> {
        let entry = Entry::parse(bytes).ok()?;
        let section = entry.section(SECTION);
        let shown = section.attr("Type") == Some(APPLICATION)
            && !flag(&section, "NoDisplay")
            && !flag(&section, "Hidden")
            && shown_in(&section, environment.desktops);
        if !shown {
            return None;
        }
        Some(Self {
            name: localized(&section, "Name", environment.locale)?.to_owned(),
            icon: localized(&section, "Icon", environment.locale).map(str::to_owned),
            exec: section.attr("Exec").map(str::to_owned),
            terminal: flag(&section, "Terminal"),
            gpu: GpuPreference::from_desktop_entry(section.attr(GpuPreference::DESKTOP_ENTRY_KEY)),
            categories: list(section.attr("Categories")),
            window_class: section.attr("StartupWMClass").map(str::to_owned),
            installed_at: 0,
            id,
            path,
        })
    }

    pub fn app_id(&self) -> &str {
        self.window_class
            .as_deref()
            .unwrap_or_else(|| self.id.stem())
    }
}

fn flag(section: &AttrSelector<'_, &str>, key: &str) -> bool {
    section.attr(key) == Some("true")
}

fn list(value: Option<&str>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(|value| value.split(';'))
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

fn shown_in(section: &AttrSelector<'_, &str>, desktops: &[String]) -> bool {
    let listed = |key| list(section.attr(key));
    let only = listed("OnlyShowIn");
    let not = listed("NotShowIn");
    let current = |items: &[String]| items.iter().any(|item| desktops.contains(item));
    (only.is_empty() || current(&only)) && !current(&not)
}

fn localized<'s>(
    section: &'s AttrSelector<'_, &str>,
    key: &str,
    locale: Option<&str>,
) -> Option<&'s str> {
    locale
        .into_iter()
        .flat_map(locale::candidates)
        .find_map(|candidate| section.attr_with_param(key, candidate))
        .or_else(|| section.attr(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str, locale: Option<&str>, desktops: &[String]) -> Option<AppEntry> {
        AppEntry::parse(
            DesktopId::new("org.example.App.desktop"),
            PathBuf::from("/apps/org.example.App.desktop"),
            text.as_bytes().to_vec(),
            &Environment { locale, desktops },
        )
    }

    const APP: &str = "[Desktop Entry]\nType=Application\nName=Files\nName[de]=Dateien\nExec=files %U\nIcon=files\nCategories=System;Utility;\n";

    #[test]
    fn localized_name_wins_over_the_default() {
        let entry = parse(APP, Some("de_DE.UTF-8"), &[]);
        assert_eq!(entry.map(|entry| entry.name), Some("Dateien".to_owned()));
    }

    #[test]
    fn categories_are_split() {
        let entry = parse(APP, None, &[]);
        assert_eq!(
            entry.map(|entry| entry.categories),
            Some(vec!["System".to_owned(), "Utility".to_owned()])
        );
    }

    #[test]
    fn hidden_and_foreign_entries_are_skipped() {
        let hidden = format!("{APP}NoDisplay=true\n");
        assert!(parse(&hidden, None, &[]).is_none());
        let kde_only = format!("{APP}OnlyShowIn=KDE;\n");
        assert!(parse(&kde_only, None, &["CrownOS".to_owned()]).is_none());
        assert!(parse(&kde_only, None, &["KDE".to_owned()]).is_some());
    }

    #[test]
    fn games_can_prefer_the_non_default_gpu() {
        let game = format!("{APP}PrefersNonDefaultGPU=true\n");
        assert_eq!(
            parse(&game, None, &[]).map(|entry| entry.gpu),
            Some(GpuPreference::NonDefault)
        );
        assert_eq!(
            parse(APP, None, &[]).map(|entry| entry.gpu),
            Some(GpuPreference::Default)
        );
    }

    #[test]
    fn app_id_prefers_the_window_class() {
        let classed = format!("{APP}StartupWMClass=files-window\n");
        let entry = parse(&classed, None, &[]);
        assert_eq!(entry.as_ref().map(AppEntry::app_id), Some("files-window"));
        let plain = parse(APP, None, &[]);
        assert_eq!(
            plain.as_ref().map(AppEntry::app_id),
            Some("org.example.App")
        );
    }
}
