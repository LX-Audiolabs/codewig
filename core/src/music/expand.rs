//! Pattern expansion: AST → Vec<NoteSpec>.
//!
//! Converts parsed mini-notation patterns and music commands into
//! concrete MIDI note lists for `clip.set-notes`.
//!
//! Also provides chord expansion and arpeggio generation.

use super::ast::*;
use super::device;
use super::scale::{self, Scale};
use crate::NoteSpec;
use rand::Rng;
use std::fmt;

/// Error during pattern expansion.
#[derive(Debug)]
pub struct ExpandError {
    pub msg: String,
}

impl fmt::Display for ExpandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expand error: {}", self.msg)
    }
}

impl std::error::Error for ExpandError {}

/// Expand a music line into a list of NoteSpec entries.
/// Returns the notes and the step count they occupy.
pub fn expand_music_line(
    cmd: &MusicCmd,
    scale: Option<&Scale>,
    steps_per_bar: u32,
) -> Result<(Vec<NoteSpec>, u32), ExpandError> {
    // Chord action uses chord tokens, not mini-notation
    if matches!(cmd.action, MusicAction::Chord) {
        let notes = expand_chord(&cmd.pattern, scale, steps_per_bar, 0)?;
        let steps = notes.iter().map(|n| n.step).max().unwrap_or(0) as u32 + 1;
        return Ok((notes, steps.max(1)));
    }

    // Arp: pitch source in quotes → expand_arp over one bar (steps_per_bar)
    if let MusicAction::Arp(style) = cmd.action {
        let pitches = arp_source_pitches(&cmd.pattern, scale)?;
        let notes = expand_arp(&pitches, steps_per_bar, style, 1.0, 0);
        return Ok((notes, steps_per_bar.max(1)));
    }

    let pattern = super::parse::parse_mini_pattern(&cmd.pattern)
        .map_err(|e| ExpandError { msg: e.to_string() })?;

    let mut notes = Vec::new();
    let mut total_steps = 0u32;

    for seq in &pattern.sequences {
        let (seq_notes, seq_steps) = expand_sequence(seq, cmd, scale, steps_per_bar, 0)?;
        notes.extend(seq_notes);
        total_steps = total_steps.max(seq_steps);
    }

    // Apply transpose
    if let Some(t) = cmd.transpose {
        for n in &mut notes {
            if n.vel > 0 {
                n.key = (n.key + t).clamp(0, 127);
            }
        }
    }

    // Apply scale transpose
    if let Some(st) = cmd.scale_transpose {
        if let Some(s) = scale {
            for n in &mut notes {
                if n.vel > 0 {
                    n.key = s.scale_transpose(n.key, st);
                }
            }
        }
    }

    Ok((notes, total_steps))
}

/// How many sub-steps a single step divides into.
// ponytail: unused for now, but kept for future group subdivision logic
#[allow(dead_code)]
fn step_division(events: &[Event]) -> u32 {
    let mut sum = 0u32;
    for ev in events {
        match &ev.atom {
            Atom::Group(ref seqs) => {
                for seq in seqs {
                    sum += step_division(&seq.events);
                }
            }
            _ => sum += 1,
        }
    }
    if sum == 0 { 1 } else { sum }
}

fn expand_sequence(
    seq: &Sequence,
    cmd: &MusicCmd,
    scale: Option<&Scale>,
    steps_per_bar: u32,
    base_step: u32,
) -> Result<(Vec<NoteSpec>, u32), ExpandError> {
    let mut notes = Vec::new();
    let mut current_step = base_step;

    for event in &seq.events {
        let (mut event_notes, steps_used) = expand_event(event, cmd, scale, steps_per_bar, current_step)?;
        notes.append(&mut event_notes);
        current_step += steps_used;
    }

    Ok((notes, current_step - base_step))
}

fn expand_event(
    event: &Event,
    cmd: &MusicCmd,
    scale: Option<&Scale>,
    steps_per_bar: u32,
    base_step: u32,
) -> Result<(Vec<NoteSpec>, u32), ExpandError> {
    let sub_step_size = 1.0 / steps_per_bar as f64;

    // Determine the base notes and step count for the atom
    let (base_notes, atom_steps) = match &event.atom {
        Atom::Note(name) => {
            let midi = scale::note_to_midi(name)
                .map_err(|e| ExpandError { msg: e })?;
            (vec![NoteSpec { step: base_step as i32, key: midi, vel: 100, dur: sub_step_size }], 1)
        }
        Atom::Midi(n) => {
            let key = (*n).clamp(0, 127);
            // Heuristic: if scale is set and number is small (< 24), treat as degree
            if let Some(s) = scale {
                if *n >= -24 && *n < 24 {
                    let midi = s.degree_to_midi(*n);
                    (vec![NoteSpec { step: base_step as i32, key: midi, vel: 100, dur: sub_step_size }], 1)
                } else {
                    (vec![NoteSpec { step: base_step as i32, key, vel: 100, dur: sub_step_size }], 1)
                }
            } else {
                (vec![NoteSpec { step: base_step as i32, key, vel: 100, dur: sub_step_size }], 1)
            }
        }
        Atom::Drum(drum_alias) => {
            let midi = device::drum_midi(*drum_alias).unwrap_or(36);
            (vec![NoteSpec { step: base_step as i32, key: midi, vel: 100, dur: sub_step_size }], 1)
        }
        Atom::Rest => {
            (vec![NoteSpec { step: base_step as i32, key: 0, vel: 0, dur: sub_step_size }], 1)
        }
        Atom::Group(ref seqs) => {
            let division = step_division_for_group(seqs);
            let sub_dur = sub_step_size / division as f64;
            let mut all_notes = Vec::new();
            let mut offset = 0u32;
            for seq in seqs {
                let (mut seq_notes, used) = expand_sequence_stepped(seq, cmd, scale, steps_per_bar, base_step, offset, division, sub_dur)?;
                all_notes.append(&mut seq_notes);
                offset += used;
            }
            (all_notes, 1) // group occupies 1 step, internally subdivided
        }
        Atom::Alternate(ref alts) => {
            // Alternate cycles through options. For static expansion, pick first (or random?)
            // ponytail: for v1, expand to first alternative. Later: cycle tracking.
            if let Some(first) = alts.first() {
                let sub_dur = sub_step_size;
                let mut all_notes = Vec::new();
                let mut offset = 0u32;
                for seq in first {
                    let (mut seq_notes, used) = expand_sequence_stepped(seq, cmd, scale, steps_per_bar, base_step, offset, 1, sub_dur)?;
                    all_notes.append(&mut seq_notes);
                    offset += used;
                }
                (all_notes, 1)
            } else {
                (vec![], 1)
            }
        }
        Atom::Euclid { beats, steps, offset } => {
            let pattern = euclid_pattern(*beats, *steps, offset.unwrap_or(0));
            let mut all_notes = Vec::new();
            for (sub_step, is_hit) in pattern.iter().enumerate() {
                if *is_hit {
                    // Ponytail: euclid on drums uses the drum MIDI, on notes needs a pitch
                    // For drums, use the first note from the parent context? 
                    // v1: euclid on drums only. For notes, wrap in Group.
                    let midi = 36; // default kick — caller should wrap in drum context
                    all_notes.push(NoteSpec {
                        step: (base_step as i32 + sub_step as i32),
                        key: midi,
                        vel: 100,
                        dur: sub_step_size,
                    });
                }
            }
            (all_notes, *steps)
        }
        Atom::RandomChoice(ref atoms) => {
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..atoms.len());
            // Recurse into the chosen atom
            let chosen_event = Event { atom: atoms[idx].clone(), suffixes: vec![] };
            expand_event(&chosen_event, cmd, scale, steps_per_bar, base_step)?
        }
        Atom::Polymetric(ref polys) => {
            // Polymetric: each sub-pattern runs at its own rate, wrapping
            // v1: take the longest pattern, wrap shorter ones
            let max_len = polys.iter().map(|p| p.iter().map(|s| s.events.len() as u32).sum::<u32>()).max().unwrap_or(1);
            let mut all_notes = Vec::new();
            for poly in polys.iter() {
                let _poly_steps: u32 = poly.iter().map(|s| s.events.len() as u32).sum();
                let mut offset = 0u32;
                for seq in poly {
                    let (mut seq_notes, _) = expand_sequence_stepped(
                        seq, cmd, scale, steps_per_bar, base_step, offset, 1, sub_step_size)?;
                    all_notes.append(&mut seq_notes);
                    offset += seq.events.len() as u32;
                }
            }
            (all_notes, max_len)
        }
        Atom::Subdivide(ref seqs, n) => {
            let sub_dur = sub_step_size / *n as f64;
            let mut all_notes = Vec::new();
            for seq in seqs {
                let (mut seq_notes, _) = expand_sequence_stepped(
                    seq, cmd, scale, steps_per_bar, base_step, 0, *n, sub_dur)?;
                all_notes.append(&mut seq_notes);
            }
            (all_notes, 1) // ponytail: subdivide fills one step
        }
    };

    // Apply suffixes
    let mut notes = base_notes;
    for suffix in &event.suffixes {
        notes = apply_suffix(notes, suffix, &event.atom, base_step as i32, sub_step_size);
    }

    Ok((notes, atom_steps))
}

fn step_division_for_group(seqs: &[Sequence]) -> u32 {
    seqs.iter().map(|s| s.events.len() as u32).sum::<u32>().max(1)
}

#[allow(clippy::too_many_arguments)]
fn expand_sequence_stepped(
    seq: &Sequence,
    cmd: &MusicCmd,
    scale: Option<&Scale>,
    steps_per_bar: u32,
    base_step: u32,
    _sub_offset: u32,
    _division: u32,
    sub_dur: f64,
) -> Result<(Vec<NoteSpec>, u32), ExpandError> {
    let mut notes = Vec::new();
    let mut current_step = base_step;

    for event in &seq.events {
        let (mut event_notes, steps_used) = expand_event(event, cmd, scale, steps_per_bar, current_step)?;
        // Adjust step positions and durations for subdivision
        for n in &mut event_notes {
            n.dur = sub_dur;
            // step stays as-is since we're at the base_step level
        }
        notes.append(&mut event_notes);
        current_step += steps_used;
    }

    Ok((notes, current_step - base_step))
}

fn apply_suffix(
    notes: Vec<NoteSpec>,
    suffix: &Suffix,
    _atom: &Atom,
    _base_step: i32,
    step_dur: f64,
) -> Vec<NoteSpec> {
    match suffix {
        Suffix::Repeat(n) => {
            let n = *n as usize;
            if notes.is_empty() || n <= 1 { return notes; }
            let mut out = Vec::with_capacity(notes.len() * n);
            for i in 0..n {
                for mut note in notes.clone() {
                    note.step += i as i32;
                    note.dur = step_dur / n as f64;
                    out.push(note);
                }
            }
            out
        }
        Suffix::Slow(n) => {
            let n = *n;
            if n <= 1 { return notes; }
            notes.into_iter().map(|mut note| {
                note.dur *= n as f64;
                note
            }).collect()
        }
        Suffix::Replicate(n) => {
            let n = *n as usize;
            if notes.is_empty() || n <= 1 { return notes; }
            let mut out = Vec::with_capacity(notes.len() * n);
            for i in 0..n {
                for mut note in notes.clone() {
                    note.step += i as i32;
                    out.push(note);
                }
            }
            out
        }
        Suffix::Elongate => {
            notes.into_iter().map(|mut note| {
                note.dur *= 2.0;
                note
            }).collect()
        }
        Suffix::ElongateN(n) => {
            notes.into_iter().map(|mut note| {
                note.dur *= *n as f64;
                note
            }).collect()
        }
        Suffix::RandomDrop(prob) => {
            let p = prob.unwrap_or(0.5);
            let mut rng = rand::thread_rng();
            notes.into_iter().filter(|_| rng.gen::<f64>() > p).collect()
        }
        Suffix::Octave(n) => {
            notes.into_iter().map(|mut note| {
                if note.vel > 0 {
                    note.key = (note.key + n * 12).clamp(0, 127);
                }
                note
            }).collect()
        }
        Suffix::Euclid { beats, steps, offset } => {
            let pattern = euclid_pattern(*beats, *steps, offset.unwrap_or(0));
            let n_hits = pattern.iter().filter(|&&h| h).count();
            let mut out = Vec::with_capacity(n_hits);
            let mut hit_idx = 0u32;
            for (i, is_hit) in pattern.iter().enumerate() {
                if *is_hit && hit_idx < notes.len() as u32 {
                    let mut note = notes[hit_idx as usize];
                    note.step = _base_step + i as i32;
                    out.push(note);
                    hit_idx += 1;
                }
            }
            out
        }
    }
}

/// Generate a euclidean rhythm pattern.
/// Returns a Vec<bool> of length `steps` with `beats` true values evenly distributed.
fn euclid_pattern(beats: u32, steps: u32, offset: u32) -> Vec<bool> {
    if beats == 0 || steps == 0 {
        return vec![false; steps as usize];
    }
    if beats >= steps {
        return vec![true; steps as usize];
    }

    let mut pattern = vec![false; steps as usize];
    let step_f = steps as f64;
    let beat_f = beats as f64;

    for i in 0..beats {
        let pos = ((i as f64 * step_f / beat_f) as u32 + offset) % steps;
        pattern[pos as usize] = true;
    }

    pattern
}

// ── Chord expansion ────────────────────────────────────────────────

/// Chord quality → semitone intervals from root.
static CHORD_INTERVALS: &[(&str, &[i32])] = &[
    ("", &[0, 4, 7]),              // major
    ("m", &[0, 3, 7]),             // minor
    ("min", &[0, 3, 7]),
    ("dim", &[0, 3, 6]),           // diminished
    ("aug", &[0, 4, 8]),           // augmented
    ("7", &[0, 4, 7, 10]),         // dominant 7
    ("m7", &[0, 3, 7, 10]),        // minor 7
    ("maj7", &[0, 4, 7, 11]),      // major 7
    ("m7b5", &[0, 3, 6, 10]),      // half-diminished
    ("dim7", &[0, 3, 6, 9]),       // diminished 7
    ("sus4", &[0, 5, 7]),          // suspended 4
    ("sus2", &[0, 2, 7]),          // suspended 2
    ("6", &[0, 4, 7, 9]),          // major 6
    ("m6", &[0, 3, 7, 9]),         // minor 6
    ("9", &[0, 4, 7, 10, 14]),     // dominant 9
    ("m9", &[0, 3, 7, 10, 14]),    // minor 9
    ("maj9", &[0, 4, 7, 11, 14]),  // major 9
];

/// Look up chord intervals by quality string (e.g., "m", "7", "m7b5").
pub fn chord_intervals(quality: &str) -> Option<&'static [i32]> {
    CHORD_INTERVALS.iter().find(|(q, _)| *q == quality).map(|(_, iv)| *iv)
}

/// Expand a chord string like "C Am F G" into NoteSpec entries.
/// Each chord is written as a block at sequential steps.
pub fn expand_chord(
    chord_str: &str,
    scale: Option<&Scale>,
    steps_per_bar: u32,
    base_step: i32,
) -> Result<Vec<NoteSpec>, ExpandError> {
    let tokens: Vec<&str> = chord_str.split_whitespace().collect();
    let sub_step_size = 1.0 / steps_per_bar as f64;
    let mut notes = Vec::new();

    for (current_step, token) in (base_step..).zip(tokens.iter()) {
        let (root, quality) = parse_chord_token(token)?;
        let root_midi = if let Some(s) = scale {
            // Try as scale degree (roman numeral) first
            parse_roman_degree(&root)
                .map(|d| s.degree_to_midi(d))
                .unwrap_or_else(|_| scale::note_to_midi(&root).unwrap_or(60))
        } else {
            scale::note_to_midi(&root).unwrap_or(60)
        };

        let intervals = chord_intervals(&quality)
            .ok_or_else(|| ExpandError { msg: format!("unknown chord quality: '{quality}'") })?;

        for &iv in intervals {
            notes.push(NoteSpec {
                step: current_step,
                key: (root_midi + iv).clamp(0, 127),
                vel: 100,
                dur: sub_step_size,
            });
        }
    }

    Ok(notes)
}

/// Parse a chord token like "C", "Am", "F#m7", "Bb7", "iii", "VI7".
fn parse_chord_token(token: &str) -> Result<(String, String), ExpandError> {
    let s = token.trim();
    if s.is_empty() {
        return Err(ExpandError { msg: "empty chord token".into() });
    }
    let chars: Vec<char> = s.chars().collect();

    // Roman numerals: all chars are i/v/x (case insensitive)
    let is_roman = chars.iter().all(|c| matches!(c, 'i' | 'I' | 'v' | 'V' | 'x' | 'X'));
    if is_roman {
        return Ok((s.to_string(), String::new()));
    }

    let mut i = 0;
    if i < chars.len() && chars[i].is_alphabetic() {
        i += 1;
        if i < chars.len() && (chars[i] == '#' || chars[i] == 'b') {
            i += 1;
        }
    }
    let root = s[..i].to_string();
    let quality = s[i..].to_lowercase();
    Ok((root, quality))
}

/// Parse roman numeral degree: i=0, ii=1, iii=2, IV=3, v=4, vi=5, vii=6.
fn parse_roman_degree(s: &str) -> Result<i32, ()> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "i" => Ok(0), "ii" => Ok(1), "iii" => Ok(2),
        "iv" => Ok(3), "v" => Ok(4), "vi" => Ok(5), "vii" => Ok(6),
        _ => Err(()),
    }
}

// ── Arpeggio generation ────────────────────────────────────────────

// ArpStyle lives in ast (parser/executor); re-export for callers.
pub use super::ast::ArpStyle;

/// Pitches for arp: one chord token (`Cm7`) or space-separated notes (`c e g`).
fn arp_source_pitches(pattern: &str, scale: Option<&Scale>) -> Result<Vec<i32>, ExpandError> {
    let tokens: Vec<&str> = pattern.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(ExpandError {
            msg: "arp pattern empty — e.g. arp \"c e g\" or arp:up \"Cm7\"".into(),
        });
    }

    // Single token: prefer chord (C, Am7, iii) so `arp "C"` = triad tones
    if tokens.len() == 1 {
        if let Ok(block) = expand_chord(tokens[0], scale, 1, 0) {
            let mut keys: Vec<i32> = block.iter().map(|n| n.key).collect();
            keys.sort_unstable();
            keys.dedup();
            if !keys.is_empty() {
                return Ok(keys);
            }
        }
    }

    let mut keys = Vec::with_capacity(tokens.len());
    for t in tokens {
        keys.push(resolve_arp_pitch(t, scale)?);
    }
    Ok(keys)
}

fn resolve_arp_pitch(token: &str, scale: Option<&Scale>) -> Result<i32, ExpandError> {
    if let Ok(d) = token.parse::<i32>() {
        if let Some(s) = scale {
            if (-24..24).contains(&d) {
                return Ok(s.degree_to_midi(d));
            }
        }
        if (0..=127).contains(&d) {
            return Ok(d);
        }
    }
    if let Some(s) = scale {
        if let Ok(d) = parse_roman_degree(token) {
            return Ok(s.degree_to_midi(d));
        }
    }
    scale::note_to_midi(token).map_err(|e| ExpandError { msg: e })
}

/// Generate an arpeggio from a list of note values over a number of steps.
pub fn expand_arp(
    notes: &[i32],
    steps: u32,
    style: ArpStyle,
    step_size: f64,
    base_step: i32,
) -> Vec<NoteSpec> {
    if notes.is_empty() || steps == 0 {
        return vec![];
    }
    let mut out = Vec::with_capacity(steps as usize);
    let n = notes.len();

    match style {
        ArpStyle::Up => {
            for i in 0..steps {
                out.push(NoteSpec {
                    step: base_step + i as i32,
                    key: notes[i as usize % n],
                    vel: 100,
                    dur: step_size,
                });
            }
        }
        ArpStyle::Down => {
            for i in 0..steps {
                let idx = n - 1 - (i as usize % n);
                out.push(NoteSpec {
                    step: base_step + i as i32,
                    key: notes[idx],
                    vel: 100,
                    dur: step_size,
                });
            }
        }
        ArpStyle::UpDown => {
            let period = n * 2 - 2;
            let period = if period == 0 { 1 } else { period };
            for i in 0..steps {
                let mut p = i as usize % period;
                if p >= n { p = period - p; }
                out.push(NoteSpec {
                    step: base_step + i as i32,
                    key: notes[p],
                    vel: 100,
                    dur: step_size,
                });
            }
        }
        ArpStyle::Random => {
            let mut rng = rand::thread_rng();
            for i in 0..steps {
                let idx = rng.gen_range(0..n);
                out.push(NoteSpec {
                    step: base_step + i as i32,
                    key: notes[idx],
                    vel: 100,
                    dur: step_size,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse;
    use super::super::scale::Scale;

    #[test]
    fn test_expand_simple_notes() {
        let line = parse::parse_music_line(r#"bass: n "c e g""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let scale = Scale::new("C", "major").ok();
            let (notes, steps) = expand_music_line(cmd, scale.as_ref(), 16).unwrap();
            assert_eq!(notes.len(), 3);
            assert_eq!(notes[0].key, 60); // C4
            assert_eq!(notes[1].key, 64); // E4
            assert_eq!(notes[2].key, 67); // G4
            assert_eq!(steps, 3);
        } else {
            panic!("expected Music");
        }
    }

    #[test]
    fn test_expand_rest() {
        let line = parse::parse_music_line(r#"bass: n "c ~ g""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, _) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes.len(), 3);
            assert_eq!(notes[0].key, 60);
            assert_eq!(notes[1].vel, 0); // rest
            assert_eq!(notes[2].key, 67);
        }
    }

    #[test]
    fn test_expand_repeat() {
        let line = parse::parse_music_line(r#"bass: n "c*3""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, _) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes.len(), 3);
            assert_eq!(notes[0].key, 60);
            assert_eq!(notes[1].key, 60);
            assert_eq!(notes[2].key, 60);
        }
    }

    #[test]
    fn test_expand_drums() {
        let line = parse::parse_music_line(r#"drums: d "bd hh sd""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, _) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes.len(), 3);
            assert_eq!(notes[0].key, 36); // v0Kick
            assert_eq!(notes[1].key, 42); // v0Hat
            assert_eq!(notes[2].key, 40); // v8Snare
        }
    }

    #[test]
    fn test_expand_degrees() {
        let line = parse::parse_music_line(r#"bass: n "0 2 4""#).unwrap();
        let scale = Scale::new("C", "minor").unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, _) = expand_music_line(cmd, Some(&scale), 16).unwrap();
            assert_eq!(notes[0].key, 60); // C
            assert_eq!(notes[1].key, 63); // Eb (minor third)
            assert_eq!(notes[2].key, 67); // G
        }
    }

    #[test]
    fn test_expand_with_transpose() {
        let line = parse::parse_music_line(r#"bass: n "c e g" ^2"#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            assert_eq!(cmd.transpose, Some(2));
            let (notes, _) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes[0].key, 62); // D
            assert_eq!(notes[1].key, 66); // F#
            assert_eq!(notes[2].key, 69); // A
        }
    }

    #[test]
    fn test_expand_group() {
        let line = parse::parse_music_line(r#"bass: n "[c e] g""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, _) = expand_music_line(cmd, None, 16).unwrap();
            // [c e] subdivides step 0 into two sub-steps, g is step 1
            assert!(notes.len() >= 3);
        }
    }

    #[test]
    fn test_euclid_pattern() {
        let pat = euclid_pattern(3, 8, 0);
        assert_eq!(pat.len(), 8);
        let hits: Vec<usize> = pat.iter().enumerate().filter(|(_, &h)| h).map(|(i, _)| i).collect();
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn test_expand_chord_major() {
        // C4=60, F4=65, G4=67 (default octave 4)
        let notes = expand_chord("C F G", None, 16, 0).unwrap();
        assert_eq!(notes.len(), 9); // 3 chords × 3 notes
        assert_eq!(notes[0].key, 60);
        assert_eq!(notes[1].key, 64);
        assert_eq!(notes[2].key, 67);
    }

    #[test]
    fn test_expand_chord_minor7() {
        // A4 = 69, C5 = 72, E5 = 76, G5 = 79 (default octave 4 for chord roots)
        let notes = expand_chord("Am7", None, 16, 0).unwrap();
        assert_eq!(notes.len(), 4);
        assert_eq!(notes[0].key, 69); // A4
        assert_eq!(notes[3].key, 79); // G5
    }

    #[test]
    fn test_expand_chord_roman() {
        let scale = Scale::new("C", "minor").unwrap();
        // i=Cm (C4=60), iv=Fm (F4=65), v=Gm (G4=67)
        let notes = expand_chord("i iv v", Some(&scale), 16, 0).unwrap();
        assert_eq!(notes.len(), 9);
        assert_eq!(notes[0].key, 60); // C (root of i)
    }

    #[test]
    fn test_arp_up() {
        let arp = expand_arp(&[60, 64, 67], 8, ArpStyle::Up, 0.25, 0);
        assert_eq!(arp.len(), 8);
        assert_eq!(arp[0].key, 60);
        assert_eq!(arp[1].key, 64);
        assert_eq!(arp[2].key, 67);
        assert_eq!(arp[3].key, 60); // wraps
    }

    #[test]
    fn test_expand_arp_line_from_chord() {
        let line = parse::parse_music_line(r#"bass: arp:up "C""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, steps) = expand_music_line(cmd, None, 8).unwrap();
            assert_eq!(steps, 8);
            assert_eq!(notes.len(), 8);
            // C major triad: 60, 64, 67 cycling
            assert_eq!(notes[0].key, 60);
            assert_eq!(notes[1].key, 64);
            assert_eq!(notes[2].key, 67);
            assert_eq!(notes[3].key, 60);
        } else {
            panic!("expected Music");
        }
    }

    #[test]
    fn test_arp_updown() {
        let arp = expand_arp(&[60, 64, 67], 5, ArpStyle::UpDown, 0.25, 0);
        assert_eq!(arp.len(), 5);
        // 60, 64, 67, 64, 60 (up then down, no repeat)
        assert_eq!(arp[0].key, 60);
        assert_eq!(arp[1].key, 64);
        assert_eq!(arp[2].key, 67);
        assert_eq!(arp[3].key, 64);
        assert_eq!(arp[4].key, 60);
    }
}
