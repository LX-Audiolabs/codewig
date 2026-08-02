//! File-based device param catalog (`devices/*.yaml`).
//!
//! Scope v1:
//! - **bitwig** stock/library devices (curated YAML)
//! - **clap** only (system paths later / extra folders via UI someday)
//! - **No** VST3 / LV2; no Bitwig plugin-path queries
//!
//! - File present → param-aware (even if `params: {}`).
//! - No file → params unsupported (insert may still work via insert allowlist).
//! - Load: embedded defaults, then runtime dirs (later overrides by `id`).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Embedded shipped devices (always available without a `devices/` folder).
const EMBEDDED: &[(&str, &str)] = &[
    ("v9kick", include_str!("../../../devices/v9kick.yaml")),
    ("polymer", include_str!("../../../devices/polymer.yaml")),
];

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
    /// Source path or `embedded:<id>`.
    pub source: String,
}

#[derive(Debug, Default)]
pub struct ParamCatalog {
    devices: Vec<DeviceParamFile>,
}

impl ParamCatalog {
    pub fn devices(&self) -> &[DeviceParamFile] {
        &self.devices
    }

    pub fn resolve(&self, name: &str) -> Option<&DeviceParamFile> {
        let n = norm(name);
        self.devices.iter().find(|d| {
            norm(&d.id) == n
                || norm(&d.bitwig_name) == n
                || d.aliases.iter().any(|a| norm(a) == n)
        })
    }

    pub fn resolve_param<'a>(
        &'a self,
        dev: &'a DeviceParamFile,
        name: &str,
    ) -> Option<&'a ParamDef> {
        let n = norm(name);
        dev.params.iter().find(|p| {
            norm(&p.name) == n || p.aliases.iter().any(|a| norm(a) == n)
        })
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

    /// Resolve param sets for execute: `(canonical_name, wire_value)`.
    pub fn map_param_sets(
        &self,
        device: &str,
        sets: &[(String, f64)],
    ) -> Result<Vec<(String, f64)>, String> {
        let Some(dev) = self.resolve(device) else {
            return Err(format!(
                "device '{device}' not in param catalog — add devices/<id>.yaml \
                 (kind: bitwig|clap only; insert may still work)"
            ));
        };
        if dev.params.is_empty() {
            return Err(format!(
                "device '{}' ({}) has no documented params yet — see devices/{}.yaml",
                dev.bitwig_name, dev.id, dev.id
            ));
        }
        let mut out = Vec::with_capacity(sets.len());
        for (name, display) in sets {
            let Some(p) = self.resolve_param(dev, name) else {
                let known: Vec<&str> = dev.params.iter().map(|p| p.name.as_str()).collect();
                return Err(format!(
                    "param '{name}' unknown on {} — known: {}",
                    dev.bitwig_name,
                    known.join(", ")
                ));
            };
            out.push((p.name.clone(), Self::display_to_wire(p, *display)));
        }
        Ok(out)
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
        serde_yaml::from_str(content).map_err(|e| format!("{source}: yaml: {e}"))?;
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

fn norm(s: &str) -> String {
    s.to_lowercase()
        .replace(' ', "")
        .replace('-', "")
        .replace('_', "")
        .replace('.', "")
        .replace('+', "plus")
}

// ── Load ───────────────────────────────────────────────────────────

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(p) = std::env::var("CODEWIG_DEVICES_DIR") {
        dirs.push(PathBuf::from(p));
    }
    dirs.push(PathBuf::from("devices"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("devices"));
            dirs.push(parent.join("../devices"));
            dirs.push(parent.join("../../devices"));
            dirs.push(parent.join("../../../devices"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("devices"));
        dirs.push(cwd.join("../devices"));
        dirs.push(cwd.join("../../devices"));
    }
    dirs
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml") | Some("yml")
    )
}

fn load_dir(dir: &Path, into: &mut HashMap<String, DeviceParamFile>) {
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if !is_yaml(&path) {
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
                eprintln!("param catalog: skip {src}: {e}");
            }
        }
    }
}

pub fn load_catalog() -> ParamCatalog {
    let mut map: HashMap<String, DeviceParamFile> = HashMap::new();

    for (id, text) in EMBEDDED {
        match parse_device_yaml(text, &format!("embedded:{id}")) {
            Ok(dev) => {
                map.insert(dev.id.clone(), dev);
            }
            Err(e) => eprintln!("param catalog: embedded {id}: {e}"),
        }
    }

    for dir in candidate_dirs() {
        if dir.is_dir() {
            load_dir(&dir, &mut map);
        }
    }

    let mut devices: Vec<_> = map.into_values().collect();
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    ParamCatalog { devices }
}

/// Process-wide catalog (embedded + dir scan).
pub fn catalog() -> &'static ParamCatalog {
    static CAT: OnceLock<ParamCatalog> = OnceLock::new();
    CAT.get_or_init(load_catalog)
}

#[cfg(test)]
pub fn catalog_from_embedded_only() -> ParamCatalog {
    let mut map: HashMap<String, DeviceParamFile> = HashMap::new();
    for (id, text) in EMBEDDED {
        let dev = parse_device_yaml(text, &format!("embedded:{id}")).expect(id);
        map.insert(dev.id.clone(), dev);
    }
    let mut devices: Vec<_> = map.into_values().collect();
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    ParamCatalog { devices }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v9kick_embedded() {
        let cat = catalog_from_embedded_only();
        let d = cat.resolve("v9 kick").expect("v9 kick");
        assert_eq!(d.bitwig_name, "v9 Kick");
        assert_eq!(d.kind, DeviceHostKind::Bitwig);
        assert!(cat.resolve_param(d, "decay").is_some());
        assert!(cat.resolve_param(d, "pitch").is_some());
        assert!(cat.resolve_param(d, "p").is_some());
    }

    #[test]
    fn polymer_empty_params() {
        let cat = catalog_from_embedded_only();
        let d = cat.resolve("poly").expect("polymer");
        assert!(d.params.is_empty());
        let err = cat
            .map_param_sets("Polymer", &[("cutoff".into(), 50.0)])
            .unwrap_err();
        assert!(err.contains("no documented params"));
    }

    #[test]
    fn display_percent_to_wire() {
        let cat = catalog_from_embedded_only();
        let sets = cat
            .map_param_sets("v9kick", &[("decay".into(), 50.0), ("pitch".into(), 100.0)])
            .unwrap();
        assert_eq!(sets[0].0, "decay");
        assert!((sets[0].1 - 0.5).abs() < 1e-9);
        assert!((sets[1].1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_device() {
        let cat = catalog_from_embedded_only();
        assert!(cat
            .map_param_sets("Surge XT", &[("cutoff".into(), 1.0)])
            .is_err());
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
