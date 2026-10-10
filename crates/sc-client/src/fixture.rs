//! The engine's side of the console parity check (openspec/changes/console-parity, design 4).
//!
//! `sc-client --headless --console-fixture <state.json> --shots <dir>` draws one console from a state the console
//! mockup saved (`tools/consoles/parity.mjs` writes `window.consoleState()` beside each of its named shots), at
//! 1280 x 720 with one layout point a pixel, and writes `<name>-engine.png`. `tools/consoles/compare.py` lays it
//! beside the mockup's picture of the same moment. It is a measurement instrument: it draws with the drill's own
//! console code and changes nothing. The viewscreen stays black here: its 3D feed is the drill's, not the mockup's
//! sketch, and the comparison leaves that rectangle out.

use crate::console::{self, ConsoleView};
use crate::ui::Ui;
use sc_client::platform::{App, Flow, Frame};
use sc_render::{Renderer, Target};
use std::path::PathBuf;

/// The fixture app: one state, drawn until the font atlas settles, then captured.
pub struct FixtureApp {
    r: Renderer,
    target: Target,
    ui: Ui,
    view: ConsoleView,
    out: PathBuf,
    frames: u32,
}

impl FixtureApp {
    /// Read the state and set the renderer up.
    pub fn new(state: PathBuf, shots: PathBuf) -> Result<Self, String> {
        let text = std::fs::read_to_string(&state).map_err(|e| format!("{}: {e}", state.display()))?;
        let view: ConsoleView = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", state.display()))?;
        let cfg_text =
            std::fs::read_to_string("data/engine/render.json").map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &cfg_text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(16, 16, 1);
        let name = state.file_stem().and_then(|s| s.to_str()).unwrap_or("console").to_owned();
        Ok(Self { r, target, ui: Ui::new(), view, out: shots.join(format!("{name}-engine.png")), frames: 0 })
    }
}

impl App for FixtureApp {
    fn frame(&mut self, f: &Frame) -> Flow {
        self.frames += 1;
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        self.r.end_pass();
        let view = &self.view;
        let frame = self.ui.run(&mut self.r, f, 0.0, |ctx| {
            let _ = console::paint(ctx, view, false, None);
        });
        let meshes = frame.meshes();
        if self.frames == 3 {
            // The console's cost against the Pi 5 budget (CLAUDE.md 2): UI draw calls (one a clipped mesh), vertices
            // and indices a frame.
            let (v, i): (usize, usize) =
                meshes.iter().fold((0, 0), |a, m| (a.0 + m.vertices.len(), a.1 + m.indices.len()));
            println!(
                "sc-client: console {}: {} UI draw calls, {v} vertices, {i} indices",
                self.view.station,
                meshes.len()
            );
        }
        self.r.present_with_ui(&self.target, f.width, f.height, true, &meshes, frame.points);
        // The first frames fill the font atlas; the third is drawn with every glyph in it.
        if self.frames == 3 {
            let px = sc_render::read_output(f.width, f.height);
            match sc_client::capture::write_png(&self.out, f.width, f.height, &px) {
                Ok(()) => println!("sc-client: wrote {}", self.out.display()),
                Err(e) => eprintln!("sc-client: {e}"),
            }
        }
        self.r.commit();
        if self.frames >= 3 {
            Flow::Done
        } else {
            Flow::Continue
        }
    }
}
