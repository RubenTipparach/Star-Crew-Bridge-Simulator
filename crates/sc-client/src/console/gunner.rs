//! The gunner's console (the mockup's `gunner()`, enemy-fighters 3): a sight, not panels. The view along the dorsal
//! turret's barrel at 30 degrees across, the chosen target ringed with its hull (left) and shield (right) as arcs, the
//! lead pip, the capacitor and heat as two bars, TAB and the targets. A drag across the sight turns the barrel; Space
//! is the trigger.

use super::kit::{self, arc_path, pol, Act, Btn, Drag, Hit};
use super::{iff_col, quat, v3, Ctx, GunnerView};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::Color32;
use glam::DVec3;
use std::sync::OnceLock;

/// Degrees between the barrel and the lead under which the reticle shows on target (the mockup's `AIM_OK_DEG`).
pub const AIM_OK_DEG: f64 = 1.2;
/// The sight's height below the title band, layout points.
const H: f32 = 656.0;
/// The sight's centre (the barrel's line), layout points.
const CX: f32 = 640.0;
const CY: f32 = 300.0;

/// The barrel's axes in ship axes for yaw to starboard and pitch up, degrees (the mockup's `gunBasis`): along it,
/// right on the sight, and up.
pub fn gun_basis(yaw: f64, pitch: f64) -> (DVec3, DVec3, DVec3) {
    let (y, p) = (yaw.to_radians(), pitch.to_radians());
    let f = DVec3::new(-y.sin() * p.cos(), p.sin(), y.cos() * p.cos());
    let r = f.cross(DVec3::Y).normalize_or_zero();
    (f, r, r.cross(f))
}

/// Degrees between the barrel and a direction (the mockup's `gunError`).
pub fn gun_error(g: &GunnerView, d: DVec3) -> f64 {
    let (f, _, _) = gun_basis(g.aim[0], g.aim[1]);
    f.dot(d.normalize_or_zero()).clamp(-1.0, 1.0).acos().to_degrees()
}

/// The mockup's `hash32` (shipsystems.js); the consoles page seeds it with 7, so the sight has the mockup's stars.
fn hash32(parts: &[&str]) -> f64 {
    let mut h: u32 = 0x9e37_79b9;
    for p in parts {
        for ch in p.encode_utf16() {
            h ^= u32::from(ch);
            h = h.wrapping_mul(0x85eb_ca6b);
            h ^= h >> 13;
        }
        h = (h ^ (h >> 16)).wrapping_mul(0xc2b2_ae35);
    }
    h ^= h >> 16;
    f64::from(h) / 4_294_967_296.0
}

/// A star: its direction in reference axes, whether it is a big one, its brightness.
struct Star {
    d: DVec3,
    big: bool,
    lum: f64,
}

/// The mockup's `STARS`: 1400 of them, spread evenly over the sky.
fn stars() -> &'static [Star] {
    static S: OnceLock<Vec<Star>> = OnceLock::new();
    S.get_or_init(|| {
        (0..1400)
            .map(|i| {
                let n = i.to_string();
                let z = hash32(&["7", "starz", &n]) * 2.0 - 1.0;
                let a = hash32(&["7", "stara", &n]) * std::f64::consts::TAU;
                let r = (1.0 - z * z).sqrt();
                Star {
                    d: DVec3::new(r * a.cos(), z, r * a.sin()),
                    big: hash32(&["7", "starr", &n]) < 0.06,
                    lum: 0.25 + hash32(&["7", "starl", &n]) * 0.75,
                }
            })
            .collect()
    })
}

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let c = &st.c;
    let v = cx.v;
    let Some(g) = v.gunner.as_ref() else { return };
    let s = cv.at(0.0, 32.0).clip(0.0, 0.0, 1280.0, H);
    let (f, r, u) = gun_basis(g.aim[0], g.aim[1]);
    let foc = 640.0 / 15f64.to_radians().tan();
    let proj = |d: DVec3| -> Option<[f32; 2]> {
        let z = d.dot(f);
        (z > 0.01).then(|| [(f64::from(CX) + d.dot(r) / z * foc) as f32, (f64::from(CY) - d.dot(u) / z * foc) as f32])
    };
    let inside = |p: Option<[f32; 2]>| p.filter(|p| p[0] > 0.0 && p[0] < 1280.0 && p[1] > 0.0 && p[1] < H);
    s.rect(0.0, 0.0, 1280.0, H, 0.0, rgba(0x020306, 1.0), None);
    cx.hits.add(&s, 0.0, 0.0, 1280.0, H, Some(Hit::Drag(Drag::GunAim)), None);
    let qi = quat(v.q).inverse();
    for star in stars() {
        if let Some(p) = inside(proj(qi * star.d)) {
            let rad = if star.big { 1.6 } else { 0.9 };
            s.circle(p[0], p[1], rad, alpha(rgba(0xcfd8e6, 1.0), star.lum as f32), None);
        }
    }
    for k in &v.contacts {
        if k.hull <= 0.0 || k.iff == "unknown" {
            continue;
        }
        let rel = v3(k.rel);
        if let Some(p) = inside(proj(rel)) {
            let sz = ((if k.cls == "fighter" { 48_000.0 } else { 120_000.0 }) / rel.length().max(1.0)).clamp(8.0, 90.0);
            kit::ship_glyph(&s, &k.cls, &k.iff, p[0], p[1], sz as f32, 0.0, iff_col(&k.iff), false);
        }
    }
    // Bolts while the trigger is held: two tracers from the barrel toward the sight's centre.
    if g.trigger && g.capacitor > 0.02 && !v.gun_locked {
        for sx in [560.0f32, 720.0] {
            let pen = Pen::new(3.0, alpha(st.role.gunner.0, 0.85)).dash(26.0, 34.0, 0.0);
            s.seg(sx, H, (CX + (sx - CX) * 0.18).round(), CY + 40.0, pen);
        }
    }
    let chosen = g.chosen.and_then(|i| g.targets.get(i));
    let mut aim_ok = false;
    if let Some(t) = chosen {
        let (rel, lead) = (v3(t.rel), v3(t.lead));
        let rr = 44.0;
        aim_ok = gun_error(g, lead) < AIM_OK_DEG;
        match inside(proj(rel)) {
            Some([px, py]) => {
                let (hk, sk) = (t.hull.clamp(0.0, 1.0), t.shield.clamp(0.0, 1.0));
                s.circle(px, py, rr, Color32::TRANSPARENT, Some(Pen::new(2.0, c.hostile.0)));
                let back = Pen::new(6.0, rgba(0x1c2734, 1.0));
                s.path(&arc_path(px, py, rr + 9.0, 200.0, 340.0), Xf::ID, None, Some(back));
                s.path(&arc_path(px, py, rr + 9.0, 20.0, 160.0), Xf::ID, None, Some(back));
                if hk > 0.005 {
                    let hc = if hk > 0.5 {
                        c.ok.0
                    } else if hk > 0.2 {
                        c.warn.0
                    } else {
                        c.danger.0
                    };
                    s.path(
                        &arc_path(px, py, rr + 9.0, 200.0, 200.0 + 140.0 * hk),
                        Xf::ID,
                        None,
                        Some(Pen::new(6.0, hc)),
                    );
                }
                if sk > 0.005 {
                    let pen = Pen::new(6.0, rgba(0x8fd3ff, 1.0));
                    s.path(&arc_path(px, py, rr + 9.0, 160.0 - 140.0 * sk, 160.0), Xf::ID, None, Some(pen));
                }
                s.text(px, py - rr - 20.0, 15.0, c.text.0, &t.id, T::mid().bold().ls(1.5));
                let tip = format!(
                    "{} · {:.1} km · hull {:.0}% · shield {:.0}%",
                    t.id,
                    rel.length() / 1000.0,
                    hk * 100.0,
                    sk * 100.0
                );
                cx.hits.add(&s, px - rr, py - rr, 2.0 * rr, 2.0 * rr, None, Some(tip));
                if let Some([lx, ly]) = inside(proj(lead)) {
                    s.seg(px, py, lx, ly, Pen::new(1.2, alpha(c.hostile.0, 0.8)).dash(3.0, 4.0, 0.0));
                    let d = format!("M{lx:.1} {:.1}l9 9l-9 9l-9 -9z", ly - 9.0);
                    let fill = if aim_ok { c.ok.0 } else { c.hostile.0 };
                    s.path(&d, Xf::ID, Some(fill), Some(Pen::new(1.5, rgba(0x020306, 1.0))));
                    cx.hits.add(&s, lx - 9.0, ly - 9.0, 18.0, 18.0, None, Some("Aim here".into()));
                }
            }
            None => {
                // Off the sight: an arrow at the edge toward it.
                let a = rel.dot(r).atan2(rel.dot(u)).to_degrees();
                let [ex, ey] = pol(CX, CY, 250.0, a);
                s.path("M0 -18L14 8H-14Z", Xf::tr(ex, ey).rot(a as f32), Some(c.hostile.0), None);
                cx.hits.add(&s, ex - 18.0, ey - 18.0, 36.0, 36.0, None, Some(format!("{} is off the sight", t.id)));
            }
        }
    }
    // The reticle: the barrel's line, green when it is on the lead.
    let rc = if aim_ok { c.ok.0 } else { st.role.gunner.0 };
    s.circle(CX, CY, 22.0, Color32::TRANSPARENT, Some(Pen::new(2.0, rc)));
    s.circle(CX, CY, 2.5, rc, None);
    let ticks = format!("M{} {CY}h16M{} {CY}h16M{CX} {}v16M{CX} {}v16", CX - 46.0, CX + 30.0, CY - 46.0, CY + 30.0);
    s.path(&ticks, Xf::ID, None, Some(Pen::new(2.0, rc)));
    kit::vignette_at(&s, 1280.0, H, Color32::BLACK, 0.9, [0.5, 0.46], 0.72, 0.62);
    // Capacitor and heat, two bars at the lower left.
    let cc = if g.capacitor > 0.3 { c.ok.0 } else { c.warn.0 };
    let hc = if v.gun_locked {
        c.danger.0
    } else if g.heat > 0.7 {
        c.warn.0
    } else {
        c.ok.0
    };
    s.icon("bolt", 52.0, 430.0, 20.0, cc);
    kit::bar(&s, 41.0, 448.0, 22.0, 170.0, g.capacitor, cc, true);
    cx.hits.add(&s, 41.0, 420.0, 22.0, 198.0, None, Some(format!("Capacitor {:.0}%", g.capacitor * 100.0)));
    s.icon("temp", 96.0, 430.0, 20.0, hc);
    kit::bar(&s, 85.0, 448.0, 22.0, 170.0, g.heat, hc, true);
    if v.gun_locked {
        s.path("M85 448l22 170M107 448l-22 170", Xf::ID, None, Some(Pen::new(2.0, c.danger.0)));
    }
    let heat_tip = if v.gun_locked { "Too hot: cooling".to_owned() } else { format!("Heat {:.0}%", g.heat * 100.0) };
    cx.hits.add(&s, 85.0, 420.0, 22.0, 198.0, None, Some(heat_tip));
    // TAB, and the targets it steps through.
    kit::button(
        &s,
        cx,
        Btn {
            x: 470.0,
            y: 590.0,
            w: 110.0,
            h: 44.0,
            icon: Some("target"),
            label: Some("TAB"),
            act: Some(Act::GunNext),
            tip: Some("Next target (Tab)".into()),
            is: Some(20.0),
            off: g.targets.len() < 2,
            ..Default::default()
        },
    );
    for (i, t) in g.targets.iter().take(5).enumerate() {
        let on = g.chosen == Some(i);
        let col = c.hostile.0;
        let x = 592.0 + i as f32 * 76.0;
        let (fill, pen) = if on {
            (alpha(col, 0x26 as f32 / 255.0), Pen::new(2.0, col))
        } else {
            (c.panel2.0, Pen::new(1.2, c.line2.0))
        };
        s.rect(x, 590.0, 68.0, 44.0, 7.0, fill, Some(pen));
        kit::ship_glyph(&s, &t.kind, "hostile", x + 20.0, 612.0, 18.0, 0.0, col, false);
        s.text(x + 53.0, 618.0, 15.0, c.text.0, &t.id, T::mid().bold());
        let tip = format!("{} · {:.1} km", t.id, v3(t.rel).length() / 1000.0);
        cx.hits.add(&s, x, 590.0, 68.0, 44.0, Some(Hit::Act(Act::GunPick(i))), Some(tip));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_barrel_at_zero_looks_along_the_bow_and_yaws_to_starboard() {
        let (f, r, u) = gun_basis(0.0, 0.0);
        assert!((f - DVec3::Z).length() < 1e-12 && u.y > 0.99, "level and dead ahead");
        assert!(r.x < -0.99, "right on the sight is starboard, -X in ship axes");
        let (f, _, _) = gun_basis(90.0, 0.0);
        assert!(f.x < -0.99, "yaw 90 to starboard points along -X");
    }

    #[test]
    fn the_stars_are_the_mockups() {
        // The mockup's hash32, as node computes it from the page's function.
        assert!((hash32(&["1", "starz", "0"]) - 0.052_446_457_557_380_2).abs() < 1e-15);
        assert!((hash32(&["1", "stara", "7"]) - 0.703_919_145_511_463_3).abs() < 1e-15);
        assert!((hash32(&["1", "starl", "1399"]) - 0.656_252_575_572_580_1).abs() < 1e-15);
        assert_eq!(stars().len(), 1400);
        let z0 = 0.573_248_929_111_287 * 2.0 - 1.0;
        assert!((stars()[0].d.y - z0).abs() < 1e-12, "the page's seed, 7: its first star's height");
        assert!(stars().iter().all(|s| (s.d.length() - 1.0).abs() < 1e-9), "every star is a direction");
    }
}
