//! Execute a parsed WIGSCRIPT [`MusicLine`] against the Bitwig bridge [`Client`].
//!
//! This is the **shared** path for codewig-live UI and `codewig-cli eval` / batch.
//! Wire protocol stays in [`Client`]; only line → Client calls live here.

use super::ast::*;
use super::device::{catalog_to_bitwig, catalog_to_drum, is_banned};
use super::expand::{expand_chord, expand_music_line, ExpandError};
use super::scale::{Scale, ScaleError};
use crate::{Client, Error, NoteSpec};
use serde_json::{json, Value};
use std::thread;
use std::time::Duration;

/// Structured error for [`execute_line`] and its helpers.
///
/// [`Error::Extension`] keeps the extension error code; frontends can fall back
/// to `format!("{e}")` for display without losing it.
#[derive(thiserror::Error, Debug)]
pub enum ExecuteError {
    #[error("{0}")]
    Scale(#[from] ScaleError),
    #[error("{0}")]
    Expand(#[from] ExpandError),
    /// Transport / protocol / extension error from the bridge client.
    #[error(transparent)]
    Client(#[from] Error),
    /// YAML param catalog mapping failed (e.g. display value outside wire 0..1).
    #[error("{0}")]
    Catalog(String),
    /// User-input / semantic error (missing track, unknown scene, passthrough in core, …).
    #[error("{0}")]
    Usage(String),
}

fn usage(msg: impl Into<String>) -> ExecuteError {
    ExecuteError::Usage(msg.into())
}

/// Session state across WIGSCRIPT lines (key/scale, last drum MIDI for `.beat`).
#[derive(Debug, Clone)]
pub struct MusicSession {
    pub scale: Option<Scale>,
    pub steps_per_bar: u32,
    pub default_beats: i32,
    pub default_slot: i32,
    /// Trigger MIDI for monophonic drum module (`.beat`). Always one key — not GM pads.
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
) -> Result<Option<Value>, ExecuteError> {
    match line {
        MusicLine::Empty => Ok(None),
        MusicLine::Transport(TransportCmd::Play) => Ok(client.play()?),
        MusicLine::Transport(TransportCmd::Stop) => Ok(client.stop()?),
        MusicLine::Tempo(bpm) => Ok(client.set_tempo(bpm)?),
        MusicLine::Key { root, scale } => {
            session.scale = Some(Scale::new(&root, &scale)?);
            Ok(Some(json!({
                "key": root,
                "scale": scale,
                "root_midi": session.scale.as_ref().map(|s| s.root),
            })))
        }
        MusicLine::ModeSwitch(mode) => Ok(Some(json!({ "mode": mode, "note": "UI/CLI mode is cosmetic here" }))),
        MusicLine::PassThrough(cmd) => Err(usage(format!(
            "passthrough `> {cmd}` not handled in execute_line — use UI/CLI entry (commands::run / codewig-cli eval), or drop `>`"
        ))),
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

/// Poll until `name` appears in track.list (Bitwig creates/renames async).
/// Fast path: returns as soon as ready; ceiling ~400ms.
/// Shared by WIGSCRIPT `chain`/fluent and `codewig-cli chain`.
pub fn wait_track_named(client: &Client, name: &str) -> Result<(), ExecuteError> {
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
    Err(usage(format!(
        "track '{name}' not visible after create — try again or rename in Bitwig"
    )))
}

/// Poll until launcher slot has content (after clip_new).
fn wait_clip_content(client: &Client, track: &str, slot: i32) -> Result<(), ExecuteError> {
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
    Err(usage(format!(
        "clip on '{track}' slot {slot} still empty after create — retry or check Bitwig launcher"
    )))
}

/// Resolve scene row: index primary, name via scene.list.
fn resolve_scene_index(client: &Client, scene: &SceneRef) -> Result<i32, ExecuteError> {
    match scene {
        SceneRef::Index(i) => {
            if *i < 0 {
                return Err(usage(format!("scene index must be >= 0, got {i}")));
            }
            Ok(*i)
        }
        SceneRef::Name(name) => {
            let list = client
                .scene_list()?
                .ok_or_else(|| usage("scene.list empty"))?;
            let arr = list
                .get("scenes")
                .and_then(Value::as_array)
                .ok_or_else(|| usage("scene.list missing scenes"))?;
            for s in arr {
                let n = s.get("name").and_then(Value::as_str).unwrap_or("");
                if n.eq_ignore_ascii_case(name) {
                    let idx = s
                        .get("index")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| usage("scene missing index"))? as i32;
                    return Ok(idx);
                }
            }
            Err(usage(format!(
                "unknown scene '{name}' — create first: new scene({name})"
            )))
        }
    }
}

fn scene_new(client: &Client, name: Option<&str>) -> Result<Option<Value>, ExecuteError> {
    Ok(client.scene_new(name)?)
}

fn scene_track_clip(
    client: &Client,
    session: &mut MusicSession,
    cmd: &SceneTrackClipCmd,
) -> Result<Option<Value>, ExecuteError> {
    let slot = resolve_scene_index(client, &cmd.scene)?;
    match &cmd.action {
        SceneClipAction::New { name } => {
            // Bitwig cell: create empty clip at track × scene row
            client.clip_new(
                &cmd.track,
                Some(slot),
                session.default_beats,
                name.as_deref(),
            )?;
            // best-effort: content may lag; note writes also auto-create in extension now,
            // so a lagging poll must not fail the line
            let _ = wait_clip_content(client, &cmd.track, slot);
            Ok(Some(json!({
                "track": cmd.track,
                "scene": scene_ref_str(&cmd.scene),
                "slot": slot,
                "clip": "new",
                "name": name,
            })))
        }
        SceneClipAction::Start => Ok(client.clip_launch(&cmd.track, slot)?),
        SceneClipAction::Stop => Ok(client.clip_stop(&cmd.track)?),
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

fn mute(client: &Client, cmd: &MuteCmd, on: bool) -> Result<Option<Value>, ExecuteError> {
    use super::ast::MuteQuantize;
    let refs: Vec<String> = cmd.refs.iter().map(track_ref).collect();
    let q = match cmd.quantize {
        MuteQuantize::Now => None,
        MuteQuantize::Bar => Some("bar"),
    };
    Ok(client.track_mute_timed(&refs, on, cmd.bars, q)?)
}

fn scene_ref_str(r: &SceneRef) -> String {
    match r {
        SceneRef::Index(i) => i.to_string(),
        SceneRef::Name(n) => n.clone(),
    }
}

fn scene(client: &Client, cmd: &SceneCmd) -> Result<Option<Value>, ExecuteError> {
    // Index primary, name secondary — one wire RPC via SceneBank (not N× clip.launch).
    let r = scene_ref_str(&cmd.scene);
    match &cmd.action {
        LaunchAction::Start => Ok(client.scene_launch(&r)?),
        LaunchAction::Stop => Ok(client.scene_stop(&r)?),
        LaunchAction::Rename(name) => Ok(client.scene_rename(&r, name)?),
        LaunchAction::Delete => Ok(client.scene_delete(&r)?),
    }
}

fn clip_ctrl(client: &Client, cmd: &ClipCtrlCmd) -> Result<Option<Value>, ExecuteError> {
    let mut results = Vec::new();
    for r in &cmd.refs {
        let track = if r.track.is_empty() {
            return Err(usage("clip ref missing track name"));
        } else {
            r.track.clone()
        };
        match &cmd.action {
            LaunchAction::Start => {
                results.push(json!({
                    "track": track,
                    "slot": r.slot,
                    "result": client.clip_launch(&track, r.slot)?,
                }));
            }
            LaunchAction::Stop => {
                results.push(json!({
                    "track": track,
                    "result": client.clip_stop(&track)?,
                }));
            }
            LaunchAction::Rename(name) => {
                results.push(json!({
                    "track": track,
                    "slot": r.slot,
                    "result": client.clip_rename(&track, r.slot, name)?,
                }));
            }
            LaunchAction::Delete => {
                results.push(json!({
                    "track": track,
                    "slot": r.slot,
                    "result": client.clip_delete(&track, r.slot)?,
                }));
            }
        }
    }
    Ok(Some(json!({ "clips": results })))
}

/// Resolve the insert index for a new track. `at < 0` = append after the last
/// instrument/audio track (effect/master tracks do not count).
/// Shared by WIGSCRIPT chain/fluent and `codewig-cli track new` / `chain`.
pub fn resolve_track_at(client: &Client, at: i32) -> Result<i32, ExecuteError> {
    if at >= 0 {
        return Ok(at);
    }
    let list = client
        .track_list()?
        .ok_or_else(|| usage("track.list empty"))?;
    let tracks = list
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or_else(|| usage("track.list missing tracks"))?;
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

fn ensure_clip(client: &Client, track: &str, slot: i32, beats: i32) -> Result<i32, ExecuteError> {
    ensure_clip_at(client, track, slot, beats, None)
}

fn ensure_clip_at(
    client: &Client,
    track: &str,
    slot: i32,
    beats: i32,
    name: Option<&str>,
) -> Result<i32, ExecuteError> {
    // If slot already has content, reuse.
    if let Ok(Some(list)) = client.clip_list(track) {
        if let Some(arr) = list.get("clips").and_then(Value::as_array) {
            if let Some(c) = arr
                .iter()
                .find(|c| c.get("slot").and_then(Value::as_i64) == Some(slot as i64))
            {
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
    client.clip_new(track, Some(slot), beats, name)?;
    // Poll; the extension errors on an empty slot (no silent auto-create),
    // so retry the create once if the first one is still racing.
    match wait_clip_content(client, track, slot) {
        Ok(()) => Ok(slot),
        Err(_) => {
            // best-effort: one retry create; if the slot is still empty the
            // following note write errors in the extension — user sees it there
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
) -> Result<i32, ExecuteError> {
    match clip {
        None => ensure_clip(client, track, default_slot, beats),
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
) -> Result<Option<Value>, ExecuteError> {
    let slot = ensure_clip(client, track, slot, beats)?;
    let playable: Vec<NoteSpec> = notes.iter().filter(|n| n.vel > 0).cloned().collect();
    // One RPC: clear all + write (empty playable = clear only / rests)
    Ok(client.clip_replace_notes(track, slot, &playable)?)
}

fn music(client: &Client, session: &mut MusicSession, cmd: &MusicCmd) -> Result<Option<Value>, ExecuteError> {
    let track = if cmd.target.track.is_empty() {
        return Err(usage("music line needs a track name, e.g. bass: n \"c e g\""));
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
        )?,
        MusicAction::Notes | MusicAction::Arp(_) => {
            let (notes, _) =
                expand_music_line(cmd, session.scale.as_ref(), session.steps_per_bar)?;
            notes
        }
    };
    apply_note_mods(&mut notes, &cmd.note_mods);

    // +params need cursor track; pure note write uses track ref in clip.* — skip select
    if !cmd.params.is_empty() {
        client.track_select(&track)?;
        wait_cursor();
        let sets: Vec<(String, f64)> = cmd
            .params
            .iter()
            .map(|p| (p.name.clone(), p.value))
            .collect();
        client.param_set_multi(&sets)?;
    }

    // Notes path already resolved slot (incl. create); don't re-ensure without name.
    let playable: Vec<NoteSpec> = notes.iter().filter(|n| n.vel > 0).cloned().collect();
    Ok(client.clip_replace_notes(&track, slot, &playable)?)
}

/// Insert one device by catalog/Bitwig/library name (alias resolution,
/// Sampler/Drum-Machine guard). Shared by WIGSCRIPT and `codewig-cli chain`.
pub fn add_device(client: &Client, catalog: &str) -> Result<Option<Value>, ExecuteError> {
    // Out of scope: multi-pad / sample hosts (not mono Bitwig modules).
    // Single client-side guard; the extension re-checks (DeviceCatalog.isBanned).
    if is_banned(catalog) {
        return Err(usage(format!(
            "device '{catalog}' not insertable (Sampler / Drum Machine out of scope)"
        )));
    }
    // Known aliases → canonical Bitwig name; otherwise pass name through (UUID / library file).
    let name = catalog_to_bitwig(catalog).unwrap_or_else(|| catalog.trim().to_string());
    if name.is_empty() {
        return Err(usage("device name empty"));
    }
    Ok(client.device_add(&name)?)
}

fn chain(
    client: &Client,
    session: &mut MusicSession,
    cmd: &ChainCmd,
) -> Result<Option<Value>, ExecuteError> {
    let at = resolve_track_at(client, -1)?;
    let created = client.track_new(&cmd.kind, at, Some(&cmd.name))?;
    wait_track_named(client, &cmd.name)?;
    client.track_select(&cmd.name)?;
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

    // best-effort: starter clip is a convenience; track + devices are already live
    let _ = ensure_clip(client, &cmd.name, session.default_slot, session.default_beats);

    Ok(Some(json!({
        "chain": { "name": cmd.name, "kind": cmd.kind, "drum_kit": cmd.drum_kit },
        "created": created,
        "devices": added,
    })))
}

/// Shared chain orchestration for `codewig-cli chain` (the WIGSCRIPT AST
/// variant is [`chain`] above): create track, wait, select, insert devices in
/// order, optional starter clip. `name` optional — without it Bitwig's default
/// track name stays and no clip is created. Progress goes to stderr.
pub fn run_chain(
    client: &Client,
    kind: &str,
    name: Option<&str>,
    at: i32,
    devices: &[String],
) -> Result<Value, ExecuteError> {
    if devices.is_empty() {
        return Err(usage("chain needs at least one device (e.g. Polymer Delay+)"));
    }

    let at = resolve_track_at(client, at)?;
    let created = client
        .track_new(kind, at, name)?
        .unwrap_or(Value::Bool(true));
    eprintln!("track: {created}");

    // Bitwig renames async — poll until name visible
    if let Some(n) = name {
        wait_track_named(client, n)?;
        let sel = client
            .track_select(n)?
            .unwrap_or(Value::Bool(true));
        eprintln!("select: {sel}");
        wait_cursor();
    }

    let mut added = Vec::new();
    for dev in devices {
        let r = add_device(client, dev)?.unwrap_or(Value::Bool(true));
        added.push(json!({ "device": dev, "result": r }));
        wait_cursor();
    }

    // Optional first empty clip for live switching
    if let Some(n) = name {
        match client.clip_new(n, None, 4, Some("A")) {
            Ok(Some(clip)) => eprintln!("clip: {clip}"),
            Ok(None) => eprintln!("clip: ok"),
            Err(e) => eprintln!("clip note: {e} (create manually with: codewig-cli clip new {n})"),
        }
    }

    Ok(json!({
        "chain": {
            "track_type": kind,
            "name": name,
            "at": at,
            "devices": devices,
        },
        "added": added,
        "next": [
            "clip new <track> --name B   # more slots for live switch",
            "clip launch <track> 0",
            "param list / param set",
            "track mute 1 3 6 / track solo 0 2",
        ]
    }))
}

fn fluent(
    client: &Client,
    session: &mut MusicSession,
    cmd: &FluentCmd,
) -> Result<Option<Value>, ExecuteError> {
    let needs_cursor = cmd.steps.iter().any(|s| {
        matches!(
            s,
            FluentStep::Device(_) | FluentStep::Add(_)
        )
    });

    if cmd.create {
        let at = resolve_track_at(client, -1)?;
        client.track_new("instrument", at, Some(&cmd.track))?;
        wait_track_named(client, &cmd.track)?;
    }
    // device_add / param need cursor; pure mute/clip/notes use track refs
    if needs_cursor || cmd.create {
        client.track_select(&cmd.track)?;
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
                    expand_music_line(&music, session.scale.as_ref(), session.steps_per_bar)?;
                apply_note_mods(&mut notes, mods);
                log.push(json!({
                    "notes": write_notes(client, &cmd.track, slot0, session.default_beats, &notes)?
                }));
            }
            FluentStep::Mute => {
                log.push(json!({
                    "mute": client.track_mute(std::slice::from_ref(&cmd.track), true)?
                }));
            }
            FluentStep::Rename(name) => {
                log.push(json!({
                    "rename": client.track_rename(&cmd.track, name)?
                }));
            }
            FluentStep::Delete => {
                log.push(json!({
                    "delete": client.track_delete(&cmd.track)?
                }));
            }
            FluentStep::ClipAction(a) => {
                ensure_clip(client, &cmd.track, slot0, session.default_beats)?;
                match a {
                    ClipAction::Start => {
                        log.push(json!({
                            "clip": client.clip_launch(&cmd.track, slot0)?
                        }));
                    }
                    ClipAction::Stop => {
                        log.push(json!({
                            "clip": client.clip_stop(&cmd.track)?
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
                    match &cc.action {
                        LaunchAction::Start => {
                            log.push(json!({
                                "clip": client.clip_launch(&track, r.slot)?
                            }));
                        }
                        LaunchAction::Stop => {
                            log.push(json!({
                                "clip": client.clip_stop(&track)?
                            }));
                        }
                        LaunchAction::Rename(name) => {
                            log.push(json!({
                                "clip": client.clip_rename(&track, r.slot, name)?
                            }));
                        }
                        LaunchAction::Delete => {
                            log.push(json!({
                                "clip": client.clip_delete(&track, r.slot)?
                            }));
                        }
                    }
                }
            }
        }
    }

    Ok(Some(json!({ "fluent": cmd.track, "create": cmd.create, "slot": slot0, "steps": log })))
}

fn param(client: &Client, cmd: &ParamCmd) -> Result<Option<Value>, ExecuteError> {
    // YAML optional: display→wire when documented; else raw wire 0..1 passthrough.
    let cat = super::param_catalog::catalog();
    let sets = cat
        .map_param_sets(&cmd.device.catalog_name, &cmd.params)
        .map_err(ExecuteError::Catalog)?;

    client.track_select(&cmd.track)?;
    wait_cursor();

    // Focus device by name if possible
    if let Ok(Some(list)) = client.device_list() {
        if let Some(arr) = list.get("devices").and_then(Value::as_array) {
            let want = catalog_to_bitwig(&cmd.device.catalog_name)
                .or_else(|| {
                    cat.resolve(&cmd.device.catalog_name)
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
                    // best-effort: param.set matches by name even without cursor focus
                    let _ = client.device_select(idx as i32);
                    wait_cursor();
                }
            }
        }
    }

    Ok(client.param_set_multi(&sets)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse::parse_music_line;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::Arc;

    // ── Fake extension server ────────────────────────────────────────
    //
    // Loopback TCP stand-in for Codewig.bwextension: speaks the real wire
    // framing (4-byte BE length + JSON, see protocol.rs) and answers each
    // request through `handler`. Every request is also forwarded to the test
    // so wire traffic can be asserted. Declare the `FakeExt` **before** the
    // `Client` in a test: drop order then closes the client stream first, so
    // the serve loop sees EOF and `Drop` can join the thread.

    struct FakeExt {
        port: u16,
        rx: Receiver<Value>,
        stop: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl FakeExt {
        fn start<F>(mut handler: F) -> Self
        where
            F: FnMut(&Value) -> Value + Send + 'static,
        {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let (tx, rx) = channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stop_thread = stop.clone();
            let handle = std::thread::spawn(move || {
                while !stop_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => Self::serve(stream, &tx, &mut handler),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                port,
                rx,
                stop,
                handle: Some(handle),
            }
        }

        fn serve<F: FnMut(&Value) -> Value>(
            mut stream: TcpStream,
            tx: &Sender<Value>,
            handler: &mut F,
        ) {
            // Windows: accepted sockets inherit the listener's nonblocking mode.
            let _ = stream.set_nonblocking(false);
            loop {
                let mut len = [0u8; 4];
                if stream.read_exact(&mut len).is_err() {
                    return;
                }
                let mut body = vec![0u8; u32::from_be_bytes(len) as usize];
                if stream.read_exact(&mut body).is_err() {
                    return;
                }
                let Ok(req) = serde_json::from_slice::<Value>(&body) else {
                    return;
                };
                let _ = tx.send(req.clone());
                let out = serde_json::to_vec(&handler(&req)).unwrap();
                let mut frame = Vec::with_capacity(4 + out.len());
                frame.extend_from_slice(&(out.len() as u32).to_be_bytes());
                frame.extend_from_slice(&out);
                if stream.write_all(&frame).is_err() {
                    return;
                }
            }
        }

        /// Requests received so far, in arrival order.
        fn drain(&self) -> Vec<Value> {
            self.rx.try_iter().collect()
        }
    }

    impl Drop for FakeExt {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }

    fn fake_client(port: u16) -> Client {
        Client::new("127.0.0.1", port, 2000)
    }

    fn ok(v: Value) -> Value {
        json!({ "ok": true, "result": v })
    }

    fn ext_err(code: &str, msg: &str) -> Value {
        json!({ "ok": false, "error": { "code": code, "msg": msg } })
    }

    fn cmd_of(req: &Value) -> &str {
        req.get("c").and_then(Value::as_str).unwrap_or("")
    }

    fn cmds(reqs: &[Value]) -> Vec<&str> {
        reqs.iter().map(cmd_of).collect()
    }

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

    #[test]
    fn execute_error_keeps_extension_code() {
        let e: ExecuteError = Error::Extension {
            code: "NO_TRACK".into(),
            msg: "no such track".into(),
        }
        .into();
        assert!(matches!(e, ExecuteError::Client(_)));
        let text = e.to_string();
        assert!(text.contains("NO_TRACK"), "{text}");
        assert!(text.contains("no such track"), "{text}");
    }

    #[test]
    fn execute_error_from_scale_and_expand() {
        let e: ExecuteError = Scale::new("C", "nope").unwrap_err().into();
        assert!(matches!(e, ExecuteError::Scale(_)));
        assert!(e.to_string().contains("unknown scale"), "{e}");
    }

    // ── resolve_track_at ─────────────────────────────────────────────

    #[test]
    fn resolve_track_at_positive_index_short_circuits() {
        // at >= 0 never touches the wire — a default client (port 9470, no
        // server) must still succeed.
        let client = Client::default();
        assert_eq!(resolve_track_at(&client, 0).unwrap(), 0);
        assert_eq!(resolve_track_at(&client, 7).unwrap(), 7);
    }

    #[test]
    fn resolve_track_at_append_counts_only_instrument_and_audio() {
        let fake = FakeExt::start(|req| {
            assert_eq!(cmd_of(req), "track.list");
            ok(json!({ "tracks": [
                { "type": "instrument" },
                { "type": "audio" },
                { "type": "effect" },
                { "type": "master" },
                { "type": "INSTRUMENT" }, // case-insensitive
                { "type": "group" },
            ]}))
        });
        let client = fake_client(fake.port);
        assert_eq!(resolve_track_at(&client, -1).unwrap(), 3);
        assert_eq!(fake.drain().len(), 1, "exactly one track.list call");
    }

    #[test]
    fn resolve_track_at_empty_result_is_usage_error() {
        let fake = FakeExt::start(|_| ok(Value::Null));
        let client = fake_client(fake.port);
        let e = resolve_track_at(&client, -1).unwrap_err();
        assert!(matches!(e, ExecuteError::Usage(_)), "{e}");
        assert!(e.to_string().contains("track.list empty"), "{e}");
    }

    #[test]
    fn resolve_track_at_missing_tracks_key_is_usage_error() {
        let fake = FakeExt::start(|_| ok(json!({ "unexpected": true })));
        let client = fake_client(fake.port);
        let e = resolve_track_at(&client, -2).unwrap_err();
        assert!(matches!(e, ExecuteError::Usage(_)), "{e}");
        assert!(e.to_string().contains("track.list missing tracks"), "{e}");
    }

    // ── wait_track_named ─────────────────────────────────────────────

    #[test]
    fn wait_track_named_succeeds_on_third_poll() {
        let polls = Arc::new(AtomicUsize::new(0));
        let p = polls.clone();
        let fake = FakeExt::start(move |req| {
            assert_eq!(cmd_of(req), "track.list");
            let n = p.fetch_add(1, Ordering::Relaxed);
            let tracks = if n < 2 {
                json!([])
            } else {
                json!([{ "name": "bass" }])
            };
            ok(json!({ "tracks": tracks }))
        });
        let client = fake_client(fake.port);
        wait_track_named(&client, "bass").unwrap();
        assert_eq!(polls.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn wait_track_named_times_out_with_usage_error() {
        let fake = FakeExt::start(|_| ok(json!({ "tracks": [{ "name": "other" }] })));
        let client = fake_client(fake.port);
        let t0 = std::time::Instant::now();
        let e = wait_track_named(&client, "bass").unwrap_err();
        assert!(matches!(e, ExecuteError::Usage(_)), "{e}");
        assert!(e.to_string().contains("not visible after create"), "{e}");
        // 20 polls × 20ms ceiling ≈ 0.4s — well inside the 2s client timeout.
        assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    }

    // ── ensure_clip_at ───────────────────────────────────────────────

    #[test]
    fn ensure_clip_at_reuses_slot_with_content() {
        let fake = FakeExt::start(|req| match cmd_of(req) {
            "clip.list" => ok(json!({ "clips": [{ "slot": 0, "hasContent": true }] })),
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        assert_eq!(ensure_clip_at(&client, "bass", 0, 4, None).unwrap(), 0);
        let reqs = fake.drain();
        assert_eq!(cmds(&reqs), ["clip.list"], "no clip.new for occupied slot");
    }

    #[test]
    fn ensure_clip_at_creates_when_slot_empty() {
        let created = Arc::new(AtomicBool::new(false));
        let c = created.clone();
        let fake = FakeExt::start(move |req| match cmd_of(req) {
            "clip.list" => ok(json!({ "clips": [
                { "slot": 0, "hasContent": c.load(Ordering::Relaxed) }
            ]})),
            "clip.new" => {
                c.store(true, Ordering::Relaxed);
                ok(json!({ "created": true }))
            }
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        assert_eq!(ensure_clip_at(&client, "bass", 0, 4, Some("verse")).unwrap(), 0);
        let reqs = fake.drain();
        assert_eq!(cmds(&reqs), ["clip.list", "clip.new", "clip.list"]);
        let new = &reqs[1];
        assert_eq!(new.get("track").and_then(Value::as_str), Some("bass"));
        assert_eq!(new.get("slot").and_then(Value::as_i64), Some(0));
        assert_eq!(new.get("beats").and_then(Value::as_i64), Some(4));
        assert_eq!(new.get("name").and_then(Value::as_str), Some("verse"));
    }

    #[test]
    fn ensure_clip_at_retries_create_when_first_races() {
        // First create never becomes visible (raced Bitwig) → one retry create.
        // Costs one full wait_clip_content sweep (~0.5s) — still fast enough.
        let creates = Arc::new(AtomicUsize::new(0));
        let n = creates.clone();
        let fake = FakeExt::start(move |req| match cmd_of(req) {
            "clip.list" => ok(json!({ "clips": [
                { "slot": 0, "hasContent": n.load(Ordering::Relaxed) >= 2 }
            ]})),
            "clip.new" => {
                n.fetch_add(1, Ordering::Relaxed);
                ok(json!({ "created": true }))
            }
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        assert_eq!(ensure_clip_at(&client, "bass", 0, 4, None).unwrap(), 0);
        assert_eq!(creates.load(Ordering::Relaxed), 2, "initial create + one retry");
    }

    #[test]
    fn ensure_clip_at_create_error_propagates() {
        let fake = FakeExt::start(|req| match cmd_of(req) {
            "clip.list" => ok(json!({ "clips": [] })),
            "clip.new" => ext_err("NO_TRACK", "no track named bass"),
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        let e = ensure_clip_at(&client, "bass", 0, 4, None).unwrap_err();
        match e {
            ExecuteError::Client(Error::Extension { code, msg }) => {
                assert_eq!(code, "NO_TRACK");
                assert!(msg.contains("no track named bass"), "{msg}");
            }
            other => panic!("expected extension error, got {other}"),
        }
        let reqs = fake.drain();
        // clip.new is not idempotent: one attempt, no wait/retry after the error.
        assert_eq!(cmds(&reqs), ["clip.list", "clip.new"]);
    }

    // ── execute_line smoke (parse → execute → wire) ──────────────────

    #[test]
    fn execute_line_notes_writes_replace_notes() {
        let fake = FakeExt::start(|req| match cmd_of(req) {
            "clip.list" => ok(json!({ "clips": [{ "slot": 0, "hasContent": true }] })),
            "clip.replace-notes" => ok(json!({ "written": true })),
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        let mut session = MusicSession::default();
        let line = parse_music_line(r#"bass: n "c e g""#).unwrap();
        execute_line(&client, &mut session, line).unwrap();

        let reqs = fake.drain();
        assert_eq!(cmds(&reqs), ["clip.list", "clip.replace-notes"]);
        let rn = &reqs[1];
        assert_eq!(rn.get("track").and_then(Value::as_str), Some("bass"));
        assert_eq!(rn.get("slot").and_then(Value::as_i64), Some(0));
        // space events = 1 beat each → 16th steps 0/4/8; Bitwig octaves (c = 60)
        let notes: Vec<(i64, i64, i64, f64)> = rn
            .get("notes")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(|n| {
                (
                    n["step"].as_i64().unwrap(),
                    n["key"].as_i64().unwrap(),
                    n["vel"].as_i64().unwrap(),
                    n["dur"].as_f64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            notes,
            vec![(0, 60, 100, 4.0), (4, 64, 100, 4.0), (8, 67, 100, 4.0)]
        );
    }

    #[test]
    fn execute_line_new_track_creates_then_selects() {
        let created = Arc::new(AtomicBool::new(false));
        let c = created.clone();
        let fake = FakeExt::start(move |req| match cmd_of(req) {
            "track.list" => {
                let mut tracks = vec![json!({ "name": "lead", "type": "instrument" })];
                if c.load(Ordering::Relaxed) {
                    tracks.push(json!({ "name": "x", "type": "instrument" }));
                }
                ok(json!({ "tracks": tracks }))
            }
            "track.new" => {
                c.store(true, Ordering::Relaxed);
                ok(json!({ "created": true }))
            }
            "track.select" => ok(json!({ "selected": true })),
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        let mut session = MusicSession::default();
        let line = parse_music_line("new track(x)").unwrap();
        execute_line(&client, &mut session, line).unwrap();

        let reqs = fake.drain();
        assert_eq!(
            cmds(&reqs),
            ["track.list", "track.new", "track.list", "track.select"]
        );
        let tn = &reqs[1];
        assert_eq!(tn.get("type").and_then(Value::as_str), Some("instrument"));
        assert_eq!(tn.get("at").and_then(Value::as_i64), Some(1)); // append after "lead"
        assert_eq!(tn.get("name").and_then(Value::as_str), Some("x"));
        assert_eq!(reqs[3].get("ref").and_then(Value::as_str), Some("x"));
    }

    #[test]
    fn execute_line_param_unknown_device_passthrough() {
        let fake = FakeExt::start(|req| match cmd_of(req) {
            "track.select" => ok(json!({ "selected": true })),
            "device.list" => ok(json!({ "devices": [] })),
            "param.set" => ok(json!({ "set": true })),
            other => panic!("unexpected request {other}"),
        });
        let client = fake_client(fake.port);
        let mut session = MusicSession::default();
        let line = parse_music_line("bass&nosuchdev: cutoff(0.5)").unwrap();
        execute_line(&client, &mut session, line).unwrap();

        let reqs = fake.drain();
        assert_eq!(cmds(&reqs), ["track.select", "device.list", "param.set"]);
        assert_eq!(reqs[0].get("ref").and_then(Value::as_str), Some("bass"));
        // No YAML entry for "nosuchdev" → raw wire 0..1 passthrough, name as typed.
        let sets = reqs[2].get("sets").and_then(Value::as_array).unwrap();
        assert_eq!(sets, &vec![json!({ "name": "cutoff", "v": 0.5 })]);
    }
}
