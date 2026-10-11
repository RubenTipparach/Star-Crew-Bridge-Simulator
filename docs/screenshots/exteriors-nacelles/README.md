# Nacelle direction and dedicated texture review

The Tern has broad forward collectors and tapered aft ends. Its level supports and
56 m nacelle bodies retain their placement. The shell, supports, rails and caps now
share a dedicated neutral 1024 x 2048 atlas, with livery color on separate accents.

- [Cyan close-up](ship-exteriors-livery-cobalt-0-nacelle-detail.png)
- [Complete warp assembly](ship-exteriors-livery-cobalt-0-pylons.png)
- [Cyan ship](ship-exteriors-livery-cobalt-0-hero.png)
- [Medium detail](ship-exteriors-livery-cobalt-1-pylons.png)
- [Distant detail](ship-exteriors-livery-cobalt-2-pylons.png)
- [Projected model before enhancement](projection.png)
- [Measured geometry and preview allocation](validation.json)

The 27 registered captures cover three liveries, three LODs and three views. Panel
wear, hatches and mechanical rails come from the [generated artwork](../../../tools/materials/sources/tern-nacelle-finish.png),
made with built-in imagegen using this [exact prompt](../../../tools/materials/sources/tern-nacelle-prompt.txt).
The actual model supplied the projected linework and receiving UV wireframe.

Delivered shell UVs allocate 35.11 longitudinal px/m; the original source painting
provides 30.41 px/m. The old fitting patch supplied about 3.8 px/m. Front width is
4.14 m and rear width 2.208 m, with fore/aft taper lengths of 3.36 / 11.76 m.
The checker rejects the previous general material, a reversed exported shell and
an exported UV regression reduced to 4.21 px/m. Dedicated atlas regions do not overlap.

The full/medium/distant Tern uses 4,440 / 1,290 / 496 triangles and 7 / 1 / 1 draws.
The complete furnished fleet uses 82,992 triangles, 51 draws and 88.83 MiB of decoded
textures with mipmaps. The 96 MiB scene ceiling is retained. These are asset counts,
not Pi frame-time measurements. All three liveries pass 460 real window/frame rays.
Exterior overlap is zero; the existing interior trim overlap remains 0.046 m2.

Reproduce the focused captures after rebuilding and inlining the models:

```powershell
$shots = foreach ($livery in @('copper','cobalt','rescue')) {
  foreach ($lod in @(0,1,2)) {
    foreach ($view in @('pylons','nacelle-detail','hero')) {
      'livery-' + $livery + '-' + $lod + '-' + $view
    }
  }
}
node tools/mockups/shoot.mjs docs/mockups/ship-exteriors.html --only ($shots -join ',') --out docs/screenshots/exteriors-nacelles
blender -b --factory-startup --python-use-system-env -P tools/blender/check_exterior_nacelles.py --
```
