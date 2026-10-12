# Design: crew uniforms by division

## Context

The owner, 2026-10-11: "Crew members should have different colored skirts based on roles like start Trek: operations,
science, engineering, and support", "Gold, blue, red, green". The figure is `crew-npcs` 7's (one tunic colour over a
96-triangle body); this change only chooses which colour each person wears.

## 1. The divisions

| Division | Tunic | sRGB |
| --- | --- | --- |
| Operations | Gold | 0.86, 0.66, 0.18 |
| Science | Blue | 0.22, 0.42, 0.78 |
| Engineering | Red | 0.78, 0.22, 0.18 |
| Support | Green | 0.30, 0.60, 0.30 |

They live in `data/crew/company.json` (`divisions`), the one source; a department names its division.

## 2. Who wears what

| Who | Division | Why |
| --- | --- | --- |
| Command, Security (walk) | Operations | Running the ship and guarding it |
| Engineering (walk) | Engineering | |
| Medical (walk) | Science | As the series dresses its doctors |
| Deckhands, Galley (walk) | Support | Stores, the hangar, the mess |
| Captain, Helm, Tactical, both laser banks, Gunner (drill) | Operations | The bridge's flying and fighting |
| Science (drill) | Science | |
| Engineering (drill) | Engineering | |

The drill's stations name their division in `data/stations.json` (`divisions`); a body that holds no station yet
wears operations. Recommendation taken (ask only with screenshots): the captain in operations gold, as the series'
command gold, and medical in science blue.

## 3. Drawing

The deck compiler builds one figure a division (`figure_<division>`) and the player's own map marker, which stays
amber and white so the player finds themselves on the map. The walk draws a bot in its department's division figure
and its map marker in the division's colour; the drill's bridge draws each body in the figure of the station it holds
in the crew list.

## 4. Checks

- A test loads `company.json` and `stations.json` and finds every department's and every station's division among the
  divisions (CLAUDE.md 6.6: validate the real artifact).
- Captures of the walk and the drill's bridge show the four colours.

## 5. Pi 5 budget

Four figures of 96 triangles in place of seven: fewer meshes. No draw call, memory or network cost added.
