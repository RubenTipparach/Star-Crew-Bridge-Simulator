//! A laser bank's console (the mockup's `laserConsole`, laser-banks 3): CHARGE, ARC, SPOT and FIRE on the 104 x 102
//! grid. One console serves both banks; the station says which.

use super::helm::panel;
use super::kit::{self, arc_path, pol, range_km, Hit};
use super::{brg_of, grid, ratio_col, v3, ContactView, Ctx, LaserView};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::Color32;
use sc_core::combat::data::DrillData;
use std::sync::OnceLock;

/// What the console draws against from the Tern's data: the bank's range (m), how far past the keel line its arc
/// reaches (degrees) and how long a beam shows (s).
struct Reach {
    range_m: f64,
    past_keel_deg: f64,
    beam_s: f64,
}

fn reach() -> &'static Reach {
    static R: OnceLock<Reach> = OnceLock::new();
    R.get_or_init(|| {
        let d = DrillData::shipped();
        let past_keel_deg = d.tern_combat.arc_overlap_deg;
        match d.tern_combat.lasers.as_ref().map(|l| d.laser(&l.laser)) {
            Some(l) => Reach { range_m: l.range_m, past_keel_deg, beam_s: l.beam_s },
            None => Reach { range_m: 8000.0, past_keel_deg, beam_s: 0.4 },
        }
    })
}

/// The face names a beam can meet, as the SPOT panel's tip writes them.
fn face_word(f: &str) -> String {
    f.to_uppercase()
}

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    let v = cx.v;
    let Some(ls) = v.lasers.as_ref() else { return };
    let port = v.station == "laser_port";
    let (lv, side, id) = if port { (&ls.port, 1.0, "port") } else { (&ls.stbd, -1.0, "stbd") };
    let beam_age = v.laser_beam_age_s.map_or(1e9, |a| if port { a.port } else { a.stbd });
    let tgt = v.target.as_ref().and_then(|id| v.contacts.iter().find(|k| &k.id == id && k.hull > 0.0));
    charge(cv, cx, lv);
    arc(cv, cx, lv, side, tgt);
    spot(cv, cx, lv, tgt);
    fire(cv, cx, lv, id, beam_age, tgt.is_some());
}

/// The capacitor filling, the least charge to fire as a dashed line across it, and the seconds to full.
fn charge(cv: &Canvas, cx: &mut Ctx, lv: &LaserView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "CHARGE", grid(0.0, 0.0, 3.0, 4.0));
    let (bw, by) = (84.0, 14.0);
    let (bx, bh) = (w / 2.0 - bw / 2.0, h - 96.0);
    let k = lv.charge.clamp(0.0, 1.0) as f32;
    let col = if lv.charge >= lv.min_charge { c.ok.0 } else { c.warn.0 };
    b.rect(bx, by, bw, bh, 10.0, rgba(0x101822, 1.0), Some(Pen::new(1.0, c.line2.0)));
    let fill = if k >= 0.999 { col } else { alpha(col, 0.8) };
    b.rect(bx + 4.0, by + 4.0 + (bh - 8.0) * (1.0 - k), bw - 8.0, (bh - 8.0) * k, 7.0, fill, None);
    let my = by + 4.0 + (bh - 8.0) * (1.0 - lv.min_charge as f32);
    b.seg(bx - 12.0, my, bx + bw + 12.0, my, Pen::new(2.0, c.text.0).dash(6.0, 4.0, 0.0));
    let tip = format!("{:.0}% · fires from {:.0}%", lv.charge * 100.0, lv.min_charge * 100.0);
    cx.hits.add(&b, bx, by, bw, bh, None, Some(tip));
    if k >= 0.999 {
        b.icon("check", w / 2.0, h - 44.0, 34.0, c.ok.0);
    } else {
        b.text(w / 2.0, h - 32.0, 34.0, c.text.0, &format!("{} s", lv.secs_to_full.max(0.0).ceil()), T::mid().bold());
    }
}

/// The Tern from above, bow up: the bank's half of the sky out to its range, and the target in it or not.
fn arc(cv: &Canvas, cx: &mut Ctx, lv: &LaserView, side: f64, tgt: Option<&ContactView>) {
    let st = vg::style();
    let c = &st.c;
    let (b, w, h) = panel(cv, cx, "ARC", grid(3.0, 0.0, 3.0, 4.0));
    let (ccx, ccy) = (w / 2.0, h / 2.0 - 6.0);
    let r = w.min(h) / 2.0 - 22.0;
    let rc = reach();
    let a0 = if side > 0.0 { 180.0 - rc.past_keel_deg } else { -rc.past_keel_deg };
    let a1 = a0 + 180.0 + 2.0 * rc.past_keel_deg;
    b.circle(ccx, ccy, r, Color32::TRANSPARENT, Some(Pen::new(1.0, c.line2.0)));
    let lc = st.role.laser.0;
    b.path(&wedge(ccx, ccy, r, a0, a1), Xf::ID, Some(alpha(lc, 0x1f as f32 / 255.0)), Some(Pen::new(1.5, lc)));
    kit::ship_glyph(&b, "corvette", "friendly", ccx, ccy, 36.0, 0.0, c.text.0, false);
    if let Some(t) = tgt {
        let rel = v3(t.rel);
        let rk = ((range_km(t.rel) * 1000.0) / rc.range_m).min(1.0);
        let [px, py] = pol(ccx, ccy, r * rk.max(0.3) as f32, brg_of(rel));
        let col = if lv.bearing { c.hostile.0 } else { c.faint.0 };
        let pen = if lv.bearing { Pen::new(2.5, col) } else { Pen::new(1.5, col).dash(3.0, 5.0, 0.0) };
        b.seg(ccx, ccy, px, py, pen);
        kit::ship_glyph(&b, &t.cls, &t.iff, px, py, 28.0, 0.0, col, !lv.bearing);
        let tip = format!(
            "{} · {:.1} km{}",
            t.id,
            range_km(t.rel),
            if lv.bearing { "" } else { " · outside this bank's arc" }
        );
        cx.hits.add(&b, px - 14.0, py - 14.0, 28.0, 28.0, None, Some(tip));
    }
}

/// The mockup's `wedge`: a slice of the circle from bearing a0 to a1 (degrees, 0 up, clockwise).
fn wedge(cx: f32, cy: f32, r: f32, a0: f64, a1: f64) -> String {
    let [x0, y0] = pol(cx, cy, r, a0);
    let a = arc_path(cx, cy, r, a0, a1);
    let arc = a.find('A').map_or("", |i| &a[i..]);
    format!("M{cx} {cy}L{x0} {y0}{arc}Z")
}

/// The face the beam would meet: its shield as a ring, its armour as a plate, struck through when gone.
fn spot(cv: &Canvas, cx: &mut Ctx, lv: &LaserView, tgt: Option<&ContactView>) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "SPOT", grid(6.0, 0.0, 3.0, 4.0));
    let (ccx, ccy) = (w / 2.0, h / 2.0 - 18.0);
    let r = w.min(h) / 2.0 - 44.0;
    let Some(t) = tgt.filter(|_| !lv.spot_face.is_empty()) else {
        b.icon("target", ccx, ccy, 48.0, c.faint.0);
        return;
    };
    let (sh, ar) = (lv.spot_shield.clamp(0.0, 1.0), lv.spot_armour.clamp(0.0, 1.0));
    let gone = ar <= 0.01;
    b.circle(ccx, ccy, r, Color32::TRANSPARENT, Some(Pen::new(12.0, rgba(0x1c2734, 1.0))));
    if sh > 0.005 {
        b.path(&arc_path(ccx, ccy, r, 0.0, 360.0 * sh), Xf::ID, None, Some(Pen::new(12.0, ratio_col(sh))));
    }
    let (pw, ph) = (r * 1.1, r * 0.8);
    let (px, py) = (ccx - pw / 2.0, ccy - ph / 2.0);
    if !gone {
        b.rect(px, py, pw, ph, 6.0, rgba(0x101822, 1.0), Some(Pen::new(2.0, c.dim.0)));
        let k = ar as f32;
        b.rect(px + 4.0, py + 4.0 + (ph - 8.0) * (1.0 - k), pw - 8.0, (ph - 8.0) * k, 4.0, alpha(c.text.0, 0.75), None);
    } else {
        b.rect(px, py, pw, ph, 6.0, Color32::TRANSPARENT, Some(Pen::new(2.0, c.danger.0).dash(7.0, 5.0, 0.0)));
        b.seg(px, py + ph, px + pw, py, Pen::new(3.0, c.danger.0));
        b.icon("hull", ccx, ccy, 30.0, c.danger.0);
    }
    b.text(ccx, h - 24.0, 17.0, c.dim.0, &face_word(&lv.spot_face), T::mid().ls(2.0).bold());
    let tip = format!("{}, {} face: shield {:.0}%, armour {:.0}%", t.id, lv.spot_face, sh * 100.0, ar * 100.0);
    cx.hits.add(&b, ccx - r, ccy - r, 2.0 * r, 2.0 * r, None, Some(tip));
}

/// The biggest control: hold to fire. Its ring is the chance to hit; the number is in its tooltip.
fn fire(cv: &Canvas, cx: &mut Ctx, lv: &LaserView, id: &str, beam_age: f64, has_target: bool) {
    let st = vg::style();
    let c = &st.c;
    let (b, w, h) = panel(cv, cx, "FIRE", grid(9.0, 0.0, 3.0, 4.0));
    let can = lv.bearing && lv.charge >= lv.min_charge;
    let (ccx, ccy) = (w / 2.0, h / 2.0);
    let r = w.min(h) / 2.0 - 26.0;
    let p = lv.hit_chance.clamp(0.0, 1.0);
    let why = if !has_target {
        "No target".to_owned()
    } else if !lv.bearing {
        "Target outside this bank's arc".to_owned()
    } else if !can {
        format!("Charging: fires from {:.0}%", lv.min_charge * 100.0)
    } else {
        format!("Hold to fire · {:.0}% to hit", p * 100.0)
    };
    b.circle(ccx, ccy, r, Color32::TRANSPARENT, Some(Pen::new(10.0, rgba(0x1c2734, 1.0))));
    if can && p > 0.0 {
        let pc = if p > 0.5 { c.ok.0 } else { c.warn.0 };
        b.path(&arc_path(ccx, ccy, r, 0.0, 360.0 * p), Xf::ID, None, Some(Pen::new(10.0, pc)));
    }
    let (fill, stroke) = if can { (rgba(0x3a0d14, 1.0), c.danger.0) } else { (c.panel2.0, c.line2.0) };
    let ri = r - 14.0;
    b.circle(ccx, ccy, ri, fill, Some(Pen::new(2.0, stroke)));
    let key = format!("lfire:{id}");
    if let Some(hv) = cx.v.hold.as_ref().filter(|hv| hv.key == key) {
        b.circle(ccx, ccy, ri * hv.k.clamp(0.0, 1.0) as f32, alpha(c.danger.0, 0.4), None);
    }
    if beam_age < reach().beam_s {
        b.circle(ccx, ccy, ri, alpha(st.role.laser.0, 0.5), None);
    }
    b.text(ccx, ccy + 11.0, 32.0, if can { c.text.0 } else { c.faint.0 }, "FIRE", T::mid().bold().ls(3.0));
    cx.hits.add(&b, ccx - ri, ccy - ri, 2.0 * ri, 2.0 * ri, can.then_some(Hit::Hold(key)), Some(why));
}
