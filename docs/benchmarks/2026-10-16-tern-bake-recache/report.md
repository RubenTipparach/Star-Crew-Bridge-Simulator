# Tern bake, 2026-10-16

Written by `tools/mockups/bake_ship.mjs` (openspec/changes/light-baking, design section 15): the deck plan baking every compartment with
`lightbake.js`, alone, doors closed, the three lighting states from one set of rays, at `data/lighting/bake.json`'s final settings,
the shell split to 1 m cells. Claude Code cloud container, headless Chromium with SwiftShader, one browser thread; not a Pi 5 and not the engine baker. **Times say nothing about a Pi 5 or the engine baker.**

Totals: 38 compartments, 2,693 m^2 of floor, 394 lamps (136 on the emergency bus), 259 cove strips;
175,088 triangles in the rooms, 73,118 of them added by the split; 197,482 vertices baked; 93,943,075 rays;
240 s of baking (247.1 s from opening the page).

| Compartment | Floor m^2 | Lamps (emergency) | Strips | Triangles / ceiling | Split added | Vertices | Cache points | Rays | Time s (scene, cache, vertices) | Digest |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Bridge | 118.8 | 26 (6) | 67 | 11,755 / 30,000 | 2,793 | 14,907 | 52,844 | 4,634,065 | 13.2 (0.3, 7.7, 5.2) | `df4c15a0` |
| Captain's ready room | 67.1 | 8 (3) | 11 | 4,139 / 8,000 | 1,427 | 4,941 | 18,933 | 1,881,972 | 4.3 (0.1, 3.1, 1.1) | `370712ff` |
| Computer core | 37.2 | 5 (2) | 5 | 2,902 / 8,000 | 1,010 | 3,402 | 14,880 | 1,380,801 | 3.3 (0.1, 2.4, 0.7) | `71fa4232` |
| Command passage | 40.0 | 8 (3) | 4 | 2,750 / 8,000 | 1,750 | 2,447 | 14,436 | 1,685,225 | 4.0 (0.1, 3.2, 0.8) | `47334e9f` |
| Dorsal turret access | 15.3 | 2 (1) | 8 | 1,010 / 8,000 | 442 | 1,198 | 3,722 | 371,117 | 0.8 (0.0, 0.6, 0.2) | `231e2059` |
| Aft passage | 45.0 | 9 (3) | 4 | 3,279 / 8,000 | 2,451 | 2,707 | 17,651 | 1,699,983 | 4.0 (0.1, 3.2, 0.8) | `ed94f442` |
| Torpedo room | 118.2 | 15 (5) | 6 | 4,498 / 8,000 | 2,750 | 4,235 | 23,030 | 2,773,243 | 6.4 (0.1, 5.0, 1.3) | `e3e818db` |
| Medbay | 68.9 | 7 (3) | 5 | 2,872 / 8,000 | 1,637 | 2,852 | 14,144 | 1,427,075 | 3.0 (0.0, 2.3, 0.6) | `1b622312` |
| Damage control | 66.8 | 11 (4) | 6 | 3,452 / 8,000 | 1,485 | 3,811 | 17,053 | 1,884,761 | 4.1 (0.0, 3.1, 0.9) | `829dca59` |
| Crew quarters | 91.3 | 13 (5) | 5 | 4,848 / 8,000 | 2,072 | 5,209 | 28,490 | 2,996,984 | 7.6 (0.1, 6.1, 1.4) | `3d4a6797` |
| Mess | 89.5 | 12 (4) | 6 | 3,808 / 8,000 | 2,100 | 3,683 | 19,807 | 2,408,616 | 5.3 (0.1, 4.3, 0.9) | `197a0942` |
| Port turret access | 77.7 | 8 (3) | 5 | 5,820 / 8,000 | 1,835 | 6,953 | 31,096 | 2,812,736 | 6.9 (0.1, 5.1, 1.7) | `8e5eebb7` |
| Starboard turret access | 77.7 | 8 (3) | 5 | 5,821 / 8,000 | 1,836 | 6,954 | 31,083 | 2,822,467 | 7.1 (0.1, 5.3, 1.7) | `adb31ddb` |
| Main corridor | 65.0 | 12 (4) | 4 | 4,257 / 8,000 | 2,743 | 3,945 | 26,942 | 2,651,264 | 6.9 (0.1, 5.6, 1.2) | `9d2dc261` |
| Hangar | 350.8 | 48 (17) | 8 | 13,653 / 8,000 **over** | 7,527 | 13,703 | 78,823 | 10,246,196 | 24.6 (0.3, 20.2, 4.1) | `a6514200` |
| Port launch bay | 63.4 | 6 (2) | 6 | 3,328 / 8,000 | 1,636 | 3,638 | 19,108 | 2,046,874 | 4.7 (0.1, 3.7, 0.9) | `92188941` |
| Starboard launch bay | 63.4 | 6 (2) | 6 | 3,328 / 8,000 | 1,636 | 3,638 | 19,459 | 2,101,249 | 4.6 (0.0, 3.8, 0.8) | `448c9a3b` |
| Engineering | 253.8 | 49 (15) | 8 | 37,731 / 30,000 **over** | 9,861 | 49,260 | 202,347 | 20,524,578 | 65.9 (0.8, 50.8, 14.4) | `03a5886e` |
| Drive section | 102.4 | 12 (4) | 6 | 5,523 / 8,000 | 2,211 | 6,295 | 27,594 | 2,741,464 | 6.2 (0.1, 4.6, 1.5) | `6eefa2f3` |
| Lower corridor | 50.0 | 9 (3) | 4 | 3,572 / 8,000 | 2,314 | 3,295 | 19,836 | 2,101,252 | 5.1 (0.1, 4.1, 0.9) | `6c56ff83` |
| Magazine | 167.0 | 21 (7) | 6 | 5,333 / 8,000 | 3,427 | 4,751 | 29,165 | 3,791,130 | 8.6 (0.1, 7.2, 1.4) | `3f488f8a` |
| Life support | 129.4 | 19 (7) | 7 | 5,430 / 8,000 | 2,637 | 5,640 | 27,331 | 3,469,034 | 8.0 (0.1, 6.4, 1.5) | `994c45e0` |
| Cargo and stores | 115.1 | 13 (5) | 10 | 4,430 / 8,000 | 2,615 | 4,109 | 21,902 | 2,600,456 | 4.9 (0.0, 4.1, 0.8) | `39ab407c` |
| Airlock | 8.8 | 2 (1) | 4 | 614 / 8,000 | 372 | 584 | 2,010 | 199,222 | 0.3 (0.0, 0.2, 0.1) | `eea53d25` |
| Shield generator | 62.3 | 9 (3) | 4 | 2,738 / 8,000 | 1,506 | 2,797 | 12,647 | 1,449,291 | 2.9 (0.0, 2.2, 0.7) | `32c57efc` |
| Forward switchboard | 62.3 | 9 (3) | 4 | 3,176 / 8,000 | 1,578 | 3,281 | 15,412 | 1,557,412 | 3.5 (0.0, 2.6, 0.8) | `39c9b2b5` |
| Dorsal turret pod | 5.2 | 2 (1) | 0 | 824 / 8,000 | 188 | 1,095 | 2,484 | 92,833 | 0.2 (0.0, 0.1, 0.1) | `5d833f31` |
| Ventral turret pod | 5.2 | 2 (1) | 0 | 1,092 / 8,000 | 290 | 1,461 | 3,675 | 154,736 | 0.4 (0.0, 0.2, 0.2) | `3ac428b5` |
| Port turret pod | 5.2 | 2 (1) | 0 | 826 / 8,000 | 206 | 1,079 | 2,336 | 86,555 | 0.2 (0.0, 0.1, 0.1) | `e212becb` |
| Starboard turret pod | 5.2 | 2 (1) | 0 | 842 / 8,000 | 222 | 1,087 | 2,344 | 86,870 | 0.3 (0.0, 0.2, 0.1) | `f75ade1b` |
| Briefing room | 63.2 | 8 (3) | 12 | 5,807 / 8,000 | 1,511 | 7,386 | 25,647 | 2,071,963 | 4.9 (0.1, 3.2, 1.7) | `2a4622a4` |
| Captain's quarters | 37.2 | 6 (2) | 5 | 2,458 / 8,000 | 1,020 | 2,766 | 11,734 | 1,227,884 | 2.7 (0.0, 2.1, 0.7) | `712f34c0` |
| Head | 45.6 | 5 (2) | 6 | 2,829 / 8,000 | 1,374 | 2,942 | 15,265 | 1,474,306 | 3.7 (0.0, 2.9, 0.7) | `79381d90` |
| Bridge locker | 45.6 | 5 (2) | 6 | 2,675 / 8,000 | 1,362 | 2,723 | 12,646 | 1,254,829 | 2.9 (0.0, 2.2, 0.7) | `86c6e857` |
| Port stair tower | 6.8 | 3 (1) | 4 | 1,358 / 8,000 | 578 | 1,570 | 6,429 | 649,374 | 1.8 (0.0, 1.6, 0.2) | `678e4ff7` |
| Starboard stair tower | 6.8 | 3 (1) | 4 | 1,358 / 8,000 | 578 | 1,570 | 6,429 | 649,150 | 1.7 (0.0, 1.5, 0.2) | `186eb0b1` |
| Lift | 3.9 | 3 (1) | 4 | 884 / 8,000 | 534 | 776 | 4,041 | 320,485 | 0.6 (0.0, 0.5, 0.1) | `063d9f41` |
| Armory | 56.1 | 6 (2) | 4 | 4,068 / 8,000 | 1,604 | 4,792 | 20,503 | 1,615,623 | 3.9 (0.0, 2.7, 1.2) | `2dbe105b` |

A room's triangles are the deck plan's whole room (shell, detail, props, the split), against its ceiling in `engine-stack`'s table
(engineering's from `engineering-fitout`). The split stands in for the engine's adaptive subdivision (design section 3), which spends fewer.
