#!/bin/bash

set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

# Install KiwiRecorder if not installed
if [ -e /usr/local/src/kiwiclient/kiwirecorder.py ]; then
    echo "✅ KiwiRecorder is already installed"
else
    echo "⬜ Refreshing package lists..."
    sudo apt update -qq
    echo "⬜ Installing dependencies..."
    sudo apt install -qq -y python3 python3-pip git make libsamplerate0
    sudo apt install -qq -y python3-numpy python3-cffi

    echo "⬜ Cloning kiwiclient repository..."
    sudo mkdir -p /usr/local/src
    cd /usr/local/src
    git clone https://github.com/jks-prv/kiwiclient.git kiwiclient
    cd kiwiclient

    echo "⬜ Building libsamplerate wrapper..."
    make samplerate_build

    echo "✅ KiwiRecorder installed successfully"
fi