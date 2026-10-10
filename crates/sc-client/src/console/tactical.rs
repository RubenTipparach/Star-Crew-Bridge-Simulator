//! The tactical console (the mockup's `tactical()`): TARGETS, PLOT, TURRETS and TUBES on the 104 x 102 grid.

use super::helm::panel;
use super::kit::{self, range_km, Act, Btn, Hit, Scan};
use super::{grid, iff_col, v3, Ctx};
use crate::vg::{self, alpha, rgba, Canvas, Pen, T};
use egui::Color32;

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    targets(cv, cx);
    plot(cv, cx);
    turrets(cv, cx);
    tubes(cv, cx);
}

/// Points along a circle from bearing a0 to a1 (degrees, 0 up, clockwise): the mockup's `arcPath`.
fn arc_pts(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<[f32; 2]> {
    let n = (((a1 - a0).abs() / 4.0).ceil() as usize).max(2);
    (0..=n)
        .map(|i| {
            let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
            [cx + r * a.sin(), cy - r * a.cos()]
        })
        .collect()
}

fn targets(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, _h) = panel(cv, cx, "TARGETS", grid(0.0, 0.0, 3.0, 4.0));
    let v = cx.v;
    let mut list: Vec<&super::ContactView> = v.contacts.iter().filter(|k| k.hull > 0.0).collect();
    list.sort_by(|a, b| {
        let h = |k: &super::ContactView| if k.iff == "hostile" { 0 } else { 1 };
        h(a).cmp(&h(b)).then(range_km(a.rel).total_cmp(&range_km(b.rel)))
    });
    let mut y = 6.0;
    for k in list.into_iter().take(4) {
        let sel = Some(&k.id) == v.target.as_ref();
        let col = iff_col(&k.iff);
        let (fill, stroke) = if sel {
            (alpha(col, 0x1f as f32 / 255.0), Pen::new(2.0, col))
        } else {
            (c.panel2.0, Pen::new(1.0, c.line2.0))
        };
        b.rect(10.0, y, w - 20.0, 80.0, 8.0, fill, Some(stroke));
        kit::ship_glyph(&b, &k.cls, &k.iff, 44.0, y + 40.0, 36.0, 0.0, col, false);
        b.text(80.0, y + 34.0, 24.0, c.text.0, &k.id, T::default().bold());
        b.text(w - 22.0, y + 34.0, 18.0, c.text.0, &format!("{:.1} km", range_km(k.rel)), T::end());
        if k.scan >= 0.3 {
            kit::bar(
                &b,
                80.0,
                y + 50.0,
                w - 102.0,
                8.0,
                k.hull / 100.0,
                if k.hull > 50.0 { col } else { c.warn.0 },
                false,
            );
        } else {
            kit::bar(&b, 80.0, y + 50.0, w - 102.0, 8.0, 0.0, c.faint.0, false);
            b.icon("scan", w - 30.0, y + 54.0, 14.0, c.faint.0);
            cx.hits.add(&b, 80.0, y + 46.0, w - 102.0, 16.0, None, Some("Hull unknown until Science scans it".into()));
        }
        cx.hits.add(
            &b,
            10.0,
            y,
            w - 20.0,
            80.0,
            Some(Hit::Act(Act::Target(k.id.clone()))),
            Some(format!("{}, {}", k.cls, k.iff)),
        );
        y += 88.0;
    }
}

fn plot(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "PLOT", grid(3.0, 0.0, 5.0, 4.0));
    let v = cx.v;
    let s = Scan::new(v, w as f64, (h - 52.0) as f64, "tac", None);
    s.draw_base(&b);
    kit::scan_wrap(&b, cx, "tac", w, h - 52.0);
    kit::feed_cone(&b, cx, &s);
    kit::shield_view(&b, cx, s.cx, s.cy, 1.25, s.yaw, s.el, None);
    for m in &v.missiles {
        let p = s.at(v3(*m));
        if p.inside {
            b.circle(p.x as f32, p.y as f32, 3.5, c.ok.0, None);
        }
    }
    let tgt = v.target.as_ref().and_then(|id| v.contacts.iter().find(|k| &k.id == id && k.hull > 0.0));
    let firing = v.turrets.iter().any(|t| t.firing && t.mode != "PD");
    if let Some(t) = tgt {
        let p = s.at(v3(t.rel));
        if p.inside {
            let pen = if firing {
                Pen::new(2.5, alpha(c.ok.0, 0.8)).dash(10.0, 4.0, 0.0)
            } else {
                Pen::new(1.5, alpha(c.hostile.0, 0.8)).dash(3.0, 5.0, 0.0)
            };
            b.seg(s.cx as f32, s.cy as f32, p.x as f32, p.y as f32, pen);
        }
    }
    kit::scanner_contacts(&b, cx, &s, false);
    kit::range_ctl(&b, cx, 8.0, 4.0, "tac");
    kit::scan_reset(&b, cx, w - 42.0, 4.0, "tac");
    let pre = [
        ("balance", "Balanced", -1),
        ("fwd", "Bow", 0),
        ("aft", "Stern", 1),
        ("port", "Port", 2),
        ("stbd", "Starboard", 3),
    ];
    let bw = (w - 20.0 - 4.0 * 6.0) / 5.0;
    for (i, (ic, tip, f)) in pre.iter().enumerate() {
        kit::button(
            &b,
            cx,
            Btn {
                x: 10.0 + i as f32 * (bw + 6.0),
                y: h - 46.0,
                w: bw,
                h: 38.0,
                icon: Some(ic),
                act: Some(Act::Favour(*f)),
                tip: Some(format!("Shields: {tip}")),
                is: Some(20.0),
                on: v.preset == *f,
                ..Default::default()
            },
        );
    }
}

fn turrets(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let c = &st.c;
    let (b, w, h) = panel(cv, cx, "TURRETS", grid(8.0, 0.0, 4.0, 2.0));
    let v = cx.v;
    let n = v.turrets.len().max(1) as f32;
    let tw = (w - 20.0 - (n - 1.0) * 6.0) / n;
    for (i, t) in v.turrets.iter().enumerate() {
        let x = 10.0 + i as f32 * (tw + 6.0);
        let (tcx, tcy) = (x + tw / 2.0, 52.0);
        let hc = if t.cooling {
            c.danger.0
        } else if t.heat > 0.7 {
            c.warn.0
        } else {
            c.ok.0
        };
        b.rect(x, 4.0, tw, h - 14.0, 8.0, c.panel2.0, Some(Pen::new(1.0, if t.firing { c.ok.0 } else { c.line2.0 })));
        b.polyline(&arc_pts(tcx, tcy, 30.0, -140.0, 140.0), Pen::new(6.0, rgba(0x1c2734, 1.0)));
        if t.heat > 0.01 {
            let a1 = -140.0 + 280.0 * t.heat.clamp(0.0, 1.0) as f32;
            b.polyline(&arc_pts(tcx, tcy, 30.0, -140.0, a1), Pen::new(6.0, hc));
        }
        let fg = if t.ok < 0.3 { c.faint.0 } else { c.text.0 };
        b.circle(tcx, tcy, 13.0, fg, None);
        let a = (t.aim_deg as f32).to_radians();
        let (ax, ay) = (tcx + 24.0 * a.sin(), tcy - 24.0 * a.cos());
        b.seg(tcx, tcy, ax, ay, Pen::new(5.0, fg).round());
        b.text(tcx, tcy + 5.0, 13.0, c.bg.0, &t.short, T::mid().bold());
        match &t.op {
            Some(op) => {
                b.icon("person", tcx - 14.0, 104.0, 16.0, st.role.gunner.0);
                b.text(tcx - 3.0, 109.0, 14.0, c.text.0, op, T::default());
            }
            None => b.icon("auto", tcx, 104.0, 16.0, c.dim.0),
        }
        let mc = match t.mode.as_str() {
            "HOLD" => c.warn.0,
            "PD" => rgba(0x8fd3ff, 1.0),
            _ => c.text.0,
        };
        b.rect(
            x + 8.0,
            h - 46.0,
            tw - 16.0,
            26.0,
            5.0,
            rgba(0x0c121a, 1.0),
            Some(Pen::new(1.0, alpha(mc, 0x66 as f32 / 255.0))),
        );
        b.text(tcx, h - 28.0, 14.0, mc, &t.mode, T::mid().ls(1.5));
        cx.hits.add(&b, x, 4.0, tw, h - 14.0, Some(Hit::Act(Act::TMode(i))), Some("Turret: tap to change mode".into()));
    }
}

fn tubes(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "TUBES", grid(8.0, 2.0, 4.0, 2.0));
    let v = cx.v;
    let tgt = v.target.as_ref().and_then(|id| v.contacts.iter().find(|k| &k.id == id && k.hull > 0.0));
    let p = v.fire_ring;
    let ready = v.tubes.iter().any(|t| t.state == "READY");
    for (i, t) in v.tubes.iter().enumerate() {
        let y = 8.0 + i as f32 * 52.0;
        let col = match t.state.as_str() {
            "READY" => c.ok.0,
            "LOADING" => c.warn.0,
            _ => c.faint.0,
        };
        b.rect(10.0, y, 190.0, 42.0, 21.0, rgba(0x101822, 1.0), Some(Pen::new(2.0, col)));
        match t.state.as_str() {
            "LOADING" => {
                let k = (t.t / v.tube_load_s).clamp(0.0, 1.0) as f32;
                b.rect(14.0, y + 4.0, 182.0 * k, 34.0, 17.0, alpha(c.warn.0, 0.35), None);
            }
            "READY" => b.icon("tubes", 105.0, y + 21.0, 30.0, c.ok.0),
            _ => b.icon("load", 105.0, y + 21.0, 22.0, if v.magazine > 0 { c.dim.0 } else { c.faint.0 }),
        }
        let tip = match t.state.as_str() {
            "EMPTY" => "Load".to_owned(),
            "LOADING" => format!("Loading, {:.0} s", (v.tube_load_s - t.t).max(0.0)),
            _ => "Ready".to_owned(),
        };
        let hit = (t.state == "EMPTY" && v.magazine > 0).then_some(Hit::Act(Act::Load(i)));
        cx.hits.add(&b, 10.0, y, 190.0, 42.0, hit, Some(tip));
    }
    // The magazine's pips, in the mockup's band: six as drawn there, narrower when the ship carries more.
    let slots = v.magazine_slots.max(1);
    let pitch = if slots <= 6 { 32.0 } else { 192.0 / slots as f32 };
    let pw = pitch * 0.75;
    for k in 0..slots {
        let fill = if k < v.magazine { c.text.0 } else { rgba(0x1c2734, 1.0) };
        b.rect(12.0 + k as f32 * pitch, h - 22.0, pw, 10.0, 3.0, fill, None);
    }
    cx.hits.add(&b, 12.0, h - 22.0, pitch * slots as f32, 10.0, None, Some("Missiles in the magazine".into()));
    let fx = 214.0;
    let fw = w - fx - 10.0;
    let (fcx, fcy) = (fx + fw / 2.0, h / 2.0 - 4.0);
    let can = ready && tgt.is_some_and(|t| t.iff == "hostile");
    b.circle(fcx, fcy, 62.0, Color32::TRANSPARENT, Some(Pen::new(8.0, rgba(0x1c2734, 1.0))));
    if can {
        let pc = if v.fire_ok { c.ok.0 } else { c.warn.0 };
        b.polyline(&arc_pts(fcx, fcy, 62.0, 0.0, 360.0 * p as f32), Pen::new(8.0, pc));
    }
    let (fill, stroke) = if can { (rgba(0x3a0d14, 1.0), c.danger.0) } else { (c.panel2.0, c.line2.0) };
    b.circle(fcx, fcy, 52.0, fill, Some(Pen::new(2.0, stroke)));
    if let Some(hv) = v.hold.as_ref().filter(|h| h.key == "fire") {
        b.circle(fcx, fcy, 52.0 * hv.k.clamp(0.0, 1.0) as f32, alpha(c.danger.0, 0.4), None);
    }
    b.text(fcx, fcy + 2.0, 22.0, if can { c.text.0 } else { c.faint.0 }, "FIRE", T::mid().bold().ls(2.0));
    if can {
        let pc = if v.fire_ok { c.ok.0 } else { c.warn.0 };
        b.text(fcx, fcy + 24.0, 15.0, pc, &v.fire_note, T::mid().ls(1.0));
    }
    let tip = if can { "Hold to fire" } else { "No tube ready or no hostile target" };
    cx.hits.add(&b, fcx - 52.0, fcy - 52.0, 104.0, 104.0, can.then(|| Hit::Hold("fire".into())), Some(tip.into()));
}
