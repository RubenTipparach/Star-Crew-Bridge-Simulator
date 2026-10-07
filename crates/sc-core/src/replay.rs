//! The replay hash: a running digest of simulation state, so two runs (a server and a replay, two
//! builds, two machines) can be compared tick by tick.
//!
//! It lives in the core because what is hashed is simulation state. Values are hashed by their
//! exact bits in a fixed order: floats as their IEEE bits, never formatted, so the hash is the state.

/// A running FNV-1a 64 digest of simulation values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayHash(u64);

impl Default for ReplayHash {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl ReplayHash {
    /// Fold in raw bytes.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        for &x in b {
            self.0 = (self.0 ^ u64::from(x)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        self
    }
    /// Fold in an integer, little-endian.
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }
    /// Fold in a float by its bits (0.0 and -0.0 differ; a NaN hashes as its payload).
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.u64(v.to_bits())
    }
    /// The digest so far.
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_order_sensitive_and_exact() {
        let a = ReplayHash::default().f64(1.0).f64(2.0).value();
        let b = ReplayHash::default().f64(2.0).f64(1.0).value();
        assert_ne!(a, b, "order is part of the state");
        let c = ReplayHash::default().f64(1.0).f64(2.0 + f64::EPSILON * 2.0).value();
        assert_ne!(a, c, "one ulp is a different state");
        assert_eq!(a, ReplayHash::default().f64(1.0).f64(2.0).value());
    }
}
