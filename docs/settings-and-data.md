# Settings and your data

## The Settings window

Open it with **⚙** in the top-right corner.

| Section | Setting | Saved when |
|---|---|---|
| **Appearance** | Theme chips, including the ✏️ on Custom for the [theme editor](themes.md#the-theme-editor) | instantly |
| | **Visualizer frame rate**: Full / 30 fps / 15 fps ([details](visualizers.md#frame-rate-and-cpu-use)) | instantly |
| **Listening stats** | Record listening on/off, and **Clear history** ([details](stats.md#privacy-and-clearing-history)) | instantly |
| **Discord Rich Presence** | On/off and Application Client ID ([details](discord.md)) | when you click **Save** |
| **About** | Version, the build (git commit) and its date, and **Check for updates** | n/a |

**Cancel** (or clicking outside the window) only throws away unsaved Discord changes. The rest
has already been applied.

### Check for updates

This compares your version number with the newest release on GitHub. Lumen doesn't update
itself. If there's a newer version, the link takes you to its release page to download the
installer (or pull and rebuild if you run from source). The build line under About still shows
the exact commit your copy was built from, which is useful in bug reports.

## Where Lumen keeps things

Everything lives in one folder per user:

| OS | Folder |
|---|---|
| macOS | `~/Library/Application Support/Lumen` |
| Windows | `%APPDATA%\Lumen` |
| Linux | `~/.config/Lumen` |

When running from source (`npm run dev`) the folder is called `lumen-player` instead, so a dev
copy and an installed copy don't share a library.

Inside it:

| What | Where |
|---|---|
| The library: songs, tags, embedded cover art, your edits, notes, playlists, custom album covers, listening history, folder list, Discord settings | `library.db` (plus `library.db-wal` and `library.db-shm` while Lumen is open) |
| Look and feel: theme, custom theme, album-art tinting, visualizer choice/on-off/frame rate, Art/Video choice, volume | Chromium's local storage, under `Local Storage/` |

`library.db` includes a copy of every song's embedded cover art, so with a big library it can
reach a few hundred MB. That's normal.

## Backing up

Quit Lumen, then copy the whole folder above somewhere safe. To restore, quit Lumen and put the
folder back.

Your music files aren't in there, just Lumen's index of them. A restored library only works if
the music is at the same paths as before, because songs are matched by their full file path.

## Moving to a new computer

Copy the folder across as above. If your music ends up at a different path on the new machine
(a different username, a different drive letter), the old entries won't match. On the first
rescan, Lumen removes the old entries and re-adds the music as new songs, so edits, notes,
playlists and stats tied to the old paths are lost. Keeping the same path avoids that.

## Starting over

To wipe the library completely: quit Lumen, then delete `library.db`, `library.db-wal` and
`library.db-shm` from the folder above. Next launch starts empty. Add your folders again.

That removes **everything** in the first row of the table above: edits, notes, playlists,
covers, stats, folders and Discord settings. Your themes and other look-and-feel settings stay,
because they're in local storage. Delete the whole folder to reset those too.
