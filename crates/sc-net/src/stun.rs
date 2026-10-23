//! STUN (RFC 8489), just enough to learn the public address a UDP socket is seen at from the internet (matchmaker
//! design 4 and 10.5): one Binding request, one answer, its XOR-MAPPED-ADDRESS. A peer socket asks before its offer
//! or answer is made and offers that address as a server-reflexive candidate beside its host one, so two ends behind
//! home routers can punch through to each other. The STUN servers are data (`data/matchmaker/client.json`).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

const MAGIC: u32 = 0x2112_A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_SUCCESS: u16 = 0x0101;
const XOR_MAPPED_ADDRESS: u16 = 0x0020;
const MAPPED_ADDRESS: u16 = 0x0001;

/// A Binding request with transaction id `tx`.
fn request(tx: [u8; 12]) -> [u8; 20] {
    let mut m = [0u8; 20];
    m[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    m[4..8].copy_from_slice(&MAGIC.to_be_bytes());
    m[8..20].copy_from_slice(&tx);
    m
}

/// The mapped address in a Binding success answering `tx`, if `b` is one.
pub fn parse_response(b: &[u8], tx: [u8; 12]) -> Option<SocketAddr> {
    if b.len() < 20 || u16::from_be_bytes([b[0], b[1]]) != BINDING_SUCCESS || b[4..8] != MAGIC.to_be_bytes() {
        return None;
    }
    if b[8..20] != tx {
        return None;
    }
    let len = usize::from(u16::from_be_bytes([b[2], b[3]]));
    let body = b.get(20..20 + len)?;
    let (mut at, mut plain) = (0, None);
    while at + 4 <= body.len() {
        let kind = u16::from_be_bytes([body[at], body[at + 1]]);
        let n = usize::from(u16::from_be_bytes([body[at + 2], body[at + 3]]));
        let v = body.get(at + 4..at + 4 + n)?;
        if (kind == XOR_MAPPED_ADDRESS || kind == MAPPED_ADDRESS) && v.len() >= 8 {
            let xor = kind == XOR_MAPPED_ADDRESS;
            let port = u16::from_be_bytes([v[2], v[3]]) ^ if xor { (MAGIC >> 16) as u16 } else { 0 };
            let ip = match v[1] {
                1 => {
                    let raw = u32::from_be_bytes([v[4], v[5], v[6], v[7]]) ^ if xor { MAGIC } else { 0 };
                    IpAddr::V4(Ipv4Addr::from(raw))
                }
                2 if v.len() >= 20 => {
                    let mut raw = [0u8; 16];
                    raw.copy_from_slice(&v[4..20]);
                    if xor {
                        let mut key = [0u8; 16];
                        key[..4].copy_from_slice(&MAGIC.to_be_bytes());
                        key[4..].copy_from_slice(&tx);
                        for (r, k) in raw.iter_mut().zip(key) {
                            *r ^= k;
                        }
                    }
                    IpAddr::V6(Ipv6Addr::from(raw))
                }
                _ => return None,
            };
            let addr = SocketAddr::new(ip, port);
            if xor {
                return Some(addr);
            }
            plain = Some(addr);
        }
        // Attributes are padded to four bytes.
        at += 4 + n.div_ceil(4) * 4;
    }
    plain
}

/// Ask `servers` in turn (`host:port`) where `socket` is seen from the internet; the first answer within `wait`
/// for each. None when no server answered (no internet, or UDP blocked): the host candidate alone is offered.
pub fn public_addr(socket: &UdpSocket, servers: &[String], wait: Duration) -> Option<SocketAddr> {
    let local = socket.local_addr().ok()?;
    let before = socket.read_timeout().ok().flatten();
    let mut found = None;
    let mut buf = [0u8; 576];
    for server in servers {
        let Some(to) = server.to_socket_addrs().ok().and_then(|mut a| a.find(|a| a.is_ipv4() == local.is_ipv4()))
        else {
            continue;
        };
        let mut tx = [0u8; 12];
        if getrandom::getrandom(&mut tx).is_err() {
            return None;
        }
        if socket.send_to(&request(tx), to).is_err() {
            continue;
        }
        let end = Instant::now() + wait;
        while let Some(left) = end.checked_duration_since(Instant::now()).filter(|d| !d.is_zero()) {
            let _ = socket.set_read_timeout(Some(left));
            match socket.recv_from(&mut buf) {
                Ok((n, from)) if from == to => {
                    if let Some(a) = parse_response(&buf[..n], tx) {
                        found = Some(a);
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        if found.is_some() {
            break;
        }
    }
    let _ = socket.set_read_timeout(before);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binding_success_gives_the_xor_mapped_address() {
        let tx = [7u8; 12];
        // 203.0.113.5:54321, XORed with the magic cookie as a STUN server sends it.
        let port = 54321u16 ^ (MAGIC >> 16) as u16;
        let ip = u32::from(Ipv4Addr::new(203, 0, 113, 5)) ^ MAGIC;
        let mut m = Vec::new();
        m.extend_from_slice(&BINDING_SUCCESS.to_be_bytes());
        m.extend_from_slice(&12u16.to_be_bytes());
        m.extend_from_slice(&MAGIC.to_be_bytes());
        m.extend_from_slice(&tx);
        m.extend_from_slice(&XOR_MAPPED_ADDRESS.to_be_bytes());
        m.extend_from_slice(&8u16.to_be_bytes());
        m.extend_from_slice(&[0, 1]);
        m.extend_from_slice(&port.to_be_bytes());
        m.extend_from_slice(&ip.to_be_bytes());
        assert_eq!(parse_response(&m, tx), Some("203.0.113.5:54321".parse().unwrap()));
        assert_eq!(parse_response(&m, [8u8; 12]), None, "an answer to another request is ignored");
    }
}
