# Fire cases with the cell model

`damage-control` design 3's cases, rerun through the mockups' simulation with `fire-spread`'s cell model (design 4 and
6a). The table between the markers is written by `tools/mockups/fire_cases.mjs` and rewritten on every run; the notes
here are not.

## What the run says

- **The calibration target is met.** The quarters, door shut, 50 kW on a bunk: 1 MW at 113 s (the table's 113), peak
  3.48 MW (3.71, -6%), peak air 295 C (307, -4%), out at 284 s (321, -12%). The room model alone reproduces the
  table within a second or two in every case but three, so the harness is the table's: engineering's two cases
  differ by 3-4 s and 0.06 MW, as the library did before this change (checked against the unchanged
  `shipsystems.js`), and the vented case moved for the reasons below.
- **Aim is worth everything.** The careless crew member (2 m short of the room's centre, aiming at it) never reaches
  the bunk fire, 4.7 m from the centre: every careless run is the unattended fire. The careful one (sweeping the
  burning cells nearest the door) puts out a fire caught at 30, 60 and 90 s with one extinguisher, as the table says.
- **Late suppression is harder than the table says.** With one, two or three extinguishers at 120 s or 180 s the
  careful crew member knocks the front of the fire down, but the cells under the bunks and the hot layer (above 50 C
  it preheats every cell) relight the knocked-down cells once their 20 s of agent wear off: the fire is not out, and
  it starves later instead, 288-515 s. Two extinguishers at 120 s, out at 129 s in the table, now fail; the fire
  peaks higher (3.99 MW) because the knock-down saved oxygen for the reflash. The table's "two by 120 s, three by
  180 s" was the room model's; with cells, a fire past about 100 s needs the hose team's sweep, mist or the vent.
- **Rooms with no furniture in the mockups' placement grow on bare deck** (110 s of dose): the forward switchboard
  and the magazine, whose cabinets and racks are layout systems the placement does not stand as props yet, grow
  slower (the magazine's 1 MW at 181 s, not 113; the switchboard's fire never passes 60 kW before the inert gas).
  That moves those rows by more than 15% and is the next thing to furnish, not to tune.
- **Water mist across every burning cell by heat release** takes longer to finish a fire than the room model's single
  cut: engineering's 1 MW fire with mist is out at 51 s, not 22. With the mist disabled engineering's fire, seeded at
  1 MW on 16 cells, involves its whole 300 m^2 floor and peaks at 23 MW (16 in the table) before its oxygen falls.
- **The vented case** has moved for three reasons, none of them the cells: the vent now warns 5 s before the dump
  opens (design 6a), the new oxygen and cold harm kill the crew member who stays, and the table's 18.7 %
  oxygen at the end is reproduced only if the dump is closed once the fire is out, which the harness now does.
- **Inert gas stays survivable.** The first oxygen harm rate began at 16 kPa and killed anyone left in an
  inert-gas-flooded room (12.5 % oxygen, about 12.7 kPa) in about 300 s, where the table had them impaired but alive.
  Recommendation taken (ask only with screenshots), 2026-10-09: the harm begins at 10.6 kPa, where `life-support`'s
  unconsciousness rule begins, so a flooded room impairs and a vented one kills. Rerun: the inert gas cases' crew are
  impaired at 63-64 s and alive at the end, as in the table.
- **Venting a room with someone in it** (design 6a's target, down within about a minute): incapacitated at 57 s and
  at 0 HP at 66 s from the command, so the rates stand. The preview's time to empty (28.8 s) matches the real vent
  (28.7 s).

<!-- fire_cases.mjs BEGIN -->
Written by `node tools/mockups/fire_cases.mjs --md openspec/changes/fire-spread/fire-cases.md` on 2026-10-09 (a measurement instrument, CLAUDE.md 4; fire-spread
design 4 and 6a). Each case of `damage-control` design 3 is run three ways through `docs/mockups/lib/shipsystems.js`:
the table as it stands, the room model alone (the cell model off, which should reproduce the table), and the cell
model (`firespread.js`, the rooms furnished as the mockups furnish them). Suppression cases are run with the cell model
twice: a careful crew member sweeping the burning cells nearest the door, and a careless one aiming at the room's centre.
Seeds: the quarters' fires start on the forward-wall bunk at x 7.3 m, the medbay's on a bed, the rest at the room's
centre. Times in seconds from ignition; the 0 HP and -50 HP times from the vent command.

| Case | Run | 1 MW | Air 60 C | Air 300 C | Smoke 2,000 ppm | Crew impaired / unconscious / dead | Out | Peak MW | Peak air C | Peak kPa | Oxygen at end % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Quarters, door shut (default) | table | 113 | 118 | 262 | 238 | 96 / 181 / 193 | 321 | 3.71 | 307 | 195 | 12.6 |
| | room model | 113 | 118 | 262 | 238 | 95 / 180 / 192 | 321 | 3.71 | 307 | 195 | 12.6 |
| | cells | 113 | 123 |  | 234 | 102 / 182 / 193 | 284 | 3.48 | 295 | 190 | 13.0 |
| Quarters, door held open | table | 113 | 120 | 267 | 246 | 97 / 184 / 196 | burning at 900 | 3.95 | 327 | 167 | 13.0 |
| | room model | 113 | 120 | 267 | 246 | 97 / 183 / 196 | burning at 900 | 3.95 | 327 | 167 | 13.0 |
| | cells | 113 | 125 | 273 | 239 | 104 / 185 / 196 | 545 | 3.75 | 327 | 176 | 12.9 |
| Quarters, one extinguisher at 30 s | table |  |  |  |  |  | 33 | 0.18 | 24 | 102 | 20.8 |
| | room model |  |  |  |  |  | 33 | 0.18 | 24 | 102 | 20.8 |
| | cells, careful |  |  |  |  |  | 31 | 0.06 | 22 | 101 | 20.9 |
| | cells, careless | 113 | 123 |  | 234 | 102 / 182 / 193 | 284 | 3.48 | 295 | 190 | 13.0 |
| Quarters, one extinguisher at 60 s | table |  |  |  |  |  | 67 | 0.40 | 31 | 104 | 20.6 |
| | room model |  |  |  |  |  | 67 | 0.40 | 31 | 104 | 20.6 |
| | cells, careful |  |  |  |  |  | 66 | 0.36 | 27 | 103 | 20.7 |
| | cells, careless | 113 | 123 |  | 234 | 102 / 182 / 193 | 284 | 3.48 | 295 | 190 | 13.0 |
| Quarters, one extinguisher at 90 s | table |  |  |  |  |  | 102 | 0.71 | 45 | 109 | 20.3 |
| | room model |  |  |  |  |  | 102 | 0.71 | 45 | 109 | 20.3 |
| | cells, careful |  |  |  |  |  | 103 | 0.69 | 41 | 107 | 20.4 |
| | cells, careless | 113 | 123 |  | 234 | 102 / 182 / 193 | 284 | 3.48 | 295 | 190 | 13.0 |
| Quarters, one extinguisher at 120 s | table | 113 | 118 | 364 | 324 | 96 / 225 / 244 | 407 | 3.29 | 300 | 193 | 12.6 |
| | room model | 113 | 118 | 363 | 323 | 95 / 224 / 242 | 407 | 3.29 | 300 | 192 | 12.6 |
| | cells, careful | 113 | 123 |  | 270 | 102 / 215 / 228 | 318 | 3.80 | 295 | 190 | 12.9 |
| | cells, careless | 113 | 123 |  | 234 | 102 / 182 / 193 | 284 | 3.47 | 295 | 190 | 12.9 |
| Quarters, two extinguishers at 120 s | table | 113 | 118 |  |  | 96 / - / - | 129 | 1.10 | 65 | 116 | 19.7 |
| | room model | 113 | 118 |  |  | 95 / - / - | 129 | 1.09 | 65 | 116 | 19.7 |
| | cells, careful | 113 | 124 |  | 472 | 102 / 431 / 442 | 515 | 3.99 | 290 | 189 | 13.0 |
| | cells, careless | 113 | 123 |  | 234 | 102 / 182 / 193 | 284 | 3.47 | 295 | 190 | 12.9 |
| Quarters, two extinguishers at 180 s | table | 113 | 118 |  | 327 | 96 / 181 / 193 | 415 | 2.55 | 291 | 190 | 12.7 |
| | room model | 113 | 118 |  | 325 | 95 / 180 / 193 | 414 | 2.56 | 292 | 190 | 12.7 |
| | cells, careful | 113 | 123 |  | 252 | 102 / 182 / 194 | 288 | 3.29 | 295 | 190 | 13.0 |
| | cells, careless | 113 | 123 |  | 235 | 102 / 182 / 193 | 284 | 3.35 | 295 | 190 | 12.9 |
| Quarters, three extinguishers at 180 s | table | 113 | 118 |  |  | 96 / 181 / - | 192 | 2.12 | 139 | 140 | 17.6 |
| | room model | 113 | 118 |  |  | 95 / 180 / 193 | 192 | 2.12 | 139 | 140 | 17.6 |
| | cells, careful | 113 | 123 |  | 266 | 102 / 182 / 194 | 298 | 3.96 | 295 | 190 | 13.0 |
| | cells, careless | 113 | 123 |  | 235 | 102 / 182 / 193 | 284 | 3.35 | 295 | 190 | 12.9 |
| Quarters, vented at 60 s | table |  |  |  |  | 66 / 92 / - | 132 | 0.63 | 30 | 104 | 18.7 |
| | room model |  |  |  |  | 70 / 96 / 151 | 137 | 0.68 | 32 | 105 | 18.4 |
| | cells |  |  |  |  | 70 / 96 / 151 | 105 | 0.54 | 28 | 103 | 20.1 |
| Medbay, door shut | table | 113 | 99 | 228 | 207 | 79 / 158 / 170 | 284 | 2.87 | 308 | 195 | 12.5 |
| | room model | 113 | 100 | 228 | 207 | 80 / 158 / 169 | 284 | 2.87 | 308 | 195 | 12.5 |
| | cells | 94 | 99 |  | 193 | 85 / 150 / 160 | 250 | 2.37 | 293 | 190 | 12.9 |
| Forward switchboard, inert gas at 30 s | table |  |  |  |  | 66 / - / - | 127 | 0.58 | 53 | 112 | 12.4 |
| | room model |  |  |  |  | 66 / - / - | 127 | 0.58 | 53 | 112 | 12.4 |
| | cells |  |  |  |  | 64 / - / - | 90 | 0.06 | 26 | 103 | 12.5 |
| Magazine, no action | table | 113 | 151 | 325 | 300 | 124 / 222 / 235 | 390 | 5.64 | 310 | 196 | 12.7 |
| | room model | 113 | 151 | 325 | 299 | 124 / 221 / 234 | 390 | 5.64 | 310 | 196 | 12.7 |
| | cells | 181 | 223 |  | 355 | 197 / 286 / 298 | 401 | 4.81 | 300 | 191 | 13.0 |
| Magazine, inert gas at 30 s | table |  |  |  |  | 63 / - / - | 131 | 0.63 | 33 | 105 | 12.5 |
| | room model |  |  |  |  | 63 / - / - | 131 | 0.63 | 33 | 105 | 12.5 |
| | cells |  |  |  |  | 63 / - / - | 90 | 0.06 | 23 | 102 | 12.5 |
| Engineering, 1 MW seed, water mist automatic | table | at once |  |  |  |  | 22 | 1.20 | 22 | 102 | 20.9 |
| | room model | at once |  |  |  |  | 22 | 1.14 | 22 | 102 | 20.9 |
| | cells | at once |  |  |  |  | 51 | 1.17 | 24 | 102 | 20.9 |
| Engineering, 1 MW seed, mist disabled | table | at once | 173 | 467 | 425 | 130 / 265 / 283 | 576 | 15.97 | 310 | 196 | 13.0 |
| | room model | at once | 176 | 471 | 428 | 133 / 267 / 285 | 580 | 15.96 | 310 | 196 | 13.0 |
| | cells | at once | 176 | 423 | 376 | 139 / 258 / 273 | 485 | 23.03 | 311 | 195 | 13.0 |
| Hangar, 300 kW seed, mist automatic | table | 66 |  |  |  |  | 87 | 1.14 | 27 | 103 | 20.7 |
| | room model | 66 |  |  |  |  | 87 | 1.14 | 27 | 103 | 20.7 |
| | cells | 82 |  |  |  |  | 103 | 1.20 | 27 | 102 | 20.8 |

## Moved by more than 15% (cell model against the table)

- Quarters, one extinguisher at 30 s: Peak MW 0.18 to 0.06 (-67%).
- Quarters, one extinguisher at 120 s: Smoke 2,000 ppm 324 to 270 (-17%); Out 407 to 318 (-22%); Peak MW 3.29 to 3.80 (+16%).
- Quarters, two extinguishers at 120 s: Out 129 to 515 (+299%); Peak MW 1.10 to 3.99 (+263%); Peak air C 65 to 290 (+346%); Peak kPa 116 to 189 (+63%); Oxygen at end % 19.7 to 13.0 (-34%).
- Quarters, two extinguishers at 180 s: Smoke 2,000 ppm 327 to 252 (-23%); Out 415 to 288 (-31%); Peak MW 2.55 to 3.29 (+29%).
- Quarters, three extinguishers at 180 s: Out 192 to 298 (+55%); Peak MW 2.12 to 3.96 (+87%); Peak air C 139 to 295 (+112%); Peak kPa 140 to 190 (+36%); Oxygen at end % 17.6 to 13.0 (-26%).
- Quarters, vented at 60 s: Out 132 to 105 (-20%).
- Medbay, door shut: 1 MW 113 to 94 (-17%); Peak MW 2.87 to 2.37 (-17%).
- Forward switchboard, inert gas at 30 s: Out 127 to 90 (-29%); Peak MW 0.58 to 0.06 (-90%); Peak air C 53 to 26 (-51%).
- Magazine, no action: 1 MW 113 to 181 (+60%); Air 60 C 151 to 223 (+48%); Smoke 2,000 ppm 300 to 355 (+18%).
- Magazine, inert gas at 30 s: Out 131 to 90 (-31%); Peak MW 0.63 to 0.06 (-90%); Peak air C 33 to 23 (-30%).
- Engineering, 1 MW seed, water mist automatic: Out 22 to 51 (+132%).
- Engineering, 1 MW seed, mist disabled: Out 576 to 485 (-16%); Peak MW 15.97 to 23.03 (+44%).
- Hangar, 300 kW seed, mist automatic: 1 MW 66 to 82 (+24%); Out 87 to 103 (+18%).

## Venting (design 6a)

- Quarters vented at 60 s with the fire burning: the preview said 28.8 s (5 s of warning and 23.8 s to 20 kPa); the crew member inside was at 0 HP 68 s and -50 HP 91 s after the command.
- Quarters vented with no fire and a crew member who stays: the preview said 28.8 s to 20 kPa and the room took 28.7 s (the dump opens at 5.0 s). Oxygen fell past 16 kPa at 10.2 s and past 6.3 kPa at 23.0 s; the air fell below 5 C at 25.8 s (3.8 C at 300 s). The crew member was incapacitated (under 20 HP, crew-on-deck 7) at 57.1 s, at 0 HP at 65.6 s, -50 HP at 86.7 s, dead at 86.7 s.

<!-- fire_cases.mjs END -->
