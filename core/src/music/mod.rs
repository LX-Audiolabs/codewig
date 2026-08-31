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
    ArpStyle, BeatSpec, ClipAction, ClipCtrlCmd, ClipCtrlRef, ClipRef, DeviceSpec, FluentCmd,
    FluentStep, LaunchAction, MusicAction, MusicCmd, MusicLine, MuteCmd, MuteQuantize, ParamCmd,
    ParamSet, SceneClipAction, SceneCmd, SceneRef, SceneTrackClipCmd, Target, TrackRef,
};
pub use device::{
    DRUM_DEVICES, Device, DeviceKind, INSERTABLE, MONO_DRUM_NOTE, catalog_to_bitwig,
    catalog_to_drum, device_param_names, device_params, is_insertable,
};
pub use execute::{ExecuteError, MusicSession, execute_line, resolve_track_at, run_chain};
pub use expand::{ExpandError, expand_arp, expand_beat, expand_chord, expand_music_line};
pub use param_catalog::{
    DeviceHostKind, DeviceParamFile, ParamCatalog, ParamDef, catalog as param_catalog,
    reload_catalog,
};
pub use parse::{ParseError, parse_mini_pattern, parse_music_line};
pub use scale::{SCALES, Scale, ScaleError, ScaleKind, key_to_midi, note_to_midi};
