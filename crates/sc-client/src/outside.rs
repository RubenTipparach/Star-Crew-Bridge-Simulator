//! The light outside the ship (openspec/changes/patrol-mission, design 11.3): one star lights the sky, the Tern's own
//! hull and every ship, craft and station drawn outside, so the windows never show two suns.
//!
//! It lives in the client because it is drawing: the walk, the drill's bridge and the viewscreens' feed all light the
//! outside through these functions (CLAUDE.md 6.1: one implementation of a rule). The sun's direction is
//! `data/space/exterior.json`'s, a direction in the system's frame; the hull's bake is `data/ships/exteriors.json`'s,
//! the light the exterior build baked into the model's vertex colours.

use glam::{Mat4, Quat, Vec3};
use sc_core::exterior::{normalized, Beacons, ExteriorData};
use sc_render::{DeckProgram, Indices, Mesh, Renderer, Target};
use serde::Deserialize;

use crate::ships3d::{beacon_lamp, station, Beacon, MeshBuilder, StationKind};

/// The light baked into the hull's vertex colours by the exterior build (`tools/blender/build_ship_exteriors.py`):
/// `ambient_fraction + (1 - ambient_fraction) max(0, n . sun_dir_ship)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullBake {
    /// The way to the sun the bake used, ship frame, unit length.
    pub sun_ship: Vec3,
    /// The share of the light that reaches a face turned from the sun, 0-1.
    pub ambient: f32,
}

/// The two fields of `data/ships/exteriors.json` the hull's light needs. The file is the exterior build's whole
/// configuration (liveries, atlases, budgets), read here only for the bake; unknown keys are the build's and are
/// ignored here on purpose.
#[derive(Deserialize)]
struct BakeFields {
    sun_dir_ship: [f64; 3],
    ambient_fraction: f64,
}

impl HullBake {
    /// The bake from the text of `data/ships/exteriors.json` (`path` names it in an error): its sun must be a finite,
    /// non-zero direction and its ambient share above 0 and at most 1, or the relight would divide by nothing.
    pub fn parse(path: &str, text: &str) -> Result<Self, String> {
        let f: BakeFields = serde_json::from_str(text).map_err(|e| format!("{path}: {e}"))?;
        let d = f.sun_dir_ship;
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if !len.is_finite() || len < 1e-6 {
            return Err(format!("{path}: sun_dir_ship must be a finite, non-zero direction"));
        }
        if !(f.ambient_fraction > 0.0 && f.ambient_fraction <= 1.0) {
            return Err(format!("{path}: ambient_fraction must be above 0 and at most 1"));
        }
        Ok(Self { sun_ship: Vec3::from(normalized(d)), ambient: f.ambient_fraction as f32 })
    }

    /// The bake from `data/ships/exteriors.json` in the working directory.
    pub fn load() -> Result<Self, String> {
        let path = "data/ships/exteriors.json";
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Self::parse(path, &text)
    }

    /// The deck program's `relight` for this bake (`sc_render::DeckParams::relight`).
    pub fn relight(&self) -> [f32; 4] {
        [self.sun_ship.x, self.sun_ship.y, self.sun_ship.z, self.ambient]
    }
}

/// Load the sky `system` names (`data/space/skies/<sky>.json`, with `data/space/starfield.json`) and set it as the
/// sky every view draws (openspec/changes/nebula-skies, task 3). A missing or bad sky stops startup with its path and
/// field (CLAUDE.md 6.5).
pub fn set_system_sky(r: &mut Renderer, system: &sc_core::system::SystemFile) -> Result<(), String> {
    use sc_core::sky::{sky_file, NebulaSky, Starfield, STARFIELD_FILE};
    if r.sky().0 == system.sky {
        return Ok(());
    }
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let starfield = Starfield::load(STARFIELD_FILE, &read(STARFIELD_FILE)?).map_err(|e| e.to_string())?;
    let path = sky_file(&system.sky);
    let sky = NebulaSky::load(&path, &read(&path)?, &starfield).map_err(|e| e.to_string())?;
    r.set_sky(&sky, &starfield);
    let [tris, lines, heads] = r.sky_counts();
    println!(
        "sc-client: the sky of {} is {}: {} vertices, {tris} triangles, {lines} stars, {heads} heads",
        system.name,
        system.sky,
        sky.directions.len()
    );
    Ok(())
}

/// Set the home system's sky (`sc_core::system::HOME_FILE`): the walk's, the drill's, and a patrol's until its own
/// system's is set.
pub fn set_home_sky(r: &mut Renderer) -> Result<(), String> {
    let path = sc_core::system::HOME_FILE;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let system: sc_core::system::SystemFile = sc_core::data::parse(path, &text).map_err(|e| e.to_string())?;
    set_system_sky(r, &system)
}

/// The way to the sun in the ship's frame, unit length, for a ship whose attitude (ship frame to the system's) is
/// `attitude`: the exterior's sun is a system direction (in the walk's dock, the system's axes are the ship's).
pub fn sun_in_ship(ext: &ExteriorData, attitude: Quat) -> Vec3 {
    attitude.inverse() * Vec3::from(normalized(ext.sun.dir))
}

/// The ambient cube that lights a ship, a craft or a station outside, from the sun's direction `sun` in the frame the
/// cube's axes are in (`sc_render::Renderer::draw_deck_probe`): a face toward the sun takes up to 1.5 times its
/// albedo, a face away a little of the shade side's fill, so the shape reads on its dark side; the flat ambient under
/// both is tinted by the sky's colour from that face's way (`tint`, [`sky_tint`]; nebula-skies design 6).
pub fn sun_cube(sun: Vec3, tint: &[[f32; 3]; 6]) -> [[f32; 3]; 6] {
    let axes = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    let mut out = [[0.0; 3]; 6];
    for (k, a) in axes.into_iter().enumerate() {
        let s = a.dot(sun).max(0.0);
        let fill = (-a.dot(sun)).max(0.0) * 0.12;
        let t = tint[k];
        out[k] = [0.2 * t[0] + 1.3 * s + fill * 0.6, 0.2 * t[1] + 1.25 * s + fill * 0.7, 0.22 * t[2] + 1.15 * s + fill];
    }
    out
}

/// A linear value as sRGB (the display's), 0-1.
fn to_srgb(x: f32) -> f32 {
    let x = x.max(0.0);
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// The sky's tint on an ambient term (openspec/changes/nebula-skies design 6): for a surface facing +X, -X, +Y, -Y, +Z
/// and -Z of a frame turned by `turn` (that frame into the sky's: a ship's attitude), what multiplies its ambient
/// share, as display values: the sky's ambient cube there over its strength, so the sky gives the ambient its colour
/// and the way it leans and the bake keeps how strong it is. All 1 when no sky is set.
pub fn sky_tint(r: &Renderer, turn: Quat) -> [[f32; 3]; 6] {
    let Some((cube, strength)) = r.sky().1 else { return sc_render::NO_TINT };
    let norm = to_srgb(strength).max(1e-4);
    let axes = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    axes.map(|a| {
        // The face's way in the sky's frame, and the sky's light there (Half-Life 2's ambient cube).
        let d = turn * a;
        let d2 = d * d;
        let pick = |v: f32, pos: usize| if v >= 0.0 { cube[pos] } else { cube[pos + 1] };
        let (x, y, z) = (pick(d.x, 0), pick(d.y, 2), pick(d.z, 4));
        [0, 1, 2].map(|c| to_srgb(d2.x * x[c] + d2.y * y[c] + d2.z * z[c]) / norm)
    })
}

/// The exterior seen from `eye` (system frame) in a patrol's system (patrol-mission design 11.2): the star's direction,
/// size and light and the near planet's direction, size and look from the system file; the rest as `ext` has it. Every
/// light outside reads the result (the sky, the hull, the ships), so they agree.
pub fn exterior_at(ext: &ExteriorData, sys: &sc_core::system::SystemFile, eye: glam::DVec3) -> ExteriorData {
    let v = sys.sky_from(eye);
    let mut e = ext.clone();
    let st = &sys.star;
    e.sun.dir = v.sun_dir.to_array();
    e.sun.disc_deg = (2.0 * v.sun_sin_r.asin().to_degrees()).max(0.05);
    e.sun.colour_srgb = st.colour_srgb.map(|c| (c * v.light).min(1.0));
    e.sun.glow = st.glow;
    if let Some((i, dir, sin_r)) = v.planet {
        let b = &sys.bodies[i];
        let l = &b.look;
        e.planet.dir = dir.to_array();
        e.planet.radius_deg = sin_r.asin().to_degrees();
        e.planet.ocean_srgb = l.ocean_srgb;
        e.planet.land_srgb = l.land_srgb;
        e.planet.cloud_srgb = l.cloud_srgb;
        e.planet.atmosphere_srgb = l.atmosphere_srgb;
        e.planet.atmosphere_frac = l.atmosphere_frac;
        e.planet.land_frac = l.land_frac;
        e.planet.cloud_frac = l.cloud_frac;
        e.planet.night = l.night;
    }
    e
}

/// A station's beacons on the GPU: each one's lamp (radius 1), where it is in the station's frame and its radius up
/// close, metres. The relay and every spacedock blink theirs by this one rule (CLAUDE.md 6.1).
pub struct BeaconLamps(Vec<(Mesh, Vec3, f32)>);

impl BeaconLamps {
    /// The lamps of `beacons`, on the GPU.
    pub fn new(r: &Renderer, beacons: &[Beacon]) -> Self {
        let mesh = |m: &MeshBuilder| r.make_mesh(&m.vertices, Indices::U16(&m.indices));
        Self(beacons.iter().map(|b| (mesh(&beacon_lamp(b.rgb)), Vec3::from(b.at_m), b.radius_m)).collect())
    }

    /// Draw them into the current pass of `t` through `vp` (a view at the eye), the station's frame at `rel` from the
    /// eye and turned by `rot`, only while `b.lit` at `time_s`, each at least `b.min_radius_deg` across from the eye.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, r: &mut Renderer, t: &Target, vp: Mat4, rel: Vec3, rot: Quat, b: &Beacons, time_s: f64) {
        if !b.lit(time_s) {
            return;
        }
        for (lamp, at, radius) in &self.0 {
            let p = rel + rot * *at;
            let size = beacon_radius_m(*radius, p.length(), b.min_radius_deg);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size));
            r.draw_deck(t, lamp, DeckProgram::Flat, &crate::drill_bridge::flat_params(vp * m), None);
        }
    }
}

/// A placeholder station ready to draw (patrol-mission design 3): its structure lit by the sun, its windows and strips
/// glowing, its beacons blinking.
pub struct StationView {
    scale: f32,
    body: Mesh,
    lights: Mesh,
    beacons: BeaconLamps,
    /// Its triangles.
    pub triangles: usize,
}

impl StationView {
    /// The placeholder of `kind`, on the GPU.
    pub fn new(r: &Renderer, kind: StationKind) -> Self {
        let s = station(kind);
        let mesh = |m: &MeshBuilder| r.make_mesh(&m.vertices, Indices::U16(&m.indices));
        let beacons = BeaconLamps::new(r, &s.beacons);
        Self { scale: s.scale, body: mesh(&s.body), lights: mesh(&s.lights), beacons, triangles: s.triangles() }
    }

    /// Draw it into the current pass of `t` through `vp` (a view at the eye), at `rel` from the eye and turned by
    /// `rot` (the frame `vp` is in), lit by the sun at `sun` in that frame. Its beacons show only while `b.lit` at
    /// `time_s`, each at least `b.min_radius_deg` in angular radius from the eye.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        r: &mut Renderer,
        t: &Target,
        vp: Mat4,
        rel: Vec3,
        rot: Quat,
        sun: Vec3,
        b: &Beacons,
        time_s: f64,
    ) {
        let model = Mat4::from_rotation_translation(rot, rel) * Mat4::from_scale(Vec3::splat(self.scale));
        let cube = sun_cube(sun, &sky_tint(r, Quat::IDENTITY));
        r.draw_deck_probe(t, &self.body, vp * model, Mat4::from_quat(rot), &cube, [1.0, 0.0, 0.0]);
        r.draw_deck(t, &self.lights, DeckProgram::Flat, &crate::drill_bridge::flat_params(vp * model), None);
        self.beacons.draw(r, t, vp, rel, rot, b, time_s);
    }
}

/// A beacon's drawn radius, metres: its own `radius_m`, or more when it is `distance_m` away, so it is never under
/// `min_radius_deg` across from the eye (a lamp seen far off is a point, not nothing).
pub fn beacon_radius_m(radius_m: f32, distance_m: f32, min_radius_deg: f64) -> f32 {
    radius_m.max(distance_m * (min_radius_deg as f32).to_radians().tan())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXTERIORS: &str = include_str!("../../../data/ships/exteriors.json");
    const EXTERIOR: &str = include_str!("../../../data/space/exterior.json");

    fn ext() -> ExteriorData {
        sc_core::data::parse("data/space/exterior.json", EXTERIOR).expect("the shipped exterior loads")
    }

    #[test]
    fn the_shipped_hull_bake_loads() {
        let b = HullBake::parse("data/ships/exteriors.json", EXTERIORS).expect("the shipped bake loads");
        assert!((b.sun_ship.length() - 1.0).abs() < 1e-5 && b.ambient > 0.0 && b.ambient <= 1.0);
    }

    #[test]
    fn a_bake_with_no_ambient_is_refused() {
        // With no ambient, a face turned from the baked sun is black and the relight would divide by zero.
        let bad = r#"{ "sun_dir_ship": [0, 1, 0], "ambient_fraction": 0.0 }"#;
        assert!(HullBake::parse("x", bad).unwrap_err().contains("ambient_fraction"));
    }

    #[test]
    fn the_hull_bake_is_not_the_skys_sun() {
        // The finding behind the relight (patrol-mission design 10.1): the hull was baked 25 degrees off the sky's sun.
        let b = HullBake::parse("data/ships/exteriors.json", EXTERIORS).unwrap();
        let deg = b.sun_ship.angle_between(sun_in_ship(&ext(), Quat::IDENTITY)).to_degrees();
        assert!(deg > 5.0, "the hull's bake and the sky's sun are {deg} degrees apart");
    }

    #[test]
    fn in_the_dock_the_sun_is_the_exteriors_direction() {
        let s = sun_in_ship(&ext(), Quat::IDENTITY);
        assert!(s.distance(Vec3::from(normalized(ext().sun.dir))) < 1e-6);
    }

    #[test]
    fn rolling_the_ship_turns_the_sun_the_other_way_in_its_frame() {
        // A roll of 90 degrees to starboard (about +Z) puts a sun that was overhead on the port side, as the crew see it.
        let mut e = ext();
        e.sun.dir = [0.0, 1.0, 0.0];
        let roll = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let s = sun_in_ship(&e, roll);
        assert!(s.distance(Vec3::X) < 1e-5, "the sun is at {s} in the ship's frame");
    }

    #[test]
    fn at_the_pell_beacon_the_sky_shows_orrin_and_a_smaller_dimmer_star() {
        let sys: sc_core::system::SystemFile = sc_core::data::parse(
            "data/space/systems/shoals.json",
            include_str!("../../../data/space/systems/shoals.json"),
        )
        .unwrap();
        let at = |id: &str| exterior_at(&ext(), &sys, sys.place(id).unwrap().arrival);
        let (home, pell) = (at("tamsin_station"), at("pell_beacon"));
        assert!((pell.planet.radius_deg - 16.9).abs() < 0.3, "Orrin, 16.9 degrees in radius");
        assert_eq!(pell.planet.ocean_srgb, sys.bodies.iter().find(|b| b.id == "orrin").unwrap().look.ocean_srgb);
        assert!(pell.sun.disc_deg < home.sun.disc_deg && pell.sun.colour_srgb[0] < home.sun.colour_srgb[0] + 1e-9);
    }

    #[test]
    fn a_far_beacon_keeps_its_minimum_size_and_a_near_one_its_own() {
        let near = beacon_radius_m(3.0, 100.0, 0.15);
        let far = beacon_radius_m(3.0, 18_600.0, 0.15);
        assert_eq!(near, 3.0, "up close a beacon is its own size");
        let deg = (far / 18_600.0).atan().to_degrees();
        assert!((deg - 0.15).abs() < 1e-3, "18.6 km off it still spans {deg} degrees");
    }

    #[test]
    fn the_sunward_face_is_brightest_and_the_far_face_gets_the_fill() {
        let c = sun_cube(Vec3::Y, &sc_render::NO_TINT);
        assert!(c[2][0] > c[0][0] && c[0][0] > 0.0, "up, toward the sun, outshines a side face");
        assert!(c[3][2] > c[0][2], "the face away from the sun takes the shade side's fill over a side face's");
    }
}
