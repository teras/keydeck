#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Panayotis Katsaloulis
#
# Builds the keydeck daemon and stages it as the Tauri sidecar the macOS bundle
# embeds, so `cargo tauri build` ships the daemon inside keydeck-config.app
# (Tauri strips the target triple on install, placing `keydeck` next to
# `keydeck-config`). Run this BEFORE `cargo tauri build`.
#
# Usage: ./stage-sidecar.sh [target-triple]
#   default target: universal-apple-darwin (fat x86_64 + arm64 binary)
set -euo pipefail

cd "$(dirname "$0")"

TARGET="${1:-universal-apple-darwin}"
BIN_DIR="keydeck-config/src-tauri/binaries"
mkdir -p "$BIN_DIR"

if [ "$TARGET" = "universal-apple-darwin" ]; then
    # A universal Tauri build needs all three sidecar names: the per-arch ones
    # satisfy each arch's build-script check (it runs once per cargo target), while
    # the bundler copies the merged universal-named binary into the .app.
    for arch in x86_64-apple-darwin aarch64-apple-darwin; do
        cargo build --release --bin keydeck --target "$arch"
        cp "target/$arch/release/keydeck" "$BIN_DIR/keydeck-$arch"
        echo "Staged sidecar: $BIN_DIR/keydeck-$arch"
    done
    lipo -create \
        "$BIN_DIR/keydeck-x86_64-apple-darwin" \
        "$BIN_DIR/keydeck-aarch64-apple-darwin" \
        -output "$BIN_DIR/keydeck-universal-apple-darwin"
    echo "Staged sidecar: $BIN_DIR/keydeck-universal-apple-darwin"
else
    cargo build --release --bin keydeck --target "$TARGET"
    cp "target/$TARGET/release/keydeck" "$BIN_DIR/keydeck-$TARGET"
    echo "Staged sidecar: $BIN_DIR/keydeck-$TARGET"
fi
