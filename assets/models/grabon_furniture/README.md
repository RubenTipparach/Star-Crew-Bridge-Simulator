# Grabon furniture

Four faction-specific props use plum armor, bronze structure, padded seats
and amber console graphics. Geometry is modeled with the shared hard-surface
CSG kit; Cycles bakes the surface finish into each prop's atlas.

| Prop | Triangles | Form |
| --- | ---: | --- |
| Tactical console | 100 | Tapered armored lectern, recessed sloping display and hand grips |
| Command chair | 200 | Faceted high back, splayed base and heavy padded arms |
| Double berth | 300 | Two full-length sleeping alcoves in an armored frame |
| Mess table | 60 | Clipped slab on two splayed trestles |

The preview uses these same models in cutaways, walking views and rooms visible
through hull windows. Medical equipment, engineering machinery and fighters
remain shared functional assets.

```powershell
blender -b --factory-startup --python-use-system-env --python-exit-code 1 -P tools/blender/build_grabon_furniture.py
blender -b --factory-startup --python-use-system-env --python-exit-code 1 -P tools/blender/build_grabon_furniture.py -- --check
```

The build checks topology, triangle budgets, exports, atlas hashes and emission.
`--check` rebuilds in memory and compares the delivered assets. An optional
`--blend build/grabon-detail/furniture.blend` saves the assembly for inspection.
