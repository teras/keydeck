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
    cargo build --release --bin keydeck --target x86_64-apple-darwin
    cargo build --release --bin keydeck --target aarch64-apple-darwin
    lipo -create \
        target/x86_64-apple-darwin/release/keydeck \
        target/aarch64-apple-darwin/release/keydeck \
        -output "$BIN_DIR/keydeck-$TARGET"
else
    cargo build --release --bin keydeck --target "$TARGET"
    cp "target/$TARGET/release/keydeck" "$BIN_DIR/keydeck-$TARGET"
fi

echo "Staged sidecar: $BIN_DIR/keydeck-$TARGET"
