//! Video. mpv draws each frame into a texture we own (through an FBO), and the
//! UI shows that texture like any other image. The drawing happens in a paint
//! callback queued just before the image, so it runs inside egui's own paint
//! pass with the GL context current, and the frame shows up without a lag.

use std::num::NonZeroU32;
use std::sync::Arc;

use eframe::egui;
use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use lumen_playback::mpv::{Mpv, RenderContext};
use parking_lot::Mutex;

struct Gl {
    render: RenderContext,
    fbo: glow::Framebuffer,
    tex: glow::Texture,
    size: (i32, i32),
}

// Only ever touched on the GL thread, from paint callbacks and drop.
unsafe impl Send for Gl {}
unsafe impl Sync for Gl {}

pub struct VideoSurface {
    gl: Arc<Mutex<Gl>>,
    texture: Option<egui::TextureId>,
    tex: glow::Texture,
}

impl VideoSurface {
    /// Set up mpv's renderer. Must run with the GL context current, which is
    /// the case while eframe builds the app.
    pub fn new(
        mpv: &Mpv,
        gl: &glow::Context,
        get_proc: Arc<dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void + Send + Sync>,
        ctx: egui::Context,
    ) -> Result<Self, String> {
        let render = RenderContext::new(
            mpv,
            Box::new(move |name| get_proc(name)),
            Box::new(move || ctx.request_repaint()),
        )
        .map_err(|e| e.to_string())?;
        unsafe {
            let tex = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            for (k, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
            }
            alloc(gl, 16, 16);
            let fbo = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(tex),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            Ok(VideoSurface {
                gl: Arc::new(Mutex::new(Gl { render, fbo, tex, size: (16, 16) })),
                texture: None,
                tex,
            })
        }
    }

    /// Draw the current video frame into `rect`.
    pub fn show(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame, rect: egui::Rect) {
        // Registered once; the texture object keeps its name when resized.
        let texture = *self.texture.get_or_insert_with(|| frame.register_native_glow_texture(self.tex));
        let gl = self.gl.clone();
        let callback = egui_glow::CallbackFn::new(move |info, painter| {
            let px = info.viewport_in_pixels();
            gl.lock().draw(painter.gl(), px.width_px.max(1), px.height_px.max(1));
        });
        ui.painter().add(egui::PaintCallback { rect, callback: Arc::new(callback) });
        ui.painter().image(
            texture,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    /// Free the GL side. Call from `on_exit`, before mpv itself goes away.
    pub fn destroy(self, gl: &glow::Context) {
        if let Ok(inner) = Arc::try_unwrap(self.gl) {
            let inner = inner.into_inner();
            unsafe { gl.delete_framebuffer(inner.fbo) };
            drop(inner.render);
            // The texture belongs to egui's painter now and is freed with it.
            let _ = inner.tex;
        }
    }
}

unsafe fn alloc(gl: &glow::Context, w: i32, h: i32) {
    unsafe {
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            w,
            h,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
    }
}

impl Gl {
    fn draw(&mut self, gl: &glow::Context, w: i32, h: i32) {
        unsafe {
            let resized = self.size != (w, h);
            if resized {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
                alloc(gl, w, h);
                gl.bind_texture(glow::TEXTURE_2D, None);
                self.size = (w, h);
            }
            if !self.render.needs_frame() && !resized {
                return;
            }
            let previous = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING);
            // mpv's unflipped output already lands the right way up in a
            // texture egui samples top-down. (flip_y is for drawing straight
            // to the window.)
            if let Err(e) = self.render.render(self.fbo.0.get() as i32, w, h, false) {
                log::warn!("video: {e}");
            }
            // Put back what egui expects; mpv leaves its own state behind.
            let previous = NonZeroU32::new(previous as u32).map(glow::NativeFramebuffer);
            gl.bind_framebuffer(glow::FRAMEBUFFER, previous);
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }
}
