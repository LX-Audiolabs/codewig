//! Wire protocol: 32-bit big-endian length prefix + UTF-8 JSON body
//! (Bitwig RemoteConnection framing).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 9470;

#[derive(Debug, Serialize)]
pub struct Request {
    pub id: u64,
    pub c: String,
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl Request {
    pub fn new(c: impl Into<String>) -> Self {
        Self {
            id: 1,
            c: c.into(),
            fields: Map::new(),
        }
    }

    pub fn field(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub msg: String,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    #[allow(dead_code)]
    pub id: Option<Value>,
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<ErrorBody>,
}

pub fn send_request(
    stream: &mut TcpStream,
    req: &Request,
) -> Result<Response, Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(req)?;
    write_frame(stream, &body)?;
    let resp_body = read_frame(stream)?;
    let resp: Response = serde_json::from_slice(&resp_body)?;
    Ok(resp)
}

fn write_frame(stream: &mut TcpStream, body: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if body.len() > u32::MAX as usize {
        return Err("message too large".into());
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(body);
    stream.write_all(&frame)?;
    stream.flush()?;
    Ok(())
}

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| {
        format!(
            "no response from extension ({e}). \
             Connected, but Bitwig did not answer — is CLIwig controller enabled?"
        )
    })?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(format!("response too large: {len} bytes").into());
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    Ok(body)
}

pub fn connect(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, String> {
    let addr = format!("{host}:{port}");
    let stream = TcpStream::connect_timeout(
        &addr
            .parse()
            .map_err(|e| format!("bad address {addr}: {e}"))?,
        timeout,
    )
    .map_err(|e| {
        format!(
            "cannot reach CLIwig extension at {addr} ({e}). \
             Is Bitwig running with the CLIwig controller enabled?"
        )
    })?;
    let _ = stream.set_nodelay(true);
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    // No fixed settle sleep — Client retries first request if Bitwig callback not ready yet.
    Ok(stream)
}
