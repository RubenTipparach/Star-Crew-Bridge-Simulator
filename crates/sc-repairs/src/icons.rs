//! The guide card's icons (repair-minigames design 6g; `kit.js`'s `ICONS`): one small picture a move, drawn in a box
//! 100 px across centred on the origin, so every game's card speaks the same picture language. A finger is the blue
//! dot with a white rim; a target is amber; good is green, a mistake red.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::pen::{c, hex, rgba, Pen};

fn finger(g: &Pen, x: f32, y: f32) {
    g.disc(x, y, 10.0, c::ACCENT);
    g.ring(x, y, 10.0, hex(0xe8f7ff), 2.5);
}

fn arrow_head(g: &Pen, x: f32, y: f32, a: f32, col: egui::Color32, s: f32) {
    g.poly(
        &[
            [x + a.cos() * s, y + a.sin() * s],
            [x + (a + 2.5).cos() * s, y + (a + 2.5).sin() * s],
            [x + (a - 2.5).cos() * s, y + (a - 2.5).sin() * s],
        ],
        col,
    );
}

fn line(g: &Pen, pts: &[[f32; 2]], col: egui::Color32, w: f32) {
    g.path_round(pts, w, col);
}

fn dashed(g: &Pen, pts: &[[f32; 2]], col: egui::Color32, w: f32, on: f32, off: f32) {
    g.dashed(pts, w, col, on, off);
}

/// Draw icon `name` at the pen's origin, animated by `t` seconds; an unknown name draws `tap`.
pub fn draw(g: &Pen, name: &str, t: f32) {
    match name {
        "drag" => {
            dashed(
                g,
                &crate::pen::round_rect_points(14.0, -38.0, 30.0, 30.0, 6.0)
                    .into_iter()
                    .chain([[20.0, -38.0]])
                    .collect::<Vec<_>>(),
                c::AMBER,
                3.0,
                6.0,
                5.0,
            );
            let u = (t * 0.5) % 1.0;
            let (x, y) = (-30.0 + 59.0 * u, 22.0 - 45.0 * u);
            dashed(g, &[[-30.0, 22.0], [22.0, -16.0]], rgba(232, 238, 246, 0.35), 3.0, 5.0, 6.0);
            g.round(x - 14.0, y - 14.0, 28.0, 28.0, 6.0, Some(c::STEEL), None);
            finger(g, x + 6.0, y + 8.0);
        }
        "hold" => {
            g.disc(0.0, 0.0, 26.0, hex(0x262e42));
            g.ring(0.0, 0.0, 34.0, hex(0x2a3446), 6.0);
            g.arc(0.0, 0.0, 34.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * ((t * 0.4) % 1.0), 6.0, c::OK);
            finger(g, 0.0, 0.0);
        }
        "turn" => {
            g.ring(0.0, 0.0, 22.0, c::STEEL, 6.0);
            let r = g.rotate(t * 2.0);
            for k in 0..3 {
                let b = k as f32 * TAU / 3.0;
                r.line(0.0, 0.0, b.cos() * 20.0, b.sin() * 20.0, 4.0, hex(0xc9d3e0));
            }
            g.turn_arrow(0.0, 0.0, 38.0, -2.6, 0.4, c::AMBER);
            let a = t * 2.0;
            finger(g, a.cos() * 38.0, a.sin() * 38.0);
        }
        "screw" => {
            g.disc(0.0, 0.0, 18.0, hex(0xb7c0cc));
            g.ring(0.0, 0.0, 18.0, hex(0x7a8494), 2.0);
            let r = g.rotate(-t * 2.0);
            r.line(-9.0, 0.0, 9.0, 0.0, 4.0, hex(0x2a313c));
            r.line(0.0, -9.0, 0.0, 9.0, 4.0, hex(0x2a313c));
            g.turn_arrow(0.0, 0.0, 34.0, 0.4, -2.6, c::AMBER);
        }
        "swap" => {
            g.round(-44.0, -18.0, 36.0, 36.0, 6.0, Some(hex(0x1d1410)), Some((2.0, c::DANGER)));
            line(g, &[[-32.0, -12.0], [-24.0, -2.0], [-30.0, 4.0], [-20.0, 14.0]], c::DANGER, 3.0);
            g.round(8.0, -18.0, 36.0, 36.0, 6.0, Some(c::STEEL), Some((2.0, c::OK)));
            line(g, &[[4.0, 28.0], [-6.0, 28.0]], c::FG, 4.0);
            arrow_head(g, -6.0, 28.0, PI, c::FG, 10.0);
            dashed(g, &[[-26.0, -28.0], [26.0, -28.0]], rgba(232, 238, 246, 0.3), 2.0, 4.0, 4.0);
        }
        "sweep" => {
            let pts: Vec<[f32; 2]> =
                (0..21).map(|i| [-40.0 + i as f32 * 4.0, 14.0 * (i as f32 * 0.35).sin()]).collect();
            dashed(g, &pts, rgba(232, 238, 246, 0.35), 3.0, 5.0, 5.0);
            let n = 1 + (((t * 0.6) % 1.0) * 20.0) as usize;
            line(g, &pts[..=n.min(20)], c::AMBER, 6.0);
            finger(g, pts[n.min(20)][0], pts[n.min(20)][1]);
        }
        "rhythm" => {
            let off = (t * 30.0) % 34.0;
            line(g, &[[-46.0, 16.0], [46.0, 16.0]], hex(0x4a5568), 2.0);
            for k in -2..=2 {
                let x = -40.0 + k as f32 * 34.0 + off;
                if !(-46.0..=46.0).contains(&x) {
                    continue;
                }
                let on = x.abs() < 6.0;
                line(g, &[[x - 8.0, 16.0], [x, -8.0], [x + 8.0, 16.0]], if on { c::OK } else { c::FG }, 3.0);
            }
            line(g, &[[0.0, -24.0], [0.0, 30.0]], c::AMBER, 3.0);
        }
        "band" => {
            g.arc(0.0, 14.0, 40.0, PI * 1.05, PI * 1.95, 10.0, hex(0x2a3446));
            g.arc(0.0, 14.0, 40.0, PI * 1.4, PI * 1.62, 10.0, c::OK);
            g.hatch_arc(0.0, 14.0, 35.0, 45.0, PI * 1.8, PI * 1.95, c::DANGER);
            let a = PI * 1.51 + 0.07 * (t * 3.0).sin();
            line(g, &[[0.0, 14.0], [a.cos() * 36.0, 14.0 + a.sin() * 36.0]], c::FG, 4.0);
            g.disc(0.0, 14.0, 6.0, c::STEEL);
        }
        "order" => {
            let p = [[-30.0, 26.0], [22.0, 26.0], [-4.0, -18.0]];
            dashed(g, &[p[0], p[1]], rgba(232, 238, 246, 0.3), 2.0, 4.0, 4.0);
            dashed(g, &[p[1], p[2]], rgba(232, 238, 246, 0.3), 2.0, 4.0, 4.0);
            let next = ((t * 0.8) as usize) % 3;
            for (i, [x, y]) in p.iter().enumerate() {
                g.disc(*x, *y, 8.0, hex(0xb7c0cc));
                g.order_badge(*x, *y, 8.0, i + 1, i == next, t);
            }
        }
        "rubber" => {
            g.round(-40.0, -34.0, 80.0, 68.0, 10.0, None, Some((9.0, hex(0x111418))));
            g.round(-40.0, -34.0, 80.0, 68.0, 10.0, None, Some((2.0, rgba(255, 71, 87, 0.5))));
            let x = -8.0 + 12.0 * (t * 2.5).sin();
            g.ring(x, 2.0, 15.0, rgba(232, 238, 246, 0.75), 3.0);
            finger(g, x, 2.0);
        }
        "alternate" => {
            let flip = (t * 1.6).sin().max(0.0);
            for (i, x) in [-30.0f32, 0.0, 30.0].iter().enumerate() {
                let up = if i == 1 { flip < 0.5 } else { true };
                let k = if i == 1 { (1.0 - 2.0 * flip).abs() } else { 1.0 };
                g.round(x - 11.0 * k, -36.0, 22.0 * k, 72.0, 4.0, Some(hex(0x4a586d)), None);
                let col = if up == (i != 1) { c::ACCENT } else { c::AMBER };
                let flipped = if i == 1 { !up } else { up };
                let d = if flipped { 1.0 } else { -1.0 };
                line(g, &[[x - 8.0 * k, 8.0 * d], [*x, -8.0 * d], [x + 8.0 * k, 8.0 * d]], col, 4.0);
            }
        }
        "slider" => {
            line(g, &[[-40.0, 0.0], [40.0, 0.0]], hex(0x2a3446), 10.0);
            line(g, &[[-6.0, 0.0], [14.0, 0.0]], c::OK, 10.0);
            let x = 4.0 + 12.0 * (t * 2.0).sin();
            g.round(x - 9.0, -20.0, 18.0, 40.0, 6.0, Some(hex(0xc9d3e0)), None);
            arrow_head(g, -46.0, 0.0, PI, c::AMBER, 9.0);
            arrow_head(g, 46.0, 0.0, 0.0, c::AMBER, 9.0);
        }
        "match" => {
            let wave = |ph: f32, amp: f32| -> Vec<[f32; 2]> {
                (0..25).map(|i| [-44.0 + i as f32 * 3.67, amp * (i as f32 * 0.5 + ph).sin()]).collect()
            };
            line(g, &wave(0.0, 18.0), rgba(61, 220, 132, 0.4), 9.0);
            line(g, &wave(0.6 * (t * 1.5).sin(), 18.0 + 4.0 * t.sin()), c::FG, 3.0);
        }
        "valve" => {
            line(g, &[[-46.0, 10.0], [46.0, 10.0]], hex(0x3a4658), 12.0);
            g.poly(&[[-18.0, -4.0], [0.0, 10.0], [-18.0, 24.0]], hex(0xc0392b));
            g.poly(&[[18.0, -4.0], [18.0, 24.0], [0.0, 10.0]], hex(0xc0392b));
            g.path(&[[-18.0, -4.0], [18.0, 24.0], [18.0, -4.0], [-18.0, 24.0]], true, 2.0, hex(0xff8a80));
            line(g, &[[0.0, 10.0], [0.0, -22.0]], hex(0xc9d3e0), 4.0);
            line(g, &[[-14.0, -22.0], [14.0, -22.0]], hex(0xc9d3e0), 5.0);
            finger(g, 10.0, -22.0);
        }
        "pipes" => {
            g.round(-30.0, -30.0, 60.0, 60.0, 8.0, Some(hex(0x0f1520)), Some((2.0, hex(0x2a3446))));
            let q = ((t * 0.7) as i32 % 4) as f32 * FRAC_PI_2 + ((t * 0.7 % 1.0) * 4.0).min(1.0) * FRAC_PI_2;
            let r = g.rotate(q);
            let curve: Vec<[f32; 2]> = (0..=12)
                .map(|i| {
                    let u = i as f32 / 12.0;
                    // The quadratic from (0, -30) through the control (0, 0) to (30, 0).
                    let (a, b) = ((1.0 - u) * (1.0 - u), 2.0 * u * (1.0 - u));
                    [b * 0.0 + u * u * 30.0, a * -30.0 + b * 0.0]
                })
                .collect();
            r.path_round(&curve, 14.0, hex(0x4a586d));
            g.turn_arrow(0.0, 0.0, 42.0, -2.2, -0.9, c::AMBER);
        }
        "flow" => {
            line(g, &[[-46.0, 0.0], [46.0, 0.0]], hex(0x263142), 22.0);
            let u = (t * 0.5) % 1.0;
            line(g, &[[-46.0, 0.0], [-46.0 + 92.0 * u, 0.0]], hex(0xff6a4d), 14.0);
            let mut x = -38.0;
            while x < -46.0 + 92.0 * u - 6.0 {
                line(g, &[[x - 4.0, -6.0], [x + 3.0, 0.0], [x - 4.0, 6.0]], hex(0xfff3e0), 2.5);
                x += 18.0;
            }
            g.round(-26.0, 18.0, 52.0, 24.0, 12.0, Some(hex(0x1f6b45)), None);
        }
        "aim" => {
            g.ring(0.0, 0.0, 30.0, hex(0x4a5568), 2.0);
            line(g, &[[-40.0, 0.0], [40.0, 0.0]], hex(0x4a5568), 2.0);
            line(g, &[[0.0, -40.0], [0.0, 40.0]], hex(0x4a5568), 2.0);
            let r = 6.0 + 10.0 * t.sin().abs();
            g.disc(4.0 * (t * 1.3).sin(), 3.0 * t.cos(), r, rgba(242, 160, 70, 0.8));
        }
        "tool" => {
            for i in 0..3 {
                let (x, on) = (-34.0 + i as f32 * 34.0, i == 1);
                g.round(
                    x - 14.0,
                    -4.0,
                    28.0,
                    34.0,
                    6.0,
                    Some(if on { hex(0x3a2a12) } else { hex(0x141c28) }),
                    Some((if on { 3.0 } else { 2.0 }, if on { c::AMBER } else { hex(0x2a3446) })),
                );
                line(g, &[[x - 5.0, 20.0], [x + 5.0, 4.0]], if on { c::FG } else { c::STEEL }, 4.0);
            }
            finger(g, 4.0, 16.0 + 3.0 * (t * 3.0).sin());
            arrow_head(g, 0.0, -20.0, FRAC_PI_2, c::AMBER, 10.0);
        }
        "plug" => {
            let x = -6.0 - 10.0 * (t * 1.5).sin().abs();
            g.round(14.0, -18.0, 32.0, 36.0, 6.0, Some(hex(0x141c28)), Some((3.0, c::STEEL)));
            g.path(&[[30.0, -9.0], [39.0, 6.0], [21.0, 6.0]], true, 2.5, c::AMBER);
            line(g, &[[-46.0, 0.0], [x - 14.0, 0.0]], c::LILAC, 6.0);
            g.round(x - 14.0, -12.0, 24.0, 24.0, 4.0, Some(c::LILAC), None);
            g.poly(&[[x - 2.0, -7.0], [x + 5.0, 5.0], [x - 9.0, 5.0]], hex(0x1d1430));
        }
        _ => {
            // tap
            for k in 0..2 {
                let ph = (t * 0.9 + k as f32 / 2.0) % 1.0;
                g.ring(0.0, 0.0, 14.0 + ph * 26.0, rgba(242, 160, 70, 0.9 * (1.0 - ph)), 3.0);
            }
            g.ring(0.0, 0.0, 16.0, c::AMBER, 4.0);
            finger(g, 0.0, 0.0);
        }
    }
}
