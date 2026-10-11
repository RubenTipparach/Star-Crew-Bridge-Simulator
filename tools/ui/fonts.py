#!/usr/bin/env python3
"""The engine's copy of the consoles' typeface: Barlow Semi Condensed as TrueType for egui.

The console mockups (docs/mockups/consoles.html) set every word in Barlow Semi Condensed 600 and
700 with `font-variant-numeric: tabular-nums`, and the browser kerns them from the font's GPOS
table. The engine draws its consoles with egui, which reads a TrueType file through ab_glyph:
no woff2, no OpenType features, and kerning only from the legacy `kern` table. So this tool bakes
what the browser does into the file itself (openspec/changes/console-parity, design 3):

- woff2 to TrueType (the glyph outlines are unchanged);
- tabular figures: the digits' cmap entries point at their `.tf` glyphs, as `tnum` would;
- kerning: every pair the GPOS `kern` feature gives a non-zero first-glyph x advance, over the
  glyphs the cmap reaches, written as a format 0 `kern` table.

Writes `assets/fonts/barlow-semi-condensed/<weight>.ttf` beside each woff2. Deterministic: the
same woff2 gives the same bytes. `--check` fails when a committed ttf is not what this writes.

Needs fontTools and brotli (pip install fonttools brotli).
"""
import argparse
import hashlib
import io
import sys
from pathlib import Path

from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables._k_e_r_n import KernTable_format_0

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "assets" / "fonts" / "barlow-semi-condensed"
FACES = ["semibold-600", "bold-700"]


def kern_lookups(font):
    """The GPOS lookups the `kern` feature uses, in lookup order."""
    gpos = font["GPOS"].table
    idx = set()
    for fr in gpos.FeatureList.FeatureRecord:
        if fr.FeatureTag == "kern":
            idx.update(fr.Feature.LookupListIndex)
    return [gpos.LookupList.Lookup[i] for i in sorted(idx)]


def pair_subtables(lookup):
    """A lookup's PairPos subtables, through Extension lookups."""
    for st in lookup.SubTable:
        if lookup.LookupType == 9:
            if st.ExtensionLookupType != 2:
                continue
            st = st.ExtSubTable
        elif lookup.LookupType != 2:
            continue
        yield st


def x_advance(v):
    return getattr(v, "XAdvance", 0) or 0 if v is not None else 0


def gpos_pairs(font, glyphs):
    """{(left, right): units} for every pair of `glyphs` the kern feature adjusts; the first subtable to cover a
    pair decides it, as a shaper does."""
    pairs = {}
    seen = set()
    for lookup in kern_lookups(font):
        for st in pair_subtables(lookup):
            cov = [g for g in st.Coverage.glyphs if g in glyphs]
            if st.Format == 1:
                for i, left in enumerate(st.Coverage.glyphs):
                    if left not in glyphs:
                        continue
                    for rec in st.PairSet[i].PairValueRecord:
                        key = (left, rec.SecondGlyph)
                        if rec.SecondGlyph not in glyphs or key in seen:
                            continue
                        seen.add(key)
                        v = x_advance(rec.Value1)
                        if v:
                            pairs[key] = v
            elif st.Format == 2:
                c1 = st.ClassDef1.classDefs if st.ClassDef1 else {}
                c2 = st.ClassDef2.classDefs if st.ClassDef2 else {}
                for left in cov:
                    row = st.Class1Record[c1.get(left, 0)]
                    for right in glyphs:
                        key = (left, right)
                        if key in seen:
                            continue
                        rec = row.Class2Record[c2.get(right, 0)]
                        v = x_advance(rec.Value1)
                        # A class pair covers the pair even at zero: it stops later subtables deciding it.
                        seen.add(key)
                        if v:
                            pairs[key] = v
    return pairs


def build(woff2: Path) -> bytes:
    # Keep the woff2's head timestamps, so the bytes do not depend on when this ran.
    font = TTFont(woff2, recalcTimestamp=False)
    font.flavor = None
    cmap = font.getBestCmap()
    order = set(font.getGlyphOrder())
    # Tabular figures: the digits as `tnum` gives them.
    remap = {}
    for d in "0123456789":
        g = cmap[ord(d)]
        tf = g + ".tf"
        if tf not in order:
            sys.exit(f"{woff2.name}: no tabular glyph {tf}")
        remap[ord(d)] = tf
    for table in font["cmap"].tables:
        if table.isUnicode():
            for cp, g in remap.items():
                if cp in table.cmap:
                    table.cmap[cp] = g
    reach = sorted(set(font.getBestCmap().values()))
    pairs = gpos_pairs(font, set(reach))
    kern = newTable("kern")
    kern.version = 0
    sub = KernTable_format_0()
    sub.version = 0
    sub.coverage = 1
    sub.format = 0
    sub.kernTable = dict(sorted(pairs.items()))
    kern.kernTables = [sub]
    font["kern"] = kern
    if len(pairs) * 6 + 14 > 0xFFFF:
        sys.exit(f"{woff2.name}: {len(pairs)} kern pairs overflow one format 0 subtable")
    out = io.BytesIO()
    font.save(out, reorderTables=True)
    return out.getvalue()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true", help="fail if a committed ttf differs from what this writes")
    a = ap.parse_args()
    bad = 0
    for face in FACES:
        data = build(DIR / f"{face}.woff2")
        dst = DIR / f"{face}.ttf"
        if a.check:
            if not dst.exists() or dst.read_bytes() != data:
                print(f"STALE {dst.relative_to(ROOT)}: run tools/ui/fonts.py")
                bad += 1
            continue
        dst.write_bytes(data)
        print(f"{dst.relative_to(ROOT)}  {len(data)} bytes  sha256 {hashlib.sha256(data).hexdigest()[:16]}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
