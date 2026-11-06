//! The active theme as colours, the way themes.css resolves them: the hex
//! presets, Glass's translucent panels, the accent-soft tints, and accents
//! taken from the album art (Glass, and Blackout) when that's switched on.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};
use lumen_core::theme::{self, Tokens};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub bg: Color32,
    pub bg2: Color32,
    pub bg3: Color32,
    pub popover: Color32,
    pub fg: Color32,
    pub fg_dim: Color32,
    pub accent: Color32,
    pub accent2: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub border: Color32,
    pub viz_bg: Color32,
    pub viz_fg: Color32,
    pub viz_accent: Color32,
    pub viz_accent2: Color32,
    pub dark: bool,
    /// Frosted panels over a blurred copy of the cover.
    pub glass: bool,
    /// The theme's font is Courier New (Retro, or a custom mono pick).
    pub mono: bool,
}

pub fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let (a, b) = (a.to_srgba_unmultiplied(), b.to_srgba_unmultiplied());
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(0), m(1), m(2), m(3))
}

fn token(t: &Tokens, key: &str, fallback: &str) -> Color32 {
    rgba(t.get(key).and_then(|v| theme::parse_color(v)).or_else(|| theme::parse_color(fallback)).unwrap_or([0, 0, 0, 255]))
}

/// Accents sampled from the current cover, when there is one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArtAccents {
    pub primary: Option<Color32>,
    pub secondary: Option<Color32>,
}

impl Palette {
    pub fn resolve(theme_id: &str, custom: &Tokens, art: ArtAccents, adaptive: bool) -> Palette {
        let tokens = if theme_id == "custom" { custom.clone() } else { theme::preset(theme_id) };
        let neon = theme::preset("neon");
        let get = |k: &str| token(&tokens, k, neon.get(k).map(String::as_str).unwrap_or("#000000"));
        let mut p = Palette {
            bg: get("bg"),
            bg2: get("bg-2"),
            bg3: get("bg-3"),
            popover: get("popover-bg"),
            fg: get("fg"),
            fg_dim: get("fg-dim"),
            accent: get("accent"),
            accent2: get("accent-2"),
            accent_soft: Color32::TRANSPARENT,
            on_accent: Color32::WHITE,
            border: get("border"),
            viz_bg: get("viz-bg"),
            viz_fg: get("viz-fg"),
            viz_accent: get("viz-accent"),
            viz_accent2: get("viz-accent-2"),
            dark: true,
            glass: theme_id == "glass",
            mono: tokens.get("font").is_some_and(|f| theme::is_mono_font(f)),
        };
        let soft = match theme_id {
            "minimal" => 0.12,
            "neon" | "retro" => 0.15,
            _ => 0.16,
        };
        if p.glass {
            // themes.css: translucent panels, with the accents from the art.
            p.bg2 = Color32::from_rgba_unmultiplied(255, 255, 255, 15);
            p.bg3 = Color32::from_rgba_unmultiplied(255, 255, 255, 26);
            p.border = Color32::from_rgba_unmultiplied(255, 255, 255, 31);
            p.popover = Color32::from_rgba_unmultiplied(16, 20, 30, 209);
            if adaptive {
                if let Some(a) = art.primary {
                    p.accent = a;
                    p.viz_accent = a;
                }
                if let Some(a) = art.secondary.or(art.primary) {
                    p.accent2 = a;
                    p.viz_accent2 = a;
                }
            }
            p.accent_soft = Color32::from_rgba_unmultiplied(255, 255, 255, 26);
        } else {
            if theme_id == "blackout" && adaptive {
                if let Some(a) = art.primary {
                    p.accent = a;
                    p.accent2 = a;
                    p.viz_accent = a;
                    p.viz_accent2 = a;
                }
            }
            p.accent_soft = with_alpha(p.accent, soft);
        }
        p.dark = theme::is_dark(p.bg.to_srgba_unmultiplied());
        p.on_accent = rgba(theme::contrast_on(p.accent.to_srgba_unmultiplied()));
        p
    }

    pub fn bg_solid(&self) -> Color32 {
        self.bg
    }

    /// `bg-3` made opaque (Glass's is a translucent white), for things that
    /// must not let the backdrop through.
    pub fn bg3_solid(&self) -> Color32 {
        if self.glass { mix(self.bg, Color32::WHITE, 0.10) } else { self.bg3 }
    }

    /// egui's own widgets (text fields, scrollbars, sliders) in these colours.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut v = if self.dark { Visuals::dark() } else { Visuals::light() };
        v.override_text_color = Some(self.fg);
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = self.popover;
        v.window_stroke = Stroke::new(1.0, self.border);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(10);
        v.extreme_bg_color = self.bg3;
        v.faint_bg_color = self.bg2;
        v.code_bg_color = self.bg3;
        v.hyperlink_color = self.accent;
        v.selection.bg_fill = with_alpha(self.accent, 0.35);
        v.selection.stroke = Stroke::new(1.0, self.accent);
        v.text_cursor.stroke = Stroke::new(1.5, self.accent);
        let w = &mut v.widgets;
        for (state, fill) in [
            (&mut w.noninteractive, Color32::TRANSPARENT),
            (&mut w.inactive, self.bg3),
            (&mut w.hovered, mix(self.bg3, self.fg, 0.08)),
            (&mut w.active, mix(self.bg3, self.fg, 0.14)),
            (&mut w.open, self.bg3),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.corner_radius = CornerRadius::same(8);
            state.fg_stroke = Stroke::new(1.0, self.fg);
            state.bg_stroke = Stroke::NONE;
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.fg_dim);
        w.hovered.bg_stroke = Stroke::new(1.0, self.border);
        v.popup_shadow = egui::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(110) };
        v.window_shadow = v.popup_shadow;
        // Both of egui's light/dark slots get our visuals, so the OS
        // appearance setting can't swap in egui's defaults.
        ctx.all_styles_mut(|s| {
            s.visuals = v.clone();
            s.spacing.item_spacing = egui::vec2(8.0, 6.0);
            s.spacing.button_padding = egui::vec2(12.0, 6.0);
            s.spacing.scroll = egui::style::ScrollStyle::floating();
            s.spacing.slider_rail_height = 4.0;
            s.interaction.selectable_labels = false;
        });
    }
}

/// Two vivid colours from a cover, ported from ThemeContext.tsx's
/// extractPalette: coarse hue buckets over a 48x48 sample, ranked by how many
/// pixels fall in each times how saturated they are.
pub fn extract_accents(rgba: &[u8], width: usize, height: usize) -> ArtAccents {
    #[derive(Default, Clone, Copy)]
    struct Bucket {
        count: f32,
        r: f32,
        g: f32,
        b: f32,
        s: f32,
    }
    let mut buckets = std::collections::HashMap::<i32, Bucket>::new();
    let step_x = (width as f32 / 48.0).max(1.0);
    let step_y = (height as f32 / 48.0).max(1.0);
    let mut y = 0.0;
    while (y as usize) < height {
        let mut x = 0.0;
        while (x as usize) < width {
            let i = ((y as usize) * width + x as usize) * 4;
            if let Some(px) = rgba.get(i..i + 3) {
                let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
                let (h, s, l) = rgb_to_hsl(r, g, b);
                // Skip near-greys and near-black/white, as the web version does.
                if s >= 0.2 && (0.12..=0.9).contains(&l) {
                    let key = (h * 12.0).round() as i32;
                    let e = buckets.entry(key).or_default();
                    e.count += 1.0;
                    e.r += r;
                    e.g += g;
                    e.b += b;
                    e.s += s;
                }
            }
            x += step_x;
        }
        y += step_y;
    }
    let mut ranked: Vec<Bucket> = buckets.into_values().collect();
    ranked.sort_by(|a, b| (b.count * b.s).total_cmp(&(a.count * a.s)));
    let to_color = |b: &Bucket| {
        Color32::from_rgb((b.r / b.count).round() as u8, (b.g / b.count).round() as u8, (b.b / b.count).round() as u8)
    };
    ArtAccents {
        primary: ranked.first().map(to_color),
        secondary: ranked.get(1).or(ranked.first()).map(to_color),
    }
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    let mut h = 0.0;
    if d != 0.0 {
        h = if max == r {
            ((g - b) / d) % 6.0
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        h /= 6.0;
        if h < 0.0 {
            h += 1.0;
        }
    }
    (h, s, l)
}
