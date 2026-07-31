//! Curated Bitwig device registry — must stay in sync with
//! `extension/.../DeviceCatalog.java` for anything that goes through `device.add`.
//!
//! Two roles:
//! - **Insertable** — small allowlist the extension can insert (UUIDs in Java).
//! - **Drum MIDI map** — alias → note for `d "bd hh"` patterns. Pads are **not**
//!   insertable; user builds Instrument Layer + v* devices by hand for live.

use super::ast::DrumAlias;

/// A Bitwig device we document / address.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    /// Canonical Bitwig name (insertable) or logical drum id (MIDI only).
    pub name: &'static str,
    pub kind: DeviceKind,
    pub alias: &'static str,
    pub params: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Synth,
    /// Insertable shell for multi-pad drum tracks (pads themselves are manual).
    Layer,
    /// Logical drum pad — MIDI only, not `device.add`.
    Drum,
    AudioFx,
}

// ── Insertable (sync with DeviceCatalog.java) ──────────────────────

/// Devices `device.add` / fluent `.device()` may create.
pub static INSERTABLE: &[Device] = &[
    Device {
        name: "Polymer",
        kind: DeviceKind::Synth,
        alias: "poly",
        params: &["cutoff", "res", "envMod", "oscMix", "noise"],
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

// ── Drum MIDI only (not insertable) ────────────────────────────────

/// Logical pad names for docs / MIDI map (user places real v* devices manually).
pub static DRUM_DEVICES: &[Device] = &[
    Device { name: "v0Kick", kind: DeviceKind::Drum, alias: "kick", params: &["decay", "pitch", "tone", "attack"] },
    Device { name: "v0Hat", kind: DeviceKind::Drum, alias: "hh", params: &["decay", "tone"] },
    Device { name: "v0Cymbal", kind: DeviceKind::Drum, alias: "cymb", params: &["decay", "tone"] },
    Device { name: "v0Tom", kind: DeviceKind::Drum, alias: "tom", params: &["decay", "pitch", "tone"] },
    Device { name: "v1Kick", kind: DeviceKind::Drum, alias: "v1kick", params: &["decay", "pitch"] },
    Device { name: "v1Hat", kind: DeviceKind::Drum, alias: "v1hh", params: &["decay", "tone"] },
    Device { name: "v1Snare", kind: DeviceKind::Drum, alias: "v1sn", params: &["decay", "pitch"] },
    Device { name: "v1Perc", kind: DeviceKind::Drum, alias: "v1perc", params: &["decay", "pitch"] },
    Device { name: "v8Kick", kind: DeviceKind::Drum, alias: "v8kick", params: &["decay", "pitch"] },
    Device { name: "v8Hat", kind: DeviceKind::Drum, alias: "v8hh", params: &["decay", "tone"] },
    Device { name: "v8Snare", kind: DeviceKind::Drum, alias: "v8sn", params: &["decay", "pitch"] },
    Device { name: "v8Clap", kind: DeviceKind::Drum, alias: "v8cp", params: &["decay", "tone"] },
    Device { name: "v8Perc", kind: DeviceKind::Drum, alias: "v8perc", params: &["decay", "pitch"] },
    Device { name: "v9Kick", kind: DeviceKind::Drum, alias: "v9kick", params: &["decay", "pitch"] },
    Device { name: "v9Hat", kind: DeviceKind::Drum, alias: "v9hh", params: &["decay", "tone"] },
    Device { name: "v9Snare", kind: DeviceKind::Drum, alias: "v9sn", params: &["decay", "pitch"] },
    Device { name: "v9Clap", kind: DeviceKind::Drum, alias: "v9cp", params: &["decay", "tone"] },
    Device { name: "v9Ride", kind: DeviceKind::Drum, alias: "v9ride", params: &["decay", "tone"] },
    Device { name: "v9Rim", kind: DeviceKind::Drum, alias: "v9rim", params: &["decay", "pitch"] },
];

type DrumEntry = (DrumAlias, i32, &'static str);

static DRUM_MAP: &[DrumEntry] = &[
    (DrumAlias::Bd, 36, "v0Kick"),
    (DrumAlias::Sd, 40, "v8Snare"),
    (DrumAlias::Hh, 42, "v0Hat"),
    (DrumAlias::Cp, 39, "v8Clap"),
    (DrumAlias::Cymb, 49, "v0Cymbal"),
    (DrumAlias::Tom, 50, "v0Tom"),
    (DrumAlias::Ride, 46, "v9Ride"),
    (DrumAlias::Rim, 49, "v9Rim"),
    (DrumAlias::V1Kick, 38, "v1Kick"),
    (DrumAlias::V1Hat, 44, "v1Hat"),
    (DrumAlias::V1Sn, 40, "v1Snare"),
    (DrumAlias::V1Perc, 46, "v1Perc"),
    (DrumAlias::V8Kick, 36, "v8Kick"),
    (DrumAlias::V8Hat, 42, "v8Hat"),
    (DrumAlias::V8Sn, 40, "v8Snare"),
    (DrumAlias::V8Clap, 39, "v8Clap"),
    (DrumAlias::V8Perc, 46, "v8Perc"),
    (DrumAlias::V9Kick, 36, "v9Kick"),
    (DrumAlias::V9Hat, 42, "v9Hat"),
    (DrumAlias::V9Sn, 40, "v9Snare"),
    (DrumAlias::V9Clap, 39, "v9Clap"),
    (DrumAlias::V9Ride, 46, "v9Ride"),
    (DrumAlias::V9Rim, 49, "v9Rim"),
];

/// MIDI note for a drum alias (`d "bd hh"` expand).
pub fn drum_midi(alias: DrumAlias) -> Option<i32> {
    DRUM_MAP.iter().find(|(a, _, _)| *a == alias).map(|(_, midi, _)| *midi)
}

/// Logical pad id for a drum alias (not insertable).
pub fn drum_device(alias: DrumAlias) -> Option<&'static str> {
    DRUM_MAP.iter().find(|(a, _, _)| *a == alias).map(|(_, _, dev)| *dev)
}

/// Resolve `kick.v9` / `808bd` style catalog → logical pad + MIDI.
/// Returns `None` if not a drum pad name.
pub fn catalog_to_drum(catalog: &str) -> Option<(&'static str, i32)> {
    let lower = catalog.to_lowercase();
    // Direct drum device name
    if let Some(d) = DRUM_DEVICES.iter().find(|d| d.name.to_lowercase() == lower || d.alias == lower) {
        let midi = DRUM_MAP.iter().find(|(_, _, n)| *n == d.name).map(|(_, m, _)| *m)?;
        return Some((d.name, midi));
    }
    // type.variant
    let (kind, variant) = match catalog.split_once('.') {
        Some(p) => (p.0.to_lowercase(), p.1.to_lowercase()),
        None => return None,
    };
    let name = match (kind.as_str(), variant.as_str()) {
        ("kick", "v0") | ("kick", "") => "v0Kick",
        ("kick", "v8") | ("kick", "808") => "v8Kick",
        ("kick", "v9") | ("kick", "909") => "v9Kick",
        ("kick", "v1") => "v1Kick",
        ("hat", "v0") | ("hat", "") => "v0Hat",
        ("hat", "v8") | ("hat", "808") => "v8Hat",
        ("hat", "v9") | ("hat", "909") => "v9Hat",
        ("hat", "v1") => "v1Hat",
        ("snare", "v8") | ("snare", "808") | ("sd", "") => "v8Snare",
        ("snare", "v9") | ("snare", "909") => "v9Snare",
        ("snare", "v1") => "v1Snare",
        ("clap", "v8") | ("clap", "808") | ("cp", "") => "v8Clap",
        ("clap", "v9") | ("clap", "909") => "v9Clap",
        ("perc", "v1") => "v1Perc",
        ("perc", "v8") | ("perc", "808") => "v8Perc",
        ("ride", _) => "v9Ride",
        ("rim", _) => "v9Rim",
        ("cymb", _) | ("cy", _) => "v0Cymbal",
        ("tom", _) => "v0Tom",
        _ => return None,
    };
    let midi = DRUM_MAP.iter().find(|(_, _, n)| *n == name).map(|(_, m, _)| *m)?;
    Some((name, midi))
}

// ── Lookups ────────────────────────────────────────────────────────

/// Find an **insertable** device by name or alias.
pub fn find_device(name: &str) -> Option<&'static Device> {
    let lower = name.to_lowercase();
    let canonical = INSERT_ALIASES
        .iter()
        .find(|(a, _)| *a == lower)
        .map(|(_, c)| *c)
        .unwrap_or(name);
    let cl = canonical.to_lowercase();
    INSERTABLE
        .iter()
        .find(|d| d.name.to_lowercase() == cl || d.alias == lower)
}

/// Parameter name hints for insertable devices (and drum pads for UI).
pub fn device_params(name: &str) -> Option<&'static [&'static str]> {
    if let Some(d) = find_device(name) {
        return Some(d.params);
    }
    let lower = name.to_lowercase();
    DRUM_DEVICES
        .iter()
        .find(|d| d.name.to_lowercase() == lower || d.alias == lower)
        .map(|d| d.params)
}

/// Map WIGSCRIPT catalog name → Bitwig name for **`device.add` only**.
///
/// Drum pads (`kick.v9`, `v0Kick`, …) return `None` — not insertable.
/// Old names `Delay-2` / `Chorus` / `Distortion` map to Delay+ / Chorus+ / Saturator.
pub fn catalog_to_bitwig(catalog: &str) -> Option<String> {
    find_device(catalog).map(|d| d.name.to_string())
}

/// Whether this catalog name is allowed for `device.add`.
pub fn is_insertable(catalog: &str) -> bool {
    catalog_to_bitwig(catalog).is_some()
}

// Kit helpers (MIDI layout only — pads must already exist in Bitwig)

pub fn default_kit_devices() -> Vec<(&'static str, i32)> {
    vec![("v0Kick", 36), ("v0Hat", 42), ("v8Snare", 40), ("v8Clap", 39)]
}

pub fn kit_808_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v8Kick", 36),
        ("v8Hat", 42),
        ("v8Snare", 40),
        ("v8Clap", 39),
        ("v8Perc", 46),
    ]
}

pub fn kit_909_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v9Kick", 36),
        ("v9Hat", 42),
        ("v9Snare", 40),
        ("v9Clap", 39),
        ("v9Ride", 46),
        ("v9Rim", 49),
    ]
}

pub fn kit_retro_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v0Kick", 36),
        ("v0Hat", 42),
        ("v0Cymbal", 49),
        ("v0Tom", 50),
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
    fn drums_not_insertable() {
        assert_eq!(catalog_to_bitwig("kick.v9"), None);
        assert_eq!(catalog_to_bitwig("v9Kick"), None);
        assert_eq!(catalog_to_bitwig("kick"), None);
        assert!(!is_insertable("kick.v9"));
    }

    #[test]
    fn drum_catalog_midi() {
        assert_eq!(catalog_to_drum("kick.v9"), Some(("v9Kick", 36)));
        assert_eq!(catalog_to_drum("hat.v8"), Some(("v8Hat", 42)));
        assert_eq!(catalog_to_drum("kick"), Some(("v0Kick", 36)));
        assert_eq!(drum_midi(DrumAlias::Bd), Some(36));
    }

    #[test]
    fn no_sampler_no_drum_machine() {
        assert_eq!(catalog_to_bitwig("Sampler"), None);
        assert_eq!(catalog_to_bitwig("Drum Machine"), None);
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
