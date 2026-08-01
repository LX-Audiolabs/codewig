//! Execute a parsed WIGSCRIPT [`MusicLine`] against the Bitwig bridge [`Client`].
//!
//! This is the **shared** path for codewig-live UI and `cliwig eval` / batch.
//! Wire protocol stays in [`Client`]; only line → Client calls live here.

use super::ast::*;
use super::device::{catalog_to_bitwig, catalog_to_drum};
use super::expand::{expand_chord, expand_music_line};
use super::scale::Scale;
use crate::{Client, Error, NoteSpec};
use serde_json::{json, Value};
use std::thread;
use std::time::Duration;

/// Session state across WIGSCRIPT lines (key/scale, last drum MIDI for `.beat`).
#[derive(Debug, Clone)]
pub struct MusicSession {
    pub scale: Option<Scale>,
    pub steps_per_bar: u32,
    pub default_beats: i32,
    pub default_slot: i32,
    /// Trigger MIDI for monophonic drum module (`.beat` / `d` hits). Always one key — not GM pads.
    pub last_drum_midi: Option<i32>,
}

impl Default for MusicSession {
    fn default() -> Self {
        Self {
            scale: None,
            steps_per_bar: 16,
            default_beats: 4,
            default_slot: 0,
            last_drum_midi: None,
        }
    }
}

/// Run an already-parsed pure WIGSCRIPT AST node.
///
/// Full user-input entry is **not** here: UI `commands::run` and CLI `run_one_line` parse,
/// peel [`MusicLine::PassThrough`] / legacy, then call this. [`MusicLine::PassThrough`] is
/// intentionally an error so core stays free of clap/shlex legacy trees.
pub fn execute_line(
    client: &Client,
    session: &mut MusicSession,
    line: MusicLine,
) -> Result<Option<Value>, String> {
    match line {
        MusicLine::Empty => Ok(None),
        MusicLine::Transport(TransportCmd::Play) => map(client.play()),
        MusicLine::Transport(TransportCmd::Stop) => map(client.stop()),
        MusicLine::Tempo(bpm) => map(client.set_tempo(bpm)),
        MusicLine::Key { root, scale } => {
            session.scale = Some(Scale::new(&root, &scale).map_err(|e| e)?);
            Ok(Some(json!({
                "key": root,
                "scale": scale,
                "root_midi": session.scale.as_ref().map(|s| s.root),
            })))
        }
        MusicLine::ModeSwitch(mode) => Ok(Some(json!({ "mode": mode, "note": "UI/CLI mode is cosmetic here" }))),
        MusicLine::PassThrough(cmd) => Err(format!(
            "passthrough `> {cmd}` not handled in execute_line — use UI/CLI entry (commands::run / cliwig eval), or drop `>`"
        )),
        MusicLine::Mute(cmd) => mute(client, &cmd, true),
        MusicLine::Unmute(cmd) => mute(client, &cmd, false),
        MusicLine::Scene(cmd) => scene(client, &cmd),
        MusicLine::NewScene { name } => scene_new(client, name.as_deref()),
        MusicLine::SceneTrackClip(cmd) => scene_track_clip(client, session, &cmd),
        MusicLine::ClipCtrl(cmd) => clip_ctrl(client, &cmd),
        MusicLine::Param(cmd) => param(client, &cmd),
        MusicLine::Chain(cmd) => chain(client, session, &cmd),
        MusicLine::Fluent(cmd) => fluent(client, session, &cmd),
        MusicLine::Music(cmd) => music(client, session, &cmd),
    }
}

fn map(r: Result<Option<Value>, Error>) -> Result<Option<Value>, String> {
    r.map_err(|e| e.to_string())
}

/// Poll until `name` appears in track.list (Bitwig creates/renames async).
/// Fast path: returns as soon as ready; ceiling ~400ms.
fn wait_track_named(client: &Client, name: &str) -> Result<(), String> {
    for _ in 0..20 {
        if let Ok(Some(list)) = client.track_list() {
            if let Some(arr) = list.get("tracks").and_then(Value::as_array) {
                if arr.iter().any(|t| t.get("name").and_then(Value::as_str) == Some(name)) {
                    return Ok(());
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "track '{name}' not visible after create — try again or rename in Bitwig"
    ))
}

/// Poll until launcher slot has content (after clip_new).
fn wait_clip_content(client: &Client, track: &str, slot: i32) -> Result<(), String> {
    for _ in 0..25 {
        if let Ok(Some(list)) = client.clip_list(track) {
            if let Some(arr) = list.get("clips").and_then(Value::as_array) {
                if arr.iter().any(|c| {
                    c.get("slot").and_then(Value::as_i64) == Some(slot as i64)
                        && c.get("hasContent").and_then(Value::as_bool).unwrap_or(false)
                }) {
                    return Ok(());
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "clip on '{track}' slot {slot} still empty after create — retry or check Bitwig launcher"
    ))
}

/// Resolve scene row: index primary, name via scene.list.
fn resolve_scene_index(client: &Client, scene: &SceneRef) -> Result<i32, String> {
    match scene {
        SceneRef::Index(i) => {
            if *i < 0 {
                return Err(format!("scene index must be >= 0, got {i}"));
            }
            Ok(*i)
        }
        SceneRef::Name(name) => {
            let list = client
                .scene_list()
                .map_err(|e| e.to_string())?
                .ok_or("scene.list empty")?;
            let arr = list
                .get("scenes")
                .and_then(Value::as_array)
                .ok_or("scene.list missing scenes")?;
            for s in arr {
                let n = s.get("name").and_then(Value::as_str).unwrap_or("");
                if n.eq_ignore_ascii_case(name) {
                    let idx = s
                        .get("index")
                        .and_then(Value::as_i64)
                        .ok_or("scene missing index")? as i32;
                    return Ok(idx);
                }
            }
            Err(format!(
                "unknown scene '{name}' — create first: new scene({name})"
            ))
        }
    }
}

fn scene_new(client: &Client, name: Option<&str>) -> Result<Option<Value>, String> {
    map(client.scene_new(name))
}

fn scene_track_clip(
    client: &Client,
    session: &mut MusicSession,
    cmd: &SceneTrackClipCmd,
) -> Result<Option<Value>, String> {
    let slot = resolve_scene_index(client, &cmd.scene)?;
    match &cmd.action {
        SceneClipAction::New { name } => {
            // Bitwig cell: create empty clip at track × scene row
            client
                .clip_new(
                    &cmd.track,
                    Some(slot),
                    session.default_beats,
                    name.as_deref(),
                )
                .map_err(|e| e.to_string())?;
            // Content may lag; note writes also auto-create in extension now
            let _ = wait_clip_content(client, &cmd.track, slot);
            Ok(Some(json!({
                "track": cmd.track,
                "scene": scene_ref_str(&cmd.scene),
                "slot": slot,
                "clip": "new",
                "name": name,
            })))
        }
        SceneClipAction::Start => map(client.clip_launch(&cmd.track, slot)),
        SceneClipAction::Stop => map(client.clip_stop(&cmd.track)),
    }
}

/// Short settle only when next call needs Bitwig cursor focus (device chain).
fn wait_cursor() {
    // ponytail: fixed 15ms vs 50ms blanket; enough for select→device_add on localhost
    thread::sleep(Duration::from_millis(15));
}

fn track_ref(r: &TrackRef) -> String {
    match r {
        TrackRef::Name(n) => n.clone(),
        TrackRef::Index(i) => i.to_string(),
    }
}

fn mute(client: &Client, cmd: &MuteCmd, on: bool) -> Result<Option<Value>, String> {
    use super::ast::MuteQuantize;
    let refs: Vec<String> = cmd.refs.iter().map(track_ref).collect();
    let q = match cmd.quantize {
        MuteQuantize::Now => None,
        MuteQuantize::Bar => Some("bar"),
    };
    map(client.track_mute_timed(&refs, on, cmd.bars, q))
}

fn scene_ref_str(r: &SceneRef) -> String {
    match r {
        SceneRef::Index(i) => i.to_string(),
        SceneRef::Name(n) => n.clone(),
    }
}

fn scene(client: &Client, cmd: &SceneCmd) -> Result<Option<Value>, String> {
    // Index primary, name secondary — one wire RPC via SceneBank (not N× clip.launch).
    let r = scene_ref_str(&cmd.scene);
    match cmd.action {
        LaunchAction::Start => map(client.scene_launch(&r)),
        LaunchAction::Stop => map(client.scene_stop(&r)),
    }
}

fn clip_ctrl(client: &Client, cmd: &ClipCtrlCmd) -> Result<Option<Value>, String> {
    let mut results = Vec::new();
    for r in &cmd.refs {
        let track = if r.track.is_empty() {
            return Err("clip ref missing track name".into());
        } else {
            r.track.clone()
        };
        match cmd.action {
            LaunchAction::Start => {
                results.push(json!({
                    "track": track,
                    "slot": r.slot,
                    "result": client.clip_launch(&track, r.slot).map_err(|e| e.to_string())?,
                }));
            }
            LaunchAction::Stop => {
                results.push(json!({
                    "track": track,
                    "result": client.clip_stop(&track).map_err(|e| e.to_string())?,
                }));
            }
        }
    }
    Ok(Some(json!({ "clips": results })))
}

fn resolve_track_at(client: &Client, at: i32) -> Result<i32, String> {
    if at >= 0 {
        return Ok(at);
    }
    let list = client
        .track_list()
        .map_err(|e| e.to_string())?
        .ok_or("track.list empty")?;
    let tracks = list
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or("track.list missing tracks")?;
    let count = tracks
        .iter()
        .filter(|t| {
            t.get("type")
                .and_then(Value::as_str)
                .map(|ty| {
                    let ty = ty.to_lowercase();
                    ty == "instrument" || ty == "audio"
                })
                .unwrap_or(false)
        })
        .count();
    Ok(count as i32)
}

fn ensure_clip(client: &Client, track: &str, slot: i32, beats: i32) -> Result<i32, String> {
    ensure_clip_at(client, track, slot, beats, None)
}

fn ensure_clip_at(
    client: &Client,
    track: &str,
    slot: i32,
    beats: i32,
    name: Option<&str>,
) -> Result<i32, String> {
    // If slot already has content, reuse.
    match client.clip_list(track) {
        Ok(Some(list)) => {
            if let Some(arr) = list.get("clips").and_then(Value::as_array) {
                if let Some(c) = arr.iter().find(|c| {
                    c.get("slot").and_then(Value::as_i64) == Some(slot as i64)
                }) {
                    let has = c
                        .get("hasContent")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if has {
                        return Ok(slot);
                    }
                }
            }
        }
        Ok(None) | Err(_) => {}
    }
    client
        .clip_new(track, Some(slot), beats, name)
        .map_err(|e| e.to_string())?;
    // Poll; extension replace-notes also auto-creates if still racing
    match wait_clip_content(client, track, slot) {
        Ok(()) => Ok(slot),
        Err(_) => {
            // one retry create
            let _ = client.clip_new(track, Some(slot), beats, name);
            wait_clip_content(client, track, slot).map(|_| slot).or(Ok(slot))
        }
    }
}

/// Bitwig addressing: default slot | `@0` (row id) | `@verse` (scene name → row).
/// Does **not** pick "first empty" silently — that is not how Bitwig thinks.
fn resolve_write_slot(
    client: &Client,
    track: &str,
    clip: &Option<ClipRef>,
    default_slot: i32,
    beats: i32,
) -> Result<i32, String> {
    match clip {
        None | Some(ClipRef::Launch) => ensure_clip(client, track, default_slot, beats),
        Some(ClipRef::Slot(i)) => ensure_clip(client, track, *i, beats),
        Some(ClipRef::Name(name)) => {
            let name = name.trim();
            if name.is_empty() {
                return ensure_clip(client, track, default_slot, beats);
            }
            // Scene name → row index (must exist: new scene(name) first)
            let slot = resolve_scene_index(client, &SceneRef::Name(name.to_string()))?;
            // Create clip at that cell if missing (named after scene for launcher label)
            ensure_clip_at(client, track, slot, beats, Some(name))
        }
    }
}

/// Apply user-facing note modifiers to expanded notes.
///
/// Display units:
/// - `vel`: 0..127
/// - `pressure` / `gain` / `chance`: 0..100 (%)
/// - `timbre` / `pan`: -100..100 (%)
///
/// Stored as wire-normalized values on `NoteSpec`.
fn apply_note_mods(notes: &mut [NoteSpec], mods: &super::ast::NoteMods) {
    for (i, n) in notes.iter_mut().enumerate() {
        if let Some(Some(v)) = mods.vel.get(i) {
            n.vel = (*v as i32).clamp(0, 127);
        }
        if let Some(Some(v)) = mods.pressure.get(i) {
            n.pressure = Some(v.clamp(0.0, 100.0) / 100.0);
        }
        if let Some(Some(v)) = mods.timbre.get(i) {
            n.timbre = Some(v.clamp(-100.0, 100.0) / 100.0);
        }
        if let Some(Some(v)) = mods.pan.get(i) {
            n.pan = Some(v.clamp(-100.0, 100.0) / 100.0);
        }
        if let Some(Some(v)) = mods.gain.get(i) {
            n.gain = Some(v.clamp(0.0, 100.0) / 100.0);
        }
        if let Some(Some(v)) = mods.chance.get(i) {
            n.chance = Some(v.clamp(0.0, 100.0) / 100.0);
        }
    }
}

fn write_notes(
    client: &Client,
    track: &str,
    slot: i32,
    beats: i32,
    notes: &[NoteSpec],
) -> Result<Option<Value>, String> {
    let slot = ensure_clip(client, track, slot, beats)?;
    let playable: Vec<NoteSpec> = notes.iter().cloned().filter(|n| n.vel > 0).collect();
    // One RPC: clear all + write (empty playable = clear only / rests)
    map(client.clip_replace_notes(track, slot, &playable))
}

fn music(client: &Client, session: &mut MusicSession, cmd: &MusicCmd) -> Result<Option<Value>, String> {
    let track = if cmd.target.track.is_empty() {
        return Err("music line needs a track name, e.g. bass: n \"c e g\"".into());
    } else {
        cmd.target.track.clone()
    };

    let beats = session.default_beats;
    // Slot id (session default / launcher row) is primary; `@verse` is secondary name lookup.
    let slot = resolve_write_slot(
        client,
        &track,
        &cmd.target.clip,
        session.default_slot,
        beats,
    )?;

    let mut notes = match cmd.action {
        MusicAction::Chord => expand_chord(
            &cmd.pattern,
            session.scale.as_ref(),
            session.steps_per_bar,
            0,
        )
        .map_err(|e| e.to_string())?,
        MusicAction::Notes | MusicAction::Arp(_) => {
            let (notes, _) =
                expand_music_line(cmd, session.scale.as_ref(), session.steps_per_bar)
                    .map_err(|e| e.to_string())?;
            notes
        }
    };
    apply_note_mods(&mut notes, &cmd.note_mods);

    // +params need cursor track; pure note write uses track ref in clip.* — skip select
    if !cmd.params.is_empty() {
        client
            .track_select(&track)
            .map_err(|e| e.to_string())?;
        wait_cursor();
        let sets: Vec<(String, f64)> = cmd
            .params
            .iter()
            .map(|p| (p.name.clone(), p.value))
            .collect();
        let _ = client.param_set_multi(&sets);
    }

    // Notes path already resolved slot (incl. create); don't re-ensure without name.
    let playable: Vec<NoteSpec> = notes.iter().cloned().filter(|n| n.vel > 0).collect();
    map(client.clip_replace_notes(&track, slot, &playable))
}

fn add_device(client: &Client, catalog: &str) -> Result<Option<Value>, String> {
    if let Some(name) = catalog_to_bitwig(catalog) {
        return map(client.device_add(&name));
    }
    Err(format!(
        "unknown / non-curated device '{catalog}'. \
         Insertable: Polymer Polysynth Organ layer; v0–v9 drums (prefer `v9 kick` / `v9kick`); \
         Filter Reverb Delay+ Chorus+ Saturator. Not allowed: Sampler, Drum Machine."
    ))
}

fn chain(
    client: &Client,
    session: &mut MusicSession,
    cmd: &ChainCmd,
) -> Result<Option<Value>, String> {
    let at = resolve_track_at(client, -1)?;
    let created = client
        .track_new(&cmd.kind, at, Some(&cmd.name))
        .map_err(|e| e.to_string())?;
    wait_track_named(client, &cmd.name)?;
    client
        .track_select(&cmd.name)
        .map_err(|e| e.to_string())?;
    wait_cursor();

    let mut added = Vec::new();
    // Drum kit shell: insert Instrument Layer when kit requested and no devices
    if cmd.drum_kit.is_some() && cmd.devices.is_empty() {
        added.push(add_device(client, "layer")?);
        wait_cursor();
    }
    for d in &cmd.devices {
        added.push(add_device(client, d)?);
        wait_cursor();
    }

    let _ = ensure_clip(client, &cmd.name, session.default_slot, session.default_beats);

    Ok(Some(json!({
        "chain": { "name": cmd.name, "kind": cmd.kind, "drum_kit": cmd.drum_kit },
        "created": created,
        "devices": added,
    })))
}

fn fluent(
    client: &Client,
    session: &mut MusicSession,
    cmd: &FluentCmd,
) -> Result<Option<Value>, String> {
    let needs_cursor = cmd.steps.iter().any(|s| {
        matches!(
            s,
            FluentStep::Device(_) | FluentStep::Add(_)
        )
    });

    if cmd.create {
        let at = resolve_track_at(client, -1)?;
        client
            .track_new("instrument", at, Some(&cmd.track))
            .map_err(|e| e.to_string())?;
        wait_track_named(client, &cmd.track)?;
    }
    // device_add / param need cursor; pure mute/clip/notes use track refs
    if needs_cursor || cmd.create {
        client
            .track_select(&cmd.track)
            .map_err(|e| e.to_string())?;
        if needs_cursor {
            wait_cursor();
        }
    }

    let mut log = Vec::new();
    // Fluent setup writes to **slot 0** (first clip). Multi-clip → colon `track@scene: n "…"`.
    let slot0 = session.default_slot;

    for step in &cmd.steps {
        match step {
            FluentStep::Device(d) | FluentStep::Add(d) => {
                // Drum module → monophonic trigger for following .beat
                if let Some((_, midi)) = catalog_to_drum(&d.catalog_name) {
                    session.last_drum_midi = Some(midi);
                }
                log.push(json!({ "device": d.catalog_name, "result": add_device(client, &d.catalog_name)? }));
                wait_cursor();
            }
            FluentStep::Beat(b) => {
                // Write immediately so later .clip(start) sees content
                let midi = session
                    .last_drum_midi
                    .unwrap_or(super::device::MONO_DRUM_NOTE);
                let dur = b.hit_duration_steps();
                let notes: Vec<NoteSpec> = b
                    .steps()
                    .into_iter()
                    .map(|s| NoteSpec {
                        step: s as i32,
                        key: midi,
                        vel: 100,
                        dur,
                        ..NoteSpec::default()
                    })
                    .collect();
                log.push(json!({
                    "beat": write_notes(client, &cmd.track, slot0, session.default_beats, &notes)?
                }));
            }
            FluentStep::Pattern { pattern, mods } => {
                let music = MusicCmd {
                    target: Target {
                        track: cmd.track.clone(),
                        clip: None, // slot 0 via write_notes/ensure
                        drum_kit: None,
                    },
                    action: MusicAction::Notes,
                    pattern: pattern.clone(),
                    params: vec![],
                    transpose: None,
                    scale_transpose: None,
                    note_mods: NoteMods::default(),
                };
                let (mut notes, _) =
                    expand_music_line(&music, session.scale.as_ref(), session.steps_per_bar)
                        .map_err(|e| e.to_string())?;
                apply_note_mods(&mut notes, mods);
                log.push(json!({
                    "notes": write_notes(client, &cmd.track, slot0, session.default_beats, &notes)?
                }));
            }
            FluentStep::Mute => {
                log.push(json!({
                    "mute": client.track_mute(&[cmd.track.clone()], true).map_err(|e| e.to_string())?
                }));
            }
            FluentStep::ClipAction(a) => {
                ensure_clip(client, &cmd.track, slot0, session.default_beats)?;
                match a {
                    ClipAction::Start => {
                        log.push(json!({
                            "clip": client.clip_launch(&cmd.track, slot0).map_err(|e| e.to_string())?
                        }));
                    }
                    ClipAction::Stop => {
                        log.push(json!({
                            "clip": client.clip_stop(&cmd.track).map_err(|e| e.to_string())?
                        }));
                    }
                }
            }
            FluentStep::ClipCtrl(cc) => {
                for r in &cc.refs {
                    let track = if r.track.is_empty() {
                        cmd.track.clone()
                    } else {
                        r.track.clone()
                    };
                    match cc.action {
                        LaunchAction::Start => {
                            log.push(json!({
                                "clip": client.clip_launch(&track, r.slot).map_err(|e| e.to_string())?
                            }));
                        }
                        LaunchAction::Stop => {
                            log.push(json!({
                                "clip": client.clip_stop(&track).map_err(|e| e.to_string())?
                            }));
                        }
                    }
                }
            }
        }
    }

    Ok(Some(json!({ "fluent": cmd.track, "create": cmd.create, "slot": slot0, "steps": log })))
}

fn param(client: &Client, cmd: &ParamCmd) -> Result<Option<Value>, String> {
    // Catalog maps display units → wire 0..1; unknown / empty device → hard error.
    let sets = super::param_catalog::catalog()
        .map_param_sets(&cmd.device.catalog_name, &cmd.params)?;

    client
        .track_select(&cmd.track)
        .map_err(|e| e.to_string())?;
    wait_cursor();

    // Focus device by name if possible
    if let Ok(Some(list)) = client.device_list() {
        if let Some(arr) = list.get("devices").and_then(Value::as_array) {
            let want = catalog_to_bitwig(&cmd.device.catalog_name)
                .or_else(|| {
                    super::param_catalog::catalog()
                        .resolve(&cmd.device.catalog_name)
                        .map(|d| d.bitwig_name.clone())
                })
                .unwrap_or_else(|| cmd.device.catalog_name.clone());
            let want_l = want.to_lowercase();
            if let Some(dev) = arr.iter().find(|d| {
                d.get("name")
                    .and_then(Value::as_str)
                    .map(|n| n.to_lowercase() == want_l || n.to_lowercase().contains(&want_l))
                    .unwrap_or(false)
            }) {
                if let Some(idx) = dev.get("index").and_then(Value::as_i64) {
                    let _ = client.device_select(idx as i32);
                    wait_cursor();
                }
            }
        }
    }

    map(client.param_set_multi(&sets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse::parse_music_line;

    #[test]
    fn parse_then_key_updates_session_shape() {
        // No live Bitwig — only check line kind routing for Key
        let line = parse_music_line("k C minor").unwrap();
        assert!(matches!(line, MusicLine::Key { .. }));
    }

    #[test]
    fn music_requires_track() {
        let line = parse_music_line(r#"n "c e g""#).unwrap();
        // Would fail at execute without client; structure is Music with empty track
        if let MusicLine::Music(cmd) = line {
            assert!(cmd.target.track.is_empty());
        }
    }

    #[test]
    fn scene_ref_str_index_and_name() {
        assert_eq!(scene_ref_str(&SceneRef::Index(2)), "2");
        assert_eq!(scene_ref_str(&SceneRef::Name("verse".into())), "verse");
    }
}
