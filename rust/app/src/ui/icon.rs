//! Icons as textures: each SVG from `icons.rs` is rendered white with resvg at
//! the exact pixel size it's shown at, cached, and tinted when drawn.

use std::cell::RefCell;
use std::collections::HashMap;

use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, TextureOptions, pos2};

thread_local! {
    static CACHE: RefCell<HashMap<(usize, u32), TextureHandle>> = RefCell::new(HashMap::new());
}

fn texture(ctx: &egui::Context, svg: &'static str, px: u32) -> Option<TextureHandle> {
    let key = (svg.as_ptr() as usize, px);
    if let Some(t) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Some(t);
    }
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(px, px)?;
    let scale = px as f32 / tree.size().width().max(1.0);
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // tiny-skia gives premultiplied RGBA, which is what egui wants.
    let image = ColorImage::from_rgba_premultiplied([px as usize, px as usize], pixmap.data());
    let tex = ctx.load_texture(format!("icon-{key:?}"), image, TextureOptions::LINEAR);
    CACHE.with(|c| c.borrow_mut().insert(key, tex.clone()));
    Some(tex)
}

/// Paint `svg` into `rect` (square, in points) tinted `color`.
pub fn paint(ui: &egui::Ui, svg: &'static str, rect: Rect, color: Color32) {
    let px = (rect.width() * ui.ctx().pixels_per_point()).round().max(1.0) as u32;
    if let Some(tex) = texture(ui.ctx(), svg, px) {
        ui.painter().image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), color);
    }
}

/// Paint `svg` centred in `rect` at `size` points.
pub fn paint_centered(ui: &egui::Ui, svg: &'static str, rect: Rect, size: f32, color: Color32) {
    paint(ui, svg, Rect::from_center_size(rect.center(), egui::vec2(size, size)), color);
}
