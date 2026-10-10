//! The drill's bridge for the crew on foot (openspec/changes/coop-drill design 9): the seats from the ship's layout,
//! the walk speed from `data/crew/walk.json`, the muster point from the mission, and the paths between them from the
//! deck's walk grid (`sc-core::nav`), the bot crew's.
//!
//! It lives in the server because it reads files and builds the walk world; the drill (`sc-core`) only follows the
//! paths it is handed. With no compiled deck the paths are straight lines, and the log says so.

use std::path::Path;

use sc_core::combat::bodies::Bridge;
use sc_core::combat::data::DrillData;
use sc_core::combat::Station;
use sc_core::nav::{NavGrid, Waypoint};
use sc_core::walk::{WalkData, WalkEntities, WalkWorld};

/// A station's seat from `data/ships/tern/layout.json`: where and which way.
fn seats(root: &Path, stations: &[Station]) -> Result<Vec<(Station, [f32; 3], f32)>, String> {
    let rel = "data/ships/tern/layout.json";
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))?;
    let list = v["stations"].as_array().ok_or_else(|| format!("{rel}: stations: missing"))?;
    stations
        .iter()
        .map(|s| {
            let e = list
                .iter()
                .find(|e| e["id"].as_str() == Some(s.id()))
                .ok_or_else(|| format!("{rel}: stations: no {:?}", s.id()))?;
            let n = |x: &serde_json::Value| x.as_f64().filter(|f| f.is_finite());
            let p =
                e["seat_m"].as_array().filter(|a| a.len() == 3).ok_or_else(|| format!("{rel}: {}.seat_m", s.id()))?;
            let mut at = [0.0f32; 3];
            for (o, x) in at.iter_mut().zip(p) {
                *o = n(x).ok_or_else(|| format!("{rel}: {}.seat_m must be numbers", s.id()))? as f32;
            }
            let yaw = n(&e["yaw_deg"]).ok_or_else(|| format!("{rel}: {}.yaw_deg", s.id()))?;
            Ok((*s, at, (yaw as f32).to_radians()))
        })
        .collect()
}

/// Build the bridge; `deck`, the compiled deck, for paths that go round the chairs. Returns it and a line for the log.
pub fn load(data: &DrillData, root: &Path, deck: Option<&Path>) -> Result<(Bridge, String), String> {
    let stations: Vec<Station> = data.mission.stations.iter().filter_map(|s| Station::from_id(s)).collect();
    let seats = seats(root, &stations)?;
    let rel = "data/crew/walk.json";
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let walk: WalkData = sc_core::data::parse(rel, &text).map_err(|e| e.to_string())?;
    let m = &data.mission;
    let muster = [m.muster_m[0] as f32, m.muster_m[1] as f32, m.muster_m[2] as f32];
    let (sp, ws, st, si) = (
        m.muster_spacing_m as f32,
        walk.r#move.walk_m_s as f32,
        data.stations.seats.stand_s as f32,
        data.stations.seats.sit_s as f32,
    );
    let Some(deck) = deck.filter(|d| d.exists()) else {
        return Ok((Bridge::straight(seats, muster, sp, ws, st, si), "straight paths (no compiled deck)".into()));
    };
    let t0 = std::time::Instant::now();
    let bytes = std::fs::read(deck).map_err(|e| format!("{}: {e}", deck.display()))?;
    let d = sc_core::deck::read(&bytes).map_err(|e| format!("{}: {e}", deck.display()))?;
    let w = &d.index.walk;
    let ents = WalkEntities {
        ladders: w.ladders.clone(),
        hatches: w.hatches.clone(),
        doors: w.doors.clone(),
        lifts: w.lifts.clone(),
    };
    let tris = d.walk_triangles();
    let world = WalkWorld::new(&tris, &ents, &walk)?;
    // The bridge's deck only, over the bridge and a margin: the drill never leaves it.
    let (y, z0, z1) = (muster[1], muster[2].min(seats.iter().map(|s| s.1[2]).fold(f32::MAX, f32::min)) - 3.0, 34.0);
    let shafts: Vec<Vec<[f32; 2]>> = w.lifts.iter().map(|l| l.poly.clone()).collect();
    let grid = NavGrid::build(&world, &[y], [-10.0, z0, 10.0, z1], &[], &shafts);
    let mut straight = 0;
    let bridge = Bridge::new(seats, muster, sp, ws, st, si, |a, b| match grid.path(&world, a, b) {
        Some(p) => p
            .iter()
            .map(|w| match *w {
                Waypoint::Walk(p) => p,
                Waypoint::Climb { at, .. } => at,
            })
            .collect(),
        None => {
            straight += 1;
            vec![a, b]
        }
    });
    let how = format!(
        "paths on the walk grid ({} cells, {:.0} ms){}",
        grid.walkable(),
        t0.elapsed().as_secs_f64() * 1000.0,
        if straight > 0 { format!(", {straight} straight where the grid found no way") } else { String::new() }
    );
    Ok((bridge, how))
}
