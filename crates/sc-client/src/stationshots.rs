//! `sc-client --headless --station-shots DIR`: every station of the Shoals drawn by the game's own `StationsView`, from
//! outside and from inside its bay, with the Tern-class ships at their berths lit by the bay (openspec/changes/spacedock
//! design 8b, tasks 2.9). A patrol only comes near Tamsin Station, so this is how the other four are seen in the
//! engine. It is a capture instrument: it draws with the stations' own code and changes nothing; the cameras are the
//! spacedock mockup's (`docs/mockups/spacedock.html` `CAMS`, [eye, target] in the station's frame), so each shot sits
//! beside the mockup's of the same view. No sky is drawn behind them.

use glam::{DVec3, Mat4, Quat, Vec3};
use sc_client::platform::{App, Flow, Frame};
use sc_render::{Renderer, Target};
use std::path::PathBuf;

use crate::hull::HullView;
use crate::stations::{StationLook, StationsView};

/// The views: the station's version, the shot's name, its eye and what it looks at (the station's frame, metres).
const VIEWS: [(char, &str, [f64; 3], [f64; 3]); 9] = [
    ('A', "outside", [2300.0, 700.0, 4400.0], [0.0, -500.0, 0.0]),
    ('A', "bay", [190.0, 40.0, 730.0], [-10.0, -45.0, 440.0]),
    ('B', "outside", [1500.0, 900.0, 3000.0], [0.0, 0.0, 0.0]),
    ('B', "bay", [-120.0, -80.0, -245.0], [10.0, -150.0, -40.0]),
    ('C', "outside", [1500.0, 700.0, 2700.0], [0.0, -50.0, -100.0]),
    ('C', "bay", [-185.0, 30.0, 150.0], [-60.0, -55.0, 440.0]),
    ('D', "outside", [1700.0, 650.0, 3300.0], [0.0, 0.0, 0.0]),
    ('D', "bay", [-205.0, -50.0, 70.0], [-60.0, -105.0, 340.0]),
    ('E', "bay", [175.0, -45.0, 245.0], [-60.0, -150.0, 60.0]),
];

/// The shots' size, pixels.
const SIZE: (u32, u32) = (1440, 900);

/// The instrument: the stations, the Tern's model for the docked ships, and the next view to take.
pub struct StationShotsApp {
    r: Renderer,
    target: Target,
    stations: StationsView,
    hull: Option<HullView>,
    beacons: sc_core::exterior::Beacons,
    /// Each station's version, in the system file's order.
    versions: Vec<char>,
    out: PathBuf,
    next: usize,
}

impl StationShotsApp {
    /// Load the system, the stations (and `compiled/stations.bays` when it is there) and the Tern's hull.
    pub fn new(out: PathBuf) -> Result<Self, String> {
        let read = |rel: &str| std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"));
        let cfg = sc_core::data::parse("data/engine/render.json", &read("data/engine/render.json")?)
            .map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(SIZE.0, SIZE.1, 4);
        let rel = "data/space/systems/shoals.json";
        let sys: sc_core::system::SystemFile = sc_core::data::parse(rel, &read(rel)?).map_err(|e| e.to_string())?;
        let rel = "data/space/exterior.json";
        let ext: sc_core::exterior::ExteriorData = sc_core::data::parse(rel, &read(rel)?).map_err(|e| e.to_string())?;
        let stations = StationsView::new(&r, &sys, StationLook::load()?, std::path::Path::new("."))?;
        let deck = std::fs::read("compiled/tern.deck").map_err(|e| format!("compiled/tern.deck: {e}"))?;
        let d = sc_core::deck::read(&deck).map_err(|e| format!("compiled/tern.deck: {e}"))?;
        let hull = HullView::load(&r, &d)?;
        let versions = sys.stations.iter().map(|s| s.version.chars().next().unwrap_or('E')).collect();
        std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        Ok(Self { r, target, stations, hull, beacons: ext.beacons, versions, out, next: 0 })
    }
}

impl App for StationShotsApp {
    fn frame(&mut self, f: &Frame) -> Flow {
        let Some(&(version, name, eye, look)) = VIEWS.get(self.next) else { return Flow::Done };
        self.next += 1;
        let Some(index) = self.versions.iter().position(|&v| v == version) else { return Flow::Continue };
        let (eye, look) = (DVec3::from_array(eye), DVec3::from_array(look));
        let dir = (look - eye).normalize().as_vec3();
        let proj = sc_render::depth::outside_lens(50f32.to_radians(), SIZE.0 as f32 / SIZE.1 as f32);
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, dir, Vec3::Y);
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        let rel = (-eye).as_vec3();
        let vp: Mat4 = proj * view;
        self.stations.draw(
            &mut self.r,
            &self.target,
            vp,
            index,
            rel,
            Quat::IDENTITY,
            &self.beacons,
            0.0,
            self.hull.as_ref(),
        );
        self.r.end_pass();
        self.r.present(&self.target, f.width, f.height, false);
        let px = sc_render::read_output(f.width, f.height);
        let file = self.out.join(format!("{version}-{name}.png"));
        match sc_client::capture::write_png(&file, f.width, f.height, &px) {
            Ok(()) => println!("sc-client: wrote {}", file.display()),
            Err(e) => eprintln!("sc-client: {e}"),
        }
        self.r.commit();
        Flow::Continue
    }
}
