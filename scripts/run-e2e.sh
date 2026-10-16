#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
export SCREENBOX_TEST_DATA_DIR="$HOME/Library/Application Support/com.morso.screenbox/e2e-test"
export SCREENBOX_TEST_EXPORT_DIR="$PWD/.cache/e2e-exports"
export VITE_E2E=1
mkdir -p "$SCREENBOX_TEST_DATA_DIR" "$SCREENBOX_TEST_EXPORT_DIR"
npm run tauri build -- --debug --features e2e --no-bundle
./node_modules/.bin/wdio run wdio.conf.js
