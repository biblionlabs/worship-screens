#!/usr/bin/env bash
set -euxo pipefail

if [[ "$DIST_TARGET" == *"unknown-linux-gnu"* ]]; then
  sudo apt-get update
  sudo apt-get install -y \
    pkg-config \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-gl1.0-dev
fi

if [[ "$DIST_TARGET" == *"apple-darwin"* ]]; then
  brew install \
    gstreamer \
    gst-plugins-base \
    gst-plugins-good \
    gst-plugins-bad
fi
