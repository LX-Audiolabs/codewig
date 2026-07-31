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
    /// `s(1).start` | `s(1).stop` — scene launch/stop
    Scene(SceneCmd),
    /// `c(track.0).start` | `c(track.0).stop` — clip launch/stop
    ClipCtrl(ClipCtrlCmd),
    /// `> bass` — CLIwig passthrough
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
    /// .device(kick.v9) or .d(hat.v8)
    Device(DeviceSpec),
    /// .add(Delay-2)
    Add(DeviceSpec),
    /// .beat(4_) or .beat:16(1,5,11,14)
    Beat(BeatSpec),
    /// .n("c e g")  — mini-notation for synth tracks
    Pattern(String),
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
    /// 4_ → 4-on-the-floor (steps 0,4,8,12 in 16-grid)
    FourToFloor,
    /// 2_4 → half notes (steps 0,8)
    HalfNotes,
    /// off → offbeats (steps 2,6,10,14)
    Offbeat,
    /// bk2 → breakbeat kick (steps 0,8)
    Break2,
    /// :16(1,5,11,14)  — explicit grid size + step positions
    Explicit { grid: u32, positions: Vec<u32> },
}

impl BeatSpec {
    /// Expand to step positions in a 16-step grid (default).
    pub fn steps(&self) -> Vec<u32> {
        match self {
            BeatSpec::FourToFloor => vec![0, 4, 8, 12],
            BeatSpec::HalfNotes => vec![0, 8],
            BeatSpec::Offbeat => vec![2, 6, 10, 14],
            BeatSpec::Break2 => vec![0, 8],
            BeatSpec::Explicit { positions, .. } => positions.clone(),
        }
    }

    /// Grid size (default 16).
    pub fn grid(&self) -> u32 {
        match self {
            BeatSpec::Explicit { grid, .. } => *grid,
            _ => 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClipAction {
    Start,
    Stop,
}

/// `t(kick).d(kick.v9): decay(280) punch(45)`
#[derive(Debug, Clone, PartialEq)]
pub struct ParamCmd {
    pub track: String,
    pub device: DeviceSpec,
    pub params: Vec<(String, f64)>,  // (name, raw_value)
}

/// `mute(kick)` | `mute(kick, bass)` | `mute(1,3,5)`
#[derive(Debug, Clone, PartialEq)]
pub struct MuteCmd {
    pub refs: Vec<TrackRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackRef {
    Name(String),
    Index(i32),
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum MusicAction {
    Notes,      // n
    Drums,      // d
    Chord,      // chord
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub track: String,
    pub clip: Option<ClipRef>,
    pub drum_kit: Option<String>,  // "808", "909", "retro"
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClipRef {
    Name(String),      // @verse
    Launch,            // ! (just launch, no pattern)
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

/// `s(1).start` | `s(1).stop` — launch/stop all clips in scene row N.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCmd {
    pub scene: i32,
    pub action: LaunchAction,
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
    Drum(DrumAlias),        // "bd", "hh", "909bd"
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrumAlias {
    Bd,       // bd, kick
    Sd,       // sd, snare
    Hh,       // hh
    Cp,       // cp, clap
    Cymb,     // cy, cymb
    Tom,      // tom
    Ride,     // ride
    Rim,      // rim
    V1Kick,
    V1Hat,
    V1Sn,
    V1Perc,
    V8Kick,
    V8Hat,
    V8Sn,
    V8Clap,
    V8Perc,
    V9Kick,
    V9Hat,
    V9Sn,
    V9Clap,
    V9Ride,
    V9Rim,
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

// ── Chord types ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Chord {
    Named { root: String, quality: String },
    Roman { degree: String, quality: String },
}
