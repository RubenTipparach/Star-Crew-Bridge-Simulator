//! The repair games in the client (openspec/changes/repair-minigames design 8): `sc-client --repairs [id]` opens the
//! menu of every system's repair, as `docs/mockups/repairs.html` does, with no ship; a game opens on the 1280 x 720
//! frame, letterboxed. Esc goes back to the menu, and from the menu quits.
//!
//! Docking at the machine in the 3D ship is `repairs-on-deck` 3; until it is built this is where the games are played.
//! `--shots DIR` (with `--headless`) plays every ported game with its steady hand and captures the guide card, a
//! level 1 round, a level 3 round and a disabled job's part step, beside the mockups' shots in
//! `docs/screenshots/repairs/`.

use std::collections::HashSet;
use std::path::PathBuf;

use egui::{Pos2, Rect, Vec2};
use sc_client::platform::{keys, App, Event, Flow, Frame};
use sc_core::repair::data::RepairData;
use sc_core::repair::{State, Who};
use sc_render::{Renderer, Target};
use sc_repairs::games;
use sc_repairs::kit::{CoverPhase, Input, Options, Runner, H, W};
use sc_repairs::pen::{c, hex, rgba, Align, Pen};

use crate::ui::{key_of, Ui};

/// The menu's columns, in order (each game's file names its group).
const GROUPS: [&str; 6] = ["Engineering", "Life", "Hangar", "Weapons and sensors", "Hull", "Outside and doors"];

/// A scripted capture's stage for one game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shot {
    Guide,
    Level1,
    Level3,
    Part,
}

/// The app.
pub struct RepairsApp {
    r: Renderer,
    target: Target,
    ui: Ui,
    data: RepairData,
    opts: Options,
    run: Option<Runner>,
    seen: HashSet<String>,
    input: Input,
    /// Window coordinates to canvas pixels, from the last frame.
    to_canvas: (f32, f32, f32),
    time_s: f64,
    shots: Option<PathBuf>,
    /// The scripted captures: the ported games left, and where the current one is.
    queue: Vec<&'static str>,
    shot: Option<(Shot, f32)>,
}

fn load() -> Result<RepairData, String> {
    games::load_shipped(std::path::Path::new("."))
}

impl RepairsApp {
    /// Set up; `open` a game at once if named.
    pub fn new(open: Option<String>, shots: Option<PathBuf>) -> Result<Self, String> {
        let text =
            std::fs::read_to_string("data/engine/render.json").map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(64, 36, 1);
        let data = load()?;
        // Captures: every ported game, or only the one named.
        let queue: Vec<&'static str> = if shots.is_some() {
            games::IDS
                .iter()
                .copied()
                .filter(|id| games::ported(id) && open.as_ref().is_none_or(|o| o == id))
                .rev()
                .collect()
        } else {
            Vec::new()
        };
        let open = if shots.is_some() { None } else { open };
        let mut app = Self {
            r,
            target,
            ui: Ui::new(),
            data,
            opts: Options::default(),
            run: None,
            seen: HashSet::new(),
            input: Input::default(),
            to_canvas: (0.0, 0.0, 1.0),
            time_s: 0.0,
            shots,
            queue,
            shot: None,
        };
        if let Some(id) = open {
            app.open(&id)?;
        }
        Ok(app)
    }

    fn open(&mut self, id: &str) -> Result<(), String> {
        let g = games::make(id).ok_or_else(|| format!("no repair game {id:?}"))?;
        let seen = self.seen.contains(id);
        self.run = Some(Runner::new(g, &self.data, self.opts, seen)?);
        Ok(())
    }

    /// The canvas's rectangle in points for an output `points` big: 16:9, centred.
    fn canvas(points: Vec2) -> Rect {
        let s = (points.x / W).min(points.y / H);
        let size = Vec2::new(W * s, H * s);
        Rect::from_min_size(Pos2::new((points.x - size.x) / 2.0, (points.y - size.y) / 2.0), size)
    }

    fn pointer(&mut self, x: f32, y: f32) {
        let (ox, oy, s) = self.to_canvas;
        let (cx, cy) = ((x - ox) / s, (y - oy) / s);
        if self.input.down {
            self.input.path.push([cx, cy]);
        }
        self.input.x = cx;
        self.input.y = cy;
    }

    fn capture(&self, f: &Frame, name: &str) {
        let Some(dir) = &self.shots else { return };
        let px = sc_render::read_output(f.width, f.height);
        match sc_client::capture::write_png(&dir.join(name), f.width, f.height, &px) {
            Ok(()) => println!("sc-client: wrote {}", dir.join(name).display()),
            Err(e) => eprintln!("sc-client: {e}"),
        }
    }

    /// The scripted captures: step the open game with its hand to the next shot. Returns a shot to take this frame.
    fn script(&mut self) -> Option<String> {
        if self.run.is_none() {
            let id = self.queue.pop()?;
            self.opts.state = State::Damaged;
            if let Err(e) = self.open(id) {
                eprintln!("sc-client: {e}");
                return None;
            }
            self.shot = Some((Shot::Guide, 0.0));
            return Some(format!("{id}-guide.png"));
        }
        let (stage, _) = self.shot?;
        let r = self.run.as_mut()?;
        let id = r.id();
        let dt = 1.0 / 60.0;
        r.guide_open = false;
        let live = |r: &Runner| r.cover.as_ref().is_none_or(|c| c.phase == CoverPhase::Work);
        let (want_round, next) = match stage {
            Shot::Guide => (0, Shot::Level1),
            Shot::Level1 => (2, Shot::Level3),
            Shot::Level3 | Shot::Part => (u32::MAX, Shot::Part),
        };
        if stage == Shot::Part {
            // The part's shot is taken; the next game.
            self.run = None;
            return self.script();
        }
        if want_round == u32::MAX {
            self.opts.state = State::Disabled;
            let seen = true;
            let g = games::make(id)?;
            let mut nr = Runner::new(g, &self.data, self.opts, seen).ok()?;
            for _ in 0..60 * 60 {
                if live(&nr) && nr.t > 0.5 {
                    break;
                }
                let i = nr.hand();
                nr.update(dt, &i);
            }
            self.run = Some(nr);
            self.shot = Some((Shot::Part, 0.0));
            return Some(format!("{id}-part.png"));
        }
        // Play on to `want_round`, then 2.5 s into it.
        let mut into = 0.0;
        for _ in 0..60 * 300 {
            if r.done {
                break;
            }
            if r.job.landed >= want_round && live(r) {
                into += dt;
                if into >= 2.5 {
                    break;
                }
            }
            let i = r.hand();
            r.update(dt, &i);
        }
        self.input = Input::default();
        self.shot = Some((next, 0.0));
        Some(format!("{id}-{}.png", if want_round == 0 { "l1" } else { "l3" }))
    }
}

impl App for RepairsApp {
    fn event(&mut self, e: Event) -> Flow {
        match e {
            Event::Quit => return Flow::Done,
            Event::KeyDown(keys::ESCAPE) => {
                if self.run.as_ref().is_some_and(|r| r.guide_open) {
                    self.input.hit.insert(egui::Key::Escape);
                } else if self.run.take().is_none() {
                    return Flow::Done;
                }
            }
            Event::Pointer(x, y) => self.pointer(x, y),
            Event::MouseButton(1, down) => {
                if down {
                    self.input.down = true;
                    self.input.pressed = true;
                    self.input.path = vec![[self.input.x, self.input.y]];
                } else {
                    self.input.down = false;
                    self.input.released = true;
                }
            }
            Event::Wheel(_, y) => self.input.wheel -= y,
            Event::KeyDown(k) => {
                if let Some(key) = key_of(k) {
                    if self.input.keys.insert(key) {
                        self.input.hit.insert(key);
                    }
                }
            }
            Event::KeyUp(k) => {
                if let Some(key) = key_of(k) {
                    self.input.keys.remove(&key);
                }
            }
            _ => {}
        }
        Flow::Continue
    }

    fn frame(&mut self, f: &Frame) -> Flow {
        let dt = if self.shots.is_some() { 1.0 / 60.0 } else { f.elapsed_s.clamp(0.0, 0.05) };
        self.time_s += dt;
        let shot = if self.shots.is_some() { self.script() } else { None };
        if self.shots.is_some() && shot.is_none() && self.run.is_none() && self.queue.is_empty() {
            return Flow::Done;
        }
        // Window coordinates to canvas pixels: the UI's points are a screen 720 high; the canvas is letterboxed in it.
        let ppp = (f.height as f32 / 720.0).max(0.5);
        let points = Vec2::new(f.width as f32 / ppp, f.height as f32 / ppp);
        let rect = Self::canvas(points);
        let win_to_pt = f.width as f32 / f.window_w.max(1.0) / ppp;
        self.to_canvas = (rect.min.x / win_to_pt, rect.min.y / win_to_pt, rect.width() / W / win_to_pt);
        let mut input = self.input.clone();
        input.derive();
        if let Some(r) = self.run.as_mut() {
            if self.shots.is_none() && r.update(dt as f32, &input) {
                self.seen.insert(r.id().to_owned());
            }
        }
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        self.r.end_pass();
        let mut open = None;
        let mut opts = self.opts;
        let (run, data) = (&self.run, &self.data);
        let frame = self.ui.run(&mut self.r, f, self.time_s, |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            painter.rect_filled(ctx.content_rect(), 0.0, c::BG);
            match run {
                Some(run) => run.draw(&painter, rect, &input),
                None => {
                    let pen = Pen::new(painter, rect);
                    open = MenuView { opts: &mut opts, data, input: &input }.draw(&pen);
                }
            }
        });
        self.opts = opts;
        if let Some(id) = open {
            if let Err(e) = self.open(id) {
                eprintln!("sc-client: {e}");
            }
        }
        // What happened this frame is spent; what is held carries on.
        let i = &mut self.input;
        i.pressed = false;
        i.released = false;
        i.hit.clear();
        i.wheel = 0.0;
        i.action_pressed = false;
        i.path = if i.down { vec![[i.x, i.y]] } else { Vec::new() };
        self.r.present_with_ui(&self.target, f.width, f.height, true, &frame.meshes(), frame.points);
        if let Some(name) = shot {
            self.capture(f, &name);
        }
        self.r.commit();
        Flow::Continue
    }
}

/// The menu, drawn from what it needs so the frame's closure borrows nothing else.
struct MenuView<'a> {
    opts: &'a mut Options,
    data: &'a RepairData,
    input: &'a Input,
}

impl MenuView<'_> {
    fn draw(&mut self, g: &Pen) -> Option<&'static str> {
        let i = self.input;
        g.rect(0.0, 0.0, W, H, c::BG);
        g.text("REPAIRS", 40.0, 48.0, 34.0, c::AMBER, Align::Left);
        let mut open = None;
        let mut x = 260.0;
        let pill = |label: &str, x: f32, w: f32, on: bool| -> bool {
            g.button(label, x, 26.0, w, 44.0, i.over(x, 26.0, w, 44.0), on, on.then_some(hex(0x3a2a12)));
            i.pressed_in(x, 26.0, w, 44.0)
        };
        for s in State::ALL {
            if pill(s.label(), x, 130.0, self.opts.state == s) {
                self.opts.state = s;
            }
            x += 140.0;
        }
        x += 30.0;
        for (w, label) in [(Who::Officer, "Officer"), (Who::Rating, "Rating")] {
            if pill(label, x, 120.0, self.opts.who == w) {
                self.opts.who = w;
            }
            x += 130.0;
        }
        x += 30.0;
        if pill("Combat", x, 120.0, self.opts.combat) {
            self.opts.combat = !self.opts.combat;
        }
        let col_w = (W - 80.0) / GROUPS.len() as f32;
        for (k, grp) in GROUPS.iter().enumerate() {
            let gx = 40.0 + k as f32 * col_w;
            g.text(&grp.to_uppercase(), gx, 120.0, 16.0, c::DIM, Align::Left);
            let mut y = 140.0;
            for id in games::IDS.iter().filter(|id| self.data.games.get(**id).is_some_and(|f| f.group == *grp)) {
                let title = self.data.games.get(*id).map(|f| f.title.as_str()).unwrap_or(id);
                let live = games::ported(id);
                let (bw, bh) = (col_w - 14.0, 56.0);
                let over = i.over(gx, y, bw, bh);
                g.round(
                    gx,
                    y,
                    bw,
                    bh,
                    12.0,
                    Some(if over { hex(0x1d2738) } else { c::PANEL }),
                    Some((2.0, if live { hex(0x33445a) } else { c::LINE })),
                );
                g.text(title, gx + 14.0, y + 22.0, 18.0, if live { c::FG } else { c::DIM }, Align::Left);
                g.text(
                    if live { "Ready" } else { "Mockup only" },
                    gx + 14.0,
                    y + 42.0,
                    13.0,
                    if live { c::OK } else { rgba(111, 127, 148, 0.7) },
                    Align::Left,
                );
                if i.pressed_in(gx, y, bw, bh) {
                    open = Some(*id);
                }
                y += bh + 10.0;
            }
        }
        g.text(
            "Esc quits. In a game: Esc for this menu, F1 or ? for its guide.",
            40.0,
            H - 30.0,
            16.0,
            c::DIM,
            Align::Left,
        );
        open
    }
}
