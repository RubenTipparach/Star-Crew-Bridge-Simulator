//! A name over everyone aboard (openspec/changes/crew-nameplates 1-2): the name, the faint crew-AI icon after it for a
//! bot or a single player's crew (ui-art-direction Q3: never "(NPC)"), where they walk to under it, on a square plate
//! in the screens' colours. The walk and the drill both draw their names with this one routine; each decides who is
//! seen (on screen, near, in line of sight).

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Vec2};

/// The name's size, points (design 1: 20, up from the drill's 17).
const NAME_PT: f32 = 20.0;
/// The walking line's size, points.
const GOING_PT: f32 = 16.0;
/// A name is clipped past this many characters, with an ellipsis (design 1: a plate is never wider than 300 px).
const MAX_CHARS: usize = 28;
/// Names show in full to this distance and fade out by the next, metres (design 2).
const FULL_M: f32 = 20.0;
const GONE_M: f32 = 25.0;

/// How much of a plate shows at `dist_m` from the eye: all of it near, fading from 20 m, none past 25 m.
pub fn fade(dist_m: f32) -> Option<f32> {
    (dist_m < GONE_M).then(|| ((GONE_M - dist_m) / (GONE_M - FULL_M)).clamp(0.0, 1.0))
}

/// A name clipped to the plate's width.
fn clipped(name: &str) -> String {
    if name.chars().count() <= MAX_CHARS {
        name.to_owned()
    } else {
        format!("{}\u{2026}", name.chars().take(MAX_CHARS - 1).collect::<String>())
    }
}

/// One plate, its name's bottom centred on `at` (the point over the head), drawn `k` opaque.
pub fn draw(painter: &Painter, at: Pos2, name: &str, npc: bool, going: Option<&str>, k: f32) {
    if k <= 0.0 {
        return;
    }
    let c = crate::look::look().c();
    let fade = |col: Color32| col.gamma_multiply(k);
    let galley = painter.layout_no_wrap(clipped(name), FontId::proportional(NAME_PT), fade(c.text.0));
    let icon = if npc { NAME_PT * 0.9 + 6.0 } else { 0.0 };
    let going = going.map(|g| painter.layout_no_wrap(g.to_owned(), FontId::proportional(GOING_PT), fade(c.dim.0)));
    let w = (galley.size().x + icon).max(going.as_ref().map_or(0.0, |g| g.size().x));
    let h = galley.size().y + going.as_ref().map_or(0.0, |g| g.size().y);
    let top = at.y - galley.size().y;
    let band = Rect::from_min_size(Pos2::new(at.x - w / 2.0 - 8.0, top - 3.0), Vec2::new(w + 16.0, h + 6.0));
    painter.rect_filled(band, 0.0, crate::look::alpha(c.panel.0, 0.82 * k));
    let x0 = at.x - (galley.size().x + icon) / 2.0;
    let name_w = galley.size().x;
    let name_h = galley.size().y;
    painter.galley(Pos2::new(x0, top), galley, c.text.0);
    if npc {
        let cv = crate::vg::Canvas::new(painter.clone(), egui::Vec2::ZERO, 1.0);
        cv.icon("auto", x0 + name_w + 6.0 + NAME_PT * 0.45, top + name_h / 2.0, NAME_PT * 0.9, fade(c.faint.0));
    }
    if let Some(g) = going {
        let p = Align2::CENTER_TOP.anchor_size(Pos2::new(at.x, at.y), g.size()).min;
        painter.galley(p, g, c.dim.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_fades_from_twenty_metres_and_is_gone_at_twenty_five() {
        assert_eq!(fade(5.0), Some(1.0));
        assert!((fade(22.5).unwrap() - 0.5).abs() < 1e-6, "half way through the fade");
        assert_eq!(fade(25.0), None);
    }

    #[test]
    fn a_long_name_is_clipped_with_an_ellipsis() {
        let n = clipped("Lieutenant Commander Aurelia Vantorius-Kell");
        assert_eq!(n.chars().count(), MAX_CHARS);
        assert!(n.ends_with('\u{2026}'));
        assert_eq!(clipped("Ens. Dara Holt"), "Ens. Dara Holt");
    }
}
