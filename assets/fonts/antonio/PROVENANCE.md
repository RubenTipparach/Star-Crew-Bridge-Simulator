# Antonio

The Concord skin's typeface (openspec/changes/concord-skin design 3), as Fleet Command's Concord uses it: tall
condensed capitals, so a label fits a narrow pill.

| File | Weight | Source | sha256 |
| --- | --- | --- | --- |
| `semibold-600.woff2` | 600 | google/fonts `ofl/antonio/Antonio[wght].ttf` at `6c1dbe2`, instanced at wght 600 with fontTools' `instancer`, saved as woff2 | `cf4b146ccf776096ff1c3a395d69850bba4e88805d06c1b238bd2a5744af1e72` |
| `bold-700.woff2` | 700 | the same file, instanced at wght 700 | `fdf7f8c4356d7ff7c028c787ede57f4cbec5edd60495558cea4b77e78b6df9ae` |

Fetched 2026-11-02. Copyright 2013 The Antonio Project Authors (https://github.com/googlefonts/antonioFont), under the SIL
Open Font License 1.1 (`OFL.txt`, from google/fonts at the same revision). The mockups carry the files inline, so a
page published as one artifact needs no font server.

**The engine's copy** follows when the owner approves the skin (concord-skin task 3.1): `tools/ui/fonts.py` writes
the engine's TrueType files from these two woff2 files, as it does for Barlow.
