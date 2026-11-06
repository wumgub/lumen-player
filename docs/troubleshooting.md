# Troubleshooting

## Installing and opening

**macOS says Lumen "can't be opened", "Apple could not verify…", or is "damaged".**
Lumen isn't signed with a paid Apple certificate, so you need to allow it once. The
[main README](../README.md#install) has step-by-step instructions, including the
`xattr -cr /Applications/Lumen.app` fix for "damaged".

**Windows says "Windows protected your PC".**
Click **More info → Run anyway**. You only need to do this once.

**The Linux AppImage won't start.**
Make it executable (`chmod +x Lumen-*.AppImage`) and install FUSE 2
(`sudo apt install libfuse2` / `sudo pacman -S fuse2`).

**There's no Mac download for my Intel Mac.**
The release is Apple silicon only. Intel Macs need to
[build from source](../README.md#build-it-yourself).

## Library and scanning

**Rescan library is greyed out.**
There are no folders yet. Add one with the **＋** next to *Folders* first.

**I added music but it isn't showing up.**
Lumen doesn't watch for new files. Click **Rescan library**. Also check that:
- the files are inside one of the folders listed in the sidebar (subfolders count)
- they have a [supported extension](library.md#supported-formats)
- neither the files nor a folder above them starts with a `.`

**Songs from a cloud folder (OneDrive, iCloud…) are missing, slow to scan, or won't play.**
They're probably online-only placeholders. Set the folder to always stay on the device and let
it download fully, then rescan ([more](library.md#cloud-folders-onedrive-icloud-drive-dropbox)).

**A song is listed but plays silence, or won't play.**
- The file may have moved or been deleted since the last scan. Rescan.
- The codec inside might not be playable (ALAC, ProRes, HEVC…), even if the extension is
  supported. See [formats](library.md#supported-formats).

**Everything is "Unknown Artist", or the titles are filenames.**
The files have no tags. Fix them in a tag editor (Mp3tag, MusicBrainz Picard, Kid3…) and
rescan, or edit them in Lumen ([Editing](editing.md)).

**One album shows up as two.**
The songs have slightly different album names (`Этажи` vs `Etazhi`, a trailing space, different
capitals). Rename one to match the other exactly and they'll merge
([Renaming](editing.md#renaming)).

**Two different albums have been merged into one.**
They have the same album name. Rename them apart, for example *Greatest Hits (Kino)* and
*Greatest Hits (ABBA)*. Note that renaming changes every song on the album, so first move the
songs from one artist onto a new album with **Add to album…**.

**Songs from a folder I removed are still there.**
Removing a folder only stops future scans. The songs stay while their files exist
([more](library.md#removing-music)).

**My edits, notes or play counts disappeared for some songs.**
The files were probably moved or renamed on disk, so Lumen treats them as new songs
([more](library.md#moving-and-renaming-files)).

## Search

**Searching doesn't find a song I know is there.**
Search looks at the title, artist, album and note only, not the file name or the folder. If the
tags are odd, add a [note](browsing.md#notes) with the words you'd search for.

## Playback

**A track sits at the end and doesn't move on to the next one.**
This was fixed in 0.2.3. Update to the latest release.

**Next/previous keyboard shortcuts don't work on Windows/Linux.**
Use **Ctrl ←/→** (before 0.2.6 they used the Windows key, which the system grabs).

**The volume is way too loud or distorted.**
The volume slider goes up to 150%. Anything past about two-thirds of the slider is a boost.

**The music video only shows the cover.**
Switch the **Art | Video** toggle under the picture to *Video*. If it's on Video and still shows
nothing, the video codec probably isn't supported ([formats](music-videos.md#formats-that-work)).

## Visualizer

**The visualizer is using a lot of CPU or battery.**
Cap it at 30 or 15 fps under **⚙ Settings → Appearance**, or turn it off with the switch in its
top-left corner ([more](visualizers.md#frame-rate-and-cpu-use)).

**The visualizer is flat.**
It only moves while something is actually playing. If it's still flat during playback, check
it isn't switched off (the switch in the top-left corner) and that the volume isn't at 0.

## Discord

**Presence isn't showing.**
Go down this list:
1. The **desktop** Discord app is running and signed in (not the browser version).
2. Your status isn't **Invisible**.
3. **User Settings → Activity Privacy → Share your detected activities** is on.
4. In Lumen, presence is switched on, the Application ID is pasted, and you clicked **Save**.
5. Check from a friend's view. You often can't see your own activity.

**The progress bar on Discord is wrong after skipping around in a song, or runs past the end on
repeat.**
Fixed in 0.2.6. Presence is now re-sent after every seek and repeat-one loop.

**It worked, then stopped after Discord restarted.**
Lumen reconnects by itself within about a minute. On versions older than 0.2.4 it didn't, so
update, or restart Lumen.

**The picture next to my presence is blank.**
Upload an image called exactly `lumen` under **Rich Presence → Art Assets** in the developer
portal, and give it a few minutes ([setup](discord.md#2-add-the-artwork-recommended)).

## Settings

**"Check for updates" says there's an update, but I have the newest release.**
Versions before 0.2.6 compared commits rather than releases, so docs changes set it off.
Update once and it'll compare against releases from then on.

## Running from source

**`npm run dev` opens a blank window.**
Usually the native database module didn't build for Electron. Look for `better-sqlite3` or
`NODE_MODULE_VERSION` in the terminal and run `npm run rebuild`. On a Node newer than 20, use the
[two-step install](../README.md#2-install-dependencies). DevTools (⌘⌥I / Ctrl+Shift+I) shows
renderer errors.

**"Electron uninstall" error.**
The Electron binary didn't unpack. The [README](../README.md#2-install-dependencies) has the
manual fix.

## Still stuck?

[Open an issue](https://github.com/wumgub/lumen-player/issues) with:
- your OS and Lumen version (**⚙ Settings → About**)
- what you did, what you expected, and what happened instead
- if you can, the terminal output. Start Lumen from a terminal to see it: on macOS run
  `/Applications/Lumen.app/Contents/MacOS/Lumen`
