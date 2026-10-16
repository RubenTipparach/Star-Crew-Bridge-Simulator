# Design: the captain's picture on every screen

## Context

| Decided elsewhere | Where |
| --- | --- |
| Who steers the viewscreen; the captain can override | `bridge-stations` 7 and 11.2 |
| The captain's camera and CHASE on the bridge's main viewscreen | `chase-camera` 2 |
| The captain's zoom rides with his camera | `viewscreen-zoom` 3 |
| The gunner's sight has no viewscreen | `enemy-fighters` 3 |
| Consoles are full screen when seated | CLAUDE.md 10 |

## 1. One camera on screen

`console::on_screen(view, main, own) -> (feed, chase, zoom)`:

- `view` is `Bridge::view`, the camera the captain holds the viewscreen with, if he does;
- `main` is the captain's camera, CHASE and zoom (`Bridge::main_view`, `main_chase`, `main_zoom`);
- `own` is the console's own camera, CHASE and zoom, or `main` for the bridge's big screen.

While `view` is set, the result is `(view, main.chase, main.zoom)`; otherwise `own`. `Seat::view` fills each console's
`feed`, `chase` and `zoom` from it, so the panel's picture, the target bracket and the camera widget's cone agree; the
bridge's feed in `drill.rs` draws the big screen from it.

## 2. SCREEN sends the whole picture

SCREEN (`Act::ViewTake`) sends `MainView { feed, chase, zoom }` and then `Viewscreen(Some(feed))`, so the instant he
takes it, the big screen and every panel show what his own panel shows. While he holds it, his camera pad, CHASE and
zoom keep sending `MainView` (and the camera `Viewscreen`), as they do.

## 3. What each player sees

| Player | Sees the captain's picture on |
| --- | --- |
| The captain, seated | His VIEWSCREEN panel (his own camera: what goes up) |
| Anyone seated, but the gunner | Their VIEWSCREEN panel, while he holds the viewscreen |
| Anyone on foot on the bridge, the captain too | The main viewscreen, always |
| The gunner, seated | Nothing: the sight has no viewscreen |

## 4. Checks

- `console`: `on_screen` gives the captain's camera, CHASE and zoom while taken and the console's own otherwise.
- `seat.rs`: SCREEN sends `MainView` and `Viewscreen` with his camera; a helm console's view while it is taken has the
  captain's CHASE and zoom.
- The walking playtest: the captain takes the viewscreen, stands, walks to the main viewscreen and sees his picture.
