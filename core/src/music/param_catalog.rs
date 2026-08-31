//! File-based device param catalog (`devices/*.yaml`).
//!
//! Scope v1:
//! - **bitwig** stock/library devices (YAML in `devices/`)
//! - **clap** (drop a YAML for any CLAP — no plugin path queries)
//! - **No** VST3 / LV2
//!
//! - **Source of truth = YAML on disk**, not compiled-in embeds.
//! - Scan dirs at load / reload (`CODEWIG_DEVICES_DIR`, `./devices`, next to exe, …).
//! - Later files override earlier ones with the same `id`.
//! - File present → listed in UI + param-aware (even if `params: {}`).
//! - No file → insert may still work; params unsupported.

use super::device::norm;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceHostKind {
    Bitwig,
    Clap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamValueKind {
    Float,
    Bool,
}

#[derive(Debug, Clone)]
pub struct ParamDef {
    /// Canonical name sent to Bitwig (`param.set` name match).
    pub name: String,
    pub aliases: Vec<String>,
    pub wire: (f64, f64),
    pub display: (f64, f64),
    pub unit: String,
    pub kind: ParamValueKind,
}

#[derive(Debug, Clone)]
pub struct PathHint {
    pub windows: Option<String>,
    pub linux: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeviceParamFile {
    pub id: String,
    pub bitwig_name: String,
    pub kind: DeviceHostKind,
    pub path_hint: Option<PathHint>,
    pub aliases: Vec<String>,
    pub params: Vec<ParamDef>,
    /// Source path of the YAML file.
    pub source: String,
}

#[derive(Debug, Default, Clone)]
pub struct ParamCatalog {
    devices: Vec<DeviceParamFile>,
    /// Non-fatal load problems (skipped YAML files, user-layout seed) from the scan.
    load_errors: Vec<String>,
}

impl ParamCatalog {
    pub fn devices(&self) -> &[DeviceParamFile] {
        &self.devices
    }

    /// Non-fatal problems hit while scanning `devices/*.yaml` (bad files were skipped).
    pub fn load_errors(&self) -> &[String] {
        &self.load_errors
    }

    pub fn resolve(&self, name: &str) -> Option<&DeviceParamFile> {
        let n = norm(name);
        self.devices.iter().find(|d| {
            norm(&d.id) == n || norm(&d.bitwig_name) == n || d.aliases.iter().any(|a| norm(a) == n)
        })
    }

    pub fn resolve_param<'a>(
        &'a self,
        dev: &'a DeviceParamFile,
        name: &str,
    ) -> Option<&'a ParamDef> {
        let n = norm(name);
        dev.params
            .iter()
            .find(|p| norm(&p.name) == n || p.aliases.iter().any(|a| norm(a) == n))
    }

    /// Map user display value → wire 0..1 (clamped).
    pub fn display_to_wire(def: &ParamDef, display: f64) -> f64 {
        match def.kind {
            ParamValueKind::Bool => {
                if display != 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            ParamValueKind::Float => {
                let (d0, d1) = def.display;
                let (w0, w1) = def.wire;
                if (d1 - d0).abs() < f64::EPSILON {
                    return w0.clamp(w0.min(w1), w0.max(w1));
                }
                let t = (display - d0) / (d1 - d0);
                let v = w0 + t * (w1 - w0);
                let lo = w0.min(w1);
                let hi = w0.max(w1);
                v.clamp(lo, hi)
            }
        }
    }

    /// Resolve param sets for execute: `(bitwig_name, wire_0..1)`.
    ///
    /// YAML catalog is **optional help**, not an allowlist:
    /// - Device + param in catalog → display range → wire, aliases, full UI labels.
    /// - Missing device / empty params / unknown param → pass through as **wire 0..1**
    ///   (same as raw `param set` on the CLI). Values outside 0..1 error with a hint.
    pub fn map_param_sets(
        &self,
        device: &str,
        sets: &[(String, f64)],
    ) -> Result<Vec<(String, f64)>, String> {
        let dev = self.resolve(device);
        let mut out = Vec::with_capacity(sets.len());
        for (name, value) in sets {
            if let Some(dev) = dev
                && let Some(p) = self.resolve_param(dev, name)
            {
                // Help path: aliases + display→wire
                let wire_name = bitwig_match_name(p);
                out.push((wire_name, Self::display_to_wire(p, *value)));
                continue;
            }
            // Open path: name as typed, value already wire-normalized
            out.push(wire_passthrough(name, *value)?);
        }
        Ok(out)
    }
}

/// Raw wire set when no YAML mapping applies. Value must already be 0..1.
fn wire_passthrough(name: &str, v: f64) -> Result<(String, f64), String> {
    if (0.0..=1.0).contains(&v) {
        Ok((name.to_string(), v))
    } else {
        Err(format!(
            "param '{name}': value {v} is outside wire 0..1. \
             Without a devices/*.yaml entry, use raw 0..1 (CLI style). \
             Add YAML for display ranges (e.g. 0..100) and aliases."
        ))
    }
}

// ── YAML ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeviceYaml {
    id: String,
    bitwig_name: String,
    kind: String,
    #[serde(default)]
    path_hint: Option<PathHintYaml>,
    #[serde(default)]
    aliases: Option<Vec<String>>,
    #[serde(default)]
    params: Option<HashMap<String, ParamYaml>>,
}

#[derive(Debug, Deserialize)]
struct PathHintYaml {
    #[serde(default)]
    windows: Option<String>,
    #[serde(default)]
    linux: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ParamYaml {
    #[serde(default)]
    wire: Option<Vec<f64>>,
    #[serde(default)]
    display: Option<Vec<f64>>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    aliases: Option<Vec<String>>,
    #[serde(default, rename = "type")]
    ty: Option<String>,
}

fn pair(v: Option<Vec<f64>>, default: (f64, f64)) -> (f64, f64) {
    match v.as_deref() {
        Some([a, b, ..]) => (*a, *b),
        _ => default,
    }
}

fn parse_kind(s: &str) -> Result<DeviceHostKind, String> {
    match s.trim().to_lowercase().as_str() {
        "bitwig" => Ok(DeviceHostKind::Bitwig),
        "clap" => Ok(DeviceHostKind::Clap),
        other => Err(format!(
            "kind '{other}' unsupported — use bitwig|clap (no vst3/lv2 yet)"
        )),
    }
}

/// Parse one `devices/*.yaml` file.
pub fn parse_device_yaml(content: &str, source: &str) -> Result<DeviceParamFile, String> {
    let content = content.trim_start_matches('\u{feff}');
    let y: DeviceYaml =
        serde_norway::from_str(content).map_err(|e| format!("{source}: yaml: {e}"))?;
    let kind = parse_kind(&y.kind).map_err(|e| format!("{source}: {e}"))?;

    let mut params = Vec::new();
    if let Some(map) = y.params {
        for (name, py) in map {
            let pkind = match py.ty.as_deref().unwrap_or("float") {
                "bool" | "boolean" | "onoff" => ParamValueKind::Bool,
                _ => ParamValueKind::Float,
            };
            params.push(ParamDef {
                name,
                aliases: py.aliases.unwrap_or_default(),
                wire: pair(py.wire, (0.0, 1.0)),
                display: pair(py.display, (0.0, 1.0)),
                unit: py.unit.unwrap_or_default(),
                kind: pkind,
            });
        }
        params.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let path_hint = y.path_hint.map(|h| PathHint {
        windows: h.windows,
        linux: h.linux,
    });

    Ok(DeviceParamFile {
        id: y.id,
        bitwig_name: y.bitwig_name,
        kind,
        path_hint,
        aliases: y.aliases.unwrap_or_default(),
        params,
        source: source.to_string(),
    })
}

/// Name sent to Bitwig `param.set` — full UI label when available via alias.
fn bitwig_match_name(p: &ParamDef) -> String {
    p.aliases
        .iter()
        .filter(|a| a.contains(' '))
        .max_by_key(|a| a.len())
        .cloned()
        .unwrap_or_else(|| p.name.clone())
}

// ── Load ───────────────────────────────────────────────────────────

/// Dirs scanned in order. **Later dirs override same `id`.**
///
/// Product path first is seeded via [`crate::paths::ensure_user_layout`]
/// (called at startup, or lazily on first global catalog access);
/// repo/`./devices` still win when developing (loaded later).
pub fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // Explicit override (same as user_devices when CODEWIG_DEVICES_DIR is set)
    if let Ok(p) = std::env::var(crate::paths::ENV_DEVICES_DIR) {
        let p = p.trim();
        if !p.is_empty() {
            dirs.push(PathBuf::from(p));
        }
    }
    // Per-user install location (created + seeded on ensure)
    if let Some(d) = crate::paths::user_devices_dir() {
        // Avoid double-push when CODEWIG_DEVICES_DIR already set to the same path
        if dirs.last().map(|x| x != &d).unwrap_or(true) {
            dirs.push(d);
        }
    }
    // Dev / portable: next to cwd and binary
    dirs.push(PathBuf::from("devices"));
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        dirs.push(parent.join("devices"));
        dirs.push(parent.join("../devices"));
        dirs.push(parent.join("../../devices"));
        dirs.push(parent.join("../../../devices"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("devices"));
        dirs.push(cwd.join("../devices"));
        dirs.push(cwd.join("../../devices"));
    }
    // Dev: repo devices/ next to core crate (last → overrides user when developing)
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../devices"));
    dirs
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml") | Some("yml")
    )
}

fn load_dir(dir: &Path, into: &mut HashMap<String, DeviceParamFile>, errors: &mut Vec<String>) {
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if !is_yaml(&path) {
            continue;
        }
        // Skip docs / non-device files
        if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            n.eq_ignore_ascii_case("README.yaml") || n.eq_ignore_ascii_case("README.yml")
        }) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let src = path.display().to_string();
        match parse_device_yaml(&text, &src) {
            Ok(dev) => {
                into.insert(dev.id.clone(), dev);
            }
            Err(e) => {
                errors.push(format!("skip {src}: {e}"));
            }
        }
    }
}

/// Scan disk and build a catalog (no global write). **Read-only** — creates
/// and seeds nothing. The user layout is created once at startup
/// ([`crate::paths::ensure_user_layout`], called from cli/ui `main`) or lazily
/// on first global catalog access (see [`catalog`]).
pub fn load_catalog() -> ParamCatalog {
    let mut map: HashMap<String, DeviceParamFile> = HashMap::new();
    let mut errors = Vec::new();
    let mut seen_dirs = Vec::new();
    for dir in candidate_dirs() {
        let Ok(canon) = dir.canonicalize() else {
            if dir.is_dir() {
                load_dir(&dir, &mut map, &mut errors);
                seen_dirs.push(dir.display().to_string());
            }
            continue;
        };
        if !canon.is_dir() {
            continue;
        }
        if seen_dirs.iter().any(|s| s == &canon.display().to_string()) {
            continue;
        }
        seen_dirs.push(canon.display().to_string());
        load_dir(&canon, &mut map, &mut errors);
    }
    let mut devices: Vec<_> = map.into_values().collect();
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    ParamCatalog {
        devices,
        load_errors: errors,
    }
}

fn global() -> &'static RwLock<Arc<ParamCatalog>> {
    static CAT: OnceLock<RwLock<Arc<ParamCatalog>>> = OnceLock::new();
    CAT.get_or_init(|| {
        // Lazy fallback for library users that never called
        // [`crate::paths::ensure_user_layout`] at startup: create + seed the user
        // layout on first global catalog access. cli/ui `main` ensure explicitly,
        // so there this is a no-op. Seed errors surface via `load_errors`.
        let seed_err = crate::paths::ensure_user_layout().err();
        let mut cat = load_catalog();
        if let Some(e) = seed_err {
            cat.load_errors.push(format!("user layout: {e}"));
        }
        RwLock::new(Arc::new(cat))
    })
}

/// Snapshot of the process-wide catalog (Arc clone — cheap, lock released).
pub fn catalog() -> Arc<ParamCatalog> {
    global().read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Rescan `devices/` dirs and replace the global catalog.
/// Returns the number of device entries loaded.
pub fn reload_catalog() -> usize {
    let cat = load_catalog();
    let n = cat.devices.len();
    *global().write().unwrap_or_else(|e| e.into_inner()) = Arc::new(cat);
    n
}

/// Load only from an explicit directory (tests / tools).
pub fn catalog_from_dir(dir: &Path) -> ParamCatalog {
    let mut map = HashMap::new();
    let mut errors = Vec::new();
    load_dir(dir, &mut map, &mut errors);
    let mut devices: Vec<_> = map.into_values().collect();
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    ParamCatalog {
        devices,
        load_errors: errors,
    }
}

#[cfg(test)]
fn repo_devices_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../devices")
}

#[cfg(test)]
pub fn catalog_from_repo_devices() -> ParamCatalog {
    catalog_from_dir(&repo_devices_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_v9_family_from_devices_dir() {
        let cat = catalog_from_repo_devices();
        assert!(
            cat.load_errors().is_empty(),
            "repo devices must load clean: {:?}",
            cat.load_errors()
        );
        // knobs/sliders only — no keytrack/consistent toggles
        let expected: &[(&str, &str, usize)] = &[
            ("v9kick", "v9 Kick", 9),
            ("v9clap", "v9 Clap", 7),
            ("v9snare", "v9 Snare", 8),
            ("v9tom", "v9 Tom", 8),
            ("v9rimshot", "v9 Rimshot", 4),
            ("v9hatclosed", "v9 Hat Closed", 8),
            ("v9hatopen", "v9 Hat Open", 8),
            ("v9crash", "v9 Crash", 8),
            ("v9ride", "v9 Ride", 8),
        ];
        for &(id, bitwig, n) in expected {
            let d = cat.resolve(id).unwrap_or_else(|| panic!("missing {id}"));
            assert_eq!(d.bitwig_name, bitwig);
            assert_eq!(d.params.len(), n, "{id} param count");
            assert!(cat.resolve_param(d, "output").is_some(), "{id} output");
            assert!(
                cat.resolve_param(d, "keytrack").is_none(),
                "{id} must not list keytrack toggle"
            );
        }
        let rim = cat.resolve("v9rimshot").unwrap();
        assert!(cat.resolve_param(rim, "decay").is_none());
        assert!(cat.resolve_param(rim, "tone").is_some());
        let sn = cat.resolve("v9snare").unwrap();
        assert!(cat.resolve_param(sn, "noise").is_some());
        let tom = cat.resolve("v9tom").unwrap();
        assert!(cat.resolve_param(tom, "snap").is_some());
        assert!(cat.resolve_param(tom, "click").is_some());
    }

    #[test]
    fn parse_v9clap_from_disk() {
        let cat = catalog_from_repo_devices();
        let d = cat.resolve("v9 clap").expect("v9 clap");
        assert_eq!(d.bitwig_name, "v9 Clap");
        assert_eq!(d.params.len(), 7);
        for name in [
            "tune",
            "decay",
            "flam",
            "width",
            "variation",
            "output",
            "velocity",
        ] {
            assert!(cat.resolve_param(d, name).is_some(), "missing param {name}");
        }
        assert!(cat.resolve_param(d, "stereo").is_some());
        assert!(cat.resolve_param(d, "var").is_some());
        let sets = cat
            .map_param_sets("v9clap", &[("decay".into(), 50.0), ("flam".into(), 40.0)])
            .unwrap();
        assert_eq!(sets[0].0, "decay");
        assert!((sets[0].1 - 0.5).abs() < 1e-9);
        assert_eq!(sets[1].0, "flam");
        assert!((sets[1].1 - 0.4).abs() < 1e-9);
    }

    #[test]
    fn parse_v9kick_from_disk() {
        let cat = catalog_from_repo_devices();
        let d = cat.resolve("v9 kick").expect("v9 kick");
        assert_eq!(d.bitwig_name, "v9 Kick");
        assert_eq!(d.kind, DeviceHostKind::Bitwig);
        // 9 knobs only (no toggles)
        assert_eq!(d.params.len(), 9);
        for name in [
            "tune",
            "decay",
            "punch",
            "shape",
            "buzz",
            "click",
            "compression",
            "output",
            "velocity",
        ] {
            assert!(cat.resolve_param(d, name).is_some(), "missing param {name}");
        }
        // legacy aliases
        assert!(cat.resolve_param(d, "pitch").is_some());
        assert!(cat.resolve_param(d, "p").is_some());
        assert!(cat.resolve_param(d, "tone").is_some());
        assert!(cat.resolve_param(d, "noise").is_some());
    }

    #[test]
    fn polymer_empty_params_passthrough_wire() {
        let cat = catalog_from_repo_devices();
        let d = cat.resolve("poly").expect("polymer");
        assert!(d.params.is_empty());
        // empty YAML params → wire 0..1, not a hard block
        let sets = cat
            .map_param_sets("Polymer", &[("cutoff".into(), 0.3)])
            .unwrap();
        assert_eq!(sets[0].0, "cutoff");
        assert!((sets[0].1 - 0.3).abs() < 1e-9);
        // display-style 50 without YAML → error (must be wire)
        let err = cat
            .map_param_sets("Polymer", &[("cutoff".into(), 50.0)])
            .unwrap_err();
        assert!(err.contains("0..1"), "{err}");
    }

    #[test]
    fn unknown_device_wire_passthrough() {
        let cat = catalog_from_repo_devices();
        let sets = cat
            .map_param_sets("SomeClap", &[("Filter Cutoff".into(), 0.7)])
            .unwrap();
        assert_eq!(sets[0].0, "Filter Cutoff");
        assert!((sets[0].1 - 0.7).abs() < 1e-9);
    }

    #[test]
    fn display_percent_to_wire() {
        let cat = catalog_from_repo_devices();
        let sets = cat
            .map_param_sets(
                "v9kick",
                &[
                    ("decay".into(), 50.0),
                    ("pitch".into(), 100.0), // alias → tune
                    ("punch".into(), 0.0),
                ],
            )
            .unwrap();
        // short keys when no spaced alias
        assert_eq!(sets[0].0, "decay");
        assert!((sets[0].1 - 0.5).abs() < 1e-9);
        // pitch alias → param tune (no spaced alias preferred beyond "tune")
        assert_eq!(sets[1].0, "tune");
        assert!((sets[1].1 - 1.0).abs() < 1e-9);
        assert_eq!(sets[2].0, "punch");
        assert!((sets[2].1 - 0.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_device_display_value_errors() {
        let cat = catalog_from_repo_devices();
        // no YAML → 50 is not valid wire
        assert!(
            cat.map_param_sets("Surge XT", &[("cutoff".into(), 50.0)])
                .is_err()
        );
    }

    #[test]
    fn reload_rescans_disk() {
        // First global access triggers the lazy user-layout seed — keep it out
        // of the real user dir by pointing CODEWIG_HOME at a temp dir.
        // Only env writer in this test binary; the temp name contains
        // "Codewig", so concurrent env *readers* (paths tests) stay green.
        let tmp = std::env::temp_dir().join(format!("Codewig-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        unsafe { std::env::set_var(crate::paths::ENV_HOME, &tmp) };
        let n = reload_catalog();
        unsafe { std::env::remove_var(crate::paths::ENV_HOME) };
        assert!(n >= 9, "expected v9 family + polymer, got {n}");
        assert!(catalog().resolve("v9kick").is_some());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn reject_vst3_kind() {
        let yaml = r#"
id: foo
bitwig_name: "Foo"
kind: vst3
params: {}
"#;
        let err = parse_device_yaml(yaml, "test").unwrap_err();
        assert!(err.contains("bitwig|clap"), "{err}");
    }
}
