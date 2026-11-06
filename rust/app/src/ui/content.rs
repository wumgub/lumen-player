//! The middle of the window (LibraryView.tsx): a view's header with sort,
//! Edit and Play all; the track list with edit mode (multi-select, or drag to
//! reorder an album) and the selection bar; and the artist and album grids.

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lumen_core::{Sort, Track};

use crate::app::{ContextMenu, Dialog, LumenApp, RenameKind, item};
use crate::art::ArtKey;
use crate::library::{SORT_OPTIONS, SortState, View, sort_label};
use crate::palette::with_alpha;
use crate::ui::{format_duration, format_time, icon, icons, pluralize, stats, widgets};

const PAD: egui::Vec2 = egui::vec2(32.0, 28.0);

pub fn show(app: &mut LumenApp, ui: &mut Ui) {
    let area = ui.max_rect();
    match app.view.clone() {
        View::Stats => {
            let inner = area.shrink2(PAD);
            let mut c = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            stats::show(app, &mut c);
        }
        View::Artists => artist_grid(app, ui, area),
        View::Albums => album_grid(app, ui, area),
        _ => track_view(app, ui, area),
    }
}

fn header_title(view: &View) -> (String, Option<&'static str>) {
    match view {
        View::Songs => ("All Songs".into(), None),
        View::Artist(n) | View::Album(n) => (n.clone(), None),
        View::Playlist(_, n) => (n.clone(), Some("Playlist")),
        View::Folder(f) => (
            std::path::Path::new(f).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| f.clone()),
            Some("Folder"),
        ),
        _ => (String::new(), None),
    }
}

fn track_view(app: &mut LumenApp, ui: &mut Ui, area: Rect) {
    let p = app.p;
    let view = app.view.clone();
    let tracks: Vec<Track> = app.lib.tracks(&view, &app.search.clone(), app.sort).to_vec();
    let total: f64 = tracks.iter().map(|t| t.duration).sum();
    let reorderable = app.edit_mode && matches!(view, View::Album(_)) && app.sort.key == Sort::Custom && app.search.is_empty();
    let selectable = app.edit_mode && !reorderable;

    // ---- header ----
    let inner = area.shrink2(PAD);
    let is_album = matches!(view, View::Album(_));
    let header_h = if is_album { 86.0 } else { 64.0 };
    let header = Rect::from_min_size(inner.min, vec2(inner.width(), header_h));
    let mut x = header.left();
    if let View::Album(name) = &view {
        let cover = Rect::from_min_size(pos2(x, header.top()), vec2(72.0, 72.0));
        widgets::artwork(ui, &p, &mut app.art, &ArtKey::Album(name.clone()), cover, 8.0);
        let cr = ui.interact(cover, Id::new("album-cover-btn"), Sense::click());
        if cr.hovered() {
            let b = Rect::from_min_size(pos2(cover.right() - 28.0, cover.bottom() - 28.0), vec2(24.0, 24.0));
            ui.painter().rect_filled(b, 6.0, Color32::from_black_alpha(160));
            icon::paint_centered(ui, icons::IMAGE, b, 14.0, Color32::WHITE);
        }
        if cr.on_hover_text("Change cover").clicked() {
            app.dialog = Some(Dialog::Cover { album: name.clone() });
        }
        x = cover.right() + 16.0;
    }
    let (title, sub_prefix) = header_title(&view);
    let actions_w = 340.0;
    let title_w = (header.right() - x - actions_w).max(120.0);
    let tg = widgets::galley(ui, &title, crate::fonts::bold(30.0), p.fg, title_w);
    let title_y = if is_album { header.top() + 6.0 } else { header.top() };
    let title_rect = Rect::from_min_size(pos2(x, title_y), tg.size());
    ui.painter().galley(title_rect.min, tg, p.fg);
    let can_rename = matches!(view, View::Artist(_) | View::Album(_));
    if can_rename {
        let pr = Rect::from_min_size(pos2(title_rect.right() + 10.0, title_rect.center().y - 13.0), vec2(26.0, 26.0));
        let hover_header = ui.rect_contains_pointer(header);
        let resp = ui.interact(pr, Id::new("title-edit"), Sense::click());
        if hover_header || resp.hovered() {
            if resp.hovered() {
                ui.painter().rect_filled(pr, 6.0, p.bg3);
            }
            icon::paint_centered(ui, icons::PENCIL, pr, 16.0, if resp.hovered() { p.fg } else { p.fg_dim });
        }
        if resp.on_hover_text("Rename").clicked() {
            app.dialog = Some(match &view {
                View::Artist(n) => Dialog::Rename {
                    kind: RenameKind::Artist(n.clone()),
                    title: "Rename artist",
                    label: "Renames this artist across every track",
                    text: n.clone(),
                    initial: n.clone(),
                },
                View::Album(n) => Dialog::Rename {
                    kind: RenameKind::Album(n.clone()),
                    title: "Rename album",
                    label: "Renames this album across every track",
                    text: n.clone(),
                    initial: n.clone(),
                },
                _ => unreachable!(),
            });
        }
    }
    let mut sub = String::new();
    if let Some(s) = sub_prefix {
        sub.push_str(s);
        sub.push_str(" · ");
    }
    sub.push_str(&pluralize(tracks.len() as i64, "song"));
    if total > 0.0 {
        sub.push_str(&format!(" · {}", format_duration(total)));
    }
    if reorderable && tracks.len() > 1 {
        sub.push_str(" · drag to reorder");
    }
    if selectable {
        sub.push_str(" · click to select");
    }
    widgets::text(ui, pos2(x, title_rect.bottom() + 4.0), &sub, FontId::proportional(13.0), p.fg_dim, title_w);

    // Actions, right-aligned.
    let actions = Rect::from_min_max(pos2(header.right() - actions_w, header.top() + if is_album { 30.0 } else { 8.0 }), pos2(header.right(), header.top() + 50.0));
    let mut a = ui.new_child(egui::UiBuilder::new().max_rect(actions).layout(egui::Layout::right_to_left(egui::Align::Center)));
    a.spacing_mut().item_spacing.x = 10.0;
    if !tracks.is_empty() && !app.edit_mode && widgets::pill(&mut a, &p, "Play all", widgets::Pill::Accent, Some(icons::PLAY)).clicked() {
        app.play_tracks(&tracks, 0);
    }
    if !tracks.is_empty() {
        let label = if app.edit_mode { "Done" } else { "Edit" };
        let kind = if app.edit_mode { widgets::Pill::Active } else { widgets::Pill::Neutral };
        if widgets::pill(&mut a, &p, label, kind, None).on_hover_text(if app.edit_mode { "Finish editing" } else { "Reorder / select tracks" }).clicked() {
            app.edit_mode = !app.edit_mode;
            if !app.edit_mode {
                app.selected.clear();
            } else if is_album && app.sort.key != Sort::Custom {
                // Entering edit on an album defaults to Track order so dragging works.
                app.sort = SortState { key: Sort::Custom, descending: false };
            }
        }
    }
    if !matches!(view, View::Playlist(..)) && !app.edit_mode {
        sort_control(app, &mut a, is_album);
    }

    // ---- rows ----
    let list = Rect::from_min_max(pos2(area.left(), header.bottom() + 12.0), area.max);
    if tracks.is_empty() {
        let msg = if app.search.is_empty() { "Nothing here yet. Add a folder and scan." } else { "No matches" };
        icon::paint_centered(ui, icons::DISC, Rect::from_center_size(list.center() - vec2(0.0, 30.0), vec2(40.0, 40.0)), 40.0, p.fg_dim);
        widgets::centered_text(ui, Rect::from_center_size(list.center() + vec2(0.0, 14.0), vec2(400.0, 20.0)), msg, FontId::proportional(14.0), p.fg_dim);
        return;
    }
    let heights: Vec<f32> = tracks.iter().map(|t| if t.note.is_empty() { 50.0 } else { 62.0 }).collect();
    let mut offsets = Vec::with_capacity(heights.len() + 1);
    let mut acc = 0.0;
    for h in &heights {
        offsets.push(acc);
        acc += h + 2.0;
    }
    let content_h = acc + if selectable && !app.selected.is_empty() { 80.0 } else { 24.0 };
    let current = app.snap.current.as_ref().map(|c| c.track_id);
    let playing = !app.snap.paused;
    let t_anim = app.started.elapsed().as_secs_f64();
    if playing && current.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
    }
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list));
    let mut clicked: Option<usize> = None;
    let mut menu: Option<(Track, egui::Pos2)> = None;
    let mut drop_to: Option<usize> = None;
    egui::ScrollArea::vertical()
        .id_salt(("tracks", format!("{view:?}")))
        .auto_shrink([false, false])
        .show_viewport(&mut c, |ui, viewport| {
            ui.set_height(content_h);
            let origin = ui.max_rect().min;
            let width = ui.max_rect().width() - PAD.x * 2.0;
            let first = offsets.partition_point(|&o| o + 64.0 < viewport.min.y);
            for i in first..tracks.len() {
                let top = offsets[i];
                if top > viewport.max.y {
                    break;
                }
                let t = &tracks[i];
                let rect = Rect::from_min_size(origin + vec2(PAD.x, top), vec2(width, heights[i]));
                let resp = ui.interact(rect, Id::new(("row", i, t.id)), if reorderable { Sense::click_and_drag() } else { Sense::click() });
                let is_current = current == Some(t.id);
                let is_selected = app.selected.contains(&t.id);
                let fill = if selectable && is_selected {
                    with_alpha(p.accent, 0.22)
                } else if is_current {
                    p.accent_soft
                } else if resp.hovered() {
                    p.bg3
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, 6.0, fill);
                if let Some(from) = app.drag_row
                    && from != i
                    && ui.rect_contains_pointer(rect)
                {
                    ui.painter().rect_stroke(rect, 6.0, Stroke::new(1.5, p.accent), StrokeKind::Inside);
                    if ui.input(|inp| inp.pointer.any_released()) {
                        drop_to = Some(i);
                    }
                }
                let cy = rect.center().y;
                // Index column: number, eq bars, play on hover, checkbox or grip.
                let idx = Rect::from_min_size(pos2(rect.left() + 10.0, cy - 17.0), vec2(34.0, 34.0));
                if selectable {
                    let b = Rect::from_center_size(idx.center(), vec2(16.0, 16.0));
                    if is_selected {
                        ui.painter().rect_filled(b, 4.0, p.accent);
                        icon::paint_centered(ui, icons::CHECK, b, 12.0, p.on_accent);
                    } else {
                        ui.painter().rect_stroke(b, 4.0, Stroke::new(1.5, p.fg_dim), StrokeKind::Inside);
                    }
                } else if reorderable {
                    icon::paint_centered(ui, icons::GRIP, idx, 16.0, p.fg_dim);
                } else if resp.hovered() {
                    icon::paint_centered(ui, icons::PLAY, idx, 14.0, p.fg);
                } else if is_current && playing {
                    widgets::eq_bars(ui, idx.center(), p.accent, t_anim);
                } else {
                    ui.painter().text(idx.center(), egui::Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(13.0), p.fg_dim);
                }
                let art = Rect::from_min_size(pos2(idx.right() + 12.0, cy - 18.0), vec2(36.0, 36.0));
                widgets::artwork(ui, &p, &mut app.art, &ArtKey::Track(t.id), art, 5.0);
                // Columns: title block, album, time, more.
                let more = Rect::from_center_size(pos2(rect.right() - 28.0, cy), vec2(30.0, 30.0));
                let time_r = rect.right() - 56.0 - 12.0;
                let main_x = art.right() + 12.0;
                let flex = (time_r - main_x - 24.0).max(60.0);
                let main_w = flex / 2.2;
                let album_x = main_x + main_w + 12.0;
                let block_h = if t.note.is_empty() { 34.0 } else { 46.0 };
                let mut ty = cy - block_h / 2.0;
                let tr = widgets::text(ui, pos2(main_x, ty), &t.title, crate::fonts::medium(14.0), if is_current { p.accent } else { p.fg }, main_w);
                ty = tr.bottom() + 1.0;
                let ar = widgets::text(ui, pos2(main_x, ty), &t.artist, FontId::proportional(12.0), p.fg_dim, main_w);
                if !t.note.is_empty() {
                    let ny = ar.bottom() + 2.0;
                    icon::paint(ui, icons::NOTE, Rect::from_min_size(pos2(main_x, ny + 1.5), vec2(10.0, 10.0)), with_alpha(p.accent, 0.85));
                    widgets::text(ui, pos2(main_x + 14.0, ny), &t.note, FontId::proportional(11.0), with_alpha(p.accent, 0.85), main_w - 14.0);
                }
                widgets::text_v(ui, album_x, cy, &t.album, FontId::proportional(13.0), p.fg_dim, (time_r - album_x - 12.0).max(20.0));
                let time = format_time(t.duration);
                let tw = widgets::text_width(ui, &time, FontId::proportional(12.0));
                widgets::text_v(ui, time_r + 56.0 - tw, cy, &time, FontId::proportional(12.0), p.fg_dim, 60.0);
                let mr = ui.interact(more, Id::new(("more", i, t.id)), Sense::click());
                if resp.hovered() || mr.hovered() || is_current {
                    if mr.hovered() {
                        ui.painter().circle_filled(more.center(), 15.0, p.bg3_solid());
                    }
                    icon::paint_centered(ui, icons::MORE, more, 18.0, if mr.hovered() { p.fg } else { p.fg_dim });
                }
                if mr.on_hover_text("More…").clicked() {
                    menu = Some((t.clone(), more.left_bottom()));
                } else if resp.secondary_clicked() {
                    menu = Some((t.clone(), ui.input(|i| i.pointer.interact_pos()).unwrap_or(rect.center())));
                } else if resp.clicked() {
                    clicked = Some(i);
                }
                if reorderable && resp.drag_started() {
                    app.drag_row = Some(i);
                }
            }
        });
    if let Some((track, pos)) = menu {
        app.menu = Some(ContextMenu { track, pos, opened: Instant::now() });
    }
    if let Some(i) = clicked {
        if !app.edit_mode {
            app.play_tracks(&tracks, i);
        } else if selectable {
            let id = tracks[i].id;
            if !app.selected.remove(&id) {
                app.selected.insert(id);
            }
        }
    }
    if let (Some(from), Some(to), View::Album(name)) = (app.drag_row, drop_to, &view) {
        let mut ids: Vec<i64> = tracks.iter().map(|t| t.id).collect();
        let moved = ids.remove(from);
        ids.insert(to, moved);
        let _ = app.lib.db.reorder_album(name, &ids);
        app.library_changed();
        app.art.invalidate_album(name);
    }
    if ui.input(|i| i.pointer.any_released()) {
        app.drag_row = None;
    }

    if selectable && !app.selected.is_empty() {
        selection_bar(app, ui, list, &tracks);
    }
}

fn sort_control(app: &mut LumenApp, ui: &mut Ui, is_album: bool) {
    let p = app.p;
    let label = sort_label(app.sort.key);
    let font = crate::fonts::bold(13.0);
    let kw = widgets::text_width(ui, label, font.clone());
    let sw = widgets::text_width(ui, "Sort", FontId::proportional(13.0));
    let w = 14.0 + sw + 6.0 + kw + 18.0 + 30.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    ui.painter().rect(rect, 17.0, p.bg3_solid(), Stroke::new(1.0, p.border), StrokeKind::Inside);
    let main = Rect::from_min_max(rect.min, pos2(rect.right() - 32.0, rect.bottom()));
    let mr = ui.interact(main, Id::new("sort-main"), Sense::click());
    if mr.hovered() {
        ui.painter().rect_filled(main.shrink(2.0), 15.0, p.bg3);
    }
    let r = widgets::text_v(ui, main.left() + 14.0, rect.center().y, "Sort", FontId::proportional(13.0), p.fg_dim, sw + 2.0);
    let k = widgets::text_v(ui, r.right() + 6.0, rect.center().y, label, font, p.fg, kw + 2.0);
    ui.painter().circle_filled(pos2(k.right() + 7.0, rect.center().y), 1.5, p.fg_dim);
    let dir = Rect::from_center_size(pos2(rect.right() - 17.0, rect.center().y), vec2(26.0, 26.0));
    let dr = ui.interact(dir, Id::new("sort-dir"), Sense::click());
    ui.painter().circle_filled(dir.center(), 13.0, if dr.hovered() { p.border } else { p.bg2 });
    icon::paint_centered(ui, if app.sort.descending { icons::ARROW_DOWN } else { icons::ARROW_UP }, dir, 13.0, p.fg);
    if dr.on_hover_text(if app.sort.descending { "Descending" } else { "Ascending" }).clicked() {
        app.sort.descending = !app.sort.descending;
    }
    let main_clicked = mr.clicked();
    if mr.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        app.sort_menu_open = !app.sort_menu_open;
    }
    if app.sort_menu_open {
        let mut options: Vec<Sort> = Vec::new();
        if is_album {
            options.push(Sort::Custom);
        }
        options.extend(SORT_OPTIONS);
        let area = egui::Area::new(Id::new("sort-menu")).order(egui::Order::Foreground).fixed_pos(rect.left_bottom() + vec2(0.0, 6.0));
        let resp = area.show(ui.ctx(), |ui| {
            egui::Frame::new().fill(p.popover).stroke(Stroke::new(1.0, p.border)).corner_radius(10).inner_margin(6).show(ui, |ui| {
                ui.set_min_width(160.0);
                for o in options {
                    let active = app.sort.key == o;
                    let (r, resp) = ui.allocate_exact_size(vec2(160.0, 30.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 6.0, p.bg3);
                    }
                    widgets::text_v(ui, r.left() + 10.0, r.center().y, sort_label(o), FontId::proportional(13.0), if active { p.accent } else { p.fg }, 140.0);
                    if resp.clicked() {
                        app.sort = SortState { key: o, descending: false };
                        app.sort_menu_open = false;
                    }
                }
            });
        });
        if resp.response.clicked_elsewhere() && !main_clicked {
            app.sort_menu_open = false;
        }
    }
}

fn selection_bar(app: &mut LumenApp, ui: &mut Ui, list: Rect, tracks: &[Track]) {
    let p = app.p;
    let n = app.selected.len();
    let bar = Rect::from_center_size(pos2(list.center().x, list.bottom() - 36.0), vec2((list.width() - 64.0).min(640.0), 48.0));
    ui.painter().rect(bar, 24.0, p.popover, Stroke::new(1.0, p.border), StrokeKind::Inside);
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(14.0, 7.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    c.spacing_mut().item_spacing.x = 8.0;
    c.label(egui::RichText::new(format!("{} selected", pluralize(n as i64, "song"))).color(p.fg).size(13.0));
    c.add_space(6.0);
    let ids: Vec<i64> = tracks.iter().filter(|t| app.selected.contains(&t.id)).map(|t| t.id).collect();
    let pl = widgets::pill(&mut c, &p, "Add to playlist…", widgets::Pill::Neutral, None);
    if pl.clicked() {
        app.bulk_playlist_open = !app.bulk_playlist_open;
    }
    if widgets::pill(&mut c, &p, "Add to queue", widgets::Pill::Neutral, None).clicked() {
        let items = tracks.iter().filter(|t| app.selected.contains(&t.id)).map(item).collect();
        app.player.add_to_queue(items);
        app.selected.clear();
    }
    if widgets::pill(&mut c, &p, "Add to album…", widgets::Pill::Neutral, None).clicked() {
        app.dialog = Some(Dialog::AlbumPicker { tracks: ids.clone(), text: String::new() });
    }
    if widgets::pill(&mut c, &p, "Clear", widgets::Pill::Ghost, None).clicked() {
        app.selected.clear();
    }
    if app.bulk_playlist_open {
        let playlists = app.lib.playlists.clone();
        let area = egui::Area::new(Id::new("bulk-playlists")).order(egui::Order::Foreground).fixed_pos(pl.rect.left_top() - vec2(0.0, 8.0)).pivot(egui::Align2::LEFT_BOTTOM);
        area.show(ui.ctx(), |ui| {
            egui::Frame::new().fill(p.popover).stroke(Stroke::new(1.0, p.border)).corner_radius(10).inner_margin(6).show(ui, |ui| {
                ui.set_min_width(200.0);
                if playlists.is_empty() {
                    ui.label(egui::RichText::new("No playlists yet").color(p.fg_dim).size(12.0));
                }
                for pl in playlists {
                    let (r, resp) = ui.allocate_exact_size(vec2(200.0, 30.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 6.0, p.bg3);
                    }
                    widgets::text_v(ui, r.left() + 10.0, r.center().y, &pl.name, FontId::proportional(13.0), p.fg, 180.0);
                    if resp.clicked() {
                        let _ = app.lib.db.add_to_playlist(pl.id, &ids);
                        app.bulk_playlist_open = false;
                        app.selected.clear();
                        app.library_changed();
                    }
                }
            });
        });
    }
}

fn grid_header(app: &LumenApp, ui: &Ui, inner: Rect, title: &str, sub: &str) -> f32 {
    let p = app.p;
    let r = widgets::text(ui, inner.min, title, crate::fonts::bold(30.0), p.fg, inner.width());
    let s = widgets::text(ui, pos2(inner.left(), r.bottom() + 4.0), sub, FontId::proportional(13.0), p.fg_dim, inner.width());
    s.bottom() + 20.0
}

/// Columns for `repeat(auto-fill, minmax(150px, 1fr))` with an 18px gap.
fn grid_columns(width: f32) -> (usize, f32) {
    let gap = 18.0;
    let cols = (((width + gap) / (150.0 + gap)).floor() as usize).max(1);
    let w = (width - gap * (cols as f32 - 1.0)) / cols as f32;
    (cols, w)
}

fn artist_grid(app: &mut LumenApp, ui: &mut Ui, area: Rect) {
    let p = app.p;
    let inner = area.shrink2(PAD);
    let artists = app.lib.artists.clone();
    let top = grid_header(app, ui, inner, "Artists", &pluralize(artists.len() as i64, "artist"));
    let (cols, w) = grid_columns(inner.width());
    let card_h = w + 28.0 + 50.0;
    let rows = artists.len().div_ceil(cols);
    let list = Rect::from_min_max(pos2(area.left(), top), area.max);
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list));
    let mut open: Option<String> = None;
    let mut rename: Option<String> = None;
    egui::ScrollArea::vertical().id_salt("artists").auto_shrink([false, false]).show_viewport(&mut c, |ui, viewport| {
        ui.set_height(rows as f32 * (card_h + 18.0) + 24.0);
        let origin = ui.max_rect().min + vec2(PAD.x, 0.0);
        for (i, a) in artists.iter().enumerate() {
            let (row, col) = (i / cols, i % cols);
            let y = row as f32 * (card_h + 18.0);
            if y + card_h < viewport.min.y || y > viewport.max.y {
                continue;
            }
            let rect = Rect::from_min_size(origin + vec2(col as f32 * (w + 18.0), y), vec2(w, card_h));
            let resp = ui.interact(rect, Id::new(("artist", &a.artist)), Sense::click());
            ui.painter().rect_filled(rect, 10.0, if resp.hovered() { p.bg3 } else { p.bg2 });
            let circle = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(w - 28.0, w - 28.0));
            let (c1, c2) = widgets::seeded_gradient(&a.artist);
            ui.painter().circle_filled(circle.center(), circle.width() / 2.0, c1);
            ui.painter().circle_filled(circle.center() + vec2(circle.width() * 0.12, circle.width() * 0.12), circle.width() * 0.36, with_alpha(c2, 0.55));
            let initials: String = a.artist.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect();
            ui.painter().text(circle.center(), egui::Align2::CENTER_CENTER, initials, crate::fonts::bold(circle.width() * 0.28), Color32::from_white_alpha(217));
            widgets::text(ui, pos2(rect.left() + 14.0, circle.bottom() + 10.0), &a.artist, crate::fonts::medium(14.0), p.fg, w - 28.0);
            widgets::text(ui, pos2(rect.left() + 14.0, circle.bottom() + 30.0), &pluralize(a.count, "song"), FontId::proportional(12.0), p.fg_dim, w - 28.0);
            let edit = Rect::from_min_size(pos2(rect.right() - 36.0, rect.top() + 8.0), vec2(28.0, 28.0));
            let er = ui.interact(edit, Id::new(("artist-edit", &a.artist)), Sense::click());
            if resp.hovered() || er.hovered() {
                ui.painter().circle_filled(edit.center(), 14.0, Color32::from_black_alpha(140));
                icon::paint_centered(ui, icons::PENCIL, edit, 14.0, Color32::WHITE);
            }
            if er.on_hover_text("Rename artist").clicked() {
                rename = Some(a.artist.clone());
            } else if resp.clicked() {
                open = Some(a.artist.clone());
            }
        }
    });
    if let Some(n) = rename {
        app.dialog = Some(Dialog::Rename { kind: RenameKind::Artist(n.clone()), title: "Rename artist", label: "Renames this artist across every track", text: n.clone(), initial: n });
    } else if let Some(n) = open {
        app.set_view(View::Artist(n));
    }
}

fn album_grid(app: &mut LumenApp, ui: &mut Ui, area: Rect) {
    let p = app.p;
    let inner = area.shrink2(PAD);
    let albums = app.lib.albums.clone();
    let top = grid_header(app, ui, inner, "Albums", &pluralize(albums.len() as i64, "album"));
    let (cols, w) = grid_columns(inner.width());
    let card_h = w + 28.0 + 50.0;
    let rows = albums.len().div_ceil(cols);
    let list = Rect::from_min_max(pos2(area.left(), top), area.max);
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list));
    let mut open: Option<String> = None;
    let mut rename: Option<String> = None;
    let mut cover: Option<String> = None;
    egui::ScrollArea::vertical().id_salt("albums").auto_shrink([false, false]).show_viewport(&mut c, |ui, viewport| {
        ui.set_height(rows as f32 * (card_h + 18.0) + 24.0);
        let origin = ui.max_rect().min + vec2(PAD.x, 0.0);
        for (i, a) in albums.iter().enumerate() {
            let (row, col) = (i / cols, i % cols);
            let y = row as f32 * (card_h + 18.0);
            if y + card_h < viewport.min.y || y > viewport.max.y {
                continue;
            }
            let rect = Rect::from_min_size(origin + vec2(col as f32 * (w + 18.0), y), vec2(w, card_h));
            let resp = ui.interact(rect, Id::new(("album", &a.album)), Sense::click());
            ui.painter().rect_filled(rect, 10.0, if resp.hovered() { p.bg3 } else { p.bg2 });
            let art = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(w - 28.0, w - 28.0));
            widgets::artwork(ui, &p, &mut app.art, &ArtKey::Album(a.album.clone()), art, 6.0);
            widgets::text(ui, pos2(rect.left() + 14.0, art.bottom() + 10.0), &a.album, crate::fonts::medium(14.0), p.fg, w - 28.0);
            widgets::text(ui, pos2(rect.left() + 14.0, art.bottom() + 30.0), &a.artist, FontId::proportional(12.0), p.fg_dim, w - 28.0);
            let edit = Rect::from_min_size(pos2(rect.right() - 36.0, rect.top() + 8.0), vec2(28.0, 28.0));
            let img = Rect::from_min_size(pos2(art.right() - 30.0, art.bottom() - 30.0), vec2(26.0, 26.0));
            let er = ui.interact(edit, Id::new(("album-edit", &a.album)), Sense::click());
            let ir = ui.interact(img, Id::new(("album-img", &a.album)), Sense::click());
            if resp.hovered() || er.hovered() || ir.hovered() {
                ui.painter().circle_filled(edit.center(), 14.0, Color32::from_black_alpha(140));
                icon::paint_centered(ui, icons::PENCIL, edit, 14.0, Color32::WHITE);
                ui.painter().rect_filled(img, 6.0, Color32::from_black_alpha(160));
                icon::paint_centered(ui, icons::IMAGE, img, 14.0, Color32::WHITE);
            }
            if er.on_hover_text("Rename album").clicked() {
                rename = Some(a.album.clone());
            } else if ir.on_hover_text("Change cover").clicked() {
                cover = Some(a.album.clone());
            } else if resp.clicked() {
                open = Some(a.album.clone());
            }
        }
    });
    if let Some(n) = rename {
        app.dialog = Some(Dialog::Rename { kind: RenameKind::Album(n.clone()), title: "Rename album", label: "Renames this album across every track", text: n.clone(), initial: n });
    } else if let Some(n) = cover {
        app.dialog = Some(Dialog::Cover { album: n });
    } else if let Some(n) = open {
        app.set_view(View::Album(n));
    }
}
