//! AST types for WIGSCRIPT music commands.

/// A parsed line from the `♫` REPL.
#[derive(Debug, Clone, PartialEq)]
pub enum MusicLine {
    /// `bass: n "c e g" +cutoff:0.3`
    Music(MusicCmd),
    /// `!bass Polymer Filter Delay-2`
    Chain(ChainCmd),
    /// `new track(kick).device(kick.v9).beat(4_).mute().clip(start)`
    Fluent(FluentCmd),
    /// `k C minor`
    Key { root: String, scale: String },
    /// `play` | `stop`
    Transport(TransportCmd),
    /// `tempo 128`
    Tempo(f64),
    /// `t(kick).d(kick.v9): decay(280) punch(45)`
    Param(ParamCmd),
    /// `mute(kick)` | `mute(kick, bass)` | `mute(1,3,5)`
    Mute(MuteCmd),
    /// `unmute(kick)` | `unmute(1,3,5)`
    Unmute(MuteCmd),
    /// `s(1).start` | `s(verse).stop` — scene launch/stop
    Scene(SceneCmd),
    /// `new scene` | `new scene()` | `new scene(verse)` — name a scene row (Bitwig launcher)
    NewScene { name: Option<String> },
    /// `s(verse).t(lead).c(new)` | `s(1).t(lead).c(new, A)` — clip at track × scene
    SceneTrackClip(SceneTrackClipCmd),
    /// `c(track.0).start` | `c(track.0).stop` — clip launch/stop
    ClipCtrl(ClipCtrlCmd),
    /// `> bass` — Codewig passthrough
    PassThrough(String),
    /// `mode cmd` | `mode music`
    ModeSwitch(String),
    /// Empty / comment
    Empty,
}

/// Fluent creation/update: `new track(kick).device(kick.v9).beat(4_).mute().clip(start)`
#[derive(Debug, Clone, PartialEq)]
pub struct FluentCmd {
    pub create: bool,                   // true for "new track(...)", false for "t(...)"
    pub track: String,
    pub steps: Vec<FluentStep>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FluentStep {
    /// `.device(Polymer)` / `.device(kick.v9)` — insert by name (open resolve)
    Device(DeviceSpec),
    /// .add(Delay+)
    Add(DeviceSpec),
    /// .beat(4_) or .beat:16(1,5,11,14)
    Beat(BeatSpec),
    /// .n("c e g")  — mini-notation for synth tracks
    Pattern { pattern: String, mods: NoteMods },
    /// .mute()
    Mute,
    /// .clip(start) or .clip(stop) — launch/stop current clip on track
    ClipAction(ClipAction),
    /// .c(0).start or .c(0,1).start — launch specific clip slot(s)
    ClipCtrl(ClipCtrlCmd),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceSpec {
    pub catalog_name: String,   // "kick.v9", "hat.v8", "Polymer", "Delay-2"
}

#[derive(Debug, Clone, PartialEq)]
pub enum BeatSpec {
    /// `4_` — 4-on-the-floor: every quarter (musician 1-based **1, 5, 9, 13**).
    /// Stored 0-based 0,4,8,12. Duration: **1 beat** (= 4 sixteenths).
    FourToFloor,
    /// `2_4` — half notes: 1-based 1, 9 → 0-based 0, 8; duration 2 beats.
    HalfNotes,
    /// `off` — off-beats: musician 1-based **3, 7, 11, 15** → 0-based 2,6,10,14.
    /// Duration: 1 beat each.
    Offbeat,
    /// `bk2` — sparse: 1-based 1, 9 → 0, 8; duration 2 beats.
    Break2,
    /// `beat:16(0,4,8,12)` — explicit 0-based steps (or 1-based 1..grid, normalized).
    Explicit { grid: u32, positions: Vec<u32> },
}

impl BeatSpec {
    /// Hit start positions on the 16th-note grid (0-based).
    pub fn steps(&self) -> Vec<u32> {
        match self {
            // 4-on-the-floor: every 4th 16th = quarter notes
            BeatSpec::FourToFloor => vec![0, 4, 8, 12],
            BeatSpec::HalfNotes => vec![0, 8],
            BeatSpec::Offbeat => vec![2, 6, 10, 14],
            BeatSpec::Break2 => vec![0, 8],
            BeatSpec::Explicit { positions, .. } => positions.clone(),
        }
    }

    /// Grid size in 16th steps per bar (default 16).
    pub fn grid(&self) -> u32 {
        match self {
            BeatSpec::Explicit { grid, .. } => *grid,
            _ => 16,
        }
    }

    /// Note length in **grid steps** (Bitwig clip step units).
    /// One musical beat at 16-grid = 4 steps; half note = 8.
    pub fn hit_duration_steps(&self) -> f64 {
        let g = self.grid() as f64;
        match self {
            BeatSpec::FourToFloor | BeatSpec::Offbeat => g / 4.0, // quarter note
            BeatSpec::HalfNotes | BeatSpec::Break2 => g / 2.0,    // half note
            BeatSpec::Explicit { .. } => g / 4.0,                 // default one beat
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClipAction {
    Start,
    Stop,
}

#[cfg(test)]
mod beat_tests {
    use super::*;

    #[test]
    fn four_on_the_floor_and_offbeat() {
        // musician 1-based 1,5,9,13 → 0-based 0,4,8,12; dur 1 beat
        let four = BeatSpec::FourToFloor;
        assert_eq!(four.steps(), vec![0, 4, 8, 12]);
        assert!((four.hit_duration_steps() - 4.0).abs() < f64::EPSILON);
        // musician 1-based 3,7,11,15 → 0-based 2,6,10,14
        let off = BeatSpec::Offbeat;
        assert_eq!(off.steps(), vec![2, 6, 10, 14]);
        assert!((off.hit_duration_steps() - 4.0).abs() < f64::EPSILON);
    }
}

/// Param snapshot on a device.
/// Preferred: `kick&v9kick: decay(50) pitch(40)` (display units from `devices/*.yaml`).
/// Legacy: `t(kick).d(kick.v9): decay(50) pitch(40)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamCmd {
    pub track: String,
    pub device: DeviceSpec,
    /// User-facing values (display range from catalog; execute maps → wire 0..1).
    pub params: Vec<(String, f64)>,
}

/// `mute(kick)` | `mute(kick) 4` | `mute(kick) @bar` | `mute(kick) 4 @bar`
#[derive(Debug, Clone, PartialEq)]
pub struct MuteCmd {
    pub refs: Vec<TrackRef>,
    /// After primary action, invert mute state after N bars (auto unmute/mute).
    pub bars: Option<u32>,
    /// When to apply the primary mute/unmute.
    pub quantize: MuteQuantize,
}

/// Quantize primary mute action to transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MuteQuantize {
    /// Apply immediately.
    #[default]
    Now,
    /// Apply at next bar boundary (while transport playing).
    Bar,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackRef {
    Name(String),
    Index(i32),
}

/// Per-note expression / performance modifiers.
/// Values are user-facing display units; execute normalizes before sending to Bitwig.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NoteMods {
    pub vel: Vec<Option<f64>>,      // 0..127
    pub pressure: Vec<Option<f64>>, // 0..100 (%)
    pub timbre: Vec<Option<f64>>,   // -100..100 (%)
    pub pan: Vec<Option<f64>>,      // -100..100 (%)
    pub gain: Vec<Option<f64>>,     // 0..100 (%)
    pub chance: Vec<Option<f64>>,   // 0..100 (%)
}

/// A music command: target, action, pattern, params.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicCmd {
    pub target: Target,
    pub action: MusicAction,
    pub pattern: String,                  // mini-notation string (without quotes)
    pub params: Vec<ParamSet>,
    pub transpose: Option<i32>,           // ^N
    pub scale_transpose: Option<i32>,     // ^^N
    pub note_mods: NoteMods,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MusicAction {
    Notes,      // n — exact pitches (any track, including drum modules if user wants)
    Chord,      // chord
    /// `arp` / `arp:up` / `arp:down` / `arp:updown` / `arp:rand` — expand_arp
    Arp(ArpStyle),
}

/// Arpeggio direction for [`MusicAction::Arp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpStyle {
    Up,
    Down,
    UpDown,
    Random,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub track: String,
    pub clip: Option<ClipRef>,
    pub drum_kit: Option<String>,  // "808", "909", "retro"
}

/// Clip / scene-row address on a track (Bitwig: column=track, row=scene slot).
#[derive(Debug, Clone, PartialEq)]
pub enum ClipRef {
    /// Scene **row index** — `lead@0`, `lead@2` (primary id).
    Slot(i32),
    /// Scene **row name** — `lead@verse` (secondary; resolve via scene.list).
    Name(String),
}

/// `s(verse).t(lead).c(new)` | `s(1).t(bass).c(new, intro)` | `.c(start)`
#[derive(Debug, Clone, PartialEq)]
pub struct SceneTrackClipCmd {
    pub scene: SceneRef,
    pub track: String,
    pub action: SceneClipAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SceneClipAction {
    /// Create empty launcher clip at track × scene (optional clip display name).
    New { name: Option<String> },
    Start,
    Stop,
}

/// A parameter assignment: `+cutoff:0.3` — one snapshot via `param.set`.
/// No sequences, ramps, or clip automation (Controller API limit + product choice).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSet {
    pub name: String,
    pub value: f64,
}

/// Chain command: `!bass Polymer Filter Delay-2`
#[derive(Debug, Clone, PartialEq)]
pub struct ChainCmd {
    pub name: String,
    pub kind: String,          // "instrument" default
    pub drum_kit: Option<String>,
    pub devices: Vec<String>,
}

/// Transport commands.
#[derive(Debug, Clone, PartialEq)]
pub enum TransportCmd {
    Play,
    Stop,
}

/// `s(1).start` | `s(verse).start` | `scene(0).stop` — scene by index (primary) or name.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCmd {
    pub scene: SceneRef,
    pub action: LaunchAction,
}

/// Scene address: slot/row index is primary; name resolves via SceneBank.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneRef {
    Index(i32),
    Name(String),
}

/// `c(bass.0).start` | `c(bass.0, kick.1).start` — launch/stop clip(s).
#[derive(Debug, Clone, PartialEq)]
pub struct ClipCtrlCmd {
    pub refs: Vec<ClipCtrlRef>,
    pub action: LaunchAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipCtrlRef {
    pub track: String,
    pub slot: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LaunchAction {
    Start,
    Stop,
}

// ── Mini-notation AST ──────────────────────────────────────────────

/// A parsed mini-notation pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub sequences: Vec<Sequence>,  // comma-separated (superposition)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub atom: Atom,
    pub suffixes: Vec<Suffix>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Note(String),           // "c4", "eb", "f#"
    Midi(i32),              // 60, 36, 127
    Rest,
    Group(Vec<Sequence>),   // [ ... ]
    Alternate(Vec<Vec<Sequence>>), // < ... >
    Polymetric(Vec<Vec<Sequence>>), // { ... }
    RandomChoice(Vec<Atom>),   // | inside group
    Euclid {
        beats: u32,
        steps: u32,
        offset: Option<u32>,
    },
    Subdivide(Vec<Sequence>, u32), // { ... }%N
}

#[derive(Debug, Clone, PartialEq)]
pub enum Suffix {
    Repeat(u32),       // *N
    Slow(u32),         // /N
    Replicate(u32),    // !N
    Elongate,          // _
    ElongateN(u32),    // @N
    RandomDrop(Option<f64>), // ? or ?0.3
    Octave(i32),       // :N
    Euclid { beats: u32, steps: u32, offset: Option<u32> }, // (beats,steps) or (beats,steps,offset)
}
