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
    pub c: String,
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl Request {
    pub fn new(c: impl Into<String>) -> Self {
        Self {
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
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<ErrorBody>,
}

pub fn send_request(stream: &mut TcpStream, req: &Request) -> Result<Response, crate::Error> {
    let body = serde_json::to_vec(req).map_err(|e| crate::Error::Request(e.to_string()))?;
    write_frame(stream, &body)?;
    let resp_body = read_frame(stream)?;
    let resp: Response = serde_json::from_slice(&resp_body)
        .map_err(|e| crate::Error::InvalidResponse(e.to_string()))?;
    Ok(resp)
}

fn write_frame(stream: &mut TcpStream, body: &[u8]) -> Result<(), crate::Error> {
    if body.len() > u32::MAX as usize {
        return Err(crate::Error::Request("message too large".into()));
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(body);
    stream
        .write_all(&frame)
        .map_err(|e| crate::Error::Request(e.to_string()))?;
    stream
        .flush()
        .map_err(|e| crate::Error::Request(e.to_string()))?;
    Ok(())
}

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, crate::Error> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| {
        crate::Error::Request(format!(
            "no response from extension ({e}). \
             Connected, but Bitwig did not answer — is Codewig controller enabled?"
        ))
    })?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(crate::Error::Request(format!(
            "response too large: {len} bytes"
        )));
    }
    let mut body = vec![0u8; len];
    stream
        .read_exact(&mut body)
        .map_err(|e| crate::Error::Request(e.to_string()))?;
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
            "cannot reach Codewig extension at {addr} ({e}). \
             Is Bitwig running with the Codewig controller enabled?"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Loopback socket pair: (client_end, server_end).
    fn socket_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    #[test]
    fn frame_roundtrip() {
        let (mut a, mut b) = socket_pair();
        let body = br#"{"c":"ping","k":"tempo","v":120}"#;
        write_frame(&mut a, body).unwrap();
        let got = read_frame(&mut b).unwrap();
        assert_eq!(got, body);

        // empty body and a larger one (UTF-8, multi-byte)
        write_frame(&mut a, b"").unwrap();
        assert_eq!(read_frame(&mut b).unwrap(), b"");
        let big = "ü".repeat(100_000).into_bytes();
        write_frame(&mut a, &big).unwrap();
        assert_eq!(read_frame(&mut b).unwrap(), big);
    }

    #[test]
    fn frame_length_cap_16mib() {
        let (mut a, mut b) = socket_pair();
        // Hand-write a header claiming 16 MiB + 1 — must fail before body read.
        a.write_all(&(16 * 1024 * 1024u32 + 1).to_be_bytes()).unwrap();
        a.flush().unwrap();
        let err = read_frame(&mut b).unwrap_err();
        assert!(err.to_string().contains("response too large"), "{err}");

        // exactly 16 MiB is accepted (send a real body of that size)
        let body = vec![7u8; 16 * 1024 * 1024];
        write_frame(&mut a, &body).unwrap();
        assert_eq!(read_frame(&mut b).unwrap(), body);
    }
}
