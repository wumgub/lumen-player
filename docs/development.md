# Development guide

How Lumen is put together, for anyone who wants to change it. Setup steps (Node version,
compiler toolchain, the native-module workaround) are in the
[main README](../README.md#build-it-yourself).

## Stack

- **Electron 33** with **electron-vite** for bundling and dev hot reload
- **React 18** and TypeScript for the UI, with plain CSS (no framework)
- **better-sqlite3** for the library database (a native module, rebuilt against Electron's ABI
  on install)
- **music-metadata** for reading tags, durations and embedded art
- **@xhayper/discord-rpc** for Discord Rich Presence
- **electron-builder** for packaging

## Layout

```
src/
  main/              Electron main process (Node)
    index.ts         app lifecycle, window, custom protocols, every IPC handler
    db.ts            SQLite schema, migrations and all queries
    scanner.ts       recursive folder walk + tag parsing
    presence.ts      Discord RPC connection, reconnect, activity
    coverart.ts      iTunes Search lookup for presence cover art
  preload/
    index.ts         contextBridge: exposes window.lumen to the renderer
  shared/
    types.ts         types shared by main and renderer, plus the extension lists
  renderer/
    index.html
    src/
      App.tsx        shell: layout, keyboard shortcuts, immersive + video-fill overlays
      audio/AudioEngine.ts       the single <video> element + Web Audio graph
      context/PlayerContext.tsx  queue, transport, stats accrual, media session
      context/ThemeContext.tsx   theme state, custom tokens, album-art palette extraction
      theme/tokens.ts            editable tokens, base presets, theme import/export
      visualizers/               bars, waveform, radial (+ shared types)
      components/                everything else in the UI
      styles/app.css, themes.css
build/               icons and the Discord asset (used by electron-builder)
scripts/screenshot/  README screenshot harness
docs/                you are here
```

## Process model

The renderer has **no Node access** (`contextIsolation: true`, `nodeIntegration: false`). It
talks to the main process only through `window.lumen`, defined in `src/preload/index.ts`. Each
method there is a thin `ipcRenderer.invoke` onto a handler registered in `registerIpc()` in
`src/main/index.ts`.

To add a capability:

1. write the function in `db.ts` (or wherever it belongs) in main
2. register an `ipcMain.handle('area:name', …)` in `registerIpc()`
3. expose it in `src/preload/index.ts`
4. add it to the mock in `scripts/screenshot/mock-preload.cjs`, or the screenshot harness
   breaks (React crashes on the missing method and you get blank captures)

### IPC channels

| Area | Channels |
|---|---|
| Dialogs | `dialog:pickFolder` |
| Library | `library:folders` `library:addFolder` `library:removeFolder` `library:scan` (+ `library:scanProgress` events to the renderer) `library:query` `library:artists` `library:albums` `library:stats` |
| Tracks | `track:get` `track:reveal` |
| Edits | `meta:renameTrack` `meta:renameArtist` `meta:renameAlbum` `meta:setTrackAlbum` `meta:setTrackNote` |
| Albums | `album:reorder` `album:coverFromTrack` `album:coverCustom` `album:clearCover` `album:hasCover` |
| Playlists | `playlist:list` `playlist:create` `playlist:delete` `playlist:rename` `playlist:reorder` `playlist:add` `playlist:remove` `playlist:tracks` |
| Stats | `stats:recordPlay` `stats:history` `stats:clear` |
| Settings | `settings:get` `settings:set` (also reconfigures presence) |
| Presence | `presence:set` |
| App | `app:info` `app:checkForUpdate` |

## Custom protocols

The renderer never sees raw file paths for playback. Two privileged schemes are registered
before the app is ready:

- **`lumen-media://t/<trackId>`** streams the file with HTTP **Range** support, which seeking
  needs. Out-of-bounds ranges (the media element probing past EOF) get a clean `416`. Before
  0.2.3 those built a broken stream that left tracks stuck on their last frame.
- **`lumen-art://t/<trackId>`** serves a track's embedded art (cached for a day).
  **`lumen-art://album/<name>`** serves an album's resolved cover with `no-store`, because it
  can change under the same URL. The renderer also remounts the `<img>` on a version bump.

## The database

One SQLite file, `library.db`, in `app.getPath('userData')`, in WAL mode with foreign keys on.
The schema is created in `initDb()`, and new columns are added by small `PRAGMA table_info`
migrations underneath it. There's no migration framework; just add another
`if (!cols.some(...)) ALTER TABLE ...`.

| Table | Holds |
|---|---|
| `tracks` | one row per file: `path` (unique), tags, `duration`, `hasArt`, `addedAt`, plus `locked`, `albumPos`, `note`, `isVideo` |
| `art` | embedded art blobs, one per track |
| `album_covers` | per-album cover overrides, keyed by album **name**: `source = 'track'` (points at another track) or `'custom'` (stores the image) |
| `playlists` | `name`, `createdAt`, `position` (sidebar order) |
| `playlist_tracks` | `(playlist_id, track_id)` primary key, `position` |
| `folders` | scanned folder paths |
| `settings` | key/value strings: `statsEnabled`, `discord*` |
| `plays` | listening log: `track_id`, `played_at`, `seconds`, `counted` |

### Edits and the lock bitmask

User edits are written straight into `tracks` and protected by `locked`, a bitmask:

| Bit | Field |
|---|---|
| `1` | title |
| `2` | artist / albumArtist |
| `4` | album |

`upsertTrack()` keeps the stored value for any locked field and refreshes everything else from
the tags. `note` and `albumPos` are never written by the scanner, so they don't need bits.

### Grouping rules

- "Effective artist" = `albumArtist` if non-empty, else `artist`. This is used by
  `listArtists()`, the artist filter in `queryTracks()`, and `renameArtist()`.
- Albums group on `album` alone, and empty albums are excluded from `listAlbums()`.
- Search is `LIKE '%term%'` across title, artist, album and note. SQLite's `LIKE` only folds
  ASCII case.

### Scanning

`scanFolders()` walks every folder (skipping dot-entries and unreadable directories), parses
each file with music-metadata, and upserts it. Unparseable files are still indexed with
filename-only data. Afterwards it prunes rows whose files no longer exist, keeping any path
that still exists on disk even if it wasn't under a scanned folder. Pruning is skipped entirely
when a scan finds zero files, so an unmounted drive can't wipe the library.

## Playback

`AudioEngine` owns **one `<video>` element** for everything (audio-only tracks just don't show
it), wired into Web Audio as `MediaElementSource → Analyser (fft 2048) → Gain → destination`.
The gain node is what allows volume up to 1.5. The graph is created lazily on the first
`play()`, because an AudioContext needs a user gesture.

The element is kept outside React. `VideoSurface` re-parents it into whichever container
should show it. On unmount it moves the element into a hidden "park" div rather than removing
it, because taking a playing media element out of the document pauses it. **Only one
VideoSurface may be mounted at a time.** The Now Playing panel hides its own while the
full-window video is open.

Auto-advance hangs off `ended`, which occasionally never fires. A watchdog arms on
`timeupdate`/`waiting`/`stalled` near the end, and if playback is still parked within 0.5 s of the
end 1.5 s later, it emits `ended` itself.

`PlayerContext` holds the queue, index, shuffle and repeat. It mirrors them into a ref so the
engine callbacks (registered once) always see current state. It also accrues listening time
for stats: forward progress under 2 s per tick counts, a jump back to near 0 banks the pass,
and a listen is flushed on track change if it's at least 5 s. It's `counted` once it reaches
`min(duration / 2, 240 s)`. It also drives `navigator.mediaSession` for media keys and the OS
widget.

## Visualizers

Each visualizer is a factory returning `{ id, label, draw(ctx, width, height, frame, colors, t), reset? }`
(see `visualizers/types.ts`). `VisualizerHost` runs the `requestAnimationFrame` loop, handles
DPR (capped at 2) and resizing, reads theme colours from CSS variables every 300 ms, and
throttles (user cap while playing, 15 fps paused, 4 fps hidden). To add one, write a factory
and add it to `VISUALIZER_FACTORIES`, `VISUALIZER_LIST` and `isVisualizerId` in
`visualizers/index.ts`.

Setting `window.__lumenVizDemo = true` feeds a synthetic animated frame instead of real
audio. The screenshot harness uses it.

## Themes

Built-in themes are CSS blocks on `:root[data-theme='…']` in `styles/themes.css`. The Custom
theme sets the same variables inline on `<html>` from a token map (`theme/tokens.ts`), plus a
few derived ones (`--accent-soft`, `--on-accent` for contrast, `color-scheme`). Glass reads
`--dynamic-accent` / `--dynamic-accent-2`, which `ThemeContext` sets by sampling the current
art at 48×48 and bucketing pixels by hue.

To add a token: add it to `THEME_TOKENS` and every entry of `BASE_THEMES`, and define it in
each built-in theme in `themes.css`.

## Discord presence

`PresenceManager` (`presence.ts`) connects lazily over the local Discord IPC socket. It sends a
`LISTENING` activity with start/end timestamps, and clears it when there's no track. The
renderer only calls `presence:set` on track, play-state and duration changes, never per second.

If the connection drops for a reason other than Lumen closing it (`intentionalClose`), or a
connect attempt fails, `scheduleReconnect()` arms a single timer with exponential backoff from
5 s to 60 s. A successful `ready` resets it.

Per-track cover art goes: iTunes Search lookup (`coverart.ts`) → RPC `AUTHORIZE` → OAuth token
exchange with the client secret → `POST /applications/<id>/external-assets` → an `mp:external/…`
image key. Everything falls back to the `lumen` asset on failure. The settings UI doesn't
currently expose `discordClientSecret`, so this path only runs if it's set in the DB.

## Build stamping and update check

`electron.vite.config.ts` bakes the short git SHA and build time into main as
`__APP_COMMIT__` / `__APP_BUILD_DATE__`. **Check for updates** compares that SHA to the head of
`main` through the GitHub API.

## Scripts

| Command | Does |
|---|---|
| `npm run dev` | Dev mode with hot reload (user data folder: `lumen-player`) |
| `npm run build` | Bundle main, preload and renderer into `out/` |
| `npm start` | Run the built bundle |
| `npm run typecheck` | `tsc` over the node and web projects |
| `npm run rebuild` | Rebuild native modules for Electron |
| `npm run pack:mac` / `pack:win` / `pack:linux` | Build and package installers into `dist/` (current OS only) |

## Releasing

1. Bump the version in `package.json` and commit.
2. Tag it `vX.Y.Z` and push the tag (`npm version patch && git push --follow-tags` does both).
3. `.github/workflows/release.yml` builds on macOS (arm64), Windows and Linux in parallel. Each
   job runs `electron-builder --publish always`, which uploads into a **draft** release for the
   tag. A final `tidy-release` job deletes the `.blockmap` and `latest*.yml` auto-update files,
   since nothing uses them.
4. Open the draft on the Releases page, write the notes and publish it. Or:
   `gh release edit vX.Y.Z --draft=false --latest --notes "…"`.

Builds aren't code-signed (`CSC_IDENTITY_AUTO_DISCOVERY=false`; macOS gets an ad-hoc signature),
hence the first-launch prompts users see. Running the workflow by hand from the Actions tab
packages everything without publishing, which is useful as a smoke test.

## Screenshots

The README images are captured from the real built renderer against fake data:

```bash
npm run build
npx electron scripts/screenshot/capture.cjs
```

`capture.cjs` loads `out/renderer/index.html` in a hidden window with `mock-preload.cjs` as
its preload. The mock is a fake `window.lumen` with a small Kino / Molchat Doma library. It also
serves generated gradient SVGs for `lumen-art://`, turns on the visualizer demo mode, and
writes `library.png`, `albums.png` and `album.png` into `docs/screenshots/`. To change what's
shown, edit the `songs` list or the `shot(...)` calls.
