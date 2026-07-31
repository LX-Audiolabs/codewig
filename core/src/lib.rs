//! CLIwig Core — shared library for CLI and UI.
//!
//! Provides a synchronous TCP+JSON client for the CLIwig Bitwig extension.

pub mod music;
pub mod protocol;

use protocol::{connect, send_request, Request, Response, DEFAULT_HOST, DEFAULT_PORT};
use serde_json::{json, Map, Value};
use std::net::TcpStream;
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

/// Synchronous client for the CLIwig extension.
#[derive(Debug, Clone)]
pub struct Client {
    host: String,
    port: u16,
    timeout: Duration,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            timeout: Duration::from_millis(2000),
        }
    }
}

impl Client {
    pub fn new(host: impl Into<String>, port: u16, timeout_ms: u64) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    fn connect(&self) -> Result<TcpStream, Error> {
        connect(&self.host, self.port, self.timeout).map_err(Error::Connection)
    }

    fn send(&self, req: Request) -> Result<Option<Value>, Error> {
        let mut stream = self.connect()?;
        let resp: Response = send_request(&mut stream, &req).map_err(|e| Error::Request(e.to_string()))?;
        if resp.ok {
            Ok(resp.result)
        } else {
            let err = resp
                .error
                .ok_or_else(|| Error::InvalidResponse("missing error body".into()))?;
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
        self.send(
            self.req("track.mute")
                .field("refs", refs)
                .field("on", on),
        )
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

    // Params

    pub fn param_list(&self) -> Result<Option<Value>, Error> {
        self.send(self.req("param.list"))
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

    pub fn clip_set_notes(
        &self,
        track: &str,
        slot: i32,
        notes: &[NoteSpec],
    ) -> Result<Option<Value>, Error> {
        let notes_json: Vec<Value> = notes
            .iter()
            .map(|n| {
                json!({
                    "step": n.step,
                    "key": n.key,
                    "vel": n.vel,
                    "dur": n.dur,
                })
            })
            .collect();
        self.send(
            self.req("clip.set-notes")
                .field("track", track)
                .field("slot", slot)
                .field("notes", Value::Array(notes_json)),
        )
    }

    pub fn clip_clear_notes(
        &self,
        track: &str,
        slot: i32,
        step: Option<i32>,
        key: Option<i32>,
    ) -> Result<Option<Value>, Error> {
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

/// One note for `clip.set-notes`.
#[derive(Debug, Clone, Copy)]
pub struct NoteSpec {
    pub step: i32,
    pub key: i32,
    pub vel: i32,
    pub dur: f64,
}

/// Run a batch of commands over a single connection, stopping at the first error.
pub fn run_batch<C, F>(client: &Client, commands: Vec<C>, mut execute: F) -> Vec<Result<Option<Value>, Error>>
where
    F: FnMut(&mut TcpStream, C) -> Result<Option<Value>, Error>,
{
    let mut results = Vec::with_capacity(commands.len());
    match client.connect() {
        Ok(mut stream) => {
            for cmd in commands {
                let result = execute(&mut stream, cmd);
                let is_err = result.is_err();
                results.push(result);
                if is_err {
                    break;
                }
            }
        }
        Err(e) => {
            results.push(Err(e));
        }
    }
    results
}
