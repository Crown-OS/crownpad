use std::{
    path::Path,
    process::{Command, Stdio},
};

const FLATPAK_EXPORTS: &str = "/flatpak/exports/share/applications/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Package {
    Pacman(String),
    Flatpak { id: String, user: bool },
}

#[derive(Debug, thiserror::Error)]
pub enum UninstallError {
    #[error("run {program}: {source}")]
    Spawn {
        program: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("{program} exited with {status}")]
    Failed {
        program: &'static str,
        status: std::process::ExitStatus,
    },
}

impl Package {
    pub fn name(&self) -> &str {
        match self {
            Self::Pacman(name) => name,
            Self::Flatpak { id, .. } => id,
        }
    }

    fn command(&self) -> (&'static str, Vec<&str>) {
        match self {
            Self::Pacman(name) => ("pkexec", vec!["pacman", "-Rns", "--noconfirm", name]),
            Self::Flatpak { id, user: true } => (
                "flatpak",
                vec!["uninstall", "--user", "-y", "--noninteractive", id],
            ),
            Self::Flatpak { id, user: false } => {
                ("flatpak", vec!["uninstall", "-y", "--noninteractive", id])
            }
        }
    }
}

pub fn owner(desktop_file: &Path) -> Option<Package> {
    let text = desktop_file.to_string_lossy();
    if text.contains(FLATPAK_EXPORTS) {
        let id = desktop_file.file_stem()?.to_str()?.to_owned();
        let user = dirs_home().is_some_and(|home| desktop_file.starts_with(home));
        return Some(Package::Flatpak { id, user });
    }
    let output = Command::new("pacman")
        .args(["-Qoq", "--"])
        .arg(desktop_file)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (output.status.success() && !name.is_empty()).then_some(Package::Pacman(name))
}

pub fn remove(package: &Package) -> Result<(), UninstallError> {
    let (program, arguments) = package.command();
    let status = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .status()
        .map_err(|source| UninstallError::Spawn { program, source })?;
    if status.success() {
        Ok(())
    } else {
        Err(UninstallError::Failed { program, status })
    }
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatpak_exports_are_owned_by_their_app_id() {
        let package = owner(Path::new(
            "/var/lib/flatpak/exports/share/applications/org.gimp.GIMP.desktop",
        ));
        assert_eq!(
            package,
            Some(Package::Flatpak {
                id: "org.gimp.GIMP".to_owned(),
                user: false
            })
        );
    }

    #[test]
    fn pacman_removal_asks_for_privileges() {
        let package = Package::Pacman("gimp".to_owned());
        let (program, arguments) = package.command();
        assert_eq!(program, "pkexec");
        assert_eq!(arguments, ["pacman", "-Rns", "--noconfirm", "gimp"]);
    }
}
