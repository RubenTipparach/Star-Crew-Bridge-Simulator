//! Seeded, explicit randomness: every random stream is named by the session seed, the stable id of
//! the thing it belongs to and its purpose (CLAUDE.md 6.4).
//!
//! It lives in the core so that the server, a replay and a test draw the same numbers for the same
//! thing whatever order other things ask in. The generator is SplitMix64: small, fast, and good
//! enough for game outcomes (never for cryptography).

/// A random stream for one purpose of one thing in one session.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

/// FNV-1a 64 over bytes: the purpose name's part of a stream's seed.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

impl Rng {
    /// The stream for `purpose` (a short stable name, such as "fire_spread") of the thing with
    /// `stable_id`, in the session seeded `session_seed`.
    pub fn for_purpose(session_seed: u64, stable_id: u64, purpose: &str) -> Self {
        let s = mix(session_seed ^ mix(stable_id.wrapping_add(0x9e37_79b9_7f4a_7c15)) ^ mix(fnv1a(purpose.as_bytes())));
        Self { state: s }
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix(self.state)
    }

    /// A number in [0, 1), from the top 53 bits.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_names_give_the_same_numbers_in_any_order() {
        let mut a = Rng::for_purpose(7, 42, "fire_spread");
        let mut other = Rng::for_purpose(7, 43, "fire_spread");
        let _ = other.next_u64();
        let mut b = Rng::for_purpose(7, 42, "fire_spread");
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys, "a stream depends only on its names, not on other streams");
    }

    #[test]
    fn purposes_and_things_get_different_streams() {
        let x = Rng::for_purpose(7, 42, "fire_spread").next_u64();
        assert_ne!(x, Rng::for_purpose(7, 42, "hull_breach").next_u64());
        assert_ne!(x, Rng::for_purpose(7, 41, "fire_spread").next_u64());
        assert_ne!(x, Rng::for_purpose(8, 42, "fire_spread").next_u64());
    }

    #[test]
    fn floats_stay_in_the_unit_interval() {
        let mut r = Rng::for_purpose(1, 1, "test");
        assert!((0..10_000).map(|_| r.next_f64()).all(|x| (0.0..1.0).contains(&x)));
    }
}
