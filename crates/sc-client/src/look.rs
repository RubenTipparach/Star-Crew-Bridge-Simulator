//! The look of every screen around the consoles (openspec/changes/ui-art-direction, design 3 to 9): the style read
//! from `data/ui/ui_style.json` and the few things every screen draws with it, a panel, a menu item, a keycap, text
//! in one of the four type roles.
//!
//! The style is the approved mockup's (`docs/mockups/ui-art-direction.html`), written into data by
//! `tools/ui/ui_style.py`, so no colour or size is typed here (design 9). Every screen is laid out in reference pixels
//! on the 1920 x 1080 frame and drawn through a [`Kit`]: a `vg::Canvas` whose unit is one reference pixel, hung from
//! an anchor of the screen by `ui::anchored` (design 3). Text never draws under the owner's 12 px floor (Q8). It lives
//! in the client because egui is the client's UI; the consoles keep their own look (`vg::style`, console-parity).

use crate::motion::Ease;
use crate::vg::{self, Canvas, IconPart, T};
use egui::{Color32, FontFamily, FontId, Pos2, Rect, Vec2};
use sc_core::data::{Checks, Validate};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The file the style is read from.
pub const STYLE_FILE: &str = "data/ui/ui_style.json";

/// One type role (design 4): its size in reference pixels, weight, line height (in ems), tracking and case.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TypeRole {
    /// Size, reference pixels.
    pub px: f32,
    /// 600 or 700.
    pub weight: u16,
    /// Line height, ems.
    pub line: f32,
    /// Letter spacing, ems.
    pub tracking_em: f32,
    /// Set in capitals.
    pub caps: bool,
}

/// The four type roles.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TypeRoles {
    /// A mission's name, a count that matters.
    pub display: TypeRole,
    /// Menu items, screen titles, the primary button.
    pub title: TypeRole,
    /// Everything read.
    pub body: TypeRole,
    /// Section labels, keycap letters, small states.
    pub label: TypeRole,
}

/// Keycaps, menu items and buttons, reference pixels (design 6 and 7).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sizes {
    pub key_h: f32,
    pub key_min_w: f32,
    pub key_pad: f32,
    pub key_edge: f32,
    pub key_radius: f32,
    pub key_gap: f32,
    /// HOLD before a held key.
    pub hold_px: f32,
    pub item_h: f32,
    pub item_pad: f32,
    /// The accent bar on a focused item.
    pub bar: f32,
    pub primary_h: f32,
    pub secondary_h: f32,
}

/// `#rrggbb`, opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hex(pub Color32);

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let v = s
            .strip_prefix('#')
            .filter(|h| h.len() == 6)
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .ok_or_else(|| serde::de::Error::custom(format!("not a #rrggbb colour: {s}")))?;
        Ok(Hex(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)))
    }
}

/// The colour roles (design 5).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Colours {
    pub ground: Hex,
    pub panel: Hex,
    pub raised: Hex,
    pub line: Hex,
    pub line2: Hex,
    pub text: Hex,
    pub dim: Hex,
    pub faint: Hex,
    pub key: Hex,
    pub key_edge: Hex,
    pub accent: Hex,
    pub ink: Hex,
    pub ok: Hex,
    pub warn: Hex,
    pub danger: Hex,
}

/// Motion, milliseconds (design 8).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    pub focus: f32,
    pub panel_in: f32,
    pub screen_fade: f32,
    pub load_out: f32,
    pub prompt_in: f32,
    pub prompt_out: f32,
    pub briefing_lower: f32,
    pub console_rise: f32,
}

/// Each motion's curve (design 18a), by the same names as [`Motion`].
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MotionEase {
    pub focus: Ease,
    pub panel_in: Ease,
    pub screen_fade: Ease,
    pub load_out: Ease,
    pub prompt_in: Ease,
    pub prompt_out: Ease,
    pub briefing_lower: Ease,
    pub console_rise: Ease,
}

/// `data/ui/ui_style.json` (`starcrew.ui-style/1`).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleFile {
    pub schema: String,
    /// The safe area, reference pixels left and right, top and bottom.
    pub safe_px: [f32; 2],
    /// The layout grid, reference pixels.
    pub grid_px: f32,
    /// No text is drawn under this many output pixels (Q8).
    pub min_text_px: f32,
    #[serde(rename = "type")]
    pub type_: TypeRoles,
    pub sizes_px: Sizes,
    pub colours: Colours,
    /// The accent the owner did not choose (Q1): kept with the mockup's tokens, drawn nowhere.
    #[allow(dead_code)]
    pub accent_alt: Hex,
    /// The stations' colours, for their icons only.
    pub roles: BTreeMap<String, Hex>,
    pub motion_ms: Motion,
    pub motion_ease: MotionEase,
    /// The glyphs beyond the consoles' icons: SVG fragments on a 24-unit grid.
    pub glyphs: BTreeMap<String, String>,
}

impl Validate for StyleFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ui-style/1");
        for (i, v) in self.safe_px.iter().enumerate() {
            c.number(&format!("safe_px[{i}]"), f64::from(*v), 0.0, 400.0);
        }
        c.number("grid_px", f64::from(self.grid_px), 1.0, 64.0);
        c.number("min_text_px", f64::from(self.min_text_px), 6.0, 40.0);
        let t = &self.type_;
        for (k, r) in [("display", &t.display), ("title", &t.title), ("body", &t.body), ("label", &t.label)] {
            c.number(&format!("type.{k}.px"), f64::from(r.px), 8.0, 200.0);
            c.number(&format!("type.{k}.line"), f64::from(r.line), 0.8, 2.0);
            c.number(&format!("type.{k}.tracking_em"), f64::from(r.tracking_em), 0.0, 1.0);
            if r.weight != 600 && r.weight != 700 {
                c.equals(&format!("type.{k}.weight"), &r.weight.to_string(), "600 or 700 (the two faces loaded)");
            }
        }
        let s = &self.sizes_px;
        for (k, v) in [
            ("key_h", s.key_h),
            ("key_min_w", s.key_min_w),
            ("key_pad", s.key_pad),
            ("key_edge", s.key_edge),
            ("key_radius", s.key_radius),
            ("key_gap", s.key_gap),
            ("hold_px", s.hold_px),
            ("item_h", s.item_h),
            ("item_pad", s.item_pad),
            ("bar", s.bar),
            ("primary_h", s.primary_h),
            ("secondary_h", s.secondary_h),
        ] {
            c.number(&format!("sizes_px.{k}"), f64::from(v), 0.0, 400.0);
        }
        let m = &self.motion_ms;
        for (k, v) in [
            ("focus", m.focus),
            ("panel_in", m.panel_in),
            ("screen_fade", m.screen_fade),
            ("load_out", m.load_out),
            ("prompt_in", m.prompt_in),
            ("prompt_out", m.prompt_out),
            ("briefing_lower", m.briefing_lower),
            ("console_rise", m.console_rise),
        ] {
            // Short motion only (design 8): nothing a player waits on.
            c.number(&format!("motion_ms.{k}"), f64::from(v), 1.0, 1000.0);
        }
        for need in ["command", "helm", "tactical", "engineering", "science", "laser", "gunner"] {
            if !self.roles.contains_key(need) {
                c.equals(&format!("roles.{need}"), "missing", "a colour for every station built");
            }
        }
        for need in ["mouse", "click", "drag", "wheel", "dice", "any", "diamond", "marker", "chevron"] {
            match self.glyphs.get(need) {
                Some(g) if !vg::parse_icon(g).is_empty() => {}
                _ => c.equals(&format!("glyphs.{need}"), "missing or empty", "an SVG fragment the screens draw"),
            }
        }
    }
}

/// The style, ready to draw with: the file and its glyphs flattened.
pub struct Look {
    /// What the file says.
    pub f: StyleFile,
    glyphs: BTreeMap<String, Vec<IconPart>>,
}

impl Look {
    /// A colour role by name.
    pub fn c(&self) -> &Colours {
        &self.f.colours
    }

    /// A station's colour by its role id ("helm", "command"...), the dim colour for one it does not know.
    pub fn role(&self, id: &str) -> Color32 {
        self.f.roles.get(id).map_or(self.f.colours.dim.0, |h| h.0)
    }

    /// A glyph's flattened parts, empty for one the style does not have.
    pub fn glyph(&self, name: &str) -> &[IconPart] {
        self.glyphs.get(name).map_or(&[], Vec::as_slice)
    }

    /// Reference pixels a layout point (1.5 on the 1920 x 1080 frame laid on 1280 x 720 points).
    pub fn ref_per_pt() -> f32 {
        let r = crate::ui::reference();
        (r.reference_px[0] / r.layout_pt[0]) as f32
    }
}

fn parse(text: &str) -> Result<Look, String> {
    let f: StyleFile = sc_core::data::parse(STYLE_FILE, text).map_err(|e| e.to_string())?;
    let glyphs = f.glyphs.iter().map(|(k, v)| (k.clone(), vg::parse_icon(v))).collect();
    Ok(Look { f, glyphs })
}

/// The style, parsed once from the compiled-in `data/ui/ui_style.json`; a bad file stops the client at its first
/// frame with its path and field (CLAUDE.md 6.5).
pub fn look() -> &'static Look {
    static LOOK: OnceLock<Look> = OnceLock::new();
    LOOK.get_or_init(|| parse(include_str!("../../../data/ui/ui_style.json")).unwrap_or_else(|e| panic!("{e}")))
}

/// The four type roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Display,
    Title,
    Body,
    Label,
}

impl Role {
    fn of(self) -> &'static TypeRole {
        let t = &look().f.type_;
        match self {
            Role::Display => &t.display,
            Role::Title => &t.title,
            Role::Body => &t.body,
            Role::Label => &t.label,
        }
    }
}

/// Where a text sits on its x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Middle,
    End,
}

/// One thing in a row of keys: a keycap with its name, a keycap with a mouse glyph, HOLD before it, or "/".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTok {
    /// A key by its name ("E", "Space", "Esc").
    Key(String),
    /// A keycap holding a glyph ("mouse", "click", "drag", "wheel").
    Glyph(String),
    /// HOLD, before a key that is held.
    Hold,
    /// Either of two.
    Or,
}

/// Barlow's em box: ascent and descent over the em (hhea 1000 and 200 of 1000), for CSS's line box.
const ASCENT: f32 = 1.0;
const EM_BOX: f32 = 1.2;

/// A painter whose unit is one reference pixel, with the output's pixels a point for the text floor.
#[derive(Clone)]
pub struct Kit {
    /// The canvas: user units are reference pixels.
    pub cv: Canvas,
    /// Output pixels an egui point.
    ppp: f32,
    /// A size that replaces every role's own (a title set at the body's size, as the Options rail is), reference px.
    px: Option<f32>,
    /// A tracking that replaces every role's own, ems (the station cards' 0.1 em).
    track: Option<f32>,
}

impl Kit {
    /// A kit drawing on `painter` with its origin at `o` (points) and `s` points a reference pixel.
    pub fn new(painter: egui::Painter, o: Pos2, s: f32) -> Self {
        let ppp = painter.ctx().pixels_per_point();
        Kit { cv: Canvas::new(painter, o.to_vec2(), s), ppp, px: None, track: None }
    }

    /// The same kit with its origin moved by (x, y) reference pixels.
    pub fn at(&self, x: f32, y: f32) -> Kit {
        Kit { cv: self.cv.at(x, y), ..self.clone() }
    }

    /// The same kit setting every role's text at `px` reference pixels (the mockup's few rows set off its four sizes).
    pub fn px(&self, px: f32) -> Kit {
        Kit { px: Some(px), ..self.clone() }
    }

    /// The same kit setting every role's text with `em` letter spacing.
    pub fn track_em(&self, em: f32) -> Kit {
        Kit { track: Some(em), ..self.clone() }
    }

    /// The same kit drawing at opacity `a` (a prompt fading in and out).
    pub fn faded(&self, a: f32) -> Kit {
        let mut k = self.clone();
        k.cv.painter.multiply_opacity(a.clamp(0.0, 1.0));
        k
    }

    /// A rectangle in reference pixels as egui points.
    pub fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        self.cv.r(x, y, w, h)
    }

    /// A role's size here, reference pixels: its own, or larger where that would draw it under the floor (Q8).
    pub fn size(&self, role: Role) -> f32 {
        self.size_px(self.px.unwrap_or(role.of().px))
    }

    /// `px` reference pixels, or the floor's.
    pub fn size_px(&self, px: f32) -> f32 {
        let floor = look().f.min_text_px / (self.cv.s * self.ppp).max(f32::MIN_POSITIVE);
        px.max(floor)
    }

    fn style(&self, role: Role, s: &str) -> (String, f32, T) {
        let r = role.of();
        let size = self.size(role);
        let t = T { w: r.weight, ls: self.track.unwrap_or(r.tracking_em) * size, ..T::default() };
        let s = if r.caps { s.to_uppercase() } else { s.to_owned() };
        (s, size, t)
    }

    /// A text's width in reference pixels.
    pub fn width(&self, role: Role, s: &str) -> f32 {
        let (s, size, t) = self.style(role, s);
        let family = FontFamily::Name(if t.w >= 700 { vg::BOLD } else { vg::SEMIBOLD }.into());
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &s,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(size * self.cv.s, family),
                extra_letter_spacing: t.ls * self.cv.s,
                ..Default::default()
            },
        );
        (self.cv.painter.layout_job(job).size().x + t.ls * self.cv.s) / self.cv.s
    }

    /// A role's line box, reference pixels.
    pub fn line(&self, role: Role) -> f32 {
        self.size(role) * role.of().line
    }

    /// Text in `role` whose line box's top is at `top`, set at `x` by `align`; returns its width.
    pub fn text(&self, role: Role, x: f32, top: f32, col: Color32, s: &str, align: Align) -> f32 {
        let (s, size, t) = self.style(role, s);
        let base = top + (role.of().line - EM_BOX) * size / 2.0 + ASCENT * size;
        let t = match align {
            Align::Start => t,
            Align::Middle => T { a: vg::Anchor::Middle, ..t },
            Align::End => T { a: vg::Anchor::End, ..t },
        };
        self.cv.text(x, base, size, col, &s, t)
    }

    /// Text whose line box is centred on `cy`.
    pub fn text_mid(&self, role: Role, x: f32, cy: f32, col: Color32, s: &str, align: Align) -> f32 {
        self.text(role, x, cy - self.line(role) / 2.0, col, s, align)
    }

    /// Text cut with an ellipsis to `max_w` reference pixels (CLAUDE.md 10: content never grows its band).
    pub fn text_clip(&self, role: Role, x: f32, top: f32, col: Color32, s: &str, max_w: f32) -> f32 {
        if self.width(role, s) <= max_w {
            return self.text(role, x, top, col, s, Align::Start);
        }
        let mut cut: Vec<char> = s.chars().collect();
        while !cut.is_empty() {
            cut.pop();
            let t = format!("{}\u{2026}", cut.iter().collect::<String>().trim_end());
            if self.width(role, &t) <= max_w {
                return self.text(role, x, top, col, &t, Align::Start);
            }
        }
        0.0
    }

    /// A paragraph in `role` wrapped to `w` reference pixels from `top`; returns its height.
    pub fn para(&self, role: Role, x: f32, top: f32, w: f32, col: Color32, s: &str) -> f32 {
        let (s, size, t) = self.style(role, s);
        let family = FontFamily::Name(if t.w >= 700 { vg::BOLD } else { vg::SEMIBOLD }.into());
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = w * self.cv.s;
        let line = size * role.of().line;
        job.append(
            &s,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(size * self.cv.s, family),
                color: col,
                extra_letter_spacing: t.ls * self.cv.s,
                line_height: Some(line * self.cv.s),
                ..Default::default()
            },
        );
        let g = self.cv.painter.layout_job(job);
        let h = g.size().y / self.cv.s;
        self.cv.painter.galley(self.cv.p(x, top), g, col);
        h
    }

    /// The game's wordmark (design 4: art, not type): `px` reference pixels, bold, lines 0.9 apart, 0.06 em tracking,
    /// the top of its first line box at `top`; one picture, so its lines are one text.
    pub fn wordmark(&self, x: f32, top: f32, px: f32, s: &str) {
        let px = self.size_px(px);
        let mut job = egui::text::LayoutJob::default();
        job.append(
            s,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(px * self.cv.s, FontFamily::Name(vg::BOLD.into())),
                color: look().c().text.0,
                extra_letter_spacing: 0.06 * px * self.cv.s,
                line_height: Some(0.9 * px * self.cv.s),
                ..Default::default()
            },
        );
        let g = self.cv.painter.layout_job(job);
        // egui sets the first baseline an ascent below the top; CSS centres the em box in the shorter line box.
        self.cv.painter.galley(self.cv.p(x, top + (0.9 - 1.2) / 2.0 * px), g, look().c().text.0);
    }

    /// An opaque panel with its hairline (design 6).
    pub fn panel(&self, x: f32, y: f32, w: f32, h: f32) {
        let c = look().c();
        self.cv.rect(x, y, w, h, 0.0, c.panel.0, Some(vg::Pen::new(1.0, c.line.0)));
    }

    /// A fill, square.
    pub fn fill(&self, x: f32, y: f32, w: f32, h: f32, col: Color32) {
        self.cv.rect(x, y, w, h, 0.0, col, None);
    }

    /// A box's edge, square, 1 px.
    pub fn edge(&self, x: f32, y: f32, w: f32, h: f32, col: Color32) {
        self.cv.rect(x, y, w, h, 0.0, Color32::TRANSPARENT, Some(vg::Pen::new(1.0, col)));
    }

    /// The accent bar down a focused item's left edge.
    pub fn bar(&self, x: f32, y: f32, h: f32) {
        self.fill(x, y, look().f.sizes_px.bar, h, look().c().accent.0);
    }

    /// The focus slab (design 6 and 18a): the raised fill and the accent bar of the focused item, one item high, its
    /// top sliding to `y` over `motion_ms.focus` on its curve from wherever it was drawn last (`slide` keeps that).
    /// Drawn before the items, which only change colour.
    pub fn focus_slab(&self, slide: &mut crate::motion::Slide, x: f32, y: f32, w: f32, now: f64) {
        let l = look();
        let (sz, c) = (&l.f.sizes_px, l.c());
        let y = slide.at(y, now, l.f.motion_ms.focus, l.f.motion_ease.focus);
        self.fill(x, y, w, sz.item_h, c.raised.0);
        self.bar(x, y, sz.item_h);
    }

    /// A menu item (design 6): `role` text in `dim`, or `text` when focused (its slab is [`Kit::focus_slab`]'s).
    pub fn item(&self, x: f32, y: f32, word: &str, focus: bool, role: Role) {
        let (sz, c) = (&look().f.sizes_px, look().c());
        let col = if focus { c.text.0 } else { c.dim.0 };
        self.text_mid(role, x + sz.item_pad, y + sz.item_h / 2.0, col, word, Align::Start);
    }

    /// A stroked icon (the consoles' set) centred on (x, y).
    pub fn icon(&self, name: &str, x: f32, y: f32, size: f32, col: Color32) {
        self.cv.icon(name, x, y, size, col);
    }

    /// A glyph of the UI's own set (`glyphs` in the style) centred on (x, y).
    pub fn glyph(&self, name: &str, x: f32, y: f32, size: f32, col: Color32) {
        self.cv.glyph(look().glyph(name), x, y, size, col, look().c().accent.0, 2.0);
    }

    /// A station's role icon, the consoles' own, or the UI's where the consoles have none (Any's shuffle).
    pub fn role_icon(&self, name: &str, x: f32, y: f32, size: f32, col: Color32) {
        if look().glyphs.contains_key(name) {
            self.glyph(name, x, y, size, col);
        } else {
            self.icon(name, x, y, size, col);
        }
    }

    /// One keycap's width for `t`, reference pixels (0 for HOLD and "/", which are words).
    fn cap_w(&self, t: &KeyTok) -> f32 {
        let s = &look().f.sizes_px;
        match t {
            KeyTok::Key(k) => (self.cap_text_w(k) + 2.0 * s.key_pad).max(s.key_min_w),
            KeyTok::Glyph(_) => s.key_min_w.max(28.0 + 2.0 * 8.0),
            KeyTok::Hold => self.hold_w(),
            KeyTok::Or => self.width(Role::Label, "/") + 4.0,
        }
    }

    /// A keycap's name: the label's size and weight, as written (not in capitals), 0.04 em apart (the mockup's `.key`).
    fn cap_t(&self) -> (f32, T) {
        let px = self.size_px(look().f.type_.label.px);
        (px, T { w: 700, ls: 0.04 * px, a: vg::Anchor::Middle, ..T::default() })
    }

    fn cap_text_w(&self, k: &str) -> f32 {
        let (px, t) = self.cap_t();
        let family = FontFamily::Name(vg::BOLD.into());
        let mut job = egui::text::LayoutJob::default();
        job.append(
            k,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(px * self.cv.s, family),
                extra_letter_spacing: t.ls * self.cv.s,
                ..Default::default()
            },
        );
        self.cv.painter.layout_job(job).size().x / self.cv.s
    }

    fn hold_w(&self) -> f32 {
        let px = self.size_px(look().f.sizes_px.hold_px);
        let (s, t) = ("HOLD", T { w: 700, ls: 0.16 * px, ..T::default() });
        let family = FontFamily::Name(vg::BOLD.into());
        let mut job = egui::text::LayoutJob::default();
        job.append(
            s,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(px * self.cv.s, family),
                extra_letter_spacing: t.ls * self.cv.s,
                ..Default::default()
            },
        );
        self.cv.painter.layout_job(job).size().x / self.cv.s + 2.0
    }

    /// A row of keys' width, reference pixels.
    pub fn keys_w(&self, toks: &[KeyTok]) -> f32 {
        let gap = look().f.sizes_px.key_gap;
        toks.iter().map(|t| self.cap_w(t)).sum::<f32>() + gap * toks.len().saturating_sub(1) as f32
    }

    /// A row of keys whose left edge is at `x`, centred on `cy` (design 7): keycaps 44 high, a 4 px bottom edge.
    pub fn keys(&self, x: f32, cy: f32, toks: &[KeyTok]) -> f32 {
        let (s, c) = (&look().f.sizes_px, look().c());
        let mut at = x;
        for t in toks {
            let w = self.cap_w(t);
            match t {
                KeyTok::Key(_) | KeyTok::Glyph(_) => {
                    let (top, h) = (cy - s.key_h / 2.0, s.key_h);
                    self.cv.rect(at, top, w, h, s.key_radius, c.key_edge.0, None);
                    self.cv.rect(at + 1.0, top + 1.0, w - 2.0, h - 1.0 - s.key_edge, s.key_radius - 1.0, c.key.0, None);
                    let mid = top + (h - s.key_edge) / 2.0 + 0.5;
                    match t {
                        KeyTok::Key(k) => {
                            let (px, t) = self.cap_t();
                            self.cv.text(at + w / 2.0 + t.ls / 2.0, mid + 0.35 * px, px, c.text.0, k, t);
                        }
                        KeyTok::Glyph(g) => self.glyph(g, at + w / 2.0, mid, 28.0, c.text.0),
                        _ => {}
                    }
                }
                KeyTok::Hold => {
                    let px = self.size_px(s.hold_px);
                    let base = cy + 0.35 * px;
                    self.cv.text(at, base, px, c.dim.0, "HOLD", T { w: 700, ls: 0.16 * px, ..T::default() });
                }
                KeyTok::Or => {
                    self.text_mid(Role::Label, at + w / 2.0, cy, c.faint.0, "/", Align::Middle);
                }
            }
            at += w + s.key_gap;
        }
        at - x - s.key_gap
    }

    /// A row of keys ending at `right`.
    pub fn keys_right(&self, right: f32, cy: f32, toks: &[KeyTok]) -> f32 {
        let w = self.keys_w(toks);
        self.keys(right - w, cy, toks)
    }

    /// A thin bar: `k` of `w` filled with the accent over the line colour (loading, Launch in).
    pub fn progress(&self, x: f32, y: f32, w: f32, k: f32) {
        let c = look().c();
        self.fill(x, y, w, 6.0, c.line.0);
        self.fill(x, y, w * k.clamp(0.0, 1.0), 6.0, c.accent.0);
    }

    /// A horizontal gradient over a rectangle: `stops` are (offset 0-1, colour), left to right.
    pub fn hgrad(&self, x: f32, y: f32, w: f32, h: f32, stops: &[(f32, Color32)]) {
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        for (i, (t, col)) in stops.iter().enumerate() {
            let px = x + w * t;
            verts.push(([px, y], *col));
            verts.push(([px, y + h], *col));
            if i > 0 {
                let b = (2 * i) as u32;
                idx.extend([b - 2, b - 1, b, b - 1, b + 1, b]);
            }
        }
        self.cv.mesh(&verts, idx);
    }

    /// A vertical gradient (top to bottom).
    pub fn vgrad(&self, x: f32, y: f32, w: f32, h: f32, stops: &[(f32, Color32)]) {
        self.cv.vgrad(x, y, w, h, stops);
    }
}

/// A click area over `rect` (points) in `ui`, said to the browser's tests under `mark` (ui::mark): where it is drawn
/// and when it is pressed.
pub fn hit(ui: &mut egui::Ui, rect: Rect, mark: &str) -> egui::Response {
    let r = ui.interact(rect, egui::Id::new(("look-hit", mark)), egui::Sense::click());
    crate::ui::mark(mark, r)
}

/// A whole-screen layer to draw a screen on and take its clicks: `add` gets its `Ui`, whose rectangle is the screen.
pub fn screen_area<R>(
    ctx: &egui::Context,
    id: &str,
    order: egui::Order,
    add: impl FnOnce(&mut egui::Ui, Rect) -> R,
) -> R {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new(id))
        .fixed_pos(screen.min)
        .order(order)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            ui.expand_to_include_rect(screen);
            add(ui, screen)
        })
        .inner
}

/// A kit for a group `size_ref` reference pixels big hung from `anchor` of `screen`.
pub fn kit(painter: &egui::Painter, screen: Rect, anchor: crate::ui::Anchor, size_ref: Vec2) -> Kit {
    let (o, s) = crate::ui::anchored(screen, anchor, size_ref);
    Kit::new(painter.clone(), o, s)
}

/// A key art picture (design 11 and 15): the front menu's (the Tern in its berth) or the loading screen's (the
/// system's station), 1280 x 720, loaded while a menu or the loading screen is up and freed with its handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    Menu,
    Loading,
}

impl Art {
    /// Its file, from the repository's root (the browser build embeds `assets/ui`).
    pub fn file(self) -> &'static str {
        match self {
            Art::Menu => "assets/ui/keyart-menu.png",
            Art::Loading => "assets/ui/keyart-loading.png",
        }
    }
}

/// Load a key art picture as an egui texture; a picture that cannot be read is said and drawn as the ground colour
/// (it is art, not data the game needs).
pub fn load_art(ctx: &egui::Context, a: Art) -> Option<egui::TextureHandle> {
    let read = || -> Result<egui::ColorImage, String> {
        let f = std::fs::File::open(a.file()).map_err(|e| format!("{}: {e}", a.file()))?;
        let mut reader = png::Decoder::new(std::io::BufReader::new(f)).read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; reader.output_buffer_size().ok_or("too large")?];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width as usize, info.height as usize);
        let rgba: Vec<u8> = match info.color_type {
            png::ColorType::Rgb => buf[..w * h * 3].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
            png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
            t => return Err(format!("{}: an RGB PNG, not {t:?}", a.file())),
        };
        Ok(egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba))
    };
    match read() {
        Ok(img) => Some(ctx.load_texture(a.file(), img, egui::TextureOptions::LINEAR)),
        Err(e) => {
            eprintln!("sc-client: {e}");
            None
        }
    }
}

/// A picture covering `screen` (the larger of the two ratios: art is not UI, design 3), its middle `at_y` (0-1) of
/// the way down kept in view, at opacity `a`; `screen` is filled with the ground colour first.
pub fn cover_art(painter: &egui::Painter, screen: Rect, tex: Option<&egui::TextureHandle>, at_y: f32, a: f32) {
    painter.rect_filled(screen, 0.0, look().c().ground.0);
    let Some(t) = tex else { return };
    let size = t.size_vec2();
    let k = (screen.width() / size.x).max(screen.height() / size.y);
    let drawn = size * k;
    let x = screen.center().x - drawn.x / 2.0;
    let y = screen.min.y - (drawn.y - screen.height()) * at_y;
    let rect = Rect::from_min_size(Pos2::new(x, y), drawn);
    let tint = Color32::from_white_alpha((255.0 * a.clamp(0.0, 1.0)) as u8);
    painter.image(t.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
}

/// `col` at opacity `a` (0-1).
pub fn alpha(col: Color32, a: f32) -> Color32 {
    vg::alpha(col, a)
}

/// The ground colour at opacity `a`: the scrims and the art's darkening (design 6).
pub fn ground(a: f32) -> Color32 {
    alpha(look().c().ground.0, a)
}

/// A whole-screen fill (a scrim, a solid backdrop): not UI, so it covers the screen whatever its shape (design 3).
pub fn cover(painter: &egui::Painter, col: Color32) {
    painter.rect_filled(painter.clip_rect(), 0.0, col);
}

/// A key's name for a keycap from a binding's name ("Left Shift" is Shift, "Escape" is Esc).
pub fn cap_name(binding: &str) -> String {
    match binding {
        "Left Shift" | "Right Shift" => "Shift".into(),
        "Left Ctrl" => "Ctrl".into(),
        "Escape" => "Esc".into(),
        "Return" => "Enter".into(),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED: &str = include_str!("../../../data/ui/ui_style.json");

    #[test]
    fn the_style_file_is_valid_and_holds_the_mockups_four_sizes() {
        let l = look();
        let t = &l.f.type_;
        assert_eq!([t.display.px, t.title.px, t.body.px, t.label.px], [72.0, 40.0, 28.0, 20.0], "design 4");
        assert_eq!(l.c().accent.0, Color32::from_rgb(0x6f, 0xd3, 0xff), "Q1: the ice accent");
        assert_eq!(l.f.min_text_px, 12.0, "Q8: a 12 px floor");
        assert!(!l.glyph("click").is_empty(), "the mouse keycaps are drawn from data");
    }

    #[test]
    fn a_missing_field_a_bad_colour_and_an_unknown_key_are_refused_with_the_path() {
        let missing = SHIPPED.replacen("\"grid_px\": 8,", "", 1);
        assert!(parse(&missing).err().is_some_and(|e| e.contains("grid_px")), "a missing field");
        let bad = SHIPPED.replacen("\"#6fd3ff\"", "\"ice\"", 1);
        assert!(parse(&bad).err().is_some_and(|e| e.contains("accent")), "a bad colour names its field");
        let extra = SHIPPED.replacen('{', "{\"blur\": 4,", 1);
        assert!(parse(&extra).is_err(), "an unknown key");
    }

    #[test]
    fn text_never_draws_under_twelve_pixels() {
        // At the itch.io embed (985 x 455) the scale is 0.42: Label (20 px) would be 8 px, so it grows to the floor.
        let ctx = egui::Context::default();
        let mut sizes = Vec::new();
        for (w, h) in [(1920u32, 1080u32), (1280, 720), (985, 455)] {
            let fit = crate::ui::Fit::new(w, h, w as f32);
            let raw =
                egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, fit.points)), ..Default::default() };
            let mut raw = raw;
            raw.viewports.entry(raw.viewport_id).or_default().native_pixels_per_point = Some(fit.ppp);
            let _ = ctx.run(raw, |ctx| {
                let (o, s) = crate::ui::anchored(ctx.content_rect(), crate::ui::Anchor::TopLeft, Vec2::ZERO);
                let k = Kit::new(ctx.layer_painter(egui::LayerId::background()), o, s);
                sizes.push((w, k.size(Role::Label) * s * fit.ppp, k.size(Role::Body) * s * fit.ppp));
            });
        }
        for (w, label_px, body_px) in sizes {
            assert!(label_px >= 12.0 - 1e-3, "Label is {label_px} px at {w} wide");
            assert!(body_px >= 12.0 - 1e-3, "Body is {body_px} px at {w} wide");
        }
    }
}
