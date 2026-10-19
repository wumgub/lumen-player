# Lumen 🎧

An Electron music player with reactive audio visualizers, a scanned SQLite library,
and four swappable themes.

## Features

- **Full library** — point it at folders, it recursively scans and indexes your audio
  (MP3, FLAC, WAV/AIFF, M4A/AAC, OGG/Opus) into SQLite, reading ID3 tags + embedded art.
- **Browse** by Songs, Artists, Albums, or custom Playlists, with instant search.
- **4 visualizers** — Spectrum Bars, Waveform, Radial, and a reactive Particle field,
  driven by a Web Audio `AnalyserNode`. Toggle between a docked panel and an immersive
  full-window mode (press `f`, or the expand icon).
- **4 themes** — Neon, Minimal, Retro, and Glass (which tints its accents from the
  current album art). Switch live from the top-right selector.
- Gapless-ish queue with shuffle / repeat-all / repeat-one, seeking, volume, and
  keyboard shortcuts (`Space` play/pause, `⌘←/⌘→` prev/next, `f` immersive, `Esc` exit).

## Architecture

| Layer | Responsibility |
|-------|----------------|
| `src/main` | App lifecycle, SQLite library (`better-sqlite3`), recursive scanner (`music-metadata`), native folder picker, and two custom protocols: `lumen-media://` (Range-aware audio streaming) and `lumen-art://` (album art). |
| `src/preload` | Locked-down `contextBridge` API (`window.lumen`). Context isolation on, no Node in the renderer. |
| `src/renderer` | React UI: library views, now-playing, transport, theme engine, and the canvas visualizer host. |
| `src/shared` | Types shared across processes. |

## Install

> **Heads up:** this machine runs **Node 26**, which is newer than the prebuilt binaries
> shipped by `better-sqlite3`, and Electron's own installer mis-extracted its binary here.
> The two-step install below sidesteps both. On a machine with an LTS Node, a plain
> `npm install` should just work.

```bash
# 1. Install JS deps without running native build scripts (which target the wrong ABI)
npm install --ignore-scripts

# 2. Build the native module (better-sqlite3) against Electron's ABI
npm run rebuild

# 3. If `npm run dev` complains "Electron uninstall", the electron binary didn't extract.
#    Extract it manually from the download cache:
unzip -q ~/Library/Caches/electron/*/electron-v*-darwin-arm64.zip -d node_modules/electron/dist
printf 'Electron.app/Contents/MacOS/Electron' > node_modules/electron/path.txt
```

## Run

```bash
npm run dev          # dev with HMR
npm run build        # bundle main/preload/renderer into out/
npm run pack:mac     # package a .dmg/.zip (macOS)
npm run pack:linux   # package an AppImage/.deb (Linux)
```

Launch it, click **＋** next to *Folders* in the sidebar, pick your music directory,
and it'll scan. Double-click any track to play.

## License

[GPL-3.0-or-later](./LICENSE) © wumgub
