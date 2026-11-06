//! The three visualizers, ported from src/renderer/src/visualizers/*.ts with
//! the same maths. Canvas `shadowBlur` glows become blurred shapes or wider,
//! fainter strokes drawn underneath.

use eframe::egui::{self, Color32, CornerRadius, Mesh, Pos2, Rect, Shape, Stroke, pos2, vec2};
use eframe::epaint::RectShape;
use lumen_playback::analyzer::AudioFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bars,
    Waveform,
    Radial,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Bars, Kind::Waveform, Kind::Radial];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Bars => "Spectrum Bars",
            Kind::Waveform => "Waveform",
            Kind::Radial => "Radial",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub accent: Color32,
    pub accent2: Color32,
    pub fg: Color32,
}

fn alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[derive(Default)]
pub struct State {
    peaks: Vec<f32>,
}

impl State {
    pub fn reset(&mut self) {
        self.peaks.clear();
    }
}

/// Draw one frame. `t` is seconds since the visualizer started, `dt` the time
/// since the last frame.
pub fn draw(
    kind: Kind,
    painter: &egui::Painter,
    rect: Rect,
    frame: &AudioFrame,
    colors: Colors,
    state: &mut State,
    t: f32,
    dt: f32,
) {
    match kind {
        Kind::Bars => bars(painter, rect, frame, colors, state, dt),
        Kind::Waveform => waveform(painter, rect, frame, colors),
        Kind::Radial => radial(painter, rect, frame, colors, t),
    }
}

/// A cheap stand-in for canvas `shadowBlur` around a rectangle: the rect, plus
/// a ring `spread` wide that fades to nothing, with the colour going from
/// `top` to `bottom` the way a shadow under a gradient fill would.
fn soft_glow(mesh: &mut Mesh, r: Rect, spread: f32, top: Color32, bottom: Color32) {
    let xs = [r.left() - spread, r.left(), r.right(), r.right() + spread];
    let ys = [r.top() - spread, r.top(), r.bottom(), r.bottom() + spread * 0.5];
    let i0 = mesh.vertices.len() as u32;
    for (yi, &y) in ys.iter().enumerate() {
        let t = ((y - r.top()) / r.height().max(1.0)).clamp(0.0, 1.0);
        let inner = lerp_color(top, bottom, t);
        for (xi, &x) in xs.iter().enumerate() {
            let edge = xi == 0 || xi == 3 || yi == 0 || yi == 3;
            mesh.colored_vertex(pos2(x, y), if edge { Color32::TRANSPARENT } else { inner });
        }
    }
    for yi in 0..3u32 {
        for xi in 0..3u32 {
            let a = i0 + yi * 4 + xi;
            mesh.add_triangle(a, a + 1, a + 5);
            mesh.add_triangle(a, a + 5, a + 4);
        }
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let (a, b) = (a.to_array(), b.to_array());
    let mix = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(mix(0), mix(1), mix(2), mix(3))
}

fn band(freq: &[u8], from: usize, to: usize) -> f32 {
    let to = to.min(freq.len());
    let sum: u32 = freq[from.min(to)..to].iter().map(|&v| v as u32).sum();
    sum as f32 / (to.saturating_sub(from)).max(1) as f32 / 255.0
}

/// Classic mirrored spectrum-analyser bars rising from the baseline.
fn bars(painter: &egui::Painter, rect: Rect, frame: &AudioFrame, c: Colors, s: &mut State, dt: f32) {
    const BARS: usize = 64;
    const GAP: f32 = 2.0;
    let freq = &frame.freq;
    // Roughly the lower 70% of the spectrum, where music's energy lives.
    let usable = (freq.len() as f32 * 0.7).floor();
    let step = usable / BARS as f32;
    let (width, height) = (rect.width(), rect.height());
    let bar_w = (width - GAP * (BARS - 1) as f32) / BARS as f32;
    let baseline = height * 0.92;
    s.peaks.resize(BARS, 0.0);
    // The browser dropped caps 0.012 per frame at 60 fps.
    let fall = 0.012 * dt * 60.0;

    let mut glow = Mesh::default();
    let mut body = Mesh::default();
    let mut caps = Vec::with_capacity(BARS * 2);
    for i in 0..BARS {
        let from = (i as f32 * step).floor() as usize;
        let to = ((i + 1) as f32 * step).floor() as usize;
        let avg = band(freq, from, to);
        // Perceptual tilt so highs aren't dwarfed.
        let v = avg.powf(0.8) * (1.0 + i as f32 / BARS as f32);
        let h = v.min(1.0) * baseline * 0.95;
        let x = rect.left() + i as f32 * (bar_w + GAP);
        let color = if i < BARS / 2 { c.accent } else { c.accent2 };
        let bottom = rect.top() + baseline;

        if h > 0.5 {
            let r = (bar_w / 2.0).min(4.0).min(h / 2.0);
            let bar = Rect::from_min_max(pos2(x, bottom - h), pos2(x + bar_w, bottom));
            soft_glow(&mut glow, bar, 7.0, alpha(color, 0.38), alpha(color, 0.08));
            // Gradient body, from 25% at the baseline up to full colour, with a
            // solid rounded cap where the gradient has already reached it.
            let top = bar.top() + r;
            let i0 = body.vertices.len() as u32;
            body.colored_vertex(pos2(bar.left(), top), color);
            body.colored_vertex(pos2(bar.right(), top), color);
            body.colored_vertex(pos2(bar.right(), bottom), alpha(color, 0.25));
            body.colored_vertex(pos2(bar.left(), bottom), alpha(color, 0.25));
            body.add_triangle(i0, i0 + 1, i0 + 2);
            body.add_triangle(i0, i0 + 2, i0 + 3);
            let cap = Rect::from_min_max(bar.min, pos2(bar.right(), top + r * 0.5));
            let radius = CornerRadius { nw: r as u8, ne: r as u8, sw: 0, se: 0 };
            caps.push(Shape::Rect(RectShape::filled(cap, radius, color)));
        }

        // Falling peak cap.
        s.peaks[i] = (s.peaks[i] - fall).max(h / baseline);
        let py = rect.top() + baseline - s.peaks[i] * baseline;
        caps.push(Shape::rect_filled(
            Rect::from_min_size(pos2(x, py - 2.0), vec2(bar_w, 2.0)),
            0.0,
            c.fg,
        ));
    }
    painter.add(Shape::mesh(glow));
    painter.add(Shape::mesh(body));
    painter.extend(caps);
}

/// Centered oscilloscope with a soft glow and a mirrored echo.
fn waveform(painter: &egui::Painter, rect: Rect, frame: &AudioFrame, c: Colors) {
    let wave = &frame.wave;
    let mid = rect.center().y;
    let amp = rect.height() * 0.4 * (0.4 + frame.level * 2.0);
    let slice = rect.width() / wave.len() as f32;
    let points = |mirror: bool| -> Vec<Pos2> {
        wave.iter()
            .enumerate()
            .map(|(i, &w)| {
                let v = (w as f32 - 128.0) / 128.0;
                pos2(rect.left() + i as f32 * slice, mid + if mirror { -v } else { v } * amp)
            })
            .collect()
    };
    painter.line_segment(
        [pos2(rect.left(), mid), pos2(rect.right(), mid)],
        Stroke::new(1.0, alpha(c.fg, 0.08)),
    );
    let line = |pts: Vec<Pos2>, color: Color32, width: f32, a: f32| {
        // Glow: two wide faint passes under the line.
        painter.add(Shape::line(pts.clone(), Stroke::new(width * 5.0, alpha(color, a * 0.10))));
        painter.add(Shape::line(pts.clone(), Stroke::new(width * 2.6, alpha(color, a * 0.22))));
        painter.add(Shape::line(pts, Stroke::new(width, alpha(color, a))));
    };
    line(points(true), c.accent2, 2.0, 0.4);
    line(points(false), c.accent, 2.5, 1.0);
}

/// Frequency bars wrapped into a ring, with a pulsing core.
fn radial(painter: &egui::Painter, rect: Rect, frame: &AudioFrame, c: Colors, t: f32) {
    const BARS: usize = 96;
    let center = rect.center();
    let freq = &frame.freq;
    let usable = (freq.len() as f32 * 0.6).floor();
    let step = usable / BARS as f32;
    let short = rect.width().min(rect.height());
    let base_r = short * 0.16 * (1.0 + frame.bass * 0.35);
    let max_len = short * 0.3;
    let rot = t * 0.15;

    // Pulsing core glow: a radial gradient as a triangle fan.
    let core_r = base_r * 1.4;
    let mut core = Mesh::default();
    core.colored_vertex(center, alpha(c.accent, 0.5 + frame.bass * 0.4));
    const SEGMENTS: u32 = 48;
    for k in 0..=SEGMENTS {
        let a = k as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        core.colored_vertex(center + vec2(a.cos(), a.sin()) * core_r, Color32::TRANSPARENT);
        if k > 0 {
            core.add_triangle(0, k, k + 1);
        }
    }
    painter.add(Shape::mesh(core));

    let mut glow = Vec::with_capacity(BARS);
    let mut lines = Vec::with_capacity(BARS * 3);
    for i in 0..BARS {
        let from = (i as f32 * step).floor() as usize;
        let to = ((i + 1) as f32 * step).floor() as usize;
        let v = band(freq, from, to);
        let len = v.powf(0.85) * max_len;
        let angle = i as f32 / BARS as f32 * std::f32::consts::TAU + rot;
        let dir = vec2(angle.cos(), angle.sin());
        let (p1, p2) = (center + dir * base_r, center + dir * (base_r + len));
        let color = if i % 2 == 0 { c.accent } else { c.accent2 };
        glow.push(Shape::line_segment([p1, p2], Stroke::new(9.0, alpha(color, 0.12))));
        lines.push(Shape::line_segment([p1, p2], Stroke::new(3.0, color)));
        // Round line caps.
        lines.push(Shape::circle_filled(p1, 1.5, color));
        lines.push(Shape::circle_filled(p2, 1.5, color));
    }
    painter.extend(glow);
    painter.extend(lines);
    painter.circle_stroke(center, base_r, Stroke::new(1.0, alpha(c.fg, 0.15)));
}
