# Proposal: crew characters, the owner's requirements

## Why

The owner asked for better crew bodies on 2026-10-10 and shaped them over one round of renders, then had
them deleted and asked for the requirements to be written down: "type out some requirements that I just
gave you, for now use blocky people". This change holds those requirements so the next attempt starts
from them. It builds nothing: the crew stay the blocky figures of `crew-npcs` 7, lit by light probes
(`light-baking` section 16).

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

## What Changes

- **For now, blocky people.** The crew aboard are the `crew-npcs` 7 figures (boxes, about 100 triangles,
  the tunic in the department's colour), lit by probes. No character model replaces them until a new
  attempt is approved on screenshots.
- **The requirements for that attempt** (design sections 1-5, spec delta): a stylised cartoon look after
  the references, exaggerated proportions, big eyes with the references' iris and pupil ratios, clothes
  that are garments rather than paint, the Pi 5 avatar budget, and probe lighting.
- **What was tried and rejected** (design section 6): MakeHuman through MPFB with pose-scaled cartoon
  proportions, fitted CC0 clothes and generated eyes (commit `3419c0b`, PR #6, closed). Recorded so it is
  not tried again unchanged.

## Impact

- `crew-npcs` 7: the figures stay; their design notes the decision.
- `crew-on-deck` 17 (the avatar budget): unchanged; these requirements live inside it.
- `light-baking` 16: the probes that light whatever body the crew have.
