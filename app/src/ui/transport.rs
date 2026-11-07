//! The transport bar: what's playing, the controls and seek
//! bar in the middle, the queue button and volume on the right.

use eframe::egui::{self, FontId, Id, Rect, Sense, Ui, pos2, vec2};
use lumen_playback::queue::Repeat;

use crate::app::LumenApp;
use crate::art::ArtKey;
use crate::ui::{format_time, icons, widgets};

pub fn show(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let bar = ui.max_rect().shrink2(vec2(24.0, 0.0));
    let cy = bar.center().y;

    // Left: art and titles.
    let left_w = (bar.width() - 420.0) / 2.0;
    match app.current_track() {
        Some(t) => {
            let art = Rect::from_min_size(pos2(bar.left(), cy - 26.0), vec2(52.0, 52.0));
            widgets::artwork(ui, &p, &mut app.art, &ArtKey::Track(t.id), art, 6.0);
            let x = art.right() + 12.0;
            let w = left_w - 64.0;
            widgets::text_v(ui, x, cy - 9.0, &t.title, crate::fonts::medium(14.0), p.fg, w);
            let sub = if t.album.is_empty() { t.artist.clone() } else { format!("{} — {}", t.artist, t.album) };
            widgets::text_v(ui, x, cy + 10.0, &sub, FontId::proportional(12.0), p.fg_dim, w);
        }
        None => {
            widgets::text_v(ui, bar.left(), cy - 9.0, "Nothing playing", crate::fonts::medium(14.0), p.fg, left_w);
            widgets::text_v(ui, bar.left(), cy + 10.0, "Pick a track to start", FontId::proportional(12.0), p.fg_dim, left_w);
        }
    }

    // Centre: buttons over the seek row.
    let center_w = 420.0;
    let cx = bar.center().x;
    let buttons = Rect::from_center_size(pos2(cx, cy - 12.0), vec2(250.0, 46.0));
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(buttons).layout(egui::Layout::left_to_right(egui::Align::Center)));
    b.spacing_mut().item_spacing.x = 10.0;
    let snap = app.snap.clone();
    if widgets::icon_button(&mut b, &p, icons::SHUFFLE, 38.0, 18.0, snap.shuffle).on_hover_text("Shuffle").clicked() {
        app.player.set_shuffle(!snap.shuffle);
    }
    if widgets::icon_button(&mut b, &p, icons::PREV, 38.0, 22.0, false).on_hover_text("Previous").clicked() {
        app.player.prev();
    }
    if widgets::play_button(&mut b, &p, !snap.paused, 46.0, 24.0).on_hover_text("Play/Pause").clicked() {
        app.player.toggle();
    }
    if widgets::icon_button(&mut b, &p, icons::NEXT, 38.0, 22.0, false).on_hover_text("Next").clicked() {
        app.player.next();
    }
    let rep_icon = if snap.repeat == Repeat::One { icons::REPEAT_ONE } else { icons::REPEAT };
    let rep_label = match snap.repeat {
        Repeat::Off => "Repeat: off",
        Repeat::All => "Repeat: all",
        Repeat::One => "Repeat: one",
    };
    if widgets::icon_button(&mut b, &p, rep_icon, 38.0, 18.0, snap.repeat != Repeat::Off).on_hover_text(rep_label).clicked() {
        app.player.cycle_repeat();
    }

    let seek_y = cy + 23.0;
    let time_font = FontId::proportional(11.0);
    let shown = app.seek_drag.unwrap_or(snap.position);
    let l = Rect::from_center_size(pos2(cx - center_w / 2.0 + 18.0, seek_y), vec2(36.0, 14.0));
    ui.painter().text(l.center(), egui::Align2::CENTER_CENTER, format_time(shown), time_font.clone(), p.fg_dim);
    let r = Rect::from_center_size(pos2(cx + center_w / 2.0 - 18.0, seek_y), vec2(36.0, 14.0));
    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, format_time(snap.duration), time_font, p.fg_dim);
    let seek_rect = Rect::from_min_max(pos2(l.right() + 6.0, seek_y - 8.0), pos2(r.left() - 6.0, seek_y + 8.0));
    let mut s = ui.new_child(egui::UiBuilder::new().max_rect(seek_rect));
    seek_bar(app, &mut s, seek_rect.width(), Id::unique("seek"));

    // Right: queue and volume.
    let vol_w = 110.0;
    let right = bar.right();
    let vr = Rect::from_min_max(pos2(right - vol_w, cy - 8.0), pos2(right, cy + 8.0));
    let mut v = ui.new_child(egui::UiBuilder::new().max_rect(vr));
    let mut vol = app.prefs.volume;
    if widgets::slider(&mut v, &p, Id::unique("volume"), &mut vol, 1.5, vol_w).changed {
        app.prefs.volume = vol;
        app.player.set_volume(vol);
    }
    let mute = Rect::from_center_size(pos2(vr.left() - 24.0, cy), vec2(38.0, 38.0));
    let mut m = ui.new_child(egui::UiBuilder::new().max_rect(mute));
    let svg = if app.prefs.volume > 0.0 { icons::VOL_HIGH } else { icons::VOL_MUTE };
    if widgets::icon_button(&mut m, &p, svg, 38.0, 18.0, false).on_hover_text("Mute").clicked() {
        let v = if app.prefs.volume > 0.0 { 0.0 } else { 0.9 };
        app.prefs.volume = v;
        app.player.set_volume(v);
    }
    let qr = Rect::from_center_size(pos2(mute.left() - 26.0, cy), vec2(38.0, 38.0));
    let mut q = ui.new_child(egui::UiBuilder::new().max_rect(qr));
    let resp = widgets::icon_button(&mut q, &p, icons::LIST, 38.0, 18.0, app.queue_open).on_hover_text("Queue");
    if resp.clicked() {
        app.queue_open = !app.queue_open;
    }
    let up_next = snap.index.map(|i| snap.queue.len().saturating_sub(i + 1)).unwrap_or(0);
    if up_next > 0 {
        let label = if up_next > 99 { "99+".to_owned() } else { up_next.to_string() };
        let w = widgets::text_width(ui, &label, FontId::proportional(10.0)) + 10.0;
        let badge = Rect::from_min_size(pos2(qr.center().x + 2.0, qr.top() + 2.0), vec2(w.max(16.0), 16.0));
        ui.painter().rect_filled(badge, 8.0, p.accent);
        widgets::centered_text(ui, badge, &label, FontId::proportional(10.0), p.on_accent);
    }
}

/// The seek bar; dragging previews the time and seeks on release.
pub fn seek_bar(app: &mut LumenApp, ui: &mut Ui, width: f32, id: Id) {
    let p = app.p;
    let dur = app.snap.duration;
    let mut v = app.seek_drag.unwrap_or(app.snap.position);
    let r = widgets::slider(ui, &p, id, &mut v, dur, width);
    if r.dragging {
        app.seek_drag = Some(v);
    }
    if r.released {
        app.player.seek(v);
        app.seek_drag = None;
    } else if !r.dragging {
        app.seek_drag = None;
    }
    let _ = Sense::hover();
}
