# Design: the consoles draw the Tern's low poly model

## Context

| Decided elsewhere | Where |
| --- | --- |
| The camera widget and the shield view | `bridge-stations` 8, `weapons-and-shields` 11, the approved `consoles.html` |
| The Tern's exterior and its levels of detail; the model's origin and axes are the layout's | `tern-exterior`, `assets/models/exteriors/props.json` |
| The shield ellipsoid: the smallest that holds the hull loft, the mounts and the pods | `weapons-and-shields` 11, `data/ships/tern/shields.json` |
| Validate the real artifact; one source for each thing | CLAUDE.md 6.1, 6.6 |

## 1. What draws it

Every console picture of the Tern in 3D takes its polygons from one function (`kit::hull_polys`): the camera widget
(beside every viewscreen) and the shield view, small and big. They draw `tern_lod2`'s 496 triangles flat, each lit by
the light the picture already uses, sorted back to front as the loft's quads were. The model is in ship axes (+x port,
+y dorsal, +z bow) with the layout's origin, so it sits where the loft did; the shield view moves it by the shield's
centre as before.

## 2. One data file

`tools/ships/console_model.py` (Python, standard library and Pillow) reads `tern_lod2.glb`: positions, indices, the
vertex colours and the UVs, and samples the atlas at each triangle's middle, times its vertex colour, for one sRGB
colour a triangle. It writes `data/ships/tern/console_model.json`:

```json
{ "schema": "starcrew.console-model/1", "source": "assets/models/exteriors/tern_lod2.glb", "sha256": "...",
  "positions_m": [[x, y, z], ...], "triangles": [[i, j, k, "#rrggbb"], ...] }
```

Positions to the centimetre. `--check` rebuilds it in memory and fails when the file differs (a new model without a
new file). The engine includes the file (`include_str!`); `tools/mockups/inline.py` copies it into `consoles.html`
between `INLINE data:tern/console_model` markers, so the mockup and the engine draw the same triangles.

## 3. The shield bubble envelops the ship

The exterior is 45 m across at its widest; the old ellipsoid's half-width was 17 m, sized to the hull loft, so the low
poly model's sides reached past the bubble (the first captures of this change). The owner, 2026-10-14: "shield should
envolop the ship entirely".

The ellipsoid becomes the smallest axis-aligned one that holds every vertex of the full exterior model (`tern.glb`,
8,730 vertices) with 1 m to spare: a search over its centre (y, z; x is 0 by symmetry) and its semi-axes, the third
semi-axis the least that holds every point for the other two, keeping the least volume. Rounded up:

| | Before | After |
| --- | --- | --- |
| Semi-axes, m | 17.0 x 13.0 x 54.0 | 27.0 x 10.5 x 59.5 |
| Centre, m | 0.0, 1.5, -2.0 | 0.0, 1.0, -11.0 |

The model's farthest vertex sits at 0.995 of the bubble shrunk by 1 m. The bubble is the shield's hit shape (a bolt
strikes the face whose patch it meets), so hits now land on the bubble a little further out to the sides and aft and a
little lower overhead; the faces and the rule that names them are unchanged. `tools/ships/console_model.py --check`
fails if the model or the ellipsoid changes so that a vertex is no longer held with 1 m to spare. The mockups that copy
the numbers (`consoles.html`'s `SH`, `exterior.html`) are updated with it.

## 4. Pi 5 budget

496 triangles against the loft's ~100 quads, as UI polygons. The camera widget memoises its projection by camera
angle, so it projects once per turn of the view; the shield view projects each frame it is shown: about 500 small
polygons, ~1,500 UI vertices, in the console's one UI batch. No draw call of its own.

## 5. Checks

- `python3 tools/ships/console_model.py --check`.
- `sc-client`: the file parses, every index is in range, and the model's box is the props manifest's
  `bounds_m` within 2 cm.
- Console parity with the mockup's states regenerated.
- Captures of the camera widget and the shield view, before and after.
