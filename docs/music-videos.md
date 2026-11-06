# Music videos

Video files (`.mp4`, `.m4v`, `.webm`, `.mov`) in your folders are added to the library like any
song. They have a title, artist and album from their tags, they go in playlists and the queue,
and they count in stats. You can just listen to them.

## Watching

While a video track plays, the Now Playing panel on the right shows an **Art | Video** switch
under the picture:

- **Video** shows the video itself in the panel.
- **Art** shows the cover art instead. The sound carries on without a hiccup either way.

Your choice is remembered and applies to every video track. The visualizer keeps working while
a video plays.

## Filling the window

To make the video fill the whole Lumen window, do any of these:

- double-click the video
- hover it and click the **expand** icon in its top-right corner
- press **f**

This fills the app window, but doesn't switch your screen into fullscreen mode. To get the
video covering the entire screen, make the Lumen window itself fullscreen first (the green
button on macOS, or maximise it), then fill it.

While it's filling the window:

- moving the mouse shows the title, artist, previous / play-pause / next and a seek bar
- leave the mouse still for about 2.5 seconds and the controls and pointer fade away
- **Esc**, **f**, double-clicking, or the collapse button in the top-right corner takes you back

If the next song in the queue isn't a video, Lumen leaves the full-window view by itself.

## Formats that work

Lumen plays video through Chromium, so the picture needs to be **H.264, VP8, VP9 or AV1**, with
**AAC, MP3, Opus or Vorbis** audio. That covers nearly every `.mp4` and `.webm` you'll find. A
`.mov` with ProRes or HEVC video may play sound only, or not at all. If a video won't play,
converting it to H.264/AAC `.mp4` fixes it:

```bash
ffmpeg -i input.mov -c:v libx264 -c:a aac output.mp4
```

Cover art for videos comes from art embedded in the file's tags, the same as audio. Most videos
don't have any, so they show the placeholder in Art mode. You can still give the album a
[cover](editing.md#album-covers).
