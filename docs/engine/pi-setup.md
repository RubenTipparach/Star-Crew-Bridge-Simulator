# Setting up a Raspberry Pi 5 for Star Crew

How to make a Pi 5 that builds and runs the engine, runs the probe, and can be the main server
(openspec/changes/engine-stack, design sections 1, 10 and 11). The client floor is a **Pi 5 with
1 GB**; a **4 GB Pi 5** can be the main server. Cross-compiling from a desktop (task 2.2) is not built
yet, so for now the Pi builds the game itself: slow the first time (SDL is built from source, about
10-20 minutes on a Pi 5), quick after that.

## 1. The card

- Raspberry Pi Imager: **Raspberry Pi OS Lite (64-bit)**, the current release (Bookworm or later).
  Lite: no desktop, so the client draws straight to the display through KMS/DRM (design section 4).
- In the imager's settings: a hostname (`starcrew-pi`), a user, Wi-Fi if you need it, and **SSH on**.
- A 1 GB board for the client measurements; note which board, its RAM and its cooler for the report.
- The Active Cooler on any board that will run the probe for long or be the server.

## 2. First boot

```sh
ssh <user>@starcrew-pi.local
sudo apt update && sudo apt full-upgrade -y
sudo apt install -y git build-essential cmake ninja-build pkg-config python3 \
  libegl-dev libgles-dev libdrm-dev libgbm-dev libudev-dev libinput-dev libxkbcommon-dev \
  libasound2-dev libpulse-dev
# The groups the client needs to open the display, input devices and audio without root:
sudo usermod -aG video,render,input,audio "$USER"
sudo reboot
```

Rust, for your user (not root):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

On a 1 GB board the first build can run out of memory. Give it swap for the build (not for play):

```sh
sudo dphys-swapfile swapoff
sudo sed -i 's/^CONF_SWAPSIZE=.*/CONF_SWAPSIZE=2048/' /etc/dphys-swapfile
sudo dphys-swapfile setup && sudo dphys-swapfile swapon
```

and build with one job less than the cores if it still struggles: `cargo build --release -j 2`.

## 3. Build

```sh
git clone https://github.com/RubenTipparach/Star-Crew-Bridge-Simulator.git
cd Star-Crew-Bridge-Simulator
cargo build --release
```

The shaders are already compiled into the repository (`crates/sc-render/src/shaders/`), so the Pi does
not need `sokol-shdc`. `RUSTFLAGS="-C target-cpu=cortex-a76" cargo build --release` tunes for the
Pi 5's cores.

## 4. The probe (what the owner runs first)

From the Pi's console or over SSH (the client takes the display through KMS/DRM, so nothing else may
be using it; on Raspberry Pi OS Lite nothing is):

```sh
SDL_VIDEODRIVER=kmsdrm ./target/release/sc-probe --out docs/benchmarks/$(date +%F)-pi5-probe
```

It runs every built scene full screen at 1920 x 1080 with 3D at 1280 x 720, three repeats, and writes
`report.json` and `report.md` into that folder. Then commit the folder and push it (or copy it off the
Pi): that report corrects the budget table (task 2.6). Note in the commit which Pi it was (RAM,
cooler, any overclock). Escape stops it early.

## 5. The ship

The compiled ship is not in git (57 MB, rebuilt from the deck plan). Build it on a desktop with Node and
Playwright, then copy it over:

```sh
# on the desktop, in the repository
node tools/deck/export_deck.mjs
cargo run --release -p sc-tools -- deckc
scp compiled/tern.deck <user>@starcrew-pi.local:Star-Crew-Bridge-Simulator/compiled/
```

On the Pi:

```sh
SDL_VIDEODRIVER=kmsdrm ./target/release/sc-client
```

Click to look around, W A S D to fly, Space and C up and down, Shift faster, 1, 2 and 3 for the
lighting states, F12 a capture (into `captures/`), Escape to let the mouse go and again to quit.
Keyboard and mouse plug into the Pi's USB ports.

## 6. As the main server (later)

`sc-server` is a skeleton today (the 30 Hz loop). When it hosts games it runs as a systemd service on
a 4 GB Pi 5 on Ethernet; the unit file and the deploy script are task 3.6 of `engine-stack`.

## When something is wrong

| You see | Try |
| --- | --- |
| `SDL_Init: ... kmsdrm not available` | Run from the Pi's own console or SSH with nothing else on the display; check the `video` and `render` groups (`groups`) |
| A black screen or garbage on a Pi 5 | SDL's atomic KMS path is on by default in our build (`SDL_KMSDRM_ATOMIC=1`); report it with `dmesg` output: it is the platform risk the probe exists to find (design section 13) |
| The build is killed | Swap (section 2), or `-j 1` |
| `no ship (compiled/tern.deck ...)` | Section 5: the client shows its test room until the deck is copied over |
