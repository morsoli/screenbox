<div align="center">
  <img src="./public/favicon.svg" alt="Screenbox icon" width="88" />
  <h1>Screenbox</h1>
  <p>Record your screen. Make every detail count.</p>
  <p>
  <a href="./README_CN.md">简体中文</a> ·
  <a href="https://github.com/morsoli/screenbox/releases/latest">Download</a> ·
  <a href="#implementation">Implementation</a> ·
  <a href="#run-locally">Run locally</a>
  </p>
  <p>
  <img src="https://img.shields.io/badge/macOS-15%2B-111111?logo=apple&logoColor=white" alt="macOS 15 or newer" />
  <img src="https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=white" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=111111" alt="React 19" />
  <img src="https://img.shields.io/badge/TypeScript-6-3178C6?logo=typescript&logoColor=white" alt="TypeScript 6" />
  <a href="https://github.com/morsoli/screenbox/releases/latest"><img src="https://img.shields.io/github/downloads/morsoli/screenbox/total?label=downloads" alt="GitHub downloads" /></a>
  <img src="https://img.shields.io/badge/release-macOS_.pkg-4C7D5A?logo=apple&logoColor=white" alt="macOS installer package" />
  </p>
</div>

---

Screenbox is a personal macOS screen recorder. It captures a display, a window, or a region on one display, then lets you trim, crop, follow the cursor, highlight it, add text callouts, and export a local MP4 or WebM.

## Run locally

Requires **macOS 15 or newer**, Xcode Command Line Tools, Rust, Node.js 22+, CMake, and ffmpeg on `PATH`. Install those once with your preferred package manager. From this directory, use one command:

```sh
./dev.sh
```

The script checks prerequisites, builds the Swift ScreenCaptureKit helper and a pinned static `whisper.cpp` CLI, installs npm dependencies, and opens the Tauri app. The first build can take several minutes. `vendor/`, `.cache/`, and binaries under `src-tauri/resources/` are generated locally and ignored by Git. For a packaged app after bootstrapping, run `npm run build`; the result is under `src-tauri/target/release/bundle/`.

To customize local paths, copy `.env.example` to `.env` and edit it. `.env` is ignored by Git. Do not put credentials in tracked files. Screenbox has no credential requirement.

## Interface settings

Open **Settings** in the top bar to choose **Follow system**, **简体中文**, or **English**, and **Follow system**, **Light**, or **Dark** appearance. Both settings default to the macOS preference and are saved in the local Screenbox database. Language changes apply to the main window, recording timer, menu bar, status and error messages, and the headings in newly generated transcript and summary Markdown. Spoken transcript text stays in its original language.

## First recording

1. Open Screenbox and choose **Full screen**, **Window**, or **Region**. Drag inside the region selector to choose a rectangle on that display.
2. Leave **Microphone audio** on if you want voice in the video. Click **Start recording**. macOS may ask for Screen Recording and Microphone permissions; grant them in **System Settings → Privacy & Security** and reopen the app if capture fails.
3. Stop from the floating timer, the app, or the menu bar. Select the saved recording, set the trim and visual options, then click **Export video** to use the default folder. Use **Choose a different export folder** for another location.

Each export creates `name.mp4` or `name.webm`, `name.poster.jpg`, `name.transcript.md`, and `name.summary.md` in the same folder. The app offers **Copy file path** and **Reveal in Finder**. The first voiced export downloads a multilingual `whisper.cpp` model into Application Support; later transcription runs offline. If model download or transcription fails, the video and companion Markdown files remain available with a retry message. Export again after fixing the issue.

## Implementation

1. **Recording:** A native Swift helper uses ScreenCaptureKit to capture a display, a window, or a region on one display, with optional microphone audio. It saves the original video and timestamped cursor path. React shows a floating timer; the menu bar shows the same elapsed time and can stop capture. Permission denial, disconnected devices, and interrupted capture are handled while keeping recoverable recordings. See [Apple’s capture example](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos?language=objc) and [region capture configuration](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/sourcerect?language=objc).
2. **Non-destructive editing:** SQLite stores the project catalog and edit document; the original is always kept. The edit document contains trim points, crop, zoom, cursor highlight, text callouts, and export preset. Automatic framing uses the cursor path to create bounded, smoothly transitioning keyframes. If the path is unavailable, framing stays centered and the UI explains why.
3. **Local export:** The Rust backend validates edit parameters, builds ffmpeg filters and safe temporary files, then atomically writes the finished output. Presets include H.264 MP4 (`libx264`, AAC) and VP9 WebM (`libvpx-vp9`, Opus). Each export also gets a poster frame, transcript Markdown, and summary Markdown. Text callouts use an escaped subtitle file instead of interpolating user input into a command. See the [ffmpeg filter documentation](https://www.ffmpeg.org/ffmpeg-filters.html) for scaling and subtitle rendering.
4. **Transcription and file actions:** When transcription is first requested, Screenbox downloads a multilingual `whisper.cpp` model, shows progress, and caches it for offline inference. The summary extracts timestamped points from the transcript. Recordings without microphone audio still get a “No speech” transcript and a recording-information summary. Exports default to `~/Movies/Screenbox/Exports/`; choose another folder, copy the file path, or reveal it in Finder. See [whisper.cpp](https://github.com/ggml-org/whisper.cpp).
5. **Architecture and local data:** `native/Capture.swift` uses ScreenCaptureKit and AVAssetWriter to write the original and cursor path; the Tauri Rust backend controls it. `src-tauri/src/` owns the SQLite catalog, preferences, validation, recording lifecycle, ffmpeg rendering, transcription, and Finder/clipboard actions; `src/` is the React/TypeScript interface. App data lives in `~/Library/Application Support/com.morso.screenbox/`, including `screenbox.sqlite3` (projects, edits, and preferences), `recordings/<id>/original.mp4`, cursor tracks, cached models, and temporary renders. To back up, quit Screenbox and copy this directory plus every export folder you use. To restore, copy them back before launching the app. Recordings, transcripts, and exports stay local; the app does not upload videos, text, or usage data.

## Tests

```sh
npm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:e2e
npm run build:web
```
