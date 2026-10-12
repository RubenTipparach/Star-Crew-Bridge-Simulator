//! OBJECTIVES (the mockup's `objectivesPanel`, objectives-tab 2): the mission's title and the fight's clock, then a
//! row for each objective with its mark (a tick when done, a cross when failed, a ring filling while in progress)
//! and its one number. It opens over the four panels on every console, and over the gunner's sight.

use super::helm::panel;
use super::kit::arc_path;
use super::{accent, grid, Ctx};
use crate::vg::{self, rgba, Canvas, Pen, Xf, T};
use egui::Color32;

pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let Some(ov) = cx.v.objectives.as_ref() else { return };
    let (b, w, _h) = panel(cv, cx, "OBJECTIVES", grid(0.0, 0.0, 12.0, 4.0));
    let t = ov.clock_s.max(0.0).floor() as i64;
    b.text(24.0, 40.0, 26.0, c.text.0, &ov.title, T::default().bold());
    b.text(w - 24.0, 40.0, 26.0, c.dim.0, &format!("{:02}:{:02}", t / 60, t % 60), T::end());
    b.rect(24.0, 58.0, w - 48.0, 1.0, 0.0, c.line.0, None);
    for (i, r) in ov.rows.iter().take(6).enumerate() {
        let y = 78.0 + i as f32 * 82.0;
        let (mx, my) = (60.0, y + 34.0);
        let col = match r.state.as_str() {
            "done" => c.ok.0,
            "failed" => c.danger.0,
            _ => c.text.0,
        };
        let progress = r.state != "done" && r.state != "failed";
        let stroke = if progress { c.line2.0 } else { col };
        b.rect(20.0, y, w - 40.0, 68.0, 8.0, c.panel2.0, Some(Pen::new(1.0, stroke)));
        match r.state.as_str() {
            "done" => {
                b.circle(mx, my, 22.0, c.ok.0, None);
                b.icon_w("check", mx, my, 26.0, c.bg.0, 3.0);
            }
            "failed" => {
                b.circle(mx, my, 22.0, c.danger.0, None);
                b.icon_w("cross", mx, my, 24.0, c.bg.0, 3.0);
            }
            _ => {
                b.circle(mx, my, 20.0, Color32::TRANSPARENT, Some(Pen::new(6.0, rgba(0x1c2734, 1.0))));
                if r.k > 0.005 {
                    let d = arc_path(mx, my, 20.0, 0.0, 360.0 * r.k.clamp(0.0, 1.0));
                    b.path(&d, Xf::ID, None, Some(Pen::new(6.0, accent(cx.v))));
                }
            }
        }
        b.text(100.0, my + 8.0, 24.0, if progress { c.text.0 } else { col }, &r.words, T::default());
        if let Some(n) = &r.number {
            b.text(w - 44.0, my + 10.0, 30.0, col, n, T::end().bold());
        }
    }
}
