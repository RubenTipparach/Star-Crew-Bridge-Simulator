//! The science console (the mockup's `science()`): SENSORS (the scanner with its sweep and ping, and PING), SHIELDS
//! (the ship in its shield in 3D, large, with the presets, the frequency band and who set them), CONTACT (the picked
//! contact and its scan) and SCREEN (what the viewscreen shows, and its zoom).

use super::helm::panel;
use super::kit::{self, arc_path, button, Act, Btn, Drag, Hit, Scan};
use super::{accent, grid, iff_col, Ctx};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::Color32;

/// The viewscreen's cameras and their icons, as the camera pad lays them out (the mockup's `FEED_PAD`).
const FEED_PAD: [(&str, &str); 6] =
    [("TARGET", "target"), ("FWD", "fwd"), ("CHASE", "chasecam"), ("PORT", "port"), ("AFT", "aft"), ("STBD", "stbd")];

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    sensors(cv, cx);
    shields(cv, cx);
    contact(cv, cx);
    screen(cv, cx);
}

fn sensors(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "SENSORS", grid(0.0, 0.0, 5.0, 4.0));
    let v = cx.v;
    let acc = accent(v);
    let s = Scan::new(v, w as f64, (h - 56.0) as f64, "sci", None);
    s.draw_base(&b);
    kit::scan_wrap(&b, cx, "sci", w, h - 56.0);
    // The sweep: a wedge 25 degrees wide turning once in six seconds.
    let sweep = (v.t * 60.0) % 360.0;
    let [x0, y0] = s.dir(sweep - 25.0, 1.0);
    let [x1, y1] = s.dir(sweep, 1.0);
    let d = format!("M{} {}L{x0} {y0}A{} {} 0 0 1 {x1} {y1}Z", s.cx, s.cy, s.r, s.r * s.k);
    b.path(&d, Xf::ID, Some(alpha(acc, 0.10)), None);
    let ping = v.sci.as_ref().map(|q| q.ping).unwrap_or(0.0);
    if ping > 0.0 {
        let k = 1.0 - ping / 2.0;
        let pen = Pen::new(3.0, alpha(acc, (ping / 2.0) as f32));
        b.ellipse(s.cx as f32, s.cy as f32, (s.r * k) as f32, (s.r * s.k * k) as f32, Color32::TRANSPARENT, Some(pen));
    }
    kit::feed_cone(&b, cx, &s);
    kit::ship_glyph(&b, "corvette", "friendly", s.cx as f32, s.cy as f32, 20.0, s.hdg as f32, c.text.0, false);
    kit::scanner_contacts(&b, cx, &s, true);
    kit::range_ctl(&b, cx, 12.0, 6.0, "sci");
    kit::scan_reset(&b, cx, w - 46.0, 6.0, "sci");
    button(
        &b,
        cx,
        Btn {
            x: w - 160.0,
            y: h - 50.0,
            w: 148.0,
            h: 42.0,
            icon: Some("ping"),
            label: Some("PING"),
            hold: Some("ping"),
            col: Some(acc),
            tip: Some("Active ping: hold. Everyone hears it.".into()),
            ..Default::default()
        },
    );
}

fn shields(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let (b, w, h) = panel(cv, cx, "SHIELDS", grid(5.0, 0.0, 4.0, 4.0));
    let v = cx.v;
    let vh = h - 60.0;
    let (cam, freq, by) = match &v.sci {
        Some(q) => (q.cam, q.freq.clone(), q.by.clone()),
        None => (super::Cam { yaw: 62.0, el: 24.0 }, "A".to_owned(), "Tactical".to_owned()),
    };
    cx.hits.add(
        &b,
        0.0,
        0.0,
        w,
        vh,
        Some(Hit::Drag(Drag::ShieldCam)),
        Some("Drag to turn the view; press a face to favour it".into()),
    );
    // The mockup draws the view in a nested <svg> clipped to the panel's top. The view stays inside it (the labels
    // are held 30 lp in from the sides, the ship and bubble span about 300 x 200 lp of 404 x 300), so it is drawn
    // unclipped: a clip rectangle is a UI draw call (CLAUDE.md 2).
    kit::shield_view(&b, cx, f64::from(w / 2.0 + 10.0), f64::from(vh / 2.0 + 6.0), 2.8, cam.yaw, cam.el, Some([w, vh]));
    let pre = [
        ("balance", "Balanced", -1),
        ("fwd", "Bow", 0),
        ("aft", "Stern", 1),
        ("port", "Port", 2),
        ("stbd", "Starboard", 3),
    ];
    for (i, (ic, tip, f)) in pre.iter().enumerate() {
        button(
            &b,
            cx,
            Btn {
                x: 10.0 + i as f32 * 50.0,
                y: h - 52.0,
                w: 44.0,
                h: 42.0,
                icon: Some(ic),
                act: Some(Act::Favour(*f)),
                tip: Some(format!("Shields: {tip}")),
                is: Some(20.0),
                on: v.preset == *f,
                ..Default::default()
            },
        );
    }
    button(
        &b,
        cx,
        Btn {
            x: 268.0,
            y: h - 52.0,
            w: 44.0,
            h: 42.0,
            label: Some(&freq),
            act: Some(Act::Freq),
            tip: Some("Frequency band".into()),
            fs: Some(18.0),
            ..Default::default()
        },
    );
    button(
        &b,
        cx,
        Btn {
            x: 318.0,
            y: h - 52.0,
            w: 44.0,
            h: 42.0,
            icon: Some("eye"),
            act: Some(Act::ShieldView),
            tip: Some("Back to the usual view".into()),
            is: Some(20.0),
            ..Default::default()
        },
    );
    let role = by.to_lowercase();
    b.icon(if by == "Science" { "science" } else { "tactical" }, w - 22.0, h - 31.0, 18.0, st.role.of(&role));
    cx.hits.add(&b, w - 32.0, h - 41.0, 20.0, 20.0, None, Some(format!("Set by {by}")));
}

fn contact(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "CONTACT", grid(9.0, 0.0, 3.0, 2.0));
    let v = cx.v;
    let acc = accent(v);
    let sel = v.sci.as_ref().and_then(|q| q.sel.as_ref());
    let Some(k) = sel.and_then(|id| v.contacts.iter().find(|k| &k.id == id)) else {
        b.text(w / 2.0, h / 2.0, 14.0, c.dim.0, "NO CONTACT", T::mid().ls(2.0));
        return;
    };
    let col = iff_col(&k.iff);
    let (gx, gy) = (68.0_f32, h / 2.0);
    b.path(&arc_path(gx, gy, 58.0, 0.0, 359.99), Xf::ID, None, Some(Pen::new(7.0, rgba(0x1c2734, 1.0))));
    if k.scan > 0.0 {
        b.path(&arc_path(gx, gy, 58.0, 0.0, 360.0 * k.scan), Xf::ID, None, Some(Pen::new(7.0, acc)));
        cx.hits.add(
            &b,
            gx - 62.0,
            gy - 62.0,
            124.0,
            124.0,
            None,
            Some(format!("Scanned {}%", kit::js_round(k.scan * 100.0))),
        );
    }
    kit::ship_glyph(&b, &k.cls, &k.iff, gx, gy, 64.0, 0.0, col, false);
    if k.scan >= 1.0 {
        let r = 5.0 + 2.0 * (v.t * 6.0).sin() as f32;
        b.circle(gx + 6.0, gy + 12.0, r, Color32::TRANSPARENT, Some(Pen::new(3.0, c.warn.0)));
        cx.hits.add(&b, gx - 2.0, gy + 4.0, 16.0, 16.0, None, Some("Weak point: its drive".into()));
    }
    let unknown = k.iff == "unknown";
    b.text(140.0, 30.0, 24.0, c.text.0, if unknown { "?" } else { &k.id }, T::default().bold());
    let cls = if unknown { "UNKNOWN".to_owned() } else { k.cls.to_uppercase() };
    b.text(140.0, 50.0, 12.0, c.dim.0, &cls, T::default().ls(2.0));
    if k.scan >= 0.3 {
        kit::bar(&b, 140.0, 60.0, w - 154.0, 8.0, k.hull / 100.0, col, false);
    }
    let done = k.scan >= 1.0;
    let scanning = v.sci.as_ref().is_some_and(|q| q.scanning);
    button(
        &b,
        cx,
        Btn {
            x: 140.0,
            y: h - 58.0,
            w: w - 152.0,
            h: 46.0,
            icon: Some("scan"),
            label: Some(if done {
                "DONE"
            } else if scanning {
                "SCANNING"
            } else {
                "SCAN"
            }),
            act: (!done).then_some(Act::Scan),
            off: done,
            on: scanning,
            tip: Some("Scan (Q)".into()),
            fs: Some(14.0),
            ..Default::default()
        },
    );
}

fn screen(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "SCREEN", grid(9.0, 2.0, 3.0, 2.0));
    let v = cx.v;
    let bw = (w - 20.0 - 2.0 * 6.0) / 3.0;
    for (i, (k, ic)) in FEED_PAD.iter().enumerate() {
        button(
            &b,
            cx,
            Btn {
                x: 10.0 + (i % 3) as f32 * (bw + 6.0),
                y: 6.0 + (i / 3) as f32 * 52.0,
                w: bw,
                h: 46.0,
                icon: Some(ic),
                act: Some(Act::Feed((*k).to_owned())),
                on: v.feed == *k,
                tip: Some(k.to_lowercase()),
                is: Some(22.0),
                ..Default::default()
            },
        );
    }
    button(
        &b,
        cx,
        Btn {
            x: 10.0,
            y: h - 50.0,
            w: 60.0,
            h: 40.0,
            label: Some("\u{2212}"),
            act: Some(Act::Zoom(-1)),
            fs: Some(22.0),
            tip: Some("Zoom out".into()),
            ..Default::default()
        },
    );
    let z = if v.zoom.fract() == 0.0 { format!("{}", v.zoom as i64) } else { format!("{}", v.zoom) };
    b.text(w / 2.0, h - 23.0, 18.0, c.text.0, &format!("{z}\u{d7}"), T::mid());
    button(
        &b,
        cx,
        Btn {
            x: w - 70.0,
            y: h - 50.0,
            w: 60.0,
            h: 40.0,
            label: Some("+"),
            act: Some(Act::Zoom(1)),
            fs: Some(22.0),
            tip: Some("Zoom in".into()),
            ..Default::default()
        },
    );
}
