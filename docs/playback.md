# Playback and the queue

## The transport bar

Along the bottom of the window, from left to right:

- **What's playing**: art, title, and "artist — album".
- **⤨ Shuffle** (lit when on)
- **⏮ Previous**
- **⏯ Play / Pause**
- **⏭ Next**
- **↻ Repeat**: click to cycle **off → all → one**. It's lit for *all* and shows a little "1" for
  *one*.
- **The seek bar**, with elapsed time and length. Drag or click it to jump.
- **☰ Queue**, with a badge showing how many songs are up next
- **🔊 Mute** and the **volume slider**

## How the buttons behave

**Previous**: if you're more than 3 seconds into a song, it goes back to the start of it.
Press it again (within 3 seconds) to go to the song before.

**Next / end of song**:

| Repeat | Shuffle off | Shuffle on |
|---|---|---|
| Off | Plays the next song in the queue. Stops after the last one. | Plays a random song from the queue |
| All | Next song, and loops back to the top after the last one | Random song, forever |
| One | The current song repeats when it ends. Pressing **Next** still moves on. | Same |

Shuffle picks a random song from the queue each time, never the one that just played. It's
not a shuffled order that goes through each song once, so a song can come up again before
others have played. **Previous** also picks a random song while shuffle is on.

Shuffle and repeat reset to off when you restart Lumen.

## Volume

The slider goes past normal, up to **150%**, which helps with quiet recordings. Anything over
100% is a boost and can distort loud tracks. **Mute** sets the volume to 0. Clicking it again
sets it back to 90%, not to where you were before. Your volume is remembered between sessions.

## The queue

The queue is the list of songs Lumen is working through.

- **Clicking a song in any list replaces the queue** with that whole list, starting at the song
  you clicked. **Play all** does the same from the top. Clicking a song in the
  [Stats](stats.md) leaderboard queues just that one song.
- **Play next** (song menu) slots songs in straight after the current one.
- **Add to queue** (song menu, or the bar when several are ticked) adds them to the end.
- If nothing is playing, either of those starts playing the songs immediately.

Click **☰** in the transport bar to open the queue panel:

- Songs already played are dimmed, and the current one is highlighted.
- **Click** any song to jump to it.
- **×** removes a song. Removing the one that's playing skips to whatever was next.
- **Clear upcoming** removes everything after the current song.

Click outside the panel to close it. The queue isn't saved, so it starts empty each time you
open Lumen.

## Media keys and the OS

Lumen works with your keyboard's media keys **⏯ ⏮ ⏭**, headphone buttons, and the system
"Now Playing" controls (Control Centre on macOS, the media overlay on Windows, MPRIS on Linux).
Those show the title, artist, album, art and a scrubber.

## Keyboard shortcuts

See [Keyboard shortcuts](shortcuts.md).
