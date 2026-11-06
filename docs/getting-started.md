# Getting started

If you haven't installed Lumen yet, grab it from the
[releases page](https://github.com/wumgub/lumen-player/releases/latest). The
[main README](../README.md#install) walks through the one-time "unknown developer" prompt on
each platform.

## First launch

Lumen opens with an empty library. To fill it:

1. In the left sidebar, find **Folders** and click the **＋** next to it.
2. Pick the folder your music lives in. You can select more than one folder in the dialog.
3. Lumen scans it straight away. A progress bar at the bottom of the sidebar shows
   "Finding files…" and then "Scanning 120/800" while it reads each file.

When it finishes, your songs show up under **Songs**. Click any song to start playing.

Big libraries take a little while the first time, because Lumen reads the tags and cover art
of every file. Later rescans are quicker but still re-read every file (see
[Your library](library.md#what-a-rescan-does)).

## The window

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Lumen                                    🔍   [Theme ▾]   ⚙               │  title bar
├────────────┬────────────────────────────────────────────┬──────────────────┤
│ Songs      │                                            │  visualizer      │
│ Artists    │    the current view                        │                  │
│ Albums     │    (a song list, an album grid, stats…)    ├──────────────────┤
│ Stats      │                                            │  cover / video   │
│            │                                            │  title           │
│ Playlists ＋│                                            │  artist          │
│ Folders   ＋│                                            │  album           │
│            │                                            │                  │
│ [Rescan]   │                                            │                  │
├────────────┴────────────────────────────────────────────┴──────────────────┤
│ art  Title — Artist     ⤨  ⏮  ⏯  ⏭  ↻   0:42 ━━━━━━━━━━ 3:51    ☰  🔊 ━━   │  transport bar
└────────────────────────────────────────────────────────────────────────────┘
```

- **Title bar**: search (the magnifying glass), the theme picker, and ⚙ Settings.
- **Sidebar**: the main views, your playlists, your music folders, and the
  **Rescan library** button. Underneath that is a count of songs and artists.
- **Main area**: whatever view you've picked.
- **Now Playing panel** (right): the visualizer on top, then the current song's cover (or its
  video), title, artist and album.
- **Transport bar** (bottom): what's playing, the playback buttons, the seek bar, the queue
  button, mute and volume.

The window can't be made smaller than 900 × 600.

## Next steps

- Tags missing or wrong? See [Editing your library](editing.md).
- Make some [playlists](playlists.md).
- Pick a [theme](themes.md) or make your own.
- Show what you're playing on [Discord](discord.md).
