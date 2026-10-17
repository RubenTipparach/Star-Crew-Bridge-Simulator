//! The UI layer (openspec/changes/lobby, design 3): egui, fed the platform's input and painted by `sc-render`'s UI
//! painter. It lives in the client because egui is the client's choice of immediate-mode UI; the renderer only sees
//! triangles, clip rectangles and textures by id.
//!
//! Each frame: the platform's events since the last frame become egui's input, the app's UI code runs, the shapes are
//! tessellated into meshes, and the texture changes (the font atlas, first frame and when new glyphs appear) are
//! uploaded. The painter draws the meshes over the 3D picture in the output pass.

use egui::{Color32, Pos2, Vec2};
use sc_client::platform::{self, keys, Event, Frame};
use sc_render::{Renderer, UiMesh, UiVertex};
use std::collections::HashMap;

/// One clipped mesh: its clip rectangle in pixels, its texture, vertices and indices.
type OwnedMesh = ([i32; 4], u64, Vec<UiVertex>, Vec<u32>);

/// One frame's tessellated UI, owned until it is painted.
pub struct UiFrame {
    meshes: Vec<OwnedMesh>,
    /// The output's size in points.
    pub points: [f32; 2],
    /// Output pixels a point.
    pub ppp: f32,
}

impl UiFrame {
    /// The meshes as the painter takes them.
    pub fn meshes(&self) -> Vec<UiMesh<'_>> {
        self.meshes
            .iter()
            .map(|(clip, tex, v, i)| UiMesh { clip_px: *clip, texture: *tex, vertices: v, indices: i })
            .collect()
    }
}

/// egui and what it has been told.
pub struct Ui {
    /// The egui context the app's UI code draws with.
    pub ctx: egui::Context,
    events: Vec<egui::Event>,
    pointer: Pos2,
    /// The textures' pixels as egui last set them (a patch updates the copy, then the whole texture is uploaded).
    textures: HashMap<u64, (usize, usize, Vec<u8>)>,
    text_on: bool,
    /// The output on the reference frame, from the last frame (design 5).
    fit: Fit,
    /// Where each marked control was last said to be, window coordinates ([`mark`]).
    marks: HashMap<String, [i32; 4]>,
    /// The first finger as the pointer: menus and buttons are tapped (touch-controls design 2).
    touch: platform::TouchMouse,
}

/// `data/ui/reference.json` (`starcrew.ui-reference/1`): the one reference frame every screen is laid out on
/// (openspec/changes/browser-page-fit design 5).
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// `starcrew.ui-reference/1`.
    pub schema: String,
    /// The frame's size in reference pixels (1920 x 1080).
    pub reference_px: [f64; 2],
    /// The frame's size in layout points, the units every layout is written in (the mockups' 1280 x 720 canvas).
    pub layout_pt: [f64; 2],
}

impl sc_core::data::Validate for Reference {
    fn validate(&self, c: &mut sc_core::data::Checks) {
        c.equals("schema", &self.schema, "starcrew.ui-reference/1");
        for (i, v) in self.reference_px.iter().enumerate() {
            c.number(&format!("reference_px[{i}]"), *v, 100.0, 10000.0);
        }
        for (i, v) in self.layout_pt.iter().enumerate() {
            c.number(&format!("layout_pt[{i}]"), *v, 100.0, 10000.0);
        }
        // A layout point is the same number of reference pixels across and down, or the frame would be stretched.
        let (ax, ay) = (self.reference_px[0] / self.layout_pt[0], self.reference_px[1] / self.layout_pt[1]);
        if (ax - ay).abs() > 1e-9 * ax.abs().max(1.0) {
            c.number("layout_pt", -1.0, 0.0, 0.0);
        }
    }
}

/// The file the reference frame is read from.
pub const REFERENCE_FILE: &str = "data/ui/reference.json";

/// The reference frame, parsed once from the compiled-in `data/ui/reference.json`; a bad file stops the client at its
/// first frame with its path and field (CLAUDE.md 6.5).
pub fn reference() -> &'static Reference {
    static REF: std::sync::OnceLock<Reference> = std::sync::OnceLock::new();
    REF.get_or_init(|| {
        sc_core::data::parse(REFERENCE_FILE, include_str!("../../../data/ui/reference.json"))
            .unwrap_or_else(|e| panic!("{e}"))
    })
}

/// The reference frame's size in layout points (1280 x 720).
pub fn layout_size() -> Vec2 {
    let r = reference();
    Vec2::new(r.layout_pt[0] as f32, r.layout_pt[1] as f32)
}

/// The one mapping of the reference frame onto a screen (design 5): the frame, scaled by the smaller of the screen's
/// width and height over the frame's so it always fits whole, and centred, the extra space at the edges. Returns the
/// frame's origin on `screen` and its scale (`screen`'s units a layout point). Every screen places itself by it, in
/// pixels ([`Fit::of`]) or in points (a console, the briefing).
pub fn frame_in(screen: egui::Rect) -> (Pos2, f32) {
    let f = layout_size();
    let s = (screen.width() / f.x).min(screen.height() / f.y).max(f32::MIN_POSITIVE);
    (screen.min + (screen.size() - f * s) / 2.0, s)
}

/// Where a group of the UI hangs from (ui-art-direction design 3): a corner, the middle of an edge, or the centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    MidLeft,
    MidRight,
    Centre,
}

/// The one rule of the anchors (ui-art-direction design 3), beside the scale: a group `size_ref` reference pixels big,
/// laid out in reference pixels and scaled by the frame's scale about the point of `screen` it hangs from. Returns its
/// top left corner on `screen` and `screen`'s units a reference pixel. A corner group hangs from the screen's real
/// corner (on an ultrawide screen the objective card sits in the corner, not the centred frame's); a centred one stays
/// centred. Every screen around the consoles is placed by it.
pub fn anchored(screen: egui::Rect, a: Anchor, size_ref: Vec2) -> (Pos2, f32) {
    let r = reference();
    let (_, k) = frame_in(screen);
    let s = k * (r.layout_pt[0] / r.reference_px[0]) as f32;
    let size = size_ref * s;
    let (min, max) = (screen.min, screen.max - size);
    let mid = screen.center() - size / 2.0;
    let p = match a {
        Anchor::TopLeft => min,
        Anchor::TopRight => Pos2::new(max.x, min.y),
        Anchor::BottomLeft => Pos2::new(min.x, max.y),
        Anchor::BottomRight => max,
        Anchor::MidLeft => Pos2::new(min.x, mid.y),
        Anchor::MidRight => Pos2::new(max.x, mid.y),
        Anchor::Centre => mid,
    };
    (p, s)
}

/// The output laid out on the reference frame (design 5), made by [`frame_in`] from the output's pixels and the
/// window's size. The UI draws in layout points; this is the only place that knows the pixels, the pixel ratio or the
/// window's real size.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Fit {
    /// Output pixels a layout point.
    pub ppp: f32,
    /// The whole output in layout points: at least the frame's 1280 x 720, more on the side the aspect gives.
    pub points: Vec2,
    /// Layout points a window unit (the platform's mouse and finger coordinates).
    pub win_to_pt: f32,
}

impl Default for Fit {
    /// The reference frame drawn at one pixel a layout point, until a frame says otherwise.
    fn default() -> Self {
        let f = layout_size();
        Self::new(f.x as u32, f.y as u32, f.x)
    }
}

impl Fit {
    /// The fit for an output `width_px` x `height_px` pixels in a window `window_w` units wide (the window and its
    /// output are one shape: the output's pixels are the window's units times its pixel ratio).
    pub fn new(width_px: u32, height_px: u32, window_w: f32) -> Self {
        let px = egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(width_px.max(1) as f32, height_px.max(1) as f32));
        let (_, ppp) = frame_in(px);
        let points = px.size() / ppp;
        Self { ppp, points, win_to_pt: px.width() / window_w.max(1.0) / ppp }
    }

    /// The fit for a platform frame.
    pub fn of(f: &Frame) -> Self {
        Self::new(f.width, f.height, f.window_w)
    }

    /// The reference frame's rectangle on the screen, layout points.
    pub fn frame(&self) -> egui::Rect {
        let (o, s) = frame_in(egui::Rect::from_min_size(Pos2::ZERO, self.points));
        egui::Rect::from_min_size(o, layout_size() * s)
    }

    /// A window position (the mouse's, a finger's) in layout points: the mapping's inverse, so a click lands on
    /// what is drawn under it.
    pub fn to_points(self, x: f32, y: f32) -> Pos2 {
        Pos2::new(x * self.win_to_pt, y * self.win_to_pt)
    }

    /// A window position in the units of a canvas `size` units big drawn at `rect` (layout points): a repair game's
    /// pointer on its own 1280 x 720 canvas.
    pub fn to_canvas(self, x: f32, y: f32, rect: egui::Rect, size: Vec2) -> [f32; 2] {
        let p = self.to_points(x, y) - rect.min;
        [p.x * size.x / rect.width().max(f32::MIN_POSITIVE), p.y * size.y / rect.height().max(f32::MIN_POSITIVE)]
    }

    /// A rectangle in layout points as window coordinates (for a test placing the mouse on a control).
    pub fn to_window(self, r: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_max(
            (r.min.to_vec2() / self.win_to_pt).to_pos2(),
            (r.max.to_vec2() / self.win_to_pt).to_pos2(),
        )
    }
}

/// egui's memory slot for the controls marked this frame.
const MARKS: &str = "sc-client-ui-marks";

/// Mark a control the player presses, named `label`: where it is drawn is said whenever it moves, in window
/// coordinates (`sc-client: ui <label> at x y w h`), and a press is said (`sc-client: ui clicked <label>`). The browser's
/// release test clicks a control where the engine says it drew it and fails if the press does not land
/// (browser-page-fit design 3). Returns the response, so it wraps an `add`.
pub fn mark(label: &str, r: egui::Response) -> egui::Response {
    if r.clicked() {
        println!("sc-client: ui clicked {label}");
    }
    mark_rect(&r.ctx, label, r.rect);
    r
}

/// Mark a control drawn outside egui's widgets (a console's, by its hit area `rect` in points) as [`mark`] does: where
/// it is is said whenever it moves. Its press is said by whoever handles it.
pub fn mark_rect(ctx: &egui::Context, label: &str, rect: egui::Rect) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<(String, egui::Rect)>>(egui::Id::new(MARKS)).push((label.to_owned(), rect))
    });
}

/// Every text in a frame's shapes, nested ones too (for tests of what a screen shows).
#[cfg(test)]
pub fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn walk(s: &egui::Shape, out: &mut Vec<String>) {
        match s {
            egui::Shape::Text(t) => out.push(t.galley.text().to_owned()),
            egui::Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|c| walk(&c.shape, &mut out));
    out
}

/// The controls [`mark`]ed since the last call, layout points.
pub fn take_marks(ctx: &egui::Context) -> Vec<(String, egui::Rect)> {
    ctx.data_mut(|d| d.remove_temp::<Vec<(String, egui::Rect)>>(egui::Id::new(MARKS))).unwrap_or_default()
}

/// egui's own widgets (the name field, a slider, a list) in the style's colours, square, in the body size: the few
/// stock widgets the screens keep look like the rest (design 6). Sizes are the style's reference pixels as points.
pub fn apply_look(ctx: &egui::Context) {
    let l = crate::look::look();
    let c = l.c();
    let pt = 1.0 / crate::look::Look::ref_per_pt();
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = c.panel.0;
    v.window_fill = c.panel.0;
    v.window_stroke = egui::Stroke::new(1.0_f32, c.line.0);
    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.menu_corner_radius = egui::CornerRadius::ZERO;
    v.extreme_bg_color = c.panel.0;
    v.faint_bg_color = c.raised.0;
    // Glyph coverage as alpha, unbrightened: the consoles' text then weighs what the mockup's does in a browser.
    v.text_alpha_from_coverage = egui::epaint::AlphaFromCoverage::Linear;
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(c.accent.0.r(), c.accent.0.g(), c.accent.0.b(), 90);
    v.selection.stroke = egui::Stroke::new(1.0_f32, c.accent.0);
    v.text_cursor.stroke = egui::Stroke::new(2.0_f32, c.accent.0);
    for (w, fill, edge, text) in [
        (&mut v.widgets.noninteractive, c.panel.0, c.line.0, c.dim.0),
        (&mut v.widgets.inactive, c.panel.0, c.line2.0, c.text.0),
        (&mut v.widgets.hovered, c.raised.0, c.line2.0, c.text.0),
        (&mut v.widgets.active, c.raised.0, c.accent.0, c.text.0),
        (&mut v.widgets.open, c.raised.0, c.line2.0, c.text.0),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = egui::Stroke::new(1.0_f32, edge);
        w.fg_stroke.color = text;
        w.corner_radius = egui::CornerRadius::ZERO;
        w.expansion = 0.0;
    }
    let grid = l.f.grid_px * pt;
    style.spacing.item_spacing = Vec2::splat(grid);
    style.spacing.button_padding = Vec2::new(2.0 * grid, grid);
    let body = l.f.type_.body.px * pt;
    for (ts, size) in [
        (egui::TextStyle::Body, body),
        (egui::TextStyle::Button, body),
        (egui::TextStyle::Heading, l.f.type_.title.px * pt),
        (egui::TextStyle::Small, l.f.type_.label.px * pt),
        (egui::TextStyle::Monospace, body),
    ] {
        if let Some(f) = style.text_styles.get_mut(&ts) {
            f.size = size;
        }
    }
    ctx.set_style(style);
}

fn texture_key(id: egui::TextureId) -> u64 {
    match id {
        egui::TextureId::Managed(n) => n,
        egui::TextureId::User(n) => n | (1 << 63),
    }
}

/// egui's key for an SDL keycode, where it has one (the repair games read egui's keys).
pub fn key_of(code: u32) -> Option<egui::Key> {
    use egui::Key;
    Some(match code {
        keys::RETURN => Key::Enter,
        keys::TAB => Key::Tab,
        keys::BACKSPACE => Key::Backspace,
        keys::DELETE => Key::Delete,
        keys::LEFT => Key::ArrowLeft,
        keys::RIGHT => Key::ArrowRight,
        keys::UP => Key::ArrowUp,
        keys::DOWN => Key::ArrowDown,
        keys::HOME => Key::Home,
        keys::END => Key::End,
        keys::ESCAPE => Key::Escape,
        keys::SPACE => Key::Space,
        keys::F1 => Key::F1,
        // SDL's keycodes for letters and digits are their lowercase characters.
        c @ 0x30..=0x39 | c @ 0x61..=0x7a => {
            return char::from_u32(c).and_then(|ch| Key::from_name(&ch.to_ascii_uppercase().to_string()))
        }
        _ => return None,
    })
}

impl Ui {
    /// A context with the game's look (ui-art-direction design 4 to 6), from `data/ui/ui_style.json`: the consoles'
    /// typeface, opaque square panels with one hairline, the one accent for focus and selection.
    pub fn new() -> Self {
        let ctx = egui::Context::default();
        apply_look(&ctx);
        crate::vg::install_fonts(&ctx);
        Self {
            ctx,
            events: Vec::new(),
            pointer: Pos2::ZERO,
            textures: HashMap::new(),
            text_on: false,
            fit: Fit::default(),
            marks: HashMap::new(),
            touch: platform::TouchMouse::default(),
        }
    }

    fn modifiers() -> egui::Modifiers {
        let (shift, ctrl, alt) = platform::modifiers();
        egui::Modifiers { alt, ctrl, shift, mac_cmd: false, command: ctrl }
    }

    /// The output on the reference frame, from the last frame: where the frame is, and the mouse and fingers in its
    /// points.
    pub fn fit(&self) -> Fit {
        self.fit
    }

    /// The UI's textures as uploaded, bytes (the font atlas, the key art): ui-art-direction task 4.1's measure.
    pub fn texture_bytes(&self) -> usize {
        self.textures.values().map(|(_, _, px)| px.len()).sum()
    }

    /// Take an input event (the app calls it with every event while the UI has the input). A finger is the pointer.
    pub fn event(&mut self, e: &Event) {
        if matches!(e, Event::Touch { .. }) {
            for m in self.touch.convert(e) {
                self.event(&m);
            }
            return;
        }
        let m = Self::modifiers();
        match e {
            Event::Pointer(x, y) => {
                self.pointer = self.fit.to_points(*x, *y);
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            Event::MouseButton(b, down) => {
                let button = match b {
                    1 => egui::PointerButton::Primary,
                    2 => egui::PointerButton::Middle,
                    3 => egui::PointerButton::Secondary,
                    _ => return,
                };
                self.events.push(egui::Event::PointerButton {
                    pos: self.pointer,
                    button,
                    pressed: *down,
                    modifiers: m,
                });
            }
            Event::Wheel(x, y) => self.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(*x, *y),
                modifiers: m,
            }),
            Event::Text(t) => self.events.push(egui::Event::Text(t.clone())),
            Event::KeyDown(k) | Event::KeyRepeat(k) | Event::KeyUp(k) => {
                if let Some(key) = key_of(*k) {
                    let pressed = !matches!(e, Event::KeyUp(_));
                    let repeat = matches!(e, Event::KeyRepeat(_));
                    self.events.push(egui::Event::Key { key, physical_key: None, pressed, repeat, modifiers: m });
                }
            }
            _ => {}
        }
    }

    /// Run the app's UI for `frame` (`time_s` the session's clock) and tessellate it; textures that changed are
    /// uploaded to `r`.
    pub fn run(&mut self, r: &mut Renderer, frame: &Frame, time_s: f64, ui: impl FnMut(&egui::Context)) -> UiFrame {
        self.fit = Fit::of(frame);
        let (ppp, points) = (self.fit.ppp, self.fit.points);
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, points)),
            time: Some(time_s),
            modifiers: Self::modifiers(),
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        if let Some(vp) = raw.viewports.get_mut(&raw.viewport_id) {
            vp.native_pixels_per_point = Some(ppp);
        }
        let out = self.ctx.run(raw, ui);
        for (label, r) in take_marks(&self.ctx) {
            let w = self.fit.to_window(r);
            let at = [w.min.x, w.min.y, w.width(), w.height()].map(|v| v.round() as i32);
            if self.marks.get(&label) != Some(&at) {
                println!("sc-client: ui {label} at {} {} {} {}", at[0], at[1], at[2], at[3]);
                self.marks.insert(label, at);
            }
        }
        for (id, delta) in &out.textures_delta.set {
            let key = texture_key(*id);
            let egui::ImageData::Color(img) = &delta.image;
            let [w, h] = img.size;
            let px: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            match delta.pos {
                None => {
                    self.textures.insert(key, (w, h, px));
                }
                Some([x0, y0]) => {
                    if let Some((tw, _, data)) = self.textures.get_mut(&key) {
                        for y in 0..h {
                            let dst = ((y0 + y) * *tw + x0) * 4;
                            data[dst..dst + w * 4].copy_from_slice(&px[y * w * 4..(y + 1) * w * 4]);
                        }
                    }
                }
            }
            if let Some((tw, th, data)) = self.textures.get(&key) {
                r.ui_texture(key, *tw as u32, *th as u32, data);
            }
        }
        for id in &out.textures_delta.free {
            let key = texture_key(*id);
            self.textures.remove(&key);
            r.ui_free_texture(key);
        }
        // Text input follows a text field's focus (the platform shows a keyboard where it has one).
        let wants_text = self.ctx.wants_keyboard_input();
        if wants_text != self.text_on {
            platform::text_input(wants_text);
            self.text_on = wants_text;
        }
        let prims = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        let mut meshes = Vec::new();
        for p in prims {
            let egui::epaint::Primitive::Mesh(m) = p.primitive else { continue };
            let c = p.clip_rect;
            let clip = [
                (c.min.x * ppp).floor() as i32,
                (c.min.y * ppp).floor() as i32,
                ((c.max.x - c.min.x) * ppp).ceil() as i32,
                ((c.max.y - c.min.y) * ppp).ceil() as i32,
            ];
            let v = m
                .vertices
                .iter()
                .map(|v| UiVertex { pos: [v.pos.x, v.pos.y], uv: [v.uv.x, v.uv.y], color: v.color.to_array() })
                .collect();
            meshes.push((clip, texture_key(m.texture_id), v, m.indices));
        }
        UiFrame { meshes, points: [points.x, points.y], ppp }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The outputs every screen must fit (browser-page-fit design 5).
    const OUTPUTS: [(u32, u32); 8] =
        [(1920, 1080), (1280, 720), (1366, 768), (2560, 1440), (1024, 768), (2560, 1080), (985, 455), (844, 390)];

    #[test]
    fn the_reference_frame_file_is_valid_and_one_shape() {
        let r = reference();
        assert_eq!(r.reference_px, [1920.0, 1080.0], "the owner's reference is 1920 x 1080");
        assert_eq!(r.layout_pt, [1280.0, 720.0], "the layouts' canvas, the mockups' and the parity fixtures'");
        let bad = r#"{"schema": "starcrew.ui-reference/1", "reference_px": [1920, 1080], "layout_pt": [1280, 800]}"#;
        assert!(sc_core::data::parse::<Reference>(REFERENCE_FILE, bad).is_err(), "a stretched frame is refused");
    }

    #[test]
    fn the_whole_frame_fits_every_output_centred() {
        for (w, h) in OUTPUTS {
            let fit = Fit::new(w, h, w as f32);
            let frame = fit.frame();
            let screen = egui::Rect::from_min_size(Pos2::ZERO, fit.points);
            assert!(screen.expand(1e-3).contains_rect(frame), "the frame {frame:?} leaves the screen at {w} x {h}");
            // It touches two opposite edges (nothing is wasted) and is centred between the other two.
            let touches_x = frame.min.x.abs() < 1e-3 && (frame.max.x - screen.max.x).abs() < 1e-3;
            let touches_y = frame.min.y.abs() < 1e-3 && (frame.max.y - screen.max.y).abs() < 1e-3;
            assert!(touches_x || touches_y, "the frame fills one way at {w} x {h}");
            assert!((frame.center() - screen.center()).length() < 1e-3, "the frame is centred at {w} x {h}");
            // The scale is the owner's: min(width / 1920, height / 1080) reference pixels a pixel, 1.5 a point.
            let k = (w as f32 / 1920.0).min(h as f32 / 1080.0);
            assert!((fit.ppp - 1.5 * k).abs() < 1e-4, "{} pixels a point at {w} x {h}, not {}", fit.ppp, 1.5 * k);
        }
    }

    #[test]
    fn a_click_lands_where_it_was_drawn_at_any_pixel_ratio() {
        // The bug on itch.io: the drawing and the mouse used different sizes. Here a control drawn at a rectangle of
        // points is found again from the window position at its centre, at a pixel ratio of 1, 2 and 3.
        let control = egui::Rect::from_min_size(Pos2::new(900.0, 640.0), Vec2::new(140.0, 44.0));
        for (w, h) in OUTPUTS {
            for ratio in [1.0f32, 2.0, 3.0] {
                let (pw, ph) = ((w as f32 * ratio).round() as u32, (h as f32 * ratio).round() as u32);
                let fit = Fit::new(pw, ph, w as f32);
                let at = fit.to_window(control).center();
                let back = fit.to_points(at.x, at.y);
                assert!((back - control.center()).length() < 0.01, "missed by {:?} at {w} x {h} x {ratio}", back);
            }
        }
    }

    #[test]
    fn no_screen_reads_the_output_size() {
        // Design 5: nothing in a screen's code knows the device's pixels, its pixel ratio or the window's real size;
        // they go through `Fit` and `frame_in`, here. The renderer's present calls and captures take the output's
        // pixels as they are (no `as f32`), and the 3D camera's aspect is not layout.
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        let mut files = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).expect("read src") {
                let p = e.expect("entry").path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    files.push(p);
                }
            }
        }
        let banned = [
            "window_w",
            "window_h",
            "devicePixelRatio",
            "f.width as f32",
            "f.height as f32",
            "frame.width as f32",
            "frame.height as f32",
            "/ 720.0).max(",
            "/ 1280.0).min(",
            "/ W).min(",
        ];
        let mut bad = Vec::new();
        for p in files {
            let name = p.strip_prefix(dir).expect("in src").to_string_lossy().into_owned();
            if name == "ui.rs" || name == "platform.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&p).expect("read");
            for (n, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains("let aspect =") {
                    continue;
                }
                for b in banned {
                    if code.contains(b) {
                        bad.push(format!("{name}:{}: {b}", n + 1));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "screens reading the output's size outside ui::Fit: {bad:?}");
    }
}
