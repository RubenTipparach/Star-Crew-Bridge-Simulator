//! What the game draws around the consoles (openspec/changes/ui-art-direction design 7 and 11): the walking HUD (one
//! objective card, the centre dot, the alert chip at alert only), the prompt plate, the first-time hint and its
//! marker, the loading screen and the briefing, and the crew and objective rows the briefing and the Esc menu share.
//!
//! Each is drawn as the approved mockup draws it (shots hud-after, prompt-after, hint-after, loading-after,
//! briefing-after), in reference pixels hung from an anchor of the screen (`ui::anchored`), with the style of
//! `data/ui/ui_style.json`. It lives in the client because it is the client's screen; what it shows is gathered by
//! the game (`drill.rs`), which decides nothing here. A prompt is data, a label, its keys and verbs (design 7), so the
//! keyboard's plate, the touch layer's USE word and the log line are one prompt.

use crate::look::{self, hit, kit, look, Align, KeyTok, Kit, Role};
use crate::ui::Anchor;
use egui::{Color32, Pos2, Rect, Vec2};
use sc_core::combat::Station;

/// One crew row (the briefing's and the Esc menu's): the station's role icon and full name, who holds it, and what
/// stands at the row's right.
#[derive(Clone, Debug, PartialEq)]
pub struct CrewRow {
    /// The station.
    pub station: Station,
    /// Who holds it: an officer's name, or none for the ship's computer.
    pub name: Option<String>,
    /// This player's own.
    pub you: bool,
    /// Ready (a check at the right).
    pub ready: bool,
    /// Held by the crew's AI: the faint crew-AI icon after the name (Q3), never "(bot)".
    pub npc: bool,
}

/// A station's role id for its icon and colour (the consoles' `stations.json` roles).
pub fn role_of(s: Station) -> &'static str {
    match s {
        Station::Helm => "helm",
        Station::Tactical => "tactical",
        Station::Engineering => "engineering",
        Station::Science => "science",
        Station::Captain => "command",
        Station::LaserPort | Station::LaserStbd => "laser",
        Station::Gunner => "gunner",
    }
}

/// The name nobody holds a station by: the ship runs it (design 12; never "automation").
pub const COMPUTER: &str = "Ship's computer";

/// Crew rows from `x`, `y`, `w` wide, each `h` high with `pad` at the sides (the mockup's `crewRows`); a press on a
/// row is said under `mark_prefix` and the row's station returned.
#[allow(clippy::too_many_arguments)]
pub fn crew_rows(
    ui: &mut egui::Ui,
    k: &Kit,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pad: f32,
    rows: &[CrewRow],
    mark_prefix: Option<&str>,
) -> Option<Station> {
    let c = look().c();
    let mut pressed = None;
    for (i, r) in rows.iter().enumerate() {
        let top = y + i as f32 * h;
        let cy = top + h / 2.0;
        if r.you {
            k.fill(x, top, w, h, c.raised.0);
            k.bar(x, top, h);
        }
        let role = role_of(r.station);
        k.role_icon(role, x + pad + 14.0, cy, 28.0, look().role(role));
        k.text_mid(Role::Body, x + pad + 46.0, cy, c.text.0, &station_name(r.station), Align::Start);
        // The right end: YOU in an accent box, the crew-AI icon where the ship's computer sits, a check when ready.
        let right = x + w - pad;
        let tag_w = if r.you {
            let tw = k.width(Role::Label, "You") + 20.0;
            let th = k.line(Role::Label) + 9.0;
            k.edge(right - tw, cy - th / 2.0, tw, th, c.accent.0);
            k.text_mid(Role::Label, right - tw / 2.0 + 1.0, cy, c.accent.0, "You", Align::Middle);
            tw
        } else if r.name.is_none() || r.npc && !r.ready {
            k.role_icon("auto", right - 13.0, cy, 26.0, c.faint.0);
            26.0
        } else if r.ready {
            k.icon("check", right - 13.0, cy, 26.0, c.ok.0);
            26.0
        } else {
            26.0
        };
        let name_x = x + pad + 46.0 + 210.0 + 18.0;
        let name_w = right - tag_w - 18.0 - name_x;
        let (who, col) = match &r.name {
            Some(n) => (n.as_str(), if r.you { c.text.0 } else { c.dim.0 }),
            None => (COMPUTER, c.faint.0),
        };
        k.text_clip(Role::Body, name_x, cy - k.line(Role::Body) / 2.0, col, who, name_w);
        if let Some(m) = mark_prefix {
            if hit(ui, k.r(x, top, w, h), &format!("{m} {}", r.station.name())).clicked() {
                pressed = Some(r.station);
            }
        }
    }
    pressed
}

/// A station's full name (design 12: "Starboard laser", never "Laser stbd").
pub fn station_name(s: Station) -> String {
    s.name().to_owned()
}

/// Objective rows from `x`, `y` (48 each): a diamond for one open, a check and a struck-out faint line for one done.
pub fn objectives(k: &Kit, x: f32, y: f32, rows: &[(String, bool)]) -> f32 {
    let c = look().c();
    for (i, (t, done)) in rows.iter().enumerate() {
        let cy = y + i as f32 * 48.0 + 24.0;
        if *done {
            k.icon("check", x + 11.0, cy, 22.0, c.ok.0);
        } else {
            k.glyph("diamond", x + 11.0, cy, 22.0, c.accent.0);
        }
        let col = if *done { c.faint.0 } else { c.text.0 };
        let tw = k.text_mid(Role::Body, x + 38.0, cy, col, t, Align::Start);
        if *done {
            k.fill(x + 38.0, cy + 1.0, tw, 2.0, c.faint.0);
        }
    }
    rows.len() as f32 * 48.0
}

/// The objective card (shot hud-after): top left in the safe area, the accent bar, OBJECTIVE over one line.
pub fn objective_card(painter: &egui::Painter, screen: Rect, text: &str) {
    let c = look().c();
    let probe = kit(painter, screen, Anchor::TopLeft, Vec2::ZERO);
    let w = (probe.width(Role::Body, text) + 62.0).max(420.0);
    let h = 14.0 + probe.line(Role::Label) + 4.0 + probe.line(Role::Body) + 16.0;
    let k = kit(painter, screen, Anchor::TopLeft, Vec2::new(96.0 + w, 54.0 + h));
    k.panel(96.0, 54.0, w, h);
    k.bar(96.0, 54.0, h);
    k.text(Role::Label, 130.0, 68.0, c.accent.0, "Objective", Align::Start);
    k.text(Role::Body, 130.0, 68.0 + k.line(Role::Label) + 4.0, c.text.0, text, Align::Start);
}

/// The centre dot (6 reference pixels across), the only mark at rest in the middle of the screen.
pub fn reticle(painter: &egui::Painter, screen: Rect) {
    let k = kit(painter, screen, Anchor::Centre, Vec2::ZERO);
    k.cv.circle(0.0, 0.0, 3.0, look::alpha(look().c().text.0, 0.85), None);
}

/// The alert chip (design 11, Q6): top right, only at red alert or on emergency power, the state's colour with a shape.
pub fn alert_chip(painter: &egui::Painter, screen: Rect, red: bool) {
    let c = look().c();
    let word = if red { "Red alert" } else { "Emergency power" };
    let col = if red { c.danger.0 } else { c.warn.0 };
    let probe = kit(painter, screen, Anchor::TopRight, Vec2::ZERO);
    let w = probe.width(Role::Label, word) + 28.0 + 12.0 + 48.0;
    let k = kit(painter, screen, Anchor::TopRight, Vec2::new(w + 96.0, 54.0 + 64.0));
    k.panel(0.0, 54.0, w, 64.0);
    k.cv.rect(0.0, 54.0, w, 64.0, 0.0, Color32::TRANSPARENT, Some(crate::vg::Pen::new(2.0, col)));
    k.icon("bang", 24.0 + 14.0, 86.0, 28.0, col);
    k.text_mid(Role::Label, 24.0 + 28.0 + 12.0, 86.0, col, word, Align::Start);
}

/// A walking prompt (design 7): the thing's name, then each key with its verb; or a word where nothing is to be done.
#[derive(Clone, Debug, PartialEq)]
pub struct Prompt {
    /// The thing in reach ("Helm", "Lift", "Fire").
    pub label: String,
    /// Each key (or held key) and the verb it does.
    pub acts: Vec<(Vec<KeyTok>, String)>,
    /// A word with no key (a crewmate's name, "Empty", "Find an extinguisher").
    pub note: Option<String>,
    /// The extinguisher's agent under the plate, 0-1, while spraying a fire.
    pub bar: Option<f32>,
    /// Said in the log when it differs from what the screen shows (a crewmate walking to a seat).
    pub log_extra: Option<String>,
}

impl Prompt {
    /// A prompt for `label` with one key and its verb.
    pub fn act(label: &str, key: Vec<KeyTok>, verb: &str) -> Self {
        Prompt { label: label.into(), acts: vec![(key, verb.into())], note: None, bar: None, log_extra: None }
    }

    /// A prompt with nothing to press.
    pub fn note(label: &str, note: &str) -> Self {
        Prompt { label: label.into(), acts: Vec::new(), note: Some(note.into()), bar: None, log_extra: None }
    }

    /// The log's line (a playtest's and the browser smoke test's view of the walk): "Helm [E] Sit down".
    pub fn line(&self) -> String {
        let mut s = self.label.clone();
        for (keys, verb) in &self.acts {
            let k: Vec<String> = keys
                .iter()
                .map(|t| match t {
                    KeyTok::Key(k) => k.clone(),
                    KeyTok::Glyph(g) => g.clone(),
                    KeyTok::Hold => "hold".into(),
                    KeyTok::Or => "/".into(),
                })
                .collect();
            s += &format!(" [{}] {verb}", k.join(" "));
        }
        if let Some(n) = &self.note {
            s += &format!(" {n}");
        }
        if let Some(x) = &self.log_extra {
            s += &format!(" {x}");
        }
        s
    }

    /// The word USE carries by thumb (touch-controls 3): the verb of the use key, or of the spray. None lights nothing.
    pub fn touch_word(&self, use_label: &str, spray_label: &str) -> Option<String> {
        self.acts
            .iter()
            .find(|(k, _)| k.contains(&KeyTok::Key(use_label.into())) || k.contains(&KeyTok::Key(spray_label.into())))
            .map(|(_, v)| v.split_whitespace().next().unwrap_or(v).to_owned())
    }

    /// Whether it offers the down key (the touch layer then shows DOWN).
    pub fn has_key(&self, label: &str) -> bool {
        self.acts.iter().any(|(k, _)| k.contains(&KeyTok::Key(label.into())))
    }
}

/// The prompt plate (shot prompt-after): solid, below the centre dot, the label over its keys and verbs, at opacity
/// `a` (it fades in and out, design 8). By thumb (`touch`) the keys are not drawn: USE carries the verb.
pub fn prompt_plate(painter: &egui::Painter, screen: Rect, p: &Prompt, a: f32, touch: bool) {
    if a <= 0.0 {
        return;
    }
    let c = look().c();
    let k = kit(painter, screen, Anchor::Centre, Vec2::new(1920.0, 1080.0)).faded(a);
    // The row's width: each act's keys, 14, its verb; 32 between acts; or the note.
    let mut parts: Vec<(f32, f32)> = Vec::new();
    for (keys, verb) in &p.acts {
        let kw = if touch { 0.0 } else { k.keys_w(keys) + 14.0 };
        parts.push((kw, k.width(Role::Body, verb)));
    }
    let note_w = p.note.as_ref().map_or(0.0, |n| k.width(Role::Body, n));
    let row_w = parts.iter().map(|(a, b)| a + b).sum::<f32>() + 32.0 * parts.len().saturating_sub(1) as f32 + note_w;
    let label_w = k.width(Role::Label, &p.label);
    let w = row_w.max(label_w) + 48.0;
    let row_h =
        if p.acts.is_empty() || touch { k.line(Role::Body) } else { look().f.sizes_px.key_h.max(k.line(Role::Body)) };
    let bar_h = if p.bar.is_some() { 6.0 + 12.0 } else { 0.0 };
    let h = 14.0 + k.line(Role::Label) + 8.0 + row_h + bar_h + 16.0;
    let x = 960.0 - w / 2.0;
    k.panel(x, 640.0, w, h);
    k.text(Role::Label, 960.0, 654.0, c.dim.0, &p.label, Align::Middle);
    let cy = 654.0 + k.line(Role::Label) + 8.0 + row_h / 2.0;
    let mut at = 960.0 - row_w / 2.0;
    for ((keys, verb), (kw, vw)) in p.acts.iter().zip(&parts) {
        if !touch {
            k.keys(at, cy, keys);
        }
        k.text_mid(Role::Body, at + kw, cy, c.text.0, verb, Align::Start);
        at += kw + vw + 32.0;
    }
    if let Some(n) = &p.note {
        k.text_mid(Role::Body, at, cy, c.text.0, n, Align::Start);
    }
    if let Some(b) = p.bar {
        k.progress(x + 24.0, cy + row_h / 2.0 + 12.0, w - 48.0, b);
    }
}

/// The extinguisher in hand (design 7): a small bar beside the centre dot, its agent left.
pub fn carried_bar(painter: &egui::Painter, screen: Rect, agent: f32) {
    let k = kit(painter, screen, Anchor::Centre, Vec2::ZERO);
    let c = look().c();
    k.fill(24.0, -30.0, 8.0, 60.0, c.line.0);
    let h = 60.0 * agent.clamp(0.0, 1.0);
    k.fill(24.0, 30.0 - h, 8.0, h, c.accent.0);
}

/// The first-time hint (shot hint-after): a card at the middle left, First steps, "Walk to the {station} and sit
/// down." and three keys. `marker`: the station's seat on screen (points) and its distance, metres.
pub fn first_steps(painter: &egui::Painter, screen: Rect, station: &str, sit: &[KeyTok], marker: Option<(Pos2, f32)>) {
    let c = look().c();
    if let Some((at, dist)) = marker {
        let (_, s) = crate::ui::anchored(screen, Anchor::Centre, Vec2::ZERO);
        let m = Kit::new(painter.clone(), at, s);
        m.text(Role::Label, 0.0, -36.0 - m.line(Role::Label), c.accent.0, station, Align::Middle);
        m.glyph("marker", 0.0, 0.0, 34.0, c.accent.0);
        m.px(18.0).text(Role::Label, 0.0, 23.0, c.dim.0, &format!("{dist:.0} m"), Align::Middle);
    }
    let h = 4.0 + 22.0 + 24.0 + 8.0 + 38.0 + 16.0 + 3.0 * 56.0 + 24.0;
    let k = kit(painter, screen, Anchor::MidLeft, Vec2::new(96.0 + 460.0, h));
    k.panel(96.0, 0.0, 460.0, h);
    k.fill(96.0, 0.0, 460.0, 4.0, c.accent.0);
    k.text(Role::Label, 124.0, 26.0, c.accent.0, "First steps", Align::Start);
    let line = format!("Walk to the {} and sit down.", station.to_lowercase());
    k.text(Role::Body, 124.0, 58.0, c.text.0, &line, Align::Start);
    let rows: [(&str, Vec<KeyTok>); 3] = [
        ("Move", ["W", "A", "S", "D"].iter().map(|k| KeyTok::Key((*k).into())).collect()),
        ("Look", vec![KeyTok::Glyph("mouse".into())]),
        ("Sit", sit.to_vec()),
    ];
    for (i, (word, keys)) in rows.iter().enumerate() {
        let cy = 112.0 + i as f32 * 56.0 + 28.0;
        k.text_mid(Role::Body, 124.0, cy, c.dim.0, word, Align::Start);
        k.keys_right(96.0 + 460.0 - 28.0, cy, keys);
    }
}

/// One tip of the loading screen (`data/ui/tips.json`): its keys and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Tip {
    pub keys: Vec<KeyTok>,
    pub text: String,
}

/// `data/ui/tips.json` (`starcrew.ui-tips/1`): the loading screen's tips, each its keys (as the Controls table names
/// them) and its words.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TipsFile {
    pub schema: String,
    pub tips: Vec<TipFile>,
}

/// One tip in the file.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TipFile {
    pub keys: Vec<String>,
    pub text: String,
}

impl sc_core::data::Validate for TipsFile {
    fn validate(&self, c: &mut sc_core::data::Checks) {
        c.equals("schema", &self.schema, "starcrew.ui-tips/1");
        c.count("tips", self.tips.len() as i64, 1, 64);
        let b = crate::bindings::Bindings::shipped();
        for (i, t) in self.tips.iter().enumerate() {
            if b.tokens(&t.keys).is_none() {
                c.equals(&format!("tips[{i}].keys"), &t.keys.join(" "), "keys as the Controls table names them");
            }
            c.count(&format!("tips[{i}].text"), t.text.chars().count() as i64, 1, 60);
        }
    }
}

/// The tips file's path.
pub const TIPS_FILE: &str = "data/ui/tips.json";

/// The `n`th tip (wrapping), its keys as `b` binds them; None when the file cannot be read (said once).
pub fn tip(b: &crate::bindings::Bindings, n: u64) -> Option<Tip> {
    let read = || -> Result<TipsFile, String> {
        let text = std::fs::read_to_string(TIPS_FILE).map_err(|e| format!("{TIPS_FILE}: {e}"))?;
        sc_core::data::parse(TIPS_FILE, &text).map_err(|e| e.to_string())
    };
    match read() {
        Ok(f) => {
            let t = &f.tips[(n % f.tips.len() as u64) as usize];
            Some(Tip { keys: b.tokens(&t.keys)?, text: t.text.clone() })
        }
        Err(e) => {
            eprintln!("sc-client: {e}");
            None
        }
    }
}

/// The loading screen (shot loading-after): the key art, the mission's label and name, one tip with its keys, and
/// "Boarding the Tern" over a thin bar at `progress` (0-1). No percentages or file names.
pub fn loading(
    painter: &egui::Painter,
    screen: Rect,
    art: Option<&egui::TextureHandle>,
    label: &str,
    title: &str,
    tip: Option<&Tip>,
    progress: f32,
) {
    let c = look().c();
    look::cover_art(painter, screen, art, 0.5, 1.0);
    let flat = Kit::new(painter.clone(), screen.min, 1.0);
    let (w, h) = (screen.width(), screen.height());
    flat.vgrad(
        0.0,
        0.0,
        w,
        h,
        &[(0.0, look::ground(0.0)), (0.44, look::ground(0.0)), (0.76, look::ground(0.86)), (1.0, look::ground(1.0))],
    );
    let bl = kit(painter, screen, Anchor::BottomLeft, Vec2::new(1200.0, 300.0));
    let tip_cy = 300.0 - 54.0 - 22.0;
    let title_top = tip_cy - 22.0 - 28.0 - bl.line(Role::Display);
    let label_top = title_top - 12.0 - bl.line(Role::Label);
    bl.text(Role::Label, 96.0, label_top, c.accent.0, label, Align::Start);
    bl.text(Role::Display, 96.0, title_top, c.text.0, title, Align::Start);
    if let Some(t) = tip {
        let tw = bl.text_mid(Role::Label, 96.0, tip_cy, c.faint.0, "Tip", Align::Start);
        let kw = bl.keys(96.0 + tw + 14.0, tip_cy, &t.keys);
        bl.text_mid(Role::Body, 96.0 + tw + 14.0 + kw + 14.0, tip_cy, c.dim.0, &t.text, Align::Start);
    }
    let br = kit(painter, screen, Anchor::BottomRight, Vec2::new(640.0, 140.0));
    let bar_y = 140.0 - 62.0 - 6.0;
    br.text(Role::Label, 640.0 - 96.0, bar_y - 14.0 - br.line(Role::Label), c.dim.0, "Boarding the Tern", Align::End);
    br.progress(0.0, bar_y, 544.0, progress);
}

/// What the briefing shows (design 11, shot briefing-after).
pub struct Briefing<'a> {
    /// The small line over the title ("Training drill 1").
    pub label: &'a str,
    pub title: &'a str,
    pub situation: &'a str,
    pub objectives: Vec<(String, bool)>,
    /// This player's station and its orders.
    pub orders: (Station, &'a str),
    pub crew: Vec<CrewRow>,
    /// The countdown: seconds left and the share gone (Launch in).
    pub launch: Option<(f64, f32)>,
    /// READY: shown, pressed, or waiting for the crew to sit.
    pub ready: Option<ReadyState>,
}

/// The briefing's READY.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyState {
    /// Not pressed.
    Open,
    /// Pressed: this player is ready.
    Ready,
    /// Pressed in single player while the crew still walk to their seats.
    Waiting,
}

/// What the briefing asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BriefingAsk {
    /// READY pressed (true) or taken back (false).
    Ready(bool),
    /// A crew row pressed: take that station.
    Claim(Station),
}

/// The briefing, a solid screen on the console before the round (design 11, Q7): the system's art fading into the
/// ground at the top, the mission on the left with your orders raised, the crew on the right, Launch in at the bottom
/// right (READY until the round counts down).
pub fn briefing(
    ui: &mut egui::Ui,
    screen: Rect,
    art: Option<&egui::TextureHandle>,
    b: &Briefing<'_>,
) -> Option<BriefingAsk> {
    let c = look().c();
    let painter = ui.painter().clone();
    painter.rect_filled(screen, 0.0, c.ground.0);
    // The art band: the top 420 reference pixels, 55 % opaque, fading into the ground.
    let (_, s) = crate::ui::anchored(screen, Anchor::Centre, Vec2::ZERO);
    let band = Rect::from_min_size(screen.min, Vec2::new(screen.width(), 420.0 * s));
    look::cover_art(&painter.with_clip_rect(band), band, art, 0.38, 0.55);
    let flat = Kit::new(painter.clone(), screen.min, 1.0);
    flat.vgrad(0.0, 0.0, band.width(), band.height(), &[(0.0, look::ground(0.2)), (1.0, c.ground.0)]);
    let k = kit(&painter, screen, Anchor::Centre, Vec2::new(1920.0, 1080.0));
    let mut ask = None;
    k.text(Role::Label, 96.0, 96.0, c.accent.0, b.label, Align::Start);
    let mut y = 96.0 + k.line(Role::Label) + 12.0;
    k.text(Role::Display, 96.0, y, c.text.0, b.title, Align::Start);
    y += k.line(Role::Display) + 24.0;
    y += k.para(Role::Body, 96.0, y, 880.0, c.dim.0, b.situation);
    y += 48.0;
    k.text(Role::Label, 96.0, y, c.dim.0, "Objectives", Align::Start);
    y += k.line(Role::Label) + 12.0;
    y += objectives(&k, 96.0, y, &b.objectives);
    y += 48.0;
    // Your orders, raised with the accent bar and the station's icon.
    let (st, orders) = b.orders;
    let role = role_of(st);
    let probe_h = {
        let p = k.faded(0.0);
        p.para(Role::Body, 134.0, 0.0, 880.0 - 70.0, c.text.0, orders)
    };
    let oh = 28.0 + k.line(Role::Label) + 12.0 + probe_h + 30.0;
    k.fill(96.0, y, 880.0, oh, c.raised.0);
    k.bar(96.0, y, oh);
    k.role_icon(role, 148.0, y + 28.0 + k.line(Role::Label) / 2.0, 28.0, look().role(role));
    k.text(Role::Label, 174.0, y + 28.0, look().role(role), "Your orders", Align::Start);
    k.para(Role::Body, 134.0, y + 28.0 + k.line(Role::Label) + 12.0, 880.0 - 70.0, c.text.0, orders);
    // The crew.
    let ch = 24.0 + k.line(Role::Label) + 12.0 + b.crew.len() as f32 * 72.0 + 8.0;
    k.panel(1056.0, 96.0, 768.0, ch);
    k.text(Role::Label, 1088.0, 120.0, c.dim.0, "Crew", Align::Start);
    if let Some(st) =
        crew_rows(ui, &k, 1057.0, 120.0 + k.line(Role::Label) + 12.0, 766.0, 72.0, 32.0, &b.crew, Some("claim"))
    {
        ask = Some(BriefingAsk::Claim(st));
    }
    // Launch in, or READY.
    match (b.launch, b.ready) {
        (Some((left, share)), _) => {
            let num = format!("{}:{:02}", (left as u64) / 60, left.ceil() as u64 % 60);
            let base = 860.0 + k.line(Role::Display);
            k.text(Role::Label, 1056.0, base - k.line(Role::Label) - 4.0, c.dim.0, "Launch in", Align::Start);
            k.text(Role::Display, 1824.0, 860.0, c.text.0, &num, Align::End);
            k.progress(1056.0, base + 16.0, 768.0, share);
        }
        (None, Some(r)) => {
            let sz = &look().f.sizes_px;
            let (y, h) = (860.0, sz.primary_h);
            let (word, primary) = match r {
                ReadyState::Open => ("Ready", true),
                ReadyState::Ready => ("Ready", false),
                ReadyState::Waiting => ("Crew sitting down", false),
            };
            if primary {
                k.fill(1056.0, y, 768.0, h, c.accent.0);
                k.text_mid(Role::Title, 1440.0, y + h / 2.0, c.ink.0, word, Align::Middle);
            } else {
                k.fill(1056.0, y, 768.0, h, c.raised.0);
                k.edge(1056.0, y, 768.0, h, c.line2.0);
                let tw = k.width(Role::Title, word);
                k.icon("check", 1440.0 - tw / 2.0 - 24.0, y + h / 2.0, 28.0, c.ok.0);
                k.text_mid(Role::Title, 1440.0 + 12.0, y + h / 2.0, c.text.0, word, Align::Middle);
            }
            if hit(ui, k.r(1056.0, y, 768.0, h), "READY").clicked() {
                ask = Some(BriefingAsk::Ready(r == ReadyState::Open));
            }
        }
        (None, None) => {}
    }
    ask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tips_file_is_valid_and_its_keys_are_the_bindings() {
        let text = include_str!("../../../data/ui/tips.json");
        let f: TipsFile = sc_core::data::parse(TIPS_FILE, text).expect("the shipped tips are valid");
        let b = crate::bindings::Bindings::shipped();
        assert!(f.tips.iter().all(|t| b.tokens(&t.keys).is_some()));
        let bad = text.replacen("\"M\"", "\"@teleport\"", 1);
        assert!(sc_core::data::parse::<TipsFile>(TIPS_FILE, &bad).is_err(), "a key that is no binding is refused");
    }

    #[test]
    fn a_prompt_is_one_thing_for_the_plate_the_thumb_and_the_log() {
        let e = vec![KeyTok::Key("E".into())];
        let sit = Prompt::act("Helm", e.clone(), "Sit down");
        assert_eq!(sit.line(), "Helm [E] Sit down", "the log names the key and the verb");
        assert_eq!(sit.touch_word("E", "F").as_deref(), Some("Sit"), "USE carries the verb by thumb");
        let lift = Prompt {
            acts: vec![(e, "Up".into()), (vec![KeyTok::Key("Q".into())], "Down".into())],
            ..Prompt::act("Lift", Vec::new(), "")
        };
        assert!(lift.has_key("Q"), "the lift offers DOWN");
        let spray = Prompt::act("Fire", vec![KeyTok::Hold, KeyTok::Key("F".into())], "Spray");
        assert_eq!(spray.touch_word("E", "F").as_deref(), Some("Spray"));
        let held = Prompt::note("Captain", "Lt. Sa'ral");
        assert_eq!(held.touch_word("E", "F"), None, "nothing to press lights nothing");
    }
}
