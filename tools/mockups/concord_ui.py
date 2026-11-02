#!/usr/bin/env python3
"""concord_ui.py: docs/mockups/concord-ui.html, every screen of docs/mockups/ui-art-direction.html in the Concord skin
(openspec/changes/concord-skin), made from that page so the layouts cannot drift from the approved ones: Concord's
tokens and Antonio, the sweep on large panels and insets on small ones, pill menus and keys, round lamps and pill pips.
The console screen is the Concord Helm, captured from docs/mockups/consoles-concord.html.

Usage (from the repository root):
  node tools/mockups/console_capture.mjs docs/mockups/consoles-concord.html helm /tmp/helm.png
  python3 tools/mockups/concord_ui.py /tmp/helm.png
  python3 tools/mockups/inline.py docs/mockups/concord-ui.html
"""
import re
src = open('docs/mockups/ui-art-direction.html').read()
s = src

# 1. The page's own description.
s = s.replace('''<!--
  ui-art-direction.html: the UI art direction pass (openspec/changes/ui-art-direction), before and after.''','''<!--
  concord-ui.html: every screen of ui-art-direction.html in the Concord skin (openspec/changes/concord-skin).
  Made from ui-art-direction.html by tools/mockups/concord_ui.py: the same screens, layouts, positions and sizes,
  with Concord's tokens (palette, Antonio, three type sizes) and its shapes (the sweep, the pill, pill keycaps) laid
  over them by the stylesheet marked CONCORD below. A skin swaps the look, never the layout. "Before" is today's game.

  The ui-art-direction notes that follow describe the layout this page keeps.''',1)
s = s.replace('<title>UI Art Direction</title>', '<title>Concord UI</title>', 1)
s = s.replace('''  <h1>UI art direction: before and after</h1>''','''  <h1>Concord UI: every screen in the Concord skin</h1>''',1)
s = re.sub(r'(<header>\s*<h1>[^<]*</h1>\s*)<p>.*?</p>', r'''\1<p>Mockup for the OpenSpec change <code>concord-skin</code>. The owner, 2026-11-02: "Can you redo the UI? Pull in UI guide from
  fleet-command and adopt the concord skin here". Concord is Fleet Command's player-faction skin: flat back-lit vector on
  black after LCARS's principles (never its artwork), the sweep that changes thickness at every turn, round-capped pills,
  colour with one meaning each, Antonio in three sizes. The layouts are ui-art-direction's, unchanged. Before is today's
  game; After is Concord.</p>''', s, count=1, flags=re.S)

# 2. Antonio, inlined like Barlow (tools/mockups/inline.py fills it).
s = s.replace('<!-- INLINE font:barlow-semi-condensed END -->', '<!-- INLINE font:barlow-semi-condensed END -->\n<!-- INLINE font:antonio BEGIN -->\n<!-- INLINE font:antonio END -->', 1)

# 3. The tokens: Concord's palette (design 2) and the stations' role colours with red and green kept for state.
old_col = re.search(r'  colours: \{\n.*?\n  \},\n', s, re.S).group(0)
s = s.replace(old_col, '''  colours: {
    ground: "#000000", panel: "#000000", raised: "#0b1440", line: "#1b2a6b", line2: "#2f62e0",
    text: "#eef3ff", dim: "#b7a6f2", faint: "#7d72a8", key: "#000000", key_edge: "#2f62e0",
    accent: "#3fa9f0", ink: "#000000", ok: "#2fbf5a", warn: "#f0b43c", danger: "#e8352b",
    deep: "#1b2a6b", signal: "#2f62e0", lilac: "#b7a6f2",
  },
''', 1)
s = s.replace('  accent_alt: "#f2a046",', '  accent_alt: "#b7a6f2",', 1)
s = re.sub(r'  roles: \{[^}]*\},', '  roles: { helm: "#3fa9f0", tactical: "#b7a6f2", engineering: "#f0b43c", science: "#2f62e0", command: "#eef3ff", laser: "#b7a6f2", gunner: "#3fa9f0", comms: "#b7a6f2", flight_ops: "#3fa9f0" },', s, count=1)

# 4. Antonio everywhere Barlow was named, in CSS and in canvas/SVG fonts.
s = s.replace('"Barlow Semi Condensed"', '"Antonio"').replace("'Barlow Semi Condensed'", "'Antonio'")
# but keep the @font-face blocks of the Barlow inline untouched (they were renamed by the line above): restore them.
s = s.replace('@font-face { font-family: "Antonio"; font-weight: 700; font-style: normal; src: url(data:font/woff2;base64,d09GMgABAAAAAFrM', '@font-face { font-family: "Barlow Semi Condensed"; font-weight: 700; font-style: normal; src: url(data:font/woff2;base64,d09GMgABAAAAAFrM')
s = s.replace('@font-face { font-family: "Antonio"; font-weight: 600; font-style: normal; src: url(data:font/woff2;base64,d09GMgABAAAAAFnI', '@font-face { font-family: "Barlow Semi Condensed"; font-weight: 600; font-style: normal; src: url(data:font/woff2;base64,d09GMgABAAAAAFnI')

# 5. Concord's shapes (design 4), over ui-art-direction's classes. Sizes stay; only the drawing changes.
concord_css = '''<style>
  /* ===== CONCORD (openspec/changes/concord-skin design 4): the kit's shapes over ui-art-direction's classes. ===== */
  .screen { letter-spacing: 0.01em; }
  .t-title { letter-spacing: 0.06em; }
  .t-label { letter-spacing: 0.10em; }
  /* frame.panel: the sweep. A thick Signal bar down the left joined by rounded elbows to thin arms along the top and
     bottom, each arm ending in a Sky cap; thick to thin at every turn. Drawn over the panel, never inside its box. */
  .panel { position: relative; background: #000 !important; border: 0 !important; border-radius: 0; }
  .panel::before { content: ""; position: absolute; left: -2px; top: -2px; bottom: -2px; right: 22px; pointer-events: none;
    border: solid var(--signal); border-width: 10px 0 10px 26px; border-radius: 30px 0 0 30px; }
  .panel::after { content: ""; position: absolute; top: -2px; bottom: -2px; right: 0; width: 16px; pointer-events: none;
    background: linear-gradient(var(--accent), var(--accent)) top / 16px 10px no-repeat,
                linear-gradient(var(--accent), var(--accent)) bottom / 16px 10px no-repeat; border-radius: 0 5px 5px 0; }
  /* The inner elbow: the bar meets each arm in a quarter round. */
  .panel > :first-child { position: relative; }
  /* Menu items: pills, right-aligned black labels on Signal; focus is Sky (LCARS's lit key), not a bar. */
  .item { background: var(--deep); color: var(--text) !important; border-radius: 36px; justify-content: flex-end; text-align: right;
    border: 5px solid #000; background-clip: padding-box; }
  .item.focus { background: var(--accent); color: #000 !important; box-shadow: none !important; }
  .barl { box-shadow: none !important; }
  /* button.primary and button: pills. */
  .btn-primary { border-radius: 40px; background: var(--accent) !important; color: #000 !important; justify-content: flex-end; padding-right: 44px; }
  .btn-secondary { border: 0 !important; border-radius: 32px; background: var(--signal); color: #000 !important; justify-content: flex-end; padding-right: 36px; }
  /* keycap: a small pill outlined in Signal. */
  .key { background: #000 !important; border: 2px solid var(--signal) !important; border-radius: 22px !important; color: var(--text) !important; }
  .hold, .or { color: var(--dim) !important; }
</style>
'''
s = s.replace('</head>', concord_css + '</head>', 1)
# 6. The console screen: the Concord console (docs/mockups/consoles-concord.html, Helm) at the reference size, in place of
#    ui-art-direction's capture; its title band carries Leave station, the alert chip and the clock itself.
import base64, io, sys
from PIL import Image
im = Image.open(sys.argv[1]).convert('RGB')
buf = io.BytesIO(); im.save(buf, 'JPEG', quality=88)
uri = 'data:image/jpeg;base64,' + base64.b64encode(buf.getvalue()).decode('ascii')
i = s.index('<section class="scene after" data-id="console">')
j = s.index('</section>', i)
s = s[:i] + '<section class="scene after" data-id="console">\n  <div class="world"><img src="' + uri + '" alt="Helm in Concord"></div>\n' + s[j:]

# 7. Insets: a small panel (a field, a tile, a chip) is the kit's frame.inset, not a sweep (design 4). Measured in
#    reference pixels once laid out; lamps are round, seat pips are pills.
s = s.replace('</style>\n</head>', """  .panel.inset { border: 2px solid var(--deep) !important; border-radius: 14px !important; }
  .panel.inset::before, .panel.inset::after { display: none; }
  [style*="rotate(45deg)"] { border-radius: 50% !important; transform: none !important; }
  [style*="width:12px;height:20px"] { border-radius: 6px !important; }
</style>
<script>
  // concord-skin design 4: the sweep frames a group of controls; a field or a tile is an inset.
  function concordInsets() { document.querySelectorAll(".panel").forEach((p) => p.classList.toggle("inset", p.offsetHeight < 200 || p.offsetWidth < 240)); }
  addEventListener("DOMContentLoaded", () => { concordInsets(); new MutationObserver(concordInsets).observe(document.body, { subtree: true, attributes: true, attributeFilter: ["class"] }); });
</script>
</head>""", 1)
open('docs/mockups/concord-ui.html', 'w').write(s)
print(len(src), len(s))
