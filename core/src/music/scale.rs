//! Scale/key system for WIGSCRIPT.
//!
//! Converts `k C minor` into usable intervals and provides
//! `degree_to_midi` for `n "0 2 4"` patterns.

use std::fmt;

/// A musical scale: root MIDI note + interval pattern.
#[derive(Debug, Clone)]
pub struct Scale {
    pub root: i32,            // MIDI note of root (e.g. C4 = 60)
    pub kind: ScaleKind,
    pub intervals: &'static [u8], // semitones between scale degrees
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    Pentatonic,
    Blues,
    Chromatic,
}

impl fmt::Display for ScaleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaleKind::Major => write!(f, "major"),
            ScaleKind::Minor => write!(f, "minor"),
            ScaleKind::Dorian => write!(f, "dorian"),
            ScaleKind::Phrygian => write!(f, "phrygian"),
            ScaleKind::Lydian => write!(f, "lydian"),
            ScaleKind::Mixolydian => write!(f, "mixolydian"),
            ScaleKind::Locrian => write!(f, "locrian"),
            ScaleKind::Pentatonic => write!(f, "pentatonic"),
            ScaleKind::Blues => write!(f, "blues"),
            ScaleKind::Chromatic => write!(f, "chromatic"),
        }
    }
}

/// All known scales: (name, kind, intervals).
pub static SCALES: &[(&str, ScaleKind, &[u8])] = &[
    ("major", ScaleKind::Major, &[2, 2, 1, 2, 2, 2, 1]),
    ("ionian", ScaleKind::Major, &[2, 2, 1, 2, 2, 2, 1]),
    ("minor", ScaleKind::Minor, &[2, 1, 2, 2, 1, 2, 2]),
    ("aeolian", ScaleKind::Minor, &[2, 1, 2, 2, 1, 2, 2]),
    ("dorian", ScaleKind::Dorian, &[2, 1, 2, 2, 2, 1, 2]),
    ("phrygian", ScaleKind::Phrygian, &[1, 2, 2, 2, 1, 2, 2]),
    ("lydian", ScaleKind::Lydian, &[2, 2, 2, 1, 2, 2, 1]),
    ("mixolydian", ScaleKind::Mixolydian, &[2, 2, 1, 2, 2, 1, 2]),
    ("locrian", ScaleKind::Locrian, &[1, 2, 2, 1, 2, 2, 2]),
    ("pentatonic", ScaleKind::Pentatonic, &[2, 2, 3, 2, 3]),
    ("blues", ScaleKind::Blues, &[3, 2, 1, 1, 3, 2]),
    ("chromatic", ScaleKind::Chromatic, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
];

/// Note name → semitone offset from C (within octave).
/// English + German accidentals (cis, des, …). `b` alone = B natural (English).
static NOTE_OFFSETS: &[(&str, i32)] = &[
    ("c", 0), ("c#", 1), ("db", 1), ("cis", 1), ("des", 1),
    ("d", 2), ("d#", 3), ("eb", 3), ("dis", 3), ("es", 3),
    ("e", 4), ("eis", 5), ("fb", 4),
    ("f", 5), ("f#", 6), ("gb", 6), ("fis", 6), ("ges", 6),
    ("g", 7), ("g#", 8), ("ab", 8), ("gis", 8), ("as", 8),
    ("a", 9), ("a#", 10), ("bb", 10), ("ais", 10), ("b", 11), ("h", 11),
];

/// Default octave when omitted: **3** in **Bitwig/Yamaha** naming.
/// Bitwig: middle C = **C3** = MIDI **60**. Formula: `offset + (octave + 2) * 12`.
pub const DEFAULT_OCTAVE: i32 = 3;

/// Convert Bitwig octave number + pitch class → MIDI.
#[inline]
fn bitwig_octave_to_midi(note_offset: i32, octave: i32) -> i32 {
    note_offset + (octave + 2) * 12
}

fn lookup_note_offset(name: &str) -> Option<i32> {
    NOTE_OFFSETS.iter().find(|(n, _)| *n == name).map(|(_, o)| *o)
}

fn lookup_scale(name: &str) -> Option<(ScaleKind, &'static [u8])> {
    SCALES.iter().find(|(n, _, _)| *n == name).map(|(_, k, i)| (*k, *i))
}

impl Scale {
    /// Create a scale from root note name and scale name.
    /// `root` e.g. "C", "Eb", "F#" — defaults to octave 4 if no octave given.
    /// `scale_name` e.g. "minor", "dorian", "major"
    pub fn new(root: &str, scale_name: &str) -> Result<Self, String> {
        let (root_note, root_octave) = parse_note_root(root)?;
        // Bitwig octave: C3 = MIDI 60
        let root_midi = bitwig_octave_to_midi(root_note, root_octave);

        let (kind, intervals) = lookup_scale(&scale_name.to_lowercase())
            .ok_or_else(|| format!("unknown scale: '{scale_name}'. Known: major, minor, dorian, phrygian, lydian, mixolydian, locrian, pentatonic, blues, chromatic"))?;

        Ok(Self {
            root: root_midi,
            kind,
            intervals,
        })
    }

    /// Convert a scale degree to a MIDI note.
    /// Degree 0 = root, 1 = second, -1 = 7th below, etc.
    pub fn degree_to_midi(&self, degree: i32) -> i32 {
        let len = self.intervals.len() as i32;
        if len == 0 { return self.root; }

        // Handle wrapping
    let octave_shift;
    let idx;
        if degree < 0 {
            octave_shift = (degree + 1) / len - 1;
            idx = ((degree % len) + len) % len;
        } else {
            octave_shift = degree / len;
            idx = degree % len;
        }

        let semitones: i32 = self.intervals[..idx as usize].iter().map(|&s| s as i32).sum();
        self.root + semitones + octave_shift * 12
    }

    /// The number of degrees in this scale.
    pub fn len(&self) -> usize {
        self.intervals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Transpose within the scale: move `steps` degrees up.
    pub fn scale_transpose(&self, midi: i32, steps: i32) -> i32 {
        // Find which degree this MIDI note corresponds to
        // Simple approach: find nearest scale note, then shift
        let mut best_degree = 0;
        let mut best_dist = i32::MAX;
        for d in -12..24 {
            let note = self.degree_to_midi(d);
            let dist = (midi - note).abs();
            if dist < best_dist {
                best_dist = dist;
                best_degree = d;
            }
        }
        self.degree_to_midi(best_degree + steps)
    }
}

/// Parse a note name like "C", "Eb", "F#3" into (semitones from C, octave).
fn parse_note_root(name: &str) -> Result<(i32, i32), String> {
    let s = name.trim();
    let chars: Vec<char> = s.chars().collect();

    // Find the split between note name and octave number
    let mut note_end = chars.len();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_digit() && i > 0 {
            note_end = i;
            break;
        }
    }

    let note_part: String = chars[..note_end].iter().collect();
    let octave_str: String = chars[note_end..].iter().collect();

    let note_offset = lookup_note_offset(&note_part.to_lowercase())
        .ok_or_else(|| {
            format!("unknown note: '{note_part}'. Use c, c#, cis, db, d, … + optional octave (c3)")
        })?;

    let octave: i32 = if octave_str.is_empty() {
        DEFAULT_OCTAVE
    } else {
        octave_str
            .parse()
            .map_err(|_| format!("invalid octave: '{octave_str}'"))?
    };

    Ok((note_offset, octave))
}

/// Parse mini-notation pitch: `c`, `c#`, `cis`, `eb3`, `f#4` → MIDI.
///
/// **Bitwig octave names:** bare `c` = **C3** = MIDI **60**.
/// Explicit: `c2`, `c3`, `c4` (same numbers Bitwig shows).
pub fn note_to_midi(name: &str) -> Result<i32, String> {
    let s = name.trim();
    let chars: Vec<char> = s.chars().collect();

    let mut note_end = chars.len();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_digit() && i > 0 {
            note_end = i;
            break;
        }
    }

    let note_part: String = chars[..note_end].iter().collect();
    let octave_str: String = chars[note_end..].iter().collect();

    let note_offset = lookup_note_offset(&note_part.to_lowercase()).ok_or_else(|| {
        format!("unknown note: '{note_part}'. Use c, c#/cis, db/des, d, … or MIDI 0..127")
    })?;

    let octave: i32 = if octave_str.is_empty() {
        DEFAULT_OCTAVE
    } else {
        octave_str
            .parse()
            .map_err(|_| format!("invalid octave in '{s}'"))?
    };

    let midi = bitwig_octave_to_midi(note_offset, octave);
    if !(0..=127).contains(&midi) {
        return Err(format!("note '{s}' → MIDI {midi} out of range 0..127"));
    }
    Ok(midi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_c_minor() {
        // Bitwig: bare C = C3 = MIDI 60
        let scale = Scale::new("C", "minor").unwrap();
        assert_eq!(scale.degree_to_midi(0), 60);
        assert_eq!(scale.degree_to_midi(1), 62);
        assert_eq!(scale.degree_to_midi(2), 63); // Eb
        assert_eq!(scale.degree_to_midi(3), 65);
        assert_eq!(scale.degree_to_midi(7), 72); // C4 Bitwig
    }

    #[test]
    fn test_scale_d_major() {
        let scale = Scale::new("D", "major").unwrap();
        assert_eq!(scale.degree_to_midi(0), 62); // D3 Bitwig
        assert_eq!(scale.degree_to_midi(1), 64);
        assert_eq!(scale.degree_to_midi(2), 66);
    }

    #[test]
    fn test_note_to_midi_bitwig() {
        assert_eq!(note_to_midi("c").unwrap(), 60); // Bitwig C3
        assert_eq!(note_to_midi("c3").unwrap(), 60);
        assert_eq!(note_to_midi("c4").unwrap(), 72);
        assert_eq!(note_to_midi("c2").unwrap(), 48);
        assert_eq!(note_to_midi("c#").unwrap(), 61);
        assert_eq!(note_to_midi("cis").unwrap(), 61);
        assert_eq!(note_to_midi("cis2").unwrap(), 49);
        assert_eq!(note_to_midi("eb3").unwrap(), 63);
        assert_eq!(note_to_midi("f#").unwrap(), 66); // F#3 Bitwig
    }

    #[test]
    fn test_scale_new_flat_root() {
        let scale = Scale::new("Eb", "major").unwrap();
        assert_eq!(scale.degree_to_midi(0), 63); // Eb3 Bitwig
    }

    #[test]
    fn test_scale_transpose() {
        let scale = Scale::new("C", "major").unwrap();
        let midi = scale.degree_to_midi(0); // 60
        let up_one = scale.scale_transpose(midi, 1);
        assert_eq!(up_one, 62);
    }
}
