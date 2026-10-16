#!/bin/zsh
set -euo pipefail
cd "${0:A:h}"
for tool in xcrun swiftc cargo node npm ffmpeg cmake git; do
  command -v "$tool" >/dev/null || { print -u2 "Missing $tool. Install the prerequisites in README.md."; exit 1; }
done
if [[ $(sw_vers -productVersion | cut -d. -f1) -lt 15 ]]; then
  print -u2 "Screenbox requires macOS 15 or newer."
  exit 1
fi
mkdir -p .cache/npm
bash scripts/bootstrap-native.sh
npm_config_cache="$PWD/.cache/npm" npm_config_registry="https://registry.npmjs.org" npm ci
npm run tauri dev
