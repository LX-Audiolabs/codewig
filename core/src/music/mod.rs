//! Music module — WIGSCRIPT parse, expand, execute (shared by UI + CLI).
//!
//! Line language is primary (`mute(kick)`, `new track(bass).device(Polymer)`, …).
//! Mini-notation / chord expand live here so both frontends share one path.

pub mod ast;
pub mod device;
pub mod execute;
pub mod expand;
pub mod parse;
pub mod scale;

pub use ast::{
    ArpStyle, BeatSpec, Chord, ClipAction, ClipCtrlCmd, ClipCtrlRef, ClipRef, DeviceSpec,
    DrumAlias, FluentCmd, FluentStep, LaunchAction, MusicAction, MusicCmd, MusicLine,
    MuteCmd, MuteQuantize, ParamCmd, ParamSet, SceneCmd, SceneRef, Target, TrackRef,
};
pub use device::{
    catalog_to_bitwig, catalog_to_drum, device_params, drum_device, drum_midi,
    is_insertable, kit_devices, Device, DeviceKind, DRUM_DEVICES, INSERTABLE,
};
pub use execute::{execute_line, MusicSession};
pub use expand::{expand_arp, expand_chord, expand_music_line, ExpandError};
pub use parse::{parse_mini_pattern, parse_music_line, ParseError};
pub use scale::{note_to_midi, Scale, ScaleKind, SCALES};
