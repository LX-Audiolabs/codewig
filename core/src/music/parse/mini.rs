//! Mini-notation parser (content between the quotes in `n "…"`).

use super::super::ast::*;
use super::{ParseError, ParseResult};

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
                    match num_str.parse::<i32>() {
                        Ok(n) => tokens.push((Token::Number(n), pos)),
                        Err(_) => {
                            return Err(ParseError::new(
                                format!("invalid integer: {num_str}"), pos, input.to_string()));
                        }
                    }
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
                    let (beats, steps, offset) = self.parse_euclid_args()?;
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
                let (beats, steps, offset) = self.parse_euclid_args()?;
                Ok(Atom::Euclid { beats, steps, offset })
            }
            _ => Err(self.err("expected note, drum, number, ~, [, <, {, or ("))
        }
    }

    /// Parse `beats,steps[,offset])` of a euclid group — the opening `(` is
    /// already consumed. Shared by the atom form `(3,8)` and the suffix form.
    fn parse_euclid_args(&mut self) -> ParseResult<(u32, u32, Option<u32>)> {
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
        Ok((beats, steps, offset))
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

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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
    #[test]
    fn test_number_overflow_rejected() {
        // i32 overflow must be a parse error, not silently 0
        let err = parse_mini_pattern("99999999999999999999").unwrap_err();
        assert!(err.to_string().contains("invalid integer"), "{err}");
        // in-range still works
        assert!(parse_mini_pattern("60").is_ok());
    }
}
