//! The sidebar: library views, playlists you can drag into
//! order, music folders, and the rescan button with its progress.

use eframe::egui::{self, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use lumen_core::scanner::Phase;

use crate::app::{Dialog, LumenApp, RenameKind};
use crate::library::View;
use crate::ui::{icon, icons, pluralize, widgets};

const PLAYLIST_NAMES: &[&str] = &[
    "Late night coding",
    "Rainy day melancholy",
    "Sunday morning coffee",
    "Gym beast mode",
    "Deep focus",
    "Road trip anthems",
    "3am thoughts",
    "Cleaning the apartment",
    "Pre-game hype",
    "Slow dance with the ghosts",
    "Cold wave winter",
    "Studying for finals",
    "Heartbreak on repeat",
    "Golden hour drive",
    "Shower concert",
    "Doomscrolling at midnight",
    "Cooking dinner for one",
    "Main character energy",
    "Soft launch of spring",
    "Existential dread (upbeat)",
    "Running in the rain",
    "Lo-fi homework grind",
    "Friday night out",
    "Hungover Sunday",
    "Staring out the train window",
    "Villain origin story",
    "Dancing alone in the kitchen",
    "Melancholy with a beat",
    "Productivity sprint",
    "Chill but make it sad",
    "Neon city nights",
    "Desert highway",
    "First snow",
    "Beach bonfire",
    "Writing a novel that doesn’t exist",
    "Overthinking everything",
    "Monday morning dread",
    "Weekend reset",
    "Falling asleep slowly",
    "Vibing in traffic",
    "Breakup glow-up",
    "Analog summer",
    "Ghosts of the dancefloor",
    "Indie coffee shop",
    "Thunderstorm study",
    "Driving nowhere at 2am",
    "Pretending it’s the 80s",
    "Lonely astronaut",
    "Post-rock sunrise",
    "Quiet confidence",
];

pub fn random_playlist_name() -> String {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("e.g. {}", PLAYLIST_NAMES[n as usize % PLAYLIST_NAMES.len()])
}

pub fn show(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let full = ui.max_rect();
    let footer_h = 72.0;
    let body = Rect::from_min_max(full.min + vec2(12.0, 16.0), pos2(full.right() - 12.0, full.bottom() - footer_h));
    let mut ui_body = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)));
    egui::ScrollArea::vertical().id_salt("sidebar-scroll").auto_shrink([false, false]).show(&mut ui_body, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (view, svg, label) in [
            (View::Songs, icons::MUSIC_NOTE, "Songs"),
            (View::Artists, icons::USERS, "Artists"),
            (View::Albums, icons::DISC, "Albums"),
            (View::Stats, icons::CHART, "Stats"),
        ] {
            let active = std::mem::discriminant(&app.view) == std::mem::discriminant(&view);
            if widgets::nav_item(ui, &p, svg, label, active, None).clicked() {
                app.set_view(view);
            }
        }
        ui.add_space(18.0);

        if let Some(plus) = widgets::section_head(ui, &p, "Playlists", true)
            && plus.on_hover_text("New playlist").clicked()
        {
            app.dialog = Some(Dialog::NewPlaylist { text: String::new(), placeholder: random_playlist_name(), add: Vec::new() });
        }
        let playlists = app.lib.playlists.clone();
        let mut drop_on: Option<usize> = None;
        let dragging = app.drag_playlist;
        for (i, pl) in playlists.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click_and_drag());
            let active = matches!(&app.view, View::Playlist(id, _) if *id == pl.id);
            widgets::paint_nav_item(ui, &p, &resp, rect, icons::LIST, &pl.name, active, if resp.hovered() { None } else { Some(pl.count) }, 16.0);
            if resp.hovered() {
                // Rename and delete appear over the right edge on hover.
                let del = Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(22.0, 22.0));
                let ren = del.translate(vec2(-24.0, 0.0));
                let rr = ui.interact(ren, Id::unique(("pl-rename", pl.id)), Sense::click());
                let dr = ui.interact(del, Id::unique(("pl-delete", pl.id)), Sense::click());
                for (r, b, svg) in [(ren, &rr, icons::PENCIL), (del, &dr, icons::CLOSE)] {
                    if b.hovered() {
                        ui.painter().rect_filled(r, 5.0, p.bg3_solid());
                    }
                    icon::paint_centered(ui, svg, r, 13.0, if b.hovered() { p.fg } else { p.fg_dim });
                }
                if rr.clicked() {
                    app.dialog = Some(Dialog::Rename {
                        kind: RenameKind::Playlist(pl.id),
                        title: "Rename playlist",
                        label: "Choose a new name",
                        text: pl.name.clone(),
                        initial: pl.name.clone(),
                    });
                } else if dr.clicked() {
                    app.dialog = Some(Dialog::DeletePlaylist { id: pl.id, name: pl.name.clone() });
                } else if resp.clicked() {
                    app.set_view(View::Playlist(pl.id, pl.name.clone()));
                }
            } else if resp.clicked() {
                app.set_view(View::Playlist(pl.id, pl.name.clone()));
            }
            if resp.drag_started() {
                app.drag_playlist = Some(i);
            }
            if let Some(from) = dragging
                && from != i
                && ui.rect_contains_pointer(rect)
            {
                ui.painter().rect_stroke(rect, 6.0, Stroke::new(1.5, p.accent), StrokeKind::Inside);
                if ui.input(|inp| inp.pointer.any_released()) {
                    drop_on = Some(i);
                }
            }
        }
        if playlists.is_empty() {
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
            widgets::text_v(ui, r.left() + 12.0, r.center().y, "No playlists yet", FontId::proportional(12.0), p.fg_dim, r.width());
        }
        if let (Some(from), Some(to)) = (dragging, drop_on) {
            let mut ids: Vec<i64> = playlists.iter().map(|p| p.id).collect();
            let moved = ids.remove(from);
            ids.insert(to, moved);
            let _ = app.lib.db.reorder_playlists(&ids);
            app.library_changed();
        }
        if ui.input(|i| i.pointer.any_released()) {
            app.drag_playlist = None;
        }
        ui.add_space(18.0);

        if let Some(plus) = widgets::section_head(ui, &p, "Folders", true)
            && plus.on_hover_text("Add folder").clicked()
        {
            add_folders(app);
        }
        let folders = app.lib.folders.clone();
        for f in &folders {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
            let active = matches!(&app.view, View::Folder(x) if x == f);
            if active {
                ui.painter().rect_filled(rect, 6.0, p.accent_soft);
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 6.0, p.bg3);
            }
            icon::paint(ui, icons::FOLDER, Rect::from_center_size(pos2(rect.left() + 15.0, rect.center().y), vec2(14.0, 14.0)), p.fg_dim);
            let name = std::path::Path::new(f).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| f.clone());
            widgets::text_v(ui, rect.left() + 30.0, rect.center().y, &name, FontId::proportional(13.0), if active { p.fg } else { p.fg_dim }, rect.width() - 60.0);
            let x = Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(22.0, 22.0));
            let xr = ui.interact(x, Id::unique(("folder-x", f)), Sense::click());
            if resp.hovered() || xr.hovered() {
                icon::paint_centered(ui, icons::CLOSE, x, 12.0, if xr.hovered() { p.fg } else { p.fg_dim });
            }
            if xr.clicked() {
                let _ = app.lib.db.remove_folder(f);
                app.library_changed();
            } else if resp.on_hover_text(f).clicked() {
                app.set_view(View::Folder(f.clone()));
            }
        }
        if folders.is_empty() {
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
            widgets::text_v(ui, r.left() + 12.0, r.center().y, "Add a music folder to begin", FontId::proportional(12.0), p.fg_dim, r.width());
        }
    });

    // Footer: rescan, or the scan's progress.
    let foot = Rect::from_min_max(pos2(full.left() + 12.0, full.bottom() - footer_h), pos2(full.right() - 12.0, full.bottom() - 12.0));
    ui.painter().hline(foot.x_range(), foot.top(), Stroke::new(1.0, p.border));
    if let Some(scan) = &app.lib.scan {
        let pr = &scan.progress;
        let label = match pr.phase {
            Phase::Discovering => "Finding files…".to_owned(),
            Phase::Done => "Done".to_owned(),
            Phase::Parsing => format!("Scanning {}/{}", pr.processed, pr.total),
        };
        widgets::text_v(ui, foot.left() + 2.0, foot.top() + 22.0, &label, FontId::proportional(12.0), p.fg_dim, foot.width());
        let bar = Rect::from_min_size(pos2(foot.left(), foot.top() + 38.0), vec2(foot.width(), 4.0));
        ui.painter().rect_filled(bar, 2.0, p.bg3_solid());
        let pct = if pr.total > 0 { pr.processed as f32 / pr.total as f32 } else { 0.0 };
        ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * pct, 4.0)), 2.0, p.accent);
        ui.ctx().request_repaint();
    } else {
        let btn = Rect::from_min_size(pos2(foot.left(), foot.top() + 12.0), vec2(foot.width(), 34.0));
        let resp = ui.interact(btn, Id::unique("rescan"), Sense::click());
        let enabled = !app.lib.folders.is_empty();
        ui.painter().rect_filled(btn, 6.0, if resp.hovered() && enabled { p.bg3_solid() } else { p.bg3 });
        widgets::centered_text(ui, btn, "Rescan library", crate::fonts::bold(13.0), if enabled { p.fg } else { p.fg_dim });
        let resp = resp.on_hover_text(if enabled { "Rescan library" } else { "Add a folder first" });
        if resp.clicked() && enabled {
            app.lib.start_scan();
        }
        let stats = format!("{} · {}", pluralize(app.lib.counts.tracks, "song"), pluralize(app.lib.counts.artists, "artist"));
        ui.painter().text(pos2(foot.center().x, foot.bottom() - 6.0), egui::Align2::CENTER_CENTER, stats, FontId::proportional(11.0), p.fg_dim);
    }
}

/// Pick folders, remember them, and scan.
pub fn add_folders(app: &mut LumenApp) {
    let Some(picked) = rfd::FileDialog::new().set_title("Add music folders").pick_folders() else { return };
    for f in &picked {
        let _ = app.lib.db.add_folder(&f.to_string_lossy());
    }
    if !picked.is_empty() {
        app.library_changed();
        app.lib.start_scan();
    }
}
