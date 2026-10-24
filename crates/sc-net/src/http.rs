//! The little HTTP/1.1 both ends of signalling speak (netcode-and-sessions section 2; matchmaker design 10.4): one
//! request a connection, `Connection: close`, JSON bodies with a length. The LAN signalling endpoint, the matchmaker
//! and their clients share it, so a request is read and answered by one implementation (CLAUDE.md 6.1).
//!
//! It is deliberately small: no chunked bodies, no keep-alive, no TLS. Fly's proxy ends TLS in front of the
//! matchmaker; the native client's TLS is matchmaker task 2.4.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// The largest request head accepted, in bytes.
const MAX_HEAD: usize = 64 * 1024;
/// The largest body accepted, in bytes.
pub const MAX_BODY: usize = 256 * 1024;

/// One request as read from a connection.
#[derive(Debug)]
pub struct Request {
    /// `GET`, `POST`, `DELETE`.
    pub method: String,
    /// The path, with any query left on.
    pub path: String,
    /// Header names in lower case, with their values trimmed.
    pub headers: Vec<(String, String)>,
    /// The body (empty when there is none).
    pub body: Vec<u8>,
}

impl Request {
    /// A header's value by its lower-case name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

/// Read one request: the head, then a body of its `Content-Length`.
pub fn read_request(s: &mut TcpStream) -> Result<Request, String> {
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    let head_end = loop {
        let n = s.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("connection closed before the headers ended".into());
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(p) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break p + 4;
        }
        if data.len() > MAX_HEAD {
            return Err("headers too long".into());
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let mut lines = head.lines();
    let first = lines.next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let len: usize = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    if len > MAX_BODY {
        return Err("body too long".into());
    }
    let mut body = data[head_end..].to_vec();
    while body.len() < len {
        let n = s.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("connection closed inside the body".into());
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(len);
    Ok(Request { method, path, headers, body })
}

/// Answer a request and close.
pub fn respond(s: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| e.to_string())
}

/// Where a service is: `http://host[:port]`, the port 80 when none is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    /// The host name or address.
    pub host: String,
    /// The TCP port.
    pub port: u16,
}

impl Url {
    /// Parse `http://host[:port]` (a trailing slash is allowed). `https://` is refused with the reason until the
    /// native client speaks TLS (matchmaker task 2.4).
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim().trim_end_matches('/');
        if s.starts_with("https://") {
            return Err(format!("{s}: HTTPS is not spoken by the native client yet; use the http:// address"));
        }
        let rest = s.strip_prefix("http://").ok_or_else(|| format!("{s}: not an http:// address"))?;
        if rest.is_empty() || rest.contains('/') {
            return Err(format!("{s}: expected http://host or http://host:port"));
        }
        // `[::1]:7800` or `host:7800`; a bare IPv6 address in brackets has no port.
        let (host, port) = match rest.rsplit_once(':') {
            Some((h, p)) if !p.contains(']') && (!h.contains(':') || h.ends_with(']')) => {
                (h, p.parse().map_err(|_| format!("{s}: bad port {p}"))?)
            }
            _ => (rest, 80),
        };
        Ok(Self { host: host.trim_matches(['[', ']']).to_owned(), port })
    }

    /// The address to connect to (the first IPv4 one, as the client's server addresses).
    pub fn addr(&self) -> Result<SocketAddr, String> {
        let mut all: Vec<SocketAddr> = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|e| format!("cannot resolve {}: {e}", self.host))?
            .collect();
        all.sort_by_key(|a| !a.is_ipv4());
        all.into_iter().next().ok_or_else(|| format!("{} has no address", self.host))
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "http://{}:{}", self.host, self.port)
    }
}

/// A response: its status code and body.
#[derive(Debug)]
pub struct Response {
    /// 200, 404 and so on.
    pub status: u16,
    /// The body.
    pub body: Vec<u8>,
}

/// One request to `url`; waits at most `wait` for the whole answer. `headers` are extra `name: value` pairs.
pub fn call(
    url: &Url,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    wait: Duration,
) -> Result<Response, String> {
    let addr = url.addr()?;
    let mut s =
        TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|e| format!("cannot reach {url}: {e}"))?;
    let _ = s.set_read_timeout(Some(wait));
    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n", url.host);
    for (k, v) in headers {
        head += &format!("{k}: {v}\r\n");
    }
    if !body.is_empty() || method == "POST" {
        head += &format!("Content-Type: application/json\r\nContent-Length: {}\r\n", body.len());
    }
    head += "\r\n";
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| e.to_string())?;
    let mut resp = Vec::new();
    s.read_to_end(&mut resp).map_err(|e| format!("{url}{path}: {e}"))?;
    let end = resp.windows(4).position(|w| w == b"\r\n\r\n").ok_or("no response headers")? + 4;
    let status_line = String::from_utf8_lossy(&resp[..end]).lines().next().unwrap_or_default().to_owned();
    let status = status_line.split_whitespace().nth(1).and_then(|c| c.parse().ok()).ok_or("no status code")?;
    Ok(Response { status, body: resp[end..].to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_parse_with_and_without_a_port_and_https_is_refused_with_its_reason() {
        assert_eq!(Url::parse("http://127.0.0.1:7800").unwrap(), Url { host: "127.0.0.1".into(), port: 7800 });
        assert_eq!(Url::parse("http://mm.example.dev/").unwrap(), Url { host: "mm.example.dev".into(), port: 80 });
        assert!(Url::parse("https://mm.example.dev").unwrap_err().contains("HTTPS"));
        assert!(Url::parse("mm.example.dev").is_err(), "a bare host has no scheme");
        assert!(Url::parse("http://host/path").is_err(), "a URL is a service's root");
    }
}
