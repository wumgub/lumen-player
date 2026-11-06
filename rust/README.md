# Lumen, native

This is the start of Lumen rewritten in Rust as a native desktop app: no
Electron, no web views. The Electron app in the rest of this repo is still the
one that ships until this one catches up.

- `app/` is the app itself, using [egui](https://github.com/emilk/egui) on OpenGL.
- `crates/lumen-core` is the library: the same `library.db` as the Electron app
  (opened in place, so nothing needs migrating), folder scanning, search, stats
  and themes.
- `crates/lumen-playback` plays audio and video with libmpv and runs the
  analyzer the visualizers draw from.

## Building

You need Rust and libmpv. On macOS:

```bash
brew install mpv dylibbundler
cargo run --release
```

On Linux, install your distro's `libmpv-dev` (or `mpv-libs-devel`) instead. On
Windows, point `MPV_LIB_DIR` at a folder with `mpv.lib` from an mpv dev build.

`scripts/bundle-macos.sh` builds `Lumen.app` with libmpv and everything it needs
copied inside, so it runs on a Mac without Homebrew.
