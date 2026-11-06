# Your library

Lumen doesn't copy or move your music. You tell it which folders to look in, it reads the
files there, and it keeps an index (song names, tags, cover art, your edits) in a small
database. The files themselves are never changed.

## Folders

The **Folders** section of the sidebar lists every folder Lumen scans.

- **Add a folder**: click **＋** next to *Folders*. You can pick several at once. Adding a folder
  starts a scan right away.
- **Browse a folder**: click it in the sidebar to see just the songs inside it (and its
  subfolders).
- **Remove a folder**: hover it and click **×**. This only stops Lumen scanning it in future.
  **Songs already in your library from that folder stay there**; see
  [removing music](#removing-music) below.

Subfolders are always included, as deep as they go. Files and folders whose names start with a
dot (hidden ones like `.Trash`) are skipped. So are folders Lumen doesn't have permission to read.

## Supported formats

| Kind | Extensions |
|---|---|
| Audio | `.mp3` `.flac` `.wav` `.aif` `.aiff` `.m4a` `.aac` `.ogg` `.oga` `.opus` |
| Video ([music videos](music-videos.md)) | `.mp4` `.m4v` `.webm` `.mov` |

Anything else (`.wma`, `.ape`, DSD, `.mkv` and so on) is ignored. Whether a file actually plays
depends on the codec inside it. Lumen plays through Chromium, so the usual web codecs work: MP3,
AAC, FLAC, Vorbis, Opus, PCM WAV/AIFF, and H.264/VP8/VP9/AV1 video. Something like an `.m4a`
holding ALAC, or a `.mov` holding ProRes, gets indexed but may not play.

## How tags are read

When Lumen scans a file, it reads its tags (ID3, Vorbis comments, MP4 atoms and so on):

| Lumen shows | Comes from | If the tag is missing |
|---|---|---|
| Title | title tag | the filename, without the extension |
| Artist | artist tag (or the first of several artists) | `Unknown Artist` |
| Album artist | album artist tag | empty, so the artist is used instead |
| Album | album tag | empty, so the song isn't on any album |
| Genre | first genre tag | empty |
| Year, track number, disc number | the matching tags | empty |
| Duration | the audio stream itself | 0 |
| Cover art | the first picture embedded in the file | no art, so a placeholder is shown |

A few things worth knowing:

- **Artists are grouped by album artist when there is one.** A compilation tagged with
  album artist "Various Artists" appears once under that name in *Artists*, not split across
  every guest artist. With no album artist tag, the song's own artist is used.
- **Albums are grouped by name alone.** Two different albums both called *Greatest Hits*
  end up merged into one album. Rename one of them to keep them apart (see
  [Editing](editing.md#renaming)).
- **Songs with no album tag don't appear under *Albums*** at all. They're still in *Songs*,
  *Artists* and their folder. You can [put them on an album yourself](editing.md#making-an-album).
- **Only embedded art is used.** A `cover.jpg` sitting next to the files isn't picked up
  automatically. You can set it as the album's cover by hand, though
  ([Album covers](editing.md#album-covers)).
- If a file can't be read at all (corrupt, or a format the tag reader doesn't understand), it's
  still added using its filename, with no tags and a duration of 0, so you can see it's there.

## What a rescan does

Lumen doesn't watch your folders for changes. After adding, removing or retagging files, click
**Rescan library** at the bottom of the sidebar. The button is greyed out until at least one
folder has been added.

A rescan:

1. **Finds every media file** in all your folders.
2. **Adds new files** to the library.
3. **Re-reads existing files**, picking up any tag or cover art changes, **except for fields
   you've edited in Lumen**. Your renames and album assignments are locked, so the file's tags
   don't overwrite them ([details](editing.md#what-survives-a-rescan)).
4. **Removes songs whose files no longer exist** on disk.

There's a safety catch on step 4. If a scan finds *no files at all*, for example because a folder
is on an external drive that's unplugged, nothing is removed. A temporarily missing drive won't
empty your library.

## Removing music

There's no "delete from library" button. Lumen mirrors what's on disk:

- To drop songs, delete or move the files out of your folders, then rescan.
- Removing a folder from the sidebar doesn't remove its songs, because the files still
  exist. Delete the files, or quit Lumen and [start the library over](settings-and-data.md#starting-over).

## Moving and renaming files

Lumen tracks songs by their full file path. If you move or rename a file, the next rescan sees
it as a brand-new song and removes the old entry. That means anything attached to the old entry
is lost:

- renames and album assignments made in Lumen
- notes
- playlist membership
- play counts in [Stats](stats.md)

Organise your files first and do your Lumen edits afterwards.

## Cloud folders (OneDrive, iCloud Drive, Dropbox…)

These services often keep files as online-only placeholders until they're opened. Lumen has to
read every file to scan it and to play it, so placeholders make scans slow, fail, or come back
with missing songs. Set the music folder to always stay on the device (OneDrive: right-click →
**Always keep on this device**; iCloud: **Keep Downloaded**), let it finish downloading, then
add or rescan it.
