//! The Stats view (StatsView.tsx): totals, the last 12 weeks, and the
//! most-played leaderboard.

use std::collections::HashMap;

use eframe::egui::{self, FontId, Id, Rect, Sense, Ui, pos2, vec2};
use lumen_core::stats;

use crate::app::LumenApp;
use crate::art::ArtKey;
use crate::ui::{format_duration, icons, pluralize, widgets};

pub fn show(app: &mut LumenApp, ui: &mut Ui) {
    let p = app.p;
    let history = app.lib.db.play_history().unwrap_or_default();
    let area = ui.max_rect();
    if history.is_empty() {
        let c = area.center();
        crate::ui::icon::paint_centered(ui, icons::CHART, Rect::from_center_size(c - vec2(0.0, 50.0), vec2(40.0, 40.0)), 40.0, p.fg_dim);
        ui.painter().text(c, egui::Align2::CENTER_CENTER, "No listening yet", crate::fonts::bold(20.0), p.fg);
        ui.painter().text(c + vec2(0.0, 28.0), egui::Align2::CENTER_CENTER, "Play some music. Your most-played tracks and weekly totals will show up here.", FontId::proportional(13.0), p.fg_dim);
        return;
    }
    let tracks: HashMap<i64, lumen_core::Track> = app.lib.all_tracks().into_iter().map(|t| (t.id, t)).collect();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let s = stats::summarise(&history, |id| tracks.contains_key(&id), now);
    let this_week = s.weeks.last().copied().unwrap_or(stats::Week { start: 0, seconds: 0, plays: 0 });

    egui::ScrollArea::vertical().id_salt("stats").auto_shrink([false, false]).show(ui, |ui| {
        let w = ui.available_width();
        ui.label(egui::RichText::new("Listening").font(crate::fonts::bold(30.0)).color(p.fg));
        ui.add_space(10.0);
        // Totals.
        ui.horizontal(|ui| {
            for (label, value, sub) in [
                ("Total listening", format_duration(s.total_seconds as f64), pluralize(s.total_plays, "play")),
                ("This week", format_duration(this_week.seconds as f64), pluralize(this_week.plays, "play")),
                ("Songs played", pluralize(s.top.len() as i64, "song"), "in the top 30".to_owned()),
            ] {
                let (r, _) = ui.allocate_exact_size(vec2(((w - 24.0) / 3.0).max(140.0), 84.0), Sense::hover());
                ui.painter().rect_filled(r, 10.0, p.bg2);
                widgets::text(ui, r.min + vec2(16.0, 14.0), label, FontId::proportional(12.0), p.fg_dim, r.width() - 32.0);
                widgets::text(ui, r.min + vec2(16.0, 32.0), &value, crate::fonts::bold(22.0), p.fg, r.width() - 32.0);
                widgets::text(ui, r.min + vec2(16.0, 60.0), &sub, FontId::proportional(12.0), p.fg_dim, r.width() - 32.0);
            }
        });
        ui.add_space(18.0);

        // Weekly chart.
        ui.label(egui::RichText::new("LAST 12 WEEKS").size(11.0).color(p.fg_dim));
        let (chart, _) = ui.allocate_exact_size(vec2(w, 170.0), Sense::hover());
        ui.painter().rect_filled(chart, 10.0, p.bg2);
        let inner = chart.shrink2(vec2(16.0, 16.0));
        let bars = Rect::from_min_max(inner.min, pos2(inner.right(), inner.bottom() - 18.0));
        let max = s.weeks.iter().map(|w| w.seconds).max().unwrap_or(1).max(1) as f32;
        let n = s.weeks.len().max(1) as f32;
        let gap = 8.0;
        let bw = (bars.width() - gap * (n - 1.0)) / n;
        for (i, wk) in s.weeks.iter().enumerate() {
            let x = bars.left() + i as f32 * (bw + gap);
            let col = Rect::from_min_size(pos2(x, bars.top()), vec2(bw, bars.height()));
            ui.painter().rect_filled(col, 4.0, p.bg3);
            let h = bars.height() * wk.seconds as f32 / max;
            if h > 0.5 {
                let r = Rect::from_min_max(pos2(x, bars.bottom() - h), pos2(x + bw, bars.bottom()));
                let fill = if i + 1 == s.weeks.len() { p.accent } else { crate::palette::mix(p.accent, p.bg3, 0.35) };
                ui.painter().rect_filled(r, 4.0, fill);
            }
            let resp = ui.interact(col, Id::new(("week", i)), Sense::hover());
            resp.on_hover_text(format!("{}: {} · {}", week_range(wk.start), format_duration(wk.seconds as f64), pluralize(wk.plays, "play")));
            let tick = week_tick(wk.start);
            ui.painter().text(pos2(x + bw / 2.0, inner.bottom() - 4.0), egui::Align2::CENTER_CENTER, tick, FontId::proportional(10.0), p.fg_dim);
        }
        ui.add_space(18.0);

        // Leaderboard.
        ui.label(egui::RichText::new("MOST PLAYED").size(11.0).color(p.fg_dim));
        ui.add_space(4.0);
        let max_plays = s.top.iter().map(|r| r.plays).max().unwrap_or(1).max(1) as f32;
        let mut play: Option<lumen_core::Track> = None;
        for (i, row) in s.top.iter().enumerate() {
            let Some(t) = tracks.get(&row.track_id) else { continue };
            let (r, resp) = ui.allocate_exact_size(vec2(w, 54.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 8.0, p.bg3);
            }
            ui.painter().text(pos2(r.left() + 20.0, r.center().y), egui::Align2::CENTER_CENTER, (i + 1).to_string(), crate::fonts::bold(14.0), p.fg_dim);
            let art = Rect::from_min_size(pos2(r.left() + 40.0, r.center().y - 20.0), vec2(40.0, 40.0));
            widgets::artwork(ui, &p, &mut app.art, &ArtKey::Track(t.id), art, 5.0);
            let x = art.right() + 12.0;
            let tw = r.width() - 260.0;
            widgets::text(ui, pos2(x, r.top() + 8.0), &t.title, crate::fonts::medium(14.0), p.fg, tw);
            widgets::text(ui, pos2(x, r.top() + 27.0), &t.artist, FontId::proportional(12.0), p.fg_dim, tw);
            let bar = Rect::from_min_size(pos2(r.right() - 200.0, r.center().y - 3.0), vec2(120.0, 6.0));
            ui.painter().rect_filled(bar, 3.0, p.bg3);
            ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * row.plays as f32 / max_plays, 6.0)), 3.0, p.accent);
            ui.painter().text(pos2(r.right() - 14.0, r.center().y - 8.0), egui::Align2::RIGHT_CENTER, pluralize(row.plays, "play"), crate::fonts::medium(13.0), p.fg);
            ui.painter().text(pos2(r.right() - 14.0, r.center().y + 10.0), egui::Align2::RIGHT_CENTER, format_duration(row.seconds as f64), FontId::proportional(11.0), p.fg_dim);
            if resp.clicked() {
                play = Some(t.clone());
            }
        }
        if let Some(t) = play {
            app.play_tracks(&[t], 0);
        }
        ui.add_space(24.0);
    });
}

fn local_date(ms: i64) -> chrono_lite::Date {
    chrono_lite::Date::from_ms(ms)
}

fn week_tick(start: i64) -> String {
    let d = local_date(start);
    format!("{}/{}", d.day, d.month)
}

fn week_range(start: i64) -> String {
    let a = local_date(start);
    let b = local_date(start + 6 * 86_400_000 + 3_600_000);
    format!("{} {} – {} {}", a.month_name(), a.day, b.month_name(), b.day)
}

/// Just enough local-date formatting for the chart's labels.
mod chrono_lite {
    pub struct Date {
        pub day: u32,
        pub month: u32,
    }

    impl Date {
        pub fn from_ms(ms: i64) -> Date {
            let (month, day) = lumen_core::stats::local_month_day(ms);
            Date { day, month }
        }

        pub fn month_name(&self) -> &'static str {
            ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(self.month as usize).clamp(1, 12) - 1]
        }
    }
}
