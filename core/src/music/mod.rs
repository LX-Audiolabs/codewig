//! Music module — WIGSCRIPT parse, expand, execute (shared by UI + CLI).
//!
//! Line language is primary (`mute(kick)`, `new track(bass).device(Polymer)`, …).
//! Mini-notation / chord expand live here so both frontends share one path.

pub mod ast;
pub mod device;
pub mod execute;
pub mod expand;
pub mod param_catalog;
pub mod parse;
pub mod scale;

pub use ast::{
    ArpStyle, BeatSpec, Chord, ClipAction, ClipCtrlCmd, ClipCtrlRef, ClipRef, DeviceSpec,
    FluentCmd, FluentStep, LaunchAction, MusicAction, MusicCmd, MusicLine, MuteCmd,
    MuteQuantize, ParamCmd, ParamSet, SceneClipAction, SceneCmd, SceneRef, SceneTrackClipCmd,
    Target, TrackRef,
};
pub use device::{
    catalog_to_bitwig, catalog_to_drum, device_param_names, device_params, is_insertable,
    kit_devices, Device, DeviceKind, DRUM_DEVICES, INSERTABLE, MONO_DRUM_NOTE,
};
pub use param_catalog::{
    catalog as param_catalog, reload_catalog, DeviceHostKind, DeviceParamFile, ParamCatalog,
    ParamDef,
};
pub use execute::{execute_line, MusicSession};
pub use expand::{expand_arp, expand_chord, expand_music_line, ExpandError};
pub use parse::{parse_mini_pattern, parse_music_line, ParseError};
pub use scale::{note_to_midi, Scale, ScaleKind, SCALES};
