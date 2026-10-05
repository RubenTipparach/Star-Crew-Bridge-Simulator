# Surface materials

## Purpose

What a surface material is, how a Material Maker graph becomes a Raspberry Pi 5 texture layer,
the texel density and sampling rules, and the one texture array a deck draws from.

## ADDED Requirements

### Requirement: One source for materials
Every surface material SHALL be listed once, in `data/materials/materials.json` (schema
`starcrew.materials/1`), with its stable layer index, role, source Material Maker graph and its
provenance, tile size, span and post-process knobs. The post-process SHALL refuse the file,
naming the field, when a key is unknown, a knob is out of range, a span is not a whole multiple
or fraction of the tile, or the layer indices are not 0 to n-1.

#### Scenario: A misspelt knob
- **WHEN** a material's `ramp_mix` is written `rampmix`
- **THEN** the post-process stops naming `materials.<name>` and the unknown key, and writes nothing

### Requirement: A material is a graph
How a material looks SHALL be defined by a Material Maker graph (`tools/materials/ptex/`), and
its layer SHALL be produced from that graph's render by `tools/materials/postprocess.py`; no
other code SHALL generate a material's pixels.

#### Scenario: Rebuilding the layers
- **WHEN** `tools/materials/export_materials.sh` runs (with Material Maker, or `--from-fps` with Undercity's exports)
- **THEN** it rebuilds every layer in `assets/textures/` and the contact sheet, and prints a digest that is the same on a second run

### Requirement: Every layer is the same size and tiles
Every layer SHALL be 128 x 128 px RGBA8 (sRGB colour, alpha the emission mask), and SHALL tile
seamlessly: a graph smaller than the span is repeated a whole number of times inside the layer.

#### Scenario: The hazard layer
- **WHEN** the hazard material (1 m stripes) is built for a 2 m span
- **THEN** its layer holds the graph 2 x 2 and its wrap-around seam ratio is under 2

### Requirement: Texel density is fixed per space
A surface's texture coordinate SHALL be its world position projected on its own plane (x and z
for a floor or ceiling; its horizontal tangent and y otherwise) divided by its material's span:
2.0 m inside the ship (64 px per metre), 4.0 m on the hull (32 px per metre).

#### Scenario: Two walls, one density
- **WHEN** a 1 m long bulkhead panel and a 1 m long trim are drawn in the same room
- **THEN** each spans 64 texels

### Requirement: Relief is baked into the colour
Because the Pi 5 draws no per-pixel lighting, a layer's colour SHALL carry its graph's normal
map and occlusion, baked once under the fixed key light of `materials.json` (`bake`), and the
run-time colour SHALL be the texel times the baked vertex light (plus the emissive term).

#### Scenario: A panel's bevel
- **WHEN** the bulkhead layer is built
- **THEN** texels on its panels' top bevels are lighter, and on their bottom bevels darker, than on the flat panel faces

### Requirement: One texture array, one draw
All layers SHALL be one texture array sampled nearest on magnification and with mipmaps on
minification, and a deck vertex SHALL carry its layer index, so a compartment's static geometry
draws in one call whatever its materials.

#### Scenario: The textured Tern in the mockups
- **WHEN** the deck plan mockup draws every compartment, textured
- **THEN** its meter shows one draw call per compartment for the rooms and 0.92 MB of texture memory

### Requirement: Texture memory is stated
The post-process SHALL print each layer's bytes with mipmaps and the total, and the total SHALL
stay inside the texture memory of the Pi 5 budget table (`engine-stack`).

#### Scenario: The first eleven layers
- **WHEN** the post-process runs on the 2026-10-05 materials
- **THEN** it prints 961,180 bytes in total, under 96 MB
