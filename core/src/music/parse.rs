//! WIGSCRIPT line parser + mini-notation parser.
//!
//! Parses strings like:
//! - `bass: n "c e g" +cutoff:0.3`
//! - `!bass Polymer Filter Delay-2`
//! - Percussion: fluent `.beat(4_)` (not Strudel hit markers bd/hh)

use super::ast::*;
use std::fmt;

/// Error from parsing a WIGSCRIPT line or mini-notation pattern.
#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub pos: usize,
    pub input: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}: {}", self.pos, self.msg)?;
        if !self.input.is_empty() {
            write!(f, "\n  {}\n  {:>width$}", self.input, "^", width = self.pos + 2)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    fn new(msg: impl Into<String>, pos: usize, input: impl Into<String>) -> Self {
        Self { msg: msg.into(), pos, input: input.into() }
    }
}

type ParseResult<T> = Result<T, ParseError>;

// ── WIGSCRIPT line parser ──────────────────────────────────────────

/// Parse a single REPL line into a [`MusicLine`].
pub fn parse_music_line(input: &str) -> ParseResult<MusicLine> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(MusicLine::Empty);
    }

    let s = trimmed;

    // Transport
    if s == "play" { return Ok(MusicLine::Transport(TransportCmd::Play)); }
    if s == "stop" { return Ok(MusicLine::Transport(TransportCmd::Stop)); }

    // Tempo
    if let Some(rest) = s.strip_prefix("tempo ") {
        let bpm: f64 = rest.trim().parse().map_err(|_|
            ParseError::new("invalid tempo", 6, s.to_string()))?;
        return Ok(MusicLine::Tempo(bpm));
    }

    // Key
    if let Some(rest) = s.strip_prefix("k ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(ParseError::new(
                "key expects: k <root> <scale>, e.g. 'k C minor'",
                0, s.to_string()));
        }
        return Ok(MusicLine::Key { root: parts[0].to_string(), scale: parts[1].to_string() });
    }

    // Mode switch
    if let Some(mode) = s.strip_prefix("mode ") {
        return Ok(MusicLine::ModeSwitch(mode.trim().to_string()));
    }

    // Passthrough: `> command`
    if let Some(rest) = s.strip_prefix('>') {
        return Ok(MusicLine::PassThrough(rest.trim().to_string()));
    }

    // Param: `t(kick).d(kick.v9): decay(280)...`  — check BEFORE fluent (has `):`)
    if (s.starts_with("t(") || s.starts_with("track(")) && s.contains("):") {
        return parse_param_cmd(s);
    }

    // Fluent: `new track(...)...` or `t(...)...`
    if s.starts_with("new track(") || s.starts_with("t(") {
        return parse_fluent(s);
    }

    // Mute/Unmute
    if let Some(rest) = s.strip_prefix("mute(") {
        return parse_mute_cmd(rest, false, s);
    }
    if let Some(rest) = s.strip_prefix("unmute(") {
        return parse_mute_cmd(rest, true, s);
    }

    // Scene: `s(1).start` | `s(verse).start` | `scene(0).stop` | `scene(verse).start`
    if s.starts_with("s(") || s.starts_with("scene(") {
        return parse_scene_cmd(s);
    }

    // Clip control: `c(bass.0).start` | `c(bass.0).stop`
    if s.starts_with("c(") {
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
        return Err(ParseError::new("chain: missing name", 1, full_input.to_string()));
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
        (t, r[1..].trim_start())  // skip the colon
    } else {
        // No colon — could be `d "pattern"` shorthand
        return parse_shorthand(s);
    };

    // Parse target for track, @clip, :kit
    let target = parse_target(target_str)?;

    // Parse the action + pattern + params from rest
    let (action, pattern, rest2) = parse_action_pattern(rest, s)?;

    // Parse remaining params
    let (params, transpose, scale_transpose) = parse_params(rest2, s)?;

    Ok(MusicLine::Music(MusicCmd {
        target,
        action,
        pattern,
        params,
        transpose,
        scale_transpose,
    }))
}

/// Find the position of the first `:` that separates target from command.
fn find_colon_sep(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            // colon at position i — check if next char is space or we're at a cmd boundary
            if i + 1 >= bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'n' || bytes[i + 1] == b'd' || bytes[i + 1] == b'c' {
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
        return Ok(Target { track: String::new(), clip: None, drum_kit: None });
    }

    let mut track = s.to_string();
    let mut clip: Option<ClipRef> = None;
    let mut drum_kit: Option<String> = None;

    // Check for @clipname
    if let Some(at_pos) = track.find('@') {
        let clip_name = track[at_pos + 1..].to_string();
        track = track[..at_pos].to_string();
        clip = Some(ClipRef::Name(clip_name));
    }

    // Check for :kit suffix on track
    if let Some((t, kit)) = track.split_once(':') {
        let kit_lower = kit.to_lowercase();
        if matches!(kit_lower.as_str(), "808" | "909" | "retro" | "default") {
            track = t.to_string();
            drum_kit = Some(kit_lower);
        }
    }

    Ok(Target { track, clip, drum_kit })
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

    // Find quoted pattern
    let (pattern, rest2) = extract_quoted(rest, s)?;

    let (params, transpose, scale_transpose) = parse_params(rest2, s)?;

    Ok(MusicLine::Music(MusicCmd {
        target: Target { track: String::new(), clip: None, drum_kit },
        action,
        pattern,
        params,
        transpose,
        scale_transpose,
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

fn parse_action_pattern<'a>(rest: &'a str, full_input: &str) -> ParseResult<(MusicAction, String, &'a str)> {
    let trimmed = rest.trim();
    let (action_name, rest1) = trimmed.split_once(' ')
        .ok_or_else(|| ParseError::new(
            "expected action (n, chord, arp) followed by pattern",
            full_input.len() - rest.len(), full_input.to_string()))?;

    let (action, _drum_kit) = parse_action_name(action_name)?;

    let (pattern, rest2) = extract_quoted(rest1, full_input)?;

    Ok((action, pattern, rest2))
}

/// Extract a double-quoted string, returning content and remaining input.
fn extract_quoted<'a>(input: &'a str, full_input: &str) -> ParseResult<(String, &'a str)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with('"') {
        return Err(ParseError::new(
            "expected quoted pattern starting with '\"'",
            full_input.len() - input.len(), full_input.to_string()));
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
            full_input.len() - input.len(), full_input.to_string())),
    }
}

fn parse_params(mut rest: &str, full_input: &str) -> ParseResult<(Vec<ParamSet>, Option<i32>, Option<i32>)> {
    let mut params = Vec::new();
    let mut transpose: Option<i32> = None;
    let mut scale_transpose: Option<i32> = None;

    rest = rest.trim_start();

    while !rest.is_empty() {
        if rest.starts_with('+') {
            // +name:value — single snapshot only (no sequences / automation)
            // Find the end of this param token (next space or + or ^ or end)
            let end = rest[1..].find(|c: char| c.is_whitespace())
                .map(|p| p + 1)
                .unwrap_or(rest.len());
            let token = &rest[..end];

            if let Some((name, val_str)) = token[1..].split_once(':') {
                if val_str.contains(':') {
                    return Err(ParseError::new(
                        format!(
                            "param '{name}': sequences unsupported — snapshot only (+{name}:value)"
                        ),
                        full_input.len() - rest.len(),
                        full_input.to_string(),
                    ));
                }
                match val_str.trim().parse::<f64>() {
                    Ok(value) => {
                        params.push(ParamSet {
                            name: name.to_string(),
                            value,
                        });
                        rest = rest[end..].trim_start();
                        continue;
                    }
                    Err(_) => {
                        return Err(ParseError::new(
                            format!("invalid param value for '{name}'"),
                            full_input.len() - rest.len(),
                            full_input.to_string(),
                        ));
                    }
                }
            } else {
                return Err(ParseError::new(
                    "param format: +name:value",
                    full_input.len() - rest.len(), full_input.to_string()));
            }
        } else if rest.starts_with("^^") {
            if let Some(num_str) = rest[2..].split_whitespace().next() {
                let n: i32 = num_str.parse().map_err(|_|
                    ParseError::new("invalid scaleTranspose value", full_input.len() - rest.len(), full_input.to_string()))?;
                scale_transpose = Some(n);
                rest = &rest[2 + num_str.len()..];
            } else {
                return Err(ParseError::new("^^ requires number", full_input.len() - rest.len(), full_input.to_string()));
            }
        } else if rest.starts_with('^') {
            if let Some(num_str) = rest[1..].split_whitespace().next() {
                let n: i32 = num_str.parse().map_err(|_|
                    ParseError::new("invalid transpose value", full_input.len() - rest.len(), full_input.to_string()))?;
                transpose = Some(n);
                rest = &rest[1 + num_str.len()..];
            } else {
                return Err(ParseError::new("^ requires number", full_input.len() - rest.len(), full_input.to_string()));
            }
        } else if rest.starts_with('!') && rest.len() <= 2 {
            // `!` at end = launch — handled in parse_target
            break;
        } else {
            break;
        }
        rest = rest.trim_start();
    }

    Ok((params, transpose, scale_transpose))
}

// ── Mini-notation parser ───────────────────────────────────────────

/// Parse a mini-notation pattern string (the content between quotes in `n "..."`).
pub fn parse_mini_pattern(input: &str) -> ParseResult<Pattern> {
    let tokens = tokenize(input)?;
    let mut parser = MiniParser::new(tokens, input);
    parser.parse_pattern()
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    NoteName(String),    // "c4", "eb", "f#"
    Number(i32),
    Float(f64),
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Star,
    Slash,
    Bang,
    Underscore,
    Question,
    At,
    Pipe,
    Tilde,
    Dot,
    Comma,
    Colon,
    Percent,
}

fn tokenize(input: &str) -> ParseResult<Vec<(Token, usize)>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        let pos = i;

        match ch {
            ' ' | '\t' | '\n' => { i += 1; continue; }
            '[' => { tokens.push((Token::LBracket, pos)); i += 1; }
            ']' => { tokens.push((Token::RBracket, pos)); i += 1; }
            '<' => { tokens.push((Token::LAngle, pos)); i += 1; }
            '>' => { tokens.push((Token::RAngle, pos)); i += 1; }
            '{' => { tokens.push((Token::LBrace, pos)); i += 1; }
            '}' => { tokens.push((Token::RBrace, pos)); i += 1; }
            '(' => { tokens.push((Token::LParen, pos)); i += 1; }
            ')' => { tokens.push((Token::RParen, pos)); i += 1; }
            '*' => { tokens.push((Token::Star, pos)); i += 1; }
            '/' => { tokens.push((Token::Slash, pos)); i += 1; }
            '!' => { tokens.push((Token::Bang, pos)); i += 1; }
            '_' => { tokens.push((Token::Underscore, pos)); i += 1; }
            '?' => { tokens.push((Token::Question, pos)); i += 1; }
            '@' => { tokens.push((Token::At, pos)); i += 1; }
            '|' => { tokens.push((Token::Pipe, pos)); i += 1; }
            '~' => { tokens.push((Token::Tilde, pos)); i += 1; }
            '.' => { tokens.push((Token::Dot, pos)); i += 1; }
            ',' => { tokens.push((Token::Comma, pos)); i += 1; }
            ':' => { tokens.push((Token::Colon, pos)); i += 1; }
            '%' => { tokens.push((Token::Percent, pos)); i += 1; }

            // Number
            '0'..='9' | '-' => {
                let start = i;
                if ch == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    i += 1;
                }
                let mut has_dot = false;
                while i < chars.len() {
                    let c = chars[i];
                    if c.is_ascii_digit() {
                        i += 1;
                    } else if c == '.' && !has_dot && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                        has_dot = true;
                        i += 1;
                    } else {
                        break;
                    }
                }
                let num_str: String = chars[start..i].iter().collect();
                if has_dot {
                    if let Ok(f) = num_str.parse::<f64>() {
                        tokens.push((Token::Float(f), pos));
                    } else {
                        return Err(ParseError::new(format!("invalid float: {num_str}"), pos, input.to_string()));
                    }
                } else {
                    tokens.push((Token::Number(num_str.parse().unwrap_or(0)), pos));
                }
            }

            // Note name (c4, eb, …) — no Strudel hit markers (bd/hh)
            'a'..='z' | 'A'..='Z' => {
                let start = i;
                while i < chars.len() {
                    let c = chars[i];
                    if c.is_alphanumeric() || c == '#' || c == 'b' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push((Token::NoteName(word), pos));
            }

            _ => {
                return Err(ParseError::new(
                    format!("unexpected character: '{ch}'"), pos, input.to_string()));
            }
        }
    }

    Ok(tokens)
}

struct MiniParser {
    tokens: Vec<(Token, usize)>,
    input: String,
    pos: usize,
}

impl MiniParser {
    fn new(tokens: Vec<(Token, usize)>, input: &str) -> Self {
        Self { tokens, input: input.to_string(), pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn peek_pos(&self) -> usize {
        self.tokens.get(self.pos).map(|(_, p)| *p).unwrap_or(self.input.len())
    }

    fn advance(&mut self) -> Option<(Token, usize)> {
        let item = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        item
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(msg, self.peek_pos(), self.input.clone())
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let mut sequences = Vec::new();
        sequences.push(self.parse_sequence()?);
        while self.peek() == Some(&Token::Comma) {
            self.advance();
            sequences.push(self.parse_sequence()?);
        }
        if self.peek().is_some() {
            return Err(self.err(format!("unexpected token after pattern: {:?}", self.peek())));
        }
        Ok(Pattern { sequences })
    }

    fn parse_sequence(&mut self) -> ParseResult<Sequence> {
        let mut events = Vec::new();
        loop {
            match self.peek() {
                None | Some(Token::Comma) | Some(Token::RBracket)
                    | Some(Token::RAngle) | Some(Token::RBrace)
                    | Some(Token::RParen) | Some(Token::Pipe) => break,
                Some(Token::Dot) => {
                    self.advance(); // skip dot separator
                    continue;
                }
                _ => {
                    events.push(self.parse_event()?);
                }
            }
        }
        Ok(Sequence { events })
    }

    fn parse_event(&mut self) -> ParseResult<Event> {
        let atom = self.parse_atom()?;
        let mut suffixes = Vec::new();
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    let n = self.expect_number("*")?;
                    suffixes.push(Suffix::Repeat(n as u32));
                }
                Some(Token::Slash) => {
                    self.advance();
                    let n = self.expect_number("/")?;
                    suffixes.push(Suffix::Slow(n as u32));
                }
                Some(Token::Bang) => {
                    self.advance();
                    let n = self.expect_number("!")?;
                    suffixes.push(Suffix::Replicate(n as u32));
                }
                Some(Token::Underscore) => {
                    self.advance();
                    suffixes.push(Suffix::Elongate);
                }
                Some(Token::Question) => {
                    self.advance();
                    // Check for optional probability
                    if let Some(Token::Float(p)) = self.peek() {
                        let p = *p;
                        self.advance();
                        suffixes.push(Suffix::RandomDrop(Some(p)));
                    } else {
                        suffixes.push(Suffix::RandomDrop(None));
                    }
                }
                Some(Token::At) => {
                    self.advance();
                    let n = self.expect_number("@")?;
                    suffixes.push(Suffix::ElongateN(n as u32));
                }
                Some(Token::Colon) => {
                    self.advance();
                    let n = self.expect_number(":")?;
                    suffixes.push(Suffix::Octave(n));
                }
                Some(Token::LParen) => {
                    self.advance();
                    let beats = self.expect_number("euclid beats")? as u32;
                    self.expect_comma()?;
                    let steps = self.expect_number("euclid steps")? as u32;
                    let offset = if self.peek() == Some(&Token::Comma) {
                        self.advance();
                        Some(self.expect_number("euclid offset")? as u32)
                    } else {
                        None
                    };
                    if self.peek() == Some(&Token::RParen) {
                        self.advance();
                    }
                    suffixes.push(Suffix::Euclid { beats, steps, offset });
                }
                _ => break,
            }
        }
        Ok(Event { atom, suffixes })
    }

    fn parse_atom(&mut self) -> ParseResult<Atom> {
        match self.peek() {
            Some(Token::NoteName(name)) => {
                let name = name.clone();
                self.advance();
                Ok(Atom::Note(name))
            }
            Some(Token::Number(n)) => {
                let n = *n;
                self.advance();
                Ok(Atom::Midi(n))
            }
            Some(Token::Tilde) => {
                self.advance();
                Ok(Atom::Rest)
            }
            Some(Token::LBracket) => {
                self.advance();
                let mut seqs = Vec::new();
                seqs.push(self.parse_sequence()?);

                // Check for random choice | inside brackets
                if self.peek() == Some(&Token::Pipe) {
                    self.advance();
                    let mut atoms = Vec::new();
                    // Extract atoms from the first sequence
                    for ev in &seqs[0].events {
                        atoms.push(ev.atom.clone());
                    }
                    loop {
                        atoms.push(self.parse_atom()?);
                        match self.peek() {
                            Some(Token::Pipe) => { self.advance(); }
                            Some(Token::RBracket) => break,
                            _ => return Err(self.err("expected | or ] in random choice")),
                        }
                    }
                    // Consume RBracket
                    if self.peek() == Some(&Token::RBracket) {
                        self.advance();
                    }
                    return Ok(Atom::RandomChoice(atoms));
                }

                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    seqs.push(self.parse_sequence()?);
                }
                if self.peek() == Some(&Token::RBracket) {
                    self.advance();
                } else {
                    return Err(self.err("expected ]"));
                }
                Ok(Atom::Group(seqs))
            }
            Some(Token::LAngle) => {
                self.advance();
                let mut alts: Vec<Vec<Sequence>> = Vec::new();
                let current = vec![self.parse_sequence()?];
                alts.push(current);

                while self.peek() != Some(&Token::RAngle) && self.peek().is_some() {
                    let next = vec![self.parse_sequence()?];
                    alts.push(next);
                }
                if self.peek() == Some(&Token::RAngle) {
                    self.advance();
                }
                Ok(Atom::Alternate(alts))
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut polys: Vec<Vec<Sequence>> = Vec::new();
                let current = vec![self.parse_sequence()?];
                polys.push(current);

                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    let next = vec![self.parse_sequence()?];
                    polys.push(next);
                }

                // Check for %N subdivision
                let subdivision = if self.peek() == Some(&Token::Percent) {
                    self.advance();
                    Some(self.expect_number("%")? as u32)
                } else {
                    None
                };

                if self.peek() == Some(&Token::RBrace) {
                    self.advance();
                }

                if let Some(n) = subdivision {
                    Ok(Atom::Subdivide(polys[0].clone(), n))
                } else {
                    Ok(Atom::Polymetric(polys))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let beats = self.expect_number("euclid beats")? as u32;
                self.expect_comma()?;
                let steps = self.expect_number("euclid steps")? as u32;
                let offset = if self.peek() == Some(&Token::Comma) {
                    self.advance();
                    Some(self.expect_number("euclid offset")? as u32)
                } else {
                    None
                };
                if self.peek() == Some(&Token::RParen) {
                    self.advance();
                } else {
                    return Err(self.err("expected ) after euclid"));
                }
                Ok(Atom::Euclid { beats, steps, offset })
            }
            _ => Err(self.err("expected note, drum, number, ~, [, <, {, or ("))
        }
    }

    fn expect_number(&mut self, context: &str) -> ParseResult<i32> {
        match self.advance() {
            Some((Token::Number(n), _)) => Ok(n),
            Some((tok, _)) => Err(self.err(format!("expected number after {context}, got {:?}", tok))),
            None => Err(self.err(format!("expected number after {context}, got end of input"))),
        }
    }

    fn expect_comma(&mut self) -> ParseResult<()> {
        match self.advance() {
            Some((Token::Comma, _)) => Ok(()),
            Some((tok, _)) => Err(self.err(format!("expected comma, got {:?}", tok))),
            None => Err(self.err("expected comma, got end of input")),
        }
    }
}

// ── Fluent parser ──────────────────────────────────────────────────

/// Parse `new track(kick).device(v9 kick).beat(4_).mute().clip(start)`
fn parse_fluent(input: &str) -> ParseResult<MusicLine> {
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
            steps.push(FluentStep::Pattern(pattern));
            rest = r;
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

/// Parse `t(kick).d(kick.v9): decay(280) punch(45)`
fn parse_param_cmd(input: &str) -> ParseResult<MusicLine> {
    let s = input;
    let rest = s.strip_prefix("t(").or_else(|| s.strip_prefix("track("))
        .ok_or_else(|| ParseError::new("expected 't(' or 'track('", 0, s.to_string()))?;

    let (track_name, rest) = parse_paren_arg(rest, s, 0)?;

    let rest = rest.strip_prefix(".d(").or_else(|| rest.strip_prefix(".device("))
        .ok_or_else(|| ParseError::new("expected '.d(' or '.device('", s.len() - rest.len(), s.to_string()))?;

    let (dev, rest) = parse_device_in_paren(rest, s)?;

    let rest = rest.strip_prefix(":").or_else(|| rest.strip_prefix(": "))
        .ok_or_else(|| ParseError::new("expected ': params...' after device", s.len() - rest.len(), s.to_string()))?;

    let mut params = Vec::new();
    let mut remaining = rest.trim_start();
    while !remaining.is_empty() {
        if let Some((name, after)) = remaining.split_once('(') {
            let (val_str, after_paren) = after.split_once(')')
                .ok_or_else(|| ParseError::new("expected ')' after param value", s.len() - remaining.len(), s.to_string()))?;
            let val: f64 = val_str.trim().parse().map_err(|_|
                ParseError::new(format!("invalid param value: '{val_str}'"), s.len() - remaining.len(), s.to_string()))?;
            params.push((name.trim().to_string(), val));
            remaining = after_paren.trim_start();
        } else {
            break;
        }
    }

    Ok(MusicLine::Param(ParamCmd { track: track_name, device: dev, params }))
}

/// Parse `mute(kick)` / `mute(kick) 4` / `mute(kick) @bar` / `mute(kick) 4 @bar`
fn parse_mute_cmd(rest: &str, unmute: bool, full_input: &str) -> ParseResult<MusicLine> {
    use super::ast::MuteQuantize;

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

fn parse_paren_arg<'a>(input: &'a str, full_input: &str, _offset: usize) -> ParseResult<(String, &'a str)> {
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

fn parse_device_in_paren<'a>(after: &'a str, full_input: &str) -> ParseResult<(DeviceSpec, &'a str)> {
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
        let positions = positions.map_err(|_|
            ParseError::new(format!("invalid beat positions: '{pos_str}'"), full_input.len() - input.len(), full_input.to_string()))?;

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

#[allow(dead_code)]
fn parse_clip_action<'a>(after: &'a str, full_input: &str) -> ParseResult<(ClipAction, &'a str)> {
    let closing = after.find(')').ok_or_else(||
        ParseError::new("expected ')' after clip action", full_input.len() - after.len(), full_input.to_string()))?;
    let action_str = &after[..closing].trim();
    let action = match *action_str {
        "start" => ClipAction::Start,
        "stop" => ClipAction::Stop,
        _ => return Err(ParseError::new(
            format!("unknown clip action: '{action_str}'. Use 'start' or 'stop'"),
            full_input.len() - after.len(), full_input.to_string())),
    };
    Ok((action, &after[closing + 1..]))
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

/// Parse `s(1).start` | `s(verse).start` | `scene(0).stop` | `scene(name).start`
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
    // Index primary when the whole token is a number; else name (live-friendly).
    let scene = if let Ok(i) = scene_str.parse::<i32>() {
        SceneRef::Index(i)
    } else {
        SceneRef::Name(scene_str.to_string())
    };

    let rest = &after[closing + 1..];
    let action = parse_launch_action(rest, input)?;
    Ok(MusicLine::Scene(SceneCmd { scene, action }))
}

/// Parse `c(bass.0).start` | `c(bass.0, kick.1).start`
fn parse_clip_ctrl_cmd(input: &str) -> ParseResult<MusicLine> {
    let after = input.strip_prefix("c(").unwrap();
    let closing = after.find(')').ok_or_else(||
        ParseError::new("expected ')' after clip ref(s)", input.len() - after.len(), input.to_string()))?;
    let refs_str = &after[..closing];

    let refs: Result<Vec<ClipCtrlRef>, _> = refs_str.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (track, slot_str) = s.split_once('.').ok_or_else(||
                ParseError::new("clip ref format: track.slot", input.len() - after.len(), input.to_string()))?;
            let slot: i32 = slot_str.trim().parse().map_err(|_|
                ParseError::new(format!("invalid slot: '{slot_str}'"), input.len() - after.len(), input.to_string()))?;
            Ok(ClipCtrlRef { track: track.trim().to_string(), slot })
        })
        .collect();
    let refs = refs?;

    if refs.is_empty() {
        return Err(ParseError::new("clip ctrl needs at least one ref, e.g. c(bass.0)", input.len() - after.len(), input.to_string()));
    }

    let rest = &after[closing + 1..];
    let action = parse_launch_action(rest, input)?;
    Ok(MusicLine::ClipCtrl(ClipCtrlCmd { refs, action }))
}

fn parse_launch_action(rest: &str, full_input: &str) -> ParseResult<LaunchAction> {
    let rest = rest.trim_start();
    if rest == ".start" || rest == ".launch" {
        Ok(LaunchAction::Start)
    } else if rest == ".stop" {
        Ok(LaunchAction::Stop)
    } else {
        Err(ParseError::new(
            format!("expected .start or .stop after ref, got '{}'", rest.chars().take(10).collect::<String>()),
            full_input.len() - rest.len(), full_input.to_string()))
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
            assert_eq!(cmd.params[0].name, "cutoff");
            assert_eq!(cmd.params[0].value, 0.3);
            assert_eq!(cmd.params[1].name, "res");
            assert_eq!(cmd.params[1].value, 0.7);
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
        assert!(matches!(parse_music_line("play").unwrap(), MusicLine::Transport(TransportCmd::Play)));
        assert!(matches!(parse_music_line("stop").unwrap(), MusicLine::Transport(TransportCmd::Stop)));
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

    // ── Fluent parser tests ──────────────────────────────────────

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
                assert_eq!(*positions, vec![1, 5, 9, 13]);
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
            assert!(matches!(cmd.steps[2], FluentStep::Pattern(_)));
        } else { panic!("expected Fluent"); }
    }

    #[test]
    fn test_parse_param_cmd() {
        let line = "t(kick).d(kick.v9): decay(280) punch(45)";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Param(cmd) = result {
            assert_eq!(cmd.track, "kick");
            assert_eq!(cmd.device.catalog_name, "kick.v9");
            assert_eq!(cmd.params.len(), 2);
            assert_eq!(cmd.params[0], ("decay".to_string(), 280.0));
            assert_eq!(cmd.params[1], ("punch".to_string(), 45.0));
        } else { panic!("expected Param, got {:?}", result); }
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
        } else { panic!("expected Scene"); }
    }

    #[test]
    fn test_parse_scene_stop() {
        let line = "s(0).stop";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::Scene(cmd) = result {
            assert_eq!(cmd.scene, SceneRef::Index(0));
            assert!(matches!(cmd.action, LaunchAction::Stop));
        } else { panic!("expected Scene"); }

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
        } else { panic!("expected ClipCtrl, got {:?}", result); }
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
        } else { panic!("expected ClipCtrl"); }
    }

    #[test]
    fn test_parse_clip_ctrl_stop() {
        let line = "c(lead.3).stop";
        let result = parse_music_line(line).unwrap();
        if let MusicLine::ClipCtrl(cmd) = result {
            assert_eq!(cmd.refs[0].track, "lead");
            assert_eq!(cmd.refs[0].slot, 3);
            assert!(matches!(cmd.action, LaunchAction::Stop));
        } else { panic!("expected ClipCtrl"); }
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
    fn test_mini_simple_notes() {
        let pat = parse_mini_pattern("c e g").unwrap();
        assert_eq!(pat.sequences.len(), 1);
        assert_eq!(pat.sequences[0].events.len(), 3);
    }

    #[test]
    fn test_mini_rest() {
        let pat = parse_mini_pattern("c ~ g").unwrap();
        assert_eq!(pat.sequences[0].events.len(), 3);
        assert!(matches!(pat.sequences[0].events[1].atom, Atom::Rest));
    }

    #[test]
    fn test_mini_group() {
        let pat = parse_mini_pattern("[c e] g").unwrap();
        assert_eq!(pat.sequences[0].events.len(), 2);
        if let Atom::Group(ref seqs) = pat.sequences[0].events[0].atom {
            assert_eq!(seqs[0].events.len(), 2);
        } else {
            panic!("expected Group");
        }
    }

    #[test]
    fn test_mini_repeat() {
        let pat = parse_mini_pattern("c*3").unwrap();
        assert_eq!(pat.sequences[0].events[0].suffixes.len(), 1);
        assert_eq!(pat.sequences[0].events[0].suffixes[0], Suffix::Repeat(3));
    }

    #[test]
    fn test_mini_euclid() {
        let pat = parse_mini_pattern("c(3,8)").unwrap();
        assert_eq!(pat.sequences[0].events.len(), 1);
        assert!(matches!(pat.sequences[0].events[0].atom, Atom::Note(_)));
        assert!(pat.sequences[0].events[0].suffixes.iter().any(|s| matches!(s, Suffix::Euclid { beats: 3, steps: 8, offset: None })));
    }

    #[test]
    fn test_mini_alternate() {
        let pat = parse_mini_pattern("<c e g>").unwrap();
        assert!(matches!(pat.sequences[0].events[0].atom, Atom::Alternate(_)));
    }

    #[test]
    fn test_mini_no_hit_markers() {
        // "bd" is an ordinary note token now (invalid pitch at expand), not a drum atom
        let pat = parse_mini_pattern("bd ~").unwrap();
        assert!(matches!(pat.sequences[0].events[0].atom, Atom::Note(_)));
        assert!(matches!(pat.sequences[0].events[1].atom, Atom::Rest));
    }

    #[test]
    fn test_mini_superpose() {
        let pat = parse_mini_pattern("c e, g a").unwrap();
        assert_eq!(pat.sequences.len(), 2);
    }
}
