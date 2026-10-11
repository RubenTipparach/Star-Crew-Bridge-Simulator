//! The engineering console (the mockup's `engineering()` and `reactorLoop()`): POWER (supply against demand, a fader
//! a load group with its priority and feed breaker, the four presets), REACTOR (the core at the top of its cooling
//! loop, SCRAM and the reactor's mode, the two needles, the loop's four levers and its AUTO), BUSES (the one-line
//! diagram and the battery) and AIR (three lamps).
//!
//! Everything it draws is [`EngView`]: the power grid's, the reactor's and the loop's own numbers. Where nothing aboard
//! simulates them (the drill has no power grid yet), the console has no `eng` and every control is drawn in the
//! mockup's unavailable state, faint and inert (CLAUDE.md 10).

use super::helm::{hatch_p, panel};
use super::kit::{self, arc_path, bar, button, pol, Act, Btn, Drag, Hit};
use super::{accent, grid, CoolView, Ctx, EngView, GroupView};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::Color32;

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    match cx.v.eng.clone() {
        Some(e) => {
            power(cv, cx, &e);
            reactor(cv, cx, &e);
            buses(cv, cx, &e);
            air(cv, cx, &e);
        }
        None => unavailable(cv, cx),
    }
}

/// The shortfall's hatch: the mockup's `short` pattern, 3 of every 6 in the danger colour.
fn short(cv: &Canvas, x: f32, y: f32, w: f32, h: f32) {
    hatch_p(cv, x, y, w, h, vg::style().c.danger.0, 6.0, 3.0);
}

// ------------------------------------------------------------------------------------------------ POWER

fn power(cv: &Canvas, cx: &mut Ctx, e: &EngView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "POWER", grid(0.0, 0.0, 6.0, 4.0));
    let acc = accent(cx.v);
    let up = e.reactor.up;
    let got = e.rx_out_mw + e.bat_out_mw;
    let rated = e.rated_mw;
    let (sx, sw) = (50.0_f32, w - 200.0);
    let k = |v: f64| sx + sw * (v / rated).clamp(0.0, 1.0) as f32;
    b.icon("bolt", 26.0, 22.0, 20.0, if up { c.text.0 } else { c.emergency.0 });
    b.rect(sx, 12.0, sw, 20.0, 4.0, rgba(0x16202c, 1.0), None);
    cx.hits.add(&b, sx, 12.0, sw, 20.0, None, Some("Power in use against the reactor's maximum".into()));
    b.rect(sx, 12.0, k(e.rx_out_mw) - sx, 20.0, 4.0, alpha(c.ok.0, 0.8), None);
    if e.bat_out_mw > 0.05 {
        b.rect(k(e.rx_out_mw), 12.0, k(got) - k(e.rx_out_mw), 20.0, 0.0, c.emergency.0, None);
        cx.hits.add(&b, k(e.rx_out_mw), 12.0, k(got) - k(e.rx_out_mw), 20.0, None, Some("From the battery".into()));
    }
    if e.want_mw > got + 0.05 {
        short(&b, k(got), 12.0, k(e.want_mw) - k(got), 20.0);
        cx.hits.add(&b, k(got), 12.0, k(e.want_mw) - k(got), 20.0, None, Some("Wanted, not delivered".into()));
    }
    b.text(w - 16.0, 28.0, 18.0, c.text.0, &format!("{:.1} / {:.0} MW", got, rated), T::end());
    // One fader per load group, like a mixing desk.
    let n = e.groups.len().max(1) as f32;
    let (fx0, fw, ty) = (14.0_f32, (w - 28.0) / n, 70.0_f32);
    let th = h - 70.0 - 130.0;
    let y_of = |v: f64| ty + th * (1.0 - v as f32 / 1.5);
    for (i, g) in e.groups.iter().enumerate() {
        let x = fx0 + i as f32 * fw;
        let gx = x + fw / 2.0;
        let pv = e.preview.as_ref().filter(|p| p.gid == g.id);
        let brk = g.breaker.as_deref().unwrap_or("closed");
        let short_now = g.got < g.want - (0.02 * g.want).max(0.01);
        let off = g.sp <= 0.0 || brk != "closed";
        b.icon(
            &g.id,
            gx,
            52.0,
            22.0,
            if off {
                c.faint.0
            } else if short_now {
                c.danger.0
            } else {
                c.text.0
            },
        );
        b.rect(
            gx - 13.0,
            y_of(g.max),
            26.0,
            ty + th - y_of(g.max),
            6.0,
            rgba(0x101822, 1.0),
            Some(Pen::new(1.0, c.line2.0)),
        );
        b.seg(gx - 18.0, y_of(1.0), gx + 18.0, y_of(1.0), Pen::new(1.0, c.dim.0).dash(3.0, 3.0, 0.0));
        if short_now {
            short(&b, gx - 9.0, y_of(g.want_k), 18.0, y_of(g.got_k) - y_of(g.want_k));
        }
        let fill = if off {
            c.faint.0
        } else if g.sp > 1.0 {
            c.warn.0
        } else {
            c.ok.0
        };
        b.rect(gx - 9.0, y_of(g.got_k), 18.0, ty + th - y_of(g.got_k), 3.0, alpha(fill, 0.9), None);
        if let Some(p) = pv {
            b.rect(gx - 9.0, y_of(p.k), 18.0, ty + th - y_of(p.k), 3.0, alpha(acc, 0.45), None);
        }
        let hy = y_of(pv.map(|p| p.sp).unwrap_or(g.sp));
        b.rect(gx - 22.0, hy - 7.0, 44.0, 14.0, 4.0, if pv.is_some() { acc } else { c.text.0 }, None);
        let tip = format!("{}: {:.2} of {:.2} MW wanted", g.name, g.got, g.want);
        cx.hits.add_track(
            &b,
            gx - 22.0,
            y_of(g.max) - 7.0,
            44.0,
            ty + th - y_of(g.max) + 14.0,
            [gx - 13.0, y_of(g.max), 26.0, ty + th - y_of(g.max)],
            Hit::Drag(Drag::Fader(g.id.clone())),
            &tip,
        );
        let sp = pv.map(|p| p.sp).unwrap_or(g.sp);
        let word = if off { "OFF".to_owned() } else { kit::js_round(sp * 100.0).to_string() };
        b.text(gx, ty + th + 22.0, 15.0, if off { c.faint.0 } else { c.text.0 }, &word, T::mid().bold());
        // Priority: one to three pips, a ring round them at 0 (never shed); tap to cycle.
        if g.prio == 0 {
            b.rect(gx - 21.0, ty + th + 31.0, 42.0, 18.0, 9.0, Color32::TRANSPARENT, Some(Pen::new(1.5, acc)));
        }
        for p in 0..3_u8 {
            let lit = i32::from(p) < 4 - i32::from(g.prio);
            b.circle(
                gx - 12.0 + f32::from(p) * 12.0,
                ty + th + 40.0,
                4.0,
                if lit { acc } else { rgba(0x1c2734, 1.0) },
                None,
            );
        }
        let ptip = if g.prio == 0 { "Priority 0: never shed" } else { "Priority: 1 is fed first" };
        cx.hits.add(
            &b,
            gx - 22.0,
            ty + th + 28.0,
            44.0,
            24.0,
            Some(Hit::Act(Act::Prio(g.id.clone()))),
            Some(ptip.into()),
        );
        // The feed breaker (power-grid 5): held 0.6 s to open or close, a lock beside it once it is open.
        if g.breaker.is_some() {
            feed_breaker(&b, cx, g, gx, ty + th + 58.0, brk);
        }
    }
    let pre = [("CRUISE", "cruise"), ("COMBAT", "combat"), ("SILENT", "silent"), ("EMERG", "emergency")];
    let bw = (w - 24.0 - 3.0 * 8.0) / 4.0;
    for (i, (label, id)) in pre.iter().enumerate() {
        button(
            &b,
            cx,
            Btn {
                x: 12.0 + i as f32 * (bw + 8.0),
                y: h - 42.0,
                w: bw,
                h: 34.0,
                label: Some(label),
                act: Some(Act::Preset((*id).to_owned())),
                on: e.preset.as_deref() == Some(*id),
                fs: Some(14.0),
                ..Default::default()
            },
        );
    }
}

fn feed_breaker(b: &Canvas, cx: &mut Ctx, g: &GroupView, gx: f32, by: f32, brk: &str) {
    let c = &vg::style().c;
    let (closed, locked) = (brk == "closed", brk == "locked");
    let bx = gx - 26.0;
    let key = format!("gbrk:{}", g.id);
    b.rect(
        bx,
        by,
        26.0,
        26.0,
        4.0,
        if closed { c.ok.0 } else { Color32::TRANSPARENT },
        Some(Pen::new(2.5, if closed { c.ok.0 } else { c.warn.0 })),
    );
    if !closed {
        b.seg(bx + 6.0, by + 20.0, bx + 20.0, by + 6.0, Pen::new(2.5, c.warn.0));
    }
    if let Some(hv) = cx.v.hold.as_ref().filter(|h| h.key == key) {
        b.rect(bx, by, 26.0 * hv.k.clamp(0.0, 1.0) as f32, 26.0, 0.0, alpha(c.warn.0, 0.6), None);
    }
    let tip = if locked {
        format!("{} breaker: locked open", g.name)
    } else {
        format!("{} breaker: hold to {}", g.name, if closed { "open" } else { "close" })
    };
    cx.hits.add(b, bx, by, 26.0, 26.0, Some(Hit::Hold(key)), Some(tip));
    b.rect(
        gx + 2.0,
        by,
        24.0,
        26.0,
        4.0,
        if locked { alpha(c.warn.0, 0x33 as f32 / 255.0) } else { Color32::TRANSPARENT },
        Some(Pen::new(1.0, if locked { c.warn.0 } else { c.line2.0 })),
    );
    let lc = if locked {
        c.warn.0
    } else if closed {
        c.faint.0
    } else {
        c.dim.0
    };
    b.icon("lock", gx + 14.0, by + 13.0, 16.0, lc);
    let ltip = if closed {
        "Open the breaker to lock it"
    } else if locked {
        "Unlock"
    } else {
        "Lock open"
    };
    let hit = (!closed).then(|| Hit::Act(Act::GLock(g.id.clone())));
    cx.hits.add(b, gx + 2.0, by, 24.0, 26.0, hit, Some(ltip.into()));
}

// ------------------------------------------------------------------------------------------------ REACTOR

const COLD: u32 = 0x4aa8ff;
const RADC: u32 = 0x7fd3ff;

/// The mockup's `mixCol`: `k` of the way from `a` to `b`, each channel rounded.
fn mix_col(a: Color32, b: Color32, k: f64) -> Color32 {
    let ch = |x: u8, y: u8| (f64::from(x) * (1.0 - k) + f64::from(y) * k).round() as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

/// The hot leg's colour: red to orange with its temperature.
fn hot_col(t: f64) -> Color32 {
    mix_col(rgba(0xe0483a, 1.0), rgba(0xffa42e, 1.0), ((t - 335.0) / 35.0).clamp(0.0, 1.0))
}

/// The hot leg's state: green in its band or under it, amber over it, red from the over-temperature warning.
fn hot_state(cr: &CoolView) -> Color32 {
    let c = &vg::style().c;
    if cr.hot_k >= cr.hot_warn_k {
        c.danger.0
    } else if cr.hot_k > cr.hot_band_k[1] {
        c.warn.0
    } else {
        c.ok.0
    }
}

/// A pipe (the mockup's `pipe`): a faint underlay as thick as its flow, and dashes moving with the flow.
fn pipe(b: &Canvas, cx: &mut Ctx, d: &str, col: Color32, f: f64, t: f64, tip: String) {
    let wid = (2.5 + 7.5 * f.clamp(0.0, 1.2)) as f32;
    // SVG's `(-(t * 46 * f) % 36).toFixed(1)`: the dash offset, which keeps the sign of its dividend.
    let mv = ((-(t * 46.0 * f)) % 36.0 * 10.0).round() / 10.0;
    b.path(d, Xf::ID, None, Some(Pen::new(wid, alpha(col, 0.35))));
    b.path(d, Xf::ID, None, Some(Pen::new(wid, col).dash(12.0, 6.0, mv as f32)));
    let sub = vg::parse_path(d, 4.0);
    for s in &sub {
        for w in s.pts.windows(2) {
            let (x0, y0, x1, y1) =
                (w[0][0].min(w[1][0]), w[0][1].min(w[1][1]), w[0][0].max(w[1][0]), w[0][1].max(w[1][1]));
            cx.hits.add(b, x0 - 9.0, y0 - 9.0, x1 - x0 + 18.0, y1 - y0 + 18.0, None, Some(tip.clone()));
        }
    }
}

/// A valve on a pipe: a bow tie, filled as far as it is open.
fn valve(b: &Canvas, x: f32, y: f32, open: f64, col: Color32) {
    let c = &vg::style().c;
    let d =
        format!("M{} {}L{} {}L{} {}L{} {}Z", x - 8.0, y - 7.0, x - 8.0, y + 7.0, x + 8.0, y - 7.0, x + 8.0, y + 7.0);
    let (fill, stroke) =
        if open > 0.02 { (alpha(col, (0.35 + 0.65 * open) as f32), col) } else { (c.panel.0, c.dim.0) };
    b.path(&d, Xf::ID, Some(fill), Some(Pen::new(2.0, stroke).round()));
}

/// A half dial with coloured zones and a needle (the mockup's `needleDial`): zones (from, to, colour), v on lo..hi.
#[allow(clippy::too_many_arguments)]
fn needle_dial(
    b: &Canvas,
    cx: &mut Ctx,
    x: f32,
    y: f32,
    r: f32,
    v: f64,
    lo: f64,
    hi: f64,
    zones: &[(f64, f64, Color32)],
    ic: &str,
    col: Color32,
    tip: String,
) {
    let c = &vg::style().c;
    let a = |q: f64| -100.0 + 200.0 * ((q - lo) / (hi - lo)).clamp(0.0, 1.0);
    b.path(&arc_path(x, y, r, -100.0, 100.0), Xf::ID, None, Some(Pen::new(9.0, rgba(0x1c2734, 1.0))));
    for (z0, z1, zc) in zones {
        b.path(&arc_path(x, y, r, a(*z0), a(*z1)), Xf::ID, None, Some(Pen::new(9.0, *zc)));
    }
    let [nx, ny] = pol(x, y, r + 5.0, a(v));
    let (nx, ny) = (((nx * 10.0).round()) / 10.0, ((ny * 10.0).round()) / 10.0);
    b.seg(x, y, nx, ny, Pen::new(3.0, c.text.0).round());
    b.circle(x, y, 4.5, c.text.0, None);
    b.icon(if col == c.ok.0 { ic } else { "bang" }, x, y + 20.0, 18.0, col);
    cx.hits.add(b, x - r - 8.0, y - r - 8.0, 2.0 * r + 16.0, r + 40.0, None, Some(tip));
}

/// A lever (one of the loop's four controls): the handle at the setting (outlined while the automation holds it,
/// solid in MANUAL), a fill for what it actually does and a hatched gap where that falls short.
#[allow(clippy::too_many_arguments)]
fn lever(
    b: &Canvas,
    cx: &mut Ctx,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    key: &str,
    sp: f64,
    max: f64,
    actual: f64,
    ic: &str,
    tip: String,
    auto: bool,
) {
    let c = &vg::style().c;
    let y_of = |v: f64| y + h * (1.0 - (v.clamp(0.0, max) / max) as f32);
    let lx = x + w / 2.0;
    let off = actual < 0.02 && sp > 0.02;
    let col = if off {
        c.faint.0
    } else if actual > 1.001 {
        c.warn.0
    } else {
        c.ok.0
    };
    b.rect(x, y, w, h, 6.0, rgba(0x101822, 1.0), Some(Pen::new(1.0, c.line2.0)));
    if max > 1.0 {
        b.seg(x - 5.0, y_of(1.0), x + w + 5.0, y_of(1.0), Pen::new(1.0, c.dim.0).dash(3.0, 3.0, 0.0));
    }
    if actual < sp - 0.03 {
        short(b, x + 4.0, y_of(sp), w - 8.0, y_of(actual) - y_of(sp));
    }
    b.rect(x + 4.0, y_of(actual), w - 8.0, y + h - y_of(actual), 3.0, alpha(col, 0.9), None);
    let hy = y_of(sp);
    if auto {
        b.rect(lx - 17.0, hy - 6.0, 34.0, 12.0, 4.0, rgba(0x101822, 1.0), Some(Pen::new(2.0, c.text.0)));
    } else {
        b.rect(lx - 18.0, hy - 7.0, 36.0, 14.0, 4.0, c.text.0, None);
    }
    cx.hits.add_track(
        b,
        x - 6.0,
        y - 7.0,
        w + 12.0,
        h + 14.0,
        [x, y, w, h],
        Hit::Drag(Drag::Lever(key.to_owned())),
        &tip,
    );
    b.icon(ic, lx, y + h + 16.0, 20.0, if off { c.danger.0 } else { c.text.0 });
}

/// A pump on the loop: a ring turning at its speed, dashed when it has stopped.
#[allow(clippy::too_many_arguments)]
fn pump(b: &Canvas, cx: &mut Ctx, x: f32, y: f32, r: f32, p: &super::PumpView, t: f64, small: bool) {
    let c = &vg::style().c;
    let col = if p.speed < 0.05 {
        c.danger.0
    } else if p.capability < 1.0 {
        c.warn.0
    } else {
        c.text.0
    };
    let a = ((t * 330.0 * p.speed) % 360.0).round() as f32;
    let dashed = if small { p.speed <= 0.05 } else { !(p.speed > 0.05 && p.capability >= 1.0) };
    let mut ring = Pen::new(if small { 2.2 } else { 2.5 }, col);
    if dashed {
        ring = if small { ring.dash(3.0, 3.0, 0.0) } else { ring.dash(4.0, 3.0, 0.0) };
    }
    b.circle(x, y, r, c.panel.0, Some(ring));
    let (l, s, d, sw) = if small { (5.0, 3.0, 6.0, 2.0) } else { (7.0, 4.0, 8.0, 2.2) };
    let blades = format!("M0 0l{l} -{s}M0 0l-{l} -{s}M0 0v{d}");
    b.path(&blades, Xf::tr(x, y).rot(a), None, Some(Pen::new(sw, col).round()));
    let tip = format!("{}: {}% speed, {:.2} MW", p.name, kit::js_round(p.speed * 100.0), p.alloc_mw);
    cx.hits.add(b, x - r, y - r, 2.0 * r, 2.0 * r, None, Some(tip));
}

fn reactor(cv: &Canvas, cx: &mut Ctx, e: &EngView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "REACTOR", grid(6.0, 0.0, 4.0, 4.0));
    let (r, cr) = (&e.reactor, &e.cool);
    let run = r.up;
    let auto = cr.mode == "auto";
    let t = cx.v.t;
    let (lx, rx, ty, by, rc, mx, yb) = (34.0_f32, 202.0_f32, 78.0_f32, 296.0_f32, 22.0_f32, 118.0_f32, 252.0_f32);
    let fl = cr.flow.min(1.2);
    let hc = hot_col(cr.hot_k);
    let through = cr.chiller;
    let cold = rgba(COLD, 1.0);
    let flow_tip = format!("Flow {:.0} kg/s ({}%)", cr.flow_kg_s, kit::js_round(cr.flow * 100.0));
    // The legs: the hot leg from the core to the bypass tee, then what the chiller takes; the cold leg back.
    pipe(
        &b,
        cx,
        &format!("M{} {ty}H{}Q{rx} {ty} {rx} {}V{yb}", mx + 46.0, rx - rc, ty + rc),
        hc,
        fl,
        t,
        format!("Hot leg {:.0} K · {flow_tip}", cr.hot_k),
    );
    pipe(
        &b,
        cx,
        &format!("M{rx} {yb}V{}Q{rx} {by} {} {by}H{}", by - rc, rx - rc, mx + 38.0),
        hc,
        fl * through,
        t,
        format!("Hot leg to the chiller: {}%", kit::js_round(through * 100.0)),
    );
    pipe(
        &b,
        cx,
        &format!("M{} {by}H{}Q{lx} {by} {lx} {}V{yb}", mx - 38.0, lx + rc, by - rc),
        cold,
        fl * through,
        t,
        format!("Cold leg {:.0} K", cr.cold_k),
    );
    pipe(
        &b,
        cx,
        &format!("M{lx} {yb}V{}Q{lx} {ty} {} {ty}H{}", ty + rc, lx + rc, mx - 46.0),
        cold,
        fl,
        t,
        format!("Cold leg {:.0} K · {flow_tip}", cr.cold_k),
    );
    // The bypass: what the chiller does not take goes straight across.
    pipe(
        &b,
        cx,
        &format!("M{rx} {yb}H{lx}"),
        mix_col(hc, cold, 0.5),
        fl * (1.0 - through),
        t,
        format!("Bypass: {}%", kit::js_round((1.0 - through) * 100.0)),
    );
    valve(&b, mx, yb, 1.0 - through, c.text.0);
    // The core pumps on the cold leg, each turning at its speed.
    for (k, p) in cr.pumps.iter().enumerate() {
        pump(&b, cx, lx, 128.0 + k as f32 * 46.0, 13.0, p, t, false);
    }
    // The tanks, each filled to its level, feeding the cold leg through the makeup valve.
    let mv = cr.makeup_valve;
    b.path(
        &format!("M72 228V238M94 228V238M{} 238H94", lx + 6.0),
        Xf::ID,
        None,
        Some(Pen::new(3.0, alpha(cold, if mv > 0.02 { 0.9 } else { 0.35 }))),
    );
    for (k, tk) in cr.tanks.iter().enumerate() {
        let (x, top, hh) = (64.0 + k as f32 * 22.0, 150.0_f32, 78.0_f32);
        b.rect(x, top, 16.0, hh, 4.0, rgba(0x101822, 1.0), Some(Pen::new(1.0, c.line2.0)));
        let fr = tk.frac as f32;
        b.rect(x + 2.0, top + 2.0 + (hh - 4.0) * (1.0 - fr), 12.0, (hh - 4.0) * fr, 2.0, alpha(cold, 0.75), None);
        cx.hits.add(&b, x, top, 16.0, hh, None, Some(format!("{}: {} kg", tk.name, thousands(tk.kg))));
    }
    valve(&b, 52.0, 238.0, mv, cold);
    let mtip = format!(
        "Makeup {} · loop {}% full",
        if mv > 0.02 { format!("open, {:.1} kg/s", cr.makeup_kg_s) } else { "shut".into() },
        kit::js_round(cr.inventory * 100.0)
    );
    cx.hits.add(&b, 44.0, 231.0, 16.0, 14.0, None, Some(mtip));
    // The loop's own fill: a drop by the tanks' feet when the loop is short of coolant (cavitation below 80%).
    if cr.inventory < 0.95 {
        b.icon("drop", 118.0, 200.0, 20.0, if cr.inventory < 0.8 { c.danger.0 } else { c.warn.0 });
        cx.hits.add(
            &b,
            108.0,
            190.0,
            20.0,
            20.0,
            None,
            Some(format!("Loop {}% full", kit::js_round(cr.inventory * 100.0))),
        );
    }
    // The chiller (the heat exchanger) and, beyond it, the radiator pumps and the radiators.
    let ex_col = if cr.exchanger < 1.0 { c.warn.0 } else { c.text.0 };
    b.rect(mx - 38.0, by - 16.0, 76.0, 32.0, 6.0, c.panel2.0, Some(Pen::new(2.0, ex_col)));
    b.icon("chiller", mx, by, 22.0, ex_col);
    let ctip = format!(
        "Chiller: {}% through · radiators {:.1} of {:.1} MW",
        kit::js_round(through * 100.0),
        cr.rad_mw,
        cr.rad_cap_mw
    );
    cx.hits.add(&b, mx - 38.0, by - 16.0, 76.0, 32.0, None, Some(ctip));
    let rf = cr.rad_flow.min(1.2);
    pipe(
        &b,
        cx,
        &format!("M{} {}V348H226", mx - 26.0, by + 16.0),
        rgba(RADC, 1.0),
        rf,
        t,
        format!("Radiator loop: {}%", kit::js_round(cr.rad_flow * 100.0)),
    );
    for (k, p) in cr.radiator_pumps.iter().enumerate() {
        pump(&b, cx, 110.0 + k as f32 * 30.0, 348.0, 10.0, p, t, true);
    }
    let ro = (0.3 + 0.7 * (cr.rad_mw / 40.0).clamp(0.0, 1.0)) as f32;
    for k in 0..7 {
        let x = 172.0 + k as f32 * 9.0;
        b.seg(x, 330.0, x, 366.0, Pen::new(3.0, alpha(rgba(RADC, 1.0), (ro * 100.0).round() / 100.0)).round());
    }
    cx.hits.add(
        &b,
        168.0,
        328.0,
        64.0,
        40.0,
        None,
        Some(format!("Radiators {:.1} of {:.1} MW", cr.rad_mw, cr.rad_cap_mw)),
    );
    // The core at the top of the ring: the output dial, the target as a dot, the blanket's heat beside it.
    let a = |k: f64| -120.0 + 240.0 * (k / r.throttle_max).clamp(0.0, 1.0);
    b.circle(mx, ty, 47.0, c.panel.0, Some(Pen::new(1.0, c.line2.0)));
    b.path(&arc_path(mx, ty, 37.0, -120.0, 120.0), Xf::ID, None, Some(Pen::new(10.0, rgba(0x1c2734, 1.0))));
    b.path(&arc_path(mx, ty, 37.0, a(1.0), 120.0), Xf::ID, None, Some(Pen::new(3.0, alpha(c.warn.0, 0.6))));
    if r.throttle > 0.005 {
        let col = if run { c.ok.0 } else { c.emergency.0 };
        b.path(&arc_path(mx, ty, 37.0, -120.0, a(r.throttle)), Xf::ID, None, Some(Pen::new(10.0, col)));
        cx.hits.add(&b, mx - 42.0, ty - 42.0, 84.0, 60.0, None, Some(format!("Output {:.1} MW", r.p_e_mw)));
    }
    if run {
        let [tx, tyy] = pol(mx, ty, 47.0, a(r.target));
        b.circle((tx * 10.0).round() / 10.0, (tyy * 10.0).round() / 10.0, 4.5, c.text.0, None);
        cx.hits.add(
            &b,
            tx - 5.0,
            tyy - 5.0,
            10.0,
            10.0,
            None,
            Some(format!("Target {}%", kit::js_round(r.target * 100.0))),
        );
    }
    let word = if run { format!("{}%", kit::js_round(r.throttle * 100.0)) } else { "SCRAM".into() };
    b.text(
        mx,
        ty + if run { 8.0 } else { 6.0 },
        if run { 24.0 } else { 17.0 },
        if run { c.text.0 } else { c.emergency.0 },
        &word,
        T::mid().bold(),
    );
    if let (false, Some(cause)) = (run, r.scram_cause.as_ref()) {
        let up: String = cause.to_uppercase().chars().take(22).collect();
        b.text(mx, ty + 62.0, 11.0, c.dim.0, &up, T::mid().ls(1.0));
    }
    let top = r.scram_k * 1.08 - 300.0;
    let bk = (r.t_k - 300.0) / top;
    let bcol = if r.t_k >= r.scram_k {
        c.danger.0
    } else if r.t_k >= r.warn_k {
        c.warn.0
    } else {
        c.ok.0
    };
    bar(&b, 214.0, 18.0, 14.0, 96.0, bk, bcol, true);
    let sy = 18.0 + 96.0 * (1.0 - ((r.scram_k - 300.0) / top) as f32);
    b.seg(210.0, sy, 232.0, sy, Pen::new(2.0, c.danger.0));
    b.icon("flame", 221.0, 130.0, 16.0, bcol);
    cx.hits.add(&b, 210.0, 18.0, 22.0, 120.0, None, Some(format!("Reactor heat: {} K", kit::js_round(r.t_k))));
    // Right: SCRAM and the reactor's mode.
    let x0 = 244.0_f32;
    let rw = w - x0 - 10.0;
    let armed = e.armed_scram;
    button(
        &b,
        cx,
        Btn {
            x: x0,
            y: 8.0,
            w: rw / 2.0 - 4.0,
            h: 48.0,
            label: Some(if armed { "CONFIRM" } else { "SCRAM" }),
            act: run.then_some(Act::Scram),
            off: !run,
            col2: Some(c.danger.0),
            stroke: Some(alpha(c.danger.0, 0x88 as f32 / 255.0)),
            on: armed,
            col: Some(c.danger.0),
            fill: Some(rgba(0xff4757, 0.25)),
            fs: Some(14.0),
            tip: Some(if run { "Scram: tap, then tap again" } else { "Restart at the reactor" }.into()),
            ..Default::default()
        },
    );
    button(
        &b,
        cx,
        Btn {
            x: x0 + rw / 2.0 + 4.0,
            y: 8.0,
            w: rw / 2.0 - 4.0,
            h: 48.0,
            icon: Some("core"),
            label: Some(if r.mode == "auto" { "AUTO" } else { "MANUAL" }),
            stack: true,
            is: Some(20.0),
            fs: Some(12.0),
            act: Some(Act::RxMode),
            tip: Some(
                if r.mode == "auto" { "Reactor follows the load (AUTO)" } else { "Reactor throttle by hand (MANUAL)" }
                    .into(),
            ),
            ..Default::default()
        },
    );
    // The two needles that matter: the hot leg against its band and limit; the heat against what the flow carries.
    let hs = hot_state(cr);
    let ratio = cr.heat_mw / cr.carry_mw.max(1e-6);
    let hot_zones = [
        (cr.hot_band_k[0], cr.hot_band_k[1], c.ok.0),
        (cr.hot_warn_k, cr.limit_k, c.warn.0),
        (cr.limit_k, 400.0, c.danger.0),
    ];
    let htip = format!(
        "Hot leg {:.0} K (band {}-{}, scram {}) · cold leg {:.0} K",
        cr.hot_k,
        num(cr.hot_band_k[0]),
        num(cr.hot_band_k[1]),
        num(cr.limit_k),
        cr.cold_k
    );
    needle_dial(&b, cx, x0 + rw * 0.25, 100.0, 30.0, cr.hot_k, 300.0, 400.0, &hot_zones, "temp", hs, htip);
    let rcol = if ratio > 1.2 {
        c.danger.0
    } else if ratio > 1.0 {
        c.warn.0
    } else {
        c.ok.0
    };
    let flow_zones = [(0.0, 1.0, alpha(c.ok.0, 0x66 as f32 / 255.0)), (1.0, 1.2, c.warn.0), (1.2, 1.5, c.danger.0)];
    let ftip = format!("Heat {:.1} MW · the flow carries {:.1} MW", cr.heat_mw, cr.carry_mw);
    needle_dial(&b, cx, x0 + rw * 0.75, 100.0, 30.0, ratio, 0.0, 1.5, &flow_zones, "drop", rcol, ftip);
    // The four controls, and the loop's AUTO.
    let (lw, step, ly, lh) = (24.0_f32, rw / 4.0, 150.0_f32, 150.0_f32);
    let avg = |ps: &[super::PumpView], f: &dyn Fn(&super::PumpView) -> f64| {
        ps.iter().map(f).sum::<f64>() / ps.len().max(1) as f64
    };
    let pump_sp = avg(&cr.pumps, &|p| p.setpoint);
    let pump_act = avg(&cr.pumps, &|p| p.speed * p.capability.min(1.0));
    let rad_sp = avg(&cr.radiator_pumps, &|p| p.setpoint);
    let rad_act = avg(&cr.radiator_pumps, &|p| p.speed * p.capability.min(1.0));
    let lev_x = |k: f32| x0 + step * k - lw / 2.0;
    lever(
        &b,
        cx,
        lev_x(0.5),
        ly,
        lw,
        lh,
        "pumps",
        pump_sp,
        cr.pump_max,
        pump_act,
        "pump",
        format!("Core pumps: {}% set, {}% running", kit::js_round(pump_sp * 100.0), kit::js_round(pump_act * 100.0)),
        auto,
    );
    lever(
        &b,
        cx,
        lev_x(1.5),
        ly,
        lw,
        lh,
        "chiller",
        through,
        1.0,
        through,
        "chiller",
        format!("Chiller: {}% of the hot leg through", kit::js_round(through * 100.0)),
        auto,
    );
    lever(
        &b,
        cx,
        lev_x(2.5),
        ly,
        lw,
        lh,
        "radiators",
        rad_sp,
        cr.rad_max,
        rad_act,
        "radiator",
        format!("Radiator pumps: {}% set, {}% running", kit::js_round(rad_sp * 100.0), kit::js_round(rad_act * 100.0)),
        auto,
    );
    let mk_act = if cr.inventory >= 1.0 { mv } else { cr.makeup_kg_s / cr.makeup_max_kg_s };
    lever(
        &b,
        cx,
        lev_x(3.5),
        ly,
        lw,
        lh,
        "makeup",
        mv,
        1.0,
        mk_act,
        "makeup",
        format!("Makeup valve: {}% open, {:.1} kg/s", kit::js_round(mv * 100.0), cr.makeup_kg_s),
        auto,
    );
    button(
        &b,
        cx,
        Btn {
            x: x0,
            y: h - 40.0,
            w: rw,
            h: 34.0,
            icon: Some("cooling"),
            label: Some(if auto { "AUTO" } else { "MANUAL" }),
            is: Some(18.0),
            fs: Some(14.0),
            act: Some(Act::CMode),
            on: !auto,
            tip: Some(
                if auto { "The loop runs itself: tap for MANUAL" } else { "The loop is yours: tap for AUTO" }.into(),
            ),
            ..Default::default()
        },
    );
}

/// A number as JavaScript prints it in a template: no trailing `.0`.
fn num(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

/// A whole number with thousands separators (`toLocaleString("en")`).
fn thousands(x: f64) -> String {
    let n = kit::js_round(x);
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

// ------------------------------------------------------------------------------------------------ BUSES

fn buses(cv: &Canvas, cx: &mut Ctx, e: &EngView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "BUSES", grid(10.0, 0.0, 2.0, 2.0));
    let bs = &e.buses;
    let live = |on: bool| if on { c.ok.0 } else { c.faint.0 };
    let up = e.reactor.up;
    b.icon("bolt", w / 2.0, 18.0, 18.0, if up { c.text.0 } else { c.emergency.0 });
    b.path(
        &format!("M{} 28V38M44 38H{}M44 38V60M{} 38V60", w / 2.0, w - 44.0, w - 44.0),
        Xf::ID,
        None,
        Some(Pen::new(3.0, live(up))),
    );
    let brk = |cx: &mut Ctx, x: f32, y: f32, on: bool, id: &str, tip: &str| {
        b.rect(
            x - 10.0,
            y - 10.0,
            20.0,
            20.0,
            3.0,
            if on { c.ok.0 } else { Color32::TRANSPARENT },
            Some(Pen::new(2.5, if on { c.ok.0 } else { c.warn.0 })),
        );
        let key = format!("brk:{id}");
        if let Some(hv) = cx.v.hold.as_ref().filter(|h| h.key == key) {
            b.rect(x - 10.0, y - 10.0, 20.0 * hv.k.clamp(0.0, 1.0) as f32, 20.0, 0.0, alpha(c.warn.0, 0.6), None);
        }
        let t = format!("{tip}{}", if on { ": hold to open" } else { ": hold to close" });
        cx.hits.add(&b, x - 10.0, y - 10.0, 20.0, 20.0, Some(Hit::Hold(key)), Some(t));
    };
    brk(cx, 44.0, 64.0, bs.gen_p, "gen_p", "Port generator breaker");
    brk(cx, w - 44.0, 64.0, bs.gen_s, "gen_s", "Starboard generator breaker");
    let bus = |x: f32, ok: bool, on: bool| {
        let fill = if !ok {
            c.danger.0
        } else if on {
            c.ok.0
        } else {
            c.faint.0
        };
        b.rect(x - 34.0, 84.0, 68.0, 10.0, 3.0, fill, None);
        if !ok {
            b.icon("bang", x, 104.0, 14.0, c.danger.0);
        }
    };
    bus(44.0, bs.np, bs.gen_p);
    bus(w - 44.0, bs.ns, bs.gen_s);
    let tie = Pen::new(3.0, live(bs.tie));
    b.seg(78.0, 89.0, w - 78.0, 89.0, if bs.tie { tie } else { tie.dash(4.0, 4.0, 0.0) });
    brk(cx, w / 2.0, 89.0, bs.tie, "bt_main", "Bus tie");
    b.path(&format!("M44 94V120H{}V94", w - 44.0), Xf::ID, None, Some(Pen::new(2.0, c.faint.0)));
    brk(cx, 70.0, 120.0, bs.eb_p, "eb_p", "Emergency bus, port");
    brk(cx, w - 70.0, 120.0, bs.eb_s, "eb_s", "Emergency bus, starboard");
    // The battery's charge: a fill beside its icon.
    let soc = bs.soc;
    b.icon("battery", w / 2.0 - 22.0, 140.0, 18.0, if e.bat_out_mw > 0.05 { c.emergency.0 } else { c.dim.0 });
    bar(&b, w / 2.0 - 8.0, 135.0, 46.0, 10.0, soc, if soc > 0.3 { c.ok.0 } else { c.warn.0 }, false);
    cx.hits.add(&b, w / 2.0 - 32.0, 130.0, 74.0, 20.0, None, Some(format!("Battery {}%", kit::js_round(soc * 100.0))));
    if bs.cut > 0 {
        b.icon("breach", 18.0, h - 14.0, 14.0, c.danger.0);
        b.text(30.0, h - 9.0, 13.0, c.danger.0, &bs.cut.to_string(), T::default());
        cx.hits.add(&b, 10.0, h - 22.0, 34.0, 18.0, None, Some("Cables cut: damage control splices them".into()));
    }
}

// ------------------------------------------------------------------------------------------------ AIR

fn air(cv: &Canvas, cx: &mut Ctx, e: &EngView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "AIR", grid(10.0, 2.0, 2.0, 2.0));
    let rows = [("air_o2", "O₂"), ("air_co2", "CO₂"), ("temp", "Temperature")];
    for (k, (ic, tip)) in rows.iter().enumerate() {
        let st = e.air.status[k];
        let col = [c.ok.0, c.warn.0, c.danger.0][usize::from(st.min(2))];
        let y = 12.0 + k as f32 * 40.0;
        b.rect(
            12.0,
            y,
            w - 24.0,
            32.0,
            7.0,
            alpha(col, 0x18 as f32 / 255.0),
            Some(Pen::new(1.0, alpha(col, 0x66 as f32 / 255.0))),
        );
        b.icon(ic, 36.0, y + 16.0, 20.0, col);
        b.icon(if st > 0 { "bang" } else { "check" }, w - 36.0, y + 16.0, 18.0, col);
        cx.hits.add(&b, 12.0, y, w - 24.0, 32.0, None, Some((*tip).into()));
    }
    if let Some(worst) = &e.air.worst {
        b.text(w / 2.0, h - 10.0, 13.0, c.warn.0, worst, T::mid().ls(1.0));
    }
}

// ------------------------------------------------------------------------------------------------ unavailable

/// The console with nothing aboard to drive it: the four panels, their controls drawn as the mockup draws a control
/// that cannot act now (faint, no action), and the reason in each tip.
fn unavailable(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let tip = "Not simulated in this drill: the power grid is not built yet";
    for (title, g) in [
        ("POWER", grid(0.0, 0.0, 6.0, 4.0)),
        ("REACTOR", grid(6.0, 0.0, 4.0, 4.0)),
        ("BUSES", grid(10.0, 0.0, 2.0, 2.0)),
        ("AIR", grid(10.0, 2.0, 2.0, 2.0)),
    ] {
        let (b, w, h) = panel(cv, cx, title, g);
        match title {
            "POWER" => {
                b.icon("bolt", 26.0, 22.0, 20.0, c.faint.0);
                b.rect(50.0, 12.0, w - 200.0, 20.0, 4.0, rgba(0x16202c, 1.0), None);
                b.text(w - 16.0, 28.0, 18.0, c.faint.0, "-- MW", T::end());
                let pre = ["CRUISE", "COMBAT", "SILENT", "EMERG"];
                let bw = (w - 24.0 - 3.0 * 8.0) / 4.0;
                for (i, label) in pre.iter().enumerate() {
                    button(
                        &b,
                        cx,
                        Btn {
                            x: 12.0 + i as f32 * (bw + 8.0),
                            y: h - 42.0,
                            w: bw,
                            h: 34.0,
                            label: Some(label),
                            off: true,
                            fs: Some(14.0),
                            ..Default::default()
                        },
                    );
                }
            }
            "REACTOR" => {
                b.circle(118.0, 78.0, 47.0, c.panel.0, Some(Pen::new(1.0, c.line2.0)));
                b.path(
                    &arc_path(118.0, 78.0, 37.0, -120.0, 120.0),
                    Xf::ID,
                    None,
                    Some(Pen::new(10.0, rgba(0x1c2734, 1.0))),
                );
                b.text(118.0, 86.0, 24.0, c.faint.0, "--", T::mid().bold());
                let rw = w - 244.0 - 10.0;
                button(
                    &b,
                    cx,
                    Btn {
                        x: 244.0,
                        y: 8.0,
                        w: rw / 2.0 - 4.0,
                        h: 48.0,
                        label: Some("SCRAM"),
                        off: true,
                        fs: Some(14.0),
                        ..Default::default()
                    },
                );
                button(
                    &b,
                    cx,
                    Btn {
                        x: 244.0,
                        y: h - 40.0,
                        w: rw,
                        h: 34.0,
                        icon: Some("cooling"),
                        label: Some("AUTO"),
                        is: Some(18.0),
                        fs: Some(14.0),
                        off: true,
                        ..Default::default()
                    },
                );
            }
            "BUSES" => b.icon("bolt", w / 2.0, 18.0, 18.0, c.faint.0),
            _ => {
                for (k, ic) in ["air_o2", "air_co2", "temp"].iter().enumerate() {
                    let y = 12.0 + k as f32 * 40.0;
                    b.rect(12.0, y, w - 24.0, 32.0, 7.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.line2.0)));
                    b.icon(ic, 36.0, y + 16.0, 20.0, c.faint.0);
                }
            }
        }
        cx.hits.add(&b, 0.0, 0.0, w, h, None, Some(tip.into()));
    }
}
