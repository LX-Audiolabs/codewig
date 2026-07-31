fn main() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;
    let addr: std::net::SocketAddr = "127.0.0.1:9470".parse().unwrap();
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).expect("connect");
    s.set_nodelay(true).ok();
    s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let body = br#"{"id":1,"c":"ping"}"#;
    let mut frame = Vec::new();
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(body);
    println!("single write {} bytes", frame.len());
    s.write_all(&frame).unwrap();
    s.flush().unwrap();
    let mut hdr = [0u8; 4];
    s.read_exact(&mut hdr).expect("hdr");
    let n = u32::from_be_bytes(hdr) as usize;
    let mut buf = vec![0u8; n];
    s.read_exact(&mut buf).unwrap();
    println!("ok: {}", String::from_utf8_lossy(&buf));
}
