# Barlow Semi Condensed

The console mockups' typeface (`docs/mockups/consoles.html`, bridge-stations section 8.0):
condensed, so a word fits a small button, with clear digits.

| File | Weight | Source | sha256 |
| --- | --- | --- | --- |
| `semibold-600.woff2` | 600 | Google Fonts, Latin subset, v16: `fonts.gstatic.com/s/barlowsemicondensed/v16/wlpigxjLBV1hqnzfr-F8sEYMB0Yybp0mudRfp66_B2sl.woff2` | `f158417e9207b5362f9b71a2fe779ce5bb836ad972f38445c3163af39d2c998d` |
| `bold-700.woff2` | 700 | Google Fonts, Latin subset, v16: `fonts.gstatic.com/s/barlowsemicondensed/v16/wlpigxjLBV1hqnzfr-F8sEYMB0Yybp0mudRfw6-_B2sl.woff2` | `fb958c8c20a05552ac8a85d925d96028d52565792650e941a5fe96b6997aa5cb` |

Fetched 2026-10-07. Copyright 2017 The Barlow Project Authors (https://github.com/jpt/barlow),
under the SIL Open Font License 1.1 (`OFL.txt`, from that repository). The pages carry the files
inline (`tools/mockups/inline.py`, the `font:` kind), so a page published as one artifact needs
no font server.

**The engine's copy** (openspec/changes/console-parity, design 3). The engine draws its consoles in this face, from
TrueType files that `tools/ui/fonts.py` writes from the two woff2 above: the same outlines, the digits' cmap entries
pointed at the tabular figures (the mockup's `tabular-nums`), and the GPOS kerning written as a legacy `kern` table,
which is the only kerning egui's ab_glyph reads. `python3 tools/ui/fonts.py --check` fails when a committed file is not
what the tool writes. The client compiles them in (`crates/sc-client/src/vg.rs`).

| File | From | sha256 |
| --- | --- | --- |
| `semibold-600.ttf` | `semibold-600.woff2` | `45051a762dc521395768194ff83bf4632fd19256084b472ebecd0171e70d589d` |
| `bold-700.ttf` | `bold-700.woff2` | `5eb144c1849374dbfe259ab4f6d8737d746f719622d6c8c463b2f2378daba5eb` |
