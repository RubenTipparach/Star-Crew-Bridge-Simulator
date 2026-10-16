//! A repair played on deck (openspec/changes/all-hands 5, repairs-on-deck 3): the machine's repair game, drawn a little
//! inside the screen so the live ship shows round it, played with the pointer and the keys while the body stands at
//! the machine. Each round it lands goes to the server with how long it took, and the server checks it and lands it on
//! the ship (`Drill::round_done`); the job is done when the server says so.
//!
//! It lives in the client because it is the player's hands at the machine; the game is `sc-repairs`' own, run by the
//! same `Runner` as the repairs menu (one implementation, CLAUDE.md 6.1).

use egui::{Painter, Pos2, Rect, Vec2};
use sc_client::platform::{keys, Event, Frame};
use sc_core::repair::data::RepairData;
use sc_core::repair::{State, Who};
use sc_repairs::kit::{Input, Options, Runner, H, W};

/// The game's canvas takes this share of the screen; the ship shows round it.
const CANVAS_SHARE: f32 = 0.86;

/// A repair being played at a machine.
pub struct DeckRun {
    /// The job (its index in the Tern's jobs).
    pub job: u8,
    /// The round the server gave, and when it began, local seconds.
    pub round: u8,
    round_t0: f64,
    landed: u32,
    runner: Runner,
    input: Input,
    /// The output on the reference frame and where the canvas is drawn on it, from the last frame: the pointer's way
    /// from window coordinates to canvas units (`ui::Fit::to_canvas`).
    to_canvas: (crate::ui::Fit, Rect),
}

impl DeckRun {
    /// Open `game` for job `job` from `state`, at the round the server gave.
    pub fn open(
        data: &RepairData,
        job: u8,
        game: &str,
        state: State,
        seed: u64,
        round: u8,
        now: f64,
    ) -> Result<Self, String> {
        let g = sc_repairs::games::make(game).ok_or_else(|| format!("no repair game {game:?}"))?;
        let opts = Options { who: Who::Officer, state, combat: true, seed };
        let runner = Runner::new(g, data, opts, true)?;
        Ok(Self {
            job,
            round,
            round_t0: now,
            landed: 0,
            runner,
            input: Input::default(),
            to_canvas: (crate::ui::Fit::default(), crate::ui::Fit::default().frame()),
        })
    }

    /// The game's title.
    pub fn title(&self) -> &str {
        &self.runner.data.title
    }

    /// The canvas on a screen of `points`: the game's 1280 x 720 on the reference frame (`ui::frame_in`), a little
    /// inside it.
    pub fn canvas(points: Vec2) -> Rect {
        let (o, s) = crate::ui::frame_in(Rect::from_min_size(Pos2::ZERO, points));
        let frame = Rect::from_min_size(o, Vec2::new(W, H) * s);
        Rect::from_center_size(frame.center(), frame.size() * CANVAS_SHARE)
    }

    /// The pointer, the buttons and the keys, as the repairs menu takes them.
    pub fn event(&mut self, e: &Event) {
        let i = &mut self.input;
        match e {
            Event::Pointer(x, y) => {
                let (fit, rect) = self.to_canvas;
                let [cx, cy] = fit.to_canvas(*x, *y, rect, Vec2::new(W, H));
                if i.down {
                    i.path.push([cx, cy]);
                }
                i.x = cx;
                i.y = cy;
            }
            Event::MouseButton(1, down) => {
                if *down {
                    i.down = true;
                    i.pressed = true;
                    i.path = vec![[i.x, i.y]];
                } else {
                    i.down = false;
                    i.released = true;
                }
            }
            Event::Wheel(_, y) => i.wheel -= y,
            Event::KeyDown(k) if *k != keys::ESCAPE => {
                if let Some(key) = crate::ui::key_of(*k) {
                    if i.keys.insert(key) {
                        i.hit.insert(key);
                    }
                }
            }
            Event::KeyUp(k) => {
                if let Some(key) = crate::ui::key_of(*k) {
                    i.keys.remove(&key);
                }
            }
            _ => {}
        }
    }

    /// One frame of `dt` seconds on a frame `f`: the game plays (by the player's hands, or the steady hand's for a
    /// scripted player). Returns how long the round took when one landed this frame.
    pub fn update(&mut self, f: &Frame, dt: f64, now: f64, scripted: bool) -> Option<f32> {
        let fit = crate::ui::Fit::of(f);
        self.to_canvas = (fit, Self::canvas(fit.points));
        if scripted {
            self.runner.close_guide();
            self.input = self.runner.hand();
        }
        let mut input = self.input.clone();
        input.derive();
        self.runner.update(dt as f32, &input);
        // What happened this frame is spent; what is held carries on.
        let i = &mut self.input;
        i.pressed = false;
        i.released = false;
        i.hit.clear();
        i.wheel = 0.0;
        i.action_pressed = false;
        i.path = if i.down { vec![[i.x, i.y]] } else { Vec::new() };
        if self.runner.job.landed > self.landed {
            self.landed = self.runner.job.landed;
            let took = (now - self.round_t0) as f32;
            self.round_t0 = now;
            return Some(took);
        }
        None
    }

    /// The server gave the next round.
    pub fn next_round(&mut self, round: u8, now: f64) {
        self.round = round;
        self.round_t0 = now;
    }

    /// The game played to its end (its cover back on).
    pub fn done(&self) -> bool {
        self.runner.done
    }

    /// Draw it over the screen of `points`.
    pub fn draw(&self, painter: &Painter, points: Vec2) {
        let mut input = self.input.clone();
        input.derive();
        self.runner.draw(painter, Self::canvas(points), &input);
    }
}
