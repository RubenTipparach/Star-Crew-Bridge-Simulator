//! Paths for bodies that are not steered by a player (openspec/changes/crew-npcs, design 7): the walk grid.
//!
//! It lives in the core because a bot's path is a rule the server runs and every client must agree on (CLAUDE.md 6.1,
//! 6.4). It is the first version of design 5's nav graph: until the deck file carries the compartments' brushes, the
//! grid is sampled from the walk world itself, so it can never disagree with what a body collides with.
//!
//! The grid: a cell every `CELL_M` on each deck, walkable where a floor lies near the deck's height, a body fits (no
//! wall within its radius at knee and chest height, headroom above), and outside every lift shaft. A cell joins its
//! four neighbours where the floor steps by no more than a stair's rise and nothing stands between them at knee and
//! chest height; diagonals join where both sides do. Ladders join the cells at their foot and their top. A path is an
//! A* over the cells (ties broken by cell index, so the same query gives the same path), then shortened by dropping
//! every point a straight walk clears.

use crate::deck::WalkLadder;
use crate::walk::{in_poly, WalkWorld};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The grid's spacing, metres.
pub const CELL_M: f32 = 0.5;
/// A body's clearance from walls, metres (its radius and a margin).
const CLEAR_M: f32 = 0.34;
/// The most a floor may rise or fall between two neighbouring cells, metres (a stair at 0.5 m spacing).
const STEP_M: f32 = 0.55;
/// Headroom a body needs, metres.
const HEAD_M: f32 = 1.8;
/// Seconds a metre of ladder costs, as a walk's metres (a climb is slower than a walk).
const LADDER_COST: f32 = 3.0;

/// One point of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waypoint {
    /// Walk to this floor point (ship coordinates, metres).
    Walk([f32; 3]),
    /// At this ladder's foot (going up) or top (going down), climb: Use, with `down` for down.
    Climb {
        /// Where the climb starts.
        at: [f32; 3],
        /// Down the ladder.
        down: bool,
        /// The floor it ends on, metres.
        to_y: f32,
    },
}

/// A ladder between its foot cell `a` and its top cell `b`.
#[derive(Debug, Clone, Copy)]
struct Link {
    a: usize,
    b: usize,
    cost: f32,
}

/// The walk grid.
pub struct NavGrid {
    x0: f32,
    z0: f32,
    nx: usize,
    nz: usize,
    /// Each deck's floor height, metres.
    decks: Vec<f32>,
    /// Each cell's floor height (NaN: not walkable), deck by deck.
    floor: Vec<f32>,
    /// Joins to the four neighbours: bit 0 +x, 1 -x, 2 +z, 3 -z.
    joins: Vec<u8>,
    ladders: Vec<Link>,
}

/// A cell's index from its deck and grid position.
fn idx(nx: usize, nz: usize, d: usize, ix: usize, iz: usize) -> usize {
    (d * nz + iz) * nx + ix
}

#[derive(PartialEq)]
struct Open(f32, usize);
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        // A min-heap on cost, then on cell index (stable: never hash order, CLAUDE.md 6.4).
        o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl NavGrid {
    /// Sample the grid from `world` over `bounds` (`[x0, z0, x1, z1]`, metres) on each deck height in `decks`, joining
    /// the ladders; `lifts` are the lift shafts' footprints, which no cell is in.
    pub fn build(
        world: &WalkWorld,
        decks: &[f32],
        bounds: [f32; 4],
        ladders: &[WalkLadder],
        lifts: &[Vec<[f32; 2]>],
    ) -> Self {
        let nx = ((bounds[2] - bounds[0]) / CELL_M).ceil().max(1.0) as usize;
        let nz = ((bounds[3] - bounds[1]) / CELL_M).ceil().max(1.0) as usize;
        let (x0, z0) = (bounds[0] + CELL_M / 2.0, bounds[1] + CELL_M / 2.0);
        let mut floor = vec![f32::NAN; decks.len() * nx * nz];
        for (d, &h) in decks.iter().enumerate() {
            for iz in 0..nz {
                for ix in 0..nx {
                    let (x, z) = (x0 + ix as f32 * CELL_M, z0 + iz as f32 * CELL_M);
                    if lifts.iter().any(|p| in_poly(p, x, z)) {
                        continue;
                    }
                    let Some(y) = world.floor_below(x, h + 1.0, z) else { continue };
                    if y < h - 0.6 || y > h + 0.9 {
                        continue;
                    }
                    // Headroom, then walls within the body's clearance at knee and chest height.
                    if world.cast([x, y + 0.1, z], [0.0, 1.0, 0.0], HEAD_M - 0.1).is_some() {
                        continue;
                    }
                    let blocked = [0.5f32, 1.3].iter().any(|&k| {
                        [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]]
                            .iter()
                            .any(|dir: &[f32; 2]| world.cast([x, y + k, z], [dir[0], 0.0, dir[1]], CLEAR_M).is_some())
                    });
                    if !blocked {
                        floor[idx(nx, nz, d, ix, iz)] = y;
                    }
                }
            }
        }
        let mut joins = vec![0u8; floor.len()];
        for d in 0..decks.len() {
            for iz in 0..nz {
                for ix in 0..nx {
                    let a = idx(nx, nz, d, ix, iz);
                    let ya = floor[a];
                    if ya.is_nan() {
                        continue;
                    }
                    for (bit, dx, dz) in [(0u8, 1i32, 0i32), (2, 0, 1)] {
                        let (jx, jz) = (ix as i32 + dx, iz as i32 + dz);
                        if jx as usize >= nx || jz as usize >= nz {
                            continue;
                        }
                        let b = idx(nx, nz, d, jx as usize, jz as usize);
                        let yb = floor[b];
                        if yb.is_nan() || (yb - ya).abs() > STEP_M {
                            continue;
                        }
                        let (x, z) = (x0 + ix as f32 * CELL_M, z0 + iz as f32 * CELL_M);
                        let base = ya.max(yb);
                        let clear = [0.5f32, 1.3]
                            .iter()
                            .all(|&k| world.cast([x, base + k, z], [dx as f32, 0.0, dz as f32], CELL_M).is_none());
                        if clear {
                            joins[a] |= 1 << bit;
                            joins[b] |= 1 << (bit + 1);
                        }
                    }
                }
            }
        }
        let mut g = Self { x0, z0, nx, nz, decks: decks.to_vec(), floor, joins, ladders: Vec::new() };
        for l in ladders {
            let (Some(a), Some(b)) = (g.nearest([l.x_m, l.lo_m, l.z_m], 1.2), g.nearest([l.x_m, l.hi_m, l.z_m], 1.2))
            else {
                continue;
            };
            g.ladders.push(Link { a, b, cost: (l.hi_m - l.lo_m).abs() * LADDER_COST });
        }
        g
    }

    /// Walkable cells, for reports.
    pub fn walkable(&self) -> usize {
        self.floor.iter().filter(|y| !y.is_nan()).count()
    }

    fn deck_of(&self, y: f32) -> usize {
        let mut best = 0;
        for (i, h) in self.decks.iter().enumerate() {
            if (y - h + 0.2).abs() < (y - self.decks[best] + 0.2).abs() {
                best = i;
            }
        }
        best
    }

    fn point(&self, c: usize) -> [f32; 3] {
        let ix = c % self.nx;
        let iz = (c / self.nx) % self.nz;
        [self.x0 + ix as f32 * CELL_M, self.floor[c], self.z0 + iz as f32 * CELL_M]
    }

    /// The walkable cell nearest `p` on its deck, within `reach_m` across the plan.
    pub fn nearest(&self, p: [f32; 3], reach_m: f32) -> Option<usize> {
        let d = self.deck_of(p[1]);
        let r = (reach_m / CELL_M).ceil() as i32;
        let cx = ((p[0] - self.x0) / CELL_M).round() as i32;
        let cz = ((p[2] - self.z0) / CELL_M).round() as i32;
        let mut best: Option<(f32, usize)> = None;
        for iz in (cz - r).max(0)..=(cz + r).min(self.nz as i32 - 1) {
            for ix in (cx - r).max(0)..=(cx + r).min(self.nx as i32 - 1) {
                let c = idx(self.nx, self.nz, d, ix as usize, iz as usize);
                if self.floor[c].is_nan() {
                    continue;
                }
                let q = self.point(c);
                let dist = (q[0] - p[0]).hypot(q[2] - p[2]);
                if dist <= reach_m && best.is_none_or(|(bd, bc)| dist < bd || (dist == bd && c < bc)) {
                    best = Some((dist, c));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// The walkable point nearest `p` within `reach_m`, if any.
    pub fn snap(&self, p: [f32; 3], reach_m: f32) -> Option<[f32; 3]> {
        self.nearest(p, reach_m).map(|c| self.point(c))
    }

    fn neighbours(&self, c: usize, out: &mut Vec<(usize, f32, bool)>) {
        out.clear();
        let j = self.joins[c];
        let (sx, sz) = (1usize, self.nx);
        let mut orth = [None; 4];
        for (bit, step, plus) in [(0, sx, true), (1, sx, false), (2, sz, true), (3, sz, false)] {
            if j & (1 << bit) != 0 {
                let n = if plus { c + step } else { c - step };
                orth[bit] = Some(n);
                out.push((n, CELL_M, false));
            }
        }
        // Diagonals where both sides join (no corner cutting).
        for (a, b) in [(0usize, 2usize), (0, 3), (1, 2), (1, 3)] {
            if let (Some(na), Some(nb)) = (orth[a], orth[b]) {
                let diag_a = if b == 2 { na + sz } else { na - sz };
                let ja = self.joins[na] & (1 << b) != 0;
                let jb = self.joins[nb] & (1 << a) != 0;
                if ja && jb && !self.floor[diag_a].is_nan() {
                    out.push((diag_a, CELL_M * std::f32::consts::SQRT_2, false));
                }
            }
        }
        for l in &self.ladders {
            if l.a == c {
                out.push((l.b, l.cost, true));
            } else if l.b == c {
                out.push((l.a, l.cost, true));
            }
        }
    }

    /// A path from `from` to `to` (floor points), or `None` when either is off the grid or no way joins them.
    pub fn path(&self, world: &WalkWorld, from: [f32; 3], to: [f32; 3]) -> Option<Vec<Waypoint>> {
        let start = self.nearest(from, 1.5)?;
        let goal = self.nearest(to, 1.5)?;
        let gp = self.point(goal);
        let h = |c: usize| {
            let p = self.point(c);
            (p[0] - gp[0]).hypot(p[2] - gp[2]) + (p[1] - gp[1]).abs()
        };
        let n = self.floor.len();
        let mut cost = vec![f32::INFINITY; n];
        let mut came: Vec<(usize, bool)> = vec![(usize::MAX, false); n];
        let mut open = BinaryHeap::new();
        cost[start] = 0.0;
        open.push(Open(h(start), start));
        let mut nb = Vec::new();
        while let Some(Open(_, c)) = open.pop() {
            if c == goal {
                break;
            }
            self.neighbours(c, &mut nb);
            for &(m, w, ladder) in &nb {
                let k = cost[c] + w;
                if k < cost[m] {
                    cost[m] = k;
                    came[m] = (c, ladder);
                    open.push(Open(k + h(m), m));
                }
            }
        }
        if cost[goal].is_infinite() {
            return None;
        }
        // Back from the goal: cells, with a climb where a ladder link was taken.
        let mut cells = vec![(goal, false)];
        let mut c = goal;
        while c != start {
            let (p, ladder) = came[c];
            cells.push((p, ladder));
            c = p;
        }
        cells.reverse();
        // cells[i].1 says the link from cells[i - 1] to cells[i] was a ladder: shift it onto the climb's start.
        let mut out = Vec::new();
        let mut run: Vec<[f32; 3]> = Vec::new();
        for i in 0..cells.len() {
            let p = self.point(cells[i].0);
            let climb_next = i + 1 < cells.len() && cells[i + 1].1;
            run.push(p);
            if climb_next || i + 1 == cells.len() {
                out.extend(self.shorten(world, &run).into_iter().map(Waypoint::Walk));
                run.clear();
                if climb_next {
                    let q = self.point(cells[i + 1].0);
                    out.push(Waypoint::Climb { at: p, down: q[1] < p[1], to_y: q[1] });
                    run.push(q);
                }
            }
        }
        Some(out)
    }

    /// Drop the points of a run on one deck that a straight walk clears: from each kept point, the farthest later point
    /// whose line stays on walkable cells with no wall across it at knee height.
    fn shorten(&self, world: &WalkWorld, run: &[[f32; 3]]) -> Vec<[f32; 3]> {
        if run.len() <= 2 {
            return run.to_vec();
        }
        let mut out = vec![run[0]];
        let mut i = 0;
        while i + 1 < run.len() {
            let mut j = i + 1;
            for k in (i + 2..run.len()).rev() {
                if self.clear(world, run[i], run[k]) {
                    j = k;
                    break;
                }
            }
            out.push(run[j]);
            i = j;
        }
        out
    }

    fn clear(&self, world: &WalkWorld, a: [f32; 3], b: [f32; 3]) -> bool {
        let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
        let len = dx.hypot(dz);
        if len < 1e-3 {
            return true;
        }
        let steps = (len / (CELL_M * 0.5)).ceil() as usize;
        let mut last = a[1];
        for s in 1..=steps {
            let t = s as f32 / steps as f32;
            let p = [a[0] + dx * t, a[1] + (b[1] - a[1]) * t, a[2] + dz * t];
            let Some(c) = self.nearest(p, CELL_M * 0.75) else { return false };
            let y = self.floor[c];
            if (y - last).abs() > STEP_M {
                return false;
            }
            last = y;
        }
        let dir = [dx / len, 0.0, dz / len];
        // Both sides of the body, at knee and chest height.
        let side = [-dir[2] * (CLEAR_M - 0.05), 0.0, dir[0] * (CLEAR_M - 0.05)];
        let base = a[1].max(b[1]);
        [0.5f32, 1.3].iter().all(|&k| {
            [1.0f32, -1.0]
                .iter()
                .all(|&s| world.cast([a[0] + side[0] * s, base + k, a[2] + side[2] * s], dir, len).is_none())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walk::{WalkData, WalkEntities};

    fn quad(t: &mut Vec<[f32; 9]>, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]) {
        t.push([a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]);
        t.push([a[0], a[1], a[2], c[0], c[1], c[2], d[0], d[1], d[2]]);
    }

    fn data() -> WalkData {
        crate::data::parse("data/crew/walk.json", include_str!("../../../data/crew/walk.json")).expect("walk data")
    }

    /// Two rooms 10 m square side by side, a wall between them at x = 0 with a 1.2 m door at z 7..8.2.
    fn rooms() -> Vec<[f32; 9]> {
        let mut t = Vec::new();
        quad(&mut t, [-10.0, 0.0, 0.0], [-10.0, 0.0, 10.0], [10.0, 0.0, 10.0], [10.0, 0.0, 0.0]);
        quad(&mut t, [0.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 3.0, 7.0], [0.0, 0.0, 7.0]);
        quad(&mut t, [0.0, 0.0, 8.2], [0.0, 3.0, 8.2], [0.0, 3.0, 10.0], [0.0, 0.0, 10.0]);
        t
    }

    #[test]
    fn a_path_goes_through_the_door_and_never_through_the_wall() {
        let d = data();
        let w = WalkWorld::new(&rooms(), &WalkEntities::default(), &d).expect("world");
        let g = NavGrid::build(&w, &[0.0], [-10.0, 0.0, 10.0, 10.0], &[], &[]);
        assert!(g.walkable() > 600, "most of two 100 m^2 rooms is walkable: {}", g.walkable());
        let p = g.path(&w, [-5.0, 0.0, 2.0], [5.0, 0.0, 2.0]).expect("a way through the door");
        let pts: Vec<[f32; 3]> =
            p.iter().map(|w| if let Waypoint::Walk(q) = w { *q } else { panic!("no ladder here") }).collect();
        // Wherever the path crosses x = 0 it is in the door's gap.
        for s in pts.windows(2) {
            let (a, b) = (s[0], s[1]);
            if (a[0] < 0.0) != (b[0] < 0.0) {
                let t = -a[0] / (b[0] - a[0]);
                let z = a[2] + (b[2] - a[2]) * t;
                assert!((7.0..=8.2).contains(&z), "the path crosses the wall at z {z}: {pts:?}");
            }
        }
        assert_eq!(p, g.path(&w, [-5.0, 0.0, 2.0], [5.0, 0.0, 2.0]).unwrap(), "the same query gives the same path");
    }

    #[test]
    fn a_ladder_joins_two_decks() {
        let d = data();
        let mut t = Vec::new();
        quad(&mut t, [-6.0, 0.0, -6.0], [-6.0, 0.0, 6.0], [6.0, 0.0, 6.0], [6.0, 0.0, -6.0]);
        quad(&mut t, [-6.0, 3.5, -6.0], [-6.0, 3.5, 6.0], [6.0, 3.5, 6.0], [6.0, 3.5, -6.0]);
        let w = WalkWorld::new(&t, &WalkEntities::default(), &d).expect("world");
        let ladder = WalkLadder { x_m: 0.0, z_m: 0.0, lo_m: 0.0, hi_m: 3.5 };
        let g = NavGrid::build(&w, &[0.0, 3.5], [-6.0, -6.0, 6.0, 6.0], &[ladder], &[]);
        let p = g.path(&w, [-4.0, 0.0, -4.0], [4.0, 3.5, 4.0]).expect("up the ladder");
        assert!(
            p.iter().any(|w| matches!(w, Waypoint::Climb { down: false, .. })),
            "the way up is the ladder, climbed up: {p:?}"
        );
    }
}
