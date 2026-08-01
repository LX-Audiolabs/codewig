//! Pattern expansion: AST → Vec<NoteSpec>.
//!
//! Converts parsed mini-notation patterns and music commands into
//! concrete MIDI note lists for `clip.set-notes`.
//!
//! ## Note language (`n "…"`)
//! - **Spaces** separate successive events (not empty beats).
//! - **`~`** = rest (skip time, no note).
//! - Bare pitch `c` / `e` → Bitwig octave **3** (`c` = MIDI 60, same as Bitwig UI).
//! - Explicit: `c2`, `c#3`, `cis`, `eb4` (Bitwig octave numbers).
//! - Numbers `0 2 4` with key set = **scale degrees**; without key = raw MIDI 0..127.
//! - Space-separated events: each **1 beat** (steps 0,4,8,…; dur 4 on 16-grid).
//! - `[c d e f]` = one beat subdivided into 16ths (steps within that beat).
//! - Bitwig `dur` = length in **grid steps**; `setStepSize(0.25)`.

use super::ast::*;
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

/// Grid steps for one musical beat (16-grid → 4 sixteenths).
fn steps_per_beat(steps_per_bar: u32) -> u32 {
    (steps_per_bar / 4).max(1)
}

fn expand_event(
    event: &Event,
    cmd: &MusicCmd,
    scale: Option<&Scale>,
    steps_per_bar: u32,
    base_step: u32,
) -> Result<(Vec<NoteSpec>, u32), ExpandError> {
    // Bitwig setStep `dur` = length in **grid steps** (not fraction of bar).
    // Default event = 1 beat.
    let beat = steps_per_beat(steps_per_bar);
    let beat_dur = beat as f64;

    // Determine the base notes and step count for the atom
    let (base_notes, atom_steps) = match &event.atom {
        Atom::Note(name) => {
            let midi = scale::note_to_midi(name).map_err(|e| ExpandError { msg: e })?;
            (
                vec![NoteSpec {
                    step: base_step as i32,
                    key: midi,
                    vel: 100,
                    dur: beat_dur,
                    ..NoteSpec::default()
                }],
                beat,
            )
        }
        Atom::Midi(n) => {
            // With scale: small ints are scale degrees (0 = root, 2 = third, …).
            // Without scale: treat as raw MIDI 0..127.
            let key = if let Some(s) = scale {
                if (-24..24).contains(n) {
                    s.degree_to_midi(*n)
                } else {
                    (*n).clamp(0, 127)
                }
            } else if (0..=127).contains(n) {
                *n
            } else {
                return Err(ExpandError {
                    msg: format!(
                        "number {n}: set key first for scale degrees (0 2 4), or use MIDI 0..127 / note names"
                    ),
                });
            };
            (
                vec![NoteSpec {
                    step: base_step as i32,
                    key,
                    vel: 100,
                    dur: beat_dur,
                    ..NoteSpec::default()
                }],
                beat,
            )
        }
        Atom::Rest => {
            // Advance time, no audible note (filtered out on write if vel==0)
            (
                vec![NoteSpec {
                    step: base_step as i32,
                    key: 0,
                    vel: 0,
                    dur: beat_dur,
                    ..NoteSpec::default()
                }],
                beat,
            )
        }
        Atom::Group(ref seqs) => {
            // One beat wall-time → N equal grid slots (16-grid, 4 notes → steps +0..+3, dur 1 each).
            let events: Vec<&Event> = seqs.iter().flat_map(|s| s.events.iter()).collect();
            let division = events.len().max(1) as u32;
            let sub_len = (beat / division).max(1);
            let sub_dur = sub_len as f64;
            let mut all_notes = Vec::new();
            for (i, ev) in events.iter().enumerate() {
                let sub_step = base_step + i as u32 * sub_len;
                let (key, vel) = match &ev.atom {
                    Atom::Note(name) => {
                        let m = scale::note_to_midi(name).map_err(|e| ExpandError { msg: e })?;
                        (m, 100)
                    }
                    Atom::Midi(n) => {
                        let key = if let Some(s) = scale {
                            if (-24..24).contains(n) {
                                s.degree_to_midi(*n)
                            } else {
                                (*n).clamp(0, 127)
                            }
                        } else {
                            (*n).clamp(0, 127)
                        };
                        (key, 100)
                    }
                    Atom::Rest => (0, 0),
                    _ => {
                        // Nested structure: expand then re-stamp into this sub-slot
                        let (mut leaf, _) = expand_event(ev, cmd, scale, steps_per_bar, sub_step)?;
                        for n in &mut leaf {
                            n.step = sub_step as i32;
                            n.dur = sub_dur;
                        }
                        all_notes.append(&mut leaf);
                        continue;
                    }
                };
                all_notes.push(NoteSpec {
                    step: sub_step as i32,
                    key,
                    vel,
                    dur: sub_dur,
                    ..NoteSpec::default()
                });
            }
            (all_notes, beat)
        }
        Atom::Alternate(ref alts) => {
            // Alternate cycles through options. For static expansion, pick first.
            // ponytail: first alternative only until cycle state exists.
            if let Some(first) = alts.first() {
                let sub_dur = beat_dur;
                let mut all_notes = Vec::new();
                let mut offset = 0u32;
                for seq in first {
                    let (mut seq_notes, used) = expand_sequence_stepped(
                        seq,
                        cmd,
                        scale,
                        steps_per_bar,
                        base_step,
                        offset,
                        1,
                        sub_dur,
                    )?;
                    all_notes.append(&mut seq_notes);
                    offset += used;
                }
                (all_notes, beat)
            } else {
                (vec![], beat)
            }
        }
        Atom::Euclid { beats, steps, offset } => {
            let pattern = euclid_pattern(*beats, *steps, offset.unwrap_or(0));
            let mut all_notes = Vec::new();
            let step_dur = beat_dur; // each euclid cell = one beat unit of the outer grid
            for (sub_step, is_hit) in pattern.iter().enumerate() {
                if *is_hit {
                    let midi = scale
                        .map(|s| s.degree_to_midi(0))
                        .unwrap_or(scale::note_to_midi("c").unwrap_or(48));
                    all_notes.push(NoteSpec {
                        step: (base_step as i32 + sub_step as i32 * beat as i32),
                        key: midi,
                        vel: 100,
                        dur: step_dur,
                        ..NoteSpec::default()
                    });
                }
            }
            (all_notes, *steps * beat)
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
            let max_len = polys
                .iter()
                .map(|p| p.iter().map(|s| s.events.len() as u32).sum::<u32>())
                .max()
                .unwrap_or(1);
            let mut all_notes = Vec::new();
            for poly in polys.iter() {
                let mut offset = 0u32;
                for seq in poly {
                    let (mut seq_notes, _) = expand_sequence_stepped(
                        seq,
                        cmd,
                        scale,
                        steps_per_bar,
                        base_step,
                        offset,
                        1,
                        beat_dur,
                    )?;
                    all_notes.append(&mut seq_notes);
                    offset += seq.events.len() as u32;
                }
            }
            (all_notes, max_len * beat)
        }
        Atom::Subdivide(ref seqs, n) => {
            let sub_dur = beat_dur / *n as f64;
            let mut all_notes = Vec::new();
            for seq in seqs {
                let (mut seq_notes, _) = expand_sequence_stepped(
                    seq,
                    cmd,
                    scale,
                    steps_per_bar,
                    base_step,
                    0,
                    *n,
                    sub_dur,
                )?;
                all_notes.append(&mut seq_notes);
            }
            (all_notes, beat) // ponytail: subdivide fills one beat
        }
    };

    // Apply suffixes
    let mut notes = base_notes;
    for suffix in &event.suffixes {
        notes = apply_suffix(notes, suffix, &event.atom, base_step as i32, beat_dur);
    }

    Ok((notes, atom_steps))
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
                    let mut note = notes[hit_idx as usize].clone();
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
    let beat = steps_per_beat(steps_per_bar) as f64;
    let mut notes = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        let current_step = base_step + i as i32 * steps_per_beat(steps_per_bar) as i32;
        let (root, quality) = parse_chord_token(token)?;
        let root_midi = if let Some(s) = scale {
            // Try as scale degree (roman numeral) first
            parse_roman_degree(&root)
                .map(|d| s.degree_to_midi(d))
                .unwrap_or_else(|_| scale::note_to_midi(&root).unwrap_or(48))
        } else {
            scale::note_to_midi(&root).unwrap_or(48)
        };

        let intervals = chord_intervals(&quality)
            .ok_or_else(|| ExpandError { msg: format!("unknown chord quality: '{quality}'") })?;

        for &iv in intervals {
            notes.push(NoteSpec {
                step: current_step,
                key: (root_midi + iv).clamp(0, 127),
                vel: 100,
                dur: beat,
                ..NoteSpec::default()
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
                    ..NoteSpec::default()
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
                    ..NoteSpec::default()
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
                    ..NoteSpec::default()
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
                    ..NoteSpec::default()
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
            let (notes, steps) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes.len(), 3);
            // Bitwig C3=60, E3=64, G3=67; one beat each on 16-grid
            assert_eq!(notes[0].key, 60);
            assert_eq!(notes[1].key, 64);
            assert_eq!(notes[2].key, 67);
            assert_eq!(notes[0].step, 0);
            assert_eq!(notes[1].step, 4);
            assert_eq!(notes[2].step, 8);
            assert!((notes[0].dur - 4.0).abs() < f64::EPSILON);
            assert_eq!(steps, 12); // 3 beats × 4 sixteenths
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
            assert_eq!(notes[2].step, 8);
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
    fn test_expand_group_sixteenths() {
        // One beat subdivided: steps 0,1,2,3 within first beat
        let line = parse::parse_music_line(r#"bass: n "[c d e f]""#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            let (notes, steps) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes.len(), 4);
            assert_eq!(notes[0].step, 0);
            assert_eq!(notes[1].step, 1);
            assert_eq!(notes[2].step, 2);
            assert_eq!(notes[3].step, 3);
            assert!((notes[0].dur - 1.0).abs() < f64::EPSILON);
            assert_eq!(steps, 4); // one beat
        }
    }

    #[test]
    fn test_d_action_removed() {
        assert!(parse::parse_music_line(r#"kick: d "bd hh""#).is_err());
    }

    #[test]
    fn test_expand_degrees() {
        // With key: numbers = scale degrees (0=root, 2=third, 4=fifth)
        let line = parse::parse_music_line(r#"bass: n "0 2 4""#).unwrap();
        let scale = Scale::new("C", "minor").unwrap(); // root C3=48
        if let MusicLine::Music(ref cmd) = line {
            let (notes, _) = expand_music_line(cmd, Some(&scale), 16).unwrap();
            assert_eq!(notes[0].key, 60); // C3 Bitwig
            assert_eq!(notes[1].key, 63); // Eb
            assert_eq!(notes[2].key, 67); // G
        }
    }

    #[test]
    fn test_expand_with_transpose() {
        let line = parse::parse_music_line(r#"bass: n "c e g" ^2"#).unwrap();
        if let MusicLine::Music(ref cmd) = line {
            assert_eq!(cmd.transpose, Some(2));
            let (notes, _) = expand_music_line(cmd, None, 16).unwrap();
            assert_eq!(notes[0].key, 62); // D3
            assert_eq!(notes[1].key, 66); // F#3
            assert_eq!(notes[2].key, 69); // A3
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
        // Bitwig C3 triad
        let notes = expand_chord("C F G", None, 16, 0).unwrap();
        assert_eq!(notes.len(), 9);
        assert_eq!(notes[0].key, 60);
        assert_eq!(notes[1].key, 64);
        assert_eq!(notes[2].key, 67);
    }

    #[test]
    fn test_expand_chord_minor7() {
        let notes = expand_chord("Am7", None, 16, 0).unwrap();
        assert_eq!(notes.len(), 4);
        assert_eq!(notes[0].key, 69); // A3 Bitwig
        assert_eq!(notes[3].key, 79); // G4
    }

    #[test]
    fn test_expand_chord_roman() {
        let scale = Scale::new("C", "minor").unwrap();
        let notes = expand_chord("i iv v", Some(&scale), 16, 0).unwrap();
        assert_eq!(notes.len(), 9);
        assert_eq!(notes[0].key, 60);
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
            // C major triad Bitwig C3: 60, 64, 67
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
