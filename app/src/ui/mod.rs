//! The window's layout, as in App.tsx: title bar on top, transport at the
//! bottom, sidebar on the left, now playing on the right and the library in
//! the middle, with menus, dialogs and the full-screen modes over the top.

pub mod icon;
pub mod icons;
pub mod widgets;

mod content;
mod nowplaying;
mod overlays;
mod settings;
mod sidebar;
mod stats;
pub mod theme_editor;
mod titlebar;
mod transport;

use eframe::egui::{self, Color32, Frame, Margin, Mesh, Rect, Shape, Stroke, pos2};

use crate::app::LumenApp;

pub const TITLEBAR_H: f32 = 52.0;
pub const TRANSPORT_H: f32 = 84.0;
pub const SIDEBAR_W: f32 = 240.0;
pub const NOWPLAYING_W: f32 = 300.0;

pub fn shell(app: &mut LumenApp, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
    let p = app.p;
    backdrop(app, ui);

    if app.video_full {
        overlays::video_full(app, ui, frame);
        overlays::show(app, ui, frame);
        return;
    }
    if app.immersive {
        overlays::immersive(app, ui);
        overlays::show(app, ui, frame);
        return;
    }

    let bar = |fill: Color32| Frame::new().fill(fill).inner_margin(Margin::ZERO);
    egui::Panel::top("titlebar")
        .exact_size(TITLEBAR_H)
        .resizable(false)
        .show_separator_line(false)
        .frame(bar(p.bg2))
        .show(ui, |ui| {
            titlebar::show(app, ui);
            let r = ui.max_rect();
            ui.painter().hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, p.border));
        });
    egui::Panel::bottom("transport")
        .exact_size(TRANSPORT_H)
        .resizable(false)
        .show_separator_line(false)
        .frame(bar(p.bg2))
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().hline(r.x_range(), r.top() + 0.5, Stroke::new(1.0, p.border));
            transport::show(app, ui);
        });
    egui::Panel::left("sidebar")
        .exact_size(SIDEBAR_W)
        .resizable(false)
        .show_separator_line(false)
        .frame(bar(p.bg2))
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().vline(r.right() - 0.5, r.y_range(), Stroke::new(1.0, p.border));
            sidebar::show(app, ui);
        });
    egui::Panel::right("now-playing")
        .exact_size(NOWPLAYING_W)
        .resizable(false)
        .show_separator_line(false)
        .frame(bar(p.bg2))
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().vline(r.left() + 0.5, r.y_range(), Stroke::new(1.0, p.border));
            nowplaying::show(app, ui, frame);
        });
    egui::CentralPanel::default().frame(Frame::new().inner_margin(Margin::ZERO)).show(ui, |ui| {
        content::show(app, ui);
    });
    overlays::show(app, ui, frame);
}

/// The window background. Glass gets the playing song's art, blurred, under
/// two soft accent glows (themes.css's radial gradients).
fn backdrop(app: &mut LumenApp, ui: &egui::Ui) {
    let p = app.p;
    let screen = ui.ctx().content_rect();
    let painter = ui.ctx().layer_painter(egui::LayerId::background());
    painter.rect_filled(screen, 0.0, p.bg);
    if !p.glass {
        return;
    }
    if let Some(tex) = app.art.backdrop.as_ref().and_then(|b| b.texture.clone()) {
        // Cover-fit the square blur to the window and dim it so text stays readable.
        let side = screen.width().max(screen.height()) * 1.15;
        let r = Rect::from_center_size(screen.center(), egui::vec2(side, side));
        let mut mesh = Mesh::with_texture(tex.id());
        mesh.add_rect_with_uv(r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::from_white_alpha(150));
        painter.add(Shape::mesh(mesh));
        painter.rect_filled(screen, 0.0, Color32::from_black_alpha(120));
    }
    glow(&painter, pos2(screen.left() + screen.width() * 0.8, screen.top() - screen.height() * 0.1), 700.0, crate::palette::with_alpha(p.accent, 0.30));
    glow(&painter, pos2(screen.left() - screen.width() * 0.1, screen.bottom() + screen.height() * 0.1), 520.0, crate::palette::with_alpha(p.accent2, 0.24));
}

fn glow(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    const N: u32 = 48;
    for k in 0..=N {
        let a = k as f32 / N as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(center + egui::vec2(a.cos(), a.sin()) * radius, Color32::TRANSPARENT);
        if k > 0 {
            mesh.add_triangle(0, k, k + 1);
        }
    }
    painter.add(Shape::mesh(mesh));
}

pub fn format_time(s: f64) -> String {
    if !s.is_finite() || s < 0.0 {
        return "0:00".into();
    }
    let s = s as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// "5h 27m", like format.ts's formatDuration.
pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0m".into();
    }
    let total = seconds.round() as u64;
    let (d, h, m, s) = (total / 86400, (total % 86400) / 3600, (total % 3600) / 60, total % 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// "1,234 songs".
pub fn pluralize(n: i64, word: &str) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        out.insert(0, '-');
    }
    format!("{out} {word}{}", if n == 1 { "" } else { "s" })
}
