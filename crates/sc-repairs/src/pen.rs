//! The pen every repair game draws with: canvas pixels on the 1280 x 720 frame, the canvas 2D calls the mockups use
//! (fills, strokes, arcs, gradients, a transform, a clip, global alpha), turned into egui shapes.
//!
//! It lives in `sc-repairs` so a game ported from `docs/mockups/repairs/<id>.js` reads line for line like its mockup,
//! and every game draws through one set of helpers (the mockup kit's `KIT.draw`), so they look like one set.

use egui::epaint::{Mesh, PathShape, PathStroke, Vertex, WHITE_UV};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Shape, Stroke, StrokeKind};

/// A colour from `0xRRGGBB`.
pub const fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// A colour from 0-255 channels and an alpha of 0-1 (the mockups' `rgba(...)`).
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// `c` at alpha `a` (0-1) times its own.
pub fn alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, ca] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (f32::from(ca) * a.clamp(0.0, 1.0)).round() as u8)
}

/// Between `a` and `b` by `t` (0-1).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let (a, b) = (a.to_srgba_unmultiplied(), b.to_srgba_unmultiplied());
    let m = |i: usize| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(0), m(1), m(2), m(3))
}

/// The kit's palette (`kit.js`'s `C`).
pub mod c {
    use super::hex;
    use egui::Color32;
    /// The frame.
    pub const BG: Color32 = hex(0x04060a);
    /// A panel.
    pub const PANEL: Color32 = hex(0x0c121a);
    /// The bar.
    pub const PANEL2: Color32 = hex(0x121a25);
    /// A rule.
    pub const LINE: Color32 = hex(0x1f2a37);
    /// Text.
    pub const FG: Color32 = hex(0xe8eef6);
    /// Quiet text.
    pub const DIM: Color32 = hex(0x6f7f94);
    /// Good.
    pub const OK: Color32 = hex(0x3ddc84);
    /// Watch it.
    pub const WARN: Color32 = hex(0xffc542);
    /// A mistake.
    pub const DANGER: Color32 = hex(0xff4757);
    /// The finger, a selection.
    pub const ACCENT: Color32 = hex(0x4fc3f7);
    /// A target.
    pub const AMBER: Color32 = hex(0xf2a046);
    /// Lilac.
    pub const LILAC: Color32 = hex(0xbe9fe6);
    /// Steel.
    pub const STEEL: Color32 = hex(0x7d8796);
    /// Copper.
    pub const COPPER: Color32 = hex(0xd08a4a);
}

/// How text sits on its point (canvas's `textAlign`; the baseline is always the middle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// From the point rightwards.
    Left,
    /// Centred on it.
    Center,
    /// Ending at it.
    Right,
}

/// A 2D affine transform, canvas's `[a b c d e f]`: x' = a x + c y + e, y' = b x + d y + f.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Xf([f32; 6]);

impl Xf {
    fn apply(&self, x: f32, y: f32) -> [f32; 2] {
        let m = self.0;
        [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
    }
    fn then(&self, o: &Xf) -> Xf {
        // self * o: apply o first, then self.
        let (a, b) = (self.0, o.0);
        Xf([
            a[0] * b[0] + a[2] * b[1],
            a[1] * b[0] + a[3] * b[1],
            a[0] * b[2] + a[2] * b[3],
            a[1] * b[2] + a[3] * b[3],
            a[0] * b[4] + a[2] * b[5] + a[4],
            a[1] * b[4] + a[3] * b[5] + a[5],
        ])
    }
    fn scale(&self) -> f32 {
        let m = self.0;
        (m[0] * m[3] - m[1] * m[2]).abs().sqrt()
    }
    fn axis_aligned(&self) -> bool {
        self.0[1].abs() < 1e-6 && self.0[2].abs() < 1e-6
    }
}

/// The pen: an egui painter, the canvas-to-screen mapping, the current transform and alpha. Cheap to clone; the
/// transform calls return a new pen, so a nested `save()`/`restore()` is a clone that goes out of scope.
#[derive(Clone)]
pub struct Pen {
    painter: egui::Painter,
    /// Canvas pixels to screen points, then the game's own transform.
    xf: Xf,
    a: f32,
}

impl Pen {
    /// A pen drawing the 1280 x 720 canvas into `screen` (points) on `painter`.
    pub fn new(painter: egui::Painter, screen: Rect) -> Self {
        let s = screen.width() / crate::kit::W;
        let painter = painter.with_clip_rect(screen);
        Self { painter, xf: Xf([s, 0.0, 0.0, s, screen.min.x, screen.min.y]), a: 1.0 }
    }

    /// Screen points of canvas point (x, y).
    pub fn pt(&self, x: f32, y: f32) -> Pos2 {
        let [px, py] = self.xf.apply(x, y);
        Pos2::new(px, py)
    }

    fn len(&self, l: f32) -> f32 {
        l * self.xf.scale()
    }

    fn col(&self, c: Color32) -> Color32 {
        if self.a >= 1.0 {
            c
        } else {
            alpha(c, self.a)
        }
    }

    /// Moved by (x, y).
    pub fn translate(&self, x: f32, y: f32) -> Pen {
        Pen { xf: self.xf.then(&Xf([1.0, 0.0, 0.0, 1.0, x, y])), ..self.clone() }
    }

    /// Turned by `a` radians (clockwise on screen, as canvas's `rotate`).
    pub fn rotate(&self, a: f32) -> Pen {
        let (s, c) = a.sin_cos();
        Pen { xf: self.xf.then(&Xf([c, s, -s, c, 0.0, 0.0])), ..self.clone() }
    }

    /// Scaled by (sx, sy).
    pub fn scale(&self, sx: f32, sy: f32) -> Pen {
        Pen { xf: self.xf.then(&Xf([sx, 0.0, 0.0, sy, 0.0, 0.0])), ..self.clone() }
    }

    /// At global alpha `a` times the current one.
    pub fn alpha(&self, a: f32) -> Pen {
        Pen { a: self.a * a.clamp(0.0, 1.0), ..self.clone() }
    }

    /// Clipped to the canvas rectangle (x, y, w, h) under the current transform's bounds.
    pub fn clip(&self, x: f32, y: f32, w: f32, h: f32) -> Pen {
        let r = Rect::from_two_pos(self.pt(x, y), self.pt(x + w, y + h));
        let r = r.union(Rect::from_two_pos(self.pt(x + w, y), self.pt(x, y + h)));
        Pen { painter: self.painter.with_clip_rect(r.intersect(self.painter.clip_rect())), ..self.clone() }
    }

    fn add(&self, s: impl Into<Shape>) {
        self.painter.add(s);
    }

    fn pts(&self, pts: &[[f32; 2]]) -> Vec<Pos2> {
        pts.iter().map(|p| self.pt(p[0], p[1])).collect()
    }

    // ------------------------------------------------------------------------------------------------ shapes

    /// A filled rectangle.
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, fill: Color32) {
        self.round(x, y, w, h, 0.0, Some(fill), None);
    }

    /// A rectangle's outline.
    pub fn rect_stroke(&self, x: f32, y: f32, w: f32, h: f32, width: f32, col: Color32) {
        self.round(x, y, w, h, 0.0, None, Some((width, col)));
    }

    /// A rounded rectangle, filled and or stroked (stroke centred on the edge, as canvas's).
    #[allow(clippy::too_many_arguments)]
    pub fn round(&self, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Option<Color32>, stroke: Option<(f32, Color32)>) {
        let (x, w) = if w < 0.0 { (x + w, -w) } else { (x, w) };
        let (y, h) = if h < 0.0 { (y + h, -h) } else { (y, h) };
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        if self.xf.axis_aligned() {
            let rect = Rect::from_two_pos(self.pt(x, y), self.pt(x + w, y + h));
            let cr = CornerRadius::same(self.len(r).round().clamp(0.0, 255.0) as u8);
            if let Some(f) = fill {
                self.painter.rect_filled(rect, cr, self.col(f));
            }
            if let Some((sw, sc)) = stroke {
                self.painter.rect_stroke(rect, cr, Stroke::new(self.len(sw), self.col(sc)), StrokeKind::Middle);
            }
            return;
        }
        let pts = round_rect_points(x, y, w, h, r);
        if let Some(f) = fill {
            self.poly(&pts, f);
        }
        if let Some((sw, sc)) = stroke {
            self.path(&pts, true, sw, sc);
        }
    }

    /// The kit's panel: a rounded rectangle, filled, with a 2 px rule.
    #[allow(clippy::too_many_arguments)]
    pub fn panel(&self, x: f32, y: f32, w: f32, h: f32, r: f32, fill: Color32, stroke: Color32) {
        self.round(x, y, w, h, r, Some(fill), Some((2.0, stroke)));
    }

    /// A filled circle.
    pub fn disc(&self, x: f32, y: f32, r: f32, col: Color32) {
        self.painter.circle_filled(self.pt(x, y), self.len(r).max(0.0), self.col(col));
    }

    /// A circle's outline.
    pub fn ring(&self, x: f32, y: f32, r: f32, col: Color32, width: f32) {
        self.painter.circle_stroke(self.pt(x, y), self.len(r).max(0.0), Stroke::new(self.len(width), self.col(col)));
    }

    /// A straight line.
    pub fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, col: Color32) {
        self.painter.line_segment([self.pt(x0, y0), self.pt(x1, y1)], Stroke::new(self.len(width), self.col(col)));
    }

    /// A polyline, open or closed.
    pub fn path(&self, pts: &[[f32; 2]], closed: bool, width: f32, col: Color32) {
        if pts.len() < 2 {
            return;
        }
        let s = PathStroke::new(self.len(width), self.col(col));
        let p = self.pts(pts);
        self.add(if closed { Shape::closed_line(p, s) } else { Shape::line(p, s) });
    }

    /// A polyline with round joins and ends (the mockups' `lineCap = "round"`): the line, and a disc at each point.
    pub fn path_round(&self, pts: &[[f32; 2]], width: f32, col: Color32) {
        self.path(pts, false, width, col);
        for p in pts {
            self.disc(p[0], p[1], width / 2.0, col);
        }
    }

    /// A dashed polyline: `dash` on, `gap` off, canvas pixels.
    pub fn dashed(&self, pts: &[[f32; 2]], width: f32, col: Color32, dash: f32, gap: f32) {
        if pts.len() < 2 {
            return;
        }
        let s = self.len(1.0);
        for sh in Shape::dashed_line(&self.pts(pts), Stroke::new(self.len(width), self.col(col)), dash * s, gap * s) {
            self.add(sh);
        }
    }

    /// A filled polygon, convex or not.
    pub fn poly(&self, pts: &[[f32; 2]], fill: Color32) {
        if pts.len() < 3 {
            return;
        }
        if convex(pts) {
            self.add(Shape::Path(PathShape::convex_polygon(self.pts(pts), self.col(fill), Stroke::NONE)));
            return;
        }
        let tris = triangulate(pts);
        let mut m = Mesh::default();
        let colour = self.col(fill);
        for p in pts {
            m.vertices.push(Vertex { pos: self.pt(p[0], p[1]), uv: WHITE_UV, color: colour });
        }
        m.indices = tris;
        self.add(Shape::mesh(m));
    }

    /// The points of an arc round (x, y) from `a0` to `a1` radians (clockwise on screen when a1 > a0).
    pub fn arc_points(x: f32, y: f32, r: f32, a0: f32, a1: f32) -> Vec<[f32; 2]> {
        let n = (((a1 - a0).abs() * r.max(1.0) / 6.0).ceil() as usize).clamp(6, 128);
        Self::arc_points_n(x, y, r, a0, a1, n)
    }

    /// As [`Pen::arc_points`], in exactly `n` steps (two arcs that must pair point for point).
    pub fn arc_points_n(x: f32, y: f32, r: f32, a0: f32, a1: f32, n: usize) -> Vec<[f32; 2]> {
        let n = n.max(1);
        (0..=n)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / n as f32;
                [x + a.cos() * r, y + a.sin() * r]
            })
            .collect()
    }

    /// An arc's stroke from `a0` to `a1`.
    #[allow(clippy::too_many_arguments)]
    pub fn arc(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32, width: f32, col: Color32) {
        self.path(&Self::arc_points(x, y, r, a0, a1), false, width, col);
    }

    /// A filled band of a ring between radii r0 and r1 and angles a0 and a1 (a gauge's zone, a pie when r0 is 0).
    #[allow(clippy::too_many_arguments)]
    pub fn sector(&self, x: f32, y: f32, r0: f32, r1: f32, a0: f32, a1: f32, fill: Color32) {
        let outer = Self::arc_points(x, y, r1, a0, a1);
        let n = outer.len();
        // The inner arc point for point with the outer one, whatever its radius (a pie's is a single point, repeated).
        let inner = Self::arc_points_n(x, y, r0.max(0.0), a0, a1, n - 1);
        let mut m = Mesh::default();
        let colour = self.col(fill);
        for p in outer.iter().chain(inner.iter()) {
            m.vertices.push(Vertex { pos: self.pt(p[0], p[1]), uv: WHITE_UV, color: colour });
        }
        for i in 0..n as u32 - 1 {
            let (o0, o1, i0, i1) = (i, i + 1, n as u32 + i, n as u32 + i + 1);
            m.indices.extend_from_slice(&[o0, o1, i1, o0, i1, i0]);
        }
        self.add(Shape::mesh(m));
    }

    /// Text at (x, y), `size` canvas pixels, middle baseline. The weight is the face's (egui's has one).
    #[allow(clippy::too_many_arguments)]
    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, col: Color32, align: Align) {
        let a = match align {
            Align::Left => Align2::LEFT_CENTER,
            Align::Center => Align2::CENTER_CENTER,
            Align::Right => Align2::RIGHT_CENTER,
        };
        self.painter.text(self.pt(x, y), a, s, FontId::proportional(self.len(size).max(1.0)), self.col(col));
    }

    /// How wide `s` is at `size`, canvas pixels.
    pub fn text_width(&self, s: &str, size: f32) -> f32 {
        let g =
            self.painter.layout_no_wrap(s.to_owned(), FontId::proportional(self.len(size).max(1.0)), Color32::WHITE);
        g.size().x / self.xf.scale().max(1e-6)
    }

    /// A rectangle filled with a gradient from `c0` to `c1`, left to right or top to bottom.
    #[allow(clippy::too_many_arguments)]
    pub fn gradient(&self, x: f32, y: f32, w: f32, h: f32, c0: Color32, c1: Color32, vertical: bool) {
        let (a, b) = (self.col(c0), self.col(c1));
        let cols = if vertical { [a, a, b, b] } else { [a, b, b, a] };
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], cols);
    }

    /// A quadrilateral with a colour at each corner (a gradient's piece).
    pub fn quad(&self, p: [[f32; 2]; 4], cols: [Color32; 4]) {
        let mut m = Mesh::default();
        for (q, col) in p.iter().zip(cols) {
            m.vertices.push(Vertex { pos: self.pt(q[0], q[1]), uv: WHITE_UV, color: self.col(col) });
        }
        m.indices = vec![0, 1, 2, 0, 2, 3];
        self.add(Shape::mesh(m));
    }

    /// A disc shading from `inner` at its centre to `outer` at its rim (a radial gradient).
    pub fn radial(&self, x: f32, y: f32, r: f32, inner: Color32, outer: Color32) {
        let mut m = Mesh::default();
        m.vertices.push(Vertex { pos: self.pt(x, y), uv: WHITE_UV, color: self.col(inner) });
        let n = 40;
        for i in 0..=n {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            m.vertices.push(Vertex {
                pos: self.pt(x + a.cos() * r, y + a.sin() * r),
                uv: WHITE_UV,
                color: self.col(outer),
            });
        }
        for i in 1..=n as u32 {
            m.indices.extend_from_slice(&[0, i, i + 1]);
        }
        self.add(Shape::mesh(m));
    }

    // ---------------------------------------------------------------------------------------- the kit's pictures

    /// Diagonal hatching over a rectangle: a stop, a red zone, a gap (colour always with a shape, CLAUDE.md 10).
    pub fn hatch(&self, x: f32, y: f32, w: f32, h: f32, col: Color32) {
        let p = self.clip(x, y, w, h);
        let mut i = -h;
        while i < w {
            p.line(x + i, y + h, x + i + h, y, 3.0, col);
            i += 12.0;
        }
    }

    /// Diagonal hatching over a band of a ring (a gauge's red zone): the band, tinted, with dark stripes across it.
    #[allow(clippy::too_many_arguments)]
    pub fn hatch_arc(&self, x: f32, y: f32, r0: f32, r1: f32, a0: f32, a1: f32, col: Color32) {
        self.sector(x, y, r0, r1, a0, a1, alpha(col, 0.35));
        let n = (((a1 - a0).abs() * r1) / 9.0).ceil().max(2.0) as usize;
        for k in 0..=n {
            let a = a0 + (a1 - a0) * k as f32 / n as f32;
            let b = a + 0.12 * (a1 - a0).signum();
            self.line(x + a.cos() * r0, y + a.sin() * r0, x + b.cos() * r1, y + b.sin() * r1, 3.0, col);
        }
    }

    /// A curved arrow along a rim from a0 to a1, the head at a1: the way a ring or a crank turns.
    #[allow(clippy::too_many_arguments)]
    pub fn turn_arrow(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32, col: Color32) {
        self.arc(x, y, r, a0, a1, 4.0, col);
        let (hx, hy) = (x + a1.cos() * r, y + a1.sin() * r);
        let d = a1 + if a1 < a0 { -1.0 } else { 1.0 } * std::f32::consts::FRAC_PI_2;
        self.poly(
            &[
                [hx + d.cos() * 12.0, hy + d.sin() * 12.0],
                [hx + a1.cos() * 9.0, hy + a1.sin() * 9.0],
                [hx - a1.cos() * 9.0, hy - a1.sin() * 9.0],
            ],
            col,
        );
    }

    /// A fastener's place in its order: its number beside it, and a bright pulsing ring on the one that comes next.
    #[allow(clippy::too_many_arguments)]
    pub fn order_badge(&self, x: f32, y: f32, r: f32, n: usize, next: bool, t: f32) {
        if next {
            self.ring(x, y, r + 9.0 + 2.0 * (t * 6.0).sin(), c::ACCENT, 3.0);
            self.disc(x, y, r + 5.0, rgba(79, 195, 247, 0.18));
        }
        let br = (r * 0.8).max(13.0);
        let (bx, by) = (x + r + br - 2.0, y - r - br + 4.0);
        self.disc(bx, by, br, if next { c::ACCENT } else { hex(0x1d2738) });
        self.ring(bx, by, br, if next { hex(0xe8f7ff) } else { hex(0x4a5568) }, 1.5);
        self.text(
            &n.to_string(),
            bx,
            by + 1.0,
            (br * 1.3).round(),
            if next { hex(0x04131c) } else { c::FG },
            Align::Center,
        );
    }

    /// A button's picture (`kit.js`'s `button`): a pill, hovered or lit, its label. Whether it was pressed is
    /// [`crate::kit::Input::pressed_in`].
    #[allow(clippy::too_many_arguments)]
    pub fn button(&self, label: &str, x: f32, y: f32, w: f32, h: f32, hover: bool, on: bool, fill: Option<Color32>) {
        let f = fill.unwrap_or(if hover { hex(0x46527a) } else { hex(0x262e42) });
        self.round(x, y, w, h, h / 2.0, Some(f), on.then_some((3.0, c::AMBER)));
        self.text(label, x + w / 2.0, y + h / 2.0, 20.0, c::FG, Align::Center);
    }

    /// A tick, drawn (the face has no ✓).
    pub fn tick(&self, x: f32, y: f32, s: f32, col: Color32) {
        self.path_round(&[[x - s * 0.5, y], [x - s * 0.15, y + s * 0.38], [x + s * 0.55, y - s * 0.4]], s * 0.22, col);
    }

    /// A cross, drawn.
    pub fn cross(&self, x: f32, y: f32, s: f32, width: f32, col: Color32) {
        self.line(x - s, y - s, x + s, y + s, width, col);
        self.line(x + s, y - s, x - s, y + s, width, col);
    }

    /// Pipes with a bore (`kit.js`'s `pipe`): a dark casing, a metal body, and down the middle a bore, dry or holding
    /// `fluid` as far as each run's fill (0-1, from its first point) has got. Each layer is drawn for every run before
    /// the next, so runs that meet join cleanly.
    pub fn pipe(&self, runs: &[(Vec<[f32; 2]>, f32)], o: &PipeStyle) {
        for (width, col) in [(o.w + 6.0, o.casing), (o.w, o.body), (o.w * o.bore, o.dry)] {
            for (pts, _) in runs {
                self.path_round(pts, width, col);
            }
        }
        if let Some(f) = o.fluid {
            let p = self.alpha(o.fluid_alpha);
            for (pts, fill) in runs {
                if *fill > 0.0 {
                    let cut = if *fill >= 1.0 { pts.clone() } else { crate::kit::poly_cut(pts, *fill) };
                    p.path_round(&cut, o.w * o.bore, f);
                }
            }
        }
    }

    /// Where a tile's arms meet: a boss on the body, its bore dry or holding `fluid`.
    #[allow(clippy::too_many_arguments)]
    pub fn pipe_hub(
        &self,
        x: f32,
        y: f32,
        r: f32,
        body: Color32,
        dry: Color32,
        fluid: Option<Color32>,
        fluid_alpha: f32,
    ) {
        self.disc(x, y, r, body);
        self.disc(x, y, r * 0.43, dry);
        if let Some(f) = fluid {
            self.alpha(fluid_alpha).disc(x, y, r * 0.43, f);
        }
    }

    /// The frame's whole screen rectangle, points (for a full-frame wash).
    pub fn screen(&self) -> Rect {
        self.painter.clip_rect()
    }

    /// A vector's length on screen for `l` canvas pixels (for tests of the mapping).
    pub fn screen_len(&self, l: f32) -> f32 {
        self.len(l)
    }
}

/// How [`Pen::pipe`] draws (the mockup kit's defaults).
#[derive(Clone, Copy, Debug)]
pub struct PipeStyle {
    /// The body's width.
    pub w: f32,
    /// The metal.
    pub body: Color32,
    /// The casing round it.
    pub casing: Color32,
    /// The bore's share of the width.
    pub bore: f32,
    /// An empty bore.
    pub dry: Color32,
    /// What runs in it.
    pub fluid: Option<Color32>,
    /// Its alpha.
    pub fluid_alpha: f32,
}

impl Default for PipeStyle {
    fn default() -> Self {
        Self {
            w: 22.0,
            body: hex(0x3a4658),
            casing: hex(0x232c3b),
            bore: 0.4,
            dry: hex(0x141b27),
            fluid: None,
            fluid_alpha: 1.0,
        }
    }
}

/// The outline of a rounded rectangle.
pub fn round_rect_points(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<[f32; 2]> {
    use std::f32::consts::{FRAC_PI_2, PI};
    if r <= 0.0 {
        return vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
    }
    let mut out = Vec::new();
    for (cx, cy, a) in
        [(x + w - r, y + r, -FRAC_PI_2), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, FRAC_PI_2), (x + r, y + r, PI)]
    {
        out.extend(Pen::arc_points(cx, cy, r, a, a + FRAC_PI_2));
    }
    out
}

fn cross_z(o: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn convex(pts: &[[f32; 2]]) -> bool {
    let n = pts.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let z = cross_z(pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        if z.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = z.signum();
        } else if z.signum() != sign {
            return false;
        }
    }
    true
}

/// Ear clipping: the triangles of a simple polygon, as indices into `pts`.
fn triangulate(pts: &[[f32; 2]]) -> Vec<u32> {
    let n = pts.len();
    let area: f32 = (0..n).map(|i| cross_z([0.0, 0.0], pts[i], pts[(i + 1) % n])).sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (a, b, cc) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let z = cross_z(pts[a], pts[b], pts[cc]);
            if (z > 0.0) != ccw || z.abs() < 1e-9 {
                continue;
            }
            let inside = idx.iter().any(|&p| {
                if p == a || p == b || p == cc {
                    return false;
                }
                let (d1, d2, d3) = (
                    cross_z(pts[a], pts[b], pts[p]),
                    cross_z(pts[b], pts[cc], pts[p]),
                    cross_z(pts[cc], pts[a], pts[p]),
                );
                let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
                let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
                !(neg && pos)
            });
            if inside {
                continue;
            }
            out.extend_from_slice(&[a as u32, b as u32, cc as u32]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        out.extend(idx.iter().map(|&i| i as u32));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Vec2;

    #[test]
    fn an_l_shape_triangulates_into_four_triangles_covering_it() {
        let l = [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]];
        assert!(!convex(&l));
        let t = triangulate(&l);
        assert_eq!(t.len(), 12, "n - 2 triangles");
        let area: f32 =
            t.chunks(3).map(|c| cross_z(l[c[0] as usize], l[c[1] as usize], l[c[2] as usize]).abs() / 2.0).sum();
        assert!((area - 3.0).abs() < 1e-4, "the triangles cover the shape exactly: {area}");
    }

    #[test]
    fn the_canvas_maps_onto_the_screen_rectangle() {
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(640.0, 360.0));
        let pen = Pen::new(egui::Painter::new(ctx, egui::LayerId::background(), screen), screen);
        assert_eq!(pen.pt(0.0, 0.0), Pos2::new(100.0, 50.0));
        assert_eq!(pen.pt(1280.0, 720.0), Pos2::new(740.0, 410.0));
        let p = pen.translate(640.0, 360.0).rotate(std::f32::consts::FRAC_PI_2);
        let q = p.pt(10.0, 0.0);
        assert!((q.x - 420.0).abs() < 1e-3 && (q.y - 235.0).abs() < 1e-3, "rotate turns +x to +y (screen down): {q:?}");
        assert!((pen.screen_len(10.0) - 5.0).abs() < 1e-5);
    }
}
