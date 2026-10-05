# Tasks: reference ship SCS Tern

## 1. The plan

- [x] 1.1 Measure the layout: volumes, floor areas, hull margins (`tools/layout_check.py`), shared walls, routes and walk times, articulation points (scratch measurement; method in design sections 6 and 7).
- [x] 1.2 `tools/deck_plans.py` and `docs/design/maps/tern-deck-{A,B,C}.svg`; rendered and checked by eye.
- [x] 1.3 The design document: per-deck compartment tables, reasoning, single points of failure, the schema field by field.
- [x] 1.4 `docs/mockups/deck-plan.html` with its shots in `docs/screenshots/mockups/`.
- [ ] 1.5 Owner review of T1-T3 with the mockup shots named in the open questions.

## 2. Patches (coordinator)

- [ ] 2.1 Apply T1 (galleries to z = 0) and T3 (the bridge scuttle), re-expressed for schema v2 in design section 9, to `data/ships/tern/layout.json` in one commit (T2 was applied 2026-10-04); re-run `layout_check.py`, `deck_plans.py`, `inline.py` and the shots; update the totals quoted by `vision.md` and the system changes (9,889.6 m^3, 41 portals).
- [ ] 2.2 Add `size_m` to the systems schema (T4), with values from each system's change.

## 3. Checker (documentation tooling, coordinator)

- [ ] 3.1 Teach `layout_check.py` walkable levels inside multi-level compartments and the `stair` fixture; fail when a station, door sill or system cannot be walked to (the "Every station and level is walkable" requirement).
- [ ] 3.2 Check system boxes (T4) inside their compartment and clear of door openings.
- [ ] 3.3 Check door frames fit their walls (design section 8, item 3).

## 5. Rooms follow the hull (owner, 2026-10-05)

- [x] 5.1 Schema `starcrew.ship-layout/2`: brushes (convex prisms) instead of boxes, portal normals instead of axes, `hull.clearance_m`, each compartment's `finish`; `tools/layout_check.py` rewritten for it (convexity, winding, overlaps by separating axes, walls matched by normal, clearance segment by segment, finishes against `detailing.json` and `materials.json`).
- [x] 5.2 The Tern redrawn to follow the hull (design section 1a); same ids, graph, portals, systems and craft; seats and lockers moved where their walls moved.
- [x] 5.3 `tools/deck_plans.py` and `tools/walk_times.py` ported (shared geometry imported from `layout_check.py`; `walk_times.py` gained a route check that every leg stays in the air and crosses walls only at portals); maps regenerated.
- [x] 5.4 T1 and T3 re-expressed for v2 and checked on a scratch copy (9,889.6 m^3, 41 portals, ok).
- [ ] 5.5 Owner review of the new plan (survey T6) with the before and after shots.

## 4. Keep it current

- [x] 4.1 Regenerate the route tables when `crew-on-deck` fixes walking, running and ladder speeds (T5). Done 2026-10-04: design section 6 from `tools/walk_times.py`, and the `SPEED` constants of `docs/mockups/deck-plan.html`'s route tool.
- [ ] 4.2 When `sc-core`'s layout module lands (`deck-pipeline` task 1.3), move these requirements' checks there with tests that read as sentences, and retire or wrap the Python checker.
