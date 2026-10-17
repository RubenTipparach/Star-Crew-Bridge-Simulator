/*
 * minimap.js: the walking minimap of openspec/changes/walk-minimap, drawn for the mockups.
 *
 * One function draws it for every page that shows the walking screen (ui-art-direction.html on the desktop,
 * touch-controls.html on a phone), so the two cannot disagree (CLAUDE.md 6.1). It reads the one layout source with the
 * layout patches the compiled deck is built from (command_suite, deck_access, engineering, armory, exterior_windows,
 * in that order: deck A is the command suite, not layout.json's older rooms), through ShipKit.applyPatch.
 *
 * The picture (walk-minimap design 3): the deck the feet are on, seen from above with the bow to the right (the ship
 * map's deck view, ship-plan-view 1), 12 reference px a metre, a window that follows the body; every room's floor, the
 * room you are in lighter, doors as gaps, the ways off the deck (ladder, hatch, stair, lift) as one glyph with an up or
 * down chevron, crew as the map's markers (ship-plan-view 3), fire, damage and the injured in the state colours with
 * their shapes, and the room's name in one caps label with the deck's letter.
 *
 * Sizes are reference px of the 1920 x 1080 frame (ui-art-direction 3); the page scales the SVG.
 * It is a classic script, inlined by tools/mockups/inline.py between "INLINE lib:minimap" markers.
 */
(function () {
  "use strict";

  /** The design's numbers (walk-minimap design 2 and 3), reference px and metres. */
  const MINI = { w: 384, h: 256, band: 40, ppm: 12, edge: 1, door_m: 1.2, label_px: 20,
    // On a phone the same picture is drawn 168 x 112 px of the 390 px frame (touch.json's px), its band and name larger
    // so the name stays readable at that size (walk-minimap design 4).
    touch: { w_px: 168, h_px: 112, band: 52, label_px: 30 } };
  /** The style's colours (data/ui/ui_style.json via ui-art-direction's const UI): one look with the HUD. */
  const C = {
    ground: "#070a0f", panel: "#0c121a", raised: "#141d29", line: "#1f2a37", line2: "#2f3d4f",
    text: "#e8eef6", dim: "#98a8bc", faint: "#5d6c80", accent: "#6fd3ff", ok: "#3ddc84", warn: "#ffc542", danger: "#ff4757",
    // The ship map's own marker for you (sc-client main.rs draw_map: 255, 184, 77) and its fire (fire-spread 4).
    you: "#ffb84d", fire: "#ff8a3d",
  };
  /** The patches the compiled deck applies to the layout, in order (deckc; safety_points' WINDOW_PATCHES). */
  const PATCHES = ["command_suite", "deck_access", "engineering", "armory", "exterior_windows"];

  /** The layout as the compiled deck builds it: layout.json and each patch the page carries, in order. */
  function plan(K) {
    let L = K.layout();
    for (const n of PATCHES) {
      if (!document.getElementById("ship-data-" + n)) continue;
      const P = K.shipData(n);
      if (P.compartments || P.portals) L = K.applyPatch(L, P);
    }
    return L;
  }

  /** Deck `id`'s floor height, metres. */
  const floorOf = (L, id) => L.decks.find((d) => d.id === id).floor_y_m;

  /** The rooms standing on deck `id`: each compartment's brushes whose floor is the deck's (a trunk counts on every
   * deck it passes), as { id, name, kind, polys }. */
  function rooms(L, id) {
    const f = floorOf(L, id), out = [];
    for (const c of L.compartments) {
      const polys = c.brushes.filter((b) => b.y[0] <= f + 0.1 && b.y[1] > f + 0.5).map((b) => b.poly);
      if (polys.length) out.push({ id: c.id, name: c.name, kind: c.kind || "", polys });
    }
    return out;
  }

  /** The room on deck `id` under [x, z], or null. */
  function roomAt(K, L, id, x, z) {
    return rooms(L, id).find((r) => r.polys.some((p) => K.insidePoly(p, x, z, 0))) || null;
  }

  /** The ways off deck `id`: floor portals in its floor (down) or its ceiling (up), a hatch in a wall to a pod, and the
   * stair and lift trunks; each { at: [x, z], dir: "up" | "down" | "both", kind }. */
  function exits(L, id) {
    const d = L.decks.find((q) => q.id === id), f = d.floor_y_m, top = f + d.clear_height_m, out = [];
    const onDeck = new Set(rooms(L, id).map((r) => r.id));
    for (const p of L.portals) {
      if (!p.between.some((c) => onDeck.has(c)) || p.between.includes("space")) continue;
      const [x, y, z] = p.center_m;
      if (Math.abs(p.normal[1]) > 0.5) {
        if (Math.abs(y - (f - 0.5)) < 0.6) out.push({ at: [x, z], dir: "down", kind: p.kind });
        else if (Math.abs(y - top) < 0.6) out.push({ at: [x, z], dir: "up", kind: p.kind });
      } else if (p.kind === "hatch" && y > f && y < top && p.between.some((c) => /^pod_/.test(c))) {
        out.push({ at: [x, z], dir: "up", kind: "pod" });
      }
    }
    for (const r of rooms(L, id)) {
      if (r.kind !== "trunk") continue;
      const c = L.compartments.find((q) => q.id === r.id), decks = c.decks || [];
      const i = decks.indexOf(id), dir = i <= 0 ? "up" : i === decks.length - 1 ? "down" : "both";
      const p = r.polys[0], n = p.length;
      out.push({ at: [p.reduce((s, q) => s + q[0], 0) / n, p.reduce((s, q) => s + q[1], 0) / n], dir, kind: r.id === "lift" ? "lift" : "stair" });
    }
    // A ladder that goes both ways (deck B's, up to A and down to C) is two portals at one place: one glyph.
    const merged = [];
    for (const e of out) {
      const m = merged.find((q) => Math.hypot(q.at[0] - e.at[0], q.at[1] - e.at[1]) < 1.0);
      if (!m) merged.push(e);
      else if (m.dir !== e.dir) m.dir = "both";
    }
    return merged;
  }

  /** Doors on deck `id`: wall portals between two rooms on it, each { at: [x, z], t: [tx, tz] along the wall }. */
  function doors(L, id) {
    const d = L.decks.find((q) => q.id === id), f = d.floor_y_m, out = [];
    const onDeck = new Set(rooms(L, id).map((r) => r.id));
    for (const p of L.portals) {
      if (Math.abs(p.normal[1]) > 0.5 || p.kind === "window" || p.kind === "bay_door") continue;
      const y = p.center_m[1];
      if (y < f || y > f + d.clear_height_m || !p.between.every((c) => onDeck.has(c))) continue;
      out.push({ at: [p.center_m[0], p.center_m[2]], t: [-p.normal[2], p.normal[0]], w: p.size_m[0] });
    }
    return out;
  }

  const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;");

  /**
   * The minimap as SVG markup, `w` x `h` reference px.
   * o: { deck, at: [x, z], face: [fx, fz], crew: [{ at, face, colour, seated, downed }], fires: [[x, z]],
   *      damage: [[x, z]], red: bool, touch: bool (the map icon in its corner: it is the MAP button), here (optional) }
   */
  function svg(K, L, o) {
    const W = o.w || MINI.w, H = o.h || MINI.h, B = o.touch ? MINI.touch.band : MINI.band, ppm = o.ppm || MINI.ppm;
    const LP = o.touch ? MINI.touch.label_px : MINI.label_px;
    const cx = W / 2, cy = B + (H - B) / 2;
    // Bow to the right, port up (ship-plan-view 1): screen x from +Z, screen y from -X, about the body.
    const S = (x, z) => [cx + (z - o.at[1]) * ppm, cy - (x - o.at[0]) * ppm];
    const pts = (poly) => poly.map(([x, z]) => S(x, z).map((v) => v.toFixed(1)).join(",")).join(" ");
    const here = roomAt(K, L, o.deck, o.at[0], o.at[1]);
    const name = o.here || (here ? here.name : "");
    const rs = rooms(L, o.deck);
    const id = "mm" + Math.random().toString(36).slice(2, 8);
    let g = "";
    // Floors: every room in raised, the room you are in lighter; trunks (stairs, lift) hatched.
    for (const r of rs) {
      const fill = here && r.id === here.id ? "#22324a" : r.kind === "trunk" ? `url(#${id}h)` : C.raised;
      for (const p of r.polys) g += `<polygon points="${pts(p)}" fill="${fill}" stroke="${C.line2}" stroke-width="2" stroke-linejoin="round"/>`;
    }
    // Doors: a gap in the wall, the width of the door.
    for (const d of doors(L, o.deck)) {
      const hw = Math.min(d.w, MINI.door_m) / 2, a = S(d.at[0] - d.t[0] * hw, d.at[1] - d.t[1] * hw), b = S(d.at[0] + d.t[0] * hw, d.at[1] + d.t[1] * hw);
      g += `<line x1="${a[0]}" y1="${a[1]}" x2="${b[0]}" y2="${b[1]}" stroke="${C.raised}" stroke-width="5"/>`;
    }
    // Hazards (damage-map: an amber diamond for a damaged system; fire-spread: the flame) under the people.
    for (const [x, z] of o.damage || []) {
      const [sx, sy] = S(x, z);
      g += `<path d="M${sx} ${sy - 9}l9 9-9 9-9-9z" fill="${C.warn}" stroke="${C.ground}" stroke-width="2"/>`;
    }
    for (const [x, z] of o.fires || []) {
      const [sx, sy] = S(x, z);
      g += `<circle cx="${sx}" cy="${sy}" r="16" fill="${C.fire}" opacity="0.28"/>`
        + `<path d="M${sx} ${sy - 12}c5 6 9 9 9 15a9 9 0 0 1-18 0c0-4 2-6 4-9 0 3 1 5 3 5 0-4 0-7 2-11z" fill="${C.fire}" stroke="${C.ground}" stroke-width="1.5"/>`;
    }
    // Ways off the deck: a round plate with the way's glyph and its chevron (up, down, or both).
    for (const e of exits(L, o.deck)) {
      const [sx, sy] = S(e.at[0], e.at[1]);
      const chev = (up) => `<path d="M${sx - 5} ${sy + (up ? -3 : 3)}l5 ${up ? -5 : 5} 5 ${up ? 5 : -5}" fill="none" stroke="${C.text}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>`;
      const box = e.kind === "lift" ? `<rect x="${sx - 11}" y="${sy - 11}" width="22" height="22" rx="2" fill="${C.panel}" stroke="${C.dim}" stroke-width="2"/>`
        : `<circle cx="${sx}" cy="${sy}" r="11" fill="${C.panel}" stroke="${C.dim}" stroke-width="2"/>`;
      g += box + (e.dir === "both" ? `<g transform="translate(0,-3)">${chev(true)}</g><g transform="translate(0,3)">${chev(false)}</g>` : chev(e.dir === "up"));
    }
    // People (ship-plan-view 3): a disc with a wedge for its facing; seated, a square round it; downed, a red cross that
    // flashes (the page flashes the class). You are another shape: the arrow of your facing in the map's own colour for
    // you, outlined, with a ring, drawn last (a role colour may be near it: engineering's amber; the shape tells them apart).
    const person = (p, you) => {
      const [sx, sy] = S(p.at[0], p.at[1]);
      const ang = Math.atan2(-(p.face ? p.face[0] : 0), p.face ? p.face[1] : 1);   // screen angle of the facing
      const at = (a, r) => `${(sx + Math.cos(ang + a) * r).toFixed(1)} ${(sy + Math.sin(ang + a) * r).toFixed(1)}`;
      if (you) {
        return `<circle cx="${sx}" cy="${sy}" r="17" fill="none" stroke="${C.you}" stroke-width="2" opacity="0.6"/>`
          + `<path d="M${at(0, 15)}L${at(2.45, 12)}L${at(Math.PI, 5)}L${at(-2.45, 12)}z" fill="${C.you}" stroke="${C.ground}" stroke-width="2.5" stroke-linejoin="round"/>`;
      }
      const r = 6, col = p.colour;
      let s = `<path d="M${at(0, r + 8)}L${at(2.2, r)}L${at(-2.2, r)}z" fill="${col}"/>`;
      s += `<circle cx="${sx}" cy="${sy}" r="${r}" fill="${col}" stroke="${C.ground}" stroke-width="2"/>`;
      if (p.seated) s += `<rect x="${sx - r - 4}" y="${sy - r - 4}" width="${2 * r + 8}" height="${2 * r + 8}" fill="none" stroke="${col}" stroke-width="2"/>`;
      if (p.downed) s += `<g class="mm-flash"><circle cx="${sx}" cy="${sy}" r="${r + 5}" fill="${C.danger}" stroke="${C.ground}" stroke-width="2"/><path d="M${sx - 5} ${sy}h10M${sx} ${sy - 5}v10" stroke="${C.text}" stroke-width="3"/></g>`;
      return s;
    };
    for (const p of o.crew || []) g += person(p, false);
    g += person({ at: o.at, face: o.face }, true);
    // The band: the room's name and the deck's letter; on a phone the map icon (it is the MAP button).
    const icon = o.touch ? `<g transform="translate(${W - 30},${B / 2}) scale(1.3)" fill="none" stroke="${C.text}" stroke-width="2.4" stroke-linejoin="round">
        <path d="M-12 -9l8 -3 8 3 8 -3v18l-8 3-8-3-8 3z"/><path d="M-4 -12v18M4 -9v18"/></g>` : "";
    const cw = Math.round(LP * 1.6), chipX = W - (o.touch ? 64 : 12) - cw;
    const edge = o.red ? C.danger : C.line2;
    // A long name is squeezed to its band, never over the deck's letter (CLAUDE.md 10: clipped, never overlapping).
    const room = W - cw - (o.touch ? 96 : 44), wide = name.length * LP * 0.74;
    const fitName = wide > room ? `textLength="${room}" lengthAdjust="spacingAndGlyphs"` : "";
    return `<svg class="minimap" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg" style="display:block">
      <defs>
        <clipPath id="${id}c"><rect x="0" y="${B}" width="${W}" height="${H - B}"/></clipPath>
        <pattern id="${id}h" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
          <rect width="8" height="8" fill="${C.raised}"/><line x1="0" y1="0" x2="0" y2="8" stroke="${C.line2}" stroke-width="3"/></pattern>
      </defs>
      <rect x="0" y="0" width="${W}" height="${H}" fill="${C.panel}"/>
      <rect x="0" y="${B}" width="${W}" height="${H - B}" fill="${C.ground}"/>
      <g clip-path="url(#${id}c)">${g}</g>
      <rect x="0" y="0" width="${W}" height="${B}" fill="${C.panel}"/>
      <line x1="0" y1="${B}" x2="${W}" y2="${B}" stroke="${C.line}" stroke-width="1"/>
      <text x="16" y="${B / 2 + LP * 0.35}" fill="${C.text}" ${fitName} style="font:700 ${LP}px 'Barlow Semi Condensed';letter-spacing:0.12em;text-transform:uppercase">${esc(name)}</text>
      <rect x="${chipX}" y="${(B - LP * 1.2) / 2}" width="${cw}" height="${LP * 1.2}" fill="${C.raised}" stroke="${C.line2}"/>
      <text x="${chipX + cw / 2}" y="${B / 2 + LP * 0.35}" text-anchor="middle" fill="${C.dim}" style="font:700 ${LP}px 'Barlow Semi Condensed'">${o.deck}</text>
      ${icon}
      <rect x="${o.red ? 1.5 : 0.5}" y="${o.red ? 1.5 : 0.5}" width="${W - (o.red ? 3 : 1)}" height="${H - (o.red ? 3 : 1)}" fill="none" stroke="${edge}" stroke-width="${o.red ? 3 : 1}"/>
    </svg>`;
  }

  window.MiniMap = { MINI, C, plan, rooms, roomAt, exits, doors, svg };
})();
