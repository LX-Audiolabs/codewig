//! Per-user Codewig data directory (OS standard locations).
//!
//! | OS      | Root                                              |
//! |---------|---------------------------------------------------|
//! | Windows | `%LOCALAPPDATA%\Codewig`                          |
//! | Linux   | `$XDG_DATA_HOME/Codewig` or `~/.local/share/Codewig` |
//! | macOS   | `~/Library/Application Support/Codewig`           |
//!
//! Subfolders grow over time: `devices/` (param YAML), later maybe presets, logs, …

use std::fs;
use std::path::{Path, PathBuf};

/// Brand folder name under the OS user data root.
pub const APP_DIR_NAME: &str = "Codewig";

/// Override for the whole Codewig user data root (`CODEWIG_HOME`).
pub const ENV_HOME: &str = "CODEWIG_HOME";

/// Override only for device YAML scan (`CODEWIG_DEVICES_DIR`).
pub const ENV_DEVICES_DIR: &str = "CODEWIG_DEVICES_DIR";

/// Per-user Codewig root, if resolvable.
pub fn user_data_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var(ENV_HOME) {
        let p = p.trim();
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    platform_user_data_dir().map(|base| base.join(APP_DIR_NAME))
}

/// `…/Codewig/devices` — param catalog YAML (user-writable).
pub fn user_devices_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var(ENV_DEVICES_DIR) {
        let p = p.trim();
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    user_data_dir().map(|d| d.join("devices"))
}

/// Create `Codewig/` and `Codewig/devices/` if missing.
/// Seeds shipped factory YAMLs into user `devices/` when a file is not present yet
/// (never overwrites user edits).
///
/// Called **once explicitly at startup** (cli `main`, ui `main`). Library users
/// that skip it get a lazy fallback on first global catalog access
/// (`music::param_catalog`). Returns the devices directory path.
pub fn ensure_user_layout() -> Result<PathBuf, String> {
    let root = user_data_dir().ok_or_else(|| {
        format!(
            "cannot resolve user data dir (set {ENV_HOME} or check LOCALAPPDATA / XDG_DATA_HOME / HOME)"
        )
    })?;
    ensure_user_layout_at(&root)
}

/// [`ensure_user_layout`] with an explicit root (tests / tools) — no env lookup.
pub fn ensure_user_layout_at(root: &Path) -> Result<PathBuf, String> {
    let devices = root.join("devices");
    fs::create_dir_all(&devices).map_err(|e| format!("create {}: {e}", devices.display()))?;

    // Placeholder so the folder is obvious in Explorer
    let readme = devices.join("README.txt");
    if !readme.exists() {
        let _ = fs::write(
            &readme,
            "Codewig device param catalog\n\
             \n\
             Drop *.yaml here (bitwig or clap). In codewig-live: Devices tab → ↻ reload.\n\
             Factory defs are copied on first run; your edits are never overwritten.\n\
             See repo devices/README.md for schema.\n",
        );
    }

    seed_devices_if_missing(&devices);
    Ok(devices)
}

/// OS base (without Codewig suffix).
fn platform_user_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                // Rare fallback
                std::env::var_os("USERPROFILE")
                    .map(|u| PathBuf::from(u).join("AppData").join("Local"))
            })
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            let p = xdg.trim();
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

/// Copy factory YAMLs into user devices/ when the dest file does not exist.
fn seed_devices_if_missing(user_devices: &Path) {
    for src_dir in shipped_devices_sources() {
        if !src_dir.is_dir() {
            continue;
        }
        let Ok(rd) = fs::read_dir(&src_dir) else {
            continue;
        };
        let mut any = false;
        for ent in rd.flatten() {
            let path = ent.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            if !(lower.ends_with(".yaml") || lower.ends_with(".yml")) {
                continue;
            }
            if lower == "readme.yaml" || lower == "readme.yml" {
                continue;
            }
            let dest = user_devices.join(name);
            if dest.exists() {
                continue;
            }
            if fs::copy(&path, &dest).is_ok() {
                any = true;
            }
        }
        if any {
            // Prefer first source that had something to copy; still merge missing from later sources
            // by not breaking — continue so all factory files land.
        }
    }
}

/// Places we look for factory `devices/*.yaml` to seed the user folder.
///
/// AppImage: mount is read-only (`APPDIR`). We only **read** factory YAML from
/// there and **copy** into `~/.local/share/Codewig/devices` (writable).
fn shipped_devices_sources() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // AppImage / linuxdeploy: $APPDIR is the mounted AppDir root
    if let Ok(appdir) = std::env::var("APPDIR") {
        let root = PathBuf::from(appdir);
        dirs.push(root.join("usr/share/codewig/devices"));
        dirs.push(root.join("devices"));
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        // AppImage binary at $APPDIR/usr/bin/codewig-live
        dirs.push(parent.join("devices"));
        dirs.push(parent.join("../share/codewig/devices"));
        dirs.push(parent.join("../devices"));
        dirs.push(parent.join("../../devices"));
        dirs.push(parent.join("../../../devices"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("devices"));
        dirs.push(cwd.join("../devices"));
    }
    // Dev: repo layout next to core crate
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../devices"));
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_data_dir_is_some() {
        // CI/dev machines always have HOME or LOCALAPPDATA
        assert!(user_data_dir().is_some());
        let d = user_data_dir().unwrap();
        assert!(d.ends_with(APP_DIR_NAME) || d.to_string_lossy().contains("Codewig"));
    }

    #[test]
    fn ensure_layout_creates_devices() {
        // Isolated temp root — never the real user dir.
        let root = std::env::temp_dir().join(format!("codewig-paths-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let devices = ensure_user_layout_at(&root).expect("layout");
        assert!(devices.is_dir());
        assert_eq!(devices.file_name().unwrap(), "devices");
        assert!(devices.join("README.txt").is_file());
        let _ = fs::remove_dir_all(&root);
    }
}
