//! WIGSCRIPT line parser + mini-notation parser.
//!
//! Submodules (code moved here verbatim from the former single `parse.rs`):
//! - [`line`]: line dispatch (`target: cmd "pattern" +params`, transport, mute, scenes)
//! - [`fluent`]: fluent chains (`new track(x).device(Y).n(…)`)
//! - [`mini`]: mini-notation patterns (content of `n "…"`)
//!
//! Parses strings like:
//! - `bass: n "c e g" +cutoff:0.3`
//! - `!bass Polymer Filter Delay-2`
//! - Percussion: fluent `.beat(4_)` (not Strudel hit markers bd/hh)

mod fluent;
mod line;
mod mini;

use std::fmt;

pub use line::parse_music_line;
pub use mini::parse_mini_pattern;

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
    pub(crate) fn new(msg: impl Into<String>, pos: usize, input: impl Into<String>) -> Self {
        Self { msg: msg.into(), pos, input: input.into() }
    }
}

pub(crate) type ParseResult<T> = Result<T, ParseError>;
