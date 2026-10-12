# Level middle-deck windows

Deck B has ten 1.70 x 0.80 m openings with centers 1.50 m above its floor, sills at
1.10 m and heads at 1.90 m. The exterior and interior use one shared deck row in
`data/ships/tern/exterior_windows_design.json`.

- `port.png` and `starboard.png`: both exterior rows in the delivered Cyan GLB.
- `inside-*.png`: centered windows in quarters, mess, medbay and damage control.
- `projection.png`: actual hull projection inspected before the refreshed texture finish.
- `validation.json`: delivered portal heights, 460 opening/frame rays per livery, and
  rejection of the former mixed-height row by the preview's window validator.

The height correction also updates the UV guide, material masks and all custom LOD bakes.
Screenshots use the normal palette compression. These are desktop art checks, not Pi timings.
