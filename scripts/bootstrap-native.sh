#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

for tool in xcrun swiftc cargo git cmake; do
  command -v "$tool" >/dev/null || { echo "Missing $tool. Install the macOS build prerequisites first." >&2; exit 1; }
done

mkdir -p src-tauri/resources .cache/whisper-build vendor
host_target="$(rustc -vV | sed -n 's/^host: //p')"
case "$host_target" in
  aarch64-apple-darwin) swift_arch=arm64 ;;
  x86_64-apple-darwin) swift_arch=x86_64 ;;
  *) echo "Unsupported macOS build target: $host_target" >&2; exit 1 ;;
esac
swiftc -parse-as-library -O -target "${swift_arch}-apple-macosx15.0" \
  -framework ScreenCaptureKit -framework AVFoundation -framework AppKit -framework CoreGraphics \
  native/Capture.swift -o src-tauri/resources/screenbox-capture

if [[ ! -d vendor/whisper.cpp ]]; then
  git clone --depth 1 --branch v1.8.3 https://github.com/ggml-org/whisper.cpp.git vendor/whisper.cpp
fi
if [[ ! -x src-tauri/resources/whisper-cli ]]; then
  cmake -S vendor/whisper.cpp -B .cache/whisper-build \
    -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=ON -DBUILD_SHARED_LIBS=OFF
  cmake --build .cache/whisper-build --config Release -j 4
  cp .cache/whisper-build/bin/whisper-cli src-tauri/resources/whisper-cli
fi
