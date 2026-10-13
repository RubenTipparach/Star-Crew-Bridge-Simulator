//! The local link (openspec/changes/browser-single-player design 2): a server and its clients in one process, with
//! no socket and no thread, carrying the same three channels and the same messages as the WebRTC transport.
//!
//! It lives in `sc-net` beside the WebRTC transport because it is the other half of the same seam: `ServerNet` and
//! `ClientNet` hold one or the other, and `DrillServer` and `Session` never learn which. Single player hosts on it
//! (in a browser page, which has neither sockets nor threads, and on the desktop alike). Every message is
//! delivered, in the order it was sent, on the receiver's next poll: nothing can be lost inside a process.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::transport::{Chan, NetEvent, PeerId, PeerStats};

/// One client's two queues and who has seen what.
#[derive(Debug, Default)]
struct Pipe {
    to_server: VecDeque<(Chan, Vec<u8>)>,
    to_client: VecDeque<(Chan, Vec<u8>)>,
    /// The server has been told it opened.
    server_saw_open: bool,
    /// The client has been told it opened.
    client_saw_open: bool,
    /// The client end was dropped.
    client_gone: bool,
    /// The server closed this peer, or was dropped.
    server_gone: bool,
}

#[derive(Debug, Default)]
struct Hub {
    /// The server end is gone: no one can connect, every client hears it closed.
    server_dropped: bool,
    next_id: PeerId,
    /// By peer id, so the server hears its peers in the order they connected.
    pipes: BTreeMap<PeerId, Pipe>,
}

fn lock(hub: &Mutex<Hub>) -> MutexGuard<'_, Hub> {
    // A panic while the lock was held leaves queues that are still whole: carry on with them.
    hub.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What a client connects through to reach a local server: cheap to clone, and nothing else.
#[derive(Clone, Debug)]
pub struct LocalDoor(Arc<Mutex<Hub>>);

/// The server's end of the local link.
#[derive(Debug)]
pub struct LocalServer {
    hub: Arc<Mutex<Hub>>,
    stats: BTreeMap<PeerId, Arc<PeerStats>>,
}

impl LocalServer {
    /// A local server with no clients yet.
    pub fn new() -> Self {
        Self { hub: Arc::new(Mutex::new(Hub { next_id: 1, ..Hub::default() })), stats: BTreeMap::new() }
    }

    /// The door clients connect through.
    pub fn door(&self) -> LocalDoor {
        LocalDoor(self.hub.clone())
    }

    /// Everything that arrived since the last call: each new peer's `Open` before its data, and a peer whose client
    /// end was dropped as `Closed` after whatever it sent first.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        let mut hub = lock(&self.hub);
        let mut gone = Vec::new();
        for (id, p) in hub.pipes.iter_mut() {
            if p.server_gone {
                continue;
            }
            if !p.server_saw_open {
                p.server_saw_open = true;
                out.push(NetEvent::Open(*id));
            }
            let stats = self.stats.entry(*id).or_default();
            for (chan, data) in p.to_server.drain(..) {
                stats.bytes_in.fetch_add(data.len() as u64, Ordering::Relaxed);
                out.push(NetEvent::Data(*id, chan, data));
            }
            if p.client_gone {
                out.push(NetEvent::Closed(*id));
                gone.push(*id);
            }
        }
        for id in gone {
            hub.pipes.remove(&id);
            self.stats.remove(&id);
        }
        out
    }

    /// Send a message to a peer; nothing when it is gone.
    pub fn send(&self, peer: PeerId, chan: Chan, data: Vec<u8>) {
        let mut hub = lock(&self.hub);
        if let Some(p) = hub.pipes.get_mut(&peer).filter(|p| !p.server_gone && !p.client_gone) {
            if let Some(s) = self.stats.get(&peer) {
                s.bytes_out.fetch_add(data.len() as u64, Ordering::Relaxed);
            }
            p.to_client.push_back((chan, data));
        }
    }

    /// End a peer's connection: its client hears `Closed`.
    pub fn close(&mut self, peer: PeerId) {
        if let Some(p) = lock(&self.hub).pipes.get_mut(&peer) {
            p.server_gone = true;
            p.to_client.clear();
        }
        self.stats.remove(&peer);
    }

    /// Each open peer's counters.
    pub fn peer_stats(&self) -> Vec<(PeerId, Arc<PeerStats>)> {
        self.stats.iter().map(|(id, s)| (*id, s.clone())).collect()
    }
}

impl Default for LocalServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        let mut hub = lock(&self.hub);
        hub.server_dropped = true;
        for p in hub.pipes.values_mut() {
            p.server_gone = true;
        }
    }
}

/// A client's end of the local link.
#[derive(Debug)]
pub struct LocalClient {
    hub: Arc<Mutex<Hub>>,
    id: PeerId,
    /// Counters, as a WebRTC client keeps them.
    pub stats: Arc<PeerStats>,
    open: bool,
}

impl LocalClient {
    /// Connect through `door`. Fails only when the server is gone.
    pub fn connect(door: &LocalDoor) -> Result<Self, String> {
        let mut hub = lock(&door.0);
        if hub.server_dropped {
            return Err("the local server has stopped".into());
        }
        let id = hub.next_id;
        hub.next_id += 1;
        hub.pipes.insert(id, Pipe::default());
        Ok(Self { hub: door.0.clone(), id, stats: Arc::new(PeerStats::default()), open: false })
    }

    /// Everything that arrived since the last call: `Open` first, then the server's messages in order, then
    /// `Closed` once the server closed this peer or stopped.
    pub fn poll(&mut self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        let mut hub = lock(&self.hub);
        let Some(p) = hub.pipes.get_mut(&self.id) else {
            if self.open {
                self.open = false;
                out.push(NetEvent::Closed(0));
            }
            return out;
        };
        if !p.client_saw_open && !p.server_gone {
            p.client_saw_open = true;
            self.open = true;
            out.push(NetEvent::Open(0));
        }
        for (chan, data) in p.to_client.drain(..) {
            self.stats.bytes_in.fetch_add(data.len() as u64, Ordering::Relaxed);
            out.push(NetEvent::Data(0, chan, data));
        }
        if p.server_gone && (self.open || !p.client_saw_open) {
            p.client_saw_open = true;
            self.open = false;
            out.push(NetEvent::Closed(0));
        }
        out
    }

    /// Whether the connection is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Send a message; nothing once the server has closed this peer.
    pub fn send(&self, chan: Chan, data: Vec<u8>) {
        let mut hub = lock(&self.hub);
        if let Some(p) = hub.pipes.get_mut(&self.id).filter(|p| !p.server_gone) {
            self.stats.bytes_out.fetch_add(data.len() as u64, Ordering::Relaxed);
            p.to_server.push_back((chan, data));
        }
    }
}

impl Drop for LocalClient {
    fn drop(&mut self) {
        let mut hub = lock(&self.hub);
        let server_gone = match hub.pipes.get_mut(&self.id) {
            Some(p) => {
                p.client_gone = true;
                p.server_gone
            }
            None => false,
        };
        // A pipe the server has already closed is heard by no one: drop it now.
        if server_gone {
            hub.pipes.remove(&self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(ev: Vec<NetEvent>) -> Vec<(Chan, Vec<u8>)> {
        ev.into_iter()
            .filter_map(|e| match e {
                NetEvent::Data(_, c, d) => Some((c, d)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_client_and_a_server_open_and_exchange_messages_on_all_three_channels_in_order() {
        let mut server = LocalServer::new();
        let mut client = LocalClient::connect(&server.door()).unwrap();
        client.send(Chan::Command, b"hello".to_vec());
        client.send(Chan::Input, vec![1]);
        client.send(Chan::Input, vec![2]);
        let ev = server.poll();
        let peer = match ev.first() {
            Some(NetEvent::Open(p)) => *p,
            other => panic!("the server hears the peer open before its data, got {other:?}"),
        };
        assert_eq!(
            data(ev),
            vec![(Chan::Command, b"hello".to_vec()), (Chan::Input, vec![1]), (Chan::Input, vec![2])],
            "every message arrives, in the order it was sent"
        );
        server.send(peer, Chan::Snapshot, vec![7; 900]);
        server.send(peer, Chan::Command, b"welcome".to_vec());
        let ev = client.poll();
        assert!(matches!(ev.first(), Some(NetEvent::Open(_))), "the client hears it open first");
        assert!(client.is_open());
        assert_eq!(data(ev), vec![(Chan::Snapshot, vec![7; 900]), (Chan::Command, b"welcome".to_vec())]);
        assert_eq!(client.stats.bytes_in.load(Ordering::Relaxed), 907);
    }

    #[test]
    fn a_dropped_client_is_heard_closed_after_what_it_sent() {
        let mut server = LocalServer::new();
        let client = LocalClient::connect(&server.door()).unwrap();
        server.poll();
        client.send(Chan::Command, b"bye".to_vec());
        drop(client);
        let ev = server.poll();
        assert!(matches!(ev.as_slice(), [NetEvent::Data(..), NetEvent::Closed(_)]), "got {ev:?}");
        assert!(server.poll().is_empty(), "a closed peer is heard once");
        assert!(server.peer_stats().is_empty());
    }

    #[test]
    fn a_closed_or_stopped_server_is_heard_closed_by_its_clients() {
        let mut server = LocalServer::new();
        let door = server.door();
        let mut a = LocalClient::connect(&door).unwrap();
        let mut b = LocalClient::connect(&door).unwrap();
        let ev = server.poll();
        let first = match ev[0] {
            NetEvent::Open(p) => p,
            _ => unreachable!(),
        };
        a.poll();
        b.poll();
        server.close(first);
        assert!(matches!(a.poll().as_slice(), [NetEvent::Closed(_)]));
        assert!(!a.is_open());
        drop(server);
        assert!(matches!(b.poll().as_slice(), [NetEvent::Closed(_)]));
        assert!(LocalClient::connect(&door).is_err(), "no one connects to a stopped server");
    }
}
