//! Curated Bitwig device registry.
//!
//! Maps device names and drum aliases to MIDI notes and parameter names.

use super::ast::DrumAlias;

/// A Bitwig device we can address.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: &'static str,         // Bitwig device name
    pub kind: DeviceKind,
    pub alias: &'static str,        // short form
    pub params: &'static [&'static str], // parameter names
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Synth,
    Drum,
    AudioFx,
    NoteFx,
}

/// Drum kits: named collections of individual drum devices.
#[derive(Debug, Clone)]
pub struct DrumKit {
    pub name: &'static str,
    pub pads: &'static [(DrumAlias, &'static str, i32)], // (alias, device_name, midi_note)
}

// ── Synths ─────────────────────────────────────────────────────────

pub static SYNTHS: &[Device] = &[
    Device {
        name: "Polymer", kind: DeviceKind::Synth, alias: "poly",
        params: &["cutoff", "res", "envMod", "oscMix", "noise"],
    },
    Device {
        name: "Polysynth", kind: DeviceKind::Synth, alias: "psynth",
        params: &["cutoff", "res", "envMod", "detune", "spread"],
    },
    Device {
        name: "Organ", kind: DeviceKind::Synth, alias: "organ",
        params: &["drawbar16", "drawbar51", "percussion", "drive", "rotary"],
    },
];

// ── Drum devices (individual) ──────────────────────────────────────

pub static DRUM_DEVICES: &[Device] = &[
    // v0
    Device { name: "v0Kick",   kind: DeviceKind::Drum, alias: "kick",  params: &["decay", "pitch", "tone", "attack", "click"] },
    Device { name: "v0Hat",    kind: DeviceKind::Drum, alias: "hh",    params: &["decay", "tone", "color"] },
    Device { name: "v0Cymbal", kind: DeviceKind::Drum, alias: "cymb",  params: &["decay", "tone"] },
    Device { name: "v0Tom",    kind: DeviceKind::Drum, alias: "tom",   params: &["decay", "pitch", "tone"] },
    // v1
    Device { name: "v1Kick",   kind: DeviceKind::Drum, alias: "v1kick", params: &["decay", "pitch", "tone", "attack"] },
    Device { name: "v1Hat",    kind: DeviceKind::Drum, alias: "v1hh",   params: &["decay", "tone"] },
    Device { name: "v1Snare",  kind: DeviceKind::Drum, alias: "v1sn",   params: &["decay", "pitch", "tone", "snap"] },
    Device { name: "v1Perc",   kind: DeviceKind::Drum, alias: "v1perc", params: &["decay", "pitch"] },
    // v8 (808)
    Device { name: "v8Kick",   kind: DeviceKind::Drum, alias: "v8kick",  params: &["decay", "pitch", "tone", "attack"] },
    Device { name: "v8Hat",    kind: DeviceKind::Drum, alias: "v8hh",    params: &["decay", "tone"] },
    Device { name: "v8Snare",  kind: DeviceKind::Drum, alias: "v8sn",    params: &["decay", "pitch", "tone", "snap"] },
    Device { name: "v8Clap",   kind: DeviceKind::Drum, alias: "v8cp",    params: &["decay", "tone"] },
    Device { name: "v8Perc",   kind: DeviceKind::Drum, alias: "v8perc",  params: &["decay", "pitch"] },
    // v9 (909)
    Device { name: "v9Kick",   kind: DeviceKind::Drum, alias: "v9kick",  params: &["decay", "pitch", "tone", "attack", "drive"] },
    Device { name: "v9Hat",    kind: DeviceKind::Drum, alias: "v9hh",    params: &["decay", "tone"] },
    Device { name: "v9Snare",  kind: DeviceKind::Drum, alias: "v9sn",    params: &["decay", "pitch", "tone", "snap"] },
    Device { name: "v9Clap",   kind: DeviceKind::Drum, alias: "v9cp",    params: &["decay", "tone"] },
    Device { name: "v9Ride",   kind: DeviceKind::Drum, alias: "v9ride",  params: &["decay", "tone"] },
    Device { name: "v9Rim",    kind: DeviceKind::Drum, alias: "v9rim",   params: &["decay", "pitch"] },
];

// ── Audio FX ───────────────────────────────────────────────────────

pub static FX: &[Device] = &[
    Device { name: "Chorus",        kind: DeviceKind::AudioFx, alias: "chor",   params: &["rate", "depth", "mix", "spread"] },
    Device { name: "Delay-1",       kind: DeviceKind::AudioFx, alias: "dly1",   params: &["time", "feedback", "mix"] },
    Device { name: "Delay-2",       kind: DeviceKind::AudioFx, alias: "dly2",   params: &["timeL", "timeR", "feedback", "mix"] },
    Device { name: "Distortion",    kind: DeviceKind::AudioFx, alias: "dist",   params: &["drive", "tone", "mix"] },
    Device { name: "Filter",        kind: DeviceKind::AudioFx, alias: "filt",   params: &["cutoff", "res", "drive", "mix"] },
    Device { name: "Flanger",       kind: DeviceKind::AudioFx, alias: "flang",  params: &["rate", "depth", "feedback", "mix"] },
    Device { name: "Freq Shifter",  kind: DeviceKind::AudioFx, alias: "fshift", params: &["freq", "mix"] },
    Device { name: "Pitch Shifter", kind: DeviceKind::AudioFx, alias: "pshift", params: &["pitch", "mix"] },
    Device { name: "Reverb",        kind: DeviceKind::AudioFx, alias: "rev",    params: &["time", "damp", "mix", "size"] },
    Device { name: "Rotary",        kind: DeviceKind::AudioFx, alias: "rot",    params: &["speed", "balance", "mix"] },
    Device { name: "Sweep",         kind: DeviceKind::AudioFx, alias: "sweep",  params: &["cutoff", "res", "speed", "depth"] },
];

// ── Note FX ────────────────────────────────────────────────────────

pub static NOTE_FX: &[Device] = &[
    Device { name: "Arpeggiator",         kind: DeviceKind::NoteFx, alias: "arp",    params: &["rate", "octaves", "gate"] },
    Device { name: "Chords",              kind: DeviceKind::NoteFx, alias: "chords", params: &["spread"] },
    Device { name: "Diatonic Transposer", kind: DeviceKind::NoteFx, alias: "diatr",  params: &["steps"] },
    Device { name: "Multi-Note",          kind: DeviceKind::NoteFx, alias: "multi",  params: &["spread", "voices"] },
    Device { name: "Note Echo",           kind: DeviceKind::NoteFx, alias: "necho",  params: &["time", "feedback", "repeats"] },
    Device { name: "Note Filter",         kind: DeviceKind::NoteFx, alias: "nfilt",  params: &["low", "high"] },
];

// ── Drum alias → MIDI note lookup ──────────────────────────────────

/// (midi_note, device_name) for each drum alias.
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

/// Look up MIDI note for a drum alias.
pub fn drum_midi(alias: DrumAlias) -> Option<i32> {
    DRUM_MAP.iter().find(|(a, _, _)| *a == alias).map(|(_, midi, _)| *midi)
}

/// Look up device name for a drum alias.
pub fn drum_device(alias: DrumAlias) -> Option<&'static str> {
    DRUM_MAP.iter().find(|(a, _, _)| *a == alias).map(|(_, _, dev)| *dev)
}

/// Find a device by name (case-insensitive prefix match).
pub fn find_device(name: &str) -> Option<&'static Device> {
    let lower = name.to_lowercase();
    SYNTHS.iter().chain(DRUM_DEVICES.iter()).chain(FX.iter()).chain(NOTE_FX.iter()).find(|&dev| dev.name.to_lowercase() == lower || dev.alias == lower).map(|v| v as _)
}

/// Find device params by device name.
pub fn device_params(name: &str) -> Option<&'static [&'static str]> {
    find_device(name).map(|d| d.params)
}

/// Default drum kit: v0 Kick/Hat + v8 Snare/Clap
pub fn default_kit_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v0Kick", 36),
        ("v0Hat", 42),
        ("v8Snare", 40),
        ("v8Clap", 39),
    ]
}

/// 808 kit
pub fn kit_808_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v8Kick", 36),
        ("v8Hat", 42),
        ("v8Snare", 40),
        ("v8Clap", 39),
        ("v8Perc", 46),
    ]
}

/// 909 kit
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

/// Retro kit
pub fn kit_retro_devices() -> Vec<(&'static str, i32)> {
    vec![
        ("v0Kick", 36),
        ("v0Hat", 42),
        ("v0Cymbal", 49),
        ("v0Tom", 50),
    ]
}

/// Get kit devices by kit name.
pub fn kit_devices(kit: &str) -> Option<Vec<(&'static str, i32)>> {
    match kit {
        "default" | "" => Some(default_kit_devices()),
        "808" => Some(kit_808_devices()),
        "909" => Some(kit_909_devices()),
        "retro" => Some(kit_retro_devices()),
        _ => None,
    }
}

/// Convert catalog name to Bitwig device name.
///
/// Examples:
/// - `kick.v9` → `Some("v9Kick")`
/// - `Polymer` → `Some("Polymer")`
pub fn catalog_to_bitwig(catalog: &str) -> Option<String> {
    let lower = catalog.to_lowercase();

    // Check alias mappings first (e.g., "kick" → "v0Kick", "poly" → "Polymer")
    let alias_map: &[(&str, &str)] = &[
        ("kick", "v0Kick"), ("bd", "v0Kick"),
        ("hh", "v0Hat"), ("hat", "v0Hat"),
        ("sd", "v8Snare"), ("snare", "v8Snare"),
        ("cp", "v8Clap"), ("clap", "v8Clap"),
        ("poly", "Polymer"), ("psynth", "Polysynth"),
        ("dly1", "Delay-1"), ("dly2", "Delay-2"),
        ("filt", "Filter"), ("rev", "Reverb"),
        ("chor", "Chorus"), ("dist", "Distortion"),
        ("flang", "Flanger"), ("fshift", "Freq Shifter"),
        ("pshift", "Pitch Shifter"), ("rot", "Rotary"),
        ("sweep", "Sweep"), ("arp", "Arpeggiator"),
        ("chords", "Chords"), ("diatr", "Diatonic Transposer"),
        ("multi", "Multi-Note"), ("necho", "Note Echo"),
        ("nfilt", "Note Filter"),
    ];
    for (alias, bitwig) in alias_map {
        if lower == *alias {
            return Some(bitwig.to_string());
        }
    }

    // Direct match (case-insensitive)
    if find_device(catalog).is_some() {
        return Some(catalog.to_string());
    }

    // Parse "type.variant" format
    let (kind, variant) = catalog.split_once('.')?;

    let result = match (kind.to_lowercase().as_str(), variant.to_lowercase().as_str()) {
        ("kick", "v0") | ("kick", "") => "v0Kick",
        ("kick", "v8") | ("kick", "808") => "v8Kick",
        ("kick", "v9") | ("kick", "909") => "v9Kick",
        ("kick", "v1") => "v1Kick",
        ("hat", "v0") | ("hat", "") => "v0Hat",
        ("hat", "v8") | ("hat", "808") => "v8Hat",
        ("hat", "v9") | ("hat", "909") => "v9Hat",
        ("hat", "v1") => "v1Hat",
        ("snare", "v8") | ("snare", "808") | ("sd", "") => "v8Snare",
        ("snare", "v9") | ("snare", "909") | ("sd", "909") => "v9Snare",
        ("snare", "v1") | ("sd", "v1") => "v1Snare",
        ("clap", "v8") | ("clap", "808") | ("cp", "") => "v8Clap",
        ("clap", "v9") | ("clap", "909") | ("cp", "909") => "v9Clap",
        ("perc", "v1") => "v1Perc",
        ("perc", "v8") | ("perc", "808") => "v8Perc",
        ("ride", _) => "v9Ride",
        ("rim", _) => "v9Rim",
        ("cymb", _) | ("cy", _) => "v0Cymbal",
        ("tom", _) => "v0Tom",
        _ => return None,
    };
    Some(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_kick() {
        assert_eq!(catalog_to_bitwig("kick.v9"), Some("v9Kick".to_string()));
        assert_eq!(catalog_to_bitwig("kick.808"), Some("v8Kick".to_string()));
        assert_eq!(catalog_to_bitwig("kick"), Some("v0Kick".to_string()));
    }

    #[test]
    fn test_catalog_hat() {
        assert_eq!(catalog_to_bitwig("hat.v8"), Some("v8Hat".to_string()));
        assert_eq!(catalog_to_bitwig("hat"), Some("v0Hat".to_string()));
    }

    #[test]
    fn test_catalog_direct() {
        assert_eq!(catalog_to_bitwig("Polymer"), Some("Polymer".to_string()));
        assert_eq!(catalog_to_bitwig("Delay-2"), Some("Delay-2".to_string()));
        assert_eq!(catalog_to_bitwig("Reverb"), Some("Reverb".to_string()));
    }
}
