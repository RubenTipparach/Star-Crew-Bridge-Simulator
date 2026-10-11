//! The unreliable channels' bookkeeping (netcode-and-sessions section 2): our sequence numbers, the
//! acknowledgement of theirs, and the loss and staleness each end measures from them.
//!
//! It lives apart from the transport because it is pure arithmetic over headers, tested without a socket.

use crate::msg::Header;

/// `a` is newer than `b`, with wrap-around.
pub fn newer(a: u16, b: u16) -> bool {
    a != b && a.wrapping_sub(b) < 0x8000
}

/// One end of an unreliable channel pair.
#[derive(Clone, Debug, Default)]
pub struct Link {
    next_seq: u16,
    newest: Option<u16>,
    bits: u32,
    /// Messages received.
    pub received: u64,
    /// Received but older than the newest already applied, so dropped.
    pub stale: u64,
    /// Sequences skipped over: sent by the other end and never received (or not yet).
    pub missing: u64,
}

impl Link {
    /// The header for the next message this end sends.
    pub fn next_header(&mut self) -> Header {
        let h = Header { seq: self.next_seq, ack: self.newest.unwrap_or(0), ack_bits: self.bits };
        self.next_seq = self.next_seq.wrapping_add(1);
        h
    }

    /// Record a received header. True when the message is the newest so far and should be applied; false when it
    /// is stale (an older one arriving late, which the unreliable channels drop: newest wins).
    pub fn receive(&mut self, h: &Header) -> bool {
        self.received += 1;
        match self.newest {
            None => {
                self.newest = Some(h.seq);
                self.bits = 0;
                true
            }
            Some(n) if newer(h.seq, n) => {
                let gap = u32::from(h.seq.wrapping_sub(n));
                self.missing += u64::from(gap - 1);
                self.bits = if gap >= 32 { 0 } else { (self.bits << gap) | (1 << (gap - 1)) };
                self.newest = Some(h.seq);
                true
            }
            Some(n) => {
                let back = u32::from(n.wrapping_sub(h.seq));
                if (1..=32).contains(&back) && self.bits & (1 << (back - 1)) == 0 {
                    // A late arrival fills a gap counted as missing.
                    self.bits |= 1 << (back - 1);
                    self.missing = self.missing.saturating_sub(1);
                }
                self.stale += 1;
                false
            }
        }
    }

    /// The share of the other end's messages that never arrived, so far (0-1).
    pub fn loss(&self) -> f64 {
        // Every sequence the other end sent up to the newest either arrived (late or not) or is missing.
        let expected = self.received + self.missing;
        if expected == 0 {
            0.0
        } else {
            self.missing as f64 / expected as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(seq: u16) -> Header {
        Header { seq, ack: 0, ack_bits: 0 }
    }

    #[test]
    fn sequences_wrap() {
        assert!(newer(0, 65535));
        assert!(!newer(65535, 0));
        assert!(newer(5, 3));
    }

    #[test]
    fn a_gap_counts_as_loss_and_a_late_arrival_is_stale_but_fills_it() {
        let mut l = Link::default();
        assert!(l.receive(&h(0)));
        assert!(l.receive(&h(1)));
        assert!(l.receive(&h(4)));
        assert_eq!(l.missing, 2, "2 and 3 never came");
        assert!(!l.receive(&h(3)), "an older one is not applied: newest wins");
        assert_eq!(l.missing, 1, "but it is no longer missing");
        assert_eq!(l.stale, 1);
        assert!((l.loss() - 0.2).abs() < 1e-9, "1 lost of 5 sent: {}", l.loss());
    }

    #[test]
    fn the_header_acknowledges_what_arrived() {
        let mut l = Link::default();
        for s in [10, 11, 13] {
            l.receive(&h(s));
        }
        let out = l.next_header();
        assert_eq!(out.ack, 13);
        assert_eq!(out.ack_bits & 0b11, 0b10, "12 missing, 11 arrived");
    }
}
