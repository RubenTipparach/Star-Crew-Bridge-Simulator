# Change: cartoon crew characters for screenshot review

## Why

The owner supplied the character requirements and requested headless Blender on
2026-10-10. The crew need big heads, hands, feet and eyes, with proper uniforms,
while the existing blocky figures stay aboard until screenshots are approved.

## What Changes

- Build six original department characters from one tunable data table.
- Give eyes geometric iris and pupil boundaries and report their measured ratios.
- Build tunics, trousers and boots over a body, with visible garment edges.
- Read triangle, bone, material and primitive counts back from each GLB.
- Render the exported assets for review. Adoption and live probe sampling remain
  a later engine step after explicit screenshot approval.
- Provide a local HTML 3D viewer of the built files, with character selection,
  orbit controls, camera presets, wireframe and illustrative probe states.

## Impact

Adds review assets, a deterministic Blender generator, offline captures and an
interactive review page. Extends the shared mockup inliner for character sets.
Changes no game crew, simulation, mockup occupants or deployment. Uses the avatar
ceiling in `engine-stack` section 5, tightened to 30 bones by the owner's brief.
