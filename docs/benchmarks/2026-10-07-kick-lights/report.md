# Tern bake, 2026-10-07

Written by `tools/mockups/bake_ship.mjs` (openspec/changes/light-baking, design section 15): the deck plan baking every compartment with
`lightbake.js`, alone, doors closed, the three lighting states from one set of rays, at `data/lighting/bake.json`'s final settings,
the shell split to 1 m cells. Claude Code cloud container, headless Chromium with SwiftShader, one browser thread; not a Pi 5 and not the engine baker. **Times say nothing about a Pi 5 or the engine baker.**

Totals: 37 compartments, 2,573.1 m^2 of floor, 368 lamps (126 on the emergency bus), 257 cove strips;
160,437 triangles in the rooms, 70,851 of them added by the split; 177,728 vertices baked; 87,091,773 rays;
272 s of baking (277.6 s from opening the page).

| Compartment | Floor m^2 | Lamps (emergency) | Strips | Triangles / ceiling | Split added | Vertices | Cache points | Rays | Time s (scene, cache, vertices) | Digest |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Bridge | 118.8 | 26 (6) | 67 | 11,755 / 30,000 | 2,793 | 14,907 | 52,880 | 4,634,094 | 17.4 (0.2, 10.0, 7.1) | `4e7520f5` |
| Captain's ready room | 67.1 | 8 (3) | 11 | 4,139 / 8,000 | 1,427 | 4,941 | 18,933 | 1,882,145 | 5.3 (0.1, 3.8, 1.4) | `54fff7e0` |
| Computer core | 37.2 | 5 (2) | 5 | 2,902 / 8,000 | 1,010 | 3,402 | 14,880 | 1,380,801 | 4.1 (0.1, 3.2, 0.9) | `030e4e3b` |
| Command passage | 40.0 | 8 (3) | 4 | 2,750 / 8,000 | 1,750 | 2,447 | 14,436 | 1,685,225 | 4.7 (0.1, 3.7, 0.9) | `47334e9f` |
| Dorsal turret access | 15.3 | 2 (1) | 8 | 1,010 / 8,000 | 442 | 1,198 | 3,722 | 371,117 | 1.0 (0.0, 0.7, 0.2) | `231e2059` |
| Aft passage | 45.0 | 9 (3) | 4 | 3,527 / 8,000 | 2,745 | 2,803 | 17,888 | 1,543,261 | 4.6 (0.1, 3.4, 1.1) | `87f8bb7c` |
| Torpedo room | 118.2 | 15 (5) | 6 | 4,498 / 8,000 | 2,750 | 4,235 | 23,030 | 2,773,243 | 7.9 (0.1, 6.1, 1.7) | `7eb31c02` |
| Medbay | 62.4 | 7 (3) | 4 | 2,890 / 8,000 | 1,706 | 2,822 | 13,754 | 1,390,886 | 4.0 (0.1, 3.1, 0.8) | `7c2addcc` |
| Damage control | 60.4 | 8 (3) | 5 | 3,624 / 8,000 | 1,554 | 3,972 | 17,518 | 1,676,485 | 4.8 (0.1, 3.6, 1.1) | `ae51d3da` |
| Crew quarters | 83.7 | 12 (4) | 5 | 4,148 / 8,000 | 1,986 | 4,258 | 22,930 | 2,500,466 | 6.9 (0.1, 5.5, 1.3) | `c2e8c29a` |
| Mess | 81.9 | 11 (4) | 6 | 3,664 / 8,000 | 2,012 | 3,554 | 18,465 | 2,232,425 | 5.6 (0.1, 4.5, 1.0) | `f2dc430c` |
| Port turret access | 77.7 | 8 (3) | 5 | 2,339 / 8,000 | 1,711 | 1,920 | 12,103 | 1,653,578 | 3.7 (0.0, 3.2, 0.5) | `fdfc328e` |
| Starboard turret access | 77.7 | 8 (3) | 5 | 2,344 / 8,000 | 1,716 | 1,924 | 12,173 | 1,684,270 | 3.9 (0.0, 3.3, 0.6) | `3d46c216` |
| Main corridor | 65.0 | 12 (4) | 4 | 4,233 / 8,000 | 2,743 | 3,917 | 26,906 | 2,642,081 | 8.2 (0.1, 6.9, 1.2) | `3c05ced5` |
| Hangar | 350.8 | 44 (15) | 8 | 12,895 / 8,000 **over** | 7,707 | 12,429 | 72,596 | 9,536,723 | 26.8 (0.3, 22.6, 3.9) | `243ab297` |
| Port launch bay | 63.4 | 6 (2) | 6 | 3,328 / 8,000 | 1,636 | 3,638 | 19,108 | 2,046,874 | 5.4 (0.1, 4.3, 0.9) | `e04edfc1` |
| Starboard launch bay | 63.4 | 6 (2) | 6 | 3,328 / 8,000 | 1,636 | 3,638 | 19,459 | 2,101,249 | 5.8 (0.1, 4.8, 1.0) | `048cd129` |
| Engineering | 253.8 | 49 (15) | 8 | 37,207 / 30,000 **over** | 9,445 | 48,915 | 199,453 | 20,556,095 | 84.4 (0.8, 65.9, 17.7) | `105d4ff7` |
| Drive section | 102.4 | 12 (4) | 6 | 4,341 / 8,000 | 2,211 | 4,577 | 21,835 | 2,429,453 | 6.6 (0.1, 5.0, 1.5) | `149adc23` |
| Lower corridor | 50.0 | 9 (3) | 4 | 3,590 / 8,000 | 2,334 | 3,316 | 19,703 | 2,040,486 | 6.0 (0.1, 4.8, 1.1) | `c4c1b226` |
| Magazine | 167.0 | 21 (7) | 6 | 5,309 / 8,000 | 3,427 | 4,723 | 29,129 | 3,784,338 | 10.6 (0.1, 9.0, 1.5) | `95beb038` |
| Life support | 115.8 | 16 (6) | 7 | 5,213 / 8,000 | 2,512 | 5,436 | 25,788 | 3,092,387 | 7.9 (0.1, 6.3, 1.6) | `87732cdb` |
| Cargo and stores | 105.7 | 11 (4) | 12 | 4,168 / 8,000 | 2,467 | 3,815 | 19,952 | 2,381,856 | 5.8 (0.1, 4.8, 0.9) | `96b510a4` |
| Airlock | 8.8 | 2 (1) | 4 | 614 / 8,000 | 372 | 584 | 2,010 | 199,222 | 0.5 (0.0, 0.4, 0.1) | `eea53d25` |
| Shield generator | 55.9 | 6 (2) | 5 | 2,582 / 8,000 | 1,410 | 2,659 | 11,885 | 1,352,846 | 2.9 (0.0, 2.2, 0.6) | `bf9f7f35` |
| Forward switchboard | 55.9 | 6 (2) | 5 | 3,018 / 8,000 | 1,456 | 3,165 | 14,397 | 1,385,089 | 3.5 (0.0, 2.6, 0.8) | `e330ee93` |
| Dorsal turret pod | 5.2 | 2 (1) | 0 | 824 / 8,000 | 188 | 1,095 | 2,484 | 92,833 | 0.3 (0.0, 0.2, 0.1) | `a6b3ff28` |
| Ventral turret pod | 5.2 | 2 (1) | 0 | 1,092 / 8,000 | 290 | 1,461 | 3,675 | 154,736 | 0.5 (0.0, 0.3, 0.2) | `768690fe` |
| Port turret pod | 5.2 | 2 (1) | 0 | 826 / 8,000 | 206 | 1,079 | 2,336 | 86,555 | 0.3 (0.0, 0.1, 0.1) | `3f046bc9` |
| Starboard turret pod | 5.2 | 2 (1) | 0 | 842 / 8,000 | 222 | 1,087 | 2,344 | 86,870 | 0.3 (0.0, 0.2, 0.1) | `721f39ac` |
| Briefing room | 63.2 | 8 (3) | 12 | 5,807 / 8,000 | 1,511 | 7,386 | 25,647 | 2,071,963 | 6.3 (0.1, 3.9, 2.3) | `efdbd25f` |
| Captain's quarters | 37.2 | 6 (2) | 5 | 2,458 / 8,000 | 1,020 | 2,766 | 11,734 | 1,228,034 | 3.2 (0.0, 2.5, 0.7) | `1abd536a` |
| Head | 45.6 | 5 (2) | 6 | 2,829 / 8,000 | 1,374 | 2,942 | 15,265 | 1,474,306 | 4.1 (0.0, 3.2, 0.9) | `0829a241` |
| Bridge locker | 45.6 | 5 (2) | 6 | 2,675 / 8,000 | 1,362 | 2,723 | 12,646 | 1,254,806 | 3.4 (0.0, 2.7, 0.7) | `32dad6af` |
| Port stair tower | 6.8 | 3 (1) | 4 | 1,358 / 8,000 | 578 | 1,570 | 6,429 | 649,374 | 1.9 (0.0, 1.7, 0.2) | `c2ad8859` |
| Starboard stair tower | 6.8 | 3 (1) | 4 | 1,358 / 8,000 | 578 | 1,570 | 6,429 | 649,150 | 1.9 (0.0, 1.7, 0.2) | `5ad58309` |
| Lift | 3.9 | 3 (1) | 4 | 952 / 8,000 | 564 | 854 | 4,496 | 386,451 | 1.0 (0.0, 0.8, 0.1) | `b169a7c2` |

A room's triangles are the deck plan's whole room (shell, detail, props, the split), against its ceiling in `engine-stack`'s table
(engineering's from `engineering-fitout`). The split stands in for the engine's adaptive subdivision (design section 3), which spends fewer.
