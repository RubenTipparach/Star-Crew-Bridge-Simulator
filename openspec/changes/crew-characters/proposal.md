# Change: cartoon crew characters for screenshot review

## Why

The owner supplied the character requirements and requested headless Blender on
2026-10-10. The crew need big heads, hands, feet and eyes, with proper uniforms,
while the existing blocky figures stay aboard until screenshots are approved.

## What Changes

- Build male and female variants for six departments from shared tunable data.
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

## Original brief and rejected prototype

The earlier MakeHuman attempt (`3419c0b`, closed PR #6) was rejected and deleted.
The original brief is retained below; the current original cartoon review assets
implement the requested next attempt. Screenshot approval is still required before
replacing blocky figures aboard. Design sections 1-6 retain the original requirements
and rejection history.

The owner's words, in order (2026-10-10):

1. "using make human in blender can you make better low poly characters? exagerate portions a bit to make
   them look more cartoony"
2. Two "Star Trek Pixar" fan cards, Spock and Captain Kirk: "like this"
3. "the face still sucks... need to make big eyes nose and mouth", then "just big eyes I think might make
   them fun"
4. "measure pupil ratio", with the Kirk card and a picture of Elsa (Frozen) as the references
5. "the shirt and pants dont look right, are you using the human maker clothing tool properly? is there
   any other tools for clothing?"
6. On the result: "delete these guys they look stupid lol"
7. "type out some requirements that I just gave you, for now use blocky people. implement light probes so
   the peiple get lit properly"
