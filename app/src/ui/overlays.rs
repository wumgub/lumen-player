//! Everything that floats over the window: the queue panel, a song's context
//! menu, the theme menu, the dialogs, the immersive visualizer and the music
//! video filling the window.

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, FontId, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lumen_core::fold::fold;

use crate::app::{Dialog, LumenApp, RenameKind, SongEdit};
use crate::art::ArtKey;
use crate::library::View;
use crate::palette::with_alpha;
use crate::ui::{TRANSPORT_H, format_time, icon, icons, nowplaying, pluralize, settings, sidebar, theme_editor, widgets};

pub fn show(app: &mut LumenApp, ui: &mut Ui, frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();
    if app.queue_open {
        queue_panel(app, &ctx);
    }
    if app.theme_menu_open {
        theme_menu(app, &ctx);
    }
    if app.menu.is_some() {
        context_menu(app, &ctx);
    }
    if app.dialog.is_some() {
        dialog(app, &ctx);
    }
    if app.settings_open {
        settings::show(app, &ctx);
    }
    if app.theme_editor_open {
        theme_editor::show(app, &ctx);
    }
    if app.prefs.debug_overlay {
        debug_overlay(app, &ctx);
    }
    let _ = frame;
}

fn popup_frame(app: &LumenApp) -> egui::Frame {
    egui::Frame::new()
        .fill(app.p.popover)
        .stroke(Stroke::new(1.0, app.p.border))
        .corner_radius(12)
        .inner_margin(6)
        .shadow(egui::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(110) })
}

fn menu_item(ui: &mut Ui, p: &crate::palette::Palette, label: &str, width: f32, danger: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 6.0, p.bg3);
    }
    let color = if danger { Color32::from_rgb(0xe0, 0x60, 0x7e) } else { p.fg };
    widgets::text_v(ui, r.left() + 10.0, r.center().y, label, FontId::proportional(13.0), color, width - 20.0);
    resp
}

fn queue_panel(app: &mut LumenApp, ctx: &egui::Context) {
    let p = app.p;
    let screen = ctx.content_rect();
    let w = 340.0f32.min(screen.width() - 28.0);
    let h = (screen.height() * 0.6).min(520.0);
    let rect = Rect::from_min_size(pos2(screen.right() - 14.0 - w, screen.bottom() - TRANSPORT_H - 12.0 - h), vec2(w, h));
    let snap = app.snap.clone();
    let mut jump = None;
    let mut remove = None;
    let mut clear = false;
    let resp = egui::Area::new(Id::unique("queue")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
        popup_frame(app).inner_margin(0).show(ui, |ui| {
            ui.set_width(w);
            ui.set_height(h);
            let head = Rect::from_min_size(ui.max_rect().min, vec2(w, 58.0));
            widgets::text(ui, head.min + vec2(16.0, 12.0), "Queue", crate::fonts::bold(15.0), p.fg, 200.0);
            let upcoming = snap.index.map(|i| snap.queue.len().saturating_sub(i + 1)).unwrap_or(snap.queue.len());
            let sub = if snap.queue.is_empty() { "Empty".to_owned() } else { format!("{} up next", pluralize(upcoming as i64, "song")) };
            widgets::text(ui, head.min + vec2(16.0, 32.0), &sub, FontId::proportional(12.0), p.fg_dim, 200.0);
            if upcoming > 0 && snap.index.is_some() {
                let b = Rect::from_min_size(pos2(head.right() - 128.0, head.top() + 14.0), vec2(114.0, 28.0));
                let br = ui.interact(b, Id::unique("queue-clear"), Sense::click());
                ui.painter().rect_filled(b, 14.0, if br.hovered() { p.bg3 } else { Color32::TRANSPARENT });
                widgets::centered_text(ui, b, "Clear upcoming", FontId::proportional(12.0), p.accent);
                if br.clicked() {
                    clear = true;
                }
            }
            ui.painter().hline(head.x_range(), head.bottom(), Stroke::new(1.0, p.border));
            let list = Rect::from_min_max(pos2(head.left(), head.bottom() + 4.0), ui.max_rect().max);
            let mut c = ui.new_child(egui::UiBuilder::new().max_rect(list));
            if snap.queue.is_empty() {
                widgets::centered_text(&c, list, "Nothing queued. Use “Play next” or “Add to queue” on any track.", FontId::proportional(12.0), p.fg_dim);
                return;
            }
            egui::ScrollArea::vertical().id_salt("queue-scroll").auto_shrink([false, false]).show_rows(&mut c, 48.0, snap.queue.len(), |ui, range| {
                for i in range {
                    let it = &snap.queue[i];
                    let (r, resp) = ui.allocate_exact_size(vec2(w - 8.0, 48.0), Sense::click());
                    let current = snap.index == Some(i);
                    if current {
                        ui.painter().rect_filled(r.shrink2(vec2(4.0, 1.0)), 8.0, p.accent_soft);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r.shrink2(vec2(4.0, 1.0)), 8.0, p.bg3);
                    }
                    let art = Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - 17.0), vec2(34.0, 34.0));
                    widgets::artwork(ui, &p, &mut app.art, &ArtKey::Track(it.track_id), art, 5.0);
                    let t = app.lib.db.track(it.track_id).ok().flatten();
                    let (title, artist) = t.map(|t| (t.title, t.artist)).unwrap_or_default();
                    let played = snap.index.is_some_and(|c| i < c);
                    let tc = if current { p.accent } else if played { p.fg_dim } else { p.fg };
                    widgets::text(ui, pos2(art.right() + 10.0, r.top() + 7.0), &title, crate::fonts::medium(13.0), tc, r.width() - 140.0);
                    widgets::text(ui, pos2(art.right() + 10.0, r.top() + 26.0), &artist, FontId::proportional(11.0), p.fg_dim, r.width() - 140.0);
                    let time = format_time(it.duration);
                    ui.painter().text(pos2(r.right() - 44.0, r.center().y), egui::Align2::RIGHT_CENTER, time, FontId::proportional(11.0), p.fg_dim);
                    let x = Rect::from_center_size(pos2(r.right() - 22.0, r.center().y), vec2(24.0, 24.0));
                    let xr = ui.interact(x, Id::unique(("q-remove", i)), Sense::click());
                    if resp.hovered() || xr.hovered() {
                        icon::paint_centered(ui, icons::CLOSE, x, 13.0, if xr.hovered() { p.fg } else { p.fg_dim });
                    }
                    if xr.on_hover_text("Remove from queue").clicked() {
                        remove = Some(i);
                    } else if resp.clicked() {
                        jump = Some(i);
                    }
                }
            });
        });
    });
    if let Some(i) = jump {
        app.player.jump_to(i);
    }
    if let Some(i) = remove {
        app.player.remove(i);
    }
    if clear {
        app.player.clear_upcoming();
    }
    // Close on a click outside (but not the click on the button that opened it).
    if resp.response.clicked_elsewhere() && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|pos| pos.y < screen.bottom() - TRANSPORT_H) {
        app.queue_open = false;
    }
}

fn theme_menu(app: &mut LumenApp, ctx: &egui::Context) {
    let p = app.p;
    let at = app.theme_pill_rect.left_bottom() + vec2(0.0, 6.0);
    let mut choose: Option<&'static str> = None;
    let resp = egui::Area::new(Id::unique("theme-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        popup_frame(app).show(ui, |ui| {
            ui.set_min_width(220.0);
            for t in lumen_core::theme::THEMES.iter() {
                let (r, resp) = ui.allocate_exact_size(vec2(220.0, 42.0), Sense::click());
                let active = app.prefs.theme == t.id;
                if active {
                    ui.painter().rect_filled(r, 8.0, p.accent_soft);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 8.0, p.bg3);
                }
                let tokens = if t.id == "custom" { app.prefs.custom.clone() } else { lumen_core::theme::preset(t.id) };
                let sw = crate::palette::Palette::resolve(t.id, &tokens, Default::default(), false);
                let c = pos2(r.left() + 20.0, r.center().y);
                ui.painter().circle_filled(c, 10.0, sw.bg);
                ui.painter().circle_filled(c + vec2(-3.0, -3.0), 5.0, sw.accent);
                ui.painter().circle_filled(c + vec2(3.5, 3.5), 4.0, sw.accent2);
                ui.painter().circle_stroke(c, 10.0, Stroke::new(1.0, p.border));
                widgets::text(ui, pos2(r.left() + 40.0, r.top() + 5.0), t.label, crate::fonts::medium(13.0), p.fg, 170.0);
                widgets::text(ui, pos2(r.left() + 40.0, r.top() + 22.0), t.hint, FontId::proportional(11.0), p.fg_dim, 170.0);
                if resp.clicked() {
                    choose = Some(t.id);
                }
            }
        });
    });
    if let Some(id) = choose {
        app.prefs.theme = id.into();
        app.theme_menu_open = false;
        if id == "custom" {
            app.theme_editor_open = true;
        }
    } else if resp.response.clicked_elsewhere() && !ctx.input(|i| i.pointer.interact_pos().is_some_and(|pos| app.theme_pill_rect.contains(pos))) {
        app.theme_menu_open = false;
    }
}

fn context_menu(app: &mut LumenApp, ctx: &egui::Context) {
    let Some(menu) = app.menu.as_ref() else { return };
    let track = menu.track.clone();
    let pos = menu.pos;
    let opened = menu.opened;
    let in_playlist = if let View::Playlist(id, _) = &app.view { Some(*id) } else { None };
    let playlists = app.lib.playlists.clone();
    let width = 210.0;
    let screen = ctx.content_rect();
    let est_h = 300.0 + playlists.len().min(8) as f32 * 30.0;
    let at = pos2(pos.x.min(screen.right() - width - 20.0), pos.y.min(screen.bottom() - est_h).max(screen.top() + 8.0));
    let mut close = false;
    let pp = app.p;
    let resp = egui::Area::new(Id::unique("ctx-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        popup_frame(app).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if menu_item(ui, &pp, "Play next", width, false).clicked() {
                app.player.play_next(vec![crate::app::item(&track)]);
                close = true;
            }
            if menu_item(ui, &pp, "Add to queue", width, false).clicked() {
                app.player.add_to_queue(vec![crate::app::item(&track)]);
                close = true;
            }
            widgets::subtle_separator(ui, &app.p);
            if menu_item(ui, &pp, "Edit song…", width, false).clicked() {
                app.dialog = Some(Dialog::EditSong(Box::new(SongEdit::new(&track))));
                close = true;
            }
            if menu_item(ui, &pp, reveal_label(), width, false).clicked() {
                reveal(&track.path);
                close = true;
            }
            if !track.album.is_empty() && menu_item(ui, &pp, "Remove from album", width, false).clicked() {
                let _ = app.lib.db.set_track_album(&[track.id], "");
                app.library_changed();
                close = true;
            }
            if let Some(pl) = in_playlist
                && menu_item(ui, &pp, "Remove from playlist", width, true).clicked()
            {
                let _ = app.lib.db.remove_from_playlist(pl, track.id);
                app.library_changed();
                close = true;
            }
            widgets::subtle_separator(ui, &app.p);
            let (r, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
            widgets::text_v(ui, r.left() + 10.0, r.center().y, "ADD TO PLAYLIST", FontId::proportional(10.0), app.p.fg_dim, width);
            if playlists.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
                widgets::text_v(ui, r.left() + 10.0, r.center().y, "No playlists yet", FontId::proportional(12.0), app.p.fg_dim, width);
            }
            egui::ScrollArea::vertical().id_salt("ctx-playlists").max_height(240.0).show(ui, |ui| {
                for pl in &playlists {
                    if menu_item(ui, &pp, &pl.name, width, false).clicked() {
                        let _ = app.lib.db.add_to_playlist(pl.id, &[track.id]);
                        app.library_changed();
                        close = true;
                    }
                }
            });
        });
    });
    let fresh = opened.elapsed() < Duration::from_millis(150);
    if close || (!fresh && (resp.response.clicked_elsewhere() || ctx.input(|i| i.pointer.secondary_clicked()) && !resp.response.contains_pointer())) {
        app.menu = None;
    }
    let _ = Instant::now();
}

fn reveal_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Show in Finder"
    } else if cfg!(target_os = "windows") {
        "Show in Explorer"
    } else {
        "Show file"
    }
}

/// Open the folder holding `path`, with the file selected where the OS can.
pub fn reveal(path: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{path}")).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}

/// A centred modal with a title, used by every dialog.
fn modal<R>(p: crate::palette::Palette, ctx: &egui::Context, id: &str, width: f32, body: impl FnOnce(&mut Ui) -> R) -> (R, bool) {
    let m = egui::Modal::new(Id::unique(id))
        .backdrop_color(Color32::from_black_alpha(140))
        .frame(egui::Frame::new().fill(p.popover).stroke(Stroke::new(1.0, p.border)).corner_radius(14).inner_margin(22))
        .show(ctx, |ui| {
            ui.set_width(width);
            body(ui)
        });
    let dismissed = m.should_close();
    (m.inner, dismissed)
}

fn title(ui: &mut Ui, p: &crate::palette::Palette, s: &str) {
    ui.label(egui::RichText::new(s).font(crate::fonts::bold(17.0)).color(p.fg));
    ui.add_space(4.0);
}

fn label(ui: &mut Ui, p: &crate::palette::Palette, s: &str) {
    ui.label(egui::RichText::new(s).size(13.0).color(p.fg_dim));
    ui.add_space(8.0);
}

/// Cancel / primary buttons at the bottom right. Returns (cancel, primary).
fn actions(ui: &mut Ui, p: &crate::palette::Palette, primary: &str) -> (bool, bool) {
    ui.add_space(14.0);
    let mut out = (false, false);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        out.1 = widgets::pill(ui, p, primary, widgets::Pill::Active, None).clicked();
        out.0 = widgets::pill(ui, p, "Cancel", widgets::Pill::Ghost, None).clicked();
    });
    out
}

fn field(ui: &mut Ui, p: &crate::palette::Palette, text: &mut String, hint: &str, id: &str) -> egui::Response {
    let r = widgets::input_frame(p)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text(hint)
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .font(FontId::proportional(14.0))
                    .text_color(p.fg)
                    .id(Id::unique(id)),
            )
        })
        .inner;
    if !r.has_focus() && ui.memory(|m| m.focused().is_none()) {
        r.request_focus();
    }
    r
}

fn small_label(ui: &mut Ui, p: &crate::palette::Palette, s: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(s).size(12.0).color(p.fg_dim));
    ui.add_space(2.0);
}

/// Write only the fields that changed, so untouched ones stay unlocked and
/// keep following the file's tags on a rescan.
fn save_song_edit(app: &mut LumenApp, e: &SongEdit) {
    let t = &e.track;
    let db = &app.lib.db;
    let title = e.title.trim();
    if !title.is_empty() && title != t.title {
        let _ = db.rename_track(t.id, title);
    }
    // Same placeholder the scanner gives untagged files.
    let artist = match e.artist.trim() {
        "" => "Unknown Artist",
        a => a,
    };
    if artist != t.artist {
        let _ = db.set_track_artist(t.id, artist);
    }
    let album = e.album.trim();
    if album != t.album {
        let _ = db.set_track_album(&[t.id], album);
        app.art.invalidate_album(&t.album);
        app.art.invalidate_album(album);
    }
    if e.note.trim() != t.note {
        let _ = db.set_track_note(t.id, e.note.trim());
    }
    app.library_changed();
}

fn dialog(app: &mut LumenApp, ctx: &egui::Context) {
    let Some(mut d) = app.dialog.take() else { return };
    let pp = app.p;
    let enter = ctx.input(|i| i.key_pressed(Key::Enter));
    let mut keep = true;
    match &mut d {
        Dialog::Rename { kind, title: t, label: l, text, initial } => {
            let (submit, dismissed) = modal(app.p, ctx, "rename", 360.0, |ui| {
                title(ui, &pp, t);
                label(ui, &pp, l);
                field(ui, &pp, text, "", "rename-field");
                let (cancel, save) = actions(ui, &pp, "Save");
                if cancel {
                    keep = false;
                }
                save || enter
            });
            if dismissed {
                keep = false;
            }
            if submit {
                let v = text.trim().to_owned();
                if !v.is_empty() && v != *initial {
                    match kind {
                        RenameKind::Artist(old) => {
                            let _ = app.lib.db.rename_artist(old, &v);
                            if app.view == View::Artist(old.clone()) {
                                app.view = View::Artist(v.clone());
                            }
                        }
                        RenameKind::Album(old) => {
                            let _ = app.lib.db.rename_album(old, &v);
                            app.art.invalidate_album(old);
                            app.art.invalidate_album(&v);
                            if app.view == View::Album(old.clone()) {
                                app.view = View::Album(v.clone());
                            }
                        }
                        RenameKind::Playlist(id) => {
                            let _ = app.lib.db.rename_playlist(*id, &v);
                            if let View::Playlist(pid, _) = &app.view
                                && pid == id
                            {
                                app.view = View::Playlist(*id, v.clone());
                            }
                        }
                    }
                    app.library_changed();
                }
                keep = false;
            }
        }
        Dialog::NewPlaylist { text, placeholder, add } => {
            let (submit, dismissed) = modal(app.p, ctx, "new-playlist", 360.0, |ui| {
                title(ui, &pp, "New playlist");
                label(ui, &pp, "Give your playlist a name");
                field(ui, &pp, text, placeholder, "playlist-field");
                let (cancel, create) = actions(ui, &pp, "Create");
                if cancel {
                    keep = false;
                }
                create || enter
            });
            if dismissed {
                keep = false;
            }
            if submit && !text.trim().is_empty() {
                if let Ok(id) = app.lib.db.create_playlist(text.trim()) {
                    if !add.is_empty() {
                        let _ = app.lib.db.add_to_playlist(id, add);
                    }
                }
                app.library_changed();
                keep = false;
            }
        }
        Dialog::EditSong(e) => {
            let p = app.p;
            let albums: Vec<String> = app.lib.albums.iter().map(|a| a.album.clone()).collect();
            let mut note_focused = false;
            let (submit, dismissed) = modal(app.p, ctx, "edit-song", 400.0, |ui| {
                title(ui, &pp, "Edit song");
                label(ui, &pp, "Changes only this song, and only in Lumen; the file stays as it is.");
                small_label(ui, &p, "Title");
                field(ui, &pp, &mut e.title, &e.track.title, "edit-title");
                small_label(ui, &p, "Artist");
                field(ui, &pp, &mut e.artist, "Unknown Artist", "edit-artist");
                small_label(ui, &p, "Album");
                field(ui, &pp, &mut e.album, "No album", "edit-album");
                // Existing albums that match what's typed, to pick instead of retyping.
                let q = fold(e.album.trim());
                if !q.is_empty() && e.album.trim() != e.track.album && !albums.iter().any(|a| a == e.album.trim()) {
                    for a in albums.iter().filter(|a| fold(a).contains(&q)).take(5) {
                        if menu_item(ui, &pp, a, 400.0, false).clicked() {
                            e.album = a.clone();
                        }
                    }
                }
                small_label(ui, &p, "Note");
                widgets::input_frame(&p).show(ui, |ui| {
                    let r = ui.add(
                        egui::TextEdit::multiline(&mut e.note)
                            .hint_text("Searchable: a romanized title, keywords…")
                            .desired_rows(2)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .text_color(p.fg)
                            .id(Id::unique("edit-note")),
                    );
                    note_focused = r.has_focus();
                });
                let (cancel, save) = actions(ui, &pp, "Save");
                if cancel {
                    keep = false;
                }
                save || (enter && !note_focused) || ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.command)
            });
            if dismissed {
                keep = false;
            }
            if submit {
                save_song_edit(app, e);
                keep = false;
            }
        }
        Dialog::AlbumPicker { tracks, text } => {
            let albums: Vec<String> = app.lib.albums.iter().map(|a| a.album.clone()).collect();
            let q = fold(text.trim());
            let matches: Vec<String> = albums.iter().filter(|a| q.is_empty() || fold(a).contains(&q)).take(60).cloned().collect();
            let exact = albums.iter().any(|a| a == text.trim());
            let p = app.p;
            let mut chosen: Option<String> = None;
            let (_, dismissed) = modal(app.p, ctx, "album-picker", 380.0, |ui| {
                title(ui, &pp, if tracks.len() == 1 { "Add to album" } else { "Add songs to album" });
                label(ui, &pp, "Search for an album or type a new name");
                field(ui, &pp, text, "Album name…", "album-field");
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    if !text.trim().is_empty() && !exact {
                        let r = menu_item(ui, &pp, &format!("Create album \"{}\"", text.trim()), 370.0, false);
                        if r.clicked() || enter {
                            chosen = Some(text.trim().to_owned());
                        }
                    }
                    for a in &matches {
                        if menu_item(ui, &pp, a, 370.0, false).clicked() {
                            chosen = Some(a.clone());
                        }
                    }
                    if matches.is_empty() && text.trim().is_empty() {
                        ui.label(egui::RichText::new("No albums yet").color(p.fg_dim).size(12.0));
                    }
                });
                let (cancel, _) = actions(ui, &pp, "Done");
                if cancel {
                    keep = false;
                }
            });
            if dismissed {
                keep = false;
            }
            if let Some(name) = chosen {
                let _ = app.lib.db.set_track_album(tracks, &name);
                app.selected.clear();
                app.art.invalidate_album(&name);
                app.library_changed();
                keep = false;
            }
        }
        Dialog::Cover { album } => {
            let has = app.lib.db.album_has_cover(album).unwrap_or(false);
            let album_name = album.clone();
            let mut next: Option<Dialog> = None;
            let (_, dismissed) = modal(app.p, ctx, "cover", 320.0, |ui| {
                title(ui, &pp, "Album cover");
                label(ui, &pp, &album_name);
                if menu_item(ui, &pp, "Custom image…", 320.0, false).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Choose a cover image")
                        .add_filter("Images", &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
                        .pick_file()
                        && let Ok(bytes) = std::fs::read(&path)
                    {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_ascii_lowercase();
                        let mime = match ext.as_str() {
                            "png" => "image/png",
                            "webp" => "image/webp",
                            "gif" => "image/gif",
                            "bmp" => "image/bmp",
                            _ => "image/jpeg",
                        };
                        let _ = app.lib.db.set_album_cover_custom(&album_name, mime, &bytes);
                    }
                    keep = false;
                }
                if menu_item(ui, &pp, "Choose from a song…", 320.0, false).clicked() {
                    next = Some(Dialog::CoverFromSong { album: album_name.clone(), text: String::new() });
                }
                if has && menu_item(ui, &pp, "Reset to default", 320.0, false).clicked() {
                    let _ = app.lib.db.clear_album_cover(&album_name);
                    keep = false;
                }
                let (cancel, _) = actions(ui, &pp, "Done");
                if cancel {
                    keep = false;
                }
            });
            if dismissed {
                keep = false;
            }
            if !keep {
                app.art.invalidate_album(&album_name);
            }
            if let Some(n) = next {
                app.dialog = Some(n);
                return;
            }
        }
        Dialog::CoverFromSong { album, text } => {
            let p = app.p;
            let all = app.lib.all_tracks();
            let q = fold(text.trim());
            let matches: Vec<_> = all
                .iter()
                .filter(|t| t.has_art && (q.is_empty() || fold(&t.title).contains(&q) || fold(&t.artist).contains(&q) || fold(&t.album).contains(&q)))
                .take(100)
                .cloned()
                .collect();
            let mut chosen: Option<i64> = None;
            let (_, dismissed) = modal(app.p, ctx, "cover-song", 400.0, |ui| {
                title(ui, &pp, "Use art from a song");
                field(ui, &pp, text, "Search by title, artist or album", "cover-song-field");
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for t in &matches {
                        let (r, resp) = ui.allocate_exact_size(vec2(390.0, 46.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, 6.0, p.bg3);
                        }
                        let art = Rect::from_min_size(pos2(r.left() + 6.0, r.top() + 5.0), vec2(36.0, 36.0));
                        widgets::artwork(ui, &p, &mut app.art, &ArtKey::Track(t.id), art, 5.0);
                        widgets::text(ui, pos2(art.right() + 10.0, r.top() + 6.0), &t.title, crate::fonts::medium(13.0), p.fg, 320.0);
                        widgets::text(ui, pos2(art.right() + 10.0, r.top() + 25.0), &format!("{} · {}", t.artist, t.album), FontId::proportional(11.0), p.fg_dim, 320.0);
                        if resp.clicked() {
                            chosen = Some(t.id);
                        }
                    }
                });
                let (cancel, _) = actions(ui, &pp, "Done");
                if cancel {
                    keep = false;
                }
            });
            if dismissed {
                keep = false;
            }
            if let Some(id) = chosen {
                let _ = app.lib.db.set_album_cover_from_track(album, id);
                app.art.invalidate_album(album);
                keep = false;
            }
        }
        Dialog::DeletePlaylist { id, name } => {
            let (yes, dismissed) = modal(app.p, ctx, "delete-playlist", 340.0, |ui| {
                title(ui, &pp, "Delete playlist");
                label(ui, &pp, &format!("Delete \"{name}\"? The songs stay in your library."));
                let (cancel, del) = actions(ui, &pp, "Delete");
                if cancel {
                    keep = false;
                }
                del
            });
            if dismissed {
                keep = false;
            }
            if yes {
                let _ = app.lib.db.delete_playlist(*id);
                if matches!(&app.view, View::Playlist(pid, _) if pid == id) {
                    app.set_view(View::Songs);
                }
                app.library_changed();
                keep = false;
            }
        }
    }
    if keep && app.dialog.is_none() {
        app.dialog = Some(d);
    }
}

/// The visualizer filling the window, with the song and controls over it.
pub fn immersive(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let screen = ui.max_rect();
    ui.painter().rect_filled(screen, 0.0, p.viz_bg);
    nowplaying::visualizer_card(app, ui, screen, false);
    overlay_controls(app, ui, screen, false);
    let ex = Rect::from_min_size(pos2(screen.right() - 52.0, screen.top() + 16.0), vec2(36.0, 36.0));
    let er = ui.interact(ex, Id::unique("immersive-exit"), Sense::click());
    ui.painter().rect_filled(ex, 10.0, Color32::from_black_alpha(if er.hovered() { 160 } else { 100 }));
    icon::paint_centered(ui, icons::COLLAPSE, ex, 20.0, Color32::WHITE);
    if er.on_hover_text("Exit (Esc)").clicked() {
        app.immersive = false;
    }
}

/// The music video filling the window; controls fade out while the mouse rests.
pub fn video_full(app: &mut LumenApp, ui: &mut Ui, frame: &mut eframe::Frame) {
    let screen = ui.max_rect();
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    if let Some(v) = app.video.as_mut() {
        v.show(ui, frame, screen);
    }
    let moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO || i.pointer.any_pressed());
    if moved {
        app.idle_since = Instant::now();
    }
    let idle = app.idle_since.elapsed() > Duration::from_millis(2500);
    let stage = ui.interact(screen, Id::unique("video-stage"), Sense::click());
    if stage.double_clicked() {
        app.video_full = false;
        return;
    }
    if !idle {
        overlay_controls(app, ui, screen, true);
        let ex = Rect::from_min_size(pos2(screen.right() - 52.0, screen.top() + 16.0), vec2(36.0, 36.0));
        let er = ui.interact(ex, Id::unique("video-exit"), Sense::click());
        ui.painter().rect_filled(ex, 10.0, Color32::from_black_alpha(if er.hovered() { 160 } else { 100 }));
        icon::paint_centered(ui, icons::COLLAPSE, ex, 20.0, Color32::WHITE);
        if er.on_hover_text("Exit (Esc)").clicked() {
            app.video_full = false;
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    } else {
        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
    }
}

fn overlay_controls(app: &mut LumenApp, ui: &mut Ui, screen: Rect, with_seek: bool) {
    let p = app.p;
    let h = if with_seek { 150.0 } else { 120.0 };
    let band = Rect::from_min_max(pos2(screen.left(), screen.bottom() - h), screen.max);
    // Fade from transparent to dark, so the text reads over anything.
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(band.left_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(band.right_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(band.right_bottom(), Color32::from_black_alpha(170));
    mesh.colored_vertex(band.left_bottom(), Color32::from_black_alpha(170));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
    let t = app.current_track();
    let y = band.bottom() - if with_seek { 96.0 } else { 70.0 };
    widgets::text(ui, pos2(band.left() + 32.0, y), t.as_ref().map(|t| t.title.as_str()).unwrap_or("Nothing playing"), crate::fonts::bold(22.0), Color32::WHITE, band.width() / 2.0);
    widgets::text(ui, pos2(band.left() + 32.0, y + 30.0), t.as_ref().map(|t| t.artist.as_str()).unwrap_or(""), FontId::proportional(14.0), Color32::from_white_alpha(190), band.width() / 2.0);
    let controls = Rect::from_center_size(pos2(band.center().x, y + 22.0), vec2(170.0, 56.0));
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(controls).layout(egui::Layout::left_to_right(egui::Align::Center)));
    c.spacing_mut().item_spacing.x = 14.0;
    let mut light = p;
    light.fg = Color32::WHITE;
    light.fg_dim = Color32::from_white_alpha(200);
    light.bg3 = Color32::from_white_alpha(30);
    light.bg = Color32::from_rgb(10, 10, 14);
    if widgets::icon_button(&mut c, &light, icons::PREV, 44.0, 26.0, false).clicked() {
        app.player.prev();
    }
    if widgets::play_button(&mut c, &light, !app.snap.paused, 54.0, 28.0).clicked() {
        app.player.toggle();
    }
    if widgets::icon_button(&mut c, &light, icons::NEXT, 44.0, 26.0, false).clicked() {
        app.player.next();
    }
    if with_seek {
        let sr = Rect::from_min_max(pos2(band.left() + 80.0, band.bottom() - 34.0), pos2(band.right() - 80.0, band.bottom() - 18.0));
        let mut s = ui.new_child(egui::UiBuilder::new().max_rect(sr));
        crate::ui::transport::seek_bar(app, &mut s, sr.width(), Id::unique("video-seek"));
        let shown = app.seek_drag.unwrap_or(app.snap.position);
        ui.painter().text(pos2(sr.left() - 12.0, sr.center().y), egui::Align2::RIGHT_CENTER, format_time(shown), FontId::proportional(11.0), Color32::from_white_alpha(200));
        ui.painter().text(pos2(sr.right() + 12.0, sr.center().y), egui::Align2::LEFT_CENTER, format_time(app.snap.duration), FontId::proportional(11.0), Color32::from_white_alpha(200));
    }
    let _ = (with_alpha(p.fg, 1.0), StrokeKind::Inside, sidebar::random_playlist_name);
}

/// Settings → Debug overlay: CPU, GPU and memory in one line, at the right
/// end of the title bar (top-left in the full-window views, where the exit
/// button has that corner). Drawn over everything, and clicks pass through.
fn debug_overlay(app: &LumenApp, ctx: &egui::Context) {
    let p = app.p;
    let text = match *app.perf.usage.lock() {
        Some(u) => {
            let gpu = u.gpu.map_or("n/a".to_owned(), |g| format!("{g:.1}%"));
            format!("CPU {:.1}%  GPU {gpu}  MEM {}", u.cpu, crate::perf::format_bytes(u.mem_bytes))
        }
        None => "CPU -  GPU -  MEM -".to_owned(),
    };
    let full = app.immersive || app.video_full;
    let (align, offset) = if full {
        (egui::Align2::LEFT_TOP, vec2(16.0, 22.0))
    } else {
        (egui::Align2::RIGHT_TOP, vec2(-14.0, (crate::ui::TITLEBAR_H - 24.0) / 2.0))
    };
    egui::Area::new(Id::unique("debug-overlay"))
        .anchor(align, offset)
        .order(egui::Order::Tooltip)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(150))
                .stroke(Stroke::new(1.0, with_alpha(p.border, 0.8)))
                .corner_radius(12)
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(text).font(FontId::monospace(11.0)).color(Color32::from_gray(225))).extend());
                });
        });
}
