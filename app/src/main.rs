#![cfg_attr(windows, windows_subsystem = "windows")]
//! Lumen, native: the music player with the reactive visualizers, without a
//! browser engine inside. See README.md in this folder.

mod app;
mod art;
mod fonts;
mod library;
mod palette;
mod prefs;
mod ui;
mod video;
mod viz;

use std::path::PathBuf;

use eframe::egui;

fn parse_args() -> app::Opts {
    let mut o = app::Opts { frames: 90, ..Default::default() };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--ao" => o.ao = args.next(),
            "--screenshot" => o.screenshot = args.next().map(PathBuf::from),
            "--frames" => o.frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(90),
            "--play" => o.play = args.next(),
            "--seek" => o.seek = args.next().and_then(|v| v.parse().ok()),
            "--view" => o.view = args.next(),
            "--theme" => o.theme = args.next(),
            "--db" => o.db = args.next().map(PathBuf::from),
            "--open" => o.open = args.next(),
            _ => {}
        }
    }
    o
}

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let opts = parse_args();
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../build/icon.png")).ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Lumen")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([980.0, 620.0]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    // On macOS the window buttons sit inside the title bar, like the Electron
    // app's `hiddenInset` style.
    #[cfg(target_os = "macos")]
    {
        viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    let native = eframe::NativeOptions { viewport, renderer: eframe::Renderer::Glow, ..Default::default() };
    eframe::run_native("Lumen", native, Box::new(move |cc| Ok(Box::new(app::LumenApp::new(cc, opts)?))))
}
