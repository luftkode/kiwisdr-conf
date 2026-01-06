#!/bin/bash

set -euo pipefail

DIR="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")" && pwd)"

HOST_ARCH=$(uname -m)
echo "⬜ Host architecture: $HOST_ARCH"

# Rust targets
TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "armv7-unknown-linux-gnueabihf"
)

# Output directory for final binaries
BUILD_DIR="$DIR/build"
mkdir -p "$BUILD_DIR"

if ! docker info &>/dev/null; then
    echo "⬜ Updating package lists..."
    sudo apt update
    echo "⬜ Installing Docker"
    sudo apt install docker.io -y
    echo "✅ Docker is now installed."
fi

# Ensure user is in Docker group
if ! groups "$USER" | grep -q "\bdocker\b"; then
    echo "⬜ Adding $USER to the docker group..."
    sudo usermod -aG docker "$USER"
    echo "ℹ️ You must log out and log back in (or run 'newgrp docker') for Docker permissions to take effect."
    exit 1
fi

# Ensure Rust targets are installed
echo "⬜ Ensuring Rust targets are installed..."
for target in "${TARGETS[@]}"; do
    rustup target add "$target" || true
done

# Install cross if missing
if ! command -v cross &>/dev/null; then
    echo "⬜ Installing cross (for easy cross-compilation)..."
    cargo install cross
fi

# Map target triples to suffixes
get_suffix() {
    case "$1" in
        x86_64-unknown-linux-gnu) echo ".x86_64" ;;
        aarch64-unknown-linux-gnu) echo ".aarch64" ;;
        armv7-unknown-linux-gnueabihf) echo ".armv7" ;;
        *) echo "" ;;
    esac
}

# Build function using cross
build_target() {
    cd "$DIR"
    local target=$1
    echo "⬜ Building for target: $target"
    cross build --release --target "$target"
    local suffix
    suffix=$(get_suffix "$target")
    local src="target/$target/release/backend"
    local dst="$BUILD_DIR/backend${suffix}"
    cp "$src" "$dst"
    chmod +x "$dst"
    echo "✅ Built: $dst"
}

# Build all targets sequentially
for target in "${TARGETS[@]}"; do
    build_target "$target"
done

echo "✅ All builds completed successfully!"
echo "ℹ️ Now signing binaries"
cd "$BUILD_DIR/../.."
./sign.sh
echo "✅ Signing completed successfully!"
