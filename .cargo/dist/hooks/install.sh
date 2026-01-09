#!/usr/bin/env bash
set -euo pipefail

echo "==> Checking GStreamer installation..."

if command -v gst-launch-1.0 >/dev/null 2>&1; then
  echo "GStreamer already installed."
  exit 0
fi

echo "GStreamer not found. Attempting to install..."

# -----------------------------
# Debian / Ubuntu / Linux Mint
# -----------------------------
if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update
  sudo apt-get install -y \
    gstreamer1.0-tools \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad
  exit 0
fi

# -----------------------------
# Fedora / RHEL / Rocky / Alma
# -----------------------------
if command -v dnf >/dev/null 2>&1; then
  sudo dnf install -y \
    gstreamer1 \
    gstreamer1-plugins-base \
    gstreamer1-plugins-good \
    gstreamer1-plugins-bad
  exit 0
fi

# -----------------------------
# Arch / Manjaro
# -----------------------------
if command -v pacman >/dev/null 2>&1; then
  sudo pacman -Sy --noconfirm \
    gstreamer \
    gst-plugins-base \
    gst-plugins-good \
    gst-plugins-bad
  exit 0
fi

# -----------------------------
# openSUSE
# -----------------------------
if command -v zypper >/dev/null 2>&1; then
  sudo zypper install -y \
    gstreamer \
    gstreamer-plugins-base \
    gstreamer-plugins-good \
    gstreamer-plugins-bad
  exit 0
fi

# -----------------------------
# Unsupported
# -----------------------------
cat >&2 <<EOF

ERROR: Unsupported Linux distribution.

requires GStreamer 1.0 and the following plugins:
  - gst-plugins-base
  - gst-plugins-good
  - gst-plugins-bad

After installing GStreamer, re-run the installer.
EOF

exit 1
