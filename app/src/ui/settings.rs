//! Settings: appearance, the visualizer frame rate,
//! listening stats, Discord Rich Presence, and About with the update check.

use eframe::egui::{self, Color32, FontId, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::LumenApp;
use crate::palette::Palette;
use crate::ui::{icon, icons, widgets};

const DISCORD_DEV_URL: &str = "https://discord.com/developers/applications";
const REPO: &str = "wumgub/lumen-player";

pub fn show(app: &mut LumenApp, ctx: &egui::Context) {
    let p = app.p;
    // Solid, unlike the other popovers: Glass's see-through panel made the
    // settings hard to read over a busy window.
    let fill = p.bg.blend(p.popover);
    let m = egui::Modal::new(Id::new("settings"))
        .backdrop_color(Color32::from_black_alpha(140))
        .frame(egui::Frame::new().fill(fill).stroke(Stroke::new(1.0, p.border)).corner_radius(14).inner_margin(22))
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.label(egui::RichText::new("Settings").font(crate::fonts::bold(18.0)).color(p.fg));
            ui.add_space(10.0);
            egui::ScrollArea::vertical().max_height(ctx.content_rect().height() * 0.7).show(ui, |ui| {
                appearance(app, ui);
                section_line(ui, &p);
                stats(app, ui);
                section_line(ui, &p);
                discord(app, ui);
                section_line(ui, &p);
                row(ui, &p, "Debug overlay", "Show how much CPU, GPU and memory Lumen is using, in the bottom-left corner.", |ui| {
                    if widgets::switch(ui, &p, app.prefs.debug_overlay).clicked() {
                        app.prefs.debug_overlay = !app.prefs.debug_overlay;
                    }
                });
                section_line(ui, &p);
                about(app, ui);
            });
            ui.add_space(12.0);
            let mut done = false;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                done = widgets::pill(ui, &p, "Done", widgets::Pill::Active, None).clicked();
            });
            done
        });
    if m.should_close() || m.inner {
        let _ = app.lib.db.save_app_settings(&app.settings);
        app.configure_presence();
        app.settings_open = false;
        app.confirm_clear_stats = false;
    }
}

fn section_line(ui: &mut Ui, p: &Palette) {
    ui.add_space(12.0);
    widgets::subtle_separator(ui, p);
    ui.add_space(8.0);
}

fn heading(ui: &mut Ui, p: &Palette, title: &str, desc: &str) {
    ui.label(egui::RichText::new(title).font(crate::fonts::bold(14.0)).color(p.fg));
    if !desc.is_empty() {
        ui.label(egui::RichText::new(desc).size(12.5).color(p.fg_dim));
    }
}

/// A row with text on the left and a control on the right.
fn row(ui: &mut Ui, p: &Palette, title: &str, desc: &str, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(380.0);
            heading(ui, p, title, desc);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
}

fn appearance(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let label = lumen_core::theme::THEMES.iter().find(|t| t.id == app.prefs.theme).map(|t| t.label).unwrap_or("Neon");
    heading(ui, &p, "Appearance", &format!("Theme: {label}. Pick one below, or open Custom (the pencil) to edit every colour and the font."));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for t in lumen_core::theme::THEMES.iter() {
            let tw = widgets::text_width(ui, t.label, FontId::proportional(13.0));
            let custom = t.id == "custom";
            let w = 36.0 + tw + if custom { 34.0 } else { 14.0 };
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
            let active = app.prefs.theme == t.id;
            ui.painter().rect(
                rect,
                16.0,
                if active { p.accent_soft } else if resp.hovered() { p.bg3 } else { Color32::TRANSPARENT },
                Stroke::new(1.0, if active { p.accent } else { p.border }),
                StrokeKind::Inside,
            );
            let tokens = if custom { app.prefs.custom.clone() } else { lumen_core::theme::preset(t.id) };
            let sw = Palette::resolve(t.id, &tokens, Default::default(), false);
            let c = pos2(rect.left() + 17.0, rect.center().y);
            ui.painter().circle_filled(c, 8.0, sw.bg);
            ui.painter().circle_filled(c + vec2(-2.0, -2.0), 4.5, sw.accent);
            ui.painter().circle_filled(c + vec2(3.0, 3.0), 3.0, sw.accent2);
            ui.painter().circle_stroke(c, 8.0, Stroke::new(1.0, p.border));
            widgets::text_v(ui, rect.left() + 31.0, rect.center().y, t.label, FontId::proportional(13.0), p.fg, tw + 2.0);
            let mut pencil = false;
            if custom {
                let pr = Rect::from_center_size(pos2(rect.right() - 16.0, rect.center().y), vec2(24.0, 24.0));
                let r = ui.interact(pr, Id::new("custom-pencil"), Sense::click());
                if r.hovered() {
                    ui.painter().circle_filled(pr.center(), 12.0, p.bg3);
                }
                icon::paint_centered(ui, icons::PENCIL, pr, 13.0, if r.hovered() { p.fg } else { p.fg_dim });
                pencil = r.on_hover_text("Customize theme").clicked();
            }
            if pencil {
                app.prefs.theme = "custom".into();
                app.theme_editor_open = true;
            } else if resp.clicked() {
                app.prefs.theme = t.id.into();
            }
        }
    });
    if app.prefs.theme == "glass" || app.prefs.theme == "blackout" {
        ui.add_space(10.0);
        row(ui, &p, "Accent from album art", "Tint the theme with colours from the playing song's cover.", |ui| {
            if widgets::switch(ui, &p, app.prefs.adaptive).clicked() {
                app.prefs.adaptive = !app.prefs.adaptive;
            }
        });
    }
    ui.add_space(10.0);
    row(ui, &p, "Visualizer frame rate", "Cap the visualizer to save CPU and battery. Lower is lighter.", |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (fps, label) in [(15u32, "15 fps"), (30, "30 fps"), (0, "Full")] {
            let kind = if app.prefs.viz_fps == fps { widgets::Pill::Active } else { widgets::Pill::Neutral };
            if widgets::pill(ui, &p, label, kind, None).clicked() {
                app.prefs.viz_fps = fps;
            }
        }
    });
}

fn stats(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    row(ui, &p, "Listening stats", "Save what you play to build the Stats leaderboard. Turn this off and nothing is recorded.", |ui| {
        if widgets::switch(ui, &p, app.settings.stats_enabled).clicked() {
            app.settings.stats_enabled = !app.settings.stats_enabled;
            // Applies straight away, without waiting for Done.
            let _ = app.lib.db.save_app_settings(&app.settings);
        }
    });
    ui.add_space(8.0);
    row(ui, &p, "Clear history", "Delete everything saved so far. Can't be undone.", |ui| {
        let label = if app.confirm_clear_stats { "Tap again to confirm" } else { "Clear history" };
        if widgets::pill(ui, &p, label, widgets::Pill::Neutral, None).clicked() {
            if app.confirm_clear_stats {
                let _ = app.lib.db.clear_play_history();
                app.confirm_clear_stats = false;
            } else {
                app.confirm_clear_stats = true;
            }
        }
    });
}

fn discord(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    row(ui, &p, "Discord Rich Presence", "Show what you're listening to on your Discord profile.", |ui| {
        if widgets::switch(ui, &p, app.settings.discord_enabled).clicked() {
            app.settings.discord_enabled = !app.settings.discord_enabled;
        }
    });
    ui.add_space(8.0);
    ui.add_enabled_ui(app.settings.discord_enabled, |ui| {
        ui.label(egui::RichText::new("Application Client ID").size(12.0).color(p.fg_dim));
        widgets::input_frame(&p).show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.settings.discord_client_id)
                    .hint_text("e.g. 1234567890123456789")
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .text_color(p.fg),
            );
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(egui::RichText::new("1. In the ").size(12.5).color(p.fg_dim));
            if ui.link(egui::RichText::new("Discord Developer Portal").size(12.5)).clicked() {
                let _ = open_url(DISCORD_DEV_URL);
            }
            ui.label(egui::RichText::new(", create a New Application and copy its Application ID above.").size(12.5).color(p.fg_dim));
        });
        ui.label(egui::RichText::new("2. Keep the desktop Discord app running on this machine.").size(12.5).color(p.fg_dim));
    });
}

fn about(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let version = env!("CARGO_PKG_VERSION");
    row(ui, &p, "About", &format!("Version {version}"), |ui| {
        let label = if app.update.running { "Checking…" } else { "Check for updates" };
        if widgets::pill(ui, &p, label, widgets::Pill::Neutral, None).clicked() && !app.update.running {
            start_update_check(app, ui.ctx().clone());
        }
    });
    match &app.update.result {
        Some(Ok((latest, true, _))) => {
            let _ = latest;
            ui.label(egui::RichText::new("You're on the latest version.").size(12.5).color(p.fg_dim));
        }
        Some(Ok((latest, false, url))) => {
            ui.label(egui::RichText::new(format!("Update available: version {latest}.")).size(12.5).color(p.accent));
            if ui.link(egui::RichText::new("Download it from the releases page").size(12.5)).clicked() {
                let _ = open_url(url);
            }
        }
        Some(Err(e)) => {
            ui.label(egui::RichText::new(format!("Couldn't check: {e}.")).size(12.5).color(p.fg_dim));
        }
        None => {}
    }
}

/// Compare dotted versions numerically.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> { s.trim_start_matches('v').split(['.', '-']).map(|x| x.parse().unwrap_or(0)).collect() };
    let (x, y) = (parse(a), parse(b));
    for i in 0..x.len().max(y.len()) {
        let o = x.get(i).unwrap_or(&0).cmp(y.get(i).unwrap_or(&0));
        if o != std::cmp::Ordering::Equal {
            return o;
        }
    }
    std::cmp::Ordering::Equal
}

fn start_update_check(app: &mut LumenApp, ctx: egui::Context) {
    let (tx, rx) = std::sync::mpsc::channel();
    app.update.running = true;
    app.update.result = None;
    app.update.rx = Some(rx);
    std::thread::spawn(move || {
        let r = check(env!("CARGO_PKG_VERSION"));
        let _ = tx.send(r);
        ctx.request_repaint();
    });
}

/// Ask GitHub for the newest release. Uses the system's `curl`, which every
/// supported OS ships, rather than bundling a TLS stack for one request.
fn check(current: &str) -> Result<(String, bool, String), String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "10", "-H", "Accept: application/vnd.github+json", &url])
        .output()
        .map_err(|_| "curl isn't available".to_owned())?;
    if !out.status.success() {
        return Err("GitHub didn't answer".into());
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|_| "unexpected reply".to_owned())?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).ok_or("no releases yet")?.trim_start_matches('v').to_owned();
    let page = v.get("html_url").and_then(|t| t.as_str()).unwrap_or("https://github.com/wumgub/lumen-player/releases").to_owned();
    let up_to_date = compare_versions(current, &tag) != std::cmp::Ordering::Less;
    Ok((tag, up_to_date, page))
}

pub fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
    #[cfg(all(unix, not(target_os = "macos")))]
    std::process::Command::new("xdg-open").arg(url).spawn()?;
    Ok(())
}
