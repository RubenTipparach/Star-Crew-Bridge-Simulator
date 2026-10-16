//! Render tests (engine-stack design section 12, task 3.5): draw known deck vertices through the
//! real deck pipeline, headless (SDL's offscreen driver; Mesa's llvmpipe over EGL, OpenGL ES 3.0, in
//! a cloud session), read the output back and check what reached the shader.
//!
//! It proves the passes compose and the 28-byte vertex is read as `sc-core` packs it (the
//! engine-platform scenario "the deck vertex against its pipeline"). It proves nothing about the
//! Pi's speed. The whole frame is also compared with `tests/reference/deck_vertex.png`; a missing
//! reference is written for review, and SC_UPDATE_REFERENCES=1 rewrites it.
//!
//! The mover byte is checked by `sc-core`'s unit tests; the shader does not read it until movers
//! are drawn.
//!
//! Then the sky layer against solids (openspec/changes/sky-behind-solids, design 3): a ship in front
//! of a sky packed with bright stars, through a viewscreen feed's lens and through a bridge window,
//! the sky drawn first and drawn last. Each case is drawn once without the sky and once with it;
//! every pixel a solid covers must come out the same both times, so no star (nor any later pass of
//! the sky layer, such as star streaks, which joins `sky_layer` below) ever shows on a ship. The cases run twice: with
//! no nebula set (the sky's black octahedron) and with the shipped Bloom sky (openspec/changes/nebula-skies), the view
//! turning so its vector stars streak.
//!
//! Then the streak rule as the GPU draws it (nebula-skies design 5.2, task 4.3): one star at the view's centre, its
//! line measured in pixels at rest, turning slowly, turning at the cap and faster, across a cut, and inside the
//! planet's disc; and the sky mesh's colour at a vertex against the file's bytes (task 2.3).

use glam::{Mat4, Quat, Vec3};
use sc_client::platform::{self, App, Flow, Frame, WindowConfig};
use sc_core::vertex::{pack_deck_vertex, DeckVertexIn};
use sc_render::depth::{behind_everything, in_front_half, outside_lens, SOLID_REACH_M};
use sc_render::{DeckParams, DeckProgram, Indices, Magnify, Renderer, SkyParams, Target, TextureArray};
use std::path::PathBuf;
use std::process::ExitCode;

const SIZE: u32 = 256;
/// The view: 32 m across 256 px, so a metre is 8 px.
const HALF_M: f32 = 16.0;

fn quad(x0: f32, y0: f32, x1: f32, y1: f32, layer: u8, colors: [[u8; 4]; 3], uv: [[f32; 2]; 4]) -> (Vec<u8>, Vec<u16>) {
    let mut v = Vec::new();
    for (k, (x, y)) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].into_iter().enumerate() {
        let d = DeckVertexIn { position_m: [x, y, 0.0], mover: 9, layer, normal: [0.0, 0.0, 1.0], colors, uv: uv[k] };
        v.extend_from_slice(&pack_deck_vertex(&d).expect("test vertices fit the format"));
    }
    (v, vec![0, 1, 2, 0, 2, 3])
}

/// 256 layers of 2 x 2 px: texel (0, 0) of layer L is (L, 255 - L, 7), texel (1, 0) is (L / 2, 0, 255),
/// the bottom row grey.
fn layers() -> Vec<u8> {
    let mut out = Vec::new();
    for l in 0..=255u8 {
        out.extend_from_slice(&[l, 255 - l, 7, 255, l / 2, 0, 255, 255, 90, 90, 90, 255, 90, 90, 90, 255]);
    }
    out
}

struct Check {
    name: &'static str,
    /// Where to look, in metres.
    at: (f32, f32),
    want: [u8; 3],
}

const CHECKS: &[Check] = &[
    Check { name: "layer 200 (the sign bit of the packed lane) and its texel", at: (-12.0, 12.0), want: [200, 55, 7] },
    Check { name: "the red alert colour set alone", at: (-4.0, 12.0), want: [200, 0, 0] },
    Check { name: "the emergency colour set alone", at: (4.0, 12.0), want: [0, 0, 200] },
    Check { name: "half normal, half red alert", at: (12.0, 12.0), want: [132, 32, 32] },
    Check { name: "texture u in the first texel", at: (-14.0, 4.0), want: [5, 250, 7] },
    Check { name: "texture u in the second texel", at: (-2.0, 4.0), want: [2, 0, 255] },
    Check { name: "the normal lit by the dynamic light term", at: (4.0, 4.0), want: [128, 128, 128] },
    Check { name: "nothing drawn outside the quads", at: (12.0, -12.0), want: [0, 0, 0] },
    // A quarter of the way from the first texel's centre to the second's: nearest would give the first texel whole.
    Check { name: "a smooth array magnified bilinear", at: (-10.0, -12.0), want: [4, 188, 69] },
];

struct Test {
    target: Target,
    tex: TextureArray,
    /// The same layers, magnified bilinear (a painted atlas's sampling, tern-exterior design 2).
    smooth: TextureArray,
    /// Each quad: its mesh, program, colour set weights, dynamic light, and whether it samples `smooth`.
    draws: Vec<(sc_render::Mesh, DeckProgram, [f32; 3], f32, bool)>,
    failures: Vec<String>,
    /// A dark box, 2 m a side about its origin: the ship (and the bridge's wall) in the sky cases.
    hull: sc_render::Mesh,
    /// The sky case drawn next, and its frame without the sky, waiting for the frame with it.
    next_case: usize,
    without_sky: Option<Vec<u8>>,
    /// 0: the sky cases with no nebula; 1: again with Bloom set; 2: the streak cases; 3: the vertex colour.
    pass: usize,
    /// The clock the sky is drawn at, seconds: a sixtieth a draw.
    clock_s: f64,
    r: Renderer,
}

impl Test {
    fn new() -> Result<Self, String> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join("data/engine/render.json")).map_err(|e| e.to_string())?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(SIZE, SIZE, 1);
        let tex = r.make_texture_array(2, &layers());
        let smooth = r.make_texture_array(2, &layers()).magnified(Magnify::Linear);
        let grey = [[128, 128, 128, 255]; 3];
        let sets = [[32, 32, 32, 255], [100, 0, 0, 255], [0, 0, 100, 255]];
        let still = [[0.25, 0.25]; 4];
        let across = [[0.0, 0.25], [1.0, 0.25], [1.0, 0.25], [0.0, 0.25]];
        let mut draws = Vec::new();
        let mut add =
            |m: (Vec<u8>, Vec<u16>), p, w, f, s| draws.push((r.make_mesh(&m.0, Indices::U16(&m.1)), p, w, f, s));
        add(quad(-16.0, 8.0, -8.0, 16.0, 200, grey, still), DeckProgram::Textured, [1.0, 0.0, 0.0], 0.0, false);
        add(quad(-8.0, 8.0, 0.0, 16.0, 0, sets, still), DeckProgram::Flat, [0.0, 1.0, 0.0], 0.0, false);
        add(quad(0.0, 8.0, 8.0, 16.0, 0, sets, still), DeckProgram::Flat, [0.0, 0.0, 1.0], 0.0, false);
        add(quad(8.0, 8.0, 16.0, 16.0, 0, sets, still), DeckProgram::Flat, [0.5, 0.5, 0.0], 0.0, false);
        add(quad(-16.0, 0.0, 0.0, 8.0, 5, grey, across), DeckProgram::Textured, [1.0, 0.0, 0.0], 0.0, false);
        add(quad(0.0, 0.0, 8.0, 8.0, 0, sets, still), DeckProgram::Flat, [1.0, 0.0, 0.0], 0.25, false);
        add(quad(-16.0, -16.0, 0.0, -8.0, 5, grey, across), DeckProgram::Textured, [1.0, 0.0, 0.0], 0.0, true);
        let (v, i) = dark_box();
        let hull = r.make_mesh(&v, Indices::U16(&i));
        Ok(Self {
            target,
            tex,
            smooth,
            draws,
            failures: Vec::new(),
            hull,
            next_case: 0,
            without_sky: None,
            pass: 0,
            clock_s: 0.0,
            r,
        })
    }
}

impl App for Test {
    fn frame(&mut self, f: &Frame) -> Flow {
        if f.index > 1 {
            return self.sky_frame(f);
        }
        let mvp = glam::camera::rh::proj::opengl::orthographic(-HALF_M, HALF_M, -HALF_M, HALF_M, -1.0, 1.0);
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        for (mesh, prog, w, flash, smooth) in &self.draws {
            let p = DeckParams {
                mvp,
                state_weights: *w,
                flash_dir: Vec3::Z,
                flash: *flash,
                panel_first: u32::MAX,
                panel_glow: 0.0,
                clip_y: f32::MAX,
                relight: sc_render::NO_RELIGHT,
            };
            let tex = if *smooth { &self.smooth } else { &self.tex };
            self.r.draw_deck(&self.target, mesh, *prog, &p, Some(tex));
        }
        self.r.end_pass();
        self.r.present(&self.target, f.width, f.height, false);
        let px = sc_render::read_output(f.width, f.height);
        self.r.commit();
        assert_eq!((f.width, f.height), (SIZE, SIZE), "the offscreen window is the size asked for");
        for c in CHECKS {
            let x = ((c.at.0 + HALF_M) / (2.0 * HALF_M) * SIZE as f32) as usize;
            let y = ((HALF_M - c.at.1) / (2.0 * HALF_M) * SIZE as f32) as usize;
            let i = (y * SIZE as usize + x) * 4;
            let got = [px[i], px[i + 1], px[i + 2]];
            let ok = got.iter().zip(c.want).all(|(g, w)| (i32::from(*g) - i32::from(w)).abs() <= 2);
            println!("{} {}: got {got:?}, want {:?}", if ok { "ok  " } else { "FAIL" }, c.name, c.want);
            if !ok {
                self.failures.push(c.name.to_owned());
            }
        }
        let refp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference/deck_vertex.png");
        if let Err(e) = compare_reference(&refp, &px) {
            println!("FAIL reference image: {e}");
            self.failures.push("reference image".into());
        }
        Flow::Continue
    }
}

/// A sky case: what is drawn, in order, through which lenses.
#[derive(Copy, Clone)]
struct SkyCase {
    name: &'static str,
    /// Through the bridge's windows (a wall in the front half, the outside behind it), or a feed.
    bridge: bool,
    /// The sky layer drawn after the solids instead of before.
    sky_last: bool,
    /// The ship's distance and half size, metres.
    at_m: f32,
    half_m: [f32; 3],
}

const SKY_CASES: &[SkyCase] = &[
    SkyCase { name: "a feed, the sky first", bridge: false, sky_last: false, at_m: 3_000.0, half_m: [400.0; 3] },
    SkyCase { name: "a feed, the sky last", bridge: false, sky_last: true, at_m: 3_000.0, half_m: [400.0; 3] },
    SkyCase {
        name: "a feed, a solid at the farthest reach, the sky last",
        bridge: false,
        sky_last: true,
        at_m: SOLID_REACH_M - 1.0,
        half_m: [16_000.0, 16_000.0, 1.0],
    },
    SkyCase {
        name: "a bridge window, the sky first",
        bridge: true,
        sky_last: false,
        at_m: 3_000.0,
        half_m: [400.0; 3],
    },
    SkyCase { name: "a bridge window, the sky last", bridge: true, sky_last: true, at_m: 3_000.0, half_m: [400.0; 3] },
    SkyCase {
        name: "a bridge window, a solid at the farthest reach, the sky last",
        bridge: true,
        sky_last: true,
        at_m: SOLID_REACH_M - 1.0,
        half_m: [16_000.0, 16_000.0, 1.0],
    },
];

/// The view's vertical field of view: 40 degrees across the square target.
const SKY_FOV: f32 = 0.698_131_7;

impl Test {
    /// One sky case a frame, twice: without the sky, then with it, comparing the pixels the solids cover.
    fn sky_frame(&mut self, f: &Frame) -> Flow {
        if self.pass >= 2 {
            return self.streak_frame(f);
        }
        let Some(case) = SKY_CASES.get(self.next_case).copied() else {
            // The same cases with the shipped nebula, then the streaks.
            self.pass += 1;
            self.next_case = 0;
            if self.pass == 1 {
                let (sky, field) = shipped_sky("bloom");
                self.r.set_sky(&sky, &field);
            }
            return Flow::Continue;
        };
        let with_sky = self.without_sky.is_some();
        self.draw_sky_case(case, with_sky);
        self.r.present(&self.target, f.width, f.height, false);
        let px = sc_render::read_output(f.width, f.height);
        self.r.commit();
        let Some(bare) = self.without_sky.take() else {
            self.without_sky = Some(px);
            return Flow::Continue;
        };
        self.next_case += 1;
        let covered = |i: usize| bare[i * 4..i * 4 + 3].iter().any(|c| *c > 2);
        let n = bare.len() / 4;
        let solid = (0..n).filter(|i| covered(*i)).count();
        // On a solid, the frame with the sky is the frame without it, channel by channel.
        let starred =
            (0..n).filter(|i| covered(*i) && (0..3).any(|k| bare[i * 4 + k].abs_diff(px[i * 4 + k]) > 3)).count();
        // The sky is really there round the ship: many star-bright pixels where nothing solid stands (with the nebula,
        // its cloud: anything clearly above black).
        let bright = if self.pass == 0 { 128 } else { 8 };
        let stars = (0..n).filter(|i| !covered(*i) && px[i * 4..i * 4 + 3].iter().any(|c| *c > bright)).count();
        let ok = starred == 0 && solid > 500 && stars > 200;
        let sky = if self.pass == 0 { "stars" } else { "the nebula and its streaking stars" };
        println!(
            "{} {sky} never on a solid, {}: {starred} of {solid} solid pixels changed by the sky, {stars} sky pixels round them",
            if ok { "ok  " } else { "FAIL" },
            case.name
        );
        // SC_RENDER_SHOTS=DIR keeps every case's frame with the sky, for the captures the owner looks at.
        if let Some(dir) = std::env::var_os("SC_RENDER_SHOTS") {
            let path = PathBuf::from(dir).join(format!("sky-case-{}-{}.png", self.pass, self.next_case));
            let _ = sc_client::capture::write_png(&path, f.width, f.height, &px);
        }
        if !ok {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
            let path = dir.join(format!("sky-case-{}.png", self.next_case));
            let _ = sc_client::capture::write_png(&path, f.width, f.height, &px);
            println!("     the frame with the sky is in {}", path.display());
            self.failures.push(case.name.into());
        }
        Flow::Continue
    }

    /// Draw a sky case into the target: the ship (and, through a bridge window, the wall beside it) and, when `sky`,
    /// the sky layer, before or after them.
    fn draw_sky_case(&mut self, case: SkyCase, sky: bool) {
        let lens = outside_lens(SKY_FOV, 1.0);
        if sky && self.pass == 1 {
            // Last frame's view, a degree and a half round: this frame's stars streak.
            self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
            self.sky_layer_at(lens, Mat4::from_rotation_y(-0.026));
            self.r.end_pass();
        }
        // The outside's lens: a feed's own, or squeezed behind the bridge.
        let outside = if case.bridge { behind_everything() * lens } else { lens };
        // The bridge's own lens, as drill_bridge draws its rooms, in the front half of the buffer.
        let inside = in_front_half() * glam::camera::rh::proj::opengl::perspective(SKY_FOV, 1.0, 0.05, 400.0);
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        if sky && !case.sky_last {
            self.sky_layer(lens);
        }
        // The ship straddles the view's middle and the wall's edge, slightly turned so three faces show.
        let turn = Quat::from_euler(glam::EulerRot::YXZ, 0.5, 0.3, 0.0);
        let ship =
            Mat4::from_scale_rotation_translation(Vec3::from(case.half_m), turn, Vec3::new(0.0, 0.0, -case.at_m));
        self.draw_solid(outside * ship);
        if case.bridge {
            // The wall left of the window, 5 m off, covering the left third of the view and part of the ship.
            let wall = Mat4::from_scale_rotation_translation(
                Vec3::new(2.0, 4.0, 0.1),
                Quat::IDENTITY,
                Vec3::new(-2.6, 0.0, -5.0),
            );
            self.draw_solid(inside * wall);
        }
        if sky && case.sky_last {
            self.sky_layer(lens);
        }
        self.r.end_pass();
    }

    /// Every pass of the sky layer: today the sky's stars, sun and planet. A later pass (star streaks, a nebula) is drawn
    /// here too, so these cases hold it to the same rule.
    fn sky_layer(&mut self, lens: Mat4) {
        self.sky_layer_at(lens, Mat4::IDENTITY);
    }

    /// The sky layer seen through `lens` and the view's rotation `view`, a sixtieth of a second after the last.
    fn sky_layer_at(&mut self, lens: Mat4, view: Mat4) {
        self.clock_s += 1.0 / 60.0;
        let p = SkyParams {
            proj: lens,
            view,
            time_s: self.clock_s,
            // The sun and the planet behind the eye: the view ahead is stars alone, all of them bright.
            sun: [0.0, 0.0, 1.0, 0.9999],
            sun_colour: [0.0; 4],
            planet: [0.0, 0.0, 1.0, 0.01],
            ocean: [0.0; 4],
            land: [0.0; 4],
            cloud: [0.0; 4],
            atmosphere: [0.0; 4],
            stars: [1.0, 40.0, SKY_FOV / SIZE as f32, 0.0],
        };
        self.r.draw_sky(&self.target, &p);
    }

    fn draw_solid(&mut self, mvp: Mat4) {
        let p = DeckParams {
            mvp,
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: Vec3::Z,
            flash: 0.0,
            panel_first: u32::MAX,
            panel_glow: 0.0,
            clip_y: f32::MAX,
            relight: sc_render::NO_RELIGHT,
        };
        self.r.draw_deck(&self.target, &self.hull, DeckProgram::Flat, &p, None);
    }
}

/// A streak case: one star dead ahead, the view turning at `deg_s` about its vertical (or jumping, a cut), and the
/// least and most pixels its line may light.
struct StreakCase {
    name: &'static str,
    deg_s: f32,
    /// The planet's disc over the star.
    behind_planet: bool,
    lit: (usize, usize),
}

/// At 256 px across 40 degrees a pixel is 0.156 degrees: the 1.7 degree cap is 10.9 px, a 30 degree a second turn's
/// 0.75 degree tail 4.8 px (design 5.2).
const STREAK_CASES: &[StreakCase] = &[
    StreakCase { name: "a star at rest is a point", deg_s: 0.0, behind_planet: false, lit: (1, 2) },
    StreakCase { name: "a slow turn draws a short tail", deg_s: 30.0, behind_planet: false, lit: (4, 7) },
    StreakCase {
        name: "70 degrees a second draws the 1.7 degree cap",
        deg_s: 70.0,
        behind_planet: false,
        lit: (10, 13),
    },
    StreakCase { name: "a faster turn draws no longer a tail", deg_s: 400.0, behind_planet: false, lit: (10, 13) },
    StreakCase {
        name: "a cut (30 degrees in a frame) draws a point",
        deg_s: 1_800.0,
        behind_planet: false,
        lit: (1, 2),
    },
    StreakCase { name: "a star inside the planet's disc is not drawn", deg_s: 0.0, behind_planet: true, lit: (0, 0) },
];

/// A shipped sky and the starfield, decoded.
fn shipped_sky(id: &str) -> (sc_core::sky::NebulaSky, sc_core::sky::Starfield) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |p: &str| std::fs::read_to_string(root.join(p)).expect("a shipped file");
    let field = sc_core::sky::Starfield::load(sc_core::sky::STARFIELD_FILE, &read(sc_core::sky::STARFIELD_FILE))
        .expect("the shipped starfield");
    let path = sc_core::sky::sky_file(id);
    (sc_core::sky::NebulaSky::load(&path, &read(&path), &field).expect("the shipped sky"), field)
}

impl Test {
    /// The streak cases (pass 2), then the vertex colour (pass 3), one a frame.
    fn streak_frame(&mut self, f: &Frame) -> Flow {
        if self.pass == 2 && self.next_case == 0 {
            // One plain white star of the faint class (a line alone, no head) dead ahead, in a black sky.
            let (mut sky, field) = shipped_sky("bloom");
            let (ov, oi) = octahedron_sky();
            sky.directions = ov;
            sky.rgba = vec![[0, 0, 0, 0]; 6];
            sky.indices = oi;
            sky.dust_stars = [0.0, 0.0];
            let white =
                field.colours.iter().position(|c| c.srgb == [1.0, 1.0, 1.0]).expect("a white star colour") as u8;
            sky.stars = vec![sc_core::sky::SkyStar { dir: [0.0, 0.0, -1.0], brightness: 1.0, class: 0, colour: white }];
            self.r.set_sky(&sky, &field);
        }
        if self.pass == 3 {
            return self.vertex_colour_frame(f);
        }
        let Some(case) = STREAK_CASES.get(self.next_case) else {
            self.pass = 3;
            self.next_case = 0;
            let (sky, field) = shipped_sky("bloom");
            self.r.set_sky(&sky, &field);
            return Flow::Continue;
        };
        let lens = outside_lens(SKY_FOV, 1.0);
        let planet = if case.behind_planet { [0.0, 0.0, -1.0, 0.1] } else { [0.0, 0.0, 1.0, 0.01] };
        let turn = case.deg_s.to_radians() / 60.0;
        for (view, t) in [(Mat4::from_rotation_y(-turn), 0.0), (Mat4::IDENTITY, 1.0 / 60.0)] {
            self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
            let p = SkyParams {
                proj: lens,
                view,
                time_s: 100.0 * (self.next_case as f64 + 1.0) + t,
                sun: [0.0, 0.0, 1.0, 0.9999],
                sun_colour: [0.0; 4],
                planet,
                ocean: [0.0; 4],
                land: [0.0; 4],
                cloud: [0.0; 4],
                atmosphere: [0.0; 4],
                stars: [0.0, 0.0, SKY_FOV / SIZE as f32, 0.0],
            };
            self.r.draw_sky(&self.target, &p);
            self.r.end_pass();
        }
        self.r.present(&self.target, f.width, f.height, false);
        let px = sc_render::read_output(f.width, f.height);
        self.r.commit();
        let lit = px.chunks_exact(4).filter(|c| c[..3].iter().any(|v| *v > 16)).count();
        let ok = (case.lit.0..=case.lit.1).contains(&lit);
        println!(
            "{} {}: {lit} pixels lit, want {}-{}",
            if ok { "ok  " } else { "FAIL" },
            case.name,
            case.lit.0,
            case.lit.1
        );
        if let Some(dir) = std::env::var_os("SC_RENDER_SHOTS") {
            let path = PathBuf::from(dir).join(format!("streak-case-{}.png", self.next_case));
            let _ = sc_client::capture::write_png(&path, f.width, f.height, &px);
        }
        if !ok {
            self.failures.push(case.name.into());
        }
        self.next_case += 1;
        Flow::Continue
    }

    /// The sky mesh's colour at a vertex is the file's (task 2.3): a narrow view straight at the brightest vertex with no
    /// vector star within a degree, the faint stars off, its centre pixel against the vertex's bytes.
    fn vertex_colour_frame(&mut self, f: &Frame) -> Flow {
        let (mut sky, field) = shipped_sky("bloom");
        sky.dust_stars = [0.0, 0.0];
        self.r.set_sky(&sky, &field);
        let clear = |d: glam::Vec3| sky.stars.iter().all(|s| glam::Vec3::from(s.dir).dot(d) < 1f32.to_radians().cos());
        let (k, d) = sky
            .directions
            .iter()
            .enumerate()
            .map(|(k, d)| (k, glam::Vec3::from(*d)))
            .filter(|(_, d)| d.y.abs() < 0.9 && clear(*d))
            .max_by_key(|(k, _)| sky.rgba[*k][..3].iter().map(|c| u32::from(*c)).sum::<u32>())
            .expect("a vertex with no star near");
        let fov = 0.5f32.to_radians();
        let lens = outside_lens(fov, 1.0);
        let view = glam::camera::rh::view::look_to_mat4(
            glam::Vec3::ZERO,
            d,
            if d.y.abs() < 0.9 { glam::Vec3::Y } else { glam::Vec3::X },
        );
        self.clock_s += 1.0;
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        let p = SkyParams {
            proj: lens,
            view,
            time_s: self.clock_s,
            sun: [-d.x, -d.y, -d.z, 0.9999],
            sun_colour: [0.0; 4],
            planet: [-d.x, -d.y, -d.z, 0.01],
            ocean: [0.0; 4],
            land: [0.0; 4],
            cloud: [0.0; 4],
            atmosphere: [0.0; 4],
            stars: [0.0, 0.0, fov / SIZE as f32, 0.0],
        };
        self.r.draw_sky(&self.target, &p);
        self.r.end_pass();
        self.r.present(&self.target, f.width, f.height, false);
        let px = sc_render::read_output(f.width, f.height);
        self.r.commit();
        let c = SIZE as usize / 2;
        let i = (c * SIZE as usize + c) * 4;
        let got = [px[i], px[i + 1], px[i + 2]];
        let want = [sky.rgba[k][0], sky.rgba[k][1], sky.rgba[k][2]];
        let ok = got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 2);
        println!(
            "{} the sky's colour at vertex {k} is the file's: got {got:?}, want {want:?}",
            if ok { "ok  " } else { "FAIL" }
        );
        if !ok {
            self.failures.push("the sky's colour at a vertex".into());
        }
        if self.failures.is_empty() {
            Flow::Done
        } else {
            Flow::Fail
        }
    }
}

/// The octahedron of the sky with no nebula, as directions and indices.
fn octahedron_sky() -> (Vec<[f32; 3]>, Vec<u16>) {
    let v =
        vec![[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
    let mut idx = Vec::new();
    for x in [0u16, 1] {
        for y in [2u16, 3] {
            for z in [4u16, 5] {
                idx.extend_from_slice(&[x, y, z]);
            }
        }
    }
    (v, idx)
}

/// A box 2 m a side about the origin, dark grey, each face wound outward (the deck pipeline culls back faces).
fn dark_box() -> (Vec<u8>, Vec<u16>) {
    let (mut v, mut idx) = (Vec::new(), Vec::new());
    let grey = [[40, 42, 48, 255]; 3];
    for axis in 0..3 {
        for s in [-1.0f32, 1.0] {
            let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
            let corner = |u: f32, w: f32| {
                let mut p = [0.0f32; 3];
                p[axis] = s;
                p[a] = u;
                p[b] = w;
                p
            };
            let mut q = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
            if s < 0.0 {
                q.reverse();
            }
            let mut normal = [0.0f32; 3];
            normal[axis] = s;
            let base = (v.len() / sc_core::vertex::DECK_VERTEX_BYTES) as u16;
            for p in q {
                let d = DeckVertexIn { position_m: p, mover: 0, layer: 0, normal, colors: grey, uv: [0.0, 0.0] };
                v.extend_from_slice(&pack_deck_vertex(&d).expect("the box fits the format"));
            }
            idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    (v, idx)
}

/// The frame against the committed reference: every channel of every pixel within 3 of it.
fn compare_reference(path: &std::path::Path, px: &[u8]) -> Result<(), String> {
    let update = std::env::var("SC_UPDATE_REFERENCES").is_ok_and(|v| v == "1");
    if update || !path.exists() {
        sc_client::capture::write_png(path, SIZE, SIZE, px).map_err(|e| e.to_string())?;
        println!("note reference image written to {}; look at it and commit it", path.display());
        return Ok(());
    }
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?));
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("reference too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if (info.width, info.height) != (SIZE, SIZE) || info.color_type != png::ColorType::Rgba {
        return Err(format!("{} is not a {SIZE} px RGBA image", path.display()));
    }
    let bad = buf.iter().zip(px).filter(|(a, b)| (i32::from(**a) - i32::from(**b)).abs() > 3).count();
    if bad > 0 {
        let actual = path.with_extension("actual.png");
        let _ = sc_client::capture::write_png(&actual, SIZE, SIZE, px);
        return Err(format!("{bad} channel values differ by more than 3; this frame is in {}", actual.display()));
    }
    Ok(())
}

fn main() -> ExitCode {
    let cfg = WindowConfig {
        title: "render test".into(),
        width: SIZE,
        height: SIZE,
        fullscreen: false,
        headless: true,
        vsync: false,
    };
    let code = platform::run(cfg, |_| Test::new().map(|t| Box::new(t) as Box<dyn App>));
    println!("render tests: {}", if code == ExitCode::SUCCESS { "ok" } else { "FAILED" });
    code
}
