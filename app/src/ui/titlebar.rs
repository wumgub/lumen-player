//! The title bar: brand, the search box that opens out from its icon, the
//! theme picker and the settings button. On macOS the window's own buttons sit
//! on its left, and the empty space drags the window.

use eframe::egui::{self, Color32, FontId, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::LumenApp;
use crate::library::View;
use crate::palette::with_alpha;
use crate::ui::{icon, icons, widgets};

pub fn show(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let bar = ui.max_rect();

    // Drag the window from anywhere that isn't a control.
    let drag = ui.interact(bar, Id::unique("titlebar-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }

    let left = if cfg!(target_os = "macos") { 90.0 } else { 16.0 };
    let cy = bar.center().y;
    let mut x = bar.left() + left;

    // Brand: a glowing dot and the name.
    let dot = pos2(x + 6.0, cy);
    for (r, a) in [(12.0, 0.10), (9.0, 0.18), (7.5, 0.3)] {
        ui.painter().circle_filled(dot, r, with_alpha(p.accent, a));
    }
    ui.painter().circle_filled(dot, 6.0, crate::palette::mix(p.accent, p.accent2, 0.5));
    x += 21.0;
    let r = widgets::text_v(ui, x, cy, "Lumen", crate::fonts::bold(16.0), p.fg, 80.0);
    x = r.right() + 20.0;

    // Search: an icon that opens into a field.
    let open = app.search_open || !app.search.is_empty();
    let width = if open { (bar.right() - x - 180.0).clamp(160.0, 440.0) } else { 34.0 };
    let sr = Rect::from_min_size(pos2(x, cy - 17.0), vec2(width, 34.0));
    if open {
        ui.painter().rect(sr, 17.0, p.bg3_solid(), Stroke::new(1.0, p.border), StrokeKind::Inside);
    } else {
        ui.painter().circle_filled(sr.center(), 17.0, p.bg3_solid());
    }
    let icon_rect = Rect::from_min_size(sr.min, vec2(34.0, 34.0));
    let ic = ui.interact(icon_rect, Id::unique("search-icon"), Sense::click());
    icon::paint_centered(ui, icons::SEARCH, icon_rect, 16.0, if ic.hovered() { p.fg } else { p.fg_dim });
    let ic = ic.on_hover_cursor(egui::CursorIcon::PointingHand);
    if ic.clicked() {
        app.search_open = !open;
        if !app.search_open {
            app.search.clear();
        }
    }
    if open {
        let field = Rect::from_min_max(pos2(sr.left() + 34.0, sr.top()), pos2(sr.right() - 12.0, sr.bottom()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let before = app.search.clone();
        let edit = egui::TextEdit::singleline(&mut app.search)
            .hint_text("Search songs, artists, albums, notes")
            .frame(egui::Frame::NONE)
            .font(FontId::proportional(13.0))
            .text_color(p.fg)
            .desired_width(field.width())
            .id(Id::unique("search-field"));
        let resp = child.add(edit);
        if app.search_open && !resp.has_focus() && before.is_empty() && app.search.is_empty() && !resp.lost_focus() {
            resp.request_focus();
        }
        if resp.lost_focus() && app.search.is_empty() {
            app.search_open = false;
        }
        if resp.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
            app.search.clear();
            app.search_open = false;
        }
        if app.search != before && !app.search.is_empty() && app.view != View::Songs {
            // Searching always looks through every song.
            app.set_view(View::Songs);
        }
    }
    x = sr.right() + 14.0;

    // Theme picker pill.
    let label = lumen_core::theme::THEMES
        .iter()
        .find(|t| t.id == app.prefs.theme)
        .map(|t| t.label)
        .unwrap_or("Neon");
    let tw = widgets::text_width(ui, label, FontId::proportional(13.0));
    let pr = Rect::from_min_size(pos2(x, cy - 17.0), vec2(tw + 58.0, 34.0));
    let pill = ui.interact(pr, Id::unique("theme-pill"), Sense::click());
    ui.painter().rect(
        pr,
        17.0,
        if pill.hovered() { crate::palette::mix(p.bg3_solid(), p.fg, 0.06) } else { p.bg3_solid() },
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let sw = pos2(pr.left() + 20.0, cy);
    ui.painter().circle_filled(sw, 7.0, crate::palette::mix(p.accent, p.accent2, 0.5));
    widgets::text_v(ui, pr.left() + 36.0, cy, label, FontId::proportional(13.0), p.fg, tw + 2.0);
    icon::paint_centered(ui, icons::CHEVRON_DOWN, Rect::from_center_size(pos2(pr.right() - 14.0, cy), vec2(12.0, 12.0)), 10.0, p.fg_dim);
    if pill.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        app.theme_menu_open = !app.theme_menu_open;
    }
    app.theme_pill_rect = pr;
    x = pr.right() + 14.0;

    // Settings.
    let gr = Rect::from_min_size(pos2(x, cy - 18.0), vec2(36.0, 36.0));
    let gear = ui.interact(gr, Id::unique("settings-btn"), Sense::click());
    ui.painter().circle_filled(gr.center(), 18.0, if gear.hovered() { crate::palette::mix(p.bg3_solid(), p.fg, 0.08) } else { p.bg3_solid() });
    icon::paint_centered(ui, icons::GEAR, gr, 18.0, if gear.hovered() { p.fg } else { p.fg_dim });
    if gear.on_hover_text("Settings").clicked() {
        app.settings_open = true;
    }
    let _ = Color32::TRANSPARENT;
}
