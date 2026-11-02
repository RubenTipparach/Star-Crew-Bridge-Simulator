//! The WebRTC transport (openspec/changes/coop-drill design 4.2; netcode-and-sessions section 2): `str0m` data
//! channels on our own UDP sockets and threads, and the LAN signalling endpoint that carries the offer and answer.
//!
//! It lives in `sc-net` so the game threads only ever see whole messages on three channels and never block on a
//! socket. Each peer connection runs on its own thread with its own UDP socket (one per client, as `str0m`'s
//! `http-post` example does); the game thread exchanges messages with it through channels. Signalling is one HTTP
//! POST of the client's offer, answered with the server's answer (all candidates inside, no trickle): the LAN form
//! of the matchmaker's exchange, which a WebSocket replaces later.
//!
//! `ServerNet` and `ClientNet` can instead hold the local link (`crate::local`, browser-single-player design 2): a
//! server and its clients in one process, with no socket and no thread. Callers use the same methods either way.

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};

use crate::http::{read_request, respond};
use crate::local::{LocalClient, LocalDoor, LocalServer};
use crate::matchmaker::{self as mm, Heartbeat, Matchmaker, Register};
use sc_core::rng::Rng;
use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
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

/// How long a peer socket waits for each STUN server's answer.
const STUN_WAIT: Duration = Duration::from_millis(400);

/// Offer `socket`'s public address as a server-reflexive candidate too, when a STUN server answers and it differs from
/// the host address (design 10.5). Loopback sockets and an empty list ask nothing.
fn add_reflexive(rtc: &mut Rtc, socket: &UdpSocket, stun: &[String]) {
    let Ok(local) = socket.local_addr() else { return };
    if stun.is_empty() || local.ip().is_loopback() {
        return;
    }
    if let Some(public) = crate::stun::public_addr(socket, stun, STUN_WAIT).filter(|p| *p != local) {
        if let Ok(c) = Candidate::server_reflexive(public, local, "udp") {
            rtc.add_local_candidate(c);
        }
    }
}

/// The address this machine reaches its LAN by (no packet is sent), or loopback when it has none: the host candidate
/// of a server, and of a client that dials through the matchmaker and so does not know the server's address.
pub fn lan_ip() -> IpAddr {
    local_ip_toward(IpAddr::from([192, 168, 0, 1])).unwrap_or(IpAddr::from([127, 0, 0, 1]))
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

/// The server's end: the signalling listener and every peer's outbox, or a local link (`crate::local`).
pub struct ServerNet {
    link: ServerLink,
    /// The signalling address it listens on (`0.0.0.0:0` for a local server, which listens on nothing).
    pub addr: SocketAddr,
}

enum ServerLink {
    Rtc { events: Receiver<NetEvent>, peers: Arc<Mutex<Vec<PeerSlot>>>, status: Arc<Mutex<String>>, door: Acceptor },
    Local(LocalServer),
}

/// What turns a client's offer into a peer: shared by the LAN endpoint (`POST /rtc`) and the matchmaker's relay, so a
/// peer is made one way however its offer arrived (CLAUDE.md 6.1).
#[derive(Clone)]
struct Acceptor {
    tx: Sender<NetEvent>,
    peers: Arc<Mutex<Vec<PeerSlot>>>,
    host_ip: IpAddr,
    impair: Impair,
    seed: u64,
    next_id: Arc<AtomicU32>,
    /// STUN servers asked for each peer socket's public address, once the server is listed (design 10.5).
    stun: Arc<Mutex<Vec<String>>>,
}

impl Acceptor {
    /// Answer `offer` and start its peer; `from` is where the offer came from (`0.0.0.0:0` through the relay).
    fn accept(&self, offer: SdpOffer, from: SocketAddr) -> Result<SdpAnswer, String> {
        let socket = UdpSocket::bind(SocketAddr::new(self.host_ip, 0)).map_err(|e| e.to_string())?;
        let local = socket.local_addr().map_err(|e| e.to_string())?;
        let mut rtc = RtcConfig::new().build(Instant::now());
        let cand = Candidate::host(local, "udp").map_err(|e| e.to_string())?;
        rtc.add_local_candidate(cand);
        let stun = self.stun.lock().expect("stun").clone();
        add_reflexive(&mut rtc, &socket, &stun);
        let answer = rtc.sdp_api().accept_offer(offer).map_err(|e| e.to_string())?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (otx, orx) = mpsc::channel();
        let stats = Arc::new(PeerStats::default());
        self.peers.lock().expect("peers").push(PeerSlot { id, addr: from, outbox: otx, stats: stats.clone() });
        let (tx, impair, seed) = (self.tx.clone(), self.impair, self.seed);
        thread::Builder::new()
            .name(format!("peer-{id}"))
            .spawn(move || run_peer(rtc, socket, id, tx, orx, stats, impair, seed))
            .map_err(|e| e.to_string())?;
        Ok(answer)
    }
}

/// A server's place in the matchmaker's list (matchmaker design 10.3): its code, and the state its heartbeats carry.
pub struct Listed {
    /// The join code.
    pub code: Arc<Mutex<String>>,
    beat: Arc<Mutex<Heartbeat>>,
    stop: Arc<AtomicBool>,
    mm: Matchmaker,
    key: String,
    id: Arc<Mutex<String>>,
}

impl Listed {
    /// The state the next heartbeat carries.
    pub fn set(&self, beat: Heartbeat) {
        *self.beat.lock().expect("beat") = beat;
    }

    /// The code now (it changes only if the matchmaker lost it and gave another).
    pub fn code(&self) -> String {
        self.code.lock().expect("code").clone()
    }
}

impl Drop for Listed {
    /// A clean stop takes the session off the list at once, rather than after its time to live.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let id = self.id.lock().expect("id").clone();
        let _ = self.mm.unlist(&self.key, &id);
    }
}

/// Where a client connects: a server's signalling address, or a server in this process.
#[derive(Clone, Debug)]
pub enum Dial {
    /// A server over WebRTC.
    Addr(SocketAddr),
    /// A server over WebRTC, found by its code through a matchmaker (matchmaker design 10).
    Join(Matchmaker, String),
    /// A server in this process, over the local link.
    Local(LocalDoor),
}

impl From<SocketAddr> for Dial {
    fn from(a: SocketAddr) -> Self {
        Dial::Addr(a)
    }
}

impl std::fmt::Display for Dial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dial::Addr(a) => write!(f, "{a}"),
            Dial::Join(m, code) => write!(f, "code {code} at {}", m.url),
            Dial::Local(_) => f.write_str("the local server"),
        }
    }
}

const NOWHERE: SocketAddr = SocketAddr::V4(std::net::SocketAddrV4::new(std::net::Ipv4Addr::UNSPECIFIED, 0));

impl ServerNet {
    /// Listen for signalling on `bind` (for example `0.0.0.0:7700`); peers get UDP sockets on `host_ip`.
    pub fn start(bind: SocketAddr, host_ip: IpAddr, impair: Impair, seed: u64) -> std::io::Result<Self> {
        install_crypto();
        let listener = TcpListener::bind(bind)?;
        let addr = listener.local_addr()?;
        let (tx, rx) = mpsc::channel();
        let peers: Arc<Mutex<Vec<PeerSlot>>> = Arc::new(Mutex::new(Vec::new()));
        let status = Arc::new(Mutex::new(String::from("{}")));
        let door = Acceptor {
            tx,
            peers: peers.clone(),
            host_ip,
            impair,
            seed,
            next_id: Arc::new(AtomicU32::new(1)),
            stun: Arc::new(Mutex::new(Vec::new())),
        };
        let (door2, status2) = (door.clone(), status.clone());
        thread::Builder::new().name("signalling".into()).spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                if let Err(e) = serve(stream, &door2, &status2) {
                    eprintln!("sc-net: signalling: {e}");
                }
            }
        })?;
        Ok(Self { link: ServerLink::Rtc { events: rx, peers, status, door }, addr })
    }

    /// Register with a matchmaker (matchmaker design 10.3), listed or by code only, and answer the offers it relays
    /// through the same path as the LAN's. Two threads: one heartbeats (and registers again with the same code if the
    /// matchmaker restarted), one long-polls for offers. A local server cannot be listed.
    pub fn list(&self, mm: Matchmaker, key: String, reg: Register) -> Result<Listed, String> {
        let ServerLink::Rtc { door, .. } = &self.link else {
            return Err("a server in this process cannot be listed".into());
        };
        let first = mm.register(&key, &reg)?;
        *door.stun.lock().expect("stun") = mm.stun.clone();
        // Said once, so a host can tell whether players outside its network can be reached (design 10.5).
        if let Ok(probe) = UdpSocket::bind(SocketAddr::new(door.host_ip, 0)) {
            match crate::stun::public_addr(&probe, &mm.stun, STUN_WAIT) {
                Some(a) => println!("sc-net: seen from the internet at {}", a.ip()),
                None if !mm.stun.is_empty() => println!("sc-net: no STUN server answered; only this network can join"),
                None => {}
            }
        }
        let code = Arc::new(Mutex::new(first.code.clone()));
        let id = Arc::new(Mutex::new(first.id.clone()));
        let beat = Arc::new(Mutex::new(Heartbeat::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let every = Duration::from_secs_f64(first.heartbeat_s.clamp(1.0, 60.0));
        {
            let (mm, key, code, id, beat, stop) =
                (mm.clone(), key.clone(), code.clone(), id.clone(), beat.clone(), stop.clone());
            thread::Builder::new()
                .name("matchmaker-heartbeat".into())
                .spawn(move || {
                    let mut wait = every;
                    while !stop.load(Ordering::Relaxed) {
                        thread::sleep(wait);
                        let now = beat.lock().expect("beat").clone();
                        let at = id.lock().expect("id").clone();
                        match mm.heartbeat(&key, &at, &now) {
                            Ok(()) => wait = every,
                            Err((404, _)) => {
                                let mut again = reg.clone();
                                again.previous_code = Some(code.lock().expect("code").clone());
                                match mm.register(&key, &again) {
                                    Ok(r) => {
                                        println!("sc-net: registered with {} again, code {}", mm.url, r.code);
                                        *code.lock().expect("code") = r.code;
                                        *id.lock().expect("id") = r.id;
                                        wait = every;
                                    }
                                    Err(e) => {
                                        eprintln!("sc-net: matchmaker: {e}");
                                        wait = (wait * 2).min(Duration::from_secs(30));
                                    }
                                }
                            }
                            Err((_, e)) => {
                                eprintln!("sc-net: matchmaker heartbeat: {e}");
                                wait = (wait * 2).min(Duration::from_secs(30));
                            }
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        {
            let (mm, key, id, stop, door) = (mm.clone(), key.clone(), id.clone(), stop.clone(), door.clone());
            thread::Builder::new()
                .name("matchmaker-offers".into())
                .spawn(move || {
                    let mut backoff = Duration::from_secs(1);
                    while !stop.load(Ordering::Relaxed) {
                        let at = id.lock().expect("id").clone();
                        match mm.offers(&key, &at) {
                            Ok(o) => {
                                backoff = Duration::from_secs(1);
                                for offer in o.offers {
                                    let reply = serde_json::from_value::<SdpOffer>(offer.offer)
                                        .map_err(|e| format!("bad offer: {e}"))
                                        .and_then(|o| door.accept(o, NOWHERE))
                                        .and_then(|a| serde_json::to_value(a).map_err(|e| e.to_string()));
                                    let answer = match reply {
                                        Ok(a) => mm::Answer { join_id: offer.join_id, answer: Some(a), refused: None },
                                        Err(e) => mm::Answer { join_id: offer.join_id, answer: None, refused: Some(e) },
                                    };
                                    if let Err((_, e)) = mm.answer(&key, &at, &answer) {
                                        eprintln!("sc-net: matchmaker answer: {e}");
                                    }
                                }
                            }
                            Err((_, e)) => {
                                if !stop.load(Ordering::Relaxed) {
                                    eprintln!("sc-net: matchmaker offers: {e}");
                                }
                                thread::sleep(backoff);
                                backoff = (backoff * 2).min(Duration::from_secs(30));
                            }
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(Listed { code, beat, stop, mm, key, id })
    }

    /// A server in this process on the local link (browser-single-player design 2): no socket, no thread.
    pub fn local() -> Self {
        Self { link: ServerLink::Local(LocalServer::new()), addr: NOWHERE }
    }

    /// Where a client connects to reach this server.
    pub fn dial(&self) -> Dial {
        match &self.link {
            ServerLink::Rtc { .. } => Dial::Addr(self.addr),
            ServerLink::Local(l) => Dial::Local(l.door()),
        }
    }

    /// Everything that arrived since the last call.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        match &mut self.link {
            ServerLink::Rtc { events, peers, .. } => {
                let ev: Vec<NetEvent> = events.try_iter().collect();
                for e in &ev {
                    if let NetEvent::Closed(id) = e {
                        peers.lock().expect("peers").retain(|p| p.id != *id);
                    }
                }
                ev
            }
            ServerLink::Local(l) => l.poll(),
        }
    }

    /// Send a message to a peer.
    pub fn send(&self, peer: PeerId, chan: Chan, data: Vec<u8>) {
        match &self.link {
            ServerLink::Rtc { peers, .. } => {
                if let Some(p) = peers.lock().expect("peers").iter().find(|p| p.id == peer) {
                    let _ = p.outbox.send(Outgoing { chan, data });
                }
            }
            ServerLink::Local(l) => l.send(peer, chan, data),
        }
    }

    /// End a peer's connection.
    pub fn close(&mut self, peer: PeerId) {
        match &mut self.link {
            ServerLink::Rtc { peers, .. } => peers.lock().expect("peers").retain(|p| p.id != peer),
            ServerLink::Local(l) => l.close(peer),
        }
    }

    /// Set the JSON the status page serves (a local server serves none).
    pub fn set_status(&self, json: String) {
        if let ServerLink::Rtc { status, .. } = &self.link {
            *status.lock().expect("status") = json;
        }
    }

    /// Each peer's address and counters (`0.0.0.0:0` for a local peer).
    pub fn peer_stats(&self) -> Vec<(PeerId, SocketAddr, Arc<PeerStats>)> {
        match &self.link {
            ServerLink::Rtc { peers, .. } => {
                peers.lock().expect("peers").iter().map(|p| (p.id, p.addr, p.stats.clone())).collect()
            }
            ServerLink::Local(l) => l.peer_stats().into_iter().map(|(id, s)| (id, NOWHERE, s)).collect(),
        }
    }
}

fn serve(mut s: TcpStream, door: &Acceptor, status: &Arc<Mutex<String>>) -> Result<(), String> {
    let req = read_request(&mut s)?;
    let peer_addr = s.peer_addr().map_err(|e| e.to_string())?;
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/status") => {
            let json = status.lock().expect("status").clone();
            respond(&mut s, "200 OK", "application/json", json.as_bytes())
        }
        ("POST", "/rtc") => {
            let offer: SdpOffer = serde_json::from_slice(&req.body).map_err(|e| format!("bad offer: {e}"))?;
            let answer = door.accept(offer, peer_addr)?;
            let body = serde_json::to_vec(&answer).map_err(|e| e.to_string())?;
            respond(&mut s, "200 OK", "application/json", &body)
        }
        _ => respond(&mut s, "404 Not Found", "text/plain", b"not found"),
    }
}

/// A client's end: one connection to a server.
pub struct ClientNet {
    link: ClientLink,
    /// Counters.
    pub stats: Arc<PeerStats>,
}

enum ClientLink {
    Rtc {
        events: Receiver<NetEvent>,
        outbox: Sender<Outgoing>,
        open: Arc<AtomicBool>,
    },
    Local(LocalClient),
    /// The browser's own WebRTC, in the web build (matchmaker design 10.7).
    Web(crate::web::WebLink),
}

/// Why an address cannot be dialled from a page (matchmaker design 10.7).
pub const BROWSER_NEEDS_CODE: &str = "In the browser, join a crew by its code";

impl ClientNet {
    /// Connect to `dial`: over WebRTC to an address ([`ClientNet::connect`]), or over the local link.
    pub fn dial(dial: &Dial, impair: Impair, seed: u64) -> Result<Self, String> {
        let web = cfg!(target_os = "emscripten");
        match dial {
            // A page cannot open UDP sockets: it joins through the matchmaker or not at all.
            Dial::Addr(_) if web => Err(BROWSER_NEEDS_CODE.into()),
            Dial::Join(m, code) if web => Ok(Self {
                link: ClientLink::Web(crate::web::WebLink::join(m, code)?),
                stats: Arc::new(PeerStats::default()),
            }),
            Dial::Addr(a) => Self::connect(*a, impair, seed),
            Dial::Join(m, code) => Self::join(m, code, impair, seed),
            Dial::Local(door) => {
                let c = LocalClient::connect(door)?;
                Ok(Self { stats: c.stats.clone(), link: ClientLink::Local(c) })
            }
        }
    }

    /// Connect to a server's signalling address (`host:7700`). Returns once the answer is in; the channels open a
    /// moment later ([`NetEvent::Open`]).
    pub fn connect(server: SocketAddr, impair: Impair, seed: u64) -> Result<Self, String> {
        let ip = local_ip_toward(server.ip()).map_err(|e| format!("no route to {server}: {e}"))?;
        let (rtc, socket, offer, pending) = Self::offer(ip, &[])?;
        let body = serde_json::to_vec(&offer).map_err(|e| e.to_string())?;
        let answer_bytes = post(server, "/rtc", &body)?;
        let answer: SdpAnswer = serde_json::from_slice(&answer_bytes).map_err(|e| format!("bad answer: {e}"))?;
        Self::finish(rtc, socket, pending, answer, impair, seed)
    }

    /// Join the session holding `code` through `matchmaker` (matchmaker design 10.4): the same offer as
    /// [`ClientNet::connect`], relayed. A refusal (no such code, another protocol, a full or silent server) is the error.
    pub fn join(matchmaker: &Matchmaker, code: &str, impair: Impair, seed: u64) -> Result<Self, String> {
        let (rtc, socket, offer, pending) = Self::offer(lan_ip(), &matchmaker.stun)?;
        let offer = serde_json::to_value(&offer).map_err(|e| e.to_string())?;
        let answer = matchmaker.join(code, &mm::Join { protocol: crate::msg::PROTOCOL, offer })?;
        let answer: SdpAnswer = serde_json::from_value(answer).map_err(|e| format!("bad answer: {e}"))?;
        Self::finish(rtc, socket, pending, answer, impair, seed)
    }

    /// A connection's offer with the three channels, from a socket on `ip`, its public address asked of `stun`.
    fn offer(ip: IpAddr, stun: &[String]) -> Result<(Rtc, UdpSocket, SdpOffer, SdpPendingOffer), String> {
        install_crypto();
        let socket = UdpSocket::bind(SocketAddr::new(ip, 0)).map_err(|e| e.to_string())?;
        let local = socket.local_addr().map_err(|e| e.to_string())?;
        let mut rtc = RtcConfig::new().build(Instant::now());
        rtc.add_local_candidate(Candidate::host(local, "udp").map_err(|e| e.to_string())?);
        add_reflexive(&mut rtc, &socket, stun);
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
        Ok((rtc, socket, offer, pending))
    }

    /// Take the server's answer and start the connection's thread.
    fn finish(
        mut rtc: Rtc,
        socket: UdpSocket,
        pending: SdpPendingOffer,
        answer: SdpAnswer,
        impair: Impair,
        seed: u64,
    ) -> Result<Self, String> {
        rtc.sdp_api().accept_answer(pending, answer).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();
        let (otx, orx) = mpsc::channel();
        let stats = Arc::new(PeerStats::default());
        let s2 = stats.clone();
        thread::Builder::new()
            .name("client-peer".into())
            .spawn(move || run_peer(rtc, socket, 0, tx, orx, s2, impair, seed))
            .map_err(|e| e.to_string())?;
        Ok(Self { link: ClientLink::Rtc { events: rx, outbox: otx, open: Arc::new(AtomicBool::new(false)) }, stats })
    }

    /// Everything that arrived since the last call.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        match &mut self.link {
            ClientLink::Rtc { events, open, .. } => {
                let ev: Vec<NetEvent> = events.try_iter().collect();
                for e in &ev {
                    match e {
                        NetEvent::Open(_) => open.store(true, Ordering::Relaxed),
                        NetEvent::Closed(_) => open.store(false, Ordering::Relaxed),
                        NetEvent::Data(..) => {}
                    }
                }
                ev
            }
            ClientLink::Local(l) => l.poll(),
            ClientLink::Web(w) => w.poll(),
        }
    }

    /// Why the connection closed, when the link knows (the browser's: a refused code, no path to the crew).
    pub fn reason(&self) -> Option<String> {
        match &self.link {
            ClientLink::Web(w) => w.reason().map(str::to_owned),
            ClientLink::Rtc { .. } | ClientLink::Local(_) => None,
        }
    }

    /// Whether the channels are open.
    pub fn is_open(&self) -> bool {
        match &self.link {
            ClientLink::Rtc { open, .. } => open.load(Ordering::Relaxed),
            ClientLink::Local(l) => l.is_open(),
            ClientLink::Web(w) => w.is_open(),
        }
    }

    /// Send a message.
    pub fn send(&self, chan: Chan, data: Vec<u8>) {
        match &self.link {
            ClientLink::Rtc { outbox, .. } => {
                let _ = outbox.send(Outgoing { chan, data });
            }
            ClientLink::Local(l) => l.send(chan, data),
            ClientLink::Web(w) => w.send(chan, &data),
        }
    }
}

/// An HTTP POST on the LAN signalling endpoint; the response's body.
pub fn post(server: SocketAddr, path: &str, body: &[u8]) -> Result<Vec<u8>, String> {
    let url = crate::http::Url { host: server.ip().to_string(), port: server.port() };
    let r = crate::http::call(&url, "POST", path, &[], body, Duration::from_secs(10))?;
    if r.status != 200 {
        return Err(format!("signalling refused: HTTP {}", r.status));
    }
    Ok(r.body)
}

/// An HTTP GET; the response's body (the status page).
pub fn get(server: SocketAddr, path: &str) -> Result<Vec<u8>, String> {
    let url = crate::http::Url { host: server.ip().to_string(), port: server.port() };
    Ok(crate::http::call(&url, "GET", path, &[], &[], Duration::from_secs(5))?.body)
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
