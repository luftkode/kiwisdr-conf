#!/bin/bash

set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

source /tmp/kiwisdr-conf-main/setup.sh # Load verify_signature()
DIR=/tmp/kiwisdr-conf-main

ARCH=$(uname -m)
echo "⬜ Detected system architecture: $ARCH"

# Map system architecture to build suffix
case "$ARCH" in
    x86_64)
        SUFFIX=".x86_64"
        ;;
    aarch64)
        SUFFIX=".aarch64"
        ;;
    armv7l|armv6l)
        SUFFIX=".armv7"
        ;;
    *)
        echo "❌ Unsupported architecture: $ARCH"
        echo "   Supported: x86_64, aarch64, armv7l, armv6l"
        exit 1
        ;;
esac

BUILD_DIR="$DIR/backend/build"
BINARY_PATH="$BUILD_DIR/backend${SUFFIX}"

# Verify binary exists
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "❌ Compiled binary not found for architecture: $ARCH"
    echo "   Expected at: $BINARY_PATH"
    echo "   Try building with:"
    echo "     cargo build --release --target=$ARCH"
    exit 1
fi

echo "⬜ Verifying and installing api..."
verify_signature "$BINARY_PATH"
sudo install -m 755 "$BINARY_PATH" /usr/local/bin/kiwirecorder-backend

# Ensure data directories exist
sudo mkdir -p /var/recorder/recorded-files/gnss_pos/

SERVICE_SRC="$DIR/backend/backend.service"
SERVICE_DEST="/etc/systemd/system/kiwirecorder-backend.service"

echo "⬜ Setting up systemd service..."
verify_signature "$SERVICE_SRC"
sudo cp "$SERVICE_SRC" "$SERVICE_DEST"

# Reload systemd units
sudo systemctl daemon-reexec
sudo systemctl daemon-reload

# Stop only if running
if systemctl is-active --quiet kiwirecorder-backend.service; then
    sudo systemctl stop kiwirecorder-backend.service
fi

sudo systemctl enable kiwirecorder-backend.service
sudo systemctl restart kiwirecorder-backend.service

systemctl status kiwirecorder-backend.service

echo "✅ Api setup complete."
echo "ℹ️ To view logs: journalctl -u kiwirecorder-backend.service -f"
