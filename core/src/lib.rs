//! Codewig Core — shared library for CLI and UI.
//!
//! Provides a synchronous TCP+JSON client for the Codewig Bitwig extension.

pub mod music;
pub mod paths;
pub mod protocol;

pub use paths::{ensure_user_layout, user_data_dir, user_devices_dir, APP_DIR_NAME};

use protocol::{connect, send_request, Request, Response, DEFAULT_HOST, DEFAULT_PORT};
use serde_json::{json, Map, Value};
use std::cell::RefCell;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use thiserror::Error;

/// Errors from the core client.
#[derive(Error, Debug)]
pub enum Error {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("request failed: {0}")]
    Request(String),
    #[error("extension error {code}: {msg}")]
    Extension { code: String, msg: String },
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

impl Error {
    fn from_response(err: protocol::ErrorBody) -> Self {
        Self::Extension {
            code: err.code,
            msg: err.msg,
        }
    }
}

/// Synchronous client for the Codewig extension.
///
/// Holds one TCP stream and reuses it across requests (Bitwig settle cost is
/// paid once per live connection). IO failure drops the stream and retries once.
/// Clone is cheap config only — each clone starts without a shared socket.
#[derive(Debug)]
pub struct Client {
    host: String,
    port: u16,
    timeout: Duration,
    // ponytail: persistent TCP; reconnect on dead stream (not in MusicSession — transport ≠ music state)
    stream: RefCell<Option<TcpStream>>,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            timeout: Duration::from_millis(2000),
            stream: RefCell::new(None),
        }
    }
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port,
            timeout: self.timeout,
            stream: RefCell::new(None),
        }
    }
}

impl Client {
    pub fn new(host: impl Into<String>, port: u16, timeout_ms: u64) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Duration::from_millis(timeout_ms),
            stream: RefCell::new(None),
        }
    }

    /// Drop the live socket (next send/ping reconnects). Used by UI reconnect.
    pub fn reset(&self) {
        *self.stream.borrow_mut() = None;
    }

    fn connect(&self) -> Result<TcpStream, Error> {
        connect(&self.host, self.port, self.timeout).map_err(Error::Connection)
    }

    fn send(&self, req: Request) -> Result<Option<Value>, Error> {
        // First try existing socket; on transport error, reset + one fresh connect —
        // but only for idempotent commands: retrying `track.new`/`clip.new`/… would
        // execute the side effect twice (the first attempt may have reached Bitwig).
        match self.send_once(&req, false) {
            Ok(v) => Ok(v),
            Err(Error::Request(_)) | Err(Error::Connection(_)) if is_idempotent(&req.c) => {
                self.send_once(&req, true)
            }
            Err(e) => Err(e),
        }
    }

    fn send_once(&self, req: &Request, force_new: bool) -> Result<Option<Value>, Error> {
        if force_new {
            *self.stream.borrow_mut() = None;
        }
        let fresh = {
            let mut slot = self.stream.borrow_mut();
            if slot.is_none() {
                *slot = Some(self.connect()?);
                true
            } else {
                false
            }
        };

        match self.exchange_held(req) {
            Ok(v) => Ok(v),
            // Bitwig RemoteConnection: receive callback may attach a tick late after accept.
            Err(e) if fresh && matches!(e, Error::Request(_)) => {
                thread::sleep(Duration::from_millis(15));
                // On retry failure return the fresher second error, not the stale first.
                self.exchange_held(req)
            }
            Err(e) => Err(e),
        }
    }

    fn exchange_held(&self, req: &Request) -> Result<Option<Value>, Error> {
        let mut slot = self.stream.borrow_mut();
        let stream = slot
            .as_mut()
            .ok_or_else(|| Error::Connection("no stream".into()))?;
        let resp: Response = send_request(stream, req)?;
        if resp.ok {
            Ok(resp.result)
        } else {
            let err = resp
                .error
                .ok_or_else(|| Error::InvalidResponse("missing error body".into()))?;
            // Protocol answered — keep the socket.
            Err(Error::from_response(err))
        }
    }

    fn req(&self, c: impl Into<String>) -> Request {
        Request::new(c)
    }

    // Transport

    pub fn ping(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("ping"))
    }

    pub fn status(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("status"))
    }

    pub fn play(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("play"))
    }

    pub fn stop(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("stop"))
    }

    pub fn set_tempo(&self, bpm: f64) -> Result<Option<Value>, Error> {
        self.send(self.req("set").field("k", "tempo").field("v", bpm))
    }

    // Tracks

    pub fn track_new(&self, kind: &str, at: i32, name: Option<&str>) -> Result<Option<Value>, Error> {
        let mut r = self.req("track.new").field("type", kind).field("at", at);
        if let Some(n) = name {
            r = r.field("name", n);
        }
        self.send(r)
    }

    pub fn track_list(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("track.list"))
    }

    pub fn track_select(&self, r#ref: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("track.select").field("ref", r#ref))
    }

    pub fn track_delete(&self, r#ref: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("track.delete").field("ref", r#ref))
    }

    pub fn track_rename(&self, r#ref: &str, name: &str) -> Result<Option<Value>, Error> {
        self.send(
            self.req("track.rename")
                .field("ref", r#ref)
                .field("name", name),
        )
    }

    pub fn track_move(
        &self,
        r#ref: &str,
        before: Option<&str>,
        after: Option<&str>,
        to: Option<i32>,
    ) -> Result<Option<Value>, Error> {
        let mut r = self.req("track.move").field("ref", r#ref);
        if let Some(b) = before {
            r = r.field("before", b);
        }
        if let Some(a) = after {
            r = r.field("after", a);
        }
        if let Some(t) = to {
            r = r.field("to", t);
        }
        self.send(r)
    }

    pub fn track_mute(&self, refs: &[String], on: bool) -> Result<Option<Value>, Error> {
        self.track_mute_timed(refs, on, None, None)
    }

    /// Mute/unmute with optional musical timing.
    /// - `bars`: after primary action, invert after N bars
    /// - `q`: `"bar"` = apply primary at next bar (while playing)
    pub fn track_mute_timed(
        &self,
        refs: &[String],
        on: bool,
        bars: Option<u32>,
        q: Option<&str>,
    ) -> Result<Option<Value>, Error> {
        let mut r = self
            .req("track.mute")
            .field("refs", refs)
            .field("on", on);
        if let Some(b) = bars {
            r = r.field("bars", b);
        }
        if let Some(quantize) = q {
            r = r.field("q", quantize);
        }
        self.send(r)
    }

    pub fn track_solo(&self, refs: &[String], on: bool) -> Result<Option<Value>, Error> {
        self.send(
            self.req("track.solo")
                .field("refs", refs)
                .field("on", on),
        )
    }

    pub fn track_volume(&self, r#ref: &str, value: f64) -> Result<Option<Value>, Error> {
        self.send(
            self.req("track.volume")
                .field("ref", r#ref)
                .field("v", value),
        )
    }

    // Devices

    pub fn device_add(&self, name: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("device.add").field("name", name))
    }

    pub fn device_list(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("device.list"))
    }

    pub fn device_select(&self, index: i32) -> Result<Option<Value>, Error> {
        self.send(self.req("device.select").field("index", index))
    }

    pub fn device_delete(&self, index: i32) -> Result<Option<Value>, Error> {
        self.send(self.req("device.delete").field("index", index))
    }

    pub fn device_enable(&self, index: i32, on: bool) -> Result<Option<Value>, Error> {
        self.send(
            self.req("device.enable")
                .field("index", index)
                .field("on", on),
        )
    }

    /// Move device at `index` to chain position `to` (0-based).
    pub fn device_move(&self, index: i32, to: i32) -> Result<Option<Value>, Error> {
        self.send(
            self.req("device.move")
                .field("index", index)
                .field("to", to),
        )
    }

    // Params

    pub fn param_list(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("param.list"))
    }

    /// `source`: `"direct"` | `"remote"` | `"all"` (Bitwig Remote Controls pages).
    pub fn param_list_source(&self, source: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("param.list").field("source", source))
    }

    pub fn param_set_name_value(&self, name: &str, value: f64) -> Result<Option<Value>, Error> {
        self.send(
            self.req("param.set")
                .field("name", name)
                .field("v", value),
        )
    }

    pub fn param_set_id_value(&self, id: &str, value: f64) -> Result<Option<Value>, Error> {
        self.send(self.req("param.set").field("id", id).field("v", value))
    }

    pub fn param_set_multi(&self, sets: &[(String, f64)]) -> Result<Option<Value>, Error> {
        let values: Vec<Value> = sets
            .iter()
            .map(|(n, v)| json!({"name": n, "v": v}))
            .collect();
        self.send(self.req("param.set").field("sets", Value::Array(values)))
    }

    // Clips

    pub fn clip_new(
        &self,
        track: &str,
        slot: Option<i32>,
        beats: i32,
        name: Option<&str>,
    ) -> Result<Option<Value>, Error> {
        let mut r = self.req("clip.new").field("track", track).field("beats", beats);
        if let Some(s) = slot {
            r = r.field("slot", s);
        }
        if let Some(n) = name {
            r = r.field("name", n);
        }
        self.send(r)
    }

    pub fn clip_list(&self, track: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("clip.list").field("track", track))
    }

    pub fn clip_launch(&self, track: &str, slot: i32) -> Result<Option<Value>, Error> {
        self.send(
            self.req("clip.launch")
                .field("track", track)
                .field("slot", slot),
        )
    }

    pub fn clip_stop(&self, track: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("clip.stop").field("track", track))
    }

    pub fn clip_rename(&self, track: &str, slot: i32, name: &str) -> Result<Option<Value>, Error> {
        self.send(
            self.req("clip.rename")
                .field("track", track)
                .field("slot", slot)
                .field("name", name),
        )
    }

    pub fn clip_delete(&self, track: &str, slot: i32) -> Result<Option<Value>, Error> {
        self.send(
            self.req("clip.delete")
                .field("track", track)
                .field("slot", slot),
        )
    }

    // Scenes — index primary, name secondary (`ref` string or number)

    pub fn scene_list(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("scene.list"))
    }

    /// Claim / name a scene row. `name` optional. Idempotent if name already exists.
    pub fn scene_new(&self, name: Option<&str>) -> Result<Option<Value>, Error> {
        let mut r = self.req("scene.new");
        if let Some(n) = name {
            r = r.field("name", n);
        }
        self.send(r)
    }

    pub fn scene_launch(&self, r#ref: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("scene.launch").field("ref", r#ref))
    }

    pub fn scene_stop(&self, r#ref: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("scene.stop").field("ref", r#ref))
    }

    pub fn scene_rename(&self, r#ref: &str, name: &str) -> Result<Option<Value>, Error> {
        self.send(
            self.req("scene.rename")
                .field("ref", r#ref)
                .field("name", name),
        )
    }

    pub fn scene_delete(&self, r#ref: &str) -> Result<Option<Value>, Error> {
        self.send(self.req("scene.delete").field("ref", r#ref))
    }

    /// Clear all steps then write notes in one round-trip (`clip.replace-notes`).
    /// Empty `notes` clears only. Prefer this for live pattern rewrite.
    pub fn clip_replace_notes(
        &self,
        track: &str,
        slot: i32,
        notes: &[NoteSpec],
    ) -> Result<Option<Value>, Error> {
        self.send(
            self.req("clip.replace-notes")
                .field("track", track)
                .field("slot", slot)
                .field("notes", notes_to_json(notes)),
        )
    }

    pub fn clip_clear_notes(
        &self,
        track: &str,
        slot: i32,
        step: Option<i32>,
        key: Option<i32>,
    ) -> Result<Option<Value>, Error> {
        // Only one of step/key would make the extension wipe the WHOLE clip.
        if step.is_some() != key.is_some() {
            return Err(Error::Request(
                "clip.clear-notes: step and key must be given together (or neither to clear all)"
                    .into(),
            ));
        }
        let mut r = self
            .req("clip.clear-notes")
            .field("track", track)
            .field("slot", slot);
        if let Some(s) = step {
            r = r.field("step", s);
        }
        if let Some(k) = key {
            r = r.field("key", k);
        }
        self.send(r)
    }

    /// Low-level escape hatch: send any request and get the optional result.
    pub fn send_raw(&self, c: &str, fields: Map<String, Value>) -> Result<Option<Value>, Error> {
        let mut req = self.req(c);
        req.fields = fields;
        self.send(req)
    }
}

/// Commands safe to retry after a transport error (reads or set-to-value writes).
/// Anything that creates/deletes/moves objects (`track.new`, `device.add`,
/// `clip.new`, `scene.new`, `*.delete`, `clip.replace-notes`, `track.move`, …)
/// is NOT idempotent and must surface the error instead of retrying blindly.
fn is_idempotent(cmd: &str) -> bool {
    const IDEMPOTENT: &[&str] = &[
        "ping",
        "status",
        "param.set",
        "track.mute",
        "track.solo",
        "track.volume",
        "track.select",
        "track.rename",
        "device.select",
        "clip.set-notes",
        "clip.launch",
        "clip.stop",
        "scene.launch",
        "scene.stop",
        "play",
        "stop",
        "set",
    ];
    cmd.ends_with(".list") || cmd.starts_with("param.list") || IDEMPOTENT.contains(&cmd)
}

fn notes_to_json(notes: &[NoteSpec]) -> Value {
    Value::Array(
        notes
            .iter()
            .map(|n| {
                let mut m = Map::new();
                m.insert("step".to_string(), json!(n.step));
                m.insert("key".to_string(), json!(n.key));
                m.insert("vel".to_string(), json!(n.vel));
                m.insert("dur".to_string(), json!(n.dur));
                if let Some(v) = n.pressure {
                    m.insert("pressure".to_string(), json!(v));
                }
                if let Some(v) = n.timbre {
                    m.insert("timbre".to_string(), json!(v));
                }
                if let Some(v) = n.pan {
                    m.insert("pan".to_string(), json!(v));
                }
                if let Some(v) = n.gain {
                    m.insert("gain".to_string(), json!(v));
                }
                if let Some(v) = n.chance {
                    m.insert("chance".to_string(), json!(v));
                }
                Value::Object(m)
            })
            .collect(),
    )
}

/// One note for `clip.set-notes` / `clip.replace-notes`.
/// Optional expression fields are wire-normalized values sent to the extension only when set.
#[derive(Debug, Clone, Default)]
pub struct NoteSpec {
    pub step: i32,
    pub key: i32,
    pub vel: i32,
    pub dur: f64,
    pub pressure: Option<f64>,
    pub timbre: Option<f64>,
    pub pan: Option<f64>,
    pub gain: Option<f64>,
    pub chance: Option<f64>,
}

/// Parse the legacy CLI note format `step:key[:vel[:dur]]` into a [`NoteSpec`].
/// `key`: MIDI number or note name (core semantics, Bitwig octaves — `c` = C3 = 60).
/// Defaults: vel 100, dur 1.0 (16th steps). Shared by codewig-cli and codewig-live.
pub fn parse_note_spec(s: &str) -> Result<NoteSpec, String> {
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
    let key = music::scale::key_to_midi(parts[1]).map_err(|e| e.to_string())?;
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
    Ok(NoteSpec {
        step,
        key,
        vel,
        dur,
        ..NoteSpec::default()
    })
}

/// Parse a legacy `name=value` param pair; value must be wire-normalized 0..1.
/// Shared by codewig-cli and codewig-live.
pub fn parse_name_eq_value(s: &str) -> Result<(String, f64), String> {
    let (n, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected name=value, got '{s}'"))?;
    let val: f64 = v.parse().map_err(|_| format!("bad value in '{s}'"))?;
    if !(0.0..=1.0).contains(&val) {
        return Err(format!("value must be 0..1, got {val}"));
    }
    Ok((n.trim().to_string(), val))
}

#[cfg(test)]
mod tests {
    use super::{is_idempotent, parse_name_eq_value, parse_note_spec};

    #[test]
    fn test_idempotent_whitelist() {
        for c in [
            "ping", "status", "play", "stop", "set",
            "track.list", "device.list", "clip.list", "scene.list", "param.list",
            "param.set", "track.mute", "track.solo", "track.volume", "track.select",
            "track.rename", "device.select", "clip.set-notes", "clip.launch", "clip.stop",
            "scene.launch", "scene.stop",
        ] {
            assert!(is_idempotent(c), "{c} should be retryable");
        }
        for c in [
            "track.new", "track.delete", "track.move", "device.add", "device.delete",
            "clip.new", "scene.new", "clip.replace-notes", "clip.clear-notes",
        ] {
            assert!(!is_idempotent(c), "{c} must not be retried (double side effect)");
        }
    }

    #[test]
    fn test_parse_note_spec() {
        let n = parse_note_spec("0:C3:100:1").unwrap();
        assert_eq!((n.step, n.key, n.vel), (0, 60, 100));
        assert_eq!(n.dur, 1.0);

        let n = parse_note_spec("4:E3").unwrap();
        assert_eq!((n.step, n.key, n.vel), (4, 64, 100));
        assert_eq!(n.dur, 1.0);

        // core note semantics: bare name + accidentals
        assert_eq!(parse_note_spec("0:c").unwrap().key, 60);
        assert_eq!(parse_note_spec("0:cis").unwrap().key, 61);
        assert_eq!(parse_note_spec("0:eb3").unwrap().key, 63);

        assert!(parse_note_spec("0").is_err());
        assert!(parse_note_spec("-1:C3").is_err());
        assert!(parse_note_spec("0:C3:0").is_err()); // vel 0
        assert!(parse_note_spec("0:C3:128").is_err());
        assert!(parse_note_spec("0:C3:100:0").is_err()); // dur 0
        assert!(parse_note_spec("0:C3:100:1:extra").is_err());
    }

    #[test]
    fn test_parse_name_eq_value() {
        assert_eq!(parse_name_eq_value("cutoff=0.5").unwrap(), ("cutoff".into(), 0.5));
        assert_eq!(parse_name_eq_value("a=1").unwrap(), ("a".into(), 1.0));
        assert_eq!(parse_name_eq_value(" b =0.5").unwrap(), ("b".into(), 0.5)); // name trimmed
        assert!(parse_name_eq_value("cutoff").is_err());
        assert!(parse_name_eq_value("cutoff=x").is_err());
        assert!(parse_name_eq_value("cutoff=1.5").is_err()); // range 0..1
        assert!(parse_name_eq_value("cutoff=-0.1").is_err());
    }
}
