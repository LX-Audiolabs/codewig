//! Music module — WIGSCRIPT parser, scale system, and pattern expansion.
//!
//! Parses the music-mode REPL syntax and expands patterns into
//! [`NoteSpec`](crate::NoteSpec) lists for `clip.set-notes`.

pub mod ast;
pub mod device;
pub mod expand;
pub mod parse;
pub mod scale;

pub use ast::{
    BeatSpec, Chord, ClipAction, ClipCtrlCmd, ClipCtrlRef, ClipRef, DeviceSpec, DrumAlias,
    FluentCmd, FluentStep, LaunchAction, MusicAction, MusicCmd, MusicLine,
    MuteCmd, ParamCmd, ParamSet, SceneCmd, Target, TrackRef,
};
pub use device::{
    catalog_to_bitwig, device_params, drum_device, drum_midi,
    kit_devices, Device, DeviceKind, DrumKit, FX, NOTE_FX, SYNTHS,
};
pub use expand::{expand_chord, expand_arp, expand_music_line, ArpStyle, ExpandError};
pub use parse::{parse_mini_pattern, parse_music_line, ParseError};
pub use scale::{note_to_midi, Scale, ScaleKind, SCALES};
