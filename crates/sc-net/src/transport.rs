//! The WebRTC transport (openspec/changes/coop-drill design 4.2; netcode-and-sessions section 2): `str0m` data
//! channels on our own UDP sockets and threads, and the LAN signalling endpoint that carries the offer and answer.
//!
//! It lives in `sc-net` so the game threads only ever see whole messages on three channels and never block on a
//! socket. Each peer connection runs on its own thread with its own UDP socket (one per client, as `str0m`'s
//! `http-post` example does); the game thread exchanges messages with it through channels. Signalling is one HTTP
//! POST of the client's offer, answered with the server's answer (all candidates inside, no trickle): the LAN form
//! of the matchmaker's exchange, which a WebSocket replaces later.

use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};

use sc_core::rng::Rng;
use str0m::change::{SdpAnswer, SdpOffer};
use str0m::channel::{ChannelConfig, ChannelId, Reliability};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};

/// The three channels (design 4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Chan {
    /// Reliable, ordered: commands and events.
    Command = 0,
    /// Unordered, no retransmit: the helm's stick.
    Input = 1,
    /// Unordered, no retransmit: snapshots.
    Snapshot = 2,
}

const LABELS: [&str; 3] = ["command", "input", "snapshot"];

impl Chan {
    fn from_label(l: &str) -> Option<Self> {
        LABELS.iter().position(|x| *x == l).map(|i| [Chan::Command, Chan::Input, Chan::Snapshot][i])
    }
    fn reliable(self) -> bool {
        self == Chan::Command
    }
}

/// A peer's id on the server: stable for the connection's life.
pub type PeerId = u32;

/// What the game thread hears from the network.
#[derive(Debug)]
pub enum NetEvent {
    /// All three channels are open.
    Open(PeerId),
    /// The connection ended.
    Closed(PeerId),
    /// A whole message.
    Data(PeerId, Chan, Vec<u8>),
}

/// A test impairment on the unreliable channels, applied in both directions (design 4.2: `--loss`, `--delay-ms`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Impair {
    /// Share of unreliable messages dropped (0-1).
    pub loss: f64,
    /// Delay added to every unreliable message, in milliseconds.
    pub delay_ms: u64,
}

/// Counters for one peer, read by the status page.
#[derive(Debug, Default)]
pub struct PeerStats {
    /// Bytes of messages received.
    pub bytes_in: AtomicU64,
    /// Bytes of messages sent.
    pub bytes_out: AtomicU64,
    /// UDP datagrams' bytes received, with every header.
    pub wire_in: AtomicU64,
    /// UDP datagrams' bytes sent, with every header.
    pub wire_out: AtomicU64,
    /// Messages a full send buffer refused.
    pub refused_writes: AtomicU64,
    /// Messages the impairment dropped.
    pub impaired: AtomicU64,
}

static CRYPTO: Once = Once::new();

fn install_crypto() {
    CRYPTO.call_once(|| str0m::crypto::from_feature_flags().install_process_default());
}

/// The address this machine uses to reach `toward` (no packet is sent): the host candidate to offer.
pub fn local_ip_toward(toward: IpAddr) -> std::io::Result<IpAddr> {
    let s = UdpSocket::bind(if toward.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" })?;
    s.connect(SocketAddr::new(toward, 9))?;
    Ok(s.local_addr()?.ip())
}

struct Outgoing {
    chan: Chan,
    data: Vec<u8>,
}

/// Run one peer connection until it closes.
#[allow(clippy::too_many_arguments)]
fn run_peer(
    mut rtc: Rtc,
    socket: UdpSocket,
    id: PeerId,
    events: Sender<NetEvent>,
    outbox: Receiver<Outgoing>,
    stats: Arc<PeerStats>,
    impair: Impair,
    seed: u64,
) {
    let mut chans: [Option<ChannelId>; 3] = [None; 3];
    let mut open_sent = false;
    let mut rng = Rng::for_purpose(seed, u64::from(id), "impair");
    let delay = Duration::from_millis(impair.delay_ms);
    let mut delayed_in: VecDeque<(Instant, Chan, Vec<u8>)> = VecDeque::new();
    let mut delayed_out: VecDeque<(Instant, Chan, Vec<u8>)> = VecDeque::new();
    let mut buf = vec![0u8; 2048];
    let local = socket.local_addr().expect("a bound socket has an address");
    let start = Instant::now();
    let mut closed = false;
    let drop_it = |rng: &mut Rng| impair.loss > 0.0 && rng.next_f64() < impair.loss;
    while !closed {
        // Messages from the game thread, through the impairment for the unreliable channels.
        loop {
            match outbox.try_recv() {
                Ok(o) => {
                    if !o.chan.reliable() && drop_it(&mut rng) {
                        stats.impaired.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    if !o.chan.reliable() && impair.delay_ms > 0 {
                        delayed_out.push_back((Instant::now() + delay, o.chan, o.data));
                    } else {
                        write(&mut rtc, &chans, o.chan, &o.data, &stats);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    rtc.disconnect();
                    closed = true;
                    break;
                }
            }
        }
        let now = Instant::now();
        while delayed_out.front().is_some_and(|d| d.0 <= now) {
            let (_, c, d) = delayed_out.pop_front().expect("checked");
            write(&mut rtc, &chans, c, &d, &stats);
        }
        while delayed_in.front().is_some_and(|d| d.0 <= now) {
            let (_, c, d) = delayed_in.pop_front().expect("checked");
            let _ = events.send(NetEvent::Data(id, c, d));
        }
        // Drive the connection until it wants input.
        let timeout = loop {
            match rtc.poll_output() {
                Ok(Output::Timeout(t)) => break t,
                Ok(Output::Transmit(t)) => {
                    stats.wire_out.fetch_add(t.contents.len() as u64, Ordering::Relaxed);
                    let _ = socket.send_to(&t.contents, t.destination);
                }
                Ok(Output::Event(e)) => match e {
                    Event::ChannelOpen(cid, label) => {
                        if let Some(c) = Chan::from_label(&label) {
                            chans[c as usize] = Some(cid);
                        }
                        if !open_sent && chans.iter().all(Option::is_some) {
                            open_sent = true;
                            let _ = events.send(NetEvent::Open(id));
                        }
                    }
                    Event::ChannelData(d) => {
                        let Some(c) = chans.iter().position(|x| *x == Some(d.id)) else { continue };
                        let chan = [Chan::Command, Chan::Input, Chan::Snapshot][c];
                        stats.bytes_in.fetch_add(d.data.len() as u64, Ordering::Relaxed);
                        if !chan.reliable() && drop_it(&mut rng) {
                            stats.impaired.fetch_add(1, Ordering::Relaxed);
                        } else if !chan.reliable() && impair.delay_ms > 0 {
                            delayed_in.push_back((Instant::now() + delay, chan, d.data));
                        } else {
                            let _ = events.send(NetEvent::Data(id, chan, d.data));
                        }
                    }
                    Event::ChannelClose(_) => closed = true,
                    Event::IceConnectionStateChange(IceConnectionState::Disconnected) => closed = true,
                    _ => {}
                },
                Err(_) => {
                    closed = true;
                    break Instant::now();
                }
            }
            if closed {
                break Instant::now();
            }
        };
        if closed {
            break;
        }
        // Give up a connection that never opened.
        if !open_sent && start.elapsed() > Duration::from_secs(15) {
            break;
        }
        // Wait for a datagram, at most 2 ms, so the outbox stays prompt.
        let wait = timeout.saturating_duration_since(Instant::now()).min(Duration::from_millis(2));
        let input = if wait.is_zero() {
            Input::Timeout(Instant::now())
        } else {
            let _ = socket.set_read_timeout(Some(wait));
            match socket.recv_from(&mut buf) {
                Ok((n, source)) => {
                    stats.wire_in.fetch_add(n as u64, Ordering::Relaxed);
                    match buf[..n].try_into() {
                        Ok(contents) => Input::Receive(
                            Instant::now(),
                            Receive { proto: Protocol::Udp, source, destination: local, contents },
                        ),
                        Err(_) => continue,
                    }
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    Input::Timeout(Instant::now())
                }
                Err(_) => Input::Timeout(Instant::now()),
            }
        };
        if rtc.handle_input(input).is_err() {
            break;
        }
    }
    let _ = events.send(NetEvent::Closed(id));
}

fn write(rtc: &mut Rtc, chans: &[Option<ChannelId>; 3], chan: Chan, data: &[u8], stats: &PeerStats) {
    let Some(cid) = chans[chan as usize] else {
        stats.refused_writes.fetch_add(1, Ordering::Relaxed);
        return;
    };
    match rtc.channel(cid).map(|mut c| c.write(true, data)) {
        Some(Ok(true)) => {
            stats.bytes_out.fetch_add(data.len() as u64, Ordering::Relaxed);
        }
        _ => {
            stats.refused_writes.fetch_add(1, Ordering::Relaxed);
        }
    }
}

struct PeerSlot {
    id: PeerId,
    addr: SocketAddr,
    outbox: Sender<Outgoing>,
    stats: Arc<PeerStats>,
}

/// The server's end: the signalling listener, and every peer's outbox.
pub struct ServerNet {
    events: Receiver<NetEvent>,
    peers: Arc<Mutex<Vec<PeerSlot>>>,
    status: Arc<Mutex<String>>,
    /// The signalling address it listens on.
    pub addr: SocketAddr,
}

impl ServerNet {
    /// Listen for signalling on `bind` (for example `0.0.0.0:7700`); peers get UDP sockets on `host_ip`.
    pub fn start(bind: SocketAddr, host_ip: IpAddr, impair: Impair, seed: u64) -> std::io::Result<Self> {
        install_crypto();
        let listener = TcpListener::bind(bind)?;
        let addr = listener.local_addr()?;
        let (tx, rx) = mpsc::channel();
        let peers: Arc<Mutex<Vec<PeerSlot>>> = Arc::new(Mutex::new(Vec::new()));
        let status = Arc::new(Mutex::new(String::from("{}")));
        let (peers2, status2) = (peers.clone(), status.clone());
        thread::Builder::new().name("signalling".into()).spawn(move || {
            let mut next_id: PeerId = 1;
            for stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                if let Err(e) = serve(stream, &tx, &peers2, &status2, host_ip, impair, seed, &mut next_id) {
                    eprintln!("sc-net: signalling: {e}");
                }
            }
        })?;
        Ok(Self { events: rx, peers, status, addr })
    }

    /// Everything that arrived since the last call.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        let ev: Vec<NetEvent> = self.events.try_iter().collect();
        for e in &ev {
            if let NetEvent::Closed(id) = e {
                self.peers.lock().expect("peers").retain(|p| p.id != *id);
            }
        }
        ev
    }

    /// Send a message to a peer.
    pub fn send(&self, peer: PeerId, chan: Chan, data: Vec<u8>) {
        if let Some(p) = self.peers.lock().expect("peers").iter().find(|p| p.id == peer) {
            let _ = p.outbox.send(Outgoing { chan, data });
        }
    }

    /// End a peer's connection.
    pub fn close(&self, peer: PeerId) {
        self.peers.lock().expect("peers").retain(|p| p.id != peer);
    }

    /// Set the JSON the status page serves.
    pub fn set_status(&self, json: String) {
        *self.status.lock().expect("status") = json;
    }

    /// Each peer's address and counters.
    pub fn peer_stats(&self) -> Vec<(PeerId, SocketAddr, Arc<PeerStats>)> {
        self.peers.lock().expect("peers").iter().map(|p| (p.id, p.addr, p.stats.clone())).collect()
    }
}

#[allow(clippy::too_many_arguments)]
fn serve(
    mut s: TcpStream,
    tx: &Sender<NetEvent>,
    peers: &Arc<Mutex<Vec<PeerSlot>>>,
    status: &Arc<Mutex<String>>,
    host_ip: IpAddr,
    impair: Impair,
    seed: u64,
    next_id: &mut PeerId,
) -> Result<(), String> {
    let (method, path, body) = read_http(&mut s)?;
    let peer_addr = s.peer_addr().map_err(|e| e.to_string())?;
    match (method.as_str(), path.as_str()) {
        ("GET", "/status") => {
            let json = status.lock().expect("status").clone();
            respond(&mut s, "200 OK", "application/json", json.as_bytes())
        }
        ("POST", "/rtc") => {
            let offer: SdpOffer = serde_json::from_slice(&body).map_err(|e| format!("bad offer: {e}"))?;
            let socket = UdpSocket::bind(SocketAddr::new(host_ip, 0)).map_err(|e| e.to_string())?;
            let local = socket.local_addr().map_err(|e| e.to_string())?;
            let mut rtc = RtcConfig::new().build(Instant::now());
            let cand = Candidate::host(local, "udp").map_err(|e| e.to_string())?;
            rtc.add_local_candidate(cand);
            let answer = rtc.sdp_api().accept_offer(offer).map_err(|e| e.to_string())?;
            let id = *next_id;
            *next_id += 1;
            let (otx, orx) = mpsc::channel();
            let stats = Arc::new(PeerStats::default());
            peers.lock().expect("peers").push(PeerSlot { id, addr: peer_addr, outbox: otx, stats: stats.clone() });
            let tx = tx.clone();
            thread::Builder::new()
                .name(format!("peer-{id}"))
                .spawn(move || run_peer(rtc, socket, id, tx, orx, stats, impair, seed))
                .map_err(|e| e.to_string())?;
            let body = serde_json::to_vec(&answer).map_err(|e| e.to_string())?;
            respond(&mut s, "200 OK", "application/json", &body)
        }
        _ => respond(&mut s, "404 Not Found", "text/plain", b"not found"),
    }
}

fn read_http(s: &mut TcpStream) -> Result<(String, String, Vec<u8>), String> {
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
        if data.len() > 64 * 1024 {
            return Err("headers too long".into());
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let mut lines = head.lines();
    let first = lines.next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();
    let len: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    if len > 256 * 1024 {
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
    Ok((method, path, body))
}

fn respond(s: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| e.to_string())
}

/// A client's end: one connection to a server.
pub struct ClientNet {
    events: Receiver<NetEvent>,
    outbox: Sender<Outgoing>,
    /// Counters.
    pub stats: Arc<PeerStats>,
    open: Arc<AtomicBool>,
}

impl ClientNet {
    /// Connect to a server's signalling address (`host:7700`). Returns once the answer is in; the channels open a
    /// moment later ([`NetEvent::Open`]).
    pub fn connect(server: SocketAddr, impair: Impair, seed: u64) -> Result<Self, String> {
        install_crypto();
        let ip = local_ip_toward(server.ip()).map_err(|e| format!("no route to {server}: {e}"))?;
        let socket = UdpSocket::bind(SocketAddr::new(ip, 0)).map_err(|e| e.to_string())?;
        let local = socket.local_addr().map_err(|e| e.to_string())?;
        let mut rtc = RtcConfig::new().build(Instant::now());
        rtc.add_local_candidate(Candidate::host(local, "udp").map_err(|e| e.to_string())?);
        let mut api = rtc.sdp_api();
        for (i, label) in LABELS.iter().enumerate() {
            let unreliable = i != Chan::Command as usize;
            api.add_channel_with_config(ChannelConfig {
                label: (*label).into(),
                ordered: !unreliable,
                reliability: if unreliable {
                    Reliability::MaxRetransmits { retransmits: 0 }
                } else {
                    Reliability::Reliable
                },
                negotiated: None,
                protocol: String::new(),
            });
        }
        let (offer, pending) = api.apply().ok_or("no offer to make")?;
        let body = serde_json::to_vec(&offer).map_err(|e| e.to_string())?;
        let answer_bytes = post(server, "/rtc", &body)?;
        let answer: SdpAnswer = serde_json::from_slice(&answer_bytes).map_err(|e| format!("bad answer: {e}"))?;
        rtc.sdp_api().accept_answer(pending, answer).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();
        let (otx, orx) = mpsc::channel();
        let stats = Arc::new(PeerStats::default());
        let s2 = stats.clone();
        thread::Builder::new()
            .name("client-peer".into())
            .spawn(move || run_peer(rtc, socket, 0, tx, orx, s2, impair, seed))
            .map_err(|e| e.to_string())?;
        Ok(Self { events: rx, outbox: otx, stats, open: Arc::new(AtomicBool::new(false)) })
    }

    /// Everything that arrived since the last call.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        let ev: Vec<NetEvent> = self.events.try_iter().collect();
        for e in &ev {
            match e {
                NetEvent::Open(_) => self.open.store(true, Ordering::Relaxed),
                NetEvent::Closed(_) => self.open.store(false, Ordering::Relaxed),
                NetEvent::Data(..) => {}
            }
        }
        ev
    }

    /// Whether the channels are open.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Relaxed)
    }

    /// Send a message.
    pub fn send(&self, chan: Chan, data: Vec<u8>) {
        let _ = self.outbox.send(Outgoing { chan, data });
    }
}

/// An HTTP POST on the LAN signalling endpoint; the response's body.
pub fn post(server: SocketAddr, path: &str, body: &[u8]) -> Result<Vec<u8>, String> {
    let mut s = TcpStream::connect_timeout(&server, Duration::from_secs(5))
        .map_err(|e| format!("cannot reach {server}: {e}"))?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
    let head = format!("POST {path} HTTP/1.1\r\nHost: {server}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| e.to_string())?;
    let mut resp = Vec::new();
    s.read_to_end(&mut resp).map_err(|e| e.to_string())?;
    let end = resp.windows(4).position(|w| w == b"\r\n\r\n").ok_or("no response headers")? + 4;
    let status = String::from_utf8_lossy(&resp[..end]);
    if !status.starts_with("HTTP/1.1 200") {
        return Err(format!("signalling refused: {}", status.lines().next().unwrap_or_default()));
    }
    Ok(resp[end..].to_vec())
}

/// An HTTP GET; the response's body (the status page).
pub fn get(server: SocketAddr, path: &str) -> Result<Vec<u8>, String> {
    let mut s = TcpStream::connect_timeout(&server, Duration::from_secs(5)).map_err(|e| e.to_string())?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: {server}\r\nConnection: close\r\n\r\n").as_bytes())
        .map_err(|e| e.to_string())?;
    let mut resp = Vec::new();
    s.read_to_end(&mut resp).map_err(|e| e.to_string())?;
    let end = resp.windows(4).position(|w| w == b"\r\n\r\n").ok_or("no response headers")? + 4;
    Ok(resp[end..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for<F: FnMut() -> bool>(mut f: F, secs: u64) -> bool {
        let end = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < end {
            if f() {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn a_client_and_a_server_open_three_channels_over_loopback_and_exchange_messages() {
        let lo: IpAddr = "127.0.0.1".parse().unwrap();
        let mut server = ServerNet::start(SocketAddr::new(lo, 0), lo, Impair::default(), 1).unwrap();
        let mut client = ClientNet::connect(server.addr, Impair::default(), 2).unwrap();
        let mut peer = None;
        assert!(
            wait_for(
                || {
                    for e in server.poll() {
                        if let NetEvent::Open(p) = e {
                            peer = Some(p);
                        }
                    }
                    client.poll();
                    peer.is_some()
                },
                10
            ),
            "the server sees the channels open"
        );
        let peer = peer.unwrap();
        assert!(
            wait_for(
                || {
                    client.poll();
                    client.is_open()
                },
                10
            ),
            "the client sees them open"
        );
        client.send(Chan::Command, b"hello".to_vec());
        server.send(peer, Chan::Snapshot, vec![7; 900]);
        let (mut got_cmd, mut got_snap) = (false, false);
        assert!(wait_for(
            || {
                for e in server.poll() {
                    if let NetEvent::Data(_, Chan::Command, d) = e {
                        got_cmd = d == b"hello";
                    }
                }
                for e in client.poll() {
                    if let NetEvent::Data(_, Chan::Snapshot, d) = e {
                        got_snap = d.len() == 900;
                    }
                }
                got_cmd && got_snap
            },
            10
        ));
    }
}
