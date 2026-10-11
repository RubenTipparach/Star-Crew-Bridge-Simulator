//! The captain's console (the mockup's `captain()` and `captainShip()`). Two tabs: CMD has SHIP (the three decks in
//! plan, each compartment coloured by its state, the crew as dots), CREW (who holds each station), ALERT (red alert,
//! brace, the viewscreen) and ORDERS (write an order to a station; the last four with a tick once done). SHIP has the
//! three decks large and ROOM, the picked compartment's state, air and crew.
//!
//! The plans are the ship's one layout (`data/ships/tern/layout.json`, CLAUDE.md 8): its hull sections, its decks and
//! each compartment's brushes, compiled in.

use super::helm::panel;
use super::kit::{button, Act, Btn, Hit};
use super::{grid, CapView, Ctx};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::Color32;
use std::sync::OnceLock;

/// A compartment's brush: a convex prism between two heights, its outline [x, z] in ship axes, metres.
struct Brush {
    y: [f64; 2],
    poly: Vec<[f64; 2]>,
}

struct Compartment {
    id: String,
    name: String,
    decks: Vec<String>,
    brushes: Vec<Brush>,
}

struct Deck {
    id: String,
    floor_y_m: f64,
    clear_height_m: f64,
}

/// The layout's parts the plans draw.
struct Plan {
    /// Hull sections: z and half beam, metres.
    sections: Vec<[f64; 2]>,
    decks: Vec<Deck>,
    comps: Vec<Compartment>,
}

fn plan() -> &'static Plan {
    static P: OnceLock<Plan> = OnceLock::new();
    P.get_or_init(|| {
        let l: serde_json::Value = serde_json::from_str(include_str!("../../../../data/ships/tern/layout.json"))
            .expect("the layout is valid JSON (tools/layout_check.py)");
        let num = |v: &serde_json::Value| v.as_f64().expect("a layout number");
        let sections = l["hull"]["sections"]
            .as_array()
            .expect("hull sections")
            .iter()
            .map(|s| [num(&s["z_m"]), num(&s["half_beam_m"])])
            .collect();
        let decks = l["decks"]
            .as_array()
            .expect("decks")
            .iter()
            .map(|d| Deck {
                id: d["id"].as_str().expect("a deck id").to_owned(),
                floor_y_m: num(&d["floor_y_m"]),
                clear_height_m: num(&d["clear_height_m"]),
            })
            .collect();
        let comps = l["compartments"]
            .as_array()
            .expect("compartments")
            .iter()
            .map(|c| Compartment {
                id: c["id"].as_str().expect("an id").to_owned(),
                name: c["name"].as_str().expect("a name").to_owned(),
                decks: c["decks"]
                    .as_array()
                    .expect("decks")
                    .iter()
                    .map(|d| d.as_str().unwrap_or("").to_owned())
                    .collect(),
                brushes: c["brushes"]
                    .as_array()
                    .expect("brushes")
                    .iter()
                    .map(|b| Brush {
                        y: [num(&b["y"][0]), num(&b["y"][1])],
                        poly: b["poly"].as_array().expect("a poly").iter().map(|p| [num(&p[0]), num(&p[1])]).collect(),
                    })
                    .collect(),
            })
            .collect();
        Plan { sections, decks, comps }
    })
}

/// A compartment's look by its state word (the mockup's `compState`): fill, edge, icon and level.
fn look(word: &str) -> (Color32, Color32, &'static str, u8) {
    let c = &vg::style().c;
    match word {
        "OPEN TO SPACE" => (rgba(0x20263a, 1.0), c.danger.0, "breach", 3),
        "FIRE" => (rgba(0x5a1a10, 1.0), c.danger.0, "flame", 3),
        "SMOKE" => (rgba(0x3a3530, 1.0), c.warn.0, "flame", 2),
        "DAMAGED" => (rgba(0x3b3214, 1.0), c.warn.0, "wrench", 2),
        "NO POWER" => (rgba(0x0a0d12, 1.0), c.faint.0, "bolt", 1),
        _ => (rgba(0x172231, 1.0), rgba(0x2f3e51, 1.0), "check", 0),
    }
}

/// A compartment's state word, by its id: OK where nothing reports one.
fn word_of<'a>(cap: &'a CapView, id: &str) -> &'a str {
    cap.rooms.iter().find(|r| r.id == id).map(|r| r.word.as_str()).unwrap_or("OK")
}

/// A brush's middle (the mean of its corners), [x, z].
fn centroid(b: &Brush) -> [f64; 2] {
    let n = b.poly.len().max(1) as f64;
    let (x, z) = b.poly.iter().fold((0.0, 0.0), |a, p| (a.0 + p[0], a.1 + p[1]));
    [x / n, z / n]
}

/// One deck in plan (the mockup's `deckPlan`): the hull's outline, each compartment on the deck filled by its state,
/// icons over the hurt ones, the crew as dots. Bow to the right, port up.
#[allow(clippy::too_many_arguments)]
fn deck_plan(
    b: &Canvas,
    cx: &mut Ctx,
    cap: &CapView,
    deck: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    select: bool,
    label: bool,
) {
    let c = &vg::style().c;
    let p = plan();
    let t = cx.v.t;
    let z_min = p.sections.iter().map(|s| s[0]).fold(f64::MAX, f64::min);
    let z_max = p.sections.iter().map(|s| s[0]).fold(f64::MIN, f64::max);
    let hw_max = p.sections.iter().map(|s| s[1]).fold(f64::MIN, f64::max);
    let sc = ((f64::from(w) - 8.0) / (z_max - z_min)).min((f64::from(h) - 4.0) / (2.0 * hw_max));
    let ox = f64::from(x) + f64::from(w) / 2.0 - ((z_max + z_min) / 2.0) * sc;
    let oy = f64::from(y) + f64::from(h) / 2.0;
    let px = |z: f64| ((ox + z * sc) * 10.0).round() as f32 / 10.0;
    let py = |xx: f64| ((oy - xx * sc) * 10.0).round() as f32 / 10.0;
    let mut hull: Vec<[f32; 2]> = p.sections.iter().map(|q| [px(q[0]), py(q[1])]).collect();
    hull.extend(p.sections.iter().rev().map(|q| [px(q[0]), py(-q[1])]));
    b.poly(&hull, rgba(0x0a1017, 1.0));
    let mut closed = hull.clone();
    closed.push(hull[0]);
    b.polyline(&closed, Pen::new(1.5, c.line2.0));
    let Some(dk) = p.decks.iter().find(|d| d.id == deck) else { return };
    let (y0, y1) = (dk.floor_y_m, dk.floor_y_m + dk.clear_height_m);
    let on_deck = |br: &Brush| br.y[0] < y1 + 0.01 && br.y[1] > y0 - 0.01;
    for (i, comp) in p.comps.iter().enumerate() {
        if !comp.decks.iter().any(|d| d == deck) {
            continue;
        }
        let word = word_of(cap, &comp.id);
        let (col, edge, _, lv) = look(word);
        let sel = select && cap.ship_sel == comp.id;
        let flick = if word == "FIRE" { 0.75 + 0.25 * (t * 9.0 + i as f64).sin() } else { 1.0 } as f32;
        let (stroke, sw) = if sel {
            (c.text.0, 2.5)
        } else if lv >= 2 {
            (edge, 2.0)
        } else {
            (edge, 1.0)
        };
        for br in comp.brushes.iter().filter(|br| on_deck(br)) {
            let pts: Vec<[f32; 2]> = br.poly.iter().map(|q| [px(q[1]), py(q[0])]).collect();
            b.poly(&pts, alpha(col, flick));
            let mut ring = pts.clone();
            ring.push(pts[0]);
            b.polyline(&ring, Pen::new(sw, alpha(stroke, flick)));
            let (mut ax, mut ay, mut bx, mut by) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for q in &pts {
                (ax, ay, bx, by) = (ax.min(q[0]), ay.min(q[1]), bx.max(q[0]), by.max(q[1]));
            }
            let tip = if lv > 0 { format!("{}: {}", comp.name, word.to_lowercase()) } else { comp.name.clone() };
            cx.hits.add(b, ax, ay, bx - ax, by - ay, Some(Hit::Act(Act::Room(comp.id.clone()))), Some(tip));
        }
    }
    // Icons over hurt rooms, and crew as dots.
    for comp in &p.comps {
        if !comp.decks.iter().any(|d| d == deck) {
            continue;
        }
        let (_, edge, icon, lv) = look(word_of(cap, &comp.id));
        if lv < 2 {
            continue;
        }
        let Some(br) = comp.brushes.iter().find(|br| on_deck(br)) else { continue };
        let [mx, mz] = centroid(br);
        let size = (sc * 3.0).clamp(12.0, 22.0) as f32;
        b.icon_w(icon, px(mz), py(mx), size, edge, 2.4);
    }
    for (k, cr) in cap.crew_at.iter().enumerate() {
        let Some(comp) = cr.comp.as_ref().and_then(|id| p.comps.iter().find(|c| &c.id == id)) else { continue };
        if comp.decks.first().map(String::as_str) != Some(deck) {
            continue;
        }
        let Some(br) = comp.brushes.first() else { continue };
        let [mx, mz] = centroid(br);
        let col = if cr.ok { rgba(0x8fd3ff, 1.0) } else { c.danger.0 };
        b.circle(px(mz) - 10.0 + (k % 4) as f32 * 7.0, py(mx) + 10.0, 3.2, col, None);
    }
    if label {
        b.text(x + 4.0, y + 14.0, 13.0, c.dim.0, deck, T::default().bold().ls(2.0));
    }
}

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    let Some(cap) = cx.v.captain.clone() else { return };
    if cap.tab == "SHIP" {
        ship_tab(cv, cx, &cap);
        return;
    }
    ship(cv, cx, &cap);
    crew(cv, cx, &cap);
    alert(cv, cx, &cap);
    orders(cv, cx, &cap);
}

fn ship(cv: &Canvas, cx: &mut Ctx, cap: &CapView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "SHIP", grid(0.0, 0.0, 5.0, 4.0));
    let dh = (h - 20.0) / 3.0;
    for (k, d) in ["A", "B", "C"].iter().enumerate() {
        deck_plan(&b, cx, cap, d, 8.0, 4.0 + k as f32 * (dh + 4.0), w - 16.0, dh, false, true);
    }
    b.rect(w - 42.0, 2.0, 32.0, 26.0, 5.0, c.panel2.0, Some(Pen::new(1.0, c.line2.0)));
    let d = format!("M{} 10h6M{} 10v6M{} 24h-6M{} 24v-6", w - 34.0, w - 34.0, w - 18.0, w - 18.0);
    b.path(&d, Xf::ID, None, Some(Pen::new(2.0, c.text.0)));
    cx.hits.add(
        &b,
        w - 42.0,
        2.0,
        32.0,
        26.0,
        Some(Hit::Act(Act::Tab("SHIP".into()))),
        Some("Open the ship view".into()),
    );
}

fn crew(cv: &Canvas, cx: &mut Ctx, cap: &CapView) {
    let st = vg::style();
    let c = &st.c;
    let (b, w, h) = panel(cv, cx, "CREW", grid(5.0, 0.0, 4.0, 2.0));
    let (cw, ch) = ((w - 20.0 - 6.0) / 2.0, (h - 12.0 - 2.0 * 6.0) / 3.0);
    for (i, row) in cap.crew.iter().enumerate() {
        let (x, y) = (10.0 + (i % 2) as f32 * (cw + 6.0), 4.0 + (i / 2) as f32 * (ch + 6.0));
        b.rect(x, y, cw, ch, 7.0, c.panel2.0, Some(Pen::new(1.0, c.line2.0)));
        b.icon(&row.role, x + 22.0, y + ch / 2.0, 22.0, st.role.of(&row.role));
        match &row.op {
            Some(op) => {
                b.text(x + 42.0, y + ch / 2.0 + 6.0, 17.0, c.text.0, op, T::default());
            }
            None => b.icon("auto", x + 56.0, y + ch / 2.0, 18.0, c.dim.0),
        }
        if row.order {
            b.icon("dots", x + cw - 20.0, y + ch / 2.0, 18.0, c.warn.0);
        }
        let tip = format!("{}: {}", row.role.replacen('_', " ", 1), row.op.as_deref().unwrap_or("automation"));
        cx.hits.add(&b, x, y, cw, ch, Some(Hit::Act(Act::Compose(row.role.clone()))), Some(tip));
    }
}

fn alert(cv: &Canvas, cx: &mut Ctx, cap: &CapView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "ALERT", grid(9.0, 0.0, 3.0, 2.0));
    let red = cx.v.alert == "red_alert";
    let col = if red { c.ok.0 } else { c.alert.0 };
    button(
        &b,
        cx,
        Btn {
            x: 10.0,
            y: 6.0,
            w: w - 20.0,
            h: 74.0,
            label: Some(if red { "STAND DOWN" } else { "RED ALERT" }),
            hold: Some("redalert"),
            col: Some(col),
            col2: Some(col),
            stroke: Some(alpha(col, 0x88 as f32 / 255.0)),
            fs: Some(20.0),
            tip: Some("Hold".into()),
            ..Default::default()
        },
    );
    let bw = (w - 20.0 - 6.0) / 2.0;
    button(
        &b,
        cx,
        Btn {
            x: 10.0,
            y: 90.0,
            w: bw,
            h: h - 102.0,
            icon: Some("brace"),
            label: Some("BRACE"),
            stack: true,
            act: Some(Act::Brace),
            on: cap.braced,
            col: Some(c.warn.0),
            tip: Some(if cap.braced { "Braced: press to stand down" } else { "Brace for impact" }.into()),
            ..Default::default()
        },
    );
    button(
        &b,
        cx,
        Btn {
            x: 16.0 + bw,
            y: 90.0,
            w: bw,
            h: h - 102.0,
            icon: Some("eye"),
            label: Some("SCREEN"),
            stack: true,
            act: Some(Act::ViewTake),
            on: cap.view_override,
            tip: Some("Take the viewscreen".into()),
            ..Default::default()
        },
    );
}

fn orders(cv: &Canvas, cx: &mut Ctx, cap: &CapView) {
    let st = vg::style();
    let c = &st.c;
    let (b, w, _h) = panel(cv, cx, "ORDERS", grid(5.0, 2.0, 7.0, 2.0));
    for (i, r) in ["helm", "tactical", "engineering", "science"].iter().enumerate() {
        let rc = st.role.of(r);
        button(
            &b,
            cx,
            Btn {
                x: 10.0 + i as f32 * 58.0,
                y: 6.0,
                w: 52.0,
                h: 52.0,
                icon: Some(r),
                act: Some(Act::Compose((*r).to_owned())),
                on: cap.composer.to == *r,
                col: Some(rc),
                fill: Some(alpha(rc, 0x26 as f32 / 255.0)),
                tip: Some((*r).to_owned()),
                ..Default::default()
            },
        );
    }
    let mut vx = 10.0_f32;
    if let Some(verbs) = st.order_verbs.get(&cap.composer.to) {
        for [verb, ic] in verbs {
            let vw = 40.0 + verb.chars().count() as f32 * 9.0;
            button(
                &b,
                cx,
                Btn {
                    x: vx,
                    y: 66.0,
                    w: vw,
                    h: 44.0,
                    icon: Some(ic),
                    label: Some(verb),
                    act: Some(Act::Verb(verb.clone())),
                    on: cap.composer.verb.as_deref() == Some(verb.as_str()),
                    fs: Some(14.0),
                    is: Some(18.0),
                    ..Default::default()
                },
            );
            vx += vw + 6.0;
        }
    }
    let has = cap.composer.verb.is_some();
    button(
        &b,
        cx,
        Btn {
            x: 248.0,
            y: 6.0,
            w: 100.0,
            h: 52.0,
            icon: Some("send"),
            label: Some("SEND"),
            act: has.then_some(Act::Send),
            off: !has,
            col: Some(c.ok.0),
            on: has,
            tip: Some("Send the order".into()),
            ..Default::default()
        },
    );
    // The last orders, newest first: who, what, and a tick once done.
    for (i, o) in cap.orders.iter().take(4).enumerate() {
        let (x, y) = (364.0_f32, 6.0 + i as f32 * 36.0);
        b.rect(x, y, w - x - 10.0, 30.0, 6.0, c.panel2.0, Some(Pen::new(1.0, c.line.0)));
        b.icon(&o.to, x + 16.0, y + 15.0, 16.0, st.role.of(&o.to));
        b.text(x + 32.0, y + 20.0, 14.0, c.text.0, &o.verb, T::default().ls(1.0));
        let done = o.state == "done";
        b.icon(if done { "check" } else { "dots" }, w - 30.0, y + 15.0, 16.0, if done { c.ok.0 } else { c.warn.0 });
    }
}

fn ship_tab(cv: &Canvas, cx: &mut Ctx, cap: &CapView) {
    for (k, d) in ["A", "B", "C"].iter().enumerate() {
        let g = match k {
            0 => grid(0.0, 0.0, 6.0, 2.0),
            1 => grid(6.0, 0.0, 6.0, 2.0),
            _ => grid(0.0, 2.0, 6.0, 2.0),
        };
        let (b, w, h) = panel(cv, cx, &format!("DECK {d}"), g);
        deck_plan(&b, cx, cap, d, 6.0, 2.0, w - 12.0, h - 6.0, true, false);
    }
    room(cv, cx, cap);
}

fn room(cv: &Canvas, cx: &mut Ctx, cap: &CapView) {
    let c = &vg::style().c;
    let (b, w, h) = panel(cv, cx, "ROOM", grid(6.0, 2.0, 6.0, 2.0));
    let p = plan();
    let name = p.comps.iter().find(|c| c.id == cap.ship_sel).map(|c| c.name.to_uppercase()).unwrap_or_default();
    let word = word_of(cap, &cap.ship_sel);
    let (_, edge, icon, lv) = look(word);
    b.text(14.0, 30.0, 24.0, c.text.0, &name, T::default().bold().ls(1.0));
    b.rect(
        14.0,
        44.0,
        word.chars().count() as f32 * 9.0 + 40.0,
        28.0,
        6.0,
        alpha(edge, 0x22 as f32 / 255.0),
        Some(Pen::new(1.0, edge)),
    );
    b.icon(icon, 30.0, 58.0, 16.0, edge);
    b.text(44.0, 63.0, 14.0, if lv > 0 { edge } else { c.ok.0 }, word, T::default().ls(1.5));
    let gauge = |v: Option<f64>, f: &dyn Fn(f64) -> (String, Color32)| match v {
        Some(x) => f(x),
        None => ("--".to_owned(), c.faint.0),
    };
    let r = cap.room;
    let rows = [
        (
            "Pressure",
            "hull",
            gauge(r.map(|r| r.p_kpa), &|p| {
                (
                    format!("{p:.0} kPa"),
                    if p < 60.0 {
                        c.danger.0
                    } else if p < 90.0 {
                        c.warn.0
                    } else {
                        c.ok.0
                    },
                )
            }),
        ),
        (
            "Oxygen",
            "air_o2",
            gauge(r.map(|r| r.po2_kpa), &|p| {
                (
                    format!("{p:.0} kPa"),
                    if p < 12.0 {
                        c.danger.0
                    } else if p < 17.0 {
                        c.warn.0
                    } else {
                        c.ok.0
                    },
                )
            }),
        ),
        (
            "Temperature",
            "temp",
            gauge(r.map(|r| r.t_k), &|t| {
                (
                    format!("{:.0} \u{b0}C", t - 273.15),
                    if t > 318.0 {
                        c.danger.0
                    } else if t > 303.0 {
                        c.warn.0
                    } else {
                        c.ok.0
                    },
                )
            }),
        ),
        ("Crew here", "person", gauge(r.map(|r| f64::from(r.crew)), &|n| (format!("{n}"), c.text.0))),
    ];
    for (k, (tip, ic, (val, col))) in rows.iter().enumerate() {
        let x = 14.0 + k as f32 * ((w - 28.0) / 4.0);
        b.icon(ic, x + 14.0, 100.0, 20.0, *col);
        b.text(x + 30.0, 107.0, 18.0, c.text.0, val, T::default());
        cx.hits.add(
            &b,
            x,
            88.0,
            (w - 28.0) / 4.0,
            26.0,
            None,
            Some(if r.is_some() { (*tip).to_owned() } else { format!("{tip}: not simulated in this drill") }),
        );
    }
    button(
        &b,
        cx,
        Btn {
            x: 14.0,
            y: h - 52.0,
            w: 200.0,
            h: 42.0,
            icon: Some("wrench"),
            label: Some("SEND REPAIRS"),
            act: Some(Act::OrderRepair),
            // No damage control is simulated where the room has no readings: the mockup's unavailable state.
            off: cap.room.is_none(),
            tip: Some(
                if cap.room.is_some() {
                    "Order damage control here"
                } else {
                    "Damage control: not simulated in this drill"
                }
                .into(),
            ),
            fs: Some(14.0),
            ..Default::default()
        },
    );
}
