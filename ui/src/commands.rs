//! Input dispatcher for codewig-live.
//!
//! **WIGSCRIPT first** (same language as sidebar): `mute(kick)`, `s(1).start`,
//! `new track(bass).device(Polymer)`, `bass: n "c e g"`, …
//!
//! Fallback: legacy flat CLI tokens (`track mute kick`, `play`, …) for old habits
//! and `> track list` passthrough.

use codewig_core::music::{execute_line, key_to_midi, parse_music_line, MusicLine, MusicSession};
use codewig_core::{parse_name_eq_value, parse_note_spec, Client, NoteSpec};
use serde_json::Value;

/// Run one input line. `session` holds key/scale across lines.
pub fn run(
    client: &Client,
    session: &mut MusicSession,
    input: &str,
) -> Result<Option<Value>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    match parse_music_line(trimmed) {
        Ok(MusicLine::Empty) => Ok(None),
        Ok(MusicLine::PassThrough(cmd)) => legacy_cli(client, &cmd),
        // ExecuteError Display keeps the extension error code in the text.
        Ok(line) => execute_line(client, session, line).map_err(|e| e.to_string()),
        // Not WIGSCRIPT → legacy flat commands (same words as `codewig-cli` without binary name)
        Err(_) => legacy_cli(client, trimmed),
    }
}

// ── Legacy CLI-token path (compat) ─────────────────────────────────

fn legacy_cli(client: &Client, input: &str) -> Result<Option<Value>, String> {
    let words = shlex::split(input).ok_or_else(|| "unmatched quote in command".to_string())?;
    if words.is_empty() {
        return Ok(None);
    }
    dispatch(client, &words)
}

fn dispatch(client: &Client, words: &[String]) -> Result<Option<Value>, String> {
    let mut args = words.iter().map(|s| s.as_str());
    let head = args.next().ok_or("empty command")?;

    match head {
        "ping" => client.ping().map_err(|e| e.to_string()),
        "status" => client.status().map_err(|e| e.to_string()),
        "play" => client.play().map_err(|e| e.to_string()),
        "stop" => client.stop().map_err(|e| e.to_string()),
        "set" => dispatch_set(client, args),
        "track" => dispatch_track(client, args),
        "device" => dispatch_device(client, args),
        "param" => dispatch_param(client, args),
        "clip" => dispatch_clip(client, args),
        "scene" => dispatch_scene(client, args),
        _ => Err(format!(
            "unknown command: {head}\n\
             WIGSCRIPT: mute(kick) | s(1).start | new track(b).device(Polymer) | bass: n \"c e g\"\n\
             legacy: play | track list | clip launch <t> <slot>"
        )),
    }
}

fn dispatch_set<'a>(
    client: &Client,
    mut args: impl Iterator<Item = &'a str>,
) -> Result<Option<Value>, String> {
    let target = args.next().ok_or("set: missing target")?;
    match target {
        "tempo" => {
            let bpm: f64 = next_parse(&mut args, "bpm")?;
            ensure_no_more(args)?;
            client.set_tempo(bpm).map_err(|e| e.to_string())
        }
        _ => Err(format!("set: unknown target '{target}' (or use WIGSCRIPT: tempo 128)")),
    }
}

fn dispatch_track<'a>(
    client: &Client,
    mut args: impl Iterator<Item = &'a str>,
) -> Result<Option<Value>, String> {
    let action = args.next().ok_or("track: missing action")?;
    match action {
        "list" => {
            ensure_no_more(args)?;
            client.track_list().map_err(|e| e.to_string())
        }
        "new" => {
            let mut kind = "instrument".to_string();
            let mut name: Option<String> = None;
            let mut at: i32 = -1;
            while let Some(tok) = args.next() {
                match tok {
                    "--name" | "-n" => name = Some(next_string(&mut args, "name")?),
                    "--at" => at = next_parse(&mut args, "at")?,
                    _ if !tok.starts_with('-') && kind == "instrument" => kind = tok.to_string(),
                    _ => return Err(format!("track new: unexpected '{tok}'")),
                }
            }
            client
                .track_new(&kind, at, name.as_deref())
                .map_err(|e| e.to_string())
        }
        "select" => {
            let r#ref = next_string(&mut args, "ref")?;
            ensure_no_more(args)?;
            client.track_select(&r#ref).map_err(|e| e.to_string())
        }
        "rename" => {
            let r#ref = next_string(&mut args, "ref")?;
            let name = next_string(&mut args, "name")?;
            ensure_no_more(args)?;
            client.track_rename(&r#ref, &name).map_err(|e| e.to_string())
        }
        "delete" => {
            let r#ref = next_string(&mut args, "ref")?;
            ensure_no_more(args)?;
            client.track_delete(&r#ref).map_err(|e| e.to_string())
        }
        "mute" => {
            let (refs, off) = collect_refs_with_flag(args)?;
            if refs.is_empty() {
                return Err("track mute: need ref (or WIGSCRIPT: mute(kick))".into());
            }
            client.track_mute(&refs, !off).map_err(|e| e.to_string())
        }
        "solo" => {
            let (refs, off) = collect_refs_with_flag(args)?;
            if refs.is_empty() {
                return Err("track solo: need at least one ref".into());
            }
            client.track_solo(&refs, !off).map_err(|e| e.to_string())
        }
        "volume" => {
            let r#ref = next_string(&mut args, "ref")?;
            let value: f64 = next_parse(&mut args, "value")?;
            ensure_no_more(args)?;
            client.track_volume(&r#ref, value).map_err(|e| e.to_string())
        }
        _ => Err(format!("track: unknown action '{action}'")),
    }
}

fn dispatch_device<'a>(
    client: &Client,
    mut args: impl Iterator<Item = &'a str>,
) -> Result<Option<Value>, String> {
    let action = args.next().ok_or("device: missing action")?;
    match action {
        "list" => {
            ensure_no_more(args)?;
            client.device_list().map_err(|e| e.to_string())
        }
        "add" => {
            let name = next_string(&mut args, "name")?;
            ensure_no_more(args)?;
            client.device_add(&name).map_err(|e| e.to_string())
        }
        "select" => {
            let index: i32 = next_parse(&mut args, "index")?;
            ensure_no_more(args)?;
            client.device_select(index).map_err(|e| e.to_string())
        }
        "delete" => {
            let index: i32 = next_parse(&mut args, "index")?;
            ensure_no_more(args)?;
            client.device_delete(index).map_err(|e| e.to_string())
        }
        "on" => {
            let index: i32 = next_parse(&mut args, "index")?;
            ensure_no_more(args)?;
            client.device_enable(index, true).map_err(|e| e.to_string())
        }
        "off" => {
            let index: i32 = next_parse(&mut args, "index")?;
            ensure_no_more(args)?;
            client.device_enable(index, false).map_err(|e| e.to_string())
        }
        "move" => {
            let index: i32 = next_parse(&mut args, "index")?;
            let to: i32 = next_parse(&mut args, "to")?;
            ensure_no_more(args)?;
            client.device_move(index, to).map_err(|e| e.to_string())
        }
        _ => Err(format!("device: unknown action '{action}'")),
    }
}

fn dispatch_param<'a>(
    client: &Client,
    mut args: impl Iterator<Item = &'a str>,
) -> Result<Option<Value>, String> {
    let action = args.next().ok_or("param: missing action")?;
    match action {
        "list" => {
            ensure_no_more(args)?;
            client.param_list().map_err(|e| e.to_string())
        }
        "set" => {
            let mut name: Option<String> = None;
            let mut id: Option<String> = None;
            let mut value: Option<f64> = None;
            let mut sets: Vec<(String, f64)> = Vec::new();
            while let Some(tok) = args.next() {
                match tok {
                    "--name" | "-n" => name = Some(next_string(&mut args, "name")?),
                    "--id" => id = Some(next_string(&mut args, "id")?),
                    "--value" | "-v" => value = Some(next_parse(&mut args, "value")?),
                    "--set" => {
                        let pair = next_string(&mut args, "name=value")?;
                        let (n, v) = parse_name_eq_value(&pair)?;
                        sets.push((n, v));
                    }
                    _ => return Err(format!("param set: unexpected '{tok}'")),
                }
            }
            if !sets.is_empty() {
                return client.param_set_multi(&sets).map_err(|e| e.to_string());
            }
            let v = value.ok_or("param set: missing --value")?;
            match (name, id) {
                (Some(n), None) => client.param_set_name_value(&n, v).map_err(|e| e.to_string()),
                (None, Some(i)) => client.param_set_id_value(&i, v).map_err(|e| e.to_string()),
                _ => Err("param set: need --name or --id".into()),
            }
        }
        _ => Err(format!("param: unknown action '{action}'")),
    }
}

fn dispatch_clip<'a>(
    client: &Client,
    mut args: impl Iterator<Item = &'a str>,
) -> Result<Option<Value>, String> {
    let action = args.next().ok_or("clip: missing action")?;
    match action {
        "list" => {
            let track = next_string(&mut args, "track")?;
            ensure_no_more(args)?;
            client.clip_list(&track).map_err(|e| e.to_string())
        }
        "new" => {
            let track = next_string(&mut args, "track")?;
            let mut slot: Option<i32> = None;
            let mut beats: i32 = 4;
            let mut name: Option<String> = None;
            while let Some(tok) = args.next() {
                match tok {
                    "--slot" | "-s" => slot = Some(next_parse(&mut args, "slot")?),
                    "--beats" | "-b" => beats = next_parse(&mut args, "beats")?,
                    "--name" | "-n" => name = Some(next_string(&mut args, "name")?),
                    _ => return Err(format!("clip new: unexpected '{tok}'")),
                }
            }
            client
                .clip_new(&track, slot, beats, name.as_deref())
                .map_err(|e| e.to_string())
        }
        "launch" => {
            let track = next_string(&mut args, "track")?;
            let slot: i32 = next_parse(&mut args, "slot")?;
            ensure_no_more(args)?;
            client.clip_launch(&track, slot).map_err(|e| e.to_string())
        }
        "stop" => {
            let track = next_string(&mut args, "track")?;
            ensure_no_more(args)?;
            client.clip_stop(&track).map_err(|e| e.to_string())
        }
        "rename" => {
            let track = next_string(&mut args, "track")?;
            let slot: i32 = next_parse(&mut args, "slot")?;
            let name = next_string(&mut args, "name")?;
            ensure_no_more(args)?;
            client
                .clip_rename(&track, slot, &name)
                .map_err(|e| e.to_string())
        }
        "delete" => {
            let track = next_string(&mut args, "track")?;
            let slot: i32 = next_parse(&mut args, "slot")?;
            ensure_no_more(args)?;
            client.clip_delete(&track, slot).map_err(|e| e.to_string())
        }
        "note" => {
            let track = next_string(&mut args, "track")?;
            let slot: i32 = next_parse(&mut args, "slot")?;
            let mut notes: Vec<NoteSpec> = Vec::new();
            for spec in args {
                notes.push(parse_note_spec(spec)?);
            }
            if notes.is_empty() {
                return Err("clip note: need notes (or WIGSCRIPT: bass: n \"c e g\")".into());
            }
            client
                .clip_replace_notes(&track, slot, &notes)
                .map_err(|e| e.to_string())
        }
        "clear-notes" => {
            let track = next_string(&mut args, "track")?;
            let slot: i32 = next_parse(&mut args, "slot")?;
            let mut step: Option<i32> = None;
            let mut key: Option<i32> = None;
            while let Some(tok) = args.next() {
                match tok {
                    "--step" => step = Some(next_parse(&mut args, "step")?),
                    "--key" => key = Some(key_to_midi(&next_string(&mut args, "key")?).map_err(|e| e.to_string())?),
                    _ => return Err(format!("clip clear-notes: unexpected '{tok}'")),
                }
            }
            client
                .clip_clear_notes(&track, slot, step, key)
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("clip: unknown action '{action}'")),
    }
}

fn dispatch_scene<'a>(
    client: &Client,
    mut args: impl Iterator<Item = &'a str>,
) -> Result<Option<Value>, String> {
    let action = args.next().ok_or("scene: missing action")?;
    match action {
        "list" => {
            ensure_no_more(args)?;
            client.scene_list().map_err(|e| e.to_string())
        }
        "new" => {
            let name = args.next().map(|s| s.to_string());
            ensure_no_more(args)?;
            client.scene_new(name.as_deref()).map_err(|e| e.to_string())
        }
        "launch" => {
            let r#ref = next_string(&mut args, "ref")?;
            ensure_no_more(args)?;
            client.scene_launch(&r#ref).map_err(|e| e.to_string())
        }
        "stop" => {
            let r#ref = next_string(&mut args, "ref")?;
            ensure_no_more(args)?;
            client.scene_stop(&r#ref).map_err(|e| e.to_string())
        }
        "rename" => {
            let r#ref = next_string(&mut args, "ref")?;
            let name = next_string(&mut args, "name")?;
            ensure_no_more(args)?;
            client.scene_rename(&r#ref, &name).map_err(|e| e.to_string())
        }
        "delete" => {
            let r#ref = next_string(&mut args, "ref")?;
            ensure_no_more(args)?;
            client.scene_delete(&r#ref).map_err(|e| e.to_string())
        }
        _ => Err(format!("scene: unknown action '{action}'")),
    }
}

fn collect_refs_with_flag<'a>(
    args: impl Iterator<Item = &'a str>,
) -> Result<(Vec<String>, bool), String> {
    let mut refs = Vec::new();
    let mut off = false;
    for tok in args {
        if tok == "--off" {
            off = true;
        } else if !tok.starts_with('-') {
            refs.push(tok.to_string());
        } else {
            return Err(format!("unexpected flag '{tok}'"));
        }
    }
    Ok((refs, off))
}

fn next_string<'a>(
    args: &mut impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing argument: {label}"))
        .map(|s| s.to_string())
}

fn next_parse<'a, T: std::str::FromStr>(
    args: &mut impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    let s = args
        .next()
        .ok_or_else(|| format!("missing argument: {label}"))?;
    s.parse()
        .map_err(|e| format!("bad {label} '{s}': {e}"))
}

fn ensure_no_more<'a>(mut args: impl Iterator<Item = &'a str>) -> Result<(), String> {
    if let Some(tok) = args.next() {
        return Err(format!("unexpected argument '{tok}'"));
    }
    Ok(())
}
