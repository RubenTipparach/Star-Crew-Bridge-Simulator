# Grabon texture enhancement

Generated with the built-in image generation tool from the matching Blender
projection bake. Each ship has its own UV atlas. The projection was inspected on
the 3D hull before enhancement. Editable sources are assets/models/grabon.

`*-projection.png` is the 2048 px albedo guide baked from hull-space linework.
`*-uv.svg` and `*-uv.json` contain the actual mesh UVs. `*-regions.png` contains
the material regions, with linear color keys in `regions.json`.
`*-enhanced.png` is the delivered image-generation result at 1254 px. The
generator requested 2048 px, but the tool returned 1254 px; these files retain
their native resolution. The enhanced color remains separate from the fixed
sun lighting stored in the exported mesh's COLOR_0 attribute.

## Prompt used for each ship

Use case: precise-object-edit. Asset: production spaceship UV albedo atlas, square 2048 by 2048. Edit the supplied UV atlas in place. Absolutely preserve every existing island silhouette, its exact position and size, all black gutters, the existing sparse panel joint lines and purple/orange region boundaries. This atlas is already mapped to a 3D mesh. Add convincing restrained painted metal surface detail INSIDE the islands: rich muted purple ceramic armor, burnt-orange painted identification areas, dark plum structural steel, graphite machinery. Large purple areas should remain broad quiet armor panels, with fine edge wear, occasional small flush service covers, tiny fasteners and subtle uneven paint. Orange paint stays orange with slight scuffs. Dark gray islands get machinery grooves, vent blades, cables and service detail. Small bright orange rectangles are engine exhaust apertures with warm orange emitter cells. No generic checker or square grid, no invented large seams, no new orange bands, no white hull, no objects outside islands, no text or labels, no perspective, no shading gradient across the sheet, no rearrangement, no artistic presentation. This is a functional technical texture image, not an illustration of a spaceship. Retain original purple color family and exact line layout. Refine material finish without moving any boundaries.
