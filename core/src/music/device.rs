//! Curated Bitwig device registry — must stay in sync with
//! `extension/.../DeviceCatalog.java` for anything that goes through `device.add`.
//!
//! **Insertable:** Organ, Polymer, Polysynth; Instrument Layer; FX set;
//! all stock drum instruments (v0 Cymbal … v9 Tom) via extension `insertFile`.
//! **Not insertable:** Sampler, Drum Machine (multi-pad / samples / pages — out of scope).
//!
//! ## Drums ≠ Drum Machine (important)
//!
//! Bitwig **stock drum modules** (v0/v1/v8/v9) are **monophonic instruments**.
//! Workflow:
//! 1. `new track(kick).device(v9 kick)` — load module
//! 2. Percussion rhythm: fluent **`.beat(4_)`** (not Strudel hit markers)
//! 3. Exact pitches if wanted: **`n "c1"`** / **`n "36"`** — same as any instrument
//!
//! **No** `d "bd hh sd"` hit map — that is Drum Machine / Strudel only.

/// A Bitwig device we document / address.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    /// Canonical Bitwig name (matches library / UUID catalog).
    pub name: &'static str,
    pub kind: DeviceKind,
    pub alias: &'static str,
    pub params: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Synth,
    /// Insertable shell for multi-pad drum tracks.
    Layer,
    /// Stock drum instrument (insertable via .bwdevice file).
    Drum,
    AudioFx,
}

// ── Insertable (sync with DeviceCatalog.java) ──────────────────────

/// UUID devices + layer + FX (`device.add` via UUID).
pub static INSERTABLE: &[Device] = &[
    Device {
        name: "Polymer",
        kind: DeviceKind::Synth,
        alias: "poly",
        // Params live in devices/polymer.yaml (empty until fixed subset documented).
        params: &[],
    },
    Device {
        name: "Polysynth",
        kind: DeviceKind::Synth,
        alias: "psynth",
        params: &["cutoff", "res", "envMod", "detune", "spread"],
    },
    Device {
        name: "Organ",
        kind: DeviceKind::Synth,
        alias: "organ",
        params: &["drawbar16", "drawbar51", "percussion", "drive", "rotary"],
    },
    Device {
        name: "Instrument Layer",
        kind: DeviceKind::Layer,
        alias: "layer",
        params: &[],
    },
    Device {
        name: "Filter",
        kind: DeviceKind::AudioFx,
        alias: "filt",
        params: &["cutoff", "res", "drive", "mix"],
    },
    Device {
        name: "Reverb",
        kind: DeviceKind::AudioFx,
        alias: "rev",
        params: &["time", "damp", "mix", "size"],
    },
    Device {
        name: "Delay+",
        kind: DeviceKind::AudioFx,
        alias: "dly2",
        params: &["timeL", "timeR", "feedback", "mix"],
    },
    Device {
        name: "Chorus+",
        kind: DeviceKind::AudioFx,
        alias: "chor",
        params: &["rate", "depth", "mix", "spread"],
    },
    Device {
        name: "Saturator",
        kind: DeviceKind::AudioFx,
        alias: "dist",
        params: &["drive", "tone", "mix"],
    },
];

/// Stock drum instruments — Bitwig library display names (spaces). Insert via file.
/// Prefer writing `.device(v9 kick)` / `.device(v9kick)` — avoid `.` inside the name
/// (fluent already uses `.` as step separator).
pub static DRUM_DEVICES: &[Device] = &[
    Device { name: "v0 Cymbal", kind: DeviceKind::Drum, alias: "v0cymbal", params: &["decay", "tone"] },
    Device { name: "v0 Hat", kind: DeviceKind::Drum, alias: "v0hat", params: &["decay", "tone"] },
    Device { name: "v0 Kick", kind: DeviceKind::Drum, alias: "v0kick", params: &["decay", "pitch", "tone", "attack"] },
    Device { name: "v0 Snare", kind: DeviceKind::Drum, alias: "v0snare", params: &["decay", "pitch"] },
    Device { name: "v0 Tom", kind: DeviceKind::Drum, alias: "v0tom", params: &["decay", "pitch", "tone"] },
    Device { name: "v0 Zap Kick", kind: DeviceKind::Drum, alias: "v0zapkick", params: &["decay", "pitch"] },
    Device { name: "v1 Clap", kind: DeviceKind::Drum, alias: "v1clap", params: &["decay", "tone"] },
    Device { name: "v1 Cowbell", kind: DeviceKind::Drum, alias: "v1cowbell", params: &["decay", "pitch"] },
    Device { name: "v1 Hat", kind: DeviceKind::Drum, alias: "v1hat", params: &["decay", "tone"] },
    Device { name: "v1 Kick", kind: DeviceKind::Drum, alias: "v1kick", params: &["decay", "pitch"] },
    Device { name: "v1 Snare", kind: DeviceKind::Drum, alias: "v1snare", params: &["decay", "pitch"] },
    Device { name: "v1 Tom", kind: DeviceKind::Drum, alias: "v1tom", params: &["decay", "pitch"] },
    Device { name: "v8 Clap", kind: DeviceKind::Drum, alias: "v8clap", params: &["decay", "tone"] },
    Device { name: "v8 Claves", kind: DeviceKind::Drum, alias: "v8claves", params: &["decay"] },
    Device { name: "v8 Cowbell", kind: DeviceKind::Drum, alias: "v8cowbell", params: &["decay", "pitch"] },
    Device { name: "v8 Cymbal", kind: DeviceKind::Drum, alias: "v8cymbal", params: &["decay", "tone"] },
    Device { name: "v8 Hat", kind: DeviceKind::Drum, alias: "v8hat", params: &["decay", "tone"] },
    Device { name: "v8 Kick", kind: DeviceKind::Drum, alias: "v8kick", params: &["decay", "pitch"] },
    Device { name: "v8 Maracas", kind: DeviceKind::Drum, alias: "v8maracas", params: &["decay"] },
    Device { name: "v8 Rimshot", kind: DeviceKind::Drum, alias: "v8rimshot", params: &["decay", "pitch"] },
    Device { name: "v8 Snare", kind: DeviceKind::Drum, alias: "v8snare", params: &["decay", "pitch"] },
    Device { name: "v8 Tom", kind: DeviceKind::Drum, alias: "v8tom", params: &["decay", "pitch"] },
    Device { name: "v9 Clap", kind: DeviceKind::Drum, alias: "v9clap", params: &["decay", "tone"] },
    Device { name: "v9 Crash", kind: DeviceKind::Drum, alias: "v9crash", params: &["decay", "tone"] },
    Device { name: "v9 Hat Closed", kind: DeviceKind::Drum, alias: "v9hatclosed", params: &["decay", "tone"] },
    Device { name: "v9 Hat Open", kind: DeviceKind::Drum, alias: "v9hatopen", params: &["decay", "tone"] },
    // Param names/ranges: devices/v9kick.yaml (source of truth for WIGSCRIPT params).
    Device { name: "v9 Kick", kind: DeviceKind::Drum, alias: "v9kick", params: &["decay", "pitch"] },
    Device { name: "v9 Ride", kind: DeviceKind::Drum, alias: "v9ride", params: &["decay", "tone"] },
    Device { name: "v9 Rimshot", kind: DeviceKind::Drum, alias: "v9rimshot", params: &["decay", "pitch"] },
    Device { name: "v9 Snare", kind: DeviceKind::Drum, alias: "v9snare", params: &["decay", "pitch"] },
    Device { name: "v9 Tom", kind: DeviceKind::Drum, alias: "v9tom", params: &["decay", "pitch"] },
];

/// Trigger MIDI for monophonic Bitwig drum **modules** (not Drum Machine pads).
/// Clip hits / `.beat` / `d "…"` rhythm all use this single key.
pub const MONO_DRUM_NOTE: i32 = 36; // C1

/// Back-compat aliases used in docs / older chains (`Delay-2` → `Delay+`).
static INSERT_ALIASES: &[(&str, &str)] = &[
    ("poly", "Polymer"),
    ("psynth", "Polysynth"),
    ("organ", "Organ"),
    ("layer", "Instrument Layer"),
    ("instrumentlayer", "Instrument Layer"),
    ("filt", "Filter"),
    ("filter", "Filter"),
    ("rev", "Reverb"),
    ("reverb", "Reverb"),
    ("dly1", "Delay+"),
    ("dly2", "Delay+"),
    ("delay", "Delay+"),
    ("delay-1", "Delay+"),
    ("delay-2", "Delay+"),
    ("delay+", "Delay+"),
    ("chor", "Chorus+"),
    ("chorus", "Chorus+"),
    ("chorus+", "Chorus+"),
    ("dist", "Saturator"),
    ("distortion", "Saturator"),
    ("saturator", "Saturator"),
];

fn norm(s: &str) -> String {
    s.to_lowercase()
        .replace(' ', "")
        .replace('-', "")
        .replace('_', "")
        .replace('+', "plus")
        .replace('.', "") // kick.v9 legacy → kickv9; prefer "v9 kick" / "v9kick"
}

/// Resolve device catalog → Bitwig library name + monophonic trigger MIDI.
///
/// Preferred: `v9 kick`, `v9kick`, `V9 Kick` (case / spaces ignored).
/// Legacy: `kick.v9` still maps (`.` stripped by [`norm`]).
pub fn catalog_to_drum(catalog: &str) -> Option<(&'static str, i32)> {
    let n = norm(catalog);
    if n.contains("sampler") || n.contains("drummachine") {
        return None;
    }
    // Direct match: Bitwig name, alias, or compacted "v9kick"
    if let Some(d) = DRUM_DEVICES
        .iter()
        .find(|d| norm(d.name) == n || norm(d.alias) == n)
    {
        return Some((d.name, MONO_DRUM_NOTE));
    }
    // Family+type without spaces already in name (v9hatclosed, v0zapkick, …)
    // Legacy type+family: kickv9, hatv8, snarev9 (from kick.v9 after norm)
    let name = match n.as_str() {
        "kickv0" | "v0kick" => "v0 Kick",
        "kickv1" | "v1kick" => "v1 Kick",
        "kickv8" | "v8kick" | "kick808" => "v8 Kick",
        "kickv9" | "v9kick" | "kick909" => "v9 Kick",
        "hatv0" | "v0hat" => "v0 Hat",
        "hatv1" | "v1hat" => "v1 Hat",
        "hatv8" | "v8hat" | "hat808" => "v8 Hat",
        "hatv9" | "v9hat" | "v9hatclosed" | "hat909" => "v9 Hat Closed",
        "v9hatopen" | "hatopen" => "v9 Hat Open",
        "snarev0" | "v0snare" => "v0 Snare",
        "snarev1" | "v1snare" => "v1 Snare",
        "snarev8" | "v8snare" | "snare808" => "v8 Snare",
        "snarev9" | "v9snare" | "snare909" => "v9 Snare",
        "clapv1" | "v1clap" => "v1 Clap",
        "clapv8" | "v8clap" | "clap808" => "v8 Clap",
        "clapv9" | "v9clap" | "clap909" => "v9 Clap",
        "tomv0" | "v0tom" => "v0 Tom",
        "tomv1" | "v1tom" => "v1 Tom",
        "tomv8" | "v8tom" => "v8 Tom",
        "tomv9" | "v9tom" => "v9 Tom",
        "cymbv0" | "v0cymbal" | "cymbalv0" => "v0 Cymbal",
        "cymbv8" | "v8cymbal" => "v8 Cymbal",
        "ridev9" | "v9ride" => "v9 Ride",
        "rimv8" | "v8rim" | "v8rimshot" => "v8 Rimshot",
        "rimv9" | "v9rim" | "v9rimshot" => "v9 Rimshot",
        "crashv9" | "v9crash" => "v9 Crash",
        "zap" | "v0zap" | "v0zapkick" | "zapkick" => "v0 Zap Kick",
        // short family-only names default to v0/v8 stock where it matches library
        "kick" => "v0 Kick",
        "hat" | "hh" => "v0 Hat",
        "snare" | "sd" => "v8 Snare",
        "clap" | "cp" => "v8 Clap",
        "tom" => "v0 Tom",
        "cymb" | "cymbal" | "cy" => "v0 Cymbal",
        "ride" => "v9 Ride",
        "rim" => "v9 Rimshot",
        "crash" => "v9 Crash",
        _ => return None,
    };
    Some((name, MONO_DRUM_NOTE))
}

// ── Lookups ────────────────────────────────────────────────────────

/// Find UUID/FX/layer device by name or alias.
pub fn find_device(name: &str) -> Option<&'static Device> {
    let lower = name.to_lowercase();
    let n = norm(name);
    let canonical = INSERT_ALIASES
        .iter()
        .find(|(a, _)| *a == lower || norm(a) == n)
        .map(|(_, c)| *c)
        .unwrap_or(name);
    let cl = norm(canonical);
    INSERTABLE
        .iter()
        .find(|d| norm(d.name) == cl || norm(d.alias) == n)
}

/// Parameter name hints (static table). Prefer [`super::param_catalog`] for real support.
pub fn device_params(name: &str) -> Option<&'static [&'static str]> {
    if let Some(d) = find_device(name) {
        return Some(d.params);
    }
    if let Some((pad, _)) = catalog_to_drum(name) {
        return DRUM_DEVICES
            .iter()
            .find(|d| d.name == pad)
            .map(|d| d.params);
    }
    None
}

/// Param names from MD catalog when available; else static table.
pub fn device_param_names(name: &str) -> Vec<String> {
    if let Some(dev) = super::param_catalog::catalog().resolve(name) {
        return dev.params.iter().map(|p| p.name.clone()).collect();
    }
    device_params(name)
        .map(|s| s.iter().map(|x| (*x).to_string()).collect())
        .unwrap_or_default()
}

/// Map WIGSCRIPT catalog name → Bitwig name for **`device.add`**.
///
/// Drums resolve to library names (`v9 Kick`). Sampler / Drum Machine → `None`.
pub fn catalog_to_bitwig(catalog: &str) -> Option<String> {
    let n = norm(catalog);
    if n.contains("sampler") || n.contains("drummachine") {
        return None;
    }
    if let Some(d) = find_device(catalog) {
        return Some(d.name.to_string());
    }
    catalog_to_drum(catalog).map(|(name, _)| name.to_string())
}

/// Whether this catalog name is allowed for `device.add`.
pub fn is_insertable(catalog: &str) -> bool {
    catalog_to_bitwig(catalog).is_some()
}

// Kit helpers

pub fn default_kit_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v0 Kick", 36),
        ("v0 Hat", 42),
        ("v8 Snare", 40),
        ("v8 Clap", 39),
    ]
}

pub fn kit_808_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v8 Kick", 36),
        ("v8 Hat", 42),
        ("v8 Snare", 40),
        ("v8 Clap", 39),
        ("v8 Cowbell", 46),
    ]
}

pub fn kit_909_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v9 Kick", 36),
        ("v9 Hat Closed", 42),
        ("v9 Snare", 40),
        ("v9 Clap", 39),
        ("v9 Ride", 46),
        ("v9 Rimshot", 49),
    ]
}

pub fn kit_retro_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v0 Kick", 36),
        ("v0 Hat", 42),
        ("v0 Cymbal", 49),
        ("v0 Tom", 50),
    ]
}

pub fn kit_devices(kit: &str) -> Option<Vec<(&'static str, i32)>> {
    match kit {
        "default" | "" => Some(default_kit_devices()),
        "808" => Some(kit_808_devices()),
        "909" => Some(kit_909_devices()),
        "retro" => Some(kit_retro_devices()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertable_polymer() {
        assert_eq!(catalog_to_bitwig("Polymer"), Some("Polymer".to_string()));
        assert_eq!(catalog_to_bitwig("poly"), Some("Polymer".to_string()));
    }

    #[test]
    fn insertable_delay_aliases() {
        assert_eq!(catalog_to_bitwig("Delay+"), Some("Delay+".to_string()));
        assert_eq!(catalog_to_bitwig("Delay-2"), Some("Delay+".to_string()));
        assert_eq!(catalog_to_bitwig("dly2"), Some("Delay+".to_string()));
    }

    #[test]
    fn insertable_chorus_dist() {
        assert_eq!(catalog_to_bitwig("Chorus"), Some("Chorus+".to_string()));
        assert_eq!(catalog_to_bitwig("Distortion"), Some("Saturator".to_string()));
    }

    #[test]
    fn drums_are_insertable() {
        // Preferred names (no fluent `.` inside device token)
        assert_eq!(catalog_to_bitwig("v9 kick"), Some("v9 Kick".to_string()));
        assert_eq!(catalog_to_bitwig("v9kick"), Some("v9 Kick".to_string()));
        assert_eq!(catalog_to_bitwig("V9 Kick"), Some("v9 Kick".to_string()));
        // legacy dotted form still works (inside device(...))
        assert_eq!(catalog_to_bitwig("kick.v9"), Some("v9 Kick".to_string()));
        assert!(is_insertable("v0 Cymbal"));
        assert!(is_insertable("v9 Tom"));
    }

    #[test]
    fn drum_catalog_mono_midi() {
        // Device resolve + monophonic trigger for .beat
        assert_eq!(catalog_to_drum("v9 kick"), Some(("v9 Kick", MONO_DRUM_NOTE)));
        assert_eq!(catalog_to_drum("v8hat"), Some(("v8 Hat", MONO_DRUM_NOTE)));
        assert_eq!(catalog_to_drum("kick.v9"), Some(("v9 Kick", MONO_DRUM_NOTE)));
    }

    #[test]
    fn no_sampler_no_drum_machine() {
        assert_eq!(catalog_to_bitwig("Sampler"), None);
        assert_eq!(catalog_to_bitwig("Drum Machine"), None);
        assert!(!is_insertable("Sampler"));
    }

    #[test]
    fn layer_insertable() {
        assert_eq!(
            catalog_to_bitwig("Instrument Layer"),
            Some("Instrument Layer".to_string())
        );
        assert_eq!(catalog_to_bitwig("layer"), Some("Instrument Layer".to_string()));
    }
}
