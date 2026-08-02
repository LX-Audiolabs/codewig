//! Fluent chain parser: `new track(x).device(Y).beat(4_).n("…").mute().clip(start)`.

use super::super::ast::*;
use super::line::{extract_quoted, parse_launch_action, parse_note_mods};
use super::{ParseError, ParseResult};

/// Parse `new track(kick).device(v9 kick).beat(4_).mute().clip(start)`
pub(crate) fn parse_fluent(input: &str) -> ParseResult<MusicLine> {
    let s = input;
    let create = s.starts_with("new ");

    // Extract track name from `new track(name)` or `t(name)`
    let rest = if create {
        s.strip_prefix("new track(").or_else(|| s.strip_prefix("new t("))
    } else {
        s.strip_prefix("t(").or_else(|| s.strip_prefix("track("))
    }.ok_or_else(|| ParseError::new("expected 'new track(' or 't('", 0, s.to_string()))?;

    let (track_name, mut rest) = parse_paren_arg(rest, s, 0)?;

    let mut steps = Vec::new();

    while !rest.is_empty() && rest.starts_with('.') {
        rest = &rest[1..]; // skip dot

        if rest.starts_with("device(") || rest.starts_with("d(") {
            let (dev, r) = parse_device_step(rest, s)?;
            steps.push(FluentStep::Device(dev));
            rest = r;
        } else if rest.starts_with("add(") {
            let after = rest.strip_prefix("add(").unwrap();
            let (dev, r) = parse_device_in_paren(after, s)?;
            steps.push(FluentStep::Add(dev));
            rest = r;
        } else if rest.starts_with("beat") {
            let (beat, r) = parse_beat_step(rest, s)?;
            steps.push(FluentStep::Beat(beat));
            rest = r;
        } else if rest.starts_with("n(") || rest.starts_with("notes(") {
            let (pattern, r) = extract_paren_quoted(rest, s)?;
            let (mods, r2) = parse_note_mods(r, s)?;
            steps.push(FluentStep::Pattern { pattern, mods });
            rest = r2;
        } else if rest.starts_with("mute()") {
            steps.push(FluentStep::Mute);
            rest = &rest[6..];
        } else if rest.starts_with("clip(") || rest.starts_with("c(") {
            let after = rest.find('(').map(|i| &rest[i+1..]).unwrap_or("");
            let closing = after.find(')').ok_or_else(||
                ParseError::new("expected ')' after clip/c", s.len() - rest.len(), s.to_string()))?;
            let content = after[..closing].trim();
            // Check if content is a slot number or "start"/"stop"
            if content == "start" || content == "stop" || content == "launch" {
                // .clip(start) → ClipAction
                let action = match content {
                    "start" | "launch" => ClipAction::Start,
                    "stop" => ClipAction::Stop,
                    _ => unreachable!(),
                };
                steps.push(FluentStep::ClipAction(action));
                rest = &after[closing + 1..];
            } else {
                // .c(0) or .c(0,1) → ClipCtrl with slot refs
                let refs: Result<Vec<ClipCtrlRef>, _> = content.split(',')
                    .map(|r| r.trim())
                    .filter(|r| !r.is_empty())
                    .map(|r| {
                        let slot: i32 = r.parse().map_err(|_|
                            ParseError::new(format!("invalid slot: '{r}'"), s.len() - rest.len(), s.to_string()))?;
                        Ok(ClipCtrlRef { track: String::new(), slot })
                    })
                    .collect();
                let refs = refs?;
                let after_paren = &after[closing + 1..];
                let action = parse_launch_action(after_paren, s)?;
                steps.push(FluentStep::ClipCtrl(ClipCtrlCmd { refs, action }));
                // Advance past the .start/.stop text
                let action_len = after_paren.chars().take_while(|c| !c.is_whitespace()).count();
                rest = &after_paren[action_len..];
            }
        } else {
            return Err(ParseError::new(
                format!("unknown fluent step: '.{}'", rest.chars().take(10).collect::<String>()),
                s.len() - rest.len(), s.to_string()));
        }
    }

    Ok(MusicLine::Fluent(FluentCmd { create, track: track_name, steps }))
}

pub(crate) fn parse_paren_arg<'a>(input: &'a str, full_input: &str, _offset: usize) -> ParseResult<(String, &'a str)> {
    let closing = input.find(')').ok_or_else(||
        ParseError::new("expected ')' after argument", full_input.len() - input.len(), full_input.to_string()))?;
    let name = input[..closing].trim().to_string();
    Ok((name, &input[closing + 1..]))
}

fn parse_device_step<'a>(input: &'a str, full_input: &str) -> ParseResult<(DeviceSpec, &'a str)> {
    let after = input.strip_prefix("device(").or_else(|| input.strip_prefix("d("))
        .unwrap();
    parse_device_in_paren(after, full_input)
}

pub(crate) fn parse_device_in_paren<'a>(after: &'a str, full_input: &str) -> ParseResult<(DeviceSpec, &'a str)> {
    let closing = after.find(')').ok_or_else(||
        ParseError::new("expected ')' after device name", full_input.len() - after.len(), full_input.to_string()))?;
    let name = after[..closing].trim().to_string();
    Ok((DeviceSpec { catalog_name: name }, &after[closing + 1..]))
}

fn parse_beat_step<'a>(input: &'a str, full_input: &str) -> ParseResult<(BeatSpec, &'a str)> {
    let rest = input.strip_prefix("beat").unwrap();

    // beat:16(1,5,11,14)  — explicit
    if let Some(after_colon) = rest.strip_prefix(':') {
        let paren = after_colon.find('(').ok_or_else(||
            ParseError::new("expected '(positions)' after beat:N", full_input.len() - input.len(), full_input.to_string()))?;
        let grid_str = &after_colon[..paren];
        let grid: u32 = grid_str.parse().map_err(|_|
            ParseError::new(format!("invalid grid size: '{grid_str}'"), full_input.len() - input.len(), full_input.to_string()))?;

        let after_paren = &after_colon[paren + 1..];
        let closing = after_paren.find(')').ok_or_else(||
            ParseError::new("expected ')' after beat positions", full_input.len() - input.len(), full_input.to_string()))?;
        let pos_str = &after_paren[..closing];
        let positions: Result<Vec<u32>, _> = pos_str.split(',').map(|s| s.trim().parse::<u32>()).collect();
        let mut positions = positions.map_err(|_|
            ParseError::new(format!("invalid beat positions: '{pos_str}'"), full_input.len() - input.len(), full_input.to_string()))?;
        // Accept 1-based counts (1..grid) like musicians type; store 0-based for Bitwig.
        if !positions.is_empty()
            && positions.iter().all(|&p| p >= 1 && p <= grid)
            && positions.iter().any(|&p| p == grid || p > 0)
        {
            // If any 0 present, leave as 0-based; only convert pure 1..=grid sets
            if !positions.contains(&0) {
                for p in &mut positions {
                    *p -= 1;
                }
            }
        }

        Ok((BeatSpec::Explicit { grid, positions }, &after_paren[closing + 1..]))
    } else if rest.starts_with('(') {
        // beat(4_) or beat(off)
        let closing = rest.find(')').ok_or_else(||
            ParseError::new("expected ')' after beat shorthand", full_input.len() - input.len(), full_input.to_string()))?;
        let shorthand = &rest[1..closing].trim();
        let spec = parse_beat_shorthand(shorthand, full_input, input)?;
        Ok((spec, &rest[closing + 1..]))
    } else {
        Err(ParseError::new("expected beat(4_) or beat:16(...)", full_input.len() - input.len(), full_input.to_string()))
    }
}

fn parse_beat_shorthand(s: &str, full_input: &str, input: &str) -> ParseResult<BeatSpec> {
    match s {
        "4_" => Ok(BeatSpec::FourToFloor),
        "2_4" => Ok(BeatSpec::HalfNotes),
        "off" => Ok(BeatSpec::Offbeat),
        "bk2" => Ok(BeatSpec::Break2),
        _ => Err(ParseError::new(
            format!("unknown beat shorthand: '{s}'. Use 4_, 2_4, off, bk2, or beat:16(1,5,...)"),
            full_input.len() - input.len(), full_input.to_string())),
    }
}

/// Extract content from `name("pattern")` — for `n("c e g")` inside fluent chain.
fn extract_paren_quoted<'a>(input: &'a str, full_input: &str) -> ParseResult<(String, &'a str)> {
    let after = input.find('(').map(|i| &input[i+1..]).unwrap_or("");
    let after = after.trim_start();
    if after.starts_with('"') {
        let (content, remainder) = extract_quoted(after, full_input)?;
        // remainder starts after closing quote — need to skip the closing paren
        let remainder = remainder.trim_start();
        let remainder = remainder.strip_prefix(')').unwrap_or(remainder);
        Ok((content, remainder))
    } else {
        let closing = after.find(')').ok_or_else(||
            ParseError::new("expected ')' after pattern", full_input.len() - after.len(), full_input.to_string()))?;
        Ok((after[..closing].trim().to_string(), &after[closing + 1..]))
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::parse_music_line;
    use super::*;
    #[test]
    fn test_parse_fluent_create() {
        let line = "new track(kick).device(kick.v9).beat(4_).mute().clip(start)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Fluent(cmd) = result {
            assert!(cmd.create);
            assert_eq!(cmd.track, "kick");
            assert_eq!(cmd.steps.len(), 4);
            assert!(matches!(cmd.steps[0], FluentStep::Device(_)));
            assert!(matches!(cmd.steps[1], FluentStep::Beat(BeatSpec::FourToFloor)));
            assert!(matches!(cmd.steps[2], FluentStep::Mute));
            assert!(matches!(cmd.steps[3], FluentStep::ClipAction(ClipAction::Start)));
        } else {
            panic!("expected Fluent, got {:?}", result);
        }
    }

    #[test]
    fn test_parse_fluent_explicit_beat() {
        let line = "new track(hat).device(hat.v8).beat:16(1,5,9,13)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Fluent(cmd) = result {
            assert_eq!(cmd.track, "hat");
            if let FluentStep::Beat(BeatSpec::Explicit { grid, positions }) = &cmd.steps[1] {
                assert_eq!(*grid, 16);
                // 1-based input 1,5,9,13 → stored 0-based 0,4,8,12
                assert_eq!(*positions, vec![0, 4, 8, 12]);
            } else { panic!("expected Beat::Explicit"); }
        } else { panic!("expected Fluent"); }
    }

    #[test]
    fn test_parse_fluent_synth() {
        let line = "new track(bass).device(Polymer).add(Delay-2).n(\"0 2 4 0\")";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Fluent(cmd) = result {
            assert_eq!(cmd.track, "bass");
            assert_eq!(cmd.steps.len(), 3);
            assert!(matches!(cmd.steps[0], FluentStep::Device(_)));
            assert!(matches!(cmd.steps[1], FluentStep::Add(_)));
            assert!(matches!(cmd.steps[2], FluentStep::Pattern { .. }));
        } else { panic!("expected Fluent"); }
    }
    #[test]
    fn test_parse_fluent_c_slots() {
        // t(bass).c(0).start  — clip slot control in chain
        let line = "t(bass).c(0).start";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Fluent(cmd) = result {
            assert!(!cmd.create);
            assert_eq!(cmd.track, "bass");
            assert!(matches!(&cmd.steps[0], FluentStep::ClipCtrl(_)));
        } else { panic!("expected Fluent, got {:?}", result); }
    }
    #[test]
    fn test_parse_fluent_note_mods() {
        let line = r#"new track(bass).device(Polymer).n("c e g").vel(80).pres(50).chnz(75)"#;
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Fluent(cmd) = result {
            assert_eq!(cmd.steps.len(), 2);
            if let FluentStep::Pattern { mods, .. } = &cmd.steps[1] {
                assert_eq!(mods.vel, vec![Some(80.0)]);
                assert_eq!(mods.pressure, vec![Some(50.0)]);
                assert_eq!(mods.chance, vec![Some(75.0)]);
            } else {
                panic!("expected Pattern step");
            }
        } else {
            panic!("expected Fluent");
        }
    }
}
