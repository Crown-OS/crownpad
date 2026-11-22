use std::path::Path;

use crowndock::settings::DockSettings;

pub fn pin(desktop_file: &Path) -> Result<(), crownconfig::ConfigError> {
    let mut dock: DockSettings = crownconfig::load_section(DockSettings::SECTION);
    if dock.items.iter().any(|item| item == desktop_file) {
        return Ok(());
    }
    dock.items.push(desktop_file.to_owned());
    crownconfig::save_section(DockSettings::SECTION, &dock)
}
