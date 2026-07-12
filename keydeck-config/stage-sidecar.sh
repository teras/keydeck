#!/bin/bash
# Stage the keydeck daemon as a Tauri sidecar (externalBin) so that
# `cargo tauri build` bundles it next to the config UI on macOS (and Linux, if
# ever used there). Tauri strips the target-triple suffix on install, so the
# daemon lands beside keydeck-config where find_keydeck_binary() locates it.
#
# Run before `cargo tauri build`:  ./stage-sidecar.sh
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # keydeck-config/
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"                          # repo root
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
BIN_DIR="$SCRIPT_DIR/src-tauri/binaries"
EXT=""
case "$TRIPLE" in *windows*) EXT=".exe" ;; esac

echo "Building keydeck daemon (release) for $TRIPLE ..."
( cd "$ROOT" && cargo build --release --bin keydeck )
mkdir -p "$BIN_DIR"
cp "$ROOT/target/release/keydeck$EXT" "$BIN_DIR/keydeck-$TRIPLE$EXT"
echo "Staged sidecar: $BIN_DIR/keydeck-$TRIPLE$EXT"
