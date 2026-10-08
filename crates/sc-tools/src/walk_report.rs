//! `sc-tools walk-report`: what the walk costs (deck-pipeline 13a, "The Pi 5 budget"). A measurement instrument
//! (CLAUDE.md 4): it changes nothing, it reports the heap the walk world holds and the time a body's step takes,
//! for the budget table's walk row once the same command runs on a Pi 5.
//!
//! Memory is counted by `sc-tools`' counting allocator (main.rs): the bytes live on the heap after each stage,
//! and the most at once while the walk world is built. Time is a body walking the deck's start round for a minute
//! of game time in 1/60 s steps (two substeps each), each step timed.

use crate::alloc::{live_bytes, peak_bytes, reset_peak};
use sc_core::walk::{Body, Input, WalkData, WalkEntities, WalkWorld};
use std::path::Path;
use std::time::Instant;

fn mb(b: usize) -> f64 {
    b as f64 / 1e6
}

/// Measure `compiled/<ship>.deck`'s walk under `root`; returns the report.
pub fn run(root: &Path, ship: &str) -> Result<String, String> {
    let mut out = Vec::new();
    let base = live_bytes();
    let path = root.join("compiled").join(format!("{ship}.deck"));
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let after_file = live_bytes();
    let deck = sc_core::deck::read(&bytes).map_err(|e| e.to_string())?;
    let text =
        std::fs::read_to_string(root.join("data/crew/walk.json")).map_err(|e| format!("data/crew/walk.json: {e}"))?;
    let data: WalkData = sc_core::data::parse("data/crew/walk.json", &text).map_err(|e| e.to_string())?;
    let w = &deck.index.walk;
    let ents = WalkEntities {
        ladders: w.ladders.clone(),
        hatches: w.hatches.clone(),
        doors: w.doors.clone(),
        lifts: w.lifts.clone(),
    };
    let tris = deck.walk_triangles();
    let before_world = live_bytes();
    reset_peak();
    let t0 = Instant::now();
    let world = WalkWorld::new(&tris, &ents, &data)?;
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let world_live = live_bytes() - before_world;
    let world_peak = peak_bytes() - before_world;
    drop(tris);
    out.push(format!("the deck file in memory: {:.1} MB", mb(after_file - base)));
    out.push(format!(
        "the walk world: {} triangles; {:.1} MB held after it is built, {:.1} MB at most while building; built in {build_ms:.0} ms",
        world.triangles,
        mb(world_live),
        mb(world_peak)
    ));
    out.push(format!("its triangles as they sit in the deck file: {:.1} MB", mb(w.triangle_count as usize * 36)));

    // A body walking: forward, turning a little each second, running half the time, for a minute of game time.
    let mut body = Body::at_start(&w.start, &data);
    let before_body = live_bytes();
    let mut times = Vec::new();
    let mut yaw = w.start.face[0].atan2(w.start.face[1]);
    for k in 0..3600u32 {
        if k % 60 == 0 {
            yaw += 0.9;
        }
        let input = Input { forward: 1.0, yaw, run: (k / 600) % 2 == 1, jump: k % 240 == 0, ..Input::default() };
        let t = Instant::now();
        body.step(&world, &data, &input, 1.0 / 60.0);
        times.push(t.elapsed().as_secs_f64() * 1e6);
    }
    let steady = live_bytes() as i64 - before_body as i64;
    times.sort_by(f64::total_cmp);
    let pct = |p: f64| times[((times.len() - 1) as f64 * p) as usize];
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    out.push(format!(
        "a body's 1/60 s step (two substeps), {} steps: mean {mean:.0} us, median {:.0}, 99th {:.0}, worst {:.0}; heap change while walking {} bytes",
        times.len(),
        pct(0.5),
        pct(0.99),
        times[times.len() - 1],
        steady
    ));
    out.push(format!(
        "eight bodies at 60 steps a second (the server): about {:.1} % of one core here",
        mean * 8.0 * 60.0 / 1e4
    ));
    out.push(format!("total heap now: {:.1} MB", mb(live_bytes() - base)));
    Ok(out.join("\n"))
}
