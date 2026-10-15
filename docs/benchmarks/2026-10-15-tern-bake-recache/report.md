# Tern bake, 2026-10-15

Written by `tools/mockups/bake_ship.mjs` (openspec/changes/light-baking, design section 15): the deck plan baking every compartment with
`lightbake.js`, alone, doors closed, the three lighting states from one set of rays, at `data/lighting/bake.json`'s final settings,
the shell split to 1 m cells. Claude Code cloud container, headless Chromium with SwiftShader, one browser thread; not a Pi 5 and not the engine baker. **Times say nothing about a Pi 5 or the engine baker.**

Totals: 38 compartments, 2,693 m^2 of floor, 394 lamps (136 on the emergency bus), 259 cove strips;
175,088 triangles in the rooms, 73,118 of them added by the split; 197,482 vertices baked; 93,943,075 rays;
408 s of baking (421.7 s from opening the page).

| Compartment | Floor m^2 | Lamps (emergency) | Strips | Triangles / ceiling | Split added | Vertices | Cache points | Rays | Time s (scene, cache, vertices) | Digest |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Bridge | 118.8 | 26 (6) | 67 | 11,755 / 30,000 | 2,793 | 14,907 | 52,844 | 4,634,065 | 23.3 (0.4, 13.3, 9.5) | `df4c15a0` |
| Captain's ready room | 67.1 | 8 (3) | 11 | 4,139 / 8,000 | 1,427 | 4,941 | 18,933 | 1,881,972 | 7.6 (0.2, 5.4, 2.0) | `370712ff` |
| Computer core | 37.2 | 5 (2) | 5 | 2,902 / 8,000 | 1,010 | 3,402 | 14,880 | 1,380,801 | 6.2 (0.2, 4.7, 1.3) | `71fa4232` |
| Command passage | 40.0 | 8 (3) | 4 | 2,750 / 8,000 | 1,750 | 2,447 | 14,436 | 1,685,225 | 7.5 (0.2, 6.0, 1.3) | `47334e9f` |
| Dorsal turret access | 15.3 | 2 (1) | 8 | 1,010 / 8,000 | 442 | 1,198 | 3,722 | 371,117 | 1.4 (0.0, 1.1, 0.3) | `231e2059` |
| Aft passage | 45.0 | 9 (3) | 4 | 3,279 / 8,000 | 2,451 | 2,707 | 17,651 | 1,699,983 | 6.7 (0.1, 5.3, 1.2) | `ed94f442` |
| Torpedo room | 118.2 | 15 (5) | 6 | 4,498 / 8,000 | 2,750 | 4,235 | 23,030 | 2,773,243 | 10.9 (0.1, 8.6, 2.3) | `e3e818db` |
| Medbay | 68.9 | 7 (3) | 5 | 2,872 / 8,000 | 1,637 | 2,852 | 14,144 | 1,427,075 | 5.1 (0.2, 4.0, 0.9) | `1b622312` |
| Damage control | 66.8 | 11 (4) | 6 | 3,452 / 8,000 | 1,485 | 3,811 | 17,053 | 1,884,761 | 7.6 (0.1, 5.9, 1.5) | `829dca59` |
| Crew quarters | 91.3 | 13 (5) | 5 | 4,848 / 8,000 | 2,072 | 5,209 | 28,490 | 2,996,984 | 12.9 (0.2, 10.6, 2.1) | `3d4a6797` |
| Mess | 89.5 | 12 (4) | 6 | 3,808 / 8,000 | 2,100 | 3,683 | 19,807 | 2,408,616 | 10.0 (0.1, 8.3, 1.6) | `197a0942` |
| Port turret access | 77.7 | 8 (3) | 5 | 5,820 / 8,000 | 1,835 | 6,953 | 31,096 | 2,812,736 | 11.3 (0.1, 8.6, 2.5) | `8e5eebb7` |
| Starboard turret access | 77.7 | 8 (3) | 5 | 5,821 / 8,000 | 1,836 | 6,954 | 31,083 | 2,822,467 | 11.3 (0.2, 8.6, 2.5) | `adb31ddb` |
| Main corridor | 65.0 | 12 (4) | 4 | 4,257 / 8,000 | 2,743 | 3,945 | 26,942 | 2,651,264 | 10.7 (0.2, 8.6, 2.0) | `9d2dc261` |
| Hangar | 350.8 | 48 (17) | 8 | 13,653 / 8,000 **over** | 7,527 | 13,703 | 78,823 | 10,246,196 | 45.1 (0.6, 37.7, 6.7) | `a6514200` |
| Port launch bay | 63.4 | 6 (2) | 6 | 3,328 / 8,000 | 1,636 | 3,638 | 19,108 | 2,046,874 | 7.7 (0.1, 6.3, 1.4) | `92188941` |
| Starboard launch bay | 63.4 | 6 (2) | 6 | 3,328 / 8,000 | 1,636 | 3,638 | 19,459 | 2,101,249 | 8.7 (0.1, 7.1, 1.4) | `448c9a3b` |
| Engineering | 253.8 | 49 (15) | 8 | 37,731 / 30,000 **over** | 9,861 | 49,260 | 202,347 | 20,524,578 | 110.3 (1.5, 84.8, 23.9) | `03a5886e` |
| Drive section | 102.4 | 12 (4) | 6 | 5,523 / 8,000 | 2,211 | 6,295 | 27,594 | 2,741,464 | 10.8 (0.1, 8.4, 2.3) | `6eefa2f3` |
| Lower corridor | 50.0 | 9 (3) | 4 | 3,572 / 8,000 | 2,314 | 3,295 | 19,836 | 2,101,252 | 8.0 (0.1, 6.6, 1.4) | `6c56ff83` |
| Magazine | 167.0 | 21 (7) | 6 | 5,333 / 8,000 | 3,427 | 4,751 | 29,165 | 3,791,130 | 14.7 (0.2, 12.6, 1.9) | `3f488f8a` |
| Life support | 129.4 | 19 (7) | 7 | 5,430 / 8,000 | 2,637 | 5,640 | 27,331 | 3,469,034 | 13.6 (0.1, 11.2, 2.3) | `994c45e0` |
| Cargo and stores | 115.1 | 13 (5) | 10 | 4,430 / 8,000 | 2,615 | 4,109 | 21,902 | 2,600,456 | 9.1 (0.1, 7.8, 1.3) | `39ab407c` |
| Airlock | 8.8 | 2 (1) | 4 | 614 / 8,000 | 372 | 584 | 2,010 | 199,222 | 0.7 (0.0, 0.5, 0.1) | `eea53d25` |
| Shield generator | 62.3 | 9 (3) | 4 | 2,738 / 8,000 | 1,506 | 2,797 | 12,647 | 1,449,291 | 4.5 (0.1, 3.5, 0.9) | `32c57efc` |
| Forward switchboard | 62.3 | 9 (3) | 4 | 3,176 / 8,000 | 1,578 | 3,281 | 15,412 | 1,557,412 | 5.7 (0.1, 4.1, 1.5) | `39c9b2b5` |
| Dorsal turret pod | 5.2 | 2 (1) | 0 | 824 / 8,000 | 188 | 1,095 | 2,484 | 92,833 | 0.4 (0.0, 0.3, 0.2) | `5d833f31` |
| Ventral turret pod | 5.2 | 2 (1) | 0 | 1,092 / 8,000 | 290 | 1,461 | 3,675 | 154,736 | 0.7 (0.0, 0.4, 0.3) | `3ac428b5` |
| Port turret pod | 5.2 | 2 (1) | 0 | 826 / 8,000 | 206 | 1,079 | 2,336 | 86,555 | 0.3 (0.0, 0.2, 0.1) | `e212becb` |
| Starboard turret pod | 5.2 | 2 (1) | 0 | 842 / 8,000 | 222 | 1,087 | 2,344 | 86,870 | 0.4 (0.0, 0.2, 0.2) | `f75ade1b` |
| Briefing room | 63.2 | 8 (3) | 12 | 5,807 / 8,000 | 1,511 | 7,386 | 25,647 | 2,071,963 | 9.1 (0.1, 5.9, 3.1) | `2a4622a4` |
| Captain's quarters | 37.2 | 6 (2) | 5 | 2,458 / 8,000 | 1,020 | 2,766 | 11,734 | 1,227,884 | 4.2 (0.1, 3.2, 0.9) | `712f34c0` |
| Head | 45.6 | 5 (2) | 6 | 2,829 / 8,000 | 1,374 | 2,942 | 15,265 | 1,474,306 | 5.3 (0.1, 4.2, 1.0) | `79381d90` |
| Bridge locker | 45.6 | 5 (2) | 6 | 2,675 / 8,000 | 1,362 | 2,723 | 12,646 | 1,254,829 | 4.3 (0.1, 3.3, 0.9) | `86c6e857` |
| Port stair tower | 6.8 | 3 (1) | 4 | 1,358 / 8,000 | 578 | 1,570 | 6,429 | 649,374 | 2.4 (0.0, 2.0, 0.3) | `678e4ff7` |
| Starboard stair tower | 6.8 | 3 (1) | 4 | 1,358 / 8,000 | 578 | 1,570 | 6,429 | 649,150 | 2.5 (0.0, 2.2, 0.3) | `186eb0b1` |
| Lift | 3.9 | 3 (1) | 4 | 884 / 8,000 | 534 | 776 | 4,041 | 320,485 | 1.1 (0.0, 0.9, 0.2) | `063d9f41` |
| Armory | 56.1 | 6 (2) | 4 | 4,068 / 8,000 | 1,604 | 4,792 | 20,503 | 1,615,623 | 6.4 (0.1, 4.6, 1.7) | `2dbe105b` |

A room's triangles are the deck plan's whole room (shell, detail, props, the split), against its ceiling in `engine-stack`'s table
(engineering's from `engineering-fitout`). The split stands in for the engine's adaptive subdivision (design section 3), which spends fewer.
