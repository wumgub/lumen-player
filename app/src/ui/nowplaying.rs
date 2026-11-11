//! The now-playing panel: the visualizer card with its
//! switch, full-screen button and tabs, then the art (or the music video),
//! and the song's details.

use eframe::egui::{self, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::LumenApp;
use crate::art::ArtKey;
use crate::ui::{icon, icons, widgets};
use crate::viz;

pub fn show(app: &mut LumenApp, ui: &mut Ui, frame: &mut eframe::Frame) {
    let p = app.p;
    let area = ui.max_rect().shrink(16.0);
    let card = Rect::from_min_size(area.min, vec2(area.width(), area.width()));

    visualizer_card(app, ui, card, true);

    // Details below.
    let mut y = card.bottom() + 16.0;
    let cx = area.center().x;
    let Some(t) = app.current_track() else {
        ui.painter().text(pos2(cx, y + 40.0), egui::Align2::CENTER_CENTER, "No track selected", FontId::proportional(13.0), p.fg_dim);
        return;
    };
    let media = Rect::from_center_size(pos2(cx, y + 75.0), vec2(150.0, 150.0));
    if t.is_video && app.prefs.video_mode && !app.video_full {
        let wide = Rect::from_min_size(pos2(area.left(), y), vec2(area.width(), area.width() * 9.0 / 16.0));
        ui.painter().rect_filled(wide, 8.0, Color32::BLACK);
        if let Some(v) = app.video.as_mut() {
            v.show(ui, frame, wide);
        }
        let resp = ui.interact(wide, Id::unique("np-video"), Sense::click());
        let ex = Rect::from_min_size(pos2(wide.right() - 34.0, wide.top() + 6.0), vec2(28.0, 28.0));
        let er = ui.interact(ex, Id::unique("np-video-expand"), Sense::click());
        if resp.hovered() || er.hovered() {
            ui.painter().rect_filled(ex, 6.0, Color32::from_black_alpha(140));
            icon::paint_centered(ui, icons::EXPAND, ex, 16.0, Color32::WHITE);
        }
        if resp.double_clicked() || er.on_hover_text("Fill window (F)").clicked() {
            app.video_full = true;
        }
        y = wide.bottom() + 10.0;
    } else {
        widgets::artwork(ui, &p, &mut app.art, &ArtKey::Track(t.id), media, 10.0);
        y = media.bottom() + 8.0;
    }
    if t.is_video {
        // Art / Video segmented control.
        let seg = Rect::from_center_size(pos2(cx, y + 14.0), vec2(120.0, 26.0));
        ui.painter().rect(seg, 13.0, p.bg3, Stroke::new(1.0, p.border), StrokeKind::Inside);
        for (i, (label, on)) in [("Art", false), ("Video", true)].into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(seg.left() + i as f32 * 60.0, seg.top()), vec2(60.0, 26.0));
            let resp = ui.interact(r, Id::unique(("np-seg", i)), Sense::click());
            let active = app.prefs.video_mode == on;
            if active {
                ui.painter().rect_filled(r.shrink(2.0), 11.0, p.accent);
            }
            widgets::centered_text(ui, r, label, FontId::proportional(12.0), if active { p.on_accent } else { p.fg_dim });
            if resp.clicked() {
                app.prefs.video_mode = on;
            }
        }
        y = seg.bottom() + 10.0;
    }
    let w = area.width();
    let title = widgets::galley(ui, &t.title, crate::fonts::bold(16.0), p.fg, w);
    ui.painter().galley(pos2(cx - title.size().x / 2.0, y), title.clone(), p.fg);
    y += title.size().y + 4.0;
    let artist = widgets::galley(ui, &t.artist, FontId::proportional(13.0), p.fg_dim, w);
    ui.painter().galley(pos2(cx - artist.size().x / 2.0, y), artist.clone(), p.fg_dim);
    y += artist.size().y + 2.0;
    if !t.album.is_empty() {
        let album = widgets::galley(ui, &t.album, FontId::proportional(12.0), p.fg_dim, w);
        ui.painter().galley(pos2(cx - album.size().x / 2.0, y), album, crate::palette::with_alpha(p.fg_dim, 0.8));
    }
}

/// The visualizer card: the canvas (or "Visualizer off"), the switch at the
/// top left, the full-screen button at the top right, and the tabs.
pub fn visualizer_card(app: &mut LumenApp, ui: &mut Ui, card: Rect, rounded: bool) {
    let p = app.p;
    let radius = if rounded { 10.0 } else { 0.0 };
    ui.painter().rect_filled(card, radius, p.viz_bg);
    if app.prefs.viz_on {
        app.step_spectrum();
        let kind = app.viz_kind();
        let colors = app.viz_colors();
        let t = app.started.elapsed().as_secs_f32();
        let inner = card.shrink(2.0);
        let painter = ui.painter_at(inner);
        viz::draw(kind, &painter, inner, &app.spectrum.frame, colors, &mut app.viz_state, t, app.dt);
    } else {
        widgets::centered_text(ui, card, "Visualizer off", FontId::proportional(13.0), p.fg_dim);
    }

    // Immersive mode has these by its exit button instead (`viz_lights`).
    if !rounded {
        return;
    }

    // Switch, top left.
    let sw = Rect::from_min_size(card.min + vec2(12.0, 12.0), vec2(34.0, 20.0));
    let resp = ui.interact(sw, Id::unique(("viz-switch", rounded)), Sense::click());
    widgets::paint_switch(ui, &p, sw, app.prefs.viz_on);
    if resp.on_hover_text(if app.prefs.viz_on { "Turn the visualizer off" } else { "Turn the visualizer on" }).clicked() {
        app.prefs.viz_on = !app.prefs.viz_on;
    }

    // Immersive, top right.
    let ex = Rect::from_min_size(pos2(card.right() - 40.0, card.top() + 8.0), vec2(32.0, 32.0));
    let er = ui.interact(ex, Id::unique("viz-expand"), Sense::click());
    ui.painter().rect_filled(ex, 8.0, Color32::from_black_alpha(if er.hovered() { 150 } else { 90 }));
    icon::paint_centered(ui, icons::EXPAND, ex, 18.0, Color32::from_white_alpha(230));
    if er.on_hover_text("Immersive mode").clicked() {
        app.immersive = true;
    }

    // Tabs, bottom left.
    if app.prefs.viz_on {
        let mut x = card.left() + 8.0;
        let y = card.bottom() - 8.0 - 22.0;
        for k in viz::Kind::ALL {
            let label = k.label();
            let w = widgets::text_width(ui, label, FontId::proportional(11.0)) + 18.0;
            let r = Rect::from_min_size(pos2(x, y), vec2(w, 22.0));
            let resp = ui.interact(r, Id::unique(("viz-tab", label, rounded)), Sense::click());
            let active = app.viz_kind() == k;
            let fill = if active { p.viz_accent } else { Color32::from_black_alpha(if resp.hovered() { 130 } else { 90 }) };
            ui.painter().rect_filled(r, 11.0, fill);
            let fg = if active { crate::palette::rgba(lumen_core::theme::contrast_on(p.viz_accent.to_srgba_unmultiplied())) } else { Color32::from_white_alpha(191) };
            widgets::centered_text(ui, r, label, FontId::proportional(11.0), fg);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                set_viz(app, k);
            }
            x += w + 4.0;
        }
    }
}

fn set_viz(app: &mut LumenApp, k: viz::Kind) {
    app.prefs.viz = match k {
        viz::Kind::Bars => "bars",
        viz::Kind::Waveform => "waveform",
        viz::Kind::Radial => "radial",
    }
    .into();
    app.viz_state.reset();
}

/// Immersive mode's visualizer controls: one small pill ending at `right`,
/// with the on/off switch and a round button per visualizer, each drawn as
/// a tiny picture of itself.
pub fn viz_lights(app: &mut LumenApp, ui: &mut Ui, right: f32, top: f32) {
    let p = app.p;
    const B: f32 = 28.0;
    let width = 10.0 + 34.0 + 10.0 + (B + 4.0) * 3.0;
    let pill = Rect::from_min_size(pos2(right - width, top), vec2(width, 36.0));
    ui.painter().rect_filled(pill, 18.0, Color32::from_black_alpha(100));

    let sw = Rect::from_min_size(pos2(pill.left() + 10.0, pill.center().y - 10.0), vec2(34.0, 20.0));
    let resp = ui.interact(sw, Id::unique("immersive-viz-switch"), Sense::click());
    widgets::paint_switch(ui, &p, sw, app.prefs.viz_on);
    if resp.on_hover_text(if app.prefs.viz_on { "Turn the visualizer off" } else { "Turn the visualizer on" }).clicked() {
        app.prefs.viz_on = !app.prefs.viz_on;
    }

    let mut x = sw.right() + 10.0;
    for k in viz::Kind::ALL {
        let r = Rect::from_min_size(pos2(x, pill.center().y - B / 2.0), vec2(B, B));
        let resp = ui.interact(r, Id::unique(("immersive-viz", k.label())), Sense::click());
        let active = app.prefs.viz_on && app.viz_kind() == k;
        let fill = if active { p.viz_accent } else { Color32::from_white_alpha(if resp.hovered() { 40 } else { 18 }) };
        ui.painter().circle_filled(r.center(), B / 2.0, fill);
        let fg = if active { crate::palette::rgba(lumen_core::theme::contrast_on(p.viz_accent.to_srgba_unmultiplied())) } else { Color32::from_white_alpha(200) };
        paint_viz_glyph(ui, k, r.center(), fg);
        if resp.on_hover_text(k.label()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            app.prefs.viz_on = true;
            set_viz(app, k);
        }
        x += B + 4.0;
    }
}

fn paint_viz_glyph(ui: &Ui, k: viz::Kind, c: egui::Pos2, color: Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let painter = ui.painter();
    match k {
        viz::Kind::Bars => {
            for (dx, h) in [(-4.5, 6.0), (0.0, 11.0), (4.5, 8.0)] {
                painter.rect_filled(Rect::from_min_max(pos2(c.x + dx - 1.4, c.y + 5.5 - h), pos2(c.x + dx + 1.4, c.y + 5.5)), 1.0, color);
            }
        }
        viz::Kind::Waveform => {
            let pts = (0..=16).map(|i| {
                let t = i as f32 / 16.0;
                pos2(c.x - 7.0 + t * 14.0, c.y - (t * std::f32::consts::TAU * 1.5).sin() * 4.0)
            });
            painter.add(egui::Shape::line(pts.collect(), stroke));
        }
        viz::Kind::Radial => {
            painter.circle_stroke(c, 3.5, stroke);
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let d = vec2(a.cos(), a.sin());
                painter.line_segment([c + d * 5.5, c + d * 8.0], stroke);
            }
        }
    }
}
