//! `sc-matchmaker`: the server browser's matchmaker as a process (openspec/changes/matchmaker design 10). It reads
//! `data/matchmaker/config.json`, listens on `--listen` (or `0.0.0.0:$PORT`, or `0.0.0.0:8080`), and answers each
//! connection's one request on its own thread, so a long poll never holds up anyone else.
//!
//! Usage: sc-matchmaker [--root .] [--listen 0.0.0.0:8080]

use sc_matchmaker::{Config, Hub};
use sc_net::http::{read_request, respond};
use std::net::{IpAddr, TcpListener};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

/// The header Fly's proxy names the client's address in; without it, the connection's own address.
const CLIENT_IP_HEADER: &str = "fly-client-ip";

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "200 OK",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        404 => "404 Not Found",
        409 => "409 Conflict",
        413 => "413 Payload Too Large",
        429 => "429 Too Many Requests",
        503 => "503 Service Unavailable",
        504 => "504 Gateway Timeout",
        _ => "500 Internal Server Error",
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let root = PathBuf::from(value("--root").unwrap_or_else(|| ".".into()));
    let listen = value("--listen").unwrap_or_else(|| {
        format!("0.0.0.0:{}", std::env::var("PORT").ok().filter(|p| !p.is_empty()).unwrap_or_else(|| "8080".into()))
    });
    let path = root.join("data/matchmaker/config.json");
    let cfg = match std::fs::read_to_string(&path)
        .map_err(|e| format!("{}: {e}", path.display()))
        .and_then(|t| Config::parse(&t))
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sc-matchmaker: {e}");
            return ExitCode::from(2);
        }
    };
    let listener = match TcpListener::bind(&listen) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("sc-matchmaker: cannot listen on {listen}: {e}");
            return ExitCode::from(2);
        }
    };
    println!("sc-matchmaker: listening on {listen}, protocol {}", sc_net::msg::PROTOCOL);
    let hub = Arc::new(Hub::new(cfg));
    for stream in listener.incoming().flatten() {
        let hub = hub.clone();
        let spawned = std::thread::Builder::new().name("request".into()).spawn(move || {
            let mut s = stream;
            let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
            let Ok(req) = read_request(&mut s) else { return };
            let own: IpAddr = s.peer_addr().map(|a| a.ip()).unwrap_or(IpAddr::from([0, 0, 0, 0]));
            let from = req.header(CLIENT_IP_HEADER).and_then(|v| v.parse().ok()).unwrap_or(own);
            let (code, body) = hub.handle(&req, from);
            let _ = respond(&mut s, status_text(code), "application/json", &body);
        });
        if let Err(e) = spawned {
            eprintln!("sc-matchmaker: {e}");
        }
    }
    ExitCode::SUCCESS
}
