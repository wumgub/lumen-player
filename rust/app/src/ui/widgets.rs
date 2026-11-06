//! The building blocks, drawn to match app.css: round icon buttons, the white
//! play button, pill buttons, nav rows, the seek/volume sliders, the switch,
//! artwork with its placeholders, and single-line text that ends in "…".

use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Galley, Id, Mesh, Pos2, Rect, Response, Sense, Shape,
    Stroke, Ui, Vec2, pos2, vec2,
};

use crate::art::{ArtCache, ArtKey};
use crate::fonts;
use crate::palette::{Palette, mix, with_alpha};
use crate::ui::{icon, icons};

pub fn galley(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, color));
    job.wrap = TextWrapping::truncate_at_width(max_width.max(1.0));
    ui.painter().layout_job(job)
}

/// One line of text at `pos` (left-top), cut with "…" at `max_width`.
pub fn text(ui: &Ui, pos: Pos2, s: &str, font: FontId, color: Color32, max_width: f32) -> Rect {
    let g = galley(ui, s, font, color, max_width);
    let rect = Rect::from_min_size(pos, g.size());
    ui.painter().galley(pos, g, color);
    rect
}

/// One line of text vertically centred on `y`, starting at `x`.
pub fn text_v(ui: &Ui, x: f32, y: f32, s: &str, font: FontId, color: Color32, max_width: f32) -> Rect {
    let g = galley(ui, s, font, color, max_width);
    let pos = pos2(x, y - g.size().y / 2.0);
    let rect = Rect::from_min_size(pos, g.size());
    ui.painter().galley(pos, g, color);
    rect
}

pub fn text_width(ui: &Ui, s: &str, font: FontId) -> f32 {
    ui.painter().layout_no_wrap(s.to_owned(), font, Color32::WHITE).size().x
}

fn hover_cursor(r: Response) -> Response {
    r.on_hover_cursor(CursorIcon::PointingHand)
}

/// `.icon-btn`: a round button with a line icon, dim until hovered or active.
pub fn icon_button(ui: &mut Ui, p: &Palette, svg: &'static str, size: f32, icon_size: f32, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    paint_icon_button(ui, p, &resp, rect, svg, icon_size, active);
    hover_cursor(resp)
}

pub fn paint_icon_button(ui: &Ui, p: &Palette, resp: &Response, rect: Rect, svg: &'static str, icon_size: f32, active: bool) {
    if resp.hovered() {
        ui.painter().circle_filled(rect.center(), rect.width() / 2.0, p.bg3);
    }
    let color = if active { p.accent } else if resp.hovered() { p.fg } else { p.fg_dim };
    icon::paint_centered(ui, svg, rect, icon_size, color);
}

/// `.play-btn`: the solid circle in the transport bar.
pub fn play_button(ui: &mut Ui, p: &Palette, playing: bool, size: f32, icon_size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let grow = if resp.is_pointer_button_down_on() { 0.96 } else if resp.hovered() { 1.05 } else { 1.0 };
    ui.painter().circle_filled(rect.center(), size / 2.0 * grow, p.fg);
    let svg = if playing { icons::PAUSE } else { icons::PLAY };
    // The play triangle looks centred a touch to the right.
    let nudge = if playing { 0.0 } else { icon_size * 0.06 };
    icon::paint_centered(ui, svg, rect.translate(vec2(nudge, 0.0)), icon_size, p.bg_solid());
    hover_cursor(resp)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pill {
    /// `.edit-btn`
    Neutral,
    /// `.play-all`
    Accent,
    /// `.edit-btn.active`
    Active,
    /// A borderless one, for menus and small actions.
    Ghost,
}

/// A rounded button with text (and maybe an icon before it).
pub fn pill(ui: &mut Ui, p: &Palette, label: &str, kind: Pill, svg: Option<&'static str>) -> Response {
    let font = fonts::bold(13.0);
    let tw = text_width(ui, label, font.clone());
    let icon_w = if svg.is_some() { 14.0 + 8.0 } else { 0.0 };
    let pad = if svg.is_some() { vec2(15.0, 0.0) } else { vec2(18.0, 0.0) };
    let size = vec2(pad.x + icon_w + tw + 18.0, 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hov = resp.hovered();
    let (fill, stroke, fg) = match kind {
        Pill::Neutral => (if hov { mix(p.bg3, p.fg, 0.06) } else { p.bg3 }, Stroke::new(1.0, p.border), p.fg),
        Pill::Accent => (
            if hov { with_alpha(p.accent, 0.24) } else { p.accent_soft },
            if hov { Stroke::new(1.0, with_alpha(p.accent, 0.5)) } else { Stroke::NONE },
            p.accent,
        ),
        Pill::Active => (p.accent, Stroke::NONE, p.on_accent),
        Pill::Ghost => (if hov { p.bg3 } else { Color32::TRANSPARENT }, Stroke::NONE, if hov { p.fg } else { p.fg_dim }),
    };
    ui.painter().rect(rect, CornerRadius::same(255), fill, stroke, egui::StrokeKind::Inside);
    let mut x = rect.left() + pad.x;
    if let Some(svg) = svg {
        icon::paint(ui, svg, Rect::from_center_size(pos2(x + 7.0, rect.center().y), vec2(14.0, 14.0)), fg);
        x += icon_w;
    }
    text_v(ui, x, rect.center().y, label, font, fg, tw + 2.0);
    hover_cursor(resp)
}

/// `.nav-item`: a sidebar row with an icon, a label and maybe a count.
pub fn nav_item(ui: &mut Ui, p: &Palette, svg: &'static str, label: &str, active: bool, count: Option<i64>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    paint_nav_item(ui, p, &resp, rect, svg, label, active, count, 18.0);
    hover_cursor(resp)
}

#[allow(clippy::too_many_arguments)]
pub fn paint_nav_item(
    ui: &Ui,
    p: &Palette,
    resp: &Response,
    rect: Rect,
    svg: &'static str,
    label: &str,
    active: bool,
    count: Option<i64>,
    icon_size: f32,
) {
    if active {
        ui.painter().rect_filled(rect, 6.0, p.accent_soft);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, p.bg3);
    }
    let fg = if active || resp.hovered() { p.fg } else { p.fg_dim };
    let icon_color = if active { p.accent } else { fg };
    icon::paint(ui, svg, Rect::from_center_size(pos2(rect.left() + 12.0 + icon_size / 2.0, rect.center().y), vec2(icon_size, icon_size)), icon_color);
    let x = rect.left() + 12.0 + icon_size + 10.0;
    let mut right = rect.right() - 10.0;
    if let Some(c) = count {
        let r = text_v(ui, 0.0, -1000.0, &c.to_string(), FontId::proportional(11.0), p.fg_dim, 60.0);
        let w = r.width();
        text_v(ui, right - w, rect.center().y, &c.to_string(), FontId::proportional(11.0), p.fg_dim, 60.0);
        right -= w + 8.0;
    }
    text_v(ui, x, rect.center().y, label, FontId::proportional(14.0), fg, right - x);
}

/// `.nav-head`: a small caps section title with a + button on the right.
pub fn section_head(ui: &mut Ui, p: &Palette, label: &str, with_plus: bool) -> Option<Response> {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let caps = label.to_uppercase();
    let font = FontId::proportional(11.0);
    let mut x = rect.left() + 8.0;
    // Letter-spaced, like `letter-spacing: 1px`.
    for ch in caps.chars() {
        let s = ch.to_string();
        let r = text_v(ui, x, rect.center().y, &s, font.clone(), p.fg_dim, 20.0);
        x += r.width() + 1.0;
    }
    if !with_plus {
        return None;
    }
    let b = Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(24.0, 24.0));
    let resp = ui.interact(b, ui.id().with(("plus", label)), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(b, 6.0, p.bg3);
    }
    icon::paint_centered(ui, icons::PLUS, b, 14.0, if resp.hovered() { p.fg } else { p.fg_dim });
    Some(hover_cursor(resp))
}

pub struct SliderResult {
    pub changed: bool,
    pub released: bool,
    pub dragging: bool,
}

/// `.seek` / `.volume`: a thin track with an accent fill and a round thumb.
pub fn slider(ui: &mut Ui, p: &Palette, id: Id, value: &mut f64, max: f64, width: f32) -> SliderResult {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let max = if max > 0.0 { max } else { 1.0 };
    let mut changed = false;
    if (resp.dragged() || resp.clicked() || resp.drag_started())
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
        *value = t * max;
        changed = true;
    }
    let t = (*value / max).clamp(0.0, 1.0) as f32;
    let track = Rect::from_min_max(pos2(rect.left(), rect.center().y - 2.0), pos2(rect.right(), rect.center().y + 2.0));
    ui.painter().rect_filled(track, 2.0, p.bg3_solid());
    let fill = Rect::from_min_max(track.min, pos2(track.left() + track.width() * t, track.bottom()));
    ui.painter().rect_filled(fill, 2.0, p.accent);
    let hot = resp.hovered() || resp.dragged();
    let r = if hot { 6.5 } else { 5.5 };
    ui.painter().circle_filled(pos2(track.left() + track.width() * t, rect.center().y), r, p.fg);
    let resp = hover_cursor(resp);
    SliderResult { changed, released: resp.drag_stopped() || resp.clicked(), dragging: resp.dragged() }
}

/// The little on/off switch (VizToggle).
pub fn switch(ui: &mut Ui, p: &Palette, on: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 20.0), Sense::click());
    paint_switch(ui, p, rect, on);
    hover_cursor(resp)
}

pub fn paint_switch(ui: &Ui, p: &Palette, rect: Rect, on: bool) {
    let fill = if on { p.accent } else { with_alpha(p.fg_dim, 0.35) };
    ui.painter().rect_filled(rect, 10.0, fill);
    let x = if on { rect.right() - 10.0 } else { rect.left() + 10.0 };
    ui.painter().circle_filled(pos2(x, rect.center().y), 7.0, Color32::WHITE);
}

/// Album art for `key` in `rect`, or a placeholder (a note on track art, a
/// coloured gradient with a disc for albums, as AlbumCover.tsx does).
pub fn artwork(ui: &Ui, p: &Palette, art: &mut ArtCache, key: &ArtKey, rect: Rect, radius: f32) {
    let ppp = ui.ctx().pixels_per_point();
    if let Some(tex) = art.get(key, rect.width(), ppp) {
        let mut mesh = Mesh::with_texture(tex.id());
        mesh.add_rect_with_uv(rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        if radius <= 0.5 {
            ui.painter().add(Shape::mesh(mesh));
        } else {
            let shape = egui::epaint::RectShape::filled(rect, CornerRadius::same(radius.round() as u8), Color32::WHITE)
                .with_texture(tex.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
            ui.painter().add(shape);
        }
        return;
    }
    match key {
        ArtKey::Album(name) => album_placeholder(ui, rect, radius, name, rect.width() * 0.22),
        ArtKey::Track(_) => {
            ui.painter().rect_filled(rect, radius, p.bg3_solid());
            icon::paint_centered(ui, icons::MUSIC_NOTE, rect, (rect.width() * 0.42).clamp(10.0, 64.0), p.fg_dim);
        }
    }
}

fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Color32::from_rgb(((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

/// The gradient the web app gives an album (or artist) with no picture,
/// seeded from the name so it's always the same.
pub fn seeded_gradient(seed: &str) -> (Color32, Color32) {
    let mut h: u32 = 0;
    for c in seed.encode_utf16() {
        h = (h.wrapping_mul(31).wrapping_add(c as u32)) % 360;
    }
    let h2 = (h + 50) % 360;
    (hsl(h as f32, 0.60, 0.45), hsl(h2 as f32, 0.65, 0.35))
}

/// A 135° gradient fill, for placeholders.
pub fn diagonal_gradient(ui: &Ui, rect: Rect, radius: f32, a: Color32, b: Color32) {
    // Paint the rounded rect in `a`, then lay the gradient over it clipped to a
    // slightly inset rect so the rounded corners stay clean.
    ui.painter().rect_filled(rect, radius, a);
    let inner = rect.shrink(radius * 0.3);
    let mut mesh = Mesh::default();
    let mid = mix(a, b, 0.5);
    mesh.colored_vertex(inner.left_top(), a);
    mesh.colored_vertex(inner.right_top(), mid);
    mesh.colored_vertex(inner.right_bottom(), b);
    mesh.colored_vertex(inner.left_bottom(), mid);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(Shape::mesh(mesh));
    // Fade the gradient into the corners' colour at the very edge.
    ui.painter().rect_stroke(rect.shrink(radius * 0.15), radius, Stroke::new(radius * 0.3, mid), egui::StrokeKind::Inside);
}

pub fn album_placeholder(ui: &Ui, rect: Rect, radius: f32, name: &str, icon_size: f32) {
    let (a, b) = seeded_gradient(name);
    diagonal_gradient(ui, rect, radius, a, b);
    icon::paint_centered(ui, icons::DISC, rect, icon_size.clamp(10.0, 48.0), Color32::from_white_alpha(217));
}

/// The three bouncing bars shown on the playing row (`.eq`).
pub fn eq_bars(ui: &Ui, center: Pos2, color: Color32, t: f64) {
    for i in 0..3 {
        let phase = t * 5.0 + i as f64 * 1.7;
        let h = 4.0 + (phase.sin() * 0.5 + 0.5) as f32 * 8.0;
        let x = center.x - 5.0 + i as f32 * 4.5;
        let r = Rect::from_min_max(pos2(x, center.y + 6.0 - h), pos2(x + 3.0, center.y + 6.0));
        ui.painter().rect_filled(r, 1.0, color);
    }
}

/// A frame for text inputs in the theme's colours.
pub fn input_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg3_solid())
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(10, 7))
}

pub fn subtle_separator(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(1.0, p.border));
}

pub fn centered_text(ui: &Ui, rect: Rect, s: &str, font: FontId, color: Color32) {
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, s, font, color);
}

pub fn tooltip(resp: Response, text: &str) -> Response {
    resp.on_hover_text(text)
}

pub fn size(w: f32, h: f32) -> Vec2 {
    vec2(w, h)
}
