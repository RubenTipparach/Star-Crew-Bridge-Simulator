"""Record one application window with OBS Studio, convert the recording to MP4,
and optionally copy it into a folder (such as a Google Drive for Desktop folder).

OBS is driven through its built-in WebSocket server (obs-websocket 5). Whatever
this script changes, it puts back: the video settings, the current scene, a
temporary scene it creates, and the WebSocket server switch. If OBS was not
running, it is started for the recording and closed afterwards.

Requires: OBS Studio 28+, Python 3.9+, `pip install obsws-python imageio-ffmpeg`.

Examples:
  # Launch a program, record 40 s after it prints "ready", at the window's size.
  python obs_record.py --launch "C:/game/game.exe --demo" --window-exe game.exe \
      --start-on "ready" --duration 40 --out flight.mp4

  # Record an already-open window for 20 s and drop it into Google Drive.
  python obs_record.py --window-title "My App" --duration 20 \
      --out demo.mp4 --copy-to "G:/My Drive/recordings"
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

try:
    import imageio_ffmpeg
    import obsws_python as obs
except ImportError:
    sys.exit("missing dependencies: pip install obsws-python imageio-ffmpeg")

# obsws-python logs every refused connection attempt with a traceback while OBS
# is still starting; the script reports failures itself.
import logging

logging.getLogger("obsws_python").setLevel(logging.CRITICAL)

SCENE = "obs-record (temporary)"
SOURCE = "obs-record window (temporary)"


def obs_dir() -> Path:
    if sys.platform == "win32":
        return Path(os.environ["APPDATA"]) / "obs-studio"
    if sys.platform == "darwin":
        return Path.home() / "Library/Application Support/obs-studio"
    return Path.home() / ".config/obs-studio"


def ws_config_path() -> Path:
    return obs_dir() / "plugin_config/obs-websocket/config.json"


def obs_running() -> bool:
    if sys.platform == "win32":
        # An OBS that has exited can linger in the process list with status
        # UNKNOWN while something still holds a handle to it; it is not running.
        out = subprocess.run(["tasklist", "/FI", "IMAGENAME eq obs64.exe",
                              "/FI", "STATUS ne UNKNOWN"],
                             capture_output=True, text=True).stdout
        return "obs64.exe" in out
    return subprocess.run(["pgrep", "-x", "obs"], capture_output=True).returncode == 0


def clear_stale_crash_markers():
    """OBS leaves a marker per run in `.sentinel` and deletes it on a clean exit.
    A leftover marker makes the next launch stop at a "did not properly shut
    down / run in Safe Mode?" prompt, and Safe Mode disables WebSockets. Only
    ever called while no OBS is running, so every marker is stale."""
    sentinel = obs_dir() / ".sentinel"
    if sentinel.is_dir():
        for f in sentinel.iterdir():
            try:
                f.unlink()
            except OSError:
                pass


def close_obs():
    """Close OBS the way a user would, so it writes its config and exits
    cleanly; force it only as a last resort, then clear what that leaves."""
    polite, force = ((["taskkill", "/IM", "obs64.exe"], ["taskkill", "/F", "/IM", "obs64.exe"])
                     if sys.platform == "win32" else
                     (["pkill", "-x", "obs"], ["pkill", "-9", "-x", "obs"]))
    subprocess.run(polite, capture_output=True)
    for _ in range(40):
        if not obs_running():
            return
        time.sleep(0.5)
    subprocess.run(force, capture_output=True)
    time.sleep(1)
    clear_stale_crash_markers()


def obs_executable() -> Path:
    candidates = [
        Path(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"),
        Path("/Applications/OBS.app/Contents/MacOS/OBS"),
        Path("/usr/bin/obs"),
    ]
    for c in candidates:
        if c.exists():
            return c
    sys.exit("OBS Studio not found; pass --obs PATH")


class ObsSession:
    """Connects to OBS, enabling its WebSocket server (and starting OBS) only
    for as long as the recording needs, and undoing both afterwards."""

    def __init__(self, obs_path: Path | None):
        self.obs_path = obs_path
        self.started_obs = False
        self.enabled_ws = False
        self.config_backup: str | None = None
        self.client = None

    def __enter__(self):
        cfg_path = ws_config_path()
        if not cfg_path.exists():
            sys.exit(f"no obs-websocket config at {cfg_path}; open OBS once, then "
                     "Tools > WebSocket Server Settings > set a password")
        running = obs_running()
        backup = cfg_path.with_name(cfg_path.name + ".obs-record-backup")
        if backup.exists() and not running:
            # A previous run was killed before it could switch the server back
            # off: finish its cleanup first.
            cfg_path.write_text(backup.read_text(encoding="utf-8-sig"), encoding="utf-8")
            backup.unlink()
            print("restored OBS's WebSocket config left behind by an interrupted run")
        cfg = json.loads(cfg_path.read_text(encoding="utf-8-sig"))
        if not cfg.get("server_enabled"):
            if running:
                sys.exit("OBS is running with its WebSocket server off. Either close OBS "
                         "(this script then enables the server for the run and turns it "
                         "off again), or enable it in Tools > WebSocket Server Settings.")
            self.config_backup = cfg_path.read_text(encoding="utf-8-sig")
            # On disk too, so a run killed mid-way is undone by the next one.
            backup.write_text(self.config_backup, encoding="utf-8")
            cfg["server_enabled"] = True
            cfg_path.write_text(json.dumps(cfg, indent=4), encoding="utf-8")
            self.enabled_ws = True
        if not running:
            exe = self.obs_path or obs_executable()
            clear_stale_crash_markers()
            # A normal window, not --minimize-to-tray: a tray-only OBS cannot be
            # closed cleanly, and an unclean exit brings the Safe Mode prompt.
            # --multi: an OBS that exited uncleanly can linger in the process
            # list holding its single-instance lock, and a new one then stops at
            # "OBS is already running". No running OBS was found above.
            subprocess.Popen([str(exe), "--disable-shutdown-check", "--multi"], cwd=str(exe.parent))
            self.started_obs = True
        for _ in range(180):
            try:
                self.client = obs.ReqClient(host="localhost", port=cfg["server_port"],
                                            password=cfg.get("server_password", ""),
                                            timeout=10)
                break
            except Exception:
                time.sleep(0.5)
        if self.client is None:
            self.__exit__(None, None, None)
            sys.exit("could not connect to OBS's WebSocket server within 90 s "
                     "(is OBS waiting on a dialog?)")
        # The server accepts connections before OBS has finished loading, and
        # answers "not ready" (code 207) until it has.
        for _ in range(120):
            try:
                self.client.get_video_settings()
                break
            except Exception:
                time.sleep(0.5)
        else:
            self.__exit__(None, None, None)
            sys.exit("OBS never became ready to take requests")
        return self.client

    def _restore_config(self):
        if self.enabled_ws and self.config_backup is not None:
            path = ws_config_path()
            path.write_text(self.config_backup, encoding="utf-8")
            path.with_name(path.name + ".obs-record-backup").unlink(missing_ok=True)

    def __exit__(self, *exc):
        if self.started_obs and obs_running():
            # OBS writes its config on exit; close it first, then restore.
            close_obs()
        self._restore_config()
        return False


def report_cadence(ff: str, video: str, fps: int) -> list[int]:
    """New frames in each second of `video`: a frame that is essentially the
    previous one again (OBS repeating it because the program did not deliver a
    new one in time) is not new. Frames are compared at 160x100 grey; after
    encoding, a repeat differs by under 0.03 of a level on average and real
    motion by more than 0.1 (measured on this project's captures). Prints a
    summary and returns the per-second counts."""
    w, h = 160, 100
    raw = subprocess.run([ff, "-v", "error", "-i", video, "-vf", f"scale={w}:{h},format=gray",
                          "-f", "rawvideo", "-"], capture_output=True).stdout
    n = w * h
    frames = [raw[i * n:(i + 1) * n] for i in range(len(raw) // n)]
    new = [True]
    for prev, cur in zip(frames, frames[1:]):
        diff = sum(abs(a - b) for a, b in zip(cur[::5], prev[::5])) / len(cur[::5])
        new.append(diff > 0.05)
    per_second = [sum(new[s:s + fps]) for s in range(0, len(new) - fps + 1, fps)]
    if per_second:
        full = sum(1 for c in per_second if c >= fps - 1)
        print(f"cadence: {len(per_second)} s, new frames per second min {min(per_second)} "
              f"median {sorted(per_second)[len(per_second) // 2]} of {fps}; "
              f"{full} of {len(per_second)} seconds at {fps - 1}+")
        worst = sorted(range(len(per_second)), key=lambda s: per_second[s])[:5]
        print("  slowest seconds: " + ", ".join(f"{s}s={per_second[s]}" for s in sorted(worst)))
    return per_second


def blank_lead(ff: str, video: str, look_s: float = 60.0) -> float:
    """Seconds of flat-colour frames (a white or black loading window) at the
    start of `video`: the time of the first frame with real content. Frames are
    sampled 20 times a second, shrunk to 64x40 grey; a frame is flat when its
    pixels barely vary. Zero when the video opens on content."""
    fps, w, h = 20, 64, 40
    raw = subprocess.run([ff, "-v", "error", "-i", video, "-t", str(look_s),
                          "-vf", f"fps={fps},scale={w}:{h},format=gray", "-f", "rawvideo", "-"],
                         capture_output=True).stdout
    n = w * h
    for i in range(len(raw) // n):
        frame = raw[i * n:(i + 1) * n]
        mean = sum(frame) / n
        spread = (sum((p - mean) ** 2 for p in frame) / n) ** 0.5
        if spread > 4.0:
            return i / fps
    return 0.0


def find_window(c, pattern_exe: str | None, pattern_title: str | None, timeout: float) -> str:
    """The window_capture item for the target window, e.g.
    'Title:WindowClass:app.exe'."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        items = c.get_input_properties_list_property_items(SOURCE, "window").property_items
        for item in items:
            value = item["itemValue"]
            if pattern_exe and value.lower().endswith(":" + pattern_exe.lower()):
                return value
            if pattern_title and pattern_title.lower() in value.lower():
                return value
        time.sleep(0.5)
    sys.exit("target window never appeared")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    target = p.add_argument_group("target")
    target.add_argument("--launch", help="command line to start before recording")
    target.add_argument("--cwd", help="working directory for --launch (default: the program's folder)")
    target.add_argument("--window-exe", help="capture the window of this executable, e.g. game.exe")
    target.add_argument("--window-title", help="capture the window whose title contains this")
    timing = p.add_argument_group("timing")
    timing.add_argument("--start-on", help="regex on the launched program's output: --duration "
                        "counts from the first matching line")
    timing.add_argument("--duration", type=float, help="seconds to keep recording")
    timing.add_argument("--stop-on", help="regex on the launched program's output that ends the recording")
    timing.add_argument("--max-seconds", type=float, default=600, help="hard cap (default 600)")
    video = p.add_argument_group("video")
    video.add_argument("--scale", type=float, default=1.0,
                       help="output size as a share of the window (default 1.0, the window's full size)")
    video.add_argument("--fps", type=int, default=60)
    out = p.add_argument_group("output")
    out.add_argument("--out", required=True, help="MP4 path to write")
    out.add_argument("--copy-to", help="folder to copy the MP4 into, e.g. a Google Drive for Desktop folder")
    out.add_argument("--keep-mkv", action="store_true", help="keep OBS's original MKV")
    out.add_argument("--keep-blank-lead", action="store_true",
                     help="keep flat-colour frames at the start (a loading screen) instead of cutting them")
    out.add_argument("--contact-sheet", action="store_true", help="also write <out>_frames.png, six frames across the video")
    out.add_argument("--check-cadence", action="store_true",
                     help="report how many new frames each second of the MP4 carries (60/60 is perfect at 60 fps)")
    p.add_argument("--obs", type=Path, help="path to the OBS executable")
    a = p.parse_args()

    if not (a.window_exe or a.window_title):
        p.error("say which window: --window-exe or --window-title")
    if a.duration is None and not a.stop_on:
        p.error("say when to stop: --duration and/or --stop-on")
    if (a.start_on or a.stop_on) and not a.launch:
        p.error("--start-on/--stop-on read the output of a program started with --launch")

    # A window is picked by title and executable, and two copies of one
    # program look alike: with another copy already open, the capture can
    # film that one instead (it happened: a player's own game, not the flight
    # launched). Refuse rather than guess.
    if a.launch and a.window_exe and sys.platform == "win32":
        running = subprocess.run(["tasklist", "/FI", f"IMAGENAME eq {a.window_exe}",
                                  "/FI", "STATUS ne UNKNOWN"],
                                 capture_output=True, text=True).stdout
        if a.window_exe.lower() in running.lower():
            sys.exit(f"{a.window_exe} is already running; close it first, or the capture may "
                     "film that copy instead of the one launched")
    # The program runs in its own folder (--cwd aside), where a path relative
    # to here no longer points at it: resolve it now, before OBS is started.
    argv = None
    if a.launch:
        argv = shlex.split(a.launch, posix=(sys.platform != "win32"))
        argv = [s.strip('"') for s in argv]
        found = shutil.which(argv[0])
        if not found:
            sys.exit(f"--launch: no program at {argv[0]}")
        argv[0] = str(Path(found).resolve())
    out_path = Path(a.out).resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    log_path = out_path.with_suffix(".log")

    with ObsSession(a.obs) as c:
        orig_video = c.get_video_settings()
        orig_scene = c.get_current_program_scene().current_program_scene_name
        orig_format = c.get_profile_parameter("SimpleOutput", "RecFormat2").parameter_value
        created = False
        input_created = False
        proc = None
        mkv = None
        try:
            if SCENE in [s["sceneName"] for s in c.get_scene_list().scenes]:
                c.remove_scene(SCENE)
            c.create_scene(SCENE)
            created = True
            # MKV survives a crash mid-recording; it is remuxed to MP4 after.
            c.set_profile_parameter("SimpleOutput", "RecFormat2", "mkv")

            start_evt, stop_evt = threading.Event(), threading.Event()
            if argv:
                cwd = a.cwd or str(Path(argv[0]).parent)
                proc = subprocess.Popen(argv, cwd=cwd, stdout=subprocess.PIPE,
                                        stderr=subprocess.STDOUT, text=True,
                                        encoding="utf-8", errors="replace")
                start_re = re.compile(a.start_on) if a.start_on else None
                stop_re = re.compile(a.stop_on) if a.stop_on else None

                def pump():
                    with open(log_path, "w", encoding="utf-8") as log:
                        for line in proc.stdout:
                            log.write(line)
                            log.flush()
                            if start_re and start_re.search(line):
                                start_evt.set()
                            if stop_re and stop_re.search(line):
                                stop_evt.set()
                threading.Thread(target=pump, daemon=True).start()
            if not a.start_on:
                start_evt.set()

            c.create_input(SCENE, SOURCE, "window_capture", {"method": 2, "cursor": False}, True)
            input_created = True
            window = find_window(c, a.window_exe, a.window_title, 120)
            c.set_input_settings(SOURCE, {"window": window, "method": 2, "cursor": False}, True)
            item = c.get_scene_item_id(SCENE, SOURCE).scene_item_id
            print(f"found window {window}")
            # On program first: OBS renders only what is showing, and a window
            # capture reports no size until it has rendered.
            c.set_current_program_scene(SCENE)
            # The canvas takes the window's own size; the output is scaled.
            w = h = 0
            for _ in range(240):
                t = c.get_scene_item_transform(SCENE, item).scene_item_transform
                w, h = int(t["sourceWidth"]), int(t["sourceHeight"])
                if w and h:
                    break
                time.sleep(0.25)
            if not (w and h):
                sys.exit("the captured window reports no size after 60 s (minimized, "
                         "or not drawing?)")
            ow = max(2, int(w * a.scale) // 2 * 2)
            oh = max(2, int(h * a.scale) // 2 * 2)
            c.set_video_settings(a.fps, 1, w, h, ow, oh)
            c.set_scene_item_transform(SCENE, item, {
                "positionX": 0, "positionY": 0, "boundsType": "OBS_BOUNDS_SCALE_INNER",
                "boundsWidth": w, "boundsHeight": h})
            print(f"capturing {window} at {w}x{h}, output {ow}x{oh} @ {a.fps} fps")

            # Recording starts at once; a loading screen at the start is cut
            # afterwards (see `blank_lead`), so no program-specific cue is needed
            # to skip it. --start-on only says where --duration counts from.
            c.start_record()
            began = time.monotonic()
            print("recording")
            if not start_evt.wait(a.max_seconds):
                raise SystemExit("--start-on line never appeared")
            if a.start_on:
                print(f"start line seen {time.monotonic() - began:.1f} s in")
            limit = a.duration if a.duration is not None else a.max_seconds
            stop_evt.wait(max(0.0, min(limit, a.max_seconds - (time.monotonic() - began))))
            mkv = c.stop_record().output_path
            print(f"stopped after {time.monotonic() - began:.1f} s: {mkv}")
        finally:
            if proc and proc.poll() is None:
                proc.terminate()
            try:
                if c.get_record_status().output_active:
                    mkv = c.stop_record().output_path
            except Exception:
                pass
            # Stopping returns before the output has finished writing, and OBS
            # refuses to change video settings while an output is active.
            for _ in range(60):
                try:
                    if not c.get_record_status().output_active:
                        break
                except Exception:
                    pass
                time.sleep(0.5)
            # Each step on its own, so one failure does not leave the rest
            # undone: the scene, its input (removing a scene does not remove
            # its inputs, and an orphan is saved with the collection), the
            # video settings and the recording format.
            steps = [
                ("scene", lambda: c.set_current_program_scene(orig_scene)),
                ("temporary scene", lambda: created and c.remove_scene(SCENE)),
                ("temporary input", lambda: input_created and c.remove_input(SOURCE)),
                ("video settings", lambda: c.set_video_settings(
                    orig_video.fps_numerator, orig_video.fps_denominator,
                    orig_video.base_width, orig_video.base_height,
                    orig_video.output_width, orig_video.output_height)),
                ("recording format",
                 lambda: c.set_profile_parameter("SimpleOutput", "RecFormat2", orig_format)),
            ]
            failed = []
            for name, step in steps:
                for attempt in range(3):
                    try:
                        step()
                        break
                    except Exception as error:
                        if attempt == 2:
                            failed.append(f"{name}: {error}")
                        time.sleep(1.0)
            if failed:
                print("OBS NOT fully restored: " + "; ".join(failed))
            else:
                print("OBS settings restored")

    time.sleep(1.5)
    ff = imageio_ffmpeg.get_ffmpeg_exe()
    trim = 0.0 if a.keep_blank_lead else blank_lead(ff, mkv)
    if trim > 0:
        print(f"cutting {trim:.2f} s of blank frames from the start")
    if trim > 0:
        # A frame-exact cut needs a re-encode: a stream copy can only begin on a
        # keyframe, which may be seconds away.
        cmd = [ff, "-y", "-v", "error", "-ss", f"{trim:.3f}", "-i", mkv, "-c:v", "libx264",
               "-crf", "18", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-c:a", "copy",
               "-movflags", "+faststart", str(out_path)]
    else:
        cmd = [ff, "-y", "-v", "error", "-i", mkv, "-c", "copy", "-movflags", "+faststart",
               str(out_path)]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        sys.exit("remux failed: " + r.stderr)
    if not a.keep_mkv:
        Path(mkv).unlink(missing_ok=True)
    print(f"MP4: {out_path} ({out_path.stat().st_size / 1e6:.1f} MB)")

    if a.check_cadence:
        report_cadence(ff, str(out_path), a.fps)

    if a.contact_sheet:
        sheet = out_path.with_name(out_path.stem + "_frames.png")
        probe = subprocess.run([ff, "-i", str(out_path)], capture_output=True, text=True).stderr
        m = re.search(r"Duration: (\d+):(\d+):([\d.]+)", probe)
        dur = int(m[1]) * 3600 + int(m[2]) * 60 + float(m[3]) if m else 0
        if dur > 0:
            # By frame number: a window in time narrower than a frame misses
            # the frame about half the time (timestamps are rounded to the
            # millisecond), and a wider one can take two.
            frames = [int(dur * a.fps * (i + 0.5) / 6) for i in range(6)]
            vf = "select='" + "+".join(f"eq(n,{n})" for n in frames) + "',tile=3x2"
            subprocess.run([ff, "-y", "-v", "error", "-i", str(out_path), "-vf", vf,
                            "-frames:v", "1", "-vsync", "0", str(sheet)], capture_output=True)
            if sheet.exists():
                print(f"contact sheet: {sheet}")

    if a.copy_to:
        dest = Path(a.copy_to)
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copy2(out_path, dest / out_path.name)
        print(f"copied to {dest / out_path.name}")


if __name__ == "__main__":
    main()
