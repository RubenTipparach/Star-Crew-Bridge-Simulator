#!/usr/bin/env bash
# Installs what the obs-record skill (.claude/skills/obs-record) needs on macOS: OBS Studio, ffmpeg, and the
# recorder's Python packages. Safe to run again; anything already installed is skipped.
# The two permission steps at the end are macOS's and can only be done by hand.
set -euo pipefail

if ! command -v brew >/dev/null 2>&1; then
    echo "Homebrew is needed: https://brew.sh" >&2
    exit 1
fi

if [ -d "/Applications/OBS.app" ]; then
    echo "OBS Studio: already installed"
else
    echo "OBS Studio: installing"
    brew install --cask obs
fi

if command -v ffmpeg >/dev/null 2>&1; then
    echo "ffmpeg: already installed"
else
    echo "ffmpeg: installing"
    brew install ffmpeg
fi

echo "Python packages: obsws-python, imageio-ffmpeg"
python3 -m pip install --user --upgrade obsws-python imageio-ffmpeg
python3 -c "import obsws_python, imageio_ffmpeg; print('Python packages: ok')"

cat <<'EOF'

Two steps by hand, once:

1. Screen Recording permission. System Settings > Privacy & Security > Screen Recording:
   turn on OBS (and the terminal app, for screencapture). Opening that page now.
2. OBS's WebSocket password. Open OBS once, then Tools > WebSocket Server Settings:
   enable the server and set a password. The recorder reads it from OBS's config.
EOF
open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture" || true
