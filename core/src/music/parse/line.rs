//! WIGSCRIPT line dispatch: `target: cmd "pattern" +params`, transport,
//! mute/unmute, scenes/clips, `track&device:` param lines.

use super::super::ast::*;
use super::fluent::{parse_device_in_paren, parse_fluent, parse_paren_arg};
use super::{ParseError, ParseResult};

/// Parse a single REPL line into a [`MusicLine`].
pub fn parse_music_line(input: &str) -> ParseResult<MusicLine> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(MusicLine::Empty);
    }

    let s = trimmed;

    // Transport
    if s == "play" {
        return Ok(MusicLine::Transport(TransportCmd::Play));
    }
    if s == "stop" {
        return Ok(MusicLine::Transport(TransportCmd::Stop));
    }

    // Tempo
    if let Some(rest) = s.strip_prefix("tempo ") {
        let bpm: f64 = rest
            .trim()
            .parse()
            .map_err(|_| ParseError::new("invalid tempo", 6, s.to_string()))?;
        return Ok(MusicLine::Tempo(bpm));
    }

    // Key
    if let Some(rest) = s.strip_prefix("k ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(ParseError::new(
                "key expects: k <root> <scale>, e.g. 'k C minor'",
                0,
                s.to_string(),
            ));
        }
        return Ok(MusicLine::Key {
            root: parts[0].to_string(),
            scale: parts[1].to_string(),
        });
    }

    // Mode switch
    if let Some(mode) = s.strip_prefix("mode ") {
        return Ok(MusicLine::ModeSwitch(mode.trim().to_string()));
    }

    // Passthrough: `> command`
    if let Some(rest) = s.strip_prefix('>') {
        return Ok(MusicLine::PassThrough(rest.trim().to_string()));
    }

    // Param: `track&device: decay(50)...`  (@ is scene/slot — & is track×device)
    if s.contains('&') && s.contains(':') && !s.starts_with("new ") {
        return parse_amp_param_cmd(s);
    }

    // Param legacy: `t(kick).d(kick.v9): decay(50)...`  — check BEFORE fluent (has `):`)
    // Long/short: t|track, d|device
    if (s.starts_with("t(") || s.starts_with("track(")) && s.contains("):") {
        return parse_param_cmd(s);
    }

    // Fluent: long/short head forms (all equivalent AST)
    //   new track(name) | new t(name)   — create
    //   track(name)     | t(name)       — existing track
    if s.starts_with("new track(")
        || s.starts_with("new t(")
        || s.starts_with("track(")
        || s.starts_with("t(")
    {
        return parse_fluent(s);
    }

    // Mute/Unmute
    if let Some(rest) = s.strip_prefix("mute(") {
        return parse_mute_cmd(rest, false, s);
    }
    if let Some(rest) = s.strip_prefix("unmute(") {
        return parse_mute_cmd(rest, true, s);
    }

    // `new scene` | `new s(verse)` | `new scene(verse)` — Bitwig scene row
    if s.starts_with("new scene") || s.starts_with("new s(") {
        return parse_new_scene(s);
    }

    // Scene: `s(1).start` | `s(verse).t(lead).c(new)` | `scene(0).stop`
    // long/short: s|scene, then .t|.track, .c|.clip
    if s.starts_with("s(") || s.starts_with("scene(") {
        return parse_scene_cmd(s);
    }

    // Clip control: `c(bass.0).start` | `clip(bass.0).start` (long/short)
    if s.starts_with("c(") || s.starts_with("clip(") {
        return parse_clip_ctrl_cmd(s);
    }

    // Chain: `!name dev1 dev2 ...`
    if let Some(rest) = s.strip_prefix('!') {
        return parse_chain(rest, s);
    }

    // Music command: `target: cmd "pattern" +params`
    parse_music_cmd(s)
}

fn parse_chain(rest: &str, full_input: &str) -> ParseResult<MusicLine> {
    let mut parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ParseError::new(
            "chain: missing name",
            1,
            full_input.to_string(),
        ));
    }

    let name_kind = parts.remove(0);
    let mut name = name_kind.to_string();
    let mut kind = "instrument".to_string();
    let mut drum_kit: Option<String> = None;

    // Check for `name:kit` suffix
    if let Some((n, kit)) = name_kind.split_once(':') {
        name = n.to_string();
        let kit_lower = kit.to_lowercase();
        match kit_lower.as_str() {
            "808" | "909" | "retro" | "default" => drum_kit = Some(kit_lower),
            _ => {
                // Could be a track kind
                kind = kit_lower;
            }
        }
    }

    Ok(MusicLine::Chain(ChainCmd {
        name,
        kind,
        drum_kit,
        devices: parts.iter().map(|s| s.to_string()).collect(),
    }))
}

fn parse_music_cmd(input: &str) -> ParseResult<MusicLine> {
    let s = input;

    // Find track target: everything before the first unquoted space after `:`
    // Format: `target: cmd "pattern" +params`
    // or: `d:808 "pattern"`  (shorthand)
    // or: `d "pattern"`      (focused drum track)

    // First, try to split on `: ` or just find the command
    let (target_str, rest) = if let Some(pos) = find_colon_sep(s) {
        let (t, r) = s.split_at(pos);
        (t, r[1..].trim_start()) // skip the colon
    } else {
        // No colon — could be `d "pattern"` shorthand
        return parse_shorthand(s);
    };

    // Parse target for track, @clip, :kit
    let target = parse_target(target_str)?;

    // Parse the action + pattern + optional note expression modifiers + params from rest
    let (action, pattern, rest2) = parse_action_pattern(rest, s)?;
    let (note_mods, rest3) = parse_note_mods(rest2, s)?;
    let (params, transpose, scale_transpose, rest4) = parse_params(rest3, s)?;
    reject_trailing(rest4, s)?;

    Ok(MusicLine::Music(MusicCmd {
        target,
        action,
        pattern,
        params,
        transpose,
        scale_transpose,
        note_mods,
    }))
}

/// Find the position of the first `:` that separates target from command.
fn find_colon_sep(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            // colon at position i — check if next char is space or we're at a cmd boundary
            if i + 1 >= bytes.len()
                || bytes[i + 1] == b' '
                || bytes[i + 1] == b'n'
                || bytes[i + 1] == b'd'
                || bytes[i + 1] == b'c'
            {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn parse_target(s: &str) -> ParseResult<Target> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Target {
            track: String::new(),
            clip: None,
            drum_kit: None,
        });
    }

    let mut track = s.to_string();
    let mut clip: Option<ClipRef> = None;
    let mut drum_kit: Option<String> = None;

    // Bitwig address: track@slot or track@sceneName (row = scene)
    if let Some(at_pos) = track.find('@') {
        let rest = track[at_pos + 1..].trim().to_string();
        track = track[..at_pos].to_string();
        clip = Some(if let Ok(i) = rest.parse::<i32>() {
            ClipRef::Slot(i)
        } else if rest.is_empty() {
            return Err(ParseError::new(
                "empty @ address — use track@0 or track@verse",
                at_pos,
                s.to_string(),
            ));
        } else {
            ClipRef::Name(rest)
        });
    }

    // Check for :kit suffix on track
    if let Some((t, kit)) = track.split_once(':') {
        let kit_lower = kit.to_lowercase();
        if matches!(kit_lower.as_str(), "808" | "909" | "retro" | "default") {
            track = t.to_string();
            drum_kit = Some(kit_lower);
        }
    }

    Ok(Target {
        track,
        clip,
        drum_kit,
    })
}

fn parse_shorthand(input: &str) -> ParseResult<MusicLine> {
    // `d "pattern"` or `d:808 "pattern"`
    let s = input;
    let bytes = s.as_bytes();

    // Find where the command name ends
    let cmd_end = bytes.iter().position(|&b| b == b' ').unwrap_or(bytes.len());
    let cmd = &s[..cmd_end];
    let rest = s[cmd_end..].trim_start();

    let (action, drum_kit) = parse_action_name(cmd)?;

    // Find quoted pattern + optional note expression modifiers
    let (pattern, rest2) = extract_quoted(rest, s)?;
    let (note_mods, rest3) = parse_note_mods(rest2, s)?;
    let (params, transpose, scale_transpose, rest4) = parse_params(rest3, s)?;
    reject_trailing(rest4, s)?;

    Ok(MusicLine::Music(MusicCmd {
        target: Target {
            track: String::new(),
            clip: None,
            drum_kit,
        },
        action,
        pattern,
        params,
        transpose,
        scale_transpose,
        note_mods,
    }))
}

fn parse_action_name(cmd: &str) -> ParseResult<(MusicAction, Option<String>)> {
    if cmd == "n" {
        Ok((MusicAction::Notes, None))
    } else if cmd == "d" || cmd.starts_with("d:") {
        // Hit markers (bd/hh/sd) are Drum Machine / Strudel — not mono Bitwig modules.
        Err(ParseError::new(
            "action 'd' removed: use .beat(4_) for percussion rhythm, or n \"c1\" / n \"36\" for exact notes",
            0,
            cmd.to_string(),
        ))
    } else if cmd == "chord" {
        Ok((MusicAction::Chord, None))
    } else if cmd == "arp" {
        Ok((MusicAction::Arp(ArpStyle::Up), None))
    } else if let Some(style) = cmd.strip_prefix("arp:") {
        Ok((MusicAction::Arp(parse_arp_style(style)?), None))
    } else {
        Err(ParseError::new(
            format!(
                "unknown music action: '{cmd}'. Expected n, chord, arp, arp:up|down|updown|rand \
                 (percussion: fluent .beat(...), not d \"bd hh\")"
            ),
            0,
            cmd.to_string(),
        ))
    }
}

fn parse_arp_style(s: &str) -> ParseResult<ArpStyle> {
    match s.to_lowercase().as_str() {
        "up" | "u" => Ok(ArpStyle::Up),
        "down" | "dn" | "d" => Ok(ArpStyle::Down),
        "updown" | "ud" | "up-down" => Ok(ArpStyle::UpDown),
        "rand" | "random" | "r" => Ok(ArpStyle::Random),
        other => Err(ParseError::new(
            format!("unknown arp style: '{other}' (up|down|updown|rand)"),
            0,
            s.to_string(),
        )),
    }
}

fn parse_action_pattern<'a>(
    rest: &'a str,
    full_input: &str,
) -> ParseResult<(MusicAction, String, &'a str)> {
    let trimmed = rest.trim();
    let (action_name, rest1) = trimmed.split_once(' ').ok_or_else(|| {
        ParseError::new(
            "expected action (n, chord, arp) followed by pattern",
            full_input.len() - rest.len(),
            full_input.to_string(),
        )
    })?;

    let (action, _drum_kit) = parse_action_name(action_name)?;

    let (pattern, rest2) = extract_quoted(rest1, full_input)?;

    Ok((action, pattern, rest2))
}

/// Extract a double-quoted string, returning content and remaining input.
pub(crate) fn extract_quoted<'a>(
    input: &'a str,
    full_input: &str,
) -> ParseResult<(String, &'a str)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('"') {
        return Err(ParseError::new(
            "expected quoted pattern starting with '\"'",
            full_input.len() - input.len(),
            full_input.to_string(),
        ));
    }

    let mut chars = trimmed[1..].char_indices();
    let mut escaped = false;
    let mut end = None;
    for (i, ch) in chars.by_ref() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            end = Some(i);
            break;
        }
    }

    match end {
        Some(i) => {
            // i is byte offset from trimmed[1..], so closing quote is at trimmed[1+i]
            let content_end = 1 + i + 1; // one past the closing quote
            let content = &trimmed[..content_end]; // includes both quotes
            let inner = &content[1..content.len() - 1]; // strip both quotes
            let unescaped = inner.replace("\\\"", "\"").replace("\\\\", "\\");
            let remainder = &trimmed[content_end..];
            Ok((unescaped, remainder))
        }
        None => Err(ParseError::new(
            "unclosed quote in pattern",
            full_input.len() - input.len(),
            full_input.to_string(),
        )),
    }
}

/// parse_params output: params, transpose, scale-transpose, remaining input.
type ParamsOut<'a> = (Vec<ParamSet>, Option<i32>, Option<i32>, &'a str);

fn parse_params<'a>(mut rest: &'a str, full_input: &str) -> ParseResult<ParamsOut<'a>> {
    let mut params = Vec::new();
    let mut transpose: Option<i32> = None;
    let mut scale_transpose: Option<i32> = None;

    rest = rest.trim_start();

    while !rest.is_empty() {
        if rest.starts_with('+') {
            // +name:value or +device.name:value — single snapshot only
            // Find the end of this param token (next whitespace or + or ^ or end)
            let end = rest[1..]
                .find(|c: char| c.is_whitespace())
                .map(|p| p + 1)
                .unwrap_or(rest.len());
            let token = &rest[..end];

            if let Some((body, val_str)) = token[1..].rsplit_once(':') {
                if body.contains(':') {
                    return Err(ParseError::new(
                        format!(
                            "param '{body}': sequences unsupported — snapshot only (+name:value or +device.name:value)"
                        ),
                        full_input.len() - rest.len(),
                        full_input.to_string(),
                    ));
                }
                let (device, name) = if let Some((d, n)) = body.rsplit_once('.') {
                    (Some(d.to_string()), n.to_string())
                } else {
                    (None, body.to_string())
                };
                match val_str.trim().parse::<f64>() {
                    Ok(value) => {
                        params.push(ParamSet {
                            device,
                            name,
                            value,
                        });
                        rest = rest[end..].trim_start();
                        continue;
                    }
                    Err(_) => {
                        return Err(ParseError::new(
                            format!("invalid param value for '{body}'"),
                            full_input.len() - rest.len(),
                            full_input.to_string(),
                        ));
                    }
                }
            } else {
                return Err(ParseError::new(
                    "param format: +name:value or +device.name:value",
                    full_input.len() - rest.len(),
                    full_input.to_string(),
                ));
            }
        } else if rest.starts_with("^^") {
            if let Some(num_str) = rest[2..].split_whitespace().next() {
                let n: i32 = num_str.parse().map_err(|_| {
                    ParseError::new(
                        "invalid scaleTranspose value",
                        full_input.len() - rest.len(),
                        full_input.to_string(),
                    )
                })?;
                scale_transpose = Some(n);
                rest = &rest[2 + num_str.len()..];
            } else {
                return Err(ParseError::new(
                    "^^ requires number",
                    full_input.len() - rest.len(),
                    full_input.to_string(),
                ));
            }
        } else if rest.starts_with('^') {
            if let Some(num_str) = rest[1..].split_whitespace().next() {
                let n: i32 = num_str.parse().map_err(|_| {
                    ParseError::new(
                        "invalid transpose value",
                        full_input.len() - rest.len(),
                        full_input.to_string(),
                    )
                })?;
                transpose = Some(n);
                rest = &rest[1 + num_str.len()..];
            } else {
                return Err(ParseError::new(
                    "^ requires number",
                    full_input.len() - rest.len(),
                    full_input.to_string(),
                ));
            }
        } else if rest.starts_with('!') && rest.len() <= 2 {
            // Lone trailing `!` (legacy launch marker) — reject_trailing tolerates it
            break;
        } else {
            break;
        }
        rest = rest.trim_start();
    }

    Ok((params, transpose, scale_transpose, rest))
}

/// Reject leftover input after pattern + params (`bass: n "c e" quatsch`).
/// A lone trailing `!` (launch marker) stays allowed.
fn reject_trailing(rest: &str, full_input: &str) -> ParseResult<()> {
    let trailing = rest.trim();
    if !trailing.is_empty() && trailing != "!" {
        return Err(ParseError::new(
            format!("unexpected trailing input: '{trailing}'"),
            full_input.len() - rest.trim_start().len(),
            full_input.to_string(),
        ));
    }
    Ok(())
}

// ── Note expression modifiers ──────────────────────────────────────

/// Parse consecutive `.vel(…)` / `.pres(…)` / `.tim(…)` / `.pan(…)` / `.gain(…)` / `.chnz(…)`
/// suffixes after a pattern. Returns the consumed modifiers and the remaining input.
pub(crate) fn parse_note_mods<'a>(
    input: &'a str,
    full_input: &str,
) -> ParseResult<(NoteMods, &'a str)> {
    let mut mods = NoteMods::default();
    let mut rest = input;

    loop {
        let trimmed = rest.trim_start();
        if !trimmed.starts_with('.') {
            break;
        }
        let after_dot = &trimmed[1..];
        let Some(paren_pos) = after_dot.find('(') else {
            break;
        };
        let name = &after_dot[..paren_pos];
        if !matches!(name, "vel" | "pres" | "tim" | "pan" | "gain" | "chnz") {
            break;
        }
        let (values, after_paren) = parse_paren_values(&after_dot[paren_pos + 1..], full_input)?;
        match name {
            "vel" => mods.vel = values,
            "pres" => mods.pressure = values,
            "tim" => mods.timbre = values,
            "pan" => mods.pan = values,
            "gain" => mods.gain = values,
            "chnz" => mods.chance = values,
            _ => unreachable!(),
        }
        rest = after_paren;
    }

    Ok((mods, rest))
}

/// Parse the contents of one parenthesized modifier: space-separated numbers or `~` for skip.
fn parse_paren_values<'a>(
    input: &'a str,
    full_input: &str,
) -> ParseResult<(Vec<Option<f64>>, &'a str)> {
    let s = input.trim_start();
    let mut depth = 1usize;
    let mut end = None;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| {
        ParseError::new(
            "expected ')' after modifier values",
            full_input.len().saturating_sub(s.len()),
            full_input.to_string(),
        )
    })?;

    let content = s[..end].trim();
    let after = &s[end + 1..];
    if content.is_empty() {
        return Ok((Vec::new(), after));
    }

    let mut values = Vec::new();
    for tok in content.split_whitespace() {
        if tok == "~" {
            values.push(None);
        } else {
            let v = tok.parse::<f64>().map_err(|_| {
                ParseError::new(
                    format!("invalid modifier value: '{tok}'"),
                    full_input.len().saturating_sub(s.len()),
                    full_input.to_string(),
                )
            })?;
            values.push(Some(v));
        }
    }
    Ok((values, after))
}

/// Parse `kick&v9kick: decay(50) pitch(40)` or `kick&v9kick: +decay(50) +pitch(40)`.
fn parse_amp_param_cmd(input: &str) -> ParseResult<MusicLine> {
    let s = input;
    let amp = s
        .find('&')
        .ok_or_else(|| ParseError::new("expected 'track&device:'", 0, s.to_string()))?;
    let track = s[..amp].trim().to_string();
    if track.is_empty() {
        return Err(ParseError::new("empty track before '&'", 0, s.to_string()));
    }
    let rest = &s[amp + 1..];
    let colon = rest.find(':').ok_or_else(|| {
        ParseError::new(
            "expected 'track&device: params' (colon after device)",
            amp + 1,
            s.to_string(),
        )
    })?;
    let device_name = rest[..colon].trim().to_string();
    if device_name.is_empty() {
        return Err(ParseError::new(
            "empty device after '&'",
            amp + 1,
            s.to_string(),
        ));
    }
    // Scene/slot uses `@` — reject accidental `track@scene&...` confusion later if needed.
    let content = rest[colon + 1..].trim();
    let device = DeviceSpec {
        catalog_name: device_name,
    };
    // Bare device ops (params always have parens, so no ambiguity):
    // `kick&Polymer: on|off|delete|move N`
    let op = match content {
        "on" => Some(DeviceOp::On),
        "off" => Some(DeviceOp::Off),
        "delete" => Some(DeviceOp::Delete),
        _ => content.strip_prefix("move").and_then(|m| {
            m.trim()
                .parse::<i32>()
                .ok()
                .filter(|_| !m.trim().is_empty())
                .map(DeviceOp::Move)
        }),
    };
    if let Some(op) = op {
        return Ok(MusicLine::DeviceOp(DeviceOpCmd { track, device, op }));
    }
    if content == "move" || content.starts_with("move ") {
        return Err(ParseError::new(
            "move needs a target index: track&device: move 0",
            amp + 1,
            s.to_string(),
        ));
    }
    let params = parse_param_assignments(content, s)?;
    Ok(MusicLine::Param(ParamCmd {
        track,
        device,
        params,
    }))
}

/// Parse `t(kick).d(kick.v9): decay(50) pitch(40)` (legacy).
fn parse_param_cmd(input: &str) -> ParseResult<MusicLine> {
    let s = input;
    let rest = s
        .strip_prefix("t(")
        .or_else(|| s.strip_prefix("track("))
        .ok_or_else(|| ParseError::new("expected 't(' or 'track('", 0, s.to_string()))?;

    let (track_name, rest) = parse_paren_arg(rest, s, 0)?;

    let rest = rest
        .strip_prefix(".d(")
        .or_else(|| rest.strip_prefix(".device("))
        .ok_or_else(|| {
            ParseError::new(
                "expected '.d(' or '.device('",
                s.len() - rest.len(),
                s.to_string(),
            )
        })?;

    let (dev, rest) = parse_device_in_paren(rest, s)?;

    let rest = rest
        .strip_prefix(":")
        .or_else(|| rest.strip_prefix(": "))
        .ok_or_else(|| {
            ParseError::new(
                "expected ': params...' after device",
                s.len() - rest.len(),
                s.to_string(),
            )
        })?;

    let params = parse_param_assignments(rest, s)?;
    Ok(MusicLine::Param(ParamCmd {
        track: track_name,
        device: dev,
        params,
    }))
}

/// `decay(50) pitch(40)` or `+decay(50) +pitch(40)` — space-separated, no dots between.
fn parse_param_assignments(rest: &str, full: &str) -> ParseResult<Vec<(String, f64)>> {
    let mut params = Vec::new();
    let mut remaining = rest.trim_start();
    while !remaining.is_empty() {
        if remaining.starts_with('+') {
            remaining = remaining[1..].trim_start();
        }
        if let Some((name, after)) = remaining.split_once('(') {
            let name = name.trim();
            if name.is_empty() || name.contains(char::is_whitespace) {
                return Err(ParseError::new(
                    "param name before '('",
                    full.len().saturating_sub(remaining.len()),
                    full.to_string(),
                ));
            }
            let (val_str, after_paren) = after.split_once(')').ok_or_else(|| {
                ParseError::new(
                    "expected ')' after param value",
                    full.len().saturating_sub(remaining.len()),
                    full.to_string(),
                )
            })?;
            let val: f64 = val_str.trim().parse().map_err(|_| {
                ParseError::new(
                    format!("invalid param value: '{val_str}'"),
                    full.len().saturating_sub(remaining.len()),
                    full.to_string(),
                )
            })?;
            params.push((name.to_string(), val));
            remaining = after_paren.trim_start();
        } else {
            break;
        }
    }
    if params.is_empty() {
        return Err(ParseError::new(
            "expected at least one param name(value)",
            full.len().saturating_sub(rest.len()),
            full.to_string(),
        ));
    }
    Ok(params)
}

/// Parse `mute(kick)` / `mute(kick) 4` / `mute(kick) @bar` / `mute(kick) 4 @bar`
fn parse_mute_cmd(rest: &str, unmute: bool, full_input: &str) -> ParseResult<MusicLine> {
    let rest = rest.trim();
    let closing = rest.find(')').ok_or_else(|| {
        ParseError::new(
            "expected ')' after mute/unmute args",
            full_input.len() - rest.len(),
            full_input.to_string(),
        )
    })?;
    let args_str = &rest[..closing];
    let mut after = rest[closing + 1..].trim();

    let refs: Vec<TrackRef> = args_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if let Ok(idx) = s.parse::<i32>() {
                TrackRef::Index(idx)
            } else {
                TrackRef::Name(s.to_string())
            }
        })
        .collect();

    if refs.is_empty() {
        return Err(ParseError::new(
            "mute/unmute requires at least one track name or index",
            0,
            full_input.to_string(),
        ));
    }

    let mut bars: Option<u32> = None;
    let mut quantize = MuteQuantize::Now;

    // Optional suffix tokens: bars number and/or @bar / @1 (any order)
    while !after.is_empty() {
        if after.starts_with('@') {
            let end = after[1..]
                .find(|c: char| c.is_whitespace())
                .map(|p| p + 1)
                .unwrap_or(after.len());
            let token = &after[..end];
            let q = token[1..].trim();
            match q {
                "bar" | "1" => quantize = MuteQuantize::Bar,
                other => {
                    return Err(ParseError::new(
                        format!("unknown mute quantize '@{other}' (use @bar)"),
                        full_input.len() - after.len(),
                        full_input.to_string(),
                    ));
                }
            }
            after = after[end..].trim_start();
            continue;
        }

        // number of bars
        let end = after
            .find(|c: char| c.is_whitespace() || c == '@')
            .unwrap_or(after.len());
        let token = &after[..end];
        if token.is_empty() {
            break;
        }
        match token.parse::<u32>() {
            Ok(n) if n >= 1 => {
                if bars.is_some() {
                    return Err(ParseError::new(
                        "mute bars specified twice",
                        full_input.len() - after.len(),
                        full_input.to_string(),
                    ));
                }
                bars = Some(n);
                after = after[end..].trim_start();
            }
            _ => {
                return Err(ParseError::new(
                    format!("unexpected mute suffix '{token}' (want N bars and/or @bar)"),
                    full_input.len() - after.len(),
                    full_input.to_string(),
                ));
            }
        }
    }

    let cmd = MuteCmd {
        refs,
        bars,
        quantize,
    };
    if unmute {
        Ok(MusicLine::Unmute(cmd))
    } else {
        Ok(MusicLine::Mute(cmd))
    }
}

/// Parse `new scene` | `new scene()` | `new scene(verse)`.
fn parse_new_scene(input: &str) -> ParseResult<MusicLine> {
    let rest = input
        .strip_prefix("new scene")
        .or_else(|| input.strip_prefix("new s"))
        .ok_or_else(|| ParseError::new("expected 'new scene' or 'new s'", 0, input.to_string()))?
        .trim_start();
    if rest.is_empty() {
        return Ok(MusicLine::NewScene { name: None });
    }
    if !rest.starts_with('(') {
        return Err(ParseError::new(
            "use new scene / new scene() / new scene(name)",
            input.len() - rest.len(),
            input.to_string(),
        ));
    }
    let after = &rest[1..];
    let closing = after.find(')').ok_or_else(|| {
        ParseError::new(
            "expected ')' after new scene",
            input.len() - after.len(),
            input.to_string(),
        )
    })?;
    let name = after[..closing].trim();
    let tail = after[closing + 1..].trim();
    if !tail.is_empty() {
        return Err(ParseError::new(
            format!("unexpected after new scene: '{tail}'"),
            input.len() - after.len() + closing,
            input.to_string(),
        ));
    }
    Ok(MusicLine::NewScene {
        name: if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        },
    })
}

/// Parse `s(1).start` | `s(verse).t(lead).c(new)` | `scene(0).stop`
fn parse_scene_cmd(input: &str) -> ParseResult<MusicLine> {
    let after = if let Some(a) = input.strip_prefix("scene(") {
        a
    } else if let Some(a) = input.strip_prefix("s(") {
        a
    } else {
        return Err(ParseError::new(
            "expected s(…) or scene(…)",
            0,
            input.to_string(),
        ));
    };
    let closing = after.find(')').ok_or_else(|| {
        ParseError::new(
            "expected ')' after scene ref",
            input.len() - after.len(),
            input.to_string(),
        )
    })?;
    let scene_str = after[..closing].trim();
    if scene_str.is_empty() {
        return Err(ParseError::new(
            "empty scene ref — use s(0) or s(verse)",
            input.len() - after.len(),
            input.to_string(),
        ));
    }
    let scene = if let Ok(i) = scene_str.parse::<i32>() {
        SceneRef::Index(i)
    } else {
        SceneRef::Name(scene_str.to_string())
    };

    let rest = after[closing + 1..].trim_start();
    // Bitwig cell: s(scene).t(track).c(new|start|stop)
    if rest.starts_with(".t(") || rest.starts_with(".track(") {
        return parse_scene_track_clip(scene, rest, input);
    }
    let action = parse_launch_action(rest, input)?;
    Ok(MusicLine::Scene(SceneCmd { scene, action }))
}

/// `.t(lead).c(new)` | `.t(lead).c(new, intro)` | `.t(lead).c(start)`
fn parse_scene_track_clip(scene: SceneRef, rest: &str, full: &str) -> ParseResult<MusicLine> {
    let after_t = rest
        .strip_prefix(".t(")
        .or_else(|| rest.strip_prefix(".track("))
        .ok_or_else(|| {
            ParseError::new(
                "expected .t(track) after scene",
                full.len() - rest.len(),
                full.to_string(),
            )
        })?;
    let t_close = after_t.find(')').ok_or_else(|| {
        ParseError::new(
            "expected ')' after track name",
            full.len() - after_t.len(),
            full.to_string(),
        )
    })?;
    let track = after_t[..t_close].trim().to_string();
    if track.is_empty() {
        return Err(ParseError::new(
            "empty track in s(…).t(…)",
            full.len() - after_t.len(),
            full.to_string(),
        ));
    }
    let rest = after_t[t_close + 1..].trim_start();
    let after_c = rest
        .strip_prefix(".c(")
        .or_else(|| rest.strip_prefix(".clip("))
        .ok_or_else(|| {
            ParseError::new(
                "expected .c(new) or .c(start) after s(…).t(track)",
                full.len() - rest.len(),
                full.to_string(),
            )
        })?;
    let c_close = after_c.find(')').ok_or_else(|| {
        ParseError::new(
            "expected ')' after clip action",
            full.len() - after_c.len(),
            full.to_string(),
        )
    })?;
    let content = after_c[..c_close].trim();
    let tail = after_c[c_close + 1..].trim();
    if !tail.is_empty() {
        return Err(ParseError::new(
            format!("unexpected after clip action: '{tail}'"),
            full.len() - after_c.len() + c_close,
            full.to_string(),
        ));
    }
    let action = parse_scene_clip_action(content, full)?;
    Ok(MusicLine::SceneTrackClip(SceneTrackClipCmd {
        scene,
        track,
        action,
    }))
}

fn parse_scene_clip_action(content: &str, full: &str) -> ParseResult<SceneClipAction> {
    let c = content.trim();
    if c == "start" || c == "launch" {
        return Ok(SceneClipAction::Start);
    }
    if c == "stop" {
        return Ok(SceneClipAction::Stop);
    }
    // new | new, name | new:name
    if c == "new" {
        return Ok(SceneClipAction::New { name: None });
    }
    if let Some(rest) = c.strip_prefix("new,") {
        let name = rest.trim();
        if name.is_empty() {
            return Ok(SceneClipAction::New { name: None });
        }
        return Ok(SceneClipAction::New {
            name: Some(name.to_string()),
        });
    }
    if let Some(rest) = c.strip_prefix("new:") {
        let name = rest.trim();
        if name.is_empty() {
            return Err(ParseError::new(
                "c(new:name) needs a name",
                0,
                full.to_string(),
            ));
        }
        return Ok(SceneClipAction::New {
            name: Some(name.to_string()),
        });
    }
    Err(ParseError::new(
        format!("unknown s().t().c(…) action '{c}' — use new | new, name | start | stop"),
        0,
        full.to_string(),
    ))
}

/// Parse `c(bass.0).start` | `clip(bass.0).start` | multi-ref variants.
fn parse_clip_ctrl_cmd(input: &str) -> ParseResult<MusicLine> {
    // Prefer long form first so `clip(` is not mis-read (it does not start with `c(`).
    let after = input
        .strip_prefix("clip(")
        .or_else(|| input.strip_prefix("c("))
        .ok_or_else(|| ParseError::new("expected c(…) or clip(…)", 0, input.to_string()))?;
    let closing = after.find(')').ok_or_else(|| {
        ParseError::new(
            "expected ')' after clip ref(s)",
            input.len() - after.len(),
            input.to_string(),
        )
    })?;
    let refs_str = &after[..closing];

    let refs: Result<Vec<ClipCtrlRef>, _> = refs_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (track, slot_str) = s.split_once('.').ok_or_else(|| {
                ParseError::new(
                    "clip ref format: track.slot",
                    input.len() - after.len(),
                    input.to_string(),
                )
            })?;
            let slot: i32 = slot_str.trim().parse().map_err(|_| {
                ParseError::new(
                    format!("invalid slot: '{slot_str}'"),
                    input.len() - after.len(),
                    input.to_string(),
                )
            })?;
            Ok(ClipCtrlRef {
                track: track.trim().to_string(),
                slot,
            })
        })
        .collect();
    let refs = refs?;

    if refs.is_empty() {
        return Err(ParseError::new(
            "clip ctrl needs at least one ref, e.g. c(bass.0) or clip(bass.0)",
            input.len() - after.len(),
            input.to_string(),
        ));
    }

    let rest = &after[closing + 1..];
    let action = parse_launch_action(rest, input)?;
    Ok(MusicLine::ClipCtrl(ClipCtrlCmd { refs, action }))
}

pub(crate) fn parse_launch_action(rest: &str, full_input: &str) -> ParseResult<LaunchAction> {
    let rest = rest.trim_start();
    if rest == ".start" || rest == ".launch" {
        Ok(LaunchAction::Start)
    } else if rest == ".stop" {
        Ok(LaunchAction::Stop)
    } else if rest == ".delete()" || rest == ".delete" {
        Ok(LaunchAction::Delete)
    } else if let Some(after) = rest.strip_prefix(".rename(") {
        let closing = after.find(')').ok_or_else(|| {
            ParseError::new(
                "expected ')' after rename name",
                full_input.len() - after.len(),
                full_input.to_string(),
            )
        })?;
        let name = after[..closing].trim();
        if name.is_empty() {
            return Err(ParseError::new(
                "rename needs a name: .rename(name)",
                full_input.len() - after.len(),
                full_input.to_string(),
            ));
        }
        if after[closing + 1..].trim().is_empty() {
            Ok(LaunchAction::Rename(name.to_string()))
        } else {
            Err(ParseError::new(
                format!("unexpected after .rename(…): '{}'", &after[closing + 1..]),
                full_input.len() - after.len() + closing,
                full_input.to_string(),
            ))
        }
    } else {
        Err(ParseError::new(
            format!(
                "expected .start/.stop/.rename(name)/.delete() after ref, got '{}'",
                rest.chars().take(10).collect::<String>()
            ),
            full_input.len() - rest.len(),
            full_input.to_string(),
        ))
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_note_pattern() {
        let line = r#"bass: n "c e g""#;
        let result = parse_music_line(line).unwrap();
        assert!(matches!(result, MusicLine::Music(_)));
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.target.track, "bass");
            assert!(matches!(cmd.action, MusicAction::Notes));
            assert_eq!(cmd.pattern, "c e g");
        }
    }

    #[test]
    fn test_parse_with_params() {
        let line = r#"bass: n "c e g" +cutoff:0.3 +res:0.7"#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.params.len(), 2);
            assert_eq!(cmd.params[0].device, None);
            assert_eq!(cmd.params[0].name, "cutoff");
            assert_eq!(cmd.params[0].value, 0.3);
            assert_eq!(cmd.params[1].device, None);
            assert_eq!(cmd.params[1].name, "res");
            assert_eq!(cmd.params[1].value, 0.7);
        } else {
            panic!("expected Music");
        }
    }

    #[test]
    fn test_parse_with_device_prefixed_params() {
        let line = r#"bass: n "c e g" +Polymer.cutoff:0.3 +kick.v9.decay:0.5"#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.params.len(), 2);
            assert_eq!(cmd.params[0].device.as_deref(), Some("Polymer"));
            assert_eq!(cmd.params[0].name, "cutoff");
            assert_eq!(cmd.params[0].value, 0.3);
            assert_eq!(cmd.params[1].device.as_deref(), Some("kick.v9"));
            assert_eq!(cmd.params[1].name, "decay");
            assert_eq!(cmd.params[1].value, 0.5);
        } else {
            panic!("expected Music");
        }
    }

    #[test]
    fn test_parse_d_removed() {
        assert!(parse_music_line(r#"drums: d "bd hh""#).is_err());
        assert!(parse_music_line(r#"kick: d:808 "bd""#).is_err());
    }

    #[test]
    fn test_parse_chain() {
        let line = "!bass Polymer Filter Delay-2";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Chain(cmd) = result {
            assert_eq!(cmd.name, "bass");
            assert_eq!(cmd.devices, vec!["Polymer", "Filter", "Delay-2"]);
        } else {
            panic!("expected Chain");
        }
    }

    #[test]
    fn test_parse_chain_with_kit() {
        let line = "!drums:808";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Chain(cmd) = result {
            assert_eq!(cmd.name, "drums");
            assert_eq!(cmd.drum_kit, Some("808".to_string()));
        } else {
            panic!("expected Chain");
        }
    }

    #[test]
    fn test_parse_key() {
        let line = "k C minor";
        let result = parse_music_line(line).unwrap();
        assert!(matches!(result, MusicLine::Key { .. }));
        if let MusicLine::Key { root, scale } = result {
            assert_eq!(root, "C");
            assert_eq!(scale, "minor");
        }
    }

    #[test]
    fn test_parse_transport() {
        assert!(matches!(
            parse_music_line("play").unwrap(),
            MusicLine::Transport(TransportCmd::Play)
        ));
        assert!(matches!(
            parse_music_line("stop").unwrap(),
            MusicLine::Transport(TransportCmd::Stop)
        ));
    }

    #[test]
    fn test_parse_passthrough() {
        let line = "> track list";
        let result = parse_music_line(line).unwrap();
        assert!(matches!(result, MusicLine::PassThrough(_)));
        if let MusicLine::PassThrough(cmd) = result {
            assert_eq!(cmd, "track list");
        }
    }

    #[test]
    fn test_parse_clip_ref() {
        let line = r#"bass@verse: n "c e g""#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.target.track, "bass");
            assert_eq!(cmd.target.clip, Some(ClipRef::Name("verse".to_string())));
        } else {
            panic!("expected Music");
        }

        let slot = parse_music_line(r#"lead@2: n "c""#).unwrap();
        if let MusicLine::Music(cmd) = slot {
            assert_eq!(cmd.target.clip, Some(ClipRef::Slot(2)));
        } else {
            panic!("expected Music @slot");
        }
    }

    #[test]
    fn test_parse_new_scene() {
        assert!(matches!(
            parse_music_line("new scene").unwrap(),
            MusicLine::NewScene { name: None }
        ));
        assert!(matches!(
            parse_music_line("new scene()").unwrap(),
            MusicLine::NewScene { name: None }
        ));
        match parse_music_line("new scene(verse)").unwrap() {
            MusicLine::NewScene { name: Some(n) } => assert_eq!(n, "verse"),
            _ => panic!("expected NewScene(verse)"),
        }
    }

    #[test]
    fn test_parse_scene_track_clip_new() {
        let line = parse_music_line("s(verse).t(lead).c(new)").unwrap();
        match line {
            MusicLine::SceneTrackClip(cmd) => {
                assert_eq!(cmd.scene, SceneRef::Name("verse".to_string()));
                assert_eq!(cmd.track, "lead");
                assert_eq!(cmd.action, SceneClipAction::New { name: None });
            }
            _ => panic!("expected SceneTrackClip"),
        }
        let named = parse_music_line("s(1).t(bass).c(new, intro)").unwrap();
        match named {
            MusicLine::SceneTrackClip(cmd) => {
                assert_eq!(cmd.scene, SceneRef::Index(1));
                assert_eq!(
                    cmd.action,
                    SceneClipAction::New {
                        name: Some("intro".to_string())
                    }
                );
            }
            _ => panic!("expected SceneTrackClip named"),
        }
    }

    /// Long form (beginner) and short form (live) must parse to the same AST.
    #[test]
    fn long_short_alias_parity() {
        let pairs = [
            (
                r#"new track(lead).device(Polymer).add(Delay+)"#,
                r#"new t(lead).d(Polymer).add(Delay+)"#,
            ),
            (r#"track(lead).device(Chorus+)"#, r#"t(lead).d(Chorus+)"#),
            (
                r#"track(bass).notes("c e g").clip(start)"#,
                r#"t(bass).n("c e g").c(start)"#,
            ),
            (r#"clip(bass.0).start"#, r#"c(bass.0).start"#),
            (r#"clip(bass.0).stop"#, r#"c(bass.0).stop"#),
            (
                r#"scene(verse).track(lead).clip(new)"#,
                r#"s(verse).t(lead).c(new)"#,
            ),
            (
                r#"scene(verse).track(lead).clip(start)"#,
                r#"s(verse).t(lead).c(start)"#,
            ),
            (
                r#"track(kick).device(v9kick): decay(50)"#,
                r#"t(kick).d(v9kick): decay(50)"#,
            ),
        ];
        for (long, short) in pairs {
            let a = parse_music_line(long).unwrap_or_else(|e| panic!("long failed: {long}\n{e}"));
            let b =
                parse_music_line(short).unwrap_or_else(|e| panic!("short failed: {short}\n{e}"));
            assert_eq!(a, b, "alias parity:\n  long  {long}\n  short {short}");
        }
    }

    #[test]
    fn test_parse_arp() {
        let line = r#"bass: arp "c e g""#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.action, MusicAction::Arp(ArpStyle::Up));
            assert_eq!(cmd.pattern, "c e g");
        } else {
            panic!("expected Music arp");
        }

        let down = parse_music_line(r#"lead: arp:down "Cm7""#).unwrap();
        if let MusicLine::Music(cmd) = down {
            assert_eq!(cmd.action, MusicAction::Arp(ArpStyle::Down));
            assert_eq!(cmd.pattern, "Cm7");
        } else {
            panic!("expected arp:down");
        }
    }
    #[test]
    fn test_parse_param_cmd_legacy() {
        let line = "t(kick).d(kick.v9): decay(50) pitch(40)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Param(cmd) = result {
            assert_eq!(cmd.track, "kick");
            assert_eq!(cmd.device.catalog_name, "kick.v9");
            assert_eq!(cmd.params.len(), 2);
            assert_eq!(cmd.params[0], ("decay".to_string(), 50.0));
            assert_eq!(cmd.params[1], ("pitch".to_string(), 40.0));
        } else {
            panic!("expected Param, got {:?}", result);
        }
    }

    #[test]
    fn test_parse_amp_param_cmd() {
        let line = "kick&v9kick: decay(50) pitch(40)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Param(cmd) = result {
            assert_eq!(cmd.track, "kick");
            assert_eq!(cmd.device.catalog_name, "v9kick");
            assert_eq!(cmd.params[0], ("decay".to_string(), 50.0));
            assert_eq!(cmd.params[1], ("pitch".to_string(), 40.0));
        } else {
            panic!("expected Param, got {:?}", result);
        }
    }

    #[test]
    fn test_parse_amp_param_plus() {
        let line = r#"lead&v9 kick: +decay(75)"#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Param(cmd) = result {
            assert_eq!(cmd.track, "lead");
            assert_eq!(cmd.device.catalog_name, "v9 kick");
            assert_eq!(cmd.params[0], ("decay".to_string(), 75.0));
        } else {
            panic!("expected Param");
        }
    }

    #[test]
    fn test_parse_mute_name() {
        let line = "mute(kick)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Mute(cmd) = result {
            assert_eq!(cmd.refs.len(), 1);
            assert_eq!(cmd.refs[0], TrackRef::Name("kick".to_string()));
            assert_eq!(cmd.bars, None);
            assert_eq!(cmd.quantize, MuteQuantize::Now);
        } else {
            panic!("expected Mute");
        }
    }

    #[test]
    fn test_parse_mute_multi() {
        let line = "mute(kick, bass)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Mute(cmd) = result {
            assert_eq!(cmd.refs.len(), 2);
        } else {
            panic!("expected Mute");
        }
    }

    #[test]
    fn test_parse_mute_index() {
        let line = "mute(1,3,5)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Mute(cmd) = result {
            assert_eq!(cmd.refs.len(), 3);
            assert_eq!(cmd.refs[0], TrackRef::Index(1));
            assert_eq!(cmd.refs[2], TrackRef::Index(5));
        } else {
            panic!("expected Mute");
        }
    }

    #[test]
    fn test_parse_unmute() {
        let line = "unmute(kick)";
        let result = parse_music_line(line).unwrap();
        assert!(matches!(result, MusicLine::Unmute(_)));
    }

    #[test]
    fn test_parse_mute_bars() {
        let line = "mute(kick) 4";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Mute(cmd) = result {
            assert_eq!(cmd.bars, Some(4));
            assert_eq!(cmd.quantize, MuteQuantize::Now);
        } else {
            panic!("expected Mute");
        }
    }

    #[test]
    fn test_parse_mute_quantize_bar() {
        let line = "mute(kick) @bar";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Mute(cmd) = result {
            assert_eq!(cmd.bars, None);
            assert_eq!(cmd.quantize, MuteQuantize::Bar);
        } else {
            panic!("expected Mute");
        }
    }

    #[test]
    fn test_parse_mute_bars_and_bar() {
        let line = "mute(kick, bass) 8 @bar";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Mute(cmd) = result {
            assert_eq!(cmd.refs.len(), 2);
            assert_eq!(cmd.bars, Some(8));
            assert_eq!(cmd.quantize, MuteQuantize::Bar);
        } else {
            panic!("expected Mute");
        }
    }

    #[test]
    fn test_parse_mute_bar_then_bars() {
        let line = "unmute(lead) @bar 2";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Unmute(cmd) = result {
            assert_eq!(cmd.bars, Some(2));
            assert_eq!(cmd.quantize, MuteQuantize::Bar);
        } else {
            panic!("expected Unmute");
        }
    }

    #[test]
    fn test_parse_scene_start() {
        let line = "s(1).start";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Scene(cmd) = result {
            assert_eq!(cmd.scene, SceneRef::Index(1));
            assert!(matches!(cmd.action, LaunchAction::Start));
        } else {
            panic!("expected Scene");
        }
    }

    #[test]
    fn test_parse_scene_stop() {
        let line = "s(0).stop";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Scene(cmd) = result {
            assert_eq!(cmd.scene, SceneRef::Index(0));
            assert!(matches!(cmd.action, LaunchAction::Stop));
        } else {
            panic!("expected Scene");
        }

        let named = parse_music_line("scene(verse).start").unwrap();
        if let MusicLine::Scene(cmd) = named {
            assert_eq!(cmd.scene, SceneRef::Name("verse".to_string()));
            assert!(matches!(cmd.action, LaunchAction::Start));
        } else {
            panic!("expected Scene by name");
        }

        let alias = parse_music_line("s(Drop).stop").unwrap();
        if let MusicLine::Scene(cmd) = alias {
            assert_eq!(cmd.scene, SceneRef::Name("Drop".to_string()));
        } else {
            panic!("expected Scene");
        }
    }

    #[test]
    fn test_parse_clip_ctrl() {
        let line = "c(bass.0).start";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::ClipCtrl(cmd) = result {
            assert_eq!(cmd.refs.len(), 1);
            assert_eq!(cmd.refs[0].track, "bass");
            assert_eq!(cmd.refs[0].slot, 0);
            assert!(matches!(cmd.action, LaunchAction::Start));
        } else {
            panic!("expected ClipCtrl, got {:?}", result);
        }
    }

    #[test]
    fn test_parse_clip_ctrl_multi() {
        let line = "c(bass.0, kick.1).start";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::ClipCtrl(cmd) = result {
            assert_eq!(cmd.refs.len(), 2);
            assert_eq!(cmd.refs[0].track, "bass");
            assert_eq!(cmd.refs[1].track, "kick");
            assert_eq!(cmd.refs[1].slot, 1);
        } else {
            panic!("expected ClipCtrl");
        }
    }

    #[test]
    fn test_parse_clip_ctrl_stop() {
        let line = "c(lead.3).stop";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::ClipCtrl(cmd) = result {
            assert_eq!(cmd.refs[0].track, "lead");
            assert_eq!(cmd.refs[0].slot, 3);
            assert!(matches!(cmd.action, LaunchAction::Stop));
        } else {
            panic!("expected ClipCtrl");
        }
    }

    #[test]
    fn test_parse_scene_rename_delete() {
        let result = parse_music_line("s(verse).rename(drop)").unwrap();
        if let MusicLine::Scene(cmd) = result {
            assert_eq!(cmd.scene, SceneRef::Name("verse".to_string()));
            assert_eq!(cmd.action, LaunchAction::Rename("drop".to_string()));
        } else {
            panic!("expected Scene, got {:?}", result);
        }

        let result = parse_music_line("s(0).delete()").unwrap();
        if let MusicLine::Scene(cmd) = result {
            assert_eq!(cmd.scene, SceneRef::Index(0));
            assert_eq!(cmd.action, LaunchAction::Delete);
        } else {
            panic!("expected Scene");
        }
    }

    #[test]
    fn test_parse_clip_rename_delete() {
        let result = parse_music_line("c(bass.0).rename(intro)").unwrap();
        if let MusicLine::ClipCtrl(cmd) = result {
            assert_eq!(cmd.refs[0].track, "bass");
            assert_eq!(cmd.action, LaunchAction::Rename("intro".to_string()));
        } else {
            panic!("expected ClipCtrl, got {:?}", result);
        }

        let result = parse_music_line("c(bass.1).delete()").unwrap();
        if let MusicLine::ClipCtrl(cmd) = result {
            assert_eq!(cmd.refs[0].slot, 1);
            assert_eq!(cmd.action, LaunchAction::Delete);
        } else {
            panic!("expected ClipCtrl");
        }
    }

    #[test]
    fn test_parse_device_ops() {
        let result = parse_music_line("kick&Polymer: off").unwrap();
        assert_eq!(
            result,
            MusicLine::DeviceOp(DeviceOpCmd {
                track: "kick".to_string(),
                device: DeviceSpec {
                    catalog_name: "Polymer".to_string()
                },
                op: DeviceOp::Off,
            })
        );

        for (line, op) in [
            ("kick&Polymer: on", DeviceOp::On),
            ("kick&Polymer: delete", DeviceOp::Delete),
            ("bass&Delay+: move 0", DeviceOp::Move(0)),
            ("kick&1: delete", DeviceOp::Delete),
        ] {
            let result = parse_music_line(line).unwrap();
            match result {
                MusicLine::DeviceOp(cmd) => assert_eq!(cmd.op, op, "line: {line}"),
                other => panic!("expected DeviceOp for '{line}', got {other:?}"),
            }
        }

        // move without index → error, params unchanged
        assert!(parse_music_line("kick&Polymer: move").is_err());
        assert!(matches!(
            parse_music_line("kick&v9kick: decay(50)").unwrap(),
            MusicLine::Param(_)
        ));
    }

    #[test]
    fn test_parse_note_mods_colon() {
        let line = r#"bass: n "c e g".vel(80 60 100).pan(-50 0 50)"#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.note_mods.vel, vec![Some(80.0), Some(60.0), Some(100.0)]);
            assert_eq!(cmd.note_mods.pan, vec![Some(-50.0), Some(0.0), Some(50.0)]);
        } else {
            panic!("expected Music");
        }
    }

    #[test]
    fn test_long_short_forms_same_ast() {
        // Long ≡ short must hold for every command layer (AGENTS.md):
        // track≡t, device≡d, clip≡c, scene≡s, notes≡n.
        let pairs: &[(&str, &str)] = &[
            ("track(kick).device(Polymer)", "t(kick).d(Polymer)"),
            ("track(kick).device(Polymer)", "t(kick).device(Polymer)"),
            ("new track(kick).device(P)", "new t(kick).d(P)"),
            ("track(kick).notes(\"c e g\")", "t(kick).n(\"c e g\")"),
            ("track(kick).clip(start)", "t(kick).c(start)"),
            ("track(kick).clip(0).start", "t(kick).c(0).start"),
            ("track(kick).rename(drums)", "t(kick).rename(drums)"),
            ("track(kick).delete()", "t(kick).delete()"),
            ("scene(1).start", "s(1).start"),
            ("scene(1).track(lead).clip(new)", "s(1).t(lead).c(new)"),
            ("scene(0).rename(drop)", "s(0).rename(drop)"),
            ("scene(0).delete()", "s(0).delete()"),
            ("clip(bass.0).start", "c(bass.0).start"),
            ("clip(bass.0).rename(intro)", "c(bass.0).rename(intro)"),
            ("clip(bass.0).delete()", "c(bass.0).delete()"),
            (
                "track(kick).device(P): decay(50)",
                "t(kick).d(P): decay(50)",
            ),
            ("new scene(verse)", "new s(verse)"),
        ];
        for (long, short) in pairs {
            let l = parse_music_line(long).unwrap_or_else(|e| panic!("'{long}' failed: {e}"));
            let s = parse_music_line(short).unwrap_or_else(|e| panic!("'{short}' failed: {e}"));
            assert_eq!(l, s, "long '{long}' != short '{short}'");
        }
    }

    #[test]
    fn test_parse_note_mods_skip() {
        let line = r#"bass: n "c e g".vel(80 ~ 100)"#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Music(cmd) = result {
            assert_eq!(cmd.note_mods.vel, vec![Some(80.0), None, Some(100.0)]);
        } else {
            panic!("expected Music");
        }
    }
    #[test]
    fn test_trailing_garbage_rejected() {
        let err = parse_music_line(r#"bass: n "c e" quatsch"#).unwrap_err();
        assert!(err.to_string().contains("trailing"), "{err}");
        let err = parse_music_line(r#"n "c e" xyz"#).unwrap_err();
        assert!(err.to_string().contains("trailing"), "{err}");
        // legit lines keep parsing
        assert!(parse_music_line(r#"bass: n "c e""#).is_ok());
        assert!(parse_music_line(r#"bass: n "c e" ^2"#).is_ok());
        assert!(parse_music_line(r#"bass: n "c e" +vol:0.5"#).is_ok());
        assert!(parse_music_line(r#"bass: n "c e" !"#).is_ok()); // launch marker
    }
}
