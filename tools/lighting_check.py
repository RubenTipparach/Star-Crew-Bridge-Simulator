#!/usr/bin/env python3
"""Validate the lighting data: data/lighting/fixtures.json and data/lighting/bake.json.

They are the one source for what a light is (a fixture type's photometry and where its colour per
lighting state comes from) and how a bake is made (openspec/changes/light-baking, design sections
11 and 15). The mockups read them today and sc-tools bake will read them; a misspelt key that
silently does nothing is the bug this check exists to prevent (CLAUDE.md 6.5: an unknown key is an
error, a present zero is zero, a non-finite number stops the load with its path and field).

It is documentation tooling (CLAUDE.md section 4): it changes nothing.

Checks:
  - each file's schema, and no key outside the ones listed here (keys starting with _ are notes);
  - every number finite, non-negative where a negative means nothing, and in its range;
  - a fixture type's kind (point or area) and exactly that kind's keys; light (lamp, strip, kick,
    role, fixed) with color_srgb exactly when fixed; emergency (bus or always);
  - bake.json's presets override only keys bake.json has, with values of the same type;
  - max_added_triangles has a default.

Usage: python3 tools/lighting_check.py [--quiet]. Exit status 1 on failure. Standard library only.
"""

import json
import math
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FIXTURES = os.path.join(ROOT, "data", "lighting", "fixtures.json")
BAKE = os.path.join(ROOT, "data", "lighting", "bake.json")

LIGHTS = ("lamp", "strip", "kick", "role", "fixed")
EMERGENCY = ("bus", "always")
COMMON = {"kind", "light", "emergency", "range_m"}
OPTIONAL = {"emergency_scale", "color_srgb"}
KIND_KEYS = {
    "point": {"intensity_cd", "radius_m", "beam_exponent", "drop_m"},
    "area": {"luminance_cd_m2"},
}
AREA_OPTIONAL = {"width_m", "end_clear_m"}

BAKE_KEYS = {
    "seed": (int, 0, None), "reference_lux": (float, 1e-6, None), "ambient_scale_lux": (float, 0, None),
    "shadow_samples": (int, 1, 256), "emitter_sample_area_m2": (float, 1e-4, 10), "emitter_sample_spacing_m": (float, 0.01, 10),
    "ao_rays": (int, 0, 1024), "ao_radius_m": (float, 0, 100), "exterior_ao_radius_m": (float, 0, 100), "bounces": (int, 0, 2),
    "cache_spacing_m": (float, 0.05, 10), "cache_gather_rays": (int, 1, 4096), "cache_filter_passes": (int, 0, 8),
    "gather_rays": (int, 1, 4096), "ray_bias_m": (float, 0, 0.1), "sample_inset_m": (float, 0, 0.5), "dither": (bool, None, None),
    "lightmap_texel_m": (float, 0.01, 4),
}
BAKE_OBJECTS = {
    "adaptive": {"base_m": (float, 0.1, 10), "min_m": (float, 0.01, 10), "max_error_levels": (float, 0.1, 255)},
    "mockup": {"cell_m": (float, 0.1, 10), "uncapped_triangles": (int, 0, None)},
    "probes": {"spacing_m": (float, 0.1, 10), "wall_offset_m": (float, 0, 10), "invalid_backface_fraction": (float, 0, 1)},
    "power_loss": {"flicker_s": (float, 0, 60), "fade_s": (float, 0, 60)},
}

errors = []


def fail(where, msg):
    errors.append(f"{where}: {msg}")


def keys(obj, required, optional, where):
    if not isinstance(obj, dict):
        fail(where, "must be an object")
        return False
    extra = sorted(k for k in obj if not k.startswith("_") and k not in required and k not in optional)
    missing = sorted(k for k in required if k not in obj)
    if extra:
        fail(where, "unknown key " + ", ".join(extra))
    if missing:
        fail(where, "missing " + ", ".join(missing))
    return not extra and not missing


def number(v, where, kind=float, lo=None, hi=None):
    if kind is bool:
        if not isinstance(v, bool):
            fail(where, f"must be true or false, got {v!r}")
        return
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v):
        fail(where, f"must be a finite number, got {v!r}")
        return
    if kind is int and not float(v).is_integer():
        fail(where, f"must be a whole number, got {v}")
    if (lo is not None and v < lo) or (hi is not None and v > hi):
        fail(where, f"must be in {lo}-{'any' if hi is None else hi}, got {v}")


def check_fixtures(d):
    where = os.path.relpath(FIXTURES, ROOT)
    if d.get("schema") != "starcrew.fixtures/1":
        fail(where, "schema must be starcrew.fixtures/1")
    keys(d, {"schema", "types"}, {"status"}, where)
    types = d.get("types") or {}
    if not types:
        fail(where, "types is empty")
    for name, t in types.items():
        w = f"{where} types.{name}"
        if not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            fail(w, "a type's name is lower_snake_case")
        kind = t.get("kind") if isinstance(t, dict) else None
        if kind not in KIND_KEYS:
            fail(w, f"kind must be point or area, got {kind!r}")
            continue
        req = COMMON | KIND_KEYS[kind]
        opt = OPTIONAL | (AREA_OPTIONAL if kind == "area" else set())
        if not keys(t, req, opt, w):
            continue
        if t["light"] not in LIGHTS:
            fail(w, f"light must be one of {', '.join(LIGHTS)}, got {t['light']!r}")
        if (t["light"] == "fixed") != ("color_srgb" in t):
            fail(w, "color_srgb is given exactly when light is fixed")
        if "color_srgb" in t and not re.fullmatch(r"#[0-9a-f]{6}", str(t["color_srgb"])):
            fail(w, f"color_srgb must be #rrggbb in lower case, got {t['color_srgb']!r}")
        if t["emergency"] not in EMERGENCY:
            fail(w, f"emergency must be bus or always, got {t['emergency']!r}")
        number(t["range_m"], w + ".range_m", float, 0.1, 100)
        number(t.get("emergency_scale", 1), w + ".emergency_scale", float, 0, 10)
        if kind == "point":
            number(t["intensity_cd"], w + ".intensity_cd", float, 0, 1e6)
            number(t["radius_m"], w + ".radius_m", float, 0, 5)
            number(t["beam_exponent"], w + ".beam_exponent", float, 0, 200)
            number(t["drop_m"], w + ".drop_m", float, 0, 1)
        else:
            number(t["luminance_cd_m2"], w + ".luminance_cd_m2", float, 0, 1e6)
            for k in AREA_OPTIONAL & set(t):
                number(t[k], f"{w}.{k}", float, 0, 10)


def check_bake(d):
    where = os.path.relpath(BAKE, ROOT)
    if d.get("schema") != "starcrew.bake/1":
        fail(where, "schema must be starcrew.bake/1")
    all_keys = set(BAKE_KEYS) | set(BAKE_OBJECTS) | {"schema", "max_added_triangles", "presets"}
    keys(d, all_keys, {"status"}, where)
    for k, (kind, lo, hi) in BAKE_KEYS.items():
        if k in d:
            number(d[k], f"{where} {k}", kind, lo, hi)
    for obj, spec in BAKE_OBJECTS.items():
        o = d.get(obj)
        if keys(o, set(spec), set(), f"{where} {obj}"):
            for k, (kind, lo, hi) in spec.items():
                number(o[k], f"{where} {obj}.{k}", kind, lo, hi)
    a = d.get("adaptive") or {}
    if isinstance(a, dict) and a.get("min_m", 0) > a.get("base_m", 0):
        fail(f"{where} adaptive", "min_m must not exceed base_m")
    m = d.get("max_added_triangles")
    if not isinstance(m, dict) or "default" not in m:
        fail(f"{where} max_added_triangles", "must be an object with a default")
    else:
        for k, v in m.items():
            if not k.startswith("_"):
                number(v, f"{where} max_added_triangles.{k}", int, 0, None)
    for name, p in (d.get("presets") or {}).items():
        w = f"{where} presets.{name}"
        if not isinstance(p, dict):
            fail(w, "must be an object")
            continue
        for k, v in p.items():
            if k.startswith("_"):
                continue
            if k in BAKE_KEYS:
                kind, lo, hi = BAKE_KEYS[k]
                number(v, f"{w}.{k}", kind, lo, hi)
            elif k in BAKE_OBJECTS and isinstance(v, dict):
                for kk, vv in v.items():
                    if kk not in BAKE_OBJECTS[k]:
                        fail(f"{w}.{k}", f"unknown key {kk}")
                    else:
                        kind, lo, hi = BAKE_OBJECTS[k][kk]
                        number(vv, f"{w}.{k}.{kk}", kind, lo, hi)
            else:
                fail(w, f"overrides {k}, which bake.json does not have")


def main():
    quiet = "--quiet" in sys.argv[1:]
    for path, check in ((FIXTURES, check_fixtures), (BAKE, check_bake)):
        try:
            with open(path, encoding="utf-8") as f:
                check(json.load(f))
        except (OSError, json.JSONDecodeError) as e:
            fail(os.path.relpath(path, ROOT), f"cannot be read: {e}")
    if errors:
        for e in errors:
            print("FAIL", e)
        sys.exit(1)
    if not quiet:
        with open(FIXTURES, encoding="utf-8") as f:
            n = len(json.load(f)["types"])
        print(f"ok: {n} fixture types, bake settings valid")


if __name__ == "__main__":
    main()
