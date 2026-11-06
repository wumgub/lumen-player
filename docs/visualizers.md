# Visualizers

The top of the Now Playing panel is a live visualizer that reacts to whatever's playing.

## The three styles

Switch between them with the tabs at the bottom of the visualizer:

| Visualizer | Looks like |
|---|---|
| **Spectrum Bars** | 64 glowing bars rising from the bottom, low notes on the left and high on the right, with little caps that fall slowly from each peak |
| **Waveform** | An oscilloscope line across the middle with a faint mirrored echo. It gets taller as the music gets louder. |
| **Radial** | 96 bars around a circle that slowly rotates. Its glowing core pulses with the bass. |

Colours come from your [theme](themes.md). The Glass theme tints them from the current song's
album art. Your pick is remembered.

## Turning it off

The small **switch in the top-left corner** of the visualizer turns it off completely. Nothing is
drawn, which saves CPU and battery. Flip it back to turn it on. This is remembered too.

## Immersive mode

Click the **expand** icon in the visualizer's top-right corner, or press **f**, to blow the
visualizer up to fill the whole window. Along the bottom you get the song title and artist,
previous / play-pause / next, and the visualizer tabs. There's also the on/off switch in the
corner. **Esc**, **f** again, or the collapse button takes you back.

(If a [music video](music-videos.md) is showing, **f** fills the window with the video instead.)

## Frame rate and CPU use

By default the visualizer redraws as often as your screen refreshes, which can be 120 times a
second on a ProMotion Mac. To lighten it, go to **⚙ Settings → Appearance → Visualizer frame
rate** and choose **30 fps** or **15 fps**. It takes effect immediately.

Lumen also slows it down automatically, whatever the setting:

| Situation | Redraws per second |
|---|---|
| Playing, window visible | your setting (Full / 30 / 15) |
| Paused | 15 |
| Window hidden or minimised | 4 |
| Switched off | 0 |
