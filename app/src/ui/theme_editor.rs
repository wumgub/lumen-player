//! The custom theme editor (ThemeEditor.tsx): start from a built-in theme,
//! pick every colour and the font, and share it as the same JSON the other
//! Lumen apps read.

use eframe::egui::{self, Color32, FontId, Id, Stroke, Ui, vec2};
use lumen_core::theme::{self, FONT_PRESETS, TOKEN_LABELS};

use crate::app::LumenApp;
use crate::palette::rgba;
use crate::ui::widgets;

#[derive(Default)]
pub struct EditorState {
    pub share: Share,
    pub import: String,
    pub error: Option<String>,
    pub copied: bool,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum Share {
    #[default]
    None,
    Export,
    Import,
}

pub fn show(app: &mut LumenApp, ctx: &egui::Context) {
    let p = app.p;
    let mut st = std::mem::take(&mut app.editor);
    let m = egui::Modal::new(Id::new("theme-editor-modal"))
        .backdrop_color(Color32::from_black_alpha(140))
        .frame(egui::Frame::new().fill(p.popover).stroke(Stroke::new(1.0, p.border)).corner_radius(14).inner_margin(22))
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.label(egui::RichText::new("Customize theme").font(crate::fonts::bold(18.0)).color(p.fg));
            ui.label(egui::RichText::new("Changes apply as you go. Your custom theme is saved automatically.").size(12.5).color(p.fg_dim));
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Start from").size(12.0).color(p.fg_dim));
                for id in theme::START_FROM {
                    let label = theme::THEMES.iter().find(|t| t.id == id).map(|t| t.label).unwrap_or(id);
                    if widgets::pill(ui, &p, label, widgets::Pill::Neutral, None).clicked() {
                        app.prefs.custom = theme::preset(id);
                    }
                }
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().max_height(ctx.content_rect().height() * 0.55).show(ui, |ui| {
                let mut group = "";
                for (key, label, g) in TOKEN_LABELS {
                    if g != group {
                        group = g;
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(g.to_uppercase()).size(11.0).color(p.fg_dim));
                    }
                    ui.horizontal(|ui| {
                        ui.set_min_height(30.0);
                        ui.label(egui::RichText::new(label).size(13.0).color(p.fg));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let value = app.prefs.custom.get(key).cloned().unwrap_or_default();
                            if key == "font" {
                                font_picker(app, ui, value);
                            } else {
                                color_row(app, ui, key, value);
                            }
                        });
                    });
                }
                ui.add_space(12.0);
                share(app, ui, &mut st);
            });
            ui.add_space(12.0);
            let mut done = false;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                done = widgets::pill(ui, &p, "Done", widgets::Pill::Active, None).clicked();
            });
            done
        });
    if m.should_close() || m.inner {
        app.theme_editor_open = false;
    } else {
        app.editor = st;
    }
}

fn color_row(app: &mut LumenApp, ui: &mut Ui, key: &str, value: String) {
    let p = app.p;
    let mut color = theme::parse_color(&value).map(rgba).unwrap_or(Color32::BLACK);
    let mut hex = value.clone();
    let resp = widgets::input_frame(&p).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
        ui.add(egui::TextEdit::singleline(&mut hex).desired_width(84.0).frame(egui::Frame::NONE).font(FontId::monospace(12.0)).text_color(p.fg))
    });
    if resp.inner.changed() && theme::parse_color(&hex).is_some() {
        app.prefs.custom.insert(key.to_owned(), hex.trim().to_owned());
    }
    if egui::color_picker::color_edit_button_srgba(ui, &mut color, egui::color_picker::Alpha::Opaque).changed() {
        let [r, g, b, _] = color.to_srgba_unmultiplied();
        app.prefs.custom.insert(key.to_owned(), format!("#{r:02x}{g:02x}{b:02x}"));
    }
}

fn font_picker(app: &mut LumenApp, ui: &mut Ui, value: String) {
    let current = FONT_PRESETS.iter().find(|(_, v)| *v == value).map(|(l, _)| *l).unwrap_or("Custom");
    egui::ComboBox::from_id_salt("font-preset").selected_text(current).width(160.0).show_ui(ui, |ui| {
        for (label, v) in FONT_PRESETS {
            if ui.selectable_label(value == v, label).clicked() {
                app.prefs.custom.insert("font".into(), v.to_owned());
            }
        }
    });
}

fn share(app: &mut LumenApp, ui: &mut Ui, st: &mut EditorState) {
    let p = app.p;
    ui.label(egui::RichText::new("SHARE").size(11.0).color(p.fg_dim));
    ui.horizontal(|ui| {
        let ek = if st.share == Share::Export { widgets::Pill::Active } else { widgets::Pill::Neutral };
        if widgets::pill(ui, &p, "Export", ek, None).clicked() {
            st.share = if st.share == Share::Export { Share::None } else { Share::Export };
            st.copied = false;
        }
        let ik = if st.share == Share::Import { widgets::Pill::Active } else { widgets::Pill::Neutral };
        if widgets::pill(ui, &p, "Import", ik, None).clicked() {
            st.share = if st.share == Share::Import { Share::None } else { Share::Import };
            st.error = None;
        }
    });
    match st.share {
        Share::Export => {
            let mut text = theme::export(&app.prefs.custom);
            widgets::input_frame(&p).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut text).desired_rows(6).desired_width(f32::INFINITY).frame(egui::Frame::NONE).font(FontId::monospace(11.0)).interactive(true));
            });
            ui.horizontal(|ui| {
                if widgets::pill(ui, &p, if st.copied { "Copied ✓" } else { "Copy" }, widgets::Pill::Neutral, None).clicked() {
                    ui.ctx().copy_text(theme::export(&app.prefs.custom));
                    st.copied = true;
                }
                if widgets::pill(ui, &p, "Save as file…", widgets::Pill::Neutral, None).clicked()
                    && let Some(path) = rfd::FileDialog::new().set_file_name("lumen-theme.json").save_file()
                {
                    let _ = std::fs::write(path, theme::export(&app.prefs.custom));
                }
            });
        }
        Share::Import => {
            widgets::input_frame(&p).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut st.import).hint_text("Paste a theme here").desired_rows(5).desired_width(f32::INFINITY).frame(egui::Frame::NONE).font(FontId::monospace(11.0)));
            });
            if let Some(e) = &st.error {
                ui.label(egui::RichText::new(e).size(12.0).color(Color32::from_rgb(0xe0, 0x60, 0x7e)));
            }
            ui.horizontal(|ui| {
                if widgets::pill(ui, &p, "Open file…", widgets::Pill::Neutral, None).clicked()
                    && let Some(path) = rfd::FileDialog::new().add_filter("Theme", &["json", "txt"]).pick_file()
                    && let Ok(s) = std::fs::read_to_string(path)
                {
                    st.import = s;
                }
                if widgets::pill(ui, &p, "Apply", widgets::Pill::Accent, None).clicked() {
                    match theme::parse(&st.import) {
                        Some(tokens) => {
                            // Missing keys fall back to Neon, as the web editor does.
                            let mut merged = theme::preset("neon");
                            merged.extend(tokens);
                            app.prefs.custom = merged;
                            app.prefs.theme = "custom".into();
                            st.import.clear();
                            st.error = None;
                            st.share = Share::None;
                        }
                        None => st.error = Some("That doesn't look like a Lumen theme.".into()),
                    }
                }
            });
        }
        Share::None => {}
    }
    let _ = vec2(0.0, 0.0);
}
