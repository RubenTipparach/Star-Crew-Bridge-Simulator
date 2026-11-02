#!/usr/bin/env python3
"""concord_consoles.py: docs/mockups/consoles-concord.html, the consoles in the Concord skin (openspec/changes/concord-skin
design 6), made from docs/mockups/consoles.html so the layouts cannot drift from the approved ones: Concord's palette and
Antonio, every panel framed by the sweep, keys as pills, the title band black with pill chips, the live colour Sky on
every console, and the stations' role colours kept off red and green.

Usage (from the repository root): python3 tools/mockups/concord_consoles.py, then
python3 tools/mockups/inline.py docs/mockups/consoles-concord.html
"""
import re
s = open('docs/mockups/consoles.html').read()
n0 = len(s)
def sub1(old, new):
    global s
    assert old in s, old[:80]
    s = s.replace(old, new, 1)

sub1('<title>', '<title>Concord ')  # "Concord <old title>"
# Antonio beside Barlow; the console's face is Antonio.
sub1('<!-- INLINE font:barlow-semi-condensed END -->', '<!-- INLINE font:barlow-semi-condensed END -->\n<!-- INLINE font:antonio BEGIN -->\n<!-- INLINE font:antonio END -->')
sub1('--accent: #4fc3f7; --font: "Barlow Semi Condensed", "Arial Narrow", "Roboto Condensed", sans-serif;',
     '--accent: #3fa9f0; --font: "Antonio", "Arial Narrow", sans-serif;')
sub1('--bg: #04060a; --chrome: #0b1017; --fg: #e8eef6; --dim: #6f7f94; --line: #1f2a37;\n    --panel: #0c121a; --panel2: #121a25; --ok: #3ddc84; --warn: #ffc542; --danger: #ff4757;',
     '--bg: #000000; --chrome: #000000; --fg: #eef3ff; --dim: #9c8fd6; --line: #1b2a6b;\n    --panel: #000000; --panel2: #0b1440; --ok: #2fbf5a; --warn: #f0b43c; --danger: #e8352b;')
# The palette (concord-skin design 2).
old = re.search(r'const C = \{\n.*?\n\};', s, re.S).group(0)
s = s.replace(old, '''const C = {
  bg: "#000000", panel: "#000000", panel2: "#0b1440", line: "#1b2a6b", line2: "#2f62e0",
  text: "#eef3ff", dim: "#9c8fd6", faint: "#3b4785",
  ok: "#2fbf5a", warn: "#f0b43c", danger: "#e8352b", alert: "#e8352b", emergency: "#f0b43c",
  hostile: "#e8352b", neutral: "#f0b43c", friendly: "#2fbf5a", unknown: "#b7a6f2",
  deep: "#1b2a6b", signal: "#2f62e0", sky: "#3fa9f0", lilac: "#b7a6f2",
};''', 1)
# Role colours: red and green kept for state (design 2, CS2); a role tints its icon and its title cap only.
sub1('const ROLE = {}; for (const [k, v] of Object.entries(K.PALETTE.role)) ROLE[k] = hex(v);',
     '''const ROLE = {}; for (const [k, v] of Object.entries(K.PALETTE.role)) ROLE[k] = hex(v);
// Concord (concord-skin design 2): no role may read as a state, so red and green roles take Lilac and Sky.
Object.assign(ROLE, { helm: "#3fa9f0", tactical: "#b7a6f2", engineering: "#f0b43c", science: "#2f62e0", command: "#eef3ff", laser: "#b7a6f2", gunner: "#3fa9f0", comms: "#b7a6f2", flight_ops: "#3fa9f0" });''')
# The live colour is Sky on every console: a role is not a state.
sub1('const accent = () => ROLE[stationDef().role];', 'const accent = () => C.sky;')
# frame.panel: the sweep, a thick bar down the left into thin arms, a black notch for the title.
old = s[s.index('  s += `<rect x="0.5" y="0.5" width="${g.w - 1}" height="${g.h - 1}" rx="9" fill="${C.panel}" stroke="${o.stroke || C.line}" stroke-width="${o.stroke ? 2 : 1}"/>`;'):]
old = old[:old.index('s += text(21, 18, 13, C.dim, title, { ls: 2, w: 700 });') + len('s += text(21, 18, 13, C.dim, title, { ls: 2, w: 700 });')]
s = s.replace(old, '''  s += ccSweep(g.w, g.h, o.stroke || C.signal);
  s += text(22, 17, 14, o.titleCol || C.sky, title.toUpperCase(), { ls: 1.5, w: 700 });''', 1)
helper = '''
/** Concord's sweep (concord-skin design 4): a thick bar down the left joined by rounded elbows to thin arms along the top
 * and bottom, each arm ending in a small Sky cap; thick to thin at every turn. The title sits in the top arm's gap. */
function ccSweep(w, h, col) {
  const bar = 10, arm = 4, R = 14, r = 6, cap = 12, end = w - cap - 4, gap0 = 16, gap1 = 112;
  let d = `M${R} 0H${gap0}V${arm}H${bar + r}A${r} ${r} 0 0 0 ${bar} ${arm + r}V${h - arm - r}A${r} ${r} 0 0 0 ${bar + r} ${h - arm}H${end}V${h}H${R}A${R} ${R} 0 0 1 0 ${h - R}V${R}A${R} ${R} 0 0 1 ${R} 0Z`;
  let s = `<rect x="0" y="0" width="${w}" height="${h}" fill="#000"/>`;
  s += `<path d="${d}" fill="${col}"/>`;
  s += `<rect x="${gap1}" y="0" width="${end - gap1}" height="${arm}" fill="${col}"/>`;
  s += `<rect x="${end + 4}" y="0" width="${cap}" height="${arm}" rx="${arm / 2}" fill="${C.sky}"/><rect x="${end + 4}" y="${h - arm}" width="${cap}" height="${arm}" rx="${arm / 2}" fill="${C.sky}"/>`;
  return s;
}
'''
sub1('/** A panel: a fixed box from the grid with a one-word title; body(w, h) draws inside it. */', helper + '/** A panel: a fixed box from the grid with a one-word title; body(w, h) draws inside it. */')
# button: the pill. Lit keys are Sky with black labels; keys at rest are Deep with Text labels (a dense console stays calm).
old_btn = s[s.index('function button(b) {'):]
old_btn = old_btn[:old_btn.index('  const cy = b.y + b.h / 2;')]
s = s.replace(old_btn, '''function button(b) {
  const col = b.off ? C.faint : b.on ? "#000" : b.col2 || C.text;
  const fill = b.off ? "transparent" : b.on ? (b.col || C.sky) : C.deep;
  const rx = Math.min(b.h, b.w) / 2;
  let s = `<g ${b.act && !b.off ? `data-act="${b.act}"` : ""} ${b.hold && !b.off ? `data-hold="${b.hold}"` : ""}${tipAttr(b.tip)}>`;
  s += `<rect x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}" rx="${rx}" fill="${fill}" stroke="${b.off ? C.deep : "none"}" stroke-width="1.5"/>`;
  if (b.hold && HOLD.key === b.hold) {
    const p = clamp((now() - HOLD.t0) / HOLD_S, 0, 1);
    s += `<rect x="${b.x}" y="${b.y}" width="${b.w * p}" height="${b.h}" rx="${rx}" fill="${b.col || C.danger}" opacity="0.45"/>`;
  }
''', 1)
# The title band: black, its chips pills.
s = s.replace('s += `<rect width="1280" height="32" fill="${emerg ? "#2b1a08" : red ? "#3a0c10" : "#0a0f16"}"/>`;',
              's += `<rect width="1280" height="32" fill="#000"/><rect x="0" y="28" width="1280" height="4" rx="2" fill="${emerg ? C.emergency : red ? C.alert : C.signal}"/>`;', 1)
s = s.replace('s += `<rect y="31" width="1280" height="1" fill="${emerg ? C.emergency : red ? C.alert : C.line}"/>`;', '', 1)
s = s.replace('height="24" rx="5"', 'height="22" rx="11"')
s = s.replace('font-family="Barlow Semi Condensed"', 'font-family="Antonio"')
open('docs/mockups/consoles-concord.html', 'w').write(s)
print(n0, len(s))
