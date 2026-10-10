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
    /// Window coordinates to points, from the last frame (the UI is laid out on a screen 720 points high).
    win_to_pt: f32,
}

/// The UI is laid out on a screen this many points high, whatever the output's size, so it reads the same at 720p and
/// 1080p (and on the Pi's 1280 x 720).
const UI_HEIGHT_PT: f32 = 720.0;

fn texture_key(id: egui::TextureId) -> u64 {
    match id {
        egui::TextureId::Managed(n) => n,
        egui::TextureId::User(n) => n | (1 << 63),
    }
}

fn key_of(code: u32) -> Option<egui::Key> {
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
        // SDL's keycodes for letters and digits are their lowercase characters.
        c @ 0x30..=0x39 | c @ 0x61..=0x7a => {
            return char::from_u32(c).and_then(|ch| Key::from_name(&ch.to_ascii_uppercase().to_string()))
        }
        _ => return None,
    })
}

impl Ui {
    /// A context with the game's look: dark panels, rounded, the bridge's amber and lilac (consoles' palette).
    pub fn new() -> Self {
        let ctx = egui::Context::default();
        let mut style = (*ctx.style()).clone();
        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgba_premultiplied(10, 14, 22, 235);
        v.window_fill = v.panel_fill;
        v.extreme_bg_color = Color32::from_rgb(4, 6, 10);
        v.selection.bg_fill = Color32::from_rgb(204, 120, 40);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 46, 66);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(70, 82, 120);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(230, 150, 70);
        v.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(200, 210, 225);
        v.widgets.inactive.fg_stroke.color = Color32::from_rgb(225, 232, 245);
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        for (ts, size) in [
            (egui::TextStyle::Body, 18.0),
            (egui::TextStyle::Button, 18.0),
            (egui::TextStyle::Heading, 30.0),
            (egui::TextStyle::Small, 13.0),
            (egui::TextStyle::Monospace, 17.0),
        ] {
            if let Some(f) = style.text_styles.get_mut(&ts) {
                f.size = size;
            }
        }
        ctx.set_style(style);
        Self { ctx, events: Vec::new(), pointer: Pos2::ZERO, textures: HashMap::new(), text_on: false, win_to_pt: 1.0 }
    }

    fn modifiers() -> egui::Modifiers {
        let (shift, ctrl, alt) = platform::modifiers();
        egui::Modifiers { alt, ctrl, shift, mac_cmd: false, command: ctrl }
    }

    /// Take an input event (the app calls it with every event while the UI has the input).
    pub fn event(&mut self, e: &Event) {
        let m = Self::modifiers();
        match e {
            Event::Pointer(x, y) => {
                self.pointer = Pos2::new(*x * self.win_to_pt, *y * self.win_to_pt);
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
        let ppp = (frame.height as f32 / UI_HEIGHT_PT).max(0.5);
        self.win_to_pt = frame.width as f32 / frame.window_w / ppp;
        let points = Vec2::new(frame.width as f32 / ppp, frame.height as f32 / ppp);
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
        UiFrame { meshes, points: [points.x, points.y] }
    }
}
