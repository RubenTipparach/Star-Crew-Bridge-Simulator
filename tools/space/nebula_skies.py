#!/usr/bin/env python3
"""Paint the nebula skies (openspec/changes/nebula-skies) and write them as data.

The owner, 2026-10-15: "we also need some dope nebula backgrounds", "something with vertex colors like
homeworld did it". Homeworld's backgrounds were spheres of coloured vertices with no texture (the change's
design section 1 has the sources); this is our generator of the same kind of sky. It reads the painted
recipes in data/space/skies/recipes.json and writes one file a sky, data/space/skies/<id>.json:

  * a sphere of coloured vertices (design section 3): an icosphere refined where the painting changes fast,
    as Homeworld 2's background builder added vertices where the image had contrast, up to a vertex budget;
    each vertex's direction and its sRGB bytes, with a fourth byte that says how much of the stars behind
    that point the nebula hides, and the triangles' 16-bit indices;
  * the vector stars that streak when the camera turns (FreeSpace's way, design section 5): a direction, a
    brightness already dimmed by the dust in front of it, a class and a colour;
  * the ambient light the sky throws on a hull (design section 6): a six-sided ambient cube and the nine
    spherical harmonic coefficients it came from, scaled by the recipe's strength, and the dominant colour.

How a sky is painted, back to front, all in linear light, then tone mapped and written as sRGB:

  space colour, then each layer in its order:
    band  a great circle (the galaxy's disc seen edge on), its brightness varied along it by noise
    glow  a brush stroke: a soft line along a path of points on the sky, its width tapered at the ends,
          its edge broken up by warping the direction with noise, its body grained by noise; a colour at
          its heart and another at its edge
    dust  the same stroke absorbing what is behind it (reddening it), so a dark lane cuts a bright band
    core  a round glow: the bright heart of a nebula, a hot centre in a wider halo

Every vertex is the average of seven samples a prefilter radius apart, so no feature is finer than the
mesh can draw (no speckle between vertices: a sky that looks like noise looks cheap).

This is documentation tooling (CLAUDE.md section 4): it writes data the mockup reads and the engine will
read once the change is applied. Deterministic: integer hashes, no random state.

Usage: python3 tools/space/nebula_skies.py [--check] [--only id[,id]]
  --check   writes nothing; fails if a sky file is stale
"""

import argparse
import json
import math
import os
import sys

import numpy as np

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SKIES = os.path.join(ROOT, "data", "space", "skies")
RECIPES = os.path.join(SKIES, "recipes.json")
STARFIELD = os.path.join(ROOT, "data", "space", "starfield.json")
SCHEMA = "starcrew.sky/1"

# ----------------------------------------------------------------------------- the sphere


def icosphere(level):
    """The geodesic sphere of design section 3: the icosahedron's 12 vertices and 20 faces in this order,
    each face split into four `level` times, a new edge's midpoint appended the first time the edge is met
    (faces in order, edges ab, bc, ca), every vertex pushed out to the unit sphere. A level keeps every
    vertex of the level below at the same index, so the first 2,562 vertices of level 5 are level 4."""
    t = (1.0 + math.sqrt(5.0)) / 2.0
    verts = [(-1, t, 0), (1, t, 0), (-1, -t, 0), (1, -t, 0), (0, -1, t), (0, 1, t),
             (0, -1, -t), (0, 1, -t), (t, 0, -1), (t, 0, 1), (-t, 0, -1), (-t, 0, 1)]
    verts = [tuple(c / math.sqrt(1 + t * t) for c in v) for v in verts]
    faces = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4), (11, 10, 2),
             (10, 7, 6), (7, 1, 8), (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9), (4, 9, 5),
             (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
    for _ in range(level):
        cache, out = {}, []

        def mid(a, b):
            key = (a, b) if a < b else (b, a)
            if key not in cache:
                p = [(verts[a][i] + verts[b][i]) * 0.5 for i in range(3)]
                n = math.sqrt(sum(c * c for c in p))
                verts.append(tuple(c / n for c in p))
                cache[key] = len(verts) - 1
            return cache[key]

        for a, b, c in faces:
            ab, bc, ca = mid(a, b), mid(b, c), mid(c, a)
            out += [(a, ab, ca), (b, bc, ab), (c, ca, bc), (ab, bc, ca)]
        faces = out
    return np.array(verts, dtype=np.float64), np.array(faces, dtype=np.int64)


def direction(lon_deg, lat_deg):
    """A direction in the system frame: longitude from +z toward +x round +y, latitude up from the planets' plane."""
    lo, la = math.radians(lon_deg), math.radians(lat_deg)
    return np.array([math.cos(la) * math.sin(lo), math.sin(la), math.cos(la) * math.cos(lo)])


def normalize(v):
    return v / np.linalg.norm(v, axis=-1, keepdims=True)


# ----------------------------------------------------------------------------- noise (integer hashes)

M64 = np.uint64(0xFFFFFFFFFFFFFFFF)


def mix64(x):
    """splitmix64's finaliser on uint64 arrays: a stable hash, the same on every machine."""
    x = (x ^ (x >> np.uint64(30))) * np.uint64(0xBF58476D1CE4E5B9)
    x = (x ^ (x >> np.uint64(27))) * np.uint64(0x94D049BB133111EB)
    return x ^ (x >> np.uint64(31))


def hash3(i, j, k, seed):
    with np.errstate(over="ignore"):
        h = (i.astype(np.int64).astype(np.uint64) * np.uint64(0x9E3779B97F4A7C15)
             ^ j.astype(np.int64).astype(np.uint64) * np.uint64(0xC2B2AE3D27D4EB4F)
             ^ k.astype(np.int64).astype(np.uint64) * np.uint64(0x165667B19E3779F9)
             ^ np.uint64(seed) * np.uint64(0xD6E8FEB86659FD93))
        return mix64(h)


GRADS = np.array([[1, 1, 0], [-1, 1, 0], [1, -1, 0], [-1, -1, 0], [1, 0, 1], [-1, 0, 1], [1, 0, -1], [-1, 0, -1],
                  [0, 1, 1], [0, -1, 1], [0, 1, -1], [0, -1, -1], [1, 1, 0], [-1, 1, 0], [0, -1, 1], [0, -1, -1]],
                 dtype=np.float64)


def perlin(p, seed):
    """Gradient noise in 3D (Perlin's improved noise with hashed gradients), about -1 to 1."""
    i0 = np.floor(p).astype(np.int64)
    f = p - i0
    u = f * f * f * (f * (f * 6 - 15) + 10)
    out = 0.0
    with np.errstate(over="ignore"):
        corners = []
        for dx in (0, 1):
            for dy in (0, 1):
                for dz in (0, 1):
                    h = hash3(i0[:, 0] + dx, i0[:, 1] + dy, i0[:, 2] + dz, seed)
                    g = GRADS[(h & np.uint64(15)).astype(np.int64)]
                    d = f - np.array([dx, dy, dz], dtype=np.float64)
                    corners.append(((dx, dy, dz), np.sum(g * d, axis=1)))
    w = {c: v for c, v in corners}
    x0 = w[(0, 0, 0)] + u[:, 0] * (w[(1, 0, 0)] - w[(0, 0, 0)])
    x1 = w[(0, 1, 0)] + u[:, 0] * (w[(1, 1, 0)] - w[(0, 1, 0)])
    x2 = w[(0, 0, 1)] + u[:, 0] * (w[(1, 0, 1)] - w[(0, 0, 1)])
    x3 = w[(0, 1, 1)] + u[:, 0] * (w[(1, 1, 1)] - w[(0, 1, 1)])
    y0 = x0 + u[:, 1] * (x1 - x0)
    y1 = x2 + u[:, 1] * (x3 - x2)
    out = y0 + u[:, 2] * (y1 - y0)
    return out * 1.1


def fbm(p, seed, octaves=4, lacunarity=2.03, gain=0.5):
    """Fractal noise, about -1 to 1."""
    s, a, norm = 0.0, 1.0, 0.0
    for o in range(octaves):
        s = s + a * perlin(p, seed + 101 * o)
        norm += a
        p = p * lacunarity + np.array([3.1, 1.7, 5.3])
        a *= gain
    return s / norm


def warp(d, amount, scale, seed):
    """The direction pushed sideways by a noise field: a brush stroke's edge broken into wisps."""
    if amount <= 0:
        return d
    q = d * scale
    w = np.stack([fbm(q, seed, 3), fbm(q + 31.7, seed + 7, 3), fbm(q - 17.3, seed + 13, 3)], axis=1)
    return normalize(d + amount * w)


# ----------------------------------------------------------------------------- strokes


def srgb_to_linear(c):
    c = np.asarray(c, dtype=np.float64)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c):
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(c, 1 / 2.4) - 0.055)


def path_points(path_deg, n=400):
    """A brush stroke's spine: its points joined by great-circle arcs, smoothed by corner cutting on the
    sphere, resampled evenly. Returns the points and each one's share of the length (0-1)."""
    pts = [direction(*p) for p in path_deg]
    for _ in range(4):
        out = [pts[0]]
        for a, b in zip(pts[:-1], pts[1:]):
            out.append(normalize(0.75 * a + 0.25 * b))
            out.append(normalize(0.25 * a + 0.75 * b))
        out.append(pts[-1])
        pts = out
    pts = np.array(pts)
    seg = np.arccos(np.clip(np.sum(pts[:-1] * pts[1:], axis=1), -1, 1))
    cum = np.concatenate([[0.0], np.cumsum(seg)])
    total = cum[-1]
    want = np.linspace(0, total, n)
    res = []
    for s in want:
        i = min(np.searchsorted(cum, s, side="right") - 1, len(seg) - 1)
        f = 0.0 if seg[i] == 0 else (s - cum[i]) / seg[i]
        res.append(normalize((1 - f) * pts[i] + f * pts[i + 1]))
    return np.array(res), want / total


def smooth(e0, e1, x):
    t = np.clip((x - e0) / max(e1 - e0, 1e-9), 0, 1)
    return t * t * (3 - 2 * t)


def stroke_value(d, layer, seed):
    """How much of a stroke covers each direction (0-1): the angle to its spine against its tapered width."""
    dw = warp(d, layer.get("warp", 0.0), layer.get("warp_scale", 2.0), seed)
    if layer["kind"] == "core":
        c = direction(*layer["at_deg"])
        ang = np.degrees(np.arccos(np.clip(dw @ c, -1, 1)))
        r = layer["radius_deg"]
        v = np.exp(-0.5 * (ang / r) ** 2) + layer.get("halo", 0.35) * np.exp(-0.5 * (ang / (r * 3.2)) ** 2)
        return v / (1 + layer.get("halo", 0.35)), None
    pts, share = path_points(layer["path_deg"])
    dots = dw @ pts.T
    j = np.argmax(dots, axis=1)
    ang = np.degrees(np.arccos(np.clip(dots[np.arange(len(d)), j], -1, 1)))
    t = share[j]
    ta, tb = layer.get("taper", [0.25, 0.25])
    shape = smooth(0, ta, t) * smooth(0, tb, 1 - t) if (ta > 0 or tb > 0) else np.ones_like(t)
    wvar = layer.get("width_var", 0.0)
    wmod = 1.0 + wvar * fbm(np.stack([t * layer.get("width_scale", 4.0), np.zeros_like(t), np.zeros_like(t) + seed % 97], axis=1), seed + 3, 2)
    w = layer["width_deg"] * (0.35 + 0.65 * shape) * wmod
    v = np.exp(-0.5 * np.power(ang / w, layer.get("falloff", 2.0)))
    return v * (0.25 + 0.75 * shape) if (ta > 0 or tb > 0) else v, t


def band_pole(band):
    """The band's pole: given, or the one tilt_deg from north whose great circle passes through through_deg
    (the galaxy's plane is about 60 degrees from the planets' plane, as the Milky Way's is from the ecliptic)."""
    if "pole_deg" in band:
        return direction(*band["pole_deg"])
    lon, lat = band["through_deg"]
    tilt = math.radians(band["tilt_deg"])
    x = -math.tan(math.pi / 2 - tilt) * math.tan(math.radians(lat))
    if abs(x) > 1:
        sys.exit(f"band through {band['through_deg']} cannot be tilted {band['tilt_deg']} degrees")
    return direction(lon + math.degrees(math.acos(x)) * band.get("side", 1), 90 - band["tilt_deg"])


def band_value(d, band, seed):
    """The galaxy's disc: a soft band round the great circle whose pole is pole_deg, brighter toward its bulge."""
    dw = warp(d, band.get("warp", 0.05), band.get("warp_scale", 3.0), seed)
    pole = band_pole(band)
    lat = np.degrees(np.arcsin(np.clip(dw @ pole, -1, 1)))
    v = np.exp(-0.5 * (lat / band["width_deg"]) ** 2)
    bul = direction(*band.get("bulge_deg", band.get("through_deg", [0, 0])))
    ang = np.degrees(np.arccos(np.clip(dw @ bul, -1, 1)))
    v = v * (1.0 + band.get("bulge", 1.0) * np.exp(-0.5 * (ang / band.get("bulge_width_deg", 35)) ** 2))
    return v


def grain(d, layer, seed):
    """The body's variation: soft clumps (grain, from fractal noise) and strands (filaments, from ridged
    noise: one minus the noise's size, sharpened, so thin bright ridges run through the cloud)."""
    out = 1.0
    g = layer.get("grain", 0.0)
    if g > 0:
        n = fbm(d * layer.get("grain_scale", 3.0), seed + 29, layer.get("grain_octaves", 3))
        out = out * np.clip(1.0 + g * n * 1.6, 0.0, None)
    f = layer.get("filaments", 0.0)
    if f > 0:
        q = warp(d, 0.08, 2.0, seed + 71) * layer.get("filament_scale", 5.0)
        r = (1.0 - np.abs(fbm(q, seed + 53, 3))) ** layer.get("filament_power", 4.0)
        out = out * (1.0 - f + f * 2.2 * r)
    return out


def paint(d, sky):
    """The sky's linear colour at directions d, and how much it hides the stars behind (0-1)."""
    seed = sky["seed"]
    col = np.tile(srgb_to_linear(sky["space_srgb"]), (len(d), 1))
    trans = np.ones(len(d))
    glow = np.zeros(len(d))
    layers = ([dict(sky["band"], kind="band")] if "band" in sky else []) + sky.get("layers", [])
    for li, layer in enumerate(layers):
        s = seed + 1000 * (li + 1)
        if layer["kind"] == "band":
            v = band_value(d, layer, s)
        else:
            v, _ = stroke_value(d, layer, s)
        v = v * grain(d, layer, s)
        if layer["kind"] == "dust":
            tau = layer["density"] * v
            redden = np.array(layer.get("redden", [0.75, 1.0, 1.3]))
            col = col * np.exp(-tau[:, None] * redden[None, :])
            if "emit_srgb" in layer:
                col = col + srgb_to_linear(layer["emit_srgb"])[None, :] * (1 - np.exp(-tau))[:, None] * layer.get("emit", 0.02)
            trans = trans * np.exp(-tau)
            glow = glow * np.exp(-tau)
            continue
        heart = srgb_to_linear(layer["colour_srgb"])
        edge = srgb_to_linear(layer.get("edge_srgb", layer["colour_srgb"]))
        k = np.clip(v * layer.get("heart", 1.3), 0, 1) ** layer.get("heart_gamma", 1.5)
        c = edge[None, :] + (heart - edge)[None, :] * k[:, None]
        add = c * (v * layer["intensity"])[:, None]
        col = col + add
        glow = glow + v * layer["intensity"] * layer.get("hides_stars", 0.5)
    exposure = sky.get("exposure", 1.0)
    mapped = 1.0 - np.exp(-col * exposure)
    hide = 1.0 - trans * np.exp(-glow * 1.5)
    return mapped, np.clip(hide, 0, 1), trans


def tangent_ring(d, radius_deg, n=6):
    """n directions radius_deg round each of d (for the prefilter)."""
    up = np.where(np.abs(d[:, 1:2]) < 0.9, np.array([[0, 1, 0]]), np.array([[1, 0, 0]]))
    a = normalize(np.cross(d, up))
    b = np.cross(d, a)
    r = math.radians(radius_deg)
    out = []
    for i in range(n):
        th = 2 * math.pi * i / n
        out.append(normalize(d + math.tan(r) * (math.cos(th) * a + math.sin(th) * b)))
    return out


# ----------------------------------------------------------------------------- stars


def unit_floats(n, seed, purpose):
    """n numbers in [0, 1) from the hash of (seed, purpose, index)."""
    idx = np.arange(n, dtype=np.uint64)
    with np.errstate(over="ignore"):
        h = mix64(idx * np.uint64(0x9E3779B97F4A7C15) ^ np.uint64(seed * 7919 + purpose * 104729))
    return (h >> np.uint64(11)).astype(np.float64) / float(1 << 53)


def make_stars(sky, classes, colours):
    st = sky["stars"]
    n = st["count"]
    want, k, seed = [], 0, sky["seed"]
    band = sky.get("band")
    cluster = st.get("cluster")
    while len(want) < n:
        m = 4 * n
        u, v, r = unit_floats(m, seed, 10 + k), unit_floats(m, seed, 20 + k), unit_floats(m, seed, 30 + k)
        z = 2 * u - 1
        th = 2 * math.pi * v
        s = np.sqrt(1 - z * z)
        d = np.stack([s * np.cos(th), z, s * np.sin(th)], axis=1)
        if band:
            w = band_value(d, band, seed + 1000)
            w = w / max(w.max(), 1e-9)
            accept = (1 + st.get("band_bias", 0.0) * w) / (1 + st.get("band_bias", 0.0))
        else:
            accept = np.ones(m)
        want.extend(list(d[r < accept]))
        k += 1
    d = np.array(want[:n])
    if cluster:
        nc = int(n * cluster["share"])
        c = direction(*cluster["at_deg"])
        g1, g2 = unit_floats(nc, seed, 40), unit_floats(nc, seed, 41)
        ang = np.radians(cluster["radius_deg"]) * np.sqrt(-2 * np.log(np.maximum(g1, 1e-12)))
        up = np.array([0, 1, 0]) if abs(c[1]) < 0.9 else np.array([1, 0, 0])
        a = normalize(np.cross(c, up))
        b = np.cross(c, a)
        th = 2 * math.pi * g2
        d[:nc] = normalize(c[None, :] * np.cos(ang)[:, None] + (np.cos(th)[:, None] * a + np.sin(th)[:, None] * b) * np.sin(ang)[:, None])
    # The class by its share (most stars are faint), the brightness within the class, the colour by its share.
    cu, bu, ku = unit_floats(n, seed, 50), unit_floats(n, seed, 51), unit_floats(n, seed, 52)
    cls_cum = np.cumsum([c["share"] for c in classes])
    cls = np.minimum(np.searchsorted(cls_cum / cls_cum[-1], cu, side="right"), len(classes) - 1)
    lo = np.array([classes[c]["brightness"][0] for c in cls])
    hi = np.array([classes[c]["brightness"][1] for c in cls])
    b = lo + (hi - lo) * bu
    col_cum = np.cumsum([c["share"] for c in colours])
    col = np.minimum(np.searchsorted(col_cum / col_cum[-1], ku, side="right"), len(colours) - 1)
    # Dimmed by what is in front: the dust's transmission and a little for the glow they sit in.
    _, hide, trans = paint(d, sky)
    b = b * trans * (1 - 0.35 * (hide - (1 - trans)).clip(0, 1))
    order = np.argsort(-b, kind="stable")
    rows = [[round(float(d[i, 0]), 4), round(float(d[i, 1]), 4), round(float(d[i, 2]), 4), round(float(b[i]), 3),
             int(cls[i]), int(col[i])] for i in order]
    return rows


# ----------------------------------------------------------------------------- ambient

SH_C = [0.282095, 0.488603, 0.488603, 0.488603, 1.092548, 1.092548, 0.315392, 1.092548, 0.546274]


def sh_basis(d):
    x, y, z = d[:, 0], d[:, 1], d[:, 2]
    return np.stack([np.full_like(x, SH_C[0]), SH_C[1] * y, SH_C[2] * z, SH_C[3] * x, SH_C[4] * x * y, SH_C[5] * y * z,
                     SH_C[6] * (3 * z * z - 1), SH_C[7] * x * z, SH_C[8] * (x * x - y * y)], axis=1)


def irradiance(sh, n):
    """Ramamoorthi and Hanrahan's irradiance from nine radiance coefficients, divided by pi: the colour a
    white Lambert surface facing n takes."""
    A = np.array([math.pi, 2 * math.pi / 3, 2 * math.pi / 3, 2 * math.pi / 3, math.pi / 4, math.pi / 4, math.pi / 4, math.pi / 4, math.pi / 4])
    return (sh_basis(n[None, :])[0] * A) @ sh / math.pi


# ----------------------------------------------------------------------------- the adaptive mesh


def sample(dirs, sky):
    """The prefiltered sky at dirs: the average of the direction and six round it, prefilter_deg away."""
    lin, hide = 0.0, 0.0
    rings = [dirs] + tangent_ring(dirs, sky["prefilter_deg"])
    for s in rings:
        c, h, _ = paint(s, sky)
        lin, hide = lin + c, hide + h
    lin, hide = lin / len(rings), hide / len(rings)
    return np.concatenate([linear_to_srgb(lin) * 255.0, np.clip(hide, 0, 1)[:, None] * 255.0], axis=1)


class Mesh:
    """A sphere refined where the painting needs it (design section 3): the icosphere of `base` levels, then
    edges split at their midpoints, worst first, while the colour at an edge's midpoint differs from what the
    GPU would interpolate there by more than the threshold, until the vertex budget is spent. An edge is
    split only when it is the longest edge of both its triangles (Rivara's longest-edge bisection: a
    neighbour's longest edge is split first), so the mesh never has a crack or a T-junction and its
    triangles never grow thin."""

    def __init__(self, sky):
        self.sky = sky
        V, F = icosphere(sky["base_subdivisions"])
        self.v = [V[i] for i in range(len(V))]
        self.col = list(sample(V, sky))
        self.tris = {}
        self.edges = {}
        self.next_id = 0
        for t in F:
            self.add_tri(tuple(int(x) for x in t))
        self.mid = {}   # edge key -> the sky at its midpoint (sRGB 0-255 and the hide byte)

    @staticmethod
    def key(a, b):
        return (a, b) if a < b else (b, a)

    def add_tri(self, t):
        i = self.next_id
        self.next_id += 1
        self.tris[i] = t
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            self.edges.setdefault(self.key(a, b), set()).add(i)
        return i

    def drop_tri(self, i):
        t = self.tris.pop(i)
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            k = self.key(a, b)
            s = self.edges[k]
            s.discard(i)
            if not s:
                del self.edges[k]

    def length(self, k):
        return float(np.arccos(np.clip(np.dot(self.v[k[0]], self.v[k[1]]), -1, 1)))

    def longest(self, i):
        t = self.tris[i]
        ks = [self.key(t[0], t[1]), self.key(t[1], t[2]), self.key(t[2], t[0])]
        return max(ks, key=lambda k: (round(self.length(k), 9), k))

    def split(self, k, pending):
        """Split edge k (after its neighbours' longer edges), its midpoint's colour filled in later."""
        guard = 0
        while True:
            guard += 1
            if guard > 64 or k not in self.edges:
                return
            longer = [self.longest(i) for i in self.edges[k] if self.longest(i) != k]
            if not longer:
                break
            self.split(longer[0], pending)
        a, b = k
        m = len(self.v)
        self.v.append(normalize(self.v[a] + self.v[b]))
        self.col.append(None)
        pending.append(m)
        for i in list(self.edges[k]):
            t = self.tris[i]
            r = t.index(a)
            t = t[r:] + t[:r]          # rotate so the triangle starts at a
            if t[1] == b:              # (a, b, c)
                c = t[2]
                self.drop_tri(i)
                self.add_tri((a, m, c))
                self.add_tri((m, b, c))
            else:                      # (a, c, b)
                c = t[1]
                self.drop_tri(i)
                self.add_tri((a, c, m))
                self.add_tri((m, c, b))

    def errors(self):
        """Every edge's error: the largest difference, in bytes, between the sky at its midpoint and the
        mean of its ends (the alpha byte counts half)."""
        keys = [k for k in self.edges if k not in self.mid]
        if keys:
            dirs = np.array([normalize(self.v[a] + self.v[b]) for a, b in keys])
            vals = sample(dirs, self.sky)
            for k, val in zip(keys, vals):
                self.mid[k] = val
        out = []
        min_len = math.radians(self.sky["min_edge_deg"])
        w = np.array([1.0, 1.0, 1.0, 0.5])
        for k in self.edges:
            if self.length(k) < 2 * min_len:
                continue
            lerp = (self.col[k[0]] + self.col[k[1]]) * 0.5
            e = float(np.max(np.abs(self.mid[k] - lerp) * w))
            out.append((e, k))
        out.sort(key=lambda x: (-x[0], x[1]))
        return out

    def refine(self):
        budget, thr = self.sky["vertex_budget"], self.sky["error_bytes"]
        while len(self.v) < budget:
            errs = [x for x in self.errors() if x[0] > thr]
            if not errs:
                break
            room = budget - len(self.v)
            take = errs[:max(1, min(len(errs), room // 3))]
            pending = []
            for e, k in take:
                if len(self.v) >= budget - 8:   # a split may split a few neighbours first: stop short of the budget
                    break
                if k in self.edges:
                    known = self.mid.get(k)
                    before = len(self.v)
                    self.split(k, pending)
                    if known is not None and len(self.v) > before and k not in self.edges:
                        # The edge itself was split last: its midpoint's colour is already known.
                        self.col[len(self.v) - 1] = known
            todo = [m for m in pending if self.col[m] is None]
            if todo:
                vals = sample(np.array([self.v[m] for m in todo]), self.sky)
                for m, val in zip(todo, vals):
                    self.col[m] = val
            if not pending:
                break


def oct_encode(d):
    """A unit direction as two 16-bit numbers (octahedral mapping): about 0.003 degrees of error."""
    p = d[:, :2] / np.sum(np.abs(d), axis=1, keepdims=True)
    neg = d[:, 2] < 0
    q = (1 - np.abs(p[:, ::-1])) * np.where(p >= 0, 1.0, -1.0)
    p = np.where(neg[:, None], q, p)
    return np.round((p * 0.5 + 0.5) * 65535).astype(np.uint16)


def oct_decode(u):
    p = u.astype(np.float64) / 65535 * 2 - 1
    z = 1 - np.abs(p[:, 0]) - np.abs(p[:, 1])
    t = np.clip(-z, 0, None)
    x = p[:, 0] + np.where(p[:, 0] >= 0, -t, t)
    y = p[:, 1] + np.where(p[:, 1] >= 0, -t, t)
    return normalize(np.stack([x, y, z], axis=1))


def b64_rows(raw, row_bytes):
    import base64
    return [base64.b64encode(raw[i:i + row_bytes]).decode("ascii") for i in range(0, len(raw), row_bytes)]


# ----------------------------------------------------------------------------- one sky


def build(sky, defaults, star_classes, star_colours):
    sky = {**defaults, **sky}
    mesh = Mesh(sky)
    mesh.refine()
    V = np.array(mesh.v)
    rgba = np.clip(np.round(np.array(mesh.col)), 0, 255).astype(np.uint8)
    # Triangles facing inward, toward the eye at the centre, in a stable order.
    tris = []
    for i in sorted(mesh.tris):
        a, b, c = mesh.tris[i]
        n = np.cross(V[b] - V[a], V[c] - V[a])
        tris.append((a, c, b) if np.dot(n, V[a] + V[b] + V[c]) > 0 else (a, b, c))
    T = np.array(tris, dtype=np.uint16)
    enc = oct_encode(V)
    vbytes = np.zeros((len(V), 8), dtype=np.uint8)
    vbytes[:, 0:4] = enc.astype("<u2").view(np.uint8).reshape(-1, 4)
    vbytes[:, 4:8] = rgba
    # The ambient: the stored colours (as the engine will see them) projected onto nine coefficients, each
    # vertex weighted by a third of the area of the triangles round it.
    Vd = oct_decode(enc)
    area = np.zeros(len(V))
    for a, b, c in tris:
        ar = 0.5 * np.linalg.norm(np.cross(Vd[b] - Vd[a], Vd[c] - Vd[a]))
        area[[a, b, c]] += ar / 3
    area *= 4 * math.pi / area.sum()
    back = srgb_to_linear(rgba[:, :3] / 255.0)
    sh = (sh_basis(Vd).T * area) @ back
    strength = sky["ambient"]["strength"]
    axes = [np.array(a, dtype=np.float64) for a in ([1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1])]
    # Normalised so the brightest face's luminance is the strength: the sky sets the ambient's colour and the
    # way it leans, the recipe how strong it is (Homeworld's lighting files: never more than 40% ambient).
    faces = [np.clip(irradiance(sh, a), 0, None) for a in axes]
    peak = max(float(f @ np.array([0.2126, 0.7152, 0.0722])) for f in faces)
    scale = strength / max(peak, 1e-9)
    cube = [[round(float(x), 4) for x in f * scale] for f in faces]
    lum = back @ np.array([0.2126, 0.7152, 0.0722])
    dom = (back * (lum * area)[:, None]).sum(axis=0) / max((lum * area).sum(), 1e-9)
    dom = dom / max(dom.max(), 1e-9)
    stars = make_stars(sky, star_classes, star_colours)
    return {
        "schema": SCHEMA,
        "_doc": ("A nebula sky (openspec/changes/nebula-skies). Written by tools/space/nebula_skies.py from "
                 "data/space/skies/recipes.json: edit the recipe, then run the tool; never edit this file. Frame: the "
                 "system's (+y north, the planets in the xz plane). mesh: vertices_b64 is 8 bytes a vertex (its "
                 "direction as two little-endian 16-bit octahedral coordinates, then red, green and blue as sRGB bytes "
                 "and the share of the stars behind it the nebula hides, 0-255); triangles_b64 is three little-endian "
                 "16-bit indices a triangle, wound counter-clockwise seen from the centre; both base64, cut into rows. "
                 "stars: [x, y, z, brightness 0-1 already dimmed by the dust in front, class, colour], brightest first. "
                 "ambient: the linear RGB a white surface takes facing +x, -x, +y, -y, +z, -z, scaled so the "
                 "brightest face's luminance is the strength, and the nine radiance coefficients it came from, scaled the "
                 "same way."),
        "id": sky["id"], "label": sky["label"], "name": sky["name"], "for": sky["for"], "look": sky["look"],
        "mesh": {"vertices": int(len(V)), "triangles": int(len(T)), "base_subdivisions": sky["base_subdivisions"],
                 "vertex_budget": sky["vertex_budget"], "error_bytes": sky["error_bytes"],
                 "vertices_b64": b64_rows(vbytes.tobytes(), 8 * 96),
                 "triangles_b64": b64_rows(T.astype("<u2").tobytes(), 6 * 128)},
        "dust_stars": sky["dust_stars"],
        "stars": {"count": len(stars), "list": stars},
        "ambient": {"strength": strength, "cube_linear": cube,
                    "sh9_linear": [[round(float(x) * scale, 5) for x in row] for row in sh],
                    "dominant_srgb": [round(float(x), 3) for x in linear_to_srgb(dom)]},
    }


def dumps(obj):
    """JSON with one star a line and one hex row a line: diffable."""
    s = json.dumps(obj, indent=2, ensure_ascii=False)
    return s + "\n"


def compact_lists(text):
    import re
    # Put each star's six numbers and each short number list on one line.
    def join(m):
        inner = re.sub(r"\s+", " ", m.group(0))
        return inner.replace("[ ", "[").replace(" ]", "]")
    return re.sub(r"\[\s*(-?[0-9.e+-]+,\s*)+-?[0-9.e+-]+\s*\]", join, text)


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--only", default=None)
    a = ap.parse_args(argv)
    with open(RECIPES, encoding="utf-8") as f:
        R = json.load(f)
    if R.get("schema") != "starcrew.sky-recipes/1":
        sys.exit(f"{RECIPES}: schema must be starcrew.sky-recipes/1")
    with open(STARFIELD, encoding="utf-8") as f:
        SF = json.load(f)
    ok = True
    for sky in R["skies"]:
        if a.only and sky["id"] not in a.only.split(","):
            continue
        out = build(sky, R["defaults"], SF["classes"], SF["colours"])
        text = compact_lists(dumps(out))
        path = os.path.join(SKIES, sky["id"] + ".json")
        rel = os.path.relpath(path, ROOT)
        old = open(path, encoding="utf-8").read() if os.path.exists(path) else None
        if a.check:
            if old != text:
                print(f"  FAIL {rel}: stale; run python3 tools/space/nebula_skies.py")
                ok = False
            else:
                print(f"  {rel}: current")
            continue
        if old != text:
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
        m = out["mesh"]
        print(f"  {rel}: {m['vertices']} vertices, {m['triangles']} triangles, {out['stars']['count']} stars, "
              f"{len(text) // 1024} KB, ambient {out['ambient']['dominant_srgb']}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
