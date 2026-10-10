//! The helm console (the mockup's `helm()`): THRUST, SCANNER, ATTITUDE and ORIENT on the 104 x 102 grid.

use super::kit::{self, Act, Btn, Drag, Hit, Scan};
use super::{accent, fmt_ang, fmt_q, from_euler, grid, quat, quat_arr, to_euler, Ctx};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::Color32;

/// A panel (the mockup's `panel`): the box, the accent tick, the title, and the body's canvas below the title band,
/// clipped to the box (a panel never draws outside it, CLAUDE.md 10).
pub fn panel(cv: &Canvas, cx: &Ctx, title: &str, g: [f32; 4]) -> (Canvas, f32, f32) {
    let c = &vg::style().c;
    let [x, y, w, h] = g;
    let pc = cv.at(x, y).clip(0.0, 0.0, w, h);
    pc.rect(0.5, 0.5, w - 1.0, h - 1.0, 9.0, c.panel.0, Some(Pen::new(1.0, c.line.0)));
    pc.rect(12.0, 9.0, 3.0, 10.0, 1.0, accent(cx.v), None);
    pc.text(21.0, 18.0, 13.0, c.dim.0, title, T::default().ls(2.0).bold());
    (pc.at(0.0, 24.0), w, h - 24.0)
}

/// The diagonal hatch the mockup fills a reverse range with: stripes 4 wide every 8, at 45 degrees.
pub fn hatch(cv: &Canvas, x: f32, y: f32, w: f32, h: f32, col: Color32) {
    hatch_p(cv, x, y, w, h, col, 8.0, 4.0);
}

/// A diagonal hatch: an SVG pattern `period` square with a stripe `width` wide, turned 45 degrees, in the canvas's
/// user space (so stripes line up across shapes, as the pattern's do). Engineering's shortfall is 6 and 3.
#[allow(clippy::too_many_arguments)]
pub fn hatch_p(cv: &Canvas, x: f32, y: f32, w: f32, h: f32, col: Color32, period: f32, width: f32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let rect = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
    let s2 = std::f32::consts::SQRT_2;
    let (lo, hi) = (x + y, x + w + y + h);
    let mut k = (lo / (period * s2)).floor() - 1.0;
    while k * period * s2 < hi {
        let (a, b) = (k * period * s2, (k * period + width) * s2);
        let band: Vec<[f32; 2]> = clip_band(&rect, a, b);
        if band.len() >= 3 {
            cv.poly(&band, col);
        }
        k += 1.0;
    }
}

fn clip_band(poly: &[[f32; 2]], a: f32, b: f32) -> Vec<[f32; 2]> {
    let half = |p: &[[f32; 2]], f: &dyn Fn([f32; 2]) -> f32| {
        let mut out = Vec::new();
        for i in 0..p.len() {
            let (u, v) = (p[i], p[(i + 1) % p.len()]);
            let (su, sv) = (f(u), f(v));
            if su >= 0.0 {
                out.push(u);
            }
            if (su >= 0.0) != (sv >= 0.0) {
                let t = su / (su - sv);
                out.push([u[0] + (v[0] - u[0]) * t, u[1] + (v[1] - u[1]) * t]);
            }
        }
        out
    };
    let p1 = half(poly, &|q| q[0] + q[1] - a);
    half(&p1, &|q| b - (q[0] + q[1]))
}

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    thrust(cv, cx);
    scanner(cv, cx);
    attitude(cv, cx);
    orient(cv, cx);
}

fn thrust(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "THRUST", grid(0.0, 0.0, 3.0, 4.0));
    let v = cx.v;
    let acc = accent(v);
    let vb = v.v_body;
    b.text(70.0, 40.0, 38.0, c.text.0, &kit::js_round(vb[2]).to_string(), T::mid().bold());
    b.text(70.0, 58.0, 13.0, c.dim.0, "m/s", T::mid());
    let (tx, ty, th, tw) = (44.0f32, 74.0f32, h - 150.0, 52.0f32);
    let (lo, hi) = (v.flight.speed_min_mps, v.flight.speed_max_mps);
    let y_of = |s: f64| ty + th * (1.0 - ((s - lo) / (hi - lo)) as f32);
    b.rect(tx, ty, tw, th, 8.0, rgba(0x101822, 1.0), Some(Pen::new(1.0, c.line2.0)));
    let y0 = y_of(0.0);
    // The bands are cut to the track as polygons, so no clip rectangle (each one costs a UI draw call).
    hatch(&b, tx, y0, tw, ty + th - y0, alpha(rgba(0x2a3442, 1.0), 0.6));
    let yv = y_of(vb[2]);
    b.rect(tx + 6.0, yv.min(y0), tw - 12.0, (y0 - yv).abs(), 4.0, alpha(acc, 0.55), None);
    b.seg(tx - 6.0, y0, tx + tw + 6.0, y0, Pen::new(1.0, c.dim.0));
    let hy = y_of(v.speed_set);
    b.rect(tx - 12.0, hy - 9.0, tw + 24.0, 18.0, 5.0, c.text.0, None);
    b.seg(tx - 4.0, hy, tx + tw + 4.0, hy, Pen::new(2.0, c.bg.0));
    cx.hits.add_track(
        &b,
        tx - 12.0,
        ty - 9.0,
        tw + 24.0,
        th + 18.0,
        [tx, ty, tw, th],
        Hit::Drag(Drag::Throttle),
        "Forward speed: drag (W, S)",
    );
    let (px, py) = (150.0f32, 74.0f32);
    let ps = w - px - 14.0;
    let pc = [px + ps / 2.0, py + ps / 2.0];
    let k = ps / 2.0 / v.flight.strafe_max_mps as f32;
    b.rect(px, py, ps, ps, 10.0, rgba(0x101822, 1.0), Some(Pen::new(1.0, c.line2.0)));
    b.seg(pc[0], py + 8.0, pc[0], py + ps - 8.0, Pen::new(1.0, c.line2.0));
    b.seg(px + 8.0, pc[1], px + ps - 8.0, pc[1], Pen::new(1.0, c.line2.0));
    let arrows = format!(
        "M{} {}l-5 7h10zM{} {}l-5 -7h10zM{} {}l7 -5v10zM{} {}l-7 -5v10z",
        pc[0],
        py + 4.0,
        pc[0],
        py + ps - 4.0,
        px + 4.0,
        pc[1],
        px + ps - 4.0,
        pc[1]
    );
    b.path(&arrows, Xf::ID, Some(c.dim.0), None);
    let sm = v.flight.strafe_max_mps;
    b.circle(pc[0] + (-vb[0]).clamp(-sm, sm) as f32 * k, pc[1] - vb[1].clamp(-sm, sm) as f32 * k, 4.0, acc, None);
    b.circle(
        pc[0] + v.strafe[0] as f32 * k,
        pc[1] - v.strafe[1] as f32 * k,
        13.0,
        Color32::TRANSPARENT,
        Some(Pen::new(3.0, c.text.0)),
    );
    let tip = format!("Strafe: drag (Z, C, Space, Ctrl). Now {:.0} m/s to starboard, {:.0} m/s up", -vb[0], vb[1]);
    cx.hits.add_track(&b, px, py, ps, ps, [px, py, ps, ps], Hit::Drag(Drag::Strafe), &tip);
    b.icon("drive", px + ps / 2.0, py + ps + 22.0, 18.0, c.dim.0);
    kit::button(
        &b,
        cx,
        Btn {
            x: 14.0,
            y: h - 62.0,
            w: w - 28.0,
            h: 50.0,
            icon: Some("stop"),
            label: Some("STOP"),
            act: Some(Act::AllStop),
            col2: Some(c.danger.0),
            stroke: Some(alpha(c.danger.0, 0x88 as f32 / 255.0)),
            tip: Some("All stop: speed and drift to zero (X)".into()),
            ..Default::default()
        },
    );
}

fn scanner(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "SCANNER", grid(3.0, 0.0, 5.0, 4.0));
    let v = cx.v;
    let acc = accent(v);
    let s = Scan::new(v, w as f64, (h - 58.0) as f64, "nav", None);
    s.draw_base(&b);
    kit::scan_wrap(&b, cx, "nav", w, h - 58.0);
    let (scx, scy) = (s.cx as f32, s.cy as f32);
    let bear = v.tubes_bear;
    let [t0x, t0y] = s.dir(-v.tube_cone_deg, 1.0);
    let [t1x, t1y] = s.dir(v.tube_cone_deg, 1.0);
    let cone = [[scx, scy], [t0x as f32, t0y as f32], [t1x as f32, t1y as f32]];
    b.poly(&cone, alpha(if bear { c.ok.0 } else { acc }, if bear { 0.35 } else { 0.1 }));
    if let Some(wp) = v.waypoint {
        let p = s.at(super::v3(wp));
        let (x, y, by) = (p.x as f32, p.y as f32, p.by as f32);
        let op = if v.ap == "COURSE" { 1.0 } else { 0.45 };
        b.seg(scx, scy, x, y, Pen::new(2.0, alpha(acc, op)).dash(6.0, 5.0, 0.0));
        b.seg(x, by, x, y, Pen::new(1.0, acc).dash(2.0, 3.0, 0.0));
        b.path(&format!("M{x} {}l9 9-9 9-9-9z", y - 9.0), Xf::ID, None, Some(Pen::new(2.0, acc)));
        cx.hits.add(&b, x - 9.0, y - 9.0, 18.0, 18.0, None, Some("Waypoint".into()));
    }
    kit::feed_cone(&b, cx, &s);
    kit::ship_glyph(&b, "corvette", "friendly", scx, scy, 22.0, s.hdg as f32, c.text.0, false);
    kit::scanner_contacts(&b, cx, &s, false);
    kit::range_ctl(&b, cx, 12.0, 6.0, "nav");
    kit::scan_reset(&b, cx, w - 46.0, 6.0, "nav");
    let aps = [
        ("HOLD", "hold", "Hold attitude and speed"),
        ("COURSE", "course", "Fly to the waypoint"),
        ("CHASE", "chase", "Point at the target"),
        ("MATCH", "match", "Match the target's course and speed"),
        ("EVADE", "jink", "Jink sideways and up and down"),
    ];
    let bw = (w - 24.0 - 4.0 * 6.0) / 5.0;
    for (i, (k, ic, tp)) in aps.iter().enumerate() {
        let off = *k == "COURSE" && v.waypoint.is_none();
        let tip = if off { "Fly to the waypoint: this mission has none".to_owned() } else { (*tp).to_owned() };
        kit::button(
            &b,
            cx,
            Btn {
                x: 12.0 + i as f32 * (bw + 6.0),
                y: h - 52.0,
                w: bw,
                h: 42.0,
                icon: Some(ic),
                label: Some(k),
                on: v.ap == *k,
                off,
                act: Some(Act::Ap((*k).to_owned())),
                tip: Some(tip),
                fs: Some(13.0),
                is: Some(16.0),
                ..Default::default()
            },
        );
    }
}

fn attitude(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "ATTITUDE", grid(8.0, 0.0, 4.0, 2.0));
    let v = cx.v;
    let acc = accent(v);
    let (eh, ep, er, gimbal) = to_euler(quat(v.q));
    kit::joystick(&b, cx, 76.0, h, v.stick[0], v.stick[1]);
    kit::flipper(&b, cx, 196.0, h, v.stick[2]);
    let rl = v.flight.rate_dps.map(f64::to_radians);
    let rows = [
        ("YAW", fmt_ang(eh, false), -v.w[1] / rl[1], -v.wset[1] / rl[1], 1),
        ("PITCH", fmt_ang(ep, true), -v.w[0] / rl[0], -v.wset[0] / rl[0], 0),
        ("ROLL", fmt_ang(er, true), v.w[2] / rl[2], v.wset[2] / rl[2], 2),
    ];
    let x0 = 250.0;
    let bwid = w - x0 - 12.0;
    for (i, (word, val, rate, set, ax)) in rows.iter().enumerate() {
        let y = 2.0 + i as f32 * 56.0;
        let by = y + 34.0;
        let mid = x0 + bwid / 2.0;
        b.text(x0, y + 18.0, 11.0, c.dim.0, word, T::default().ls(2.0));
        b.text(w - 12.0, y + 24.0, 24.0, c.text.0, val, T::end().bold());
        b.rect(x0, by, bwid, 9.0, 3.0, rgba(0x16202c, 1.0), None);
        let rx = mid + rate.clamp(-1.0, 1.0) as f32 * (bwid / 2.0);
        b.rect(mid.min(rx), by, (rx - mid).abs(), 9.0, 3.0, acc, None);
        let sx = mid + set.clamp(-1.0, 1.0) as f32 * (bwid / 2.0);
        b.seg(sx, by - 3.0, sx, by + 12.0, Pen::new(2.0, alpha(c.text.0, 0.7)));
        b.seg(mid, by - 2.0, mid, by + 11.0, Pen::new(1.0, c.dim.0));
        let tip = format!("{} rate {:.1} deg/s", word.to_lowercase(), rate * v.flight.rate_dps[*ax]);
        cx.hits.add(&b, x0, by - 3.0, bwid, 15.0, None, Some(tip));
    }
    if gimbal {
        b.text(x0, h - 2.0, 11.0, c.warn.0, "ROLL IN YAW", T::default().ls(1.0));
    }
}

fn orient(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, _h) = panel(cv, cx, "ORIENT", grid(8.0, 2.0, 4.0, 2.0));
    let v = cx.v;
    let acc = accent(v);
    let q = quat_arr(from_euler(v.oe.h, v.oe.p, v.oe.r));
    let plan = v.preview;
    let wheels = [
        ("YAW", 'h', v.oe.h, fmt_ang(v.oe.h, false)),
        ("PITCH", 'p', v.oe.p, fmt_ang(v.oe.p, true)),
        ("ROLL", 'r', v.oe.r, fmt_ang(v.oe.r, true)),
    ];
    for (i, (word, k, val, txt)) in wheels.iter().enumerate() {
        let x = 42.0 + i as f32 * 82.0;
        b.text(x, 11.0, 11.0, c.dim.0, word, T::mid().ls(2.0));
        kit::thumbwheel(&b, cx, x, 16.0, 84.0, *val, *k, word);
        b.text(x, 125.0, 22.0, c.text.0, txt, T::mid().bold());
    }
    b.rect(8.0, 136.0, 232.0, 26.0, 5.0, rgba(0x0a1017, 1.0), Some(Pen::new(1.0, c.line.0)));
    // The mockup names a monospace face here, but its page's CSS sets every text's family, so it reads in Barlow.
    b.text(124.0, 154.0, 13.0, acc, &fmt_q(q), T::mid().ls(0.0));
    cx.hits.add(
        &b,
        8.0,
        136.0,
        232.0,
        26.0,
        None,
        Some("The order as a quaternion: w, x, y, z (ship axes to reference axes)".into()),
    );
    let active = v.order_active;
    let held = active && v.plan.is_some_and(|p| p.held);
    let go = if active {
        if held {
            "HELD".to_owned()
        } else {
            format!("{} s", v.plan.map(|p| p.time).unwrap_or(plan.time).ceil() as i64)
        }
    } else {
        "GO".to_owned()
    };
    let tip = if active {
        "Holding this attitude; the stick cancels it".to_owned()
    } else {
        format!("Turn there: {:.0}° in {:.0} s", plan.angle, plan.time)
    };
    kit::button(
        &b,
        cx,
        Btn {
            x: 250.0,
            y: 2.0,
            w: w - 260.0,
            h: 68.0,
            label: Some(&go),
            act: Some(Act::OGo),
            on: active,
            fs: Some(24.0),
            tip: Some(tip),
            ..Default::default()
        },
    );
    if !active {
        b.text(250.0 + (w - 260.0) / 2.0, 62.0, 12.0, c.dim.0, &format!("{:.0} s", plan.time), T::mid());
    }
    let qb = [
        ("LEVEL", Act::OLevel, "Pitch and roll to zero"),
        ("FLIP", Act::OFlip, "Turn about to the reciprocal heading"),
        ("TARGET", Act::OTarget, "Point the bow at the target"),
    ];
    for (i, (k, a, tp)) in qb.into_iter().enumerate() {
        let off = matches!(a, Act::OTarget) && v.target.is_none();
        kit::button(
            &b,
            cx,
            Btn {
                x: 250.0,
                y: 76.0 + i as f32 * 26.0,
                w: w - 260.0,
                h: 24.0,
                label: Some(k),
                act: Some(a),
                fs: Some(12.0),
                off,
                tip: Some(tp.into()),
                ..Default::default()
            },
        );
    }
}
