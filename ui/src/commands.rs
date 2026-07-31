//! Parser/dispatcher for the REPL input in codewig-live.
//!
//! Accepts the same command syntax as `cliwig` so the sidebar commands work
//! out of the box. A Strudel-style compact syntax will be added later as an
//! alternative input mode.

use cliwig_core::{Client, NoteSpec};
use serde_json::Value;

pub fn run(client: &Client, input: &str) -> Result<Option<Value>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let words = shlex::split(trimmed).ok_or_else(|| "unmatched quote in command".to_string())?;
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
        _ => Err(format!("unknown command: {head}")),
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
        _ => Err(format!("set: unknown target '{target}'")),
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
        "move" => {
            let r#ref = next_string(&mut args, "ref")?;
            let mut before: Option<String> = None;
            let mut after: Option<String> = None;
            let mut to: Option<i32> = None;
            while let Some(tok) = args.next() {
                match tok {
                    "--before" => before = Some(next_string(&mut args, "before")?),
                    "--after" => after = Some(next_string(&mut args, "after")?),
                    "--to" => to = Some(next_parse(&mut args, "to")?),
                    _ => return Err(format!("track move: unexpected '{tok}'")),
                }
            }
            client
                .track_move(&r#ref, before.as_deref(), after.as_deref(), to)
                .map_err(|e| e.to_string())
        }
        "mute" => {
            let (refs, off) = collect_refs_with_flag(args)?;
            if refs.is_empty() {
                return Err("track mute: need at least one ref".into());
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
                if name.is_some() || id.is_some() || value.is_some() {
                    return Err("param set: --set cannot be combined with --name/--id/--value".into());
                }
                return client.param_set_multi(&sets).map_err(|e| e.to_string());
            }

            let v = value.ok_or("param set: missing --value")?;
            match (name, id) {
                (Some(n), None) => client.param_set_name_value(&n, v).map_err(|e| e.to_string()),
                (None, Some(i)) => client.param_set_id_value(&i, v).map_err(|e| e.to_string()),
                _ => Err("param set: need --name or --id (or --set pairs)".into()),
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
        "note" => {
            let track = next_string(&mut args, "track")?;
            let slot: i32 = next_parse(&mut args, "slot")?;
            let mut notes: Vec<NoteSpec> = Vec::new();
            for spec in args {
                notes.push(parse_note(spec)?);
            }
            if notes.is_empty() {
                return Err("clip note: need at least one note".into());
            }
            client
                .clip_set_notes(&track, slot, &notes)
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
                    "--key" => key = Some(parse_key(&next_string(&mut args, "key")?)?),
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

fn parse_name_eq_value(s: &str) -> Result<(String, f64), String> {
    let (n, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected name=value, got '{s}'"))?;
    let val: f64 = v.parse().map_err(|_| format!("bad value in '{s}'"))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!("value must be 0..1, got {val}"));
    }
    Ok((n.trim().to_string(), val))
}

fn parse_note(s: &str) -> Result<NoteSpec, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return Err(format!("expected step:key[:vel[:dur]], got '{s}'"));
    }
    let step: i32 = parts[0]
        .trim()
        .parse()
        .map_err(|_| format!("bad step in '{s}'"))?;
    if step < 0 {
        return Err(format!("step must be >= 0, got {step}"));
    }
    let key = parse_key(parts[1])?;
    let vel: i32 = match parts.get(2) {
        Some(v) => {
            let v: i32 = v.trim().parse().map_err(|_| format!("bad vel in '{s}'"))?;
            if !(1..=127).contains(&v) {
                return Err(format!("vel must be 1..127, got {v}"));
            }
            v
        }
        None => 100,
    };
    let dur: f64 = match parts.get(3) {
        Some(d) => {
            let d: f64 = d.trim().parse().map_err(|_| format!("bad dur in '{s}'"))?;
            if d <= 0.0 {
                return Err(format!("dur must be > 0, got {d}"));
            }
            d
        }
        None => 1.0,
    };
    Ok(NoteSpec { step, key, vel, dur })
}

fn parse_key(s: &str) -> Result<i32, String> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i32>() {
        if !(0..=127).contains(&n) {
            return Err(format!("key must be 0..127, got {n}"));
        }
        return Ok(n);
    }
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Err("empty key".into());
    }
    let base = match bytes[0].to_ascii_uppercase() {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return Err(format!("bad note name '{s}'")),
    };
    let mut idx = 1;
    let mut semitone = base;
    if idx < bytes.len() && bytes[idx] == b'#' {
        semitone += 1;
        idx += 1;
    } else if idx < bytes.len() && (bytes[idx] == b'b' || bytes[idx] == b'B') {
        semitone -= 1;
        idx += 1;
    }
    let octave_str = &s[idx..];
    if octave_str.is_empty() {
        return Err(format!("missing octave in '{s}'"));
    }
    let octave: i32 = octave_str
        .parse()
        .map_err(|_| format!("bad octave in '{s}'"))?;
    let midi = (octave + 2) * 12 + semitone;
    if !(0..=127).contains(&midi) {
        return Err(format!("note '{s}' out of MIDI range ({midi})"));
    }
    Ok(midi)
}
