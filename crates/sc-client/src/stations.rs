//! The Shoals' stations as the client draws them (openspec/changes/patrol-mission design 12b; spacedock design 5a, 5b
//! and 9, tasks 2.3-2.5): Tamsin Station, spacedock's version E (the O'Neill pair: two cylinders with their domes, the
//! mirrors opened toward the star, the farm rings and the rods; the sealed hub, its lit throat and the round bay with
//! its berths, ribs and floods), generated from `data/space/spacedock.json`'s sizes; and the other four versions (A-D)
//! as **placeholders**: their outsides only, from the same file's sizes, doors shut, no bay and no berths.
//!
//! It lives in the client because it is drawing; the sizes are the core's (`sc_core::spacedock`), which the system
//! file's berths and departure legs are checked against, so the bay the Tern flies out of is the bay drawn (CLAUDE.md
//! 6.5). The light is spacedock 5b's fallback, the rule as it stands (CLAUDE.md 9: no per-pixel lighting): the
//! Material Maker textures carry their relief in their albedo, and the light is per vertex, baked when the model is
//! made, since a station does not turn against its star: outside, ambient, Tamsin's earthshine and the star; inside the
//! bay and the throat, its floods, gantry and axis lamps (`data/space/station_models.json`, a stand-in for tasks 2.4's
//! offline bake). Not yet: the habitats' land, clouds and see-through windows (the macro map's panes glow instead), the
//! iris's leaves (it stands open; tasks 2.6), a far model, and docked ships other than the Tern's class (tasks 2.5).

use glam::{DMat4, DVec3, Mat3, Mat4, Quat, Vec3};
use sc_core::data::{Checks, Validate};
use sc_core::exterior::Beacons;
use sc_core::spacedock::{OneillPair, Spacedock};
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray, NO_RELIGHT};
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;

use crate::hull::{HullLight, HullView, ProbeLight};
use crate::outside::BeaconLamps;
use crate::ships3d::{Beacon, MeshBuilder};

/// The drawing's data file, relative to the repository's root.
pub const LOOK_FILE: &str = "data/space/station_models.json";
/// An outside's stored positions are its metres over this: +/-16 km fits the vertex's +/-32.
pub const OUTSIDE_SCALE: f32 = 512.0;
/// An inside's (the hub, the throat and the bay, within a kilometre of the frame's origin): 3 cm steps.
pub const INSIDE_SCALE: f32 = 32.0;

/// The texture array's layers, in `textures`' order.
const DETAIL: u8 = 0;
const CAP: u8 = 1;
const MIRROR: u8 = 2;
/// The macro map: the last layer, whose alpha (the panes) glows.
const MACRO: u8 = 3;
const LAYERS: u32 = 4;

/// The texture layers' files.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Textures {
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// The tiled hull detail.
    pub detail: String,
    /// The domes' map, once over a dome.
    pub cap: String,
    /// The mirrors' facets, tiled.
    pub mirror: String,
    /// The cylinders' outside, once round them.
    #[serde(rename = "macro")]
    pub macro_map: String,
}

/// The light outside, display multipliers.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutsideLight {
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// What every face takes.
    pub ambient: [f32; 3],
    /// Tamsin's light on a face turned to it.
    pub earthshine: [f32; 3],
    /// The star's on a face turned to it.
    pub star: f32,
    /// How brightly the macro map's panes glow, 0-1.
    pub panes_glow: f32,
    /// The detail layer's repeat, metres.
    pub detail_tile_m: f32,
    /// The mirror layer's repeat, metres.
    pub mirror_tile_m: f32,
}

/// The round bay's and the throat's lamps (version E), for the bake at load.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BayLight {
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// What every inside face takes, display multipliers.
    pub ambient: [f32; 3],
    /// The lamps' colour.
    pub flood: [f32; 3],
    /// The first flood ring's distance from the back wall, metres.
    pub flood_first_m: f32,
    /// The flood rings' spacing, metres.
    pub flood_every_m: f32,
    /// Floods a ring.
    pub flood_per_ring: usize,
    /// How far in from the wall the floods stand, metres.
    pub flood_inset_m: f32,
    /// A flood's strength.
    pub flood_intensity: f32,
    /// A flood's reach, metres.
    pub flood_reach_m: f32,
    /// Lamps on the axis: [z in the station's frame (metres), strength, reach (metres)].
    pub axis_lamps: Vec<[f32; 3]>,
    /// The lamp under each gantry: its height in the berth's frame, metres.
    pub gantry_up_m: f32,
    /// Its strength.
    pub gantry_intensity: f32,
    /// Its reach, metres.
    pub gantry_reach_m: f32,
    /// The throat's light rings' spacing from the mouth, metres.
    pub throat_every_m: f32,
    /// Lamps a throat ring.
    pub throat_per_ring: usize,
    /// A throat lamp's strength.
    pub throat_intensity: f32,
    /// Its reach, metres.
    pub throat_reach_m: f32,
    /// The bay's steel ribs' spacing, metres.
    pub rib_every_m: f32,
    /// The back wall's window tiers' spacing, metres.
    pub window_every_m: f32,
    /// The light on a ship in the bay from its own up (the floods and gantries above it), the hull's sun strength: the
    /// stand-in's, without the bay's probes (design 8b).
    pub ship_light: f32,
    /// The most light a bay's probes give a hull, a display multiplier (0-2) on each axis of the cube: a hull's texture
    /// carries its own shading, so a pool's light is held under this, as the mockup holds it (design 8b).
    pub hull_probe_max: f32,
}

/// The parts' paints: display multipliers on the detail layer.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paint {
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// The cylinders' plating and the rods.
    pub pearl: [f32; 3],
    /// The hub, the bearings, the spokes and the mirrors' backs.
    pub graphite: [f32; 3],
    /// The seal collars and every third farm pod.
    pub orange: [f32; 3],
    /// The ribs, piers and gantries inside.
    pub steel: [f32; 3],
    /// The bay's wall and back wall.
    pub wall: [f32; 3],
    /// The throat's wall.
    pub throat: [f32; 3],
    /// The gantries' striped beams.
    pub hazard: [f32; 3],
    /// Shoals Hollow's rock.
    pub rock: [f32; 3],
}

/// The glowing parts' colours, their own light.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glow {
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// Work lights: the floods' rings, the throat's rings and strips, the gantries.
    pub work: [f32; 3],
    /// The guide rings on the dome, the iris's rim.
    pub accent: [f32; 3],
    /// Lit windows.
    pub window: [f32; 3],
    /// Port and the beacons.
    pub red: [f32; 3],
    /// Starboard.
    pub green: [f32; 3],
}

/// How finely round parts are cut.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segments {
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// The cylinders', domes' and hub's sides.
    pub round: usize,
    /// A dome's rings.
    pub dome_rings: usize,
    /// The bay's wall and the throat's sides.
    pub bay: usize,
    /// The placeholders' round parts.
    pub placeholder: usize,
}

/// `data/space/station_models.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationLook {
    /// `starcrew.station-models/1`.
    pub schema: String,
    #[serde(rename = "_doc", default)]
    _doc: IgnoredAny,
    /// The texture layers' size, px.
    pub layer_px: u32,
    /// The layers' files.
    pub textures: Textures,
    /// The light outside.
    pub outside: OutsideLight,
    /// The light inside version E's bay.
    pub bay: BayLight,
    /// The paints.
    pub paint: Paint,
    /// The glowing parts.
    pub glow: Glow,
    /// The tessellation.
    pub segments: Segments,
    /// Stations farther than this from the Tern are not drawn, metres.
    pub draw_m: f64,
}

impl Validate for StationLook {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.station-models/1");
        c.count("layer_px", i64::from(self.layer_px), 64, 2048);
        if !self.layer_px.is_power_of_two() {
            c.number("layer_px (a power of two)", f64::from(self.layer_px), 0.0, -1.0);
        }
        let o = &self.outside;
        c.number("outside.star", f64::from(o.star), 0.0, 1.9);
        c.number("outside.panes_glow", f64::from(o.panes_glow), 0.0, 1.0);
        c.number("outside.detail_tile_m", f64::from(o.detail_tile_m), 1.0, 1000.0);
        c.number("outside.mirror_tile_m", f64::from(o.mirror_tile_m), 1.0, 1000.0);
        let b = &self.bay;
        c.number("bay.flood_every_m", f64::from(b.flood_every_m), 10.0, 1000.0);
        c.number("bay.flood_reach_m", f64::from(b.flood_reach_m), 1.0, 2000.0);
        c.number("bay.gantry_reach_m", f64::from(b.gantry_reach_m), 1.0, 2000.0);
        c.number("bay.throat_every_m", f64::from(b.throat_every_m), 5.0, 1000.0);
        c.number("bay.throat_reach_m", f64::from(b.throat_reach_m), 1.0, 2000.0);
        c.number("bay.rib_every_m", f64::from(b.rib_every_m), 5.0, 1000.0);
        c.number("bay.window_every_m", f64::from(b.window_every_m), 5.0, 1000.0);
        c.number("bay.ship_light", f64::from(b.ship_light), 0.0, 1.0);
        c.number("bay.hull_probe_max", f64::from(b.hull_probe_max), 0.5, 2.0);
        for (i, l) in b.axis_lamps.iter().enumerate() {
            c.number(&format!("bay.axis_lamps[{i}] (reach)"), f64::from(l[2]), 1.0, 2000.0);
        }
        let s = &self.segments;
        c.count("segments.round", s.round as i64, 8, 128);
        c.count("segments.dome_rings", s.dome_rings as i64, 2, 32);
        c.count("segments.bay", s.bay as i64, 8, 128);
        c.count("segments.placeholder", s.placeholder as i64, 8, 128);
        c.number("draw_m", self.draw_m, 1000.0, 1.0e6);
        let colours = [
            ("outside.ambient", o.ambient),
            ("outside.earthshine", o.earthshine),
            ("bay.ambient", b.ambient),
            ("bay.flood", b.flood),
            ("glow.work", self.glow.work),
            ("glow.accent", self.glow.accent),
            ("glow.window", self.glow.window),
            ("glow.red", self.glow.red),
            ("glow.green", self.glow.green),
        ];
        for (k, rgb) in colours {
            for x in rgb {
                c.number(k, f64::from(x), 0.0, 2.0);
            }
        }
    }
}

impl StationLook {
    /// The file from the working directory, checked.
    pub fn load() -> Result<Self, String> {
        let text = std::fs::read_to_string(LOOK_FILE).map_err(|e| format!("{LOOK_FILE}: {e}"))?;
        sc_core::data::parse(LOOK_FILE, &text).map_err(|e| e.to_string())
    }
}

/// One triangle of a part being built, in metres in the station's frame.
#[derive(Clone, Copy)]
struct Tri {
    p: [Vec3; 3],
    uv: [[f32; 2]; 3],
    layer: u8,
    rgb: [f32; 3],
}

/// Triangles being built, in metres; packed at a scale once whole, each vertex lit by a rule.
#[derive(Default)]
struct Soup {
    tris: Vec<Tri>,
}

/// A frame for a part: its origin, and the axes `u`, `v` round its axis `w` (right handed).
#[derive(Clone, Copy)]
struct Frame {
    o: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
}

impl Frame {
    /// About +z at `o`.
    fn z(o: Vec3) -> Self {
        Self { o, u: Vec3::X, v: Vec3::Y, w: Vec3::Z }
    }

    /// About +y at `o` (u along +z, v along +x).
    fn y(o: Vec3) -> Self {
        Self { o, u: Vec3::Z, v: Vec3::X, w: Vec3::Y }
    }

    /// The point at angle `a` round the axis, radius `r`, height `h` along it.
    fn at(&self, a: f32, r: f32, h: f32) -> Vec3 {
        self.o + (self.u * a.cos() + self.v * a.sin()) * r + self.w * h
    }

    /// The way out from the axis at angle `a`.
    fn radial(&self, a: f32) -> Vec3 {
        self.u * a.cos() + self.v * a.sin()
    }
}

/// How a part's texture lies on it: from its angle round the axis, its radius, the profile's largest radius, its height
/// and its distance along its profile, metres. A tiled layer runs round at the largest radius, so a disc's quad never
/// spans more repeats than its rim's.
type UvRule<'a> = &'a dyn Fn(f32, f32, f32, f32, f32) -> [f32; 2];

impl Soup {
    /// A triangle whose front faces `out` (its winding chosen so).
    fn tri(&mut self, p: [Vec3; 3], uv: [[f32; 2]; 3], layer: u8, rgb: [f32; 3], out: Vec3) {
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        // A triangle with no area (a disc's centre) draws nothing.
        if n.length_squared() < 1e-6 {
            return;
        }
        let t = if n.dot(out) >= 0.0 {
            Tri { p, uv, layer, rgb }
        } else {
            Tri { p: [p[0], p[2], p[1]], uv: [uv[0], uv[2], uv[1]], layer, rgb }
        };
        self.tris.push(t);
    }

    /// A quad (corners in order round it) facing `out`.
    fn quad(&mut self, q: [Vec3; 4], uv: [[f32; 2]; 4], layer: u8, rgb: [f32; 3], out: Vec3) {
        self.tri([q[0], q[1], q[2]], [uv[0], uv[1], uv[2]], layer, rgb, out);
        self.tri([q[0], q[2], q[3]], [uv[0], uv[2], uv[3]], layer, rgb, out);
    }

    /// A surface of revolution: the profile `prof` ([radius, height] in `f`) turned round `f`'s axis in `sides`. Each
    /// face's front is `sign` times the profile's right hand ((dh, -dr) in the radius-height plane): +1 for a profile
    /// whose outside is on its right as it is walked.
    #[allow(clippy::too_many_arguments)]
    fn lathe(
        &mut self,
        f: &Frame,
        prof: &[[f32; 2]],
        sides: usize,
        sign: f32,
        layer: u8,
        rgb: [f32; 3],
        uv: UvRule<'_>,
    ) {
        let tau = std::f32::consts::TAU;
        let rmax = prof.iter().map(|p| p[0]).fold(0.0f32, f32::max);
        let mut along = vec![0.0f32];
        for j in 1..prof.len() {
            let d = ((prof[j][0] - prof[j - 1][0]).powi(2) + (prof[j][1] - prof[j - 1][1]).powi(2)).sqrt();
            along.push(along[j - 1] + d);
        }
        for i in 0..sides {
            let (a0, a1) = (tau * i as f32 / sides as f32, tau * (i + 1) as f32 / sides as f32);
            for j in 0..prof.len() - 1 {
                let ([r0, h0], [r1, h1]) = (prof[j], prof[j + 1]);
                let am = (a0 + a1) / 2.0;
                let out = (f.radial(am) * (h1 - h0) - f.w * (r1 - r0)) * sign;
                let q = [f.at(a0, r0, h0), f.at(a1, r0, h0), f.at(a1, r1, h1), f.at(a0, r1, h1)];
                let t = [
                    uv(a0, r0, rmax, h0, along[j]),
                    uv(a1, r0, rmax, h0, along[j]),
                    uv(a1, r1, rmax, h1, along[j + 1]),
                    uv(a0, r1, rmax, h1, along[j + 1]),
                ];
                self.quad(q, t, layer, rgb, out);
            }
        }
    }

    /// A box of half extents `half` about `c`, turned by `rot`; each face styled by `style` from its outward normal in
    /// the box's own axes ([layer, paint]), its texture tiled every `tile` metres.
    fn block(&mut self, c: Vec3, half: Vec3, rot: Quat, tile: f32, style: &dyn Fn(Vec3) -> (u8, [f32; 3])) {
        for k in 0..3 {
            for s in [-1.0f32, 1.0] {
                let mut n = Vec3::ZERO;
                n[k] = s;
                let (a, b) = ((k + 1) % 3, (k + 2) % 3);
                let corner = |x: f32, y: f32| {
                    let mut e = n;
                    e[a] = x;
                    e[b] = y;
                    c + rot * (half * e)
                };
                // A long face (a rod across the pair) stretches its texture rather than leave the vertex's +/-32.
                let (ua, ub) = ((half[a] * 2.0 / tile).min(30.0), (half[b] * 2.0 / tile).min(30.0));
                let q = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
                let t = [[0.0, 0.0], [ua, 0.0], [ua, ub], [0.0, ub]];
                let (layer, rgb) = style(n);
                self.quad(q, t, layer, rgb, rot * n);
            }
        }
    }

    /// A box in one paint on the detail layer.
    fn plain(&mut self, c: Vec3, half: Vec3, rot: Quat, tile: f32, rgb: [f32; 3]) {
        self.block(c, half, rot, tile, &|_| (DETAIL, rgb));
    }

    /// A square beam `size` across from `a` to `b`.
    fn beam(&mut self, a: Vec3, b: Vec3, size: f32, tile: f32, rgb: [f32; 3]) {
        let d = b - a;
        let rot = Quat::from_rotation_arc(Vec3::Z, d.normalize());
        self.plain((a + b) / 2.0, Vec3::new(size / 2.0, size / 2.0, d.length() / 2.0), rot, tile, rgb);
    }

    /// Pack at 1/`scale` into meshes of at most 16-bit indices, each vertex's colour its paint times `light(p, n)`
    /// (the face's normal `n`); the texture coordinates of each triangle moved by whole repeats toward zero, so a tiled
    /// layer fits the vertex's +/-32.
    fn pack(&self, scale: f32, light: &dyn Fn(Vec3, Vec3) -> [f32; 3]) -> Vec<MeshBuilder> {
        let mut out = vec![MeshBuilder::default()];
        for t in &self.tris {
            if out.last().is_some_and(|m| m.vertices.len() / 28 + 3 > usize::from(u16::MAX)) {
                out.push(MeshBuilder::default());
            }
            let n = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize();
            let du = t.uv.iter().map(|x| x[0]).fold(f32::MAX, f32::min).floor();
            let dv = t.uv.iter().map(|x| x[1]).fold(f32::MAX, f32::min).floor();
            let colours = t.p.map(|p| {
                let l = light(p, n);
                [t.rgb[0] * l[0], t.rgb[1] * l[1], t.rgb[2] * l[2]]
            });
            let m = out.last_mut().expect("one mesh at least");
            m.tri_full(
                t.p.map(|p| (p / scale).to_array()),
                n.to_array(),
                t.uv.map(|x| [x[0] - du, x[1] - dv]),
                t.layer,
                colours,
            );
        }
        out
    }
}

/// A light's share at `p` on a face with normal `n` from a lamp at `at` of `strength` reaching `reach` metres: the
/// cosine, fading as (1 - d / reach) squared.
fn lamp(p: Vec3, n: Vec3, at: Vec3, strength: f32, reach: f32) -> f32 {
    let d = at - p;
    let len = d.length();
    if len >= reach || len < 1e-3 {
        return 0.0;
    }
    let fade = (1.0 - len / reach).powi(2);
    strength * fade * n.dot(d / len).max(0.0)
}

/// The light outside: ambient, the earthshine by how a face turns to the planet and the star by how it turns to it
/// (both directions in the station's frame).
fn outside_light(o: &OutsideLight, sun: Vec3, planet: Vec3) -> impl Fn(Vec3, Vec3) -> [f32; 3] + '_ {
    move |_, n| {
        let (e, s) = (n.dot(planet).max(0.0), n.dot(sun).max(0.0) * o.star);
        [0, 1, 2].map(|k| o.ambient[k] + o.earthshine[k] * e + s)
    }
}

/// A station's model, generated: its parts in metres, ready to pack.
pub struct StationModel {
    /// Its outside, lit by the star (at `OUTSIDE_SCALE`).
    pub outside: Vec<MeshBuilder>,
    /// Its inside, lit by its lamps (at `INSIDE_SCALE`).
    pub inside: Vec<MeshBuilder>,
    /// Its glowing parts (at `INSIDE_SCALE`).
    pub glow: Vec<MeshBuilder>,
    /// The docked ships drawn with the Tern's model: their poses in the station's frame (the docked ship's frame).
    pub docked: Vec<DMat4>,
    /// Its beacons.
    pub beacons: Vec<Beacon>,
    /// Whether it is a placeholder (A-D).
    pub placeholder: bool,
}

impl StationModel {
    /// Its triangles, every part.
    pub fn triangles(&self) -> usize {
        self.outside.iter().chain(&self.inside).chain(&self.glow).map(MeshBuilder::triangles).sum()
    }

    /// How far its farthest drawn vertex lies from its frame's origin, metres (sky-behind-solids 2): every part at the
    /// scale it is drawn at. Its docked ships and beacons lie well inside.
    pub fn reach_m(&self) -> f32 {
        let parts = [(&self.outside, OUTSIDE_SCALE), (&self.inside, INSIDE_SCALE), (&self.glow, INSIDE_SCALE)];
        parts
            .iter()
            .flat_map(|(ms, scale)| ms.iter().map(move |m| crate::ships3d::reach(m) * scale))
            .fold(0.0, f32::max)
    }
}

/// A berth's pose in E's bay: `angle_deg` round the axis from the bottom, `z_m` along it, `radius_m` out from it; the
/// ship's up toward the axis, its bow along +z (spacedock design 5a; the mockup's `RZ(angle) T(0, -radius, z)`).
pub fn e_berth(angle_deg: f64, radius_m: f64, z_m: f64) -> DMat4 {
    DMat4::from_rotation_z(angle_deg.to_radians()) * DMat4::from_translation(DVec3::new(0.0, -radius_m, z_m))
}

/// The ships in the fleet that the Tern's model draws: `spacedock.json` `fleet`, by their model.
fn drawn_with_tern(sd_json: &Value, key: &str) -> bool {
    key != "tern" && sd_json["fleet"][key]["model"].as_str().is_some_and(|m| m.contains("tern"))
}

/// Version E, Tamsin Station (spacedock design 5a), lit by the star along `sun` and Tamsin along `planet` (the station's
/// frame); `sd_json` the spacedock file (its fleet and the port's light rings). With `baked`, the round bay's and the
/// throat's own walls, ribs, berths and lit fixtures are left out for the mockup's baked bay (`BakedBay`, design 8b).
pub fn oneill(
    e: &OneillPair,
    sd_json: &Value,
    look: &StationLook,
    sun: Vec3,
    planet: Vec3,
    baked: bool,
) -> StationModel {
    let (paint, glow, seg) = (&look.paint, &look.glow, &look.segments);
    let tile = look.outside.detail_tile_m;
    let tau = std::f32::consts::TAU;
    let y = &e.cylinders;
    let (r, len, hd) = (y.radius_m as f32, y.length_m as f32, y.dome_m as f32);
    let p = &e.port;
    let (rh, rt, zi, zm, zb) =
        (p.hub_radius_m as f32, p.throat_radius_m as f32, p.iris_z_m as f32, p.mouth_z() as f32, p.back_z() as f32);
    let zc = zm - hd - len / 2.0;
    let pa = (y.pair_angle_deg as f32).to_radians();
    let s = y.spacing_m as f32;
    let centres = [Vec3::new(0.0, 0.0, zc), Vec3::new(s * pa.cos(), s * pa.sin(), zc)];
    let rho = (r * r + hd * hd) / (2.0 * hd);
    let tiled = move |a: f32, _: f32, rmax: f32, _: f32, along: f32| [rmax * a / tile, along / tile];
    let mut out = Soup::default();
    let mut beacons = Vec::new();
    for (i, c) in centres.iter().enumerate() {
        let f = Frame::z(*c);
        // The shell, the macro map once round it (u round, v along: the mockup's eBand), its panes glowing.
        let shell: Vec<[f32; 2]> = (0..=4).map(|k| [r, -len / 2.0 + len * k as f32 / 4.0]).collect();
        let wrap = move |a: f32, _: f32, _: f32, h: f32, _: f32| [a / tau, (h + len / 2.0) / len];
        out.lathe(&f, &shell, seg.round, 1.0, MACRO, [1.0; 3], &wrap);
        // The domes, the cap map once over each; a hole on the axis for the hub (the port) or a bearing.
        for d in [-1.0f32, 1.0] {
            let port = i == 0 && d > 0.0;
            let hole = if port { rh } else { 150.0 };
            let sc = d * (len / 2.0 - (rho - hd));
            let (b0, b1) = ((r / rho).asin(), (hole / rho).asin());
            let prof: Vec<[f32; 2]> = (0..=seg.dome_rings)
                .map(|j| {
                    let b = b0 + (b1 - b0) * j as f32 / seg.dome_rings as f32;
                    [rho * b.sin(), sc + d * rho * b.cos()]
                })
                .collect();
            let planar = move |a: f32, rr: f32, _: f32, _: f32, _: f32| {
                [0.5 + 0.5 * rr / r * a.cos(), 0.5 + 0.5 * rr / r * a.sin()]
            };
            out.lathe(&f, &prof, seg.round, d, CAP, [1.0; 3], &planar);
            // The seal collar where the dome's shell meets the hub or the bearing, flush round it.
            let zo = sc + d * (rho * rho - hole * hole).sqrt();
            let collar = [
                [hole, zo - d * 14.0],
                [hole + 16.0, zo - d * 6.0],
                [hole + 16.0, zo + d * 8.0],
                [hole, zo + d * 14.0],
            ];
            out.lathe(&f, &collar, seg.round, d, DETAIL, paint.orange, &tiled);
            if !port {
                // A bearing: a closed drum through the dome on its axis.
                let z0 = d * (len / 2.0 + hd - 80.0);
                let z1 = d * (len / 2.0 + hd + 120.0);
                out.lathe(&f, &[[150.0, z0], [150.0, z1], [0.0, z1]], seg.round / 2, d, DETAIL, paint.graphite, &tiled);
            }
        }
        // Three mirrors a cylinder, hinged at the far end of each window and opened toward the star: a flower.
        let open = (e.mirrors.open_deg as f32).to_radians();
        let lm = len * e.mirrors.length_frac as f32;
        let w = 2.0 * r * (std::f32::consts::PI / 6.0).sin();
        let mtile = look.outside.mirror_tile_m;
        for k in 0..3 {
            let th = k as f32 * tau / 3.0 + std::f32::consts::FRAC_PI_2;
            let x = Vec3::new(-th.sin(), th.cos(), 0.0);
            let rad = Vec3::new(th.cos(), th.sin(), 0.0);
            let dir = Vec3::Z * open.cos() + rad * open.sin();
            let up = dir.cross(x);
            let rot = Quat::from_mat3(&Mat3::from_cols(x, up, dir));
            let hinge = *c + rad * (r + 4.0) + Vec3::Z * (-len / 2.0 - hd * 0.15);
            let half = Vec3::new(w / 2.0, e.mirrors.thickness_m as f32 / 2.0, lm / 2.0);
            let graphite = paint.graphite;
            // The face toward the window and the star (+y in its own axes) is the mirror.
            let style = move |n: Vec3| if n.y > 0.5 { (MIRROR, [1.0; 3]) } else { (DETAIL, graphite) };
            out.block(hinge + dir * lm / 2.0, half, rot, mtile, &style);
        }
        // The farm ring round the sunward end: its tube, its pods and its spokes.
        let g = &e.ring;
        let (rr, tube, rz) = (g.radius_m as f32, g.tube_m as f32, len / 2.0 - g.inset_m as f32);
        let square = [
            [rr - tube, rz - tube],
            [rr + tube, rz - tube],
            [rr + tube, rz + tube],
            [rr - tube, rz + tube],
            [rr - tube, rz - tube],
        ];
        out.lathe(&f, &square, seg.round, 1.0, DETAIL, paint.graphite, &tiled);
        for k in 0..g.pods {
            let a = k as f32 * tau / g.pods as f32;
            let (pr, pl) = (g.pod_radius_m as f32, g.pod_length_m as f32 / 2.0);
            let pf = Frame::z(*c + Vec3::new(rr * a.cos(), rr * a.sin(), rz));
            let rgb = if k % 3 == 0 { paint.orange } else { paint.pearl };
            out.lathe(&pf, &[[0.0, -pl], [pr, -pl], [pr, pl], [0.0, pl]], 8, 1.0, DETAIL, rgb, &tiled);
        }
        for k in 0..g.spokes {
            let a = (k as f32 * 2.0 / 3.0 + 1.0 / 6.0) * std::f32::consts::PI;
            let d = Vec3::new(a.cos(), a.sin(), 0.0);
            out.beam(*c + d * r + Vec3::Z * rz, *c + d * rr + Vec3::Z * rz, 24.0, tile, paint.graphite);
        }
    }
    // The hub where it stands out of the dome, and the mouth's face closing it to the throat.
    let f0 = Frame::z(Vec3::ZERO);
    let dome_z = |rr: f32| centres[0].z + len / 2.0 - (rho - hd) + (rho * rho - rr * rr).sqrt();
    out.lathe(&f0, &[[rh, dome_z(rh) - 20.0], [rh, zm]], seg.round, 1.0, DETAIL, paint.graphite, &tiled);
    out.lathe(&f0, &[[rh, zm], [rt, zm]], seg.round, 1.0, DETAIL, paint.graphite, &tiled);
    // The rods across the pair at both ends, into the bearings, and their stubs (non-rotating).
    let front = centres.map(|c| Vec3::new(c.x, c.y, zm - 200.0));
    let back_z = zc - len / 2.0 - hd - 100.0;
    let back = centres.map(|c| Vec3::new(c.x, c.y, back_z));
    let tb = 40.0;
    out.beam(front[0] + Vec3::new(700.0 * pa.cos(), 700.0 * pa.sin(), 0.0), front[1], tb, tile, paint.pearl);
    out.beam(back[0], back[1], tb, tile, paint.pearl);
    out.beam(front[1], Vec3::new(front[1].x, front[1].y, zm - 20.0), tb * 0.8, tile, paint.graphite);
    for b in back {
        out.beam(b, Vec3::new(b.x, b.y, zc - len / 2.0 - hd), tb * 0.8, tile, paint.graphite);
    }
    for q in front.iter().chain(&back) {
        beacons.push(Beacon { at_m: (*q + Vec3::Y * 60.0).to_array(), radius_m: 12.0, rgb: glow.red });
    }

    // Inside: the round bay, its ribs, back wall and berths; the throat; the glowing parts.
    let b = &look.bay;
    let mut inside = Soup::default();
    let mut lit = Soup::default();
    // The bay's and the throat's own parts, apart: with the mockup's bake (`baked`, design 8b) they give way to it.
    let mut bay_in = Soup::default();
    let mut bay_lit = Soup::default();
    let wall_rows: Vec<[f32; 2]> = {
        let n = ((zi - zb) / 20.0).ceil().max(1.0) as usize;
        (0..=n).map(|k| [rt, zb + (zi - zb) * k as f32 / n as f32]).collect()
    };
    bay_in.lathe(&f0, &wall_rows, seg.bay, -1.0, DETAIL, paint.wall, &tiled);
    // The back wall, in rings so its light falls off.
    let back_rings: Vec<[f32; 2]> = (0..=4).map(|k| [rt * (1.0 - k as f32 / 4.0), zb]).collect();
    bay_in.lathe(&f0, &back_rings, seg.bay, 1.0, DETAIL, paint.wall, &tiled);
    let mut z = zb + b.rib_every_m / 2.0;
    while z < zi - 3.0 {
        let rib = [[rt, z - 3.0], [rt - 6.0, z - 3.0], [rt - 6.0, z + 3.0], [rt, z + 3.0]];
        bay_in.lathe(&f0, &rib, seg.bay, -1.0, DETAIL, paint.steel, &tiled);
        z += b.rib_every_m;
    }
    // Floods in rings round the wall, each ring a lit band.
    let mut lamps: Vec<(Vec3, f32, f32)> = Vec::new();
    let mut z = zb + b.flood_first_m;
    while z < zi - 10.0 {
        bay_lit.lathe(&f0, &[[rt - 1.5, z], [rt - 1.5, z + 4.0]], seg.bay, -1.0, DETAIL, glow.work, &tiled);
        for k in 0..b.flood_per_ring {
            let a = k as f32 * tau / b.flood_per_ring as f32;
            lamps.push((f0.at(a, rt - b.flood_inset_m, z + 2.0), b.flood_intensity, b.flood_reach_m));
        }
        z += b.flood_every_m;
    }
    for l in &b.axis_lamps {
        lamps.push((Vec3::new(0.0, 0.0, l[0]), l[1], l[2]));
    }
    // The back wall's window tiers, and the axial post with its lamp.
    let mut wy = -rt + b.window_every_m;
    while wy < rt - 20.0 {
        let half = (rt * rt - wy * wy).max(0.0).sqrt() - 24.0;
        if half > 30.0 {
            bay_lit.plain(
                Vec3::new(0.0, wy + 3.5, zb + 1.0),
                Vec3::new(half, 3.5, 0.4),
                Quat::IDENTITY,
                tile,
                glow.window,
            );
        }
        wy += b.window_every_m;
    }
    bay_in.plain(Vec3::new(0.0, 0.0, zb + 60.0), Vec3::new(4.0, 4.0, 60.0), Quat::IDENTITY, tile, paint.steel);
    bay_lit.plain(Vec3::new(0.0, 0.0, zb + 122.0), Vec3::new(9.0, 9.0, 2.0), Quat::IDENTITY, tile, glow.work);
    // The berths: a pier and pad under each ship, a gantry over it with its hoist and lamp. The Tern's own gantry
    // stands swung back to its upright, so the way up off the wall to the axis is clear (the departure's legs).
    let br = &e.berths;
    let mut docked = Vec::new();
    for (row, angle, ship) in &br.slots {
        let md = e_berth(*angle, br.radius_m, br.rows_z_m[*row]);
        let m = md.as_mat4();
        let (scale, rot, at) = m.to_scale_rotation_translation();
        debug_assert!((scale - Vec3::ONE).length() < 1e-4);
        let place = |v: Vec3| at + rot * v;
        let wy = -(rt - br.radius_m as f32) + 0.5;
        let box_at = |s: &mut Soup, lo: Vec3, hi: Vec3, rgb: [f32; 3]| {
            s.plain(place((lo + hi) / 2.0), (hi - lo) / 2.0, rot, tile, rgb);
        };
        box_at(&mut bay_in, Vec3::new(-5.0, wy, -20.0), Vec3::new(5.0, -10.0, 20.0), paint.steel);
        box_at(&mut bay_in, Vec3::new(-14.0, -10.0, -26.0), Vec3::new(14.0, -8.5, 26.0), paint.steel);
        box_at(&mut bay_in, Vec3::new(30.0, wy, -6.0), Vec3::new(40.0, 34.0, 6.0), paint.steel);
        let own = ship.as_deref() == Some("tern");
        if !own {
            box_at(&mut bay_in, Vec3::new(-40.0, 30.0, -6.0), Vec3::new(40.0, 36.0, 6.0), paint.hazard);
            box_at(&mut bay_in, Vec3::new(-8.0, 22.0, -4.0), Vec3::new(8.0, 30.0, 4.0), paint.hazard);
            box_at(&mut bay_lit, Vec3::new(-36.0, 28.9, -2.0), Vec3::new(36.0, 29.9, 2.0), glow.work);
        }
        let lamp_at = if own { Vec3::new(35.0, b.gantry_up_m, 0.0) } else { Vec3::new(0.0, b.gantry_up_m, 0.0) };
        lamps.push((place(lamp_at), b.gantry_intensity, b.gantry_reach_m));
        if let Some(key) = ship.as_deref() {
            if drawn_with_tern(sd_json, key) {
                docked.push(md);
                // Its running lights: red to port, green to starboard.
                for (q, rgb) in [(Vec3::new(22.6, 1.2, -2.0), glow.red), (Vec3::new(-22.6, 1.2, -2.0), glow.green)] {
                    bay_lit.plain(place(q), Vec3::splat(0.9), rot, tile, rgb);
                }
            }
        }
    }
    // The throat: its wall from the iris to the mouth, light rings and eight strips; the iris's rim.
    let n = ((zm - zi) / 40.0).ceil().max(1.0) as usize;
    let throat: Vec<[f32; 2]> = (0..=n).map(|k| [rt, zi + (zm - zi) * k as f32 / n as f32]).collect();
    bay_in.lathe(&f0, &throat, seg.bay, -1.0, DETAIL, paint.throat, &tiled);
    let mut z = zm - 30.0;
    while z > zi + 20.0 {
        bay_lit.lathe(&f0, &[[rt - 2.0, z - 1.5], [rt - 2.0, z + 1.5]], seg.bay, -1.0, DETAIL, glow.work, &tiled);
        for k in 0..b.throat_per_ring {
            let a = (k as f32 + 0.5) * tau / b.throat_per_ring as f32;
            lamps.push((f0.at(a, rt - 6.0, z), b.throat_intensity, b.throat_reach_m));
        }
        z -= b.throat_every_m;
    }
    for k in 0..8 {
        let a = (k as f32 + 0.5) * tau / 8.0;
        let at = f0.at(a, rt - 4.0, (zm + zi) / 2.0);
        bay_lit.plain(at, Vec3::new(2.5, 2.5, (zm - zi - 20.0) / 2.0), Quat::from_rotation_z(a), tile, glow.work);
    }
    bay_lit.lathe(&f0, &[[rt - 3.0, zi + 4.0], [rt - 3.0, zi + 8.0]], seg.bay, -1.0, DETAIL, glow.accent, &tiled);
    // The mouth's lit ring and the guide rings on the dome leading in, standing off their surfaces.
    lit.lathe(&f0, &[[rt + 22.0, zm + 4.0], [rt + 2.0, zm + 4.0]], seg.round, 1.0, DETAIL, glow.work, &tiled);
    if let Some(rings) = sd_json["versions"]["E"]["port"]["light_rings_m"].as_array() {
        for rr in rings.iter().filter_map(Value::as_f64) {
            let rr = rr as f32;
            let zr = dome_z(rr) + 8.0;
            lit.lathe(&f0, &[[rr + 4.0, zr], [rr - 4.0, zr]], seg.round, 1.0, DETAIL, glow.accent, &tiled);
        }
    }
    if !baked {
        inside.tris.append(&mut bay_in.tris);
        lit.tris.append(&mut bay_lit.tris);
    }
    let bay_light = |p: Vec3, nn: Vec3| {
        let k: f32 = lamps.iter().map(|(at, st, reach)| lamp(p, nn, *at, *st, *reach)).sum();
        [0, 1, 2].map(|c| (b.ambient[c] + b.flood[c] * k).min(2.0))
    };
    StationModel {
        outside: out.pack(OUTSIDE_SCALE, &outside_light(&look.outside, sun, planet)),
        inside: inside.pack(INSIDE_SCALE, &bay_light),
        glow: lit.pack(INSIDE_SCALE, &|_, _| [1.0; 3]),
        docked,
        beacons,
        placeholder: false,
    }
}

/// A number at a JSON pointer of the spacedock file (its shipped values are checked by a test; a missing one is 0).
fn at(v: &Value, ptr: &str) -> f32 {
    v.pointer(ptr).and_then(Value::as_f64).unwrap_or(0.0) as f32
}

/// A deterministic share 0-1 for `(i, j, seed)`: a placeholder rock's roughness.
fn hash01(i: u32, j: u32, seed: u32) -> f32 {
    let mut h = i.wrapping_mul(0x9e37_79b1) ^ j.wrapping_mul(0x85eb_ca77) ^ seed.wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

/// The placeholder of version A-D (patrol-mission design 12b): its outside's silhouette from the spacedock file's
/// sizes, doors shut, in the frame the file uses (+y north, +z the way the doors face), lit as E's outside.
pub fn placeholder(version: char, sd_json: &Value, look: &StationLook, sun: Vec3, planet: Vec3) -> StationModel {
    let (paint, glow) = (&look.paint, &look.glow);
    let n = look.segments.placeholder;
    let tile = look.outside.detail_tile_m * 4.0;
    let tiled = move |a: f32, _: f32, rmax: f32, _: f32, along: f32| [rmax * a / tile, along / tile];
    let v = |k: &str| format!("/versions/{version}/{k}");
    let mut s = Soup::default();
    let mut beacons = Vec::new();
    let pair = |x: &Value| -> Vec<[f32; 2]> {
        x.as_array()
            .map(|a| {
                a.iter().map(|p| [p[0].as_f64().unwrap_or(0.0) as f32, p[1].as_f64().unwrap_or(0.0) as f32]).collect()
            })
            .unwrap_or_default()
    };
    match version {
        'A' => {
            // Bastion: the cap (its crown and its underside about +y) on a ringed spine.
            let fy = Frame::y(Vec3::ZERO);
            let cap = &sd_json["versions"]["A"]["cap"];
            let (crown, under) = (pair(&cap["crown"]), pair(&cap["under"]));
            let rim = at(sd_json, &v("cap/radius_m"));
            let (lo, hi) = (at(sd_json, &v("cap/rim_y_m/0")), at(sd_json, &v("cap/rim_y_m/1")));
            s.lathe(&fy, &[[rim, lo], [rim, hi]], n, 1.0, DETAIL, paint.pearl, &tiled);
            s.lathe(&fy, &crown, n, 1.0, DETAIL, paint.pearl, &tiled);
            s.lathe(&fy, &under, n, -1.0, DETAIL, paint.graphite, &tiled);
            let core = at(sd_json, &v("spine/core_radius_m"));
            let (top, tip) = (at(sd_json, &v("spine/top_y_m")), at(sd_json, &v("spine/tip_y_m")));
            s.lathe(&fy, &[[core, top], [core, tip], [0.0, tip]], n, -1.0, DETAIL, paint.graphite, &tiled);
            if let Some(rings) = sd_json.pointer(&v("spine/rings")).and_then(Value::as_array) {
                for q in rings {
                    let (yy, rr, h) = (
                        q[0].as_f64().unwrap_or(0.0) as f32,
                        q[1].as_f64().unwrap_or(0.0) as f32,
                        q[2].as_f64().unwrap_or(0.0) as f32,
                    );
                    let prof = [[core, yy + h / 2.0], [rr, yy + h / 2.0], [rr, yy - h / 2.0], [core, yy - h / 2.0]];
                    s.lathe(&fy, &prof, n, -1.0, DETAIL, paint.pearl, &tiled);
                }
            }
            // The four doors in the rim, shut.
            let (dw, dh) = (at(sd_json, &v("doors/width_m")), at(sd_json, &v("doors/height_m")));
            if let Some(bs) = sd_json.pointer(&v("doors/bearings_deg")).and_then(Value::as_array) {
                for b in bs.iter().filter_map(Value::as_f64) {
                    let b = (b as f32).to_radians();
                    let c = Vec3::new(b.sin(), 0.0, b.cos()) * (rim + 2.0);
                    s.plain(c, Vec3::new(dw / 2.0, dh / 2.0, 2.0), Quat::from_rotation_y(b), tile, paint.graphite);
                }
            }
            beacons.push(Beacon {
                at_m: [0.0, crown.last().map_or(300.0, |p| p[1]) + 20.0, 0.0],
                radius_m: 10.0,
                rgb: glow.red,
            });
            beacons.push(Beacon { at_m: [0.0, tip - 20.0, 0.0], radius_m: 10.0, rgb: glow.green });
        }
        'B' => {
            // Wheel: the habitat ring about +z on its spokes round the hub drum; the hub's doors on its +z face.
            let fz = Frame::z(Vec3::ZERO);
            let (rr, wd, dp) = (
                at(sd_json, &v("ring/radius_m")),
                at(sd_json, &v("ring/width_m")) / 2.0,
                at(sd_json, &v("ring/depth_m")) / 2.0,
            );
            let sq = [[rr - dp, -wd], [rr + dp, -wd], [rr + dp, wd], [rr - dp, wd], [rr - dp, -wd]];
            s.lathe(&fz, &sq, n, 1.0, DETAIL, paint.pearl, &tiled);
            let (hr, z0, z1) =
                (at(sd_json, &v("hub/radius_m")), at(sd_json, &v("hub/z_m/0")), at(sd_json, &v("hub/z_m/1")));
            s.lathe(&fz, &[[0.0, z0], [hr, z0], [hr, z1], [0.0, z1]], n, 1.0, DETAIL, paint.graphite, &tiled);
            let spokes = at(sd_json, &v("ring/spokes")).max(1.0) as usize;
            let sr = at(sd_json, &v("ring/spoke_radius_m"));
            for k in 0..spokes {
                let a = k as f32 * std::f32::consts::TAU / spokes as f32;
                let d = Vec3::new(a.cos(), a.sin(), 0.0);
                s.beam(d * hr, d * (rr - dp), sr * 2.0, tile, paint.graphite);
            }
            let (dw, dh) = (at(sd_json, &v("doors/width_m")), at(sd_json, &v("doors/height_m")));
            s.plain(
                Vec3::new(0.0, 0.0, z1 + 2.0),
                Vec3::new(dw / 2.0, dh / 2.0, 2.0),
                Quat::IDENTITY,
                tile,
                paint.graphite,
            );
            beacons.push(Beacon { at_m: [0.0, hr + 20.0, z1], radius_m: 10.0, rgb: glow.red });
            beacons.push(Beacon { at_m: [0.0, hr + 20.0, z0], radius_m: 10.0, rgb: glow.green });
        }
        'C' => {
            // Drydock: the long armoured hall, an octagon in section, ribbed; its doors at +z.
            let (hw, hh) = (at(sd_json, &v("hull/width_m")) / 2.0, at(sd_json, &v("hull/height_m")) / 2.0);
            let (z0, z1) = (at(sd_json, &v("hull/z_m/0")), at(sd_json, &v("hull/z_m/1")));
            let ch = at(sd_json, &v("hull/chamfer_m"));
            let section = |grow: f32| -> Vec<Vec3> {
                let (w, h, c) = (hw + grow, hh + grow, ch);
                [[w, h - c], [w - c, h], [-w + c, h], [-w, h - c], [-w, -h + c], [-w + c, -h], [w - c, -h], [w, -h + c]]
                    .map(|q| Vec3::new(q[0], q[1], 0.0))
                    .to_vec()
            };
            let prism = |s: &mut Soup, sec: &[Vec3], z0: f32, z1: f32, caps: bool, rgb: [f32; 3]| {
                for i in 0..sec.len() {
                    let (a, b) = (sec[i], sec[(i + 1) % sec.len()]);
                    let mid = (a + b) / 2.0;
                    let w = (b - a).length() / tile;
                    let q = [a + Vec3::Z * z0, b + Vec3::Z * z0, b + Vec3::Z * z1, a + Vec3::Z * z1];
                    s.quad(q, [[0.0, 0.0], [w, 0.0], [w, (z1 - z0) / tile], [0.0, (z1 - z0) / tile]], DETAIL, rgb, mid);
                    if caps {
                        for (zz, out) in [(z0, -Vec3::Z), (z1, Vec3::Z)] {
                            let t = [[0.5, 0.5], [0.0, 0.0], [1.0, 0.0]];
                            s.tri([Vec3::Z * zz, a + Vec3::Z * zz, b + Vec3::Z * zz], t, DETAIL, rgb, out);
                        }
                    }
                }
            };
            prism(&mut s, &section(0.0), z0, z1, true, paint.pearl);
            let every = at(sd_json, &v("hull/rib_every_m")).max(40.0);
            let mut z = z0 + every;
            while z < z1 - 10.0 {
                prism(&mut s, &section(6.0), z - 5.0, z + 5.0, false, paint.graphite);
                z += every;
            }
            let (dw, dh) = (at(sd_json, &v("doors/width_m")), at(sd_json, &v("doors/height_m")));
            s.plain(
                Vec3::new(0.0, 0.0, z1 + 2.0),
                Vec3::new(dw / 2.0, dh / 2.0, 2.0),
                Quat::IDENTITY,
                tile,
                paint.graphite,
            );
            beacons.push(Beacon { at_m: [0.0, hh + 20.0, z1], radius_m: 10.0, rgb: glow.red });
            beacons.push(Beacon { at_m: [0.0, hh + 20.0, z0], radius_m: 10.0, rgb: glow.green });
        }
        _ => {
            // Hollow: the rough rock, its armoured gate at +z.
            let radii = Vec3::new(
                at(sd_json, &v("rock/radii_m/0")),
                at(sd_json, &v("rock/radii_m/1")),
                at(sd_json, &v("rock/radii_m/2")),
            );
            let rough = at(sd_json, &v("rock/rough"));
            let seed = at(sd_json, &v("rock/seed")) as u32;
            let (rings, sides) = (n / 2, n);
            let point = |i: usize, j: usize| {
                let (th, ph) = (
                    std::f32::consts::PI * i as f32 / rings as f32,
                    std::f32::consts::TAU * (j % sides) as f32 / sides as f32,
                );
                let k = if i == 0 || i == rings {
                    1.0
                } else {
                    1.0 + rough * (hash01(i as u32, (j % sides) as u32, seed) * 2.0 - 1.0)
                };
                Vec3::new(th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()) * radii * k
            };
            for i in 0..rings {
                for j in 0..sides {
                    let q = [point(i, j), point(i + 1, j), point(i + 1, j + 1), point(i, j + 1)];
                    let mid = (q[0] + q[1] + q[2] + q[3]) / 4.0;
                    let t = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
                    s.quad(q, t, DETAIL, paint.rock, mid);
                }
            }
            let gz = at(sd_json, &v("gate/z_m"));
            let plate = Vec3::new(
                at(sd_json, &v("gate/plate_m/0")),
                at(sd_json, &v("gate/plate_m/1")),
                at(sd_json, &v("gate/plate_m/2")),
            ) / 2.0;
            s.plain(Vec3::new(0.0, 0.0, gz), plate, Quat::IDENTITY, tile, paint.graphite);
            let (dw, dh) = (at(sd_json, &v("gate/width_m")), at(sd_json, &v("gate/height_m")));
            s.plain(
                Vec3::new(0.0, 0.0, gz + plate.z + 2.0),
                Vec3::new(dw / 2.0, dh / 2.0, 2.0),
                Quat::IDENTITY,
                tile,
                paint.steel,
            );
            beacons.push(Beacon { at_m: [0.0, plate.y + 20.0, gz + plate.z], radius_m: 10.0, rgb: glow.red });
        }
    }
    StationModel {
        outside: s.pack(OUTSIDE_SCALE, &outside_light(&look.outside, sun, planet)),
        inside: Vec::new(),
        glow: Vec::new(),
        docked: Vec::new(),
        beacons,
        placeholder: true,
    }
}

/// Read a texture layer: an 8-bit RGBA PNG at `root`/`rel`, made `px` square (shrunk by the box filter, or grown by
/// repeating texels: `sc_core::mips`).
fn layer(root: &std::path::Path, rel: &str, px: u32) -> Result<Vec<u8>, String> {
    let path = root.join(rel);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().map_err(|e| format!("{rel}: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size().ok_or_else(|| format!("{rel}: too large"))?];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("{rel}: {e}"))?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight || info.width != info.height {
        return Err(format!("{rel}: must be a square 8-bit RGBA PNG"));
    }
    buf.truncate(info.buffer_size());
    let w = info.width;
    if w >= px && w % px == 0 {
        Ok(if w == px { buf } else { sc_core::mips::shrink(&buf, w, px) })
    } else if px % w == 0 {
        Ok(sc_core::mips::grow(&buf, w, px))
    } else {
        Err(format!("{rel}: {w} px does not divide into or by {px} px"))
    }
}

/// The station layers' array, level 0 of every layer in order (detail, cap, mirror, macro), from `root`.
pub fn layers(look: &StationLook, root: &std::path::Path) -> Result<Vec<u8>, String> {
    let t = &look.textures;
    let mut out = Vec::new();
    for rel in [&t.detail, &t.cap, &t.mirror, &t.macro_map] {
        out.extend(layer(root, rel, look.layer_px)?);
    }
    Ok(out)
}

/// A station on the GPU.
struct StationGpu {
    outside: Vec<Mesh>,
    inside: Vec<Mesh>,
    glow: Vec<Mesh>,
    docked: Vec<DMat4>,
    beacons: BeaconLamps,
    /// Its bay as the mockup baked it, when `compiled/stations.bays` holds its version (design 8b).
    baked: Option<BakedBay>,
}

/// A bay baked by the mockup (spacedock design 8b), from `compiled/stations.bays`: its inside and its fixtures' faces,
/// each with its frame (the origin its stored positions are measured from, the station's frame, and its scale), and its
/// light probes in the station's frame.
struct BakedBay {
    inside: (Mesh, [f64; 3], f32),
    glow: (Mesh, [f64; 3], f32),
    /// A-D: the station's outside with its doors shut, lit by the star at load.
    outside: Option<(Mesh, [f64; 3], f32)>,
    probes: std::rc::Rc<sc_core::probes::ProbeSet>,
}

/// A station's lighting state's weights (normal, red alert, emergency): normal until a station has an alert of its own
/// (tasks 2.6).
const STATION_STATE: [f32; 3] = [1.0, 0.0, 0.0];

/// The bays file, relative to the repository's root (`sc-tools bayc` writes it; gitignored, like the deck).
pub const BAYS_FILE: &str = "compiled/stations.bays";

/// A mockup station's outside (`m`'s packed vertices `v`) with its paints lit by `light` (the station's frame: a point
/// and its face's normal, to display multipliers), a colour whose alpha is 0 being its own light and left as it is, and
/// one whose alpha is 128 facing into the bay and lit by `probes` there instead; and its farthest vertex from the
/// station's origin, metres.
fn lit_outside(
    v: &[u8],
    m: &sc_core::bay::BayMesh,
    light: &dyn Fn(Vec3, Vec3) -> [f32; 3],
    probes: &sc_core::probes::ProbeSet,
) -> (Vec<u8>, f32) {
    let grids = probes.grids();
    let o = DVec3::from_array(m.origin_m);
    let mut out = v.to_vec();
    let mut reach = 0.0f32;
    for q in out.chunks_exact_mut(sc_core::vertex::DECK_VERTEX_BYTES) {
        let (p, n) = sc_core::vertex::unpack_position_normal(q);
        let at = (o + Vec3::from(p).as_dvec3() * f64::from(m.scale)).as_vec3();
        reach = reach.max(at.length());
        if q[15] == 0 {
            continue;
        }
        let n = Vec3::from(n).normalize_or_zero();
        let l = if q[15] == 128 {
            // The bay's light in front of the face (a probe on the wall itself is inside it): the nearest the grids
            // hold, a door in its reveal reaching out to the bay.
            [4.0f32, 12.0, 24.0, 44.0]
                .iter()
                .find_map(|d| sc_core::probes::sample_ship(&grids, (at + n * *d).as_dvec3().to_array(), STATION_STATE))
                .map_or([0.1; 3], |c| crate::outside::cube_at(&c, n))
        } else {
            light(at, n)
        };
        for set in 0..3 {
            for c in 0..3 {
                let b = &mut q[12 + set * 4 + c];
                *b = (f32::from(*b) * l[c]).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (out, reach)
}

/// The bays file under `root`, its texture array uploaded: None when there is none (the stand-in light shows, and it
/// says so), an error when it is there and does not read.
fn load_bays(r: &Renderer, root: &std::path::Path) -> Result<Option<(Vec<u8>, TextureArray)>, String> {
    let Ok(bytes) = std::fs::read(root.join(BAYS_FILE)) else {
        println!("sc-client: no {BAYS_FILE} (sc-tools bayc): the bays show their stand-in light");
        return Ok(None);
    };
    let tex = {
        let b = sc_core::bay::read(&bytes).map_err(|e| format!("{BAYS_FILE}: {e}"))?;
        let t = &b.index.textures;
        r.make_texture_array_mips(t.size_px, t.layers, &b.texture_levels())
    };
    Ok(Some((bytes, tex)))
}

/// Every station of a system, ready to draw (patrol-mission design 12b).
pub struct StationsView {
    tex: TextureArray,
    /// The baked bays' texture array, when `compiled/stations.bays` is there.
    bay_tex: Option<TextureArray>,
    stations: Vec<StationGpu>,
    look: StationLook,
    /// Version E's bay and throat, for whether the Tern is under its roof and how far down the throat it is.
    e_bore: Bore,
    /// Each station's version, in the system file's order.
    versions: Vec<char>,
    /// Their triangles, all of them.
    pub triangles: usize,
}

/// Version E's bore: the bay and the throat, one round section on the axis (the station's frame, metres).
#[derive(Debug, Clone, Copy)]
struct Bore {
    radius_m: f64,
    back_z: f64,
    iris_z: f64,
    mouth_z: f64,
}

/// The light on a docked ship whose middle is at `p` (its station's frame, metres), turned by `turn`: its bay's probes
/// (none: the stand-in).
fn berth_light(
    probes: Option<&std::rc::Rc<sc_core::probes::ProbeSet>>,
    bay: &BayLight,
    p: DVec3,
    turn: Quat,
    stand_in: HullLight,
) -> HullLight {
    let Some(probes) = probes else { return stand_in };
    let Some(cube) = probes.sample(p.to_array(), STATION_STATE) else { return stand_in };
    let cube = crate::outside::turn_cube(&cube, turn).map(|c| c.map(|x| x.min(bay.hull_probe_max)));
    let each = ProbeLight { probes: probes.clone(), at: p, turn, max: bay.hull_probe_max, state: STATION_STATE };
    HullLight::probes(cube, stand_in, 0.0).with_bay(each)
}

/// The light on a hull under E's roof whose middle is at `p` (the station's frame, metres), turned by `turn` (the hull's
/// frame to the station's), from the bay's `probes` (StationsView::hull_light): their cube at `p`, each axis held under
/// `bay.hull_probe_max`; down the throat the cube at the iris giving way to `star` toward the mouth; the stand-in from
/// the hull's own up where there are no probes.
fn roof_light(
    b: Bore,
    probes: Option<&std::rc::Rc<sc_core::probes::ProbeSet>>,
    bay: &BayLight,
    p: DVec3,
    turn: Quat,
    star: HullLight,
) -> HullLight {
    let stand_in = HullLight::from_way(Vec3::Y, bay.ship_light);
    let Some(probes) = probes else { return stand_in };
    // The probes run from the back wall to the mouth (the throat is baked with the bay); down the throat the star's
    // light takes a growing share toward the mouth, where the Tern comes out into the sun.
    let share = ((p.z - b.iris_z) / (b.mouth_z - b.iris_z)).clamp(0.0, 1.0) as f32;
    let Some(cube) = probes.sample(p.to_array(), STATION_STATE) else {
        return if share > 0.0 { star } else { stand_in };
    };
    let cube = crate::outside::turn_cube(&cube, turn).map(|c| c.map(|x| x.min(bay.hull_probe_max)));
    // In the bay the hull is lit at each vertex from the probes round it (design 8b); the cube at its middle carries
    // the throat's blend.
    let each = ProbeLight { probes: probes.clone(), at: p, turn, max: bay.hull_probe_max, state: STATION_STATE };
    HullLight::probes(cube, star, share).with_bay(each)
}

/// Where the eye may stand off the Tern, metres: the chase camera's stand and the scripted shots' eyes are within it.
const EYE_OFF_TERN_M: f64 = 2_000.0;

/// Whether a station whose farthest vertex is `reach_m` from its origin, drawn while that origin is within `draw_m` of
/// the Tern, always lies inside `sc_render::depth::SOLID_REACH_M` of the eye, where it stands in front of the sky layer
/// (sky-behind-solids 2). Checked at load, so a bigger station or a longer draw distance fails loudly.
fn fits_the_far_plane(draw_m: f64, reach_m: f32) -> Result<(), String> {
    let farthest = draw_m + f64::from(reach_m) + EYE_OFF_TERN_M;
    let limit = f64::from(sc_render::depth::SOLID_REACH_M);
    if farthest <= limit {
        Ok(())
    } else {
        Err(format!("{LOOK_FILE}: draw_m {draw_m} m and a station's reach of {reach_m} m put it {farthest} m from the eye, past {limit} m, where the sky would show through it"))
    }
}

impl StationsView {
    /// Generate and upload every station of `sys` (their sizes from `sd` and `sd_json`, the spacedock file; the look
    /// from `look`, the textures under `root`), each lit by the star and the planet from where it stands.
    pub fn new(
        r: &Renderer,
        sys: &sc_core::system::SystemFile,
        look: StationLook,
        root: &std::path::Path,
    ) -> Result<Self, String> {
        let sd = Spacedock::shipped();
        let sd_json: Value = serde_json::from_str(sc_core::spacedock::SHIPPED).map_err(|e| e.to_string())?;
        let (sd, sd_json) = (&sd, &sd_json);
        let level0 = layers(&look, root)?;
        let levels = sc_core::mips::mip_chain(level0, look.layer_px, LAYERS);
        let refs: Vec<&[u8]> = levels.iter().map(Vec::as_slice).collect();
        let tex = r.make_texture_array_mips(look.layer_px, LAYERS, &refs);
        let bays = load_bays(r, root)?;
        let bays_read = match &bays {
            Some((bytes, _)) => Some(sc_core::bay::read(bytes).map_err(|e| format!("{BAYS_FILE}: {e}"))?),
            None => None,
        };
        let mut stations = Vec::new();
        let mut versions = Vec::new();
        let mut triangles = 0;
        for st in &sys.stations {
            let pos = sys.station_pos(st);
            let inv = sys.station_rot(st).inverse();
            let sun = (inv * -pos.normalize()).as_vec3();
            let planet = sys.sky_from(pos).planet.map_or(-Vec3::Y, |(_, d, _)| (inv * d).as_vec3());
            let version = st.version.chars().next().unwrap_or('E');
            // A placeholder has no bay to bake (its doors are shut); E takes the mockup's baked bay when the file has it.
            // E takes the mockup's baked bay and throat into its own model; A-D, when the file holds them whole (their
            // outsides with the doors shut), are the mockup's stations, the placeholder giving only its beacons.
            let key = version.to_string();
            let bay = bays_read.as_ref().and_then(|b| b.bay(&key).map(|bay| (b, bay)));
            let bay = bay.filter(|(_, bay)| version == 'E' || bay.outside.is_some());
            let mut m = if version == 'E' {
                oneill(&sd.oneill, sd_json, &look, sun, planet, bay.is_some())
            } else {
                placeholder(version, sd_json, &look, sun, planet)
            };
            let mut outside_reach = 0.0f32;
            if let (Some((_, b)), true) = (bay, version != 'E') {
                m = StationModel {
                    outside: Vec::new(),
                    inside: Vec::new(),
                    glow: Vec::new(),
                    docked: b
                        .docked
                        .iter()
                        .filter(|d| drawn_with_tern(sd_json, &d.fleet))
                        .map(|d| DMat4::from_cols_array(&d.pose))
                        .collect(),
                    beacons: m.beacons,
                    placeholder: false,
                };
            }
            if m.placeholder {
                println!("sc-client: {} (version {version}) is drawn as a placeholder", st.name);
            }
            triangles += m.triangles();
            // A part with nothing in it (E's inside once the bay is baked) uploads nothing.
            let up = |ms: &[MeshBuilder]| {
                ms.iter()
                    .filter(|b| b.triangles() > 0)
                    .map(|b| r.make_mesh(&b.vertices, Indices::U16(&b.indices)))
                    .collect()
            };
            let baked = bay.map(|(b, bay)| {
                let mesh = |m: &sc_core::bay::BayMesh| {
                    (r.make_mesh(b.vertices_of(m), Indices::U32(&b.indices_of(m))), m.origin_m, m.scale)
                };
                triangles += (bay.inside.index_count + bay.glow.index_count) as usize / 3;
                // The outside's paints lit by the star and the planet from where the station stands, as E's own are; its
                // faces into the bay by the bay's probes.
                let probes = std::rc::Rc::new(sc_core::probes::ProbeSet::new(&b.grids(bay)));
                let outside = bay.outside.as_ref().map(|o| {
                    triangles += o.index_count as usize / 3;
                    let light = outside_light(&look.outside, sun, planet);
                    let (v, reach) = lit_outside(b.vertices_of(o), o, &light, &probes);
                    outside_reach = reach;
                    (r.make_mesh(&v, Indices::U32(&b.indices_of(o))), o.origin_m, o.scale)
                });
                BakedBay { inside: mesh(&bay.inside), glow: mesh(&bay.glow), outside, probes }
            });
            fits_the_far_plane(look.draw_m, m.reach_m().max(outside_reach)).map_err(|e| format!("{}: {e}", st.name))?;
            stations.push(StationGpu {
                outside: up(&m.outside),
                inside: up(&m.inside),
                glow: up(&m.glow),
                docked: m.docked,
                beacons: BeaconLamps::new(r, &m.beacons),
                baked,
            });
            versions.push(version);
        }
        let p = &sd.oneill.port;
        let e_bore = Bore { radius_m: p.throat_radius_m, back_z: p.back_z(), iris_z: p.iris_z_m, mouth_z: p.mouth_z() };
        let bay_tex = bays.map(|(_, t)| t);
        Ok(Self { tex, bay_tex, stations, look, e_bore, versions, triangles })
    }

    /// How far off a station is drawn, metres.
    pub fn draw_m(&self) -> f64 {
        self.look.draw_m
    }

    /// Whether `p` (the station's own frame, metres) is under station `index`'s roof: in E's bay or throat, where the
    /// floods light a ship and the star does not.
    pub fn under_roof(&self, index: usize, p: DVec3) -> bool {
        let b = self.e_bore;
        self.versions.get(index) == Some(&'E') && p.x.hypot(p.y) < b.radius_m && p.z > b.back_z && p.z < b.mouth_z
    }

    /// Station `index`'s docked ships drawn with the Tern's model: their poses in its frame.
    pub fn docked(&self, index: usize) -> &[DMat4] {
        self.stations.get(index).map_or(&[], |g| g.docked.as_slice())
    }

    /// The light on a hull whose middle is at `p` in station `index`'s frame (metres), turned by `turn` (the hull's
    /// frame to the station's), when it is under the station's roof; None outside, where the star lights it. In a
    /// baked bay, the probes' cube at its middle (`sc_core::probes`, design 8b); down the throat, from the cube at the
    /// iris to `star` (the star's light in the hull's frame) by how far it has come from the iris to the mouth; with no
    /// baked bay, the stand-in's light from the hull's own up.
    pub fn hull_light(&self, index: usize, p: DVec3, turn: Quat, star: HullLight) -> Option<HullLight> {
        if !self.under_roof(index, p) {
            return None;
        }
        let probes = self.stations.get(index).and_then(|g| g.baked.as_ref()).map(|b| &b.probes);
        Some(roof_light(self.e_bore, probes, &self.look.bay, p, turn, star))
    }

    /// Draw station `index` (the system file's order) into the current pass of `t` through `vp` (a view at the eye),
    /// its frame at `rel` from the eye and turned by `rot`; its beacons as `b` blinks them at `time_s`; its docked ships
    /// with `hull` (the Tern's model), lit from the bay round them (`hull_light`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        r: &mut Renderer,
        t: &Target,
        vp: Mat4,
        index: usize,
        rel: Vec3,
        rot: Quat,
        b: &Beacons,
        time_s: f64,
        hull: Option<&HullView>,
    ) {
        let Some(g) = self.stations.get(index) else { return };
        let frame = Mat4::from_rotation_translation(rot, rel);
        let params = |scale: f32, panes: bool| DeckParams {
            mvp: vp * frame * Mat4::from_scale(Vec3::splat(scale)),
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: Vec3::Z,
            flash: 0.0,
            panel_first: if panes { u32::from(MACRO) } else { u32::MAX },
            panel_glow: self.look.outside.panes_glow,
            clip_y: f32::MAX,
            relight: NO_RELIGHT,
        };
        for m in &g.outside {
            r.draw_deck(t, m, DeckProgram::Textured, &params(OUTSIDE_SCALE, true), Some(&self.tex));
        }
        for m in &g.inside {
            r.draw_deck(t, m, DeckProgram::Textured, &params(INSIDE_SCALE, false), Some(&self.tex));
        }
        for m in &g.glow {
            r.draw_deck(t, m, DeckProgram::Flat, &params(INSIDE_SCALE, false), None);
        }
        // The baked bay: its inside and its fixtures' faces, each from its own origin and scale, in the station's state.
        if let (Some(bay), Some(tex)) = (&g.baked, &self.bay_tex) {
            for (m, origin, scale) in [Some(&bay.inside), Some(&bay.glow), bay.outside.as_ref()].into_iter().flatten() {
                let o = Vec3::new(origin[0] as f32, origin[1] as f32, origin[2] as f32);
                let p = DeckParams {
                    mvp: vp * frame * Mat4::from_translation(o) * Mat4::from_scale(Vec3::splat(*scale)),
                    state_weights: STATION_STATE,
                    ..params(1.0, false)
                };
                r.draw_deck(t, m, DeckProgram::Textured, &p, Some(tex));
            }
        }
        // The docked ships, lit from the bay round their middles (they lie in it, never in the throat).
        if let Some(h) = hull {
            for pose in &g.docked {
                let (_, turn, at) = pose.to_scale_rotation_translation();
                let stand_in = HullLight::from_way(Vec3::Y, self.look.bay.ship_light);
                // A docked ship lies in its bay: lit at each vertex by that bay's probes, whichever version it is.
                let probes = g.baked.as_ref().map(|b| &b.probes);
                let light = berth_light(probes, &self.look.bay, at, turn.as_quat(), stand_in);
                h.draw_posed(r, t, vp, frame * pose.as_mat4(), &light);
            }
        }
        g.beacons.draw(r, t, vp, rel, rot, b, time_s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOOK: &str = include_str!("../../../data/space/station_models.json");
    const SYSTEM: &str = include_str!("../../../data/space/systems/shoals.json");

    fn look() -> StationLook {
        sc_core::data::parse(LOOK_FILE, LOOK).expect("the shipped station look loads")
    }

    fn json() -> Value {
        serde_json::from_str(sc_core::spacedock::SHIPPED).unwrap()
    }

    fn tamsin() -> StationModel {
        oneill(&Spacedock::shipped().oneill, &json(), &look(), Vec3::Z, -Vec3::Y, false)
    }

    #[test]
    fn with_the_mockups_bake_tamsin_leaves_out_its_own_bay_and_throat_and_keeps_its_berths() {
        // Design 8b: the baked bay and throat replace the stand-in's walls, ribs, berths and fixtures.
        let (own, baked) = (tamsin(), oneill(&Spacedock::shipped().oneill, &json(), &look(), Vec3::Z, -Vec3::Y, true));
        let tris = |m: &[MeshBuilder]| m.iter().map(MeshBuilder::triangles).sum::<usize>();
        assert!(tris(&own.inside) > 0 && tris(&baked.inside) == 0, "the bay and the throat go");
        assert!(
            tris(&baked.glow) > 0 && tris(&baked.glow) < tris(&own.glow),
            "the mouth's ring and the dome's guide rings stay (they are outside)"
        );
        assert_eq!(baked.docked, own.docked, "the docked ships keep their berths");
        assert_eq!(tris(&baked.outside), tris(&own.outside), "the outside is not the bay's");
    }

    /// A 2 x 1 x 1 probe grid 20 m apart from `origin`: the first probe's +Y face `up`, every other face and probe 0.4.
    fn probes(origin: [f64; 3], up: f32) -> std::rc::Rc<sc_core::probes::ProbeSet> {
        let mut data = vec![0u8; 2 * sc_core::probes::PROBE_BYTES];
        for (k, probe) in data.chunks_exact_mut(sc_core::probes::PROBE_BYTES).enumerate() {
            for axis in 0..6 {
                let v = if k == 0 && axis == 2 { up } else { 0.4 };
                for st in 0..3 {
                    for c in 0..3 {
                        probe[axis * 9 + st * 3 + c] = (v / 2.0 * 255.0).round() as u8;
                    }
                }
            }
            probe[54] = 1;
        }
        let grid = sc_core::probes::ProbeGrid { origin_m: origin, spacing_m: 20.0, dims: [2, 1, 1], data: &data };
        std::rc::Rc::new(sc_core::probes::ProbeSet::new(&[grid]))
    }

    #[test]
    fn a_hull_in_the_bay_takes_the_probes_light_turned_into_its_frame_and_held_under_the_cap() {
        let bore = Bore { radius_m: 250.0, back_z: -600.0, iris_z: 0.0, mouth_z: 320.0 };
        let bay = &look().bay;
        let set = probes([0.0, 0.0, -100.0], 1.0);
        let star = HullLight::from_way(Vec3::X, 1.0);
        let at_probe = DVec3::new(0.0, 0.0, -100.0);
        let l = roof_light(bore, Some(&set), bay, at_probe, Quat::IDENTITY, star.clone());
        let cube = l.cube.expect("the probes light it");
        assert!((cube[2][0] - 1.0).abs() < 0.01 && (cube[3][0] - 0.4).abs() < 0.01, "the probe's own cube: {cube:?}");
        assert_eq!(l.star_share, 0.0, "no star in the sealed bay");
        // Upside down (half a turn about its bow), the hull's up faces the bay's down.
        let flip =
            roof_light(bore, Some(&set), bay, at_probe, Quat::from_rotation_z(std::f32::consts::PI), star.clone());
        let c = flip.cube.expect("lit");
        assert!((c[3][0] - 1.0).abs() < 0.01 && (c[2][0] - 0.4).abs() < 0.01, "turned into the hull's frame: {c:?}");
        // A pool brighter than the cap is held under it.
        let hot = probes([0.0, 0.0, -100.0], 1.9);
        let h = roof_light(bore, Some(&hot), bay, at_probe, Quat::IDENTITY, star.clone()).cube.expect("lit");
        assert!((h[2][0] - bay.hull_probe_max).abs() < 0.01, "held at {}: {h:?}", bay.hull_probe_max);
    }

    #[test]
    fn down_the_throat_the_bays_light_gives_way_to_the_stars() {
        let bore = Bore { radius_m: 250.0, back_z: -600.0, iris_z: 0.0, mouth_z: 320.0 };
        let bay = &look().bay;
        let set = probes([0.0, 0.0, 155.0], 1.0); // the throat is baked: its probes run to the mouth
        let star = HullLight::from_way(Vec3::X, 1.0);
        let half = roof_light(bore, Some(&set), bay, DVec3::new(0.0, 0.0, 160.0), Quat::IDENTITY, star.clone());
        assert!(half.cube.is_some() && (half.star_share - 0.5).abs() < 1e-6, "halfway, half the star's");
        assert_eq!(half.sun, Vec3::X, "the star's way");
        let none = roof_light(bore, None, bay, DVec3::new(0.0, 0.0, -100.0), Quat::IDENTITY, star.clone());
        assert!(none.cube.is_none() && none.bay.is_none(), "no baked bay: the stand-in");
        assert_eq!((none.sun, none.strength), (Vec3::Y, bay.ship_light), "from its own up");
    }

    #[test]
    fn tamsin_station_fits_its_budget_in_three_draws_and_two_ships() {
        // patrol-mission design 13 and spacedock design 9: the pair, the hub and the bay under 25,000 triangles in its
        // outside, inside and glow (one draw each while each part fits 16-bit indices).
        let m = tamsin();
        assert!(m.triangles() <= 25_000, "Tamsin Station: {} triangles", m.triangles());
        assert_eq!((m.outside.len(), m.inside.len(), m.glow.len()), (1, 1, 1), "one draw a part");
        assert_eq!(m.docked.len(), 2, "the sister Heron and the rescue Heron (the other classes have no model yet)");
    }

    #[test]
    fn every_station_stands_in_front_of_the_sky_to_the_edge_of_its_draw_distance() {
        // Regression (sky-behind-solids 2): the outside's far plane was the draw distance, 80 km, and Tamsin Station
        // reaches 12.9 km from its origin, so from 67 km out the far plane cut it open and the stars showed through it.
        let l = look();
        let mut models = vec![('E', tamsin())];
        models.extend(['A', 'B', 'C', 'D'].map(|v| (v, placeholder(v, &json(), &l, Vec3::Z, -Vec3::Y))));
        for (v, m) in models {
            assert!(fits_the_far_plane(l.draw_m, m.reach_m()).is_ok(), "{v} reaches {} m", m.reach_m());
        }
        assert!(fits_the_far_plane(l.draw_m, 50_000.0).is_err(), "a station too big for its draw distance is refused");
    }

    #[test]
    fn every_placeholder_is_flagged_and_small() {
        for v in ['A', 'B', 'C', 'D'] {
            let m = placeholder(v, &json(), &look(), Vec3::Z, -Vec3::Y);
            assert!(m.placeholder && m.inside.is_empty() && m.docked.is_empty(), "{v} is a placeholder outside");
            assert!((100..=4_000).contains(&m.triangles()), "{v}: {} triangles", m.triangles());
        }
        assert!(!tamsin().placeholder);
    }

    #[test]
    fn the_players_berth_in_the_system_file_is_spacedocks_tern_slot() {
        // Validate the real artifacts: the berth the patrol starts in (shoals.json) is the bay's slot the mockup puts the
        // Tern in (spacedock.json), so the gantries and piers drawn round it are where the Tern lies.
        let sys: sc_core::system::SystemFile = sc_core::data::parse("shoals.json", SYSTEM).unwrap();
        let st = sys.station("tamsin_station").unwrap();
        let b = &st.berths[0];
        let e = Spacedock::shipped().oneill;
        let (row, angle, _) = e.berths.slots.iter().find(|s| s.2.as_deref() == Some("tern")).unwrap();
        let slot = e_berth(*angle, e.berths.radius_m, e.berths.rows_z_m[*row]);
        let (_, rot, pos) = slot.to_scale_rotation_translation();
        let (at, turn) = sys.berth_pose(st, b);
        let inv = sys.station_rot(st).inverse();
        let (at, turn) = (inv * (at - sys.station_pos(st)), inv * turn);
        assert!(pos.distance(at) < 1e-3, "the berth at {at}, the slot at {pos}");
        let hpr = turn;
        assert!(rot.angle_between(hpr) < 1e-6, "the berth's attitude is the slot's");
    }

    #[test]
    fn a_docked_ship_lies_keel_to_the_wall_its_up_toward_the_axis() {
        for pose in tamsin().docked {
            let (_, rot, pos) = pose.to_scale_rotation_translation();
            let up = rot * DVec3::Y;
            let to_axis = -DVec3::new(pos.x, pos.y, 0.0).normalize();
            assert!(up.dot(to_axis) > 0.999, "up {up}, the axis that way {to_axis}");
            assert!((pos.x.hypot(pos.y) - 200.0).abs() < 1e-6, "200 m from the axis");
            assert!((rot * DVec3::Z).z > 0.999, "the bow toward the mouth");
        }
    }

    #[test]
    fn a_face_to_the_star_outshines_one_turned_away_and_tamsin_lights_its_side() {
        let o = look().outside;
        let light = outside_light(&o, Vec3::Z, -Vec3::Y);
        let (lit, dark, earth) = (light(Vec3::ZERO, Vec3::Z), light(Vec3::ZERO, -Vec3::Z), light(Vec3::ZERO, -Vec3::Y));
        assert!(lit[0] > 4.0 * dark[0], "the sunward dome against the far one");
        assert!(earth[2] > dark[2], "the side toward Tamsin takes its earthshine");
    }

    #[test]
    fn a_lamp_lights_what_faces_it_and_nothing_past_its_reach() {
        assert!(lamp(Vec3::ZERO, Vec3::Y, Vec3::new(0.0, 10.0, 0.0), 1.0, 100.0) > 0.8);
        assert_eq!(lamp(Vec3::ZERO, -Vec3::Y, Vec3::new(0.0, 10.0, 0.0), 1.0, 100.0), 0.0, "behind the face");
        assert_eq!(lamp(Vec3::ZERO, Vec3::Y, Vec3::new(0.0, 150.0, 0.0), 1.0, 100.0), 0.0, "out of reach");
    }

    #[test]
    fn the_shipped_textures_load_as_four_layers_of_the_layer_size() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let l = look();
        let px = l.layer_px as usize;
        assert_eq!(layers(&l, &root).expect("the textures load").len(), px * px * 4 * LAYERS as usize);
    }
}
