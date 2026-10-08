//! `sc-server`: the authoritative loop (engine-stack design sections 3 and 6), headless on a 4 GB
//! Pi 5 or any machine. It renders nothing and plays nothing.
//!
//! Today it is the skeleton of that loop: the fixed 30 Hz clock from `sc-core`, stepped against the
//! wall clock, logging its tick count and the replay hash of its (empty) state each simulated
//! second. Sessions, the ship systems and the network arrive with their changes.

use sc_core::clock::FixedClock;
use sc_core::replay::ReplayHash;
use std::time::{Duration, Instant};

/// The server's tick rate (engine-stack design section 5, "Server tick").
const TICK_HZ: f64 = 30.0;

fn main() {
    let run_s: f64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(2.0);
    let mut clock = FixedClock::new(TICK_HZ, 8);
    let start = Instant::now();
    let mut last = start;
    let mut hash = ReplayHash::default();
    while clock.time_s() < run_s {
        let now = Instant::now();
        let n = clock.advance((now - last).as_secs_f64());
        last = now;
        for _ in 0..n {
            hash.u64(clock.tick());
            if clock.tick() % (TICK_HZ as u64) == 0 {
                println!("tick {} at {:.1} s, state hash {:016x}", clock.tick(), clock.time_s(), hash.value());
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    println!("ran {} ticks in {:.2} s of wall time", clock.tick(), start.elapsed().as_secs_f64());
}
