//! A small safe layer over libmpv: one core (`Mpv`) driven by commands and
//! property reads, events pulled with `wait_event`, and an OpenGL render
//! context (`RenderContext`) for drawing video into the app's own GL surface.

mod ffi;

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt;
use std::ptr;

#[derive(Debug, Clone)]
pub struct Error {
    pub code: i32,
    pub context: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = unsafe { CStr::from_ptr(ffi::mpv_error_string(self.code)) };
        write!(f, "{}: {}", self.context, msg.to_string_lossy())
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

fn check(code: c_int, context: impl FnOnce() -> String) -> Result<()> {
    if code >= 0 { Ok(()) } else { Err(Error { code, context: context() }) }
}

fn cstr(s: &str) -> CString {
    // Paths and titles never legitimately contain NUL; drop any rather than fail.
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Flag,
    Int64,
    Double,
    String,
}

impl Format {
    fn raw(self) -> ffi::mpv_format {
        match self {
            Format::Flag => ffi::MPV_FORMAT_FLAG,
            Format::Int64 => ffi::MPV_FORMAT_INT64,
            Format::Double => ffi::MPV_FORMAT_DOUBLE,
            Format::String => ffi::MPV_FORMAT_STRING,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The property isn't available right now (e.g. `duration` with no file).
    None,
    Flag(bool),
    Int64(i64),
    Double(f64),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Eof,
    Stop,
    Quit,
    Error,
    Redirect,
    Other(i32),
}

#[derive(Debug, Clone)]
pub enum Event {
    Shutdown,
    Log { prefix: String, level: String, text: String },
    StartFile { entry_id: i64 },
    EndFile { entry_id: i64, reason: EndReason, error: i32 },
    FileLoaded,
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    Property { id: u64, name: String, value: Value },
    CommandReply { id: u64, error: i32 },
    Other(i32),
}

/// One mpv core. libmpv's client API is thread safe, so this can be shared.
pub struct Mpv {
    handle: *mut ffi::mpv_handle,
}

unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

impl Mpv {
    /// Create a core, apply `options` (which must be set before init, like
    /// `vo`), then initialise it.
    pub fn new(options: &[(&str, &str)]) -> Result<Self> {
        let handle = unsafe { ffi::mpv_create() };
        if handle.is_null() {
            return Err(Error { code: -1, context: "mpv_create".into() });
        }
        let mpv = Mpv { handle };
        for (name, value) in options {
            let (n, v) = (cstr(name), cstr(value));
            check(unsafe { ffi::mpv_set_option_string(handle, n.as_ptr(), v.as_ptr()) }, || {
                format!("option {name}={value}")
            })?;
        }
        check(unsafe { ffi::mpv_initialize(handle) }, || "mpv_initialize".into())?;
        Ok(mpv)
    }

    /// The client API version libmpv was built with, as (major, minor).
    pub fn api_version() -> (u32, u32) {
        let v = unsafe { ffi::mpv_client_api_version() } as u64;
        ((v >> 16) as u32, (v & 0xffff) as u32)
    }

    fn with_args<R>(args: &[&str], f: impl FnOnce(*mut *const c_char) -> R) -> R {
        let owned: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        f(ptrs.as_mut_ptr())
    }

    /// Run a command and wait for it, e.g. `["loadfile", path, "replace"]`.
    pub fn command(&self, args: &[&str]) -> Result<()> {
        let code = Self::with_args(args, |p| unsafe { ffi::mpv_command(self.handle, p) });
        check(code, || format!("command {args:?}"))
    }

    /// Queue a command without waiting; the result arrives as `CommandReply { id }`.
    pub fn command_async(&self, id: u64, args: &[&str]) -> Result<()> {
        let code =
            Self::with_args(args, |p| unsafe { ffi::mpv_command_async(self.handle, id, p) });
        check(code, || format!("command {args:?}"))
    }

    pub fn set_string(&self, name: &str, value: &str) -> Result<()> {
        let (n, v) = (cstr(name), cstr(value));
        let code = unsafe { ffi::mpv_set_property_string(self.handle, n.as_ptr(), v.as_ptr()) };
        check(code, || format!("set {name}={value}"))
    }

    pub fn set_flag(&self, name: &str, value: bool) -> Result<()> {
        let n = cstr(name);
        let mut v: c_int = value as c_int;
        let code = unsafe {
            ffi::mpv_set_property(
                self.handle,
                n.as_ptr(),
                ffi::MPV_FORMAT_FLAG,
                &mut v as *mut c_int as *mut c_void,
            )
        };
        check(code, || format!("set {name}={value}"))
    }

    pub fn set_f64(&self, name: &str, value: f64) -> Result<()> {
        let n = cstr(name);
        let mut v = value;
        let code = unsafe {
            ffi::mpv_set_property(
                self.handle,
                n.as_ptr(),
                ffi::MPV_FORMAT_DOUBLE,
                &mut v as *mut f64 as *mut c_void,
            )
        };
        check(code, || format!("set {name}={value}"))
    }

    pub fn get_f64(&self, name: &str) -> Option<f64> {
        let n = cstr(name);
        let mut v = 0f64;
        let code = unsafe {
            ffi::mpv_get_property(
                self.handle,
                n.as_ptr(),
                ffi::MPV_FORMAT_DOUBLE,
                &mut v as *mut f64 as *mut c_void,
            )
        };
        (code >= 0).then_some(v)
    }

    pub fn get_i64(&self, name: &str) -> Option<i64> {
        let n = cstr(name);
        let mut v = 0i64;
        let code = unsafe {
            ffi::mpv_get_property(
                self.handle,
                n.as_ptr(),
                ffi::MPV_FORMAT_INT64,
                &mut v as *mut i64 as *mut c_void,
            )
        };
        (code >= 0).then_some(v)
    }

    pub fn get_flag(&self, name: &str) -> Option<bool> {
        let n = cstr(name);
        let mut v: c_int = 0;
        let code = unsafe {
            ffi::mpv_get_property(
                self.handle,
                n.as_ptr(),
                ffi::MPV_FORMAT_FLAG,
                &mut v as *mut c_int as *mut c_void,
            )
        };
        (code >= 0).then_some(v != 0)
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        let n = cstr(name);
        unsafe {
            let p = ffi::mpv_get_property_string(self.handle, n.as_ptr());
            if p.is_null() {
                return None;
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            ffi::mpv_free(p as *mut c_void);
            Some(s)
        }
    }

    /// Get `PropertyChange` events for `name`, tagged with `id`.
    pub fn observe(&self, id: u64, name: &str, format: Format) -> Result<()> {
        let n = cstr(name);
        let code = unsafe { ffi::mpv_observe_property(self.handle, id, n.as_ptr(), format.raw()) };
        check(code, || format!("observe {name}"))
    }

    /// Ask for log messages at `level` and above ("error", "warn", "info", ...).
    pub fn request_log_messages(&self, level: &str) -> Result<()> {
        let l = cstr(level);
        check(unsafe { ffi::mpv_request_log_messages(self.handle, l.as_ptr()) }, || {
            "request_log_messages".into()
        })
    }

    /// Wait up to `timeout` seconds (negative = forever) for the next event.
    /// Only one thread should call this.
    pub fn wait_event(&self, timeout: f64) -> Option<Event> {
        unsafe {
            let ev = &*ffi::mpv_wait_event(self.handle, timeout);
            Some(match ev.event_id {
                ffi::MPV_EVENT_NONE => return None,
                ffi::MPV_EVENT_SHUTDOWN => Event::Shutdown,
                ffi::MPV_EVENT_LOG_MESSAGE => {
                    let m = &*(ev.data as *const ffi::mpv_event_log_message);
                    Event::Log {
                        prefix: lossy(m.prefix),
                        level: lossy(m.level),
                        text: lossy(m.text),
                    }
                }
                ffi::MPV_EVENT_START_FILE => {
                    let d = &*(ev.data as *const ffi::mpv_event_start_file);
                    Event::StartFile { entry_id: d.playlist_entry_id }
                }
                ffi::MPV_EVENT_END_FILE => {
                    let d = &*(ev.data as *const ffi::mpv_event_end_file);
                    let reason = match d.reason {
                        ffi::MPV_END_FILE_REASON_EOF => EndReason::Eof,
                        ffi::MPV_END_FILE_REASON_STOP => EndReason::Stop,
                        ffi::MPV_END_FILE_REASON_QUIT => EndReason::Quit,
                        ffi::MPV_END_FILE_REASON_ERROR => EndReason::Error,
                        ffi::MPV_END_FILE_REASON_REDIRECT => EndReason::Redirect,
                        r => EndReason::Other(r),
                    };
                    Event::EndFile { entry_id: d.playlist_entry_id, reason, error: d.error }
                }
                ffi::MPV_EVENT_FILE_LOADED => Event::FileLoaded,
                ffi::MPV_EVENT_VIDEO_RECONFIG => Event::VideoReconfig,
                ffi::MPV_EVENT_AUDIO_RECONFIG => Event::AudioReconfig,
                ffi::MPV_EVENT_SEEK => Event::Seek,
                ffi::MPV_EVENT_PLAYBACK_RESTART => Event::PlaybackRestart,
                ffi::MPV_EVENT_COMMAND_REPLY => {
                    Event::CommandReply { id: ev.reply_userdata, error: ev.error }
                }
                ffi::MPV_EVENT_PROPERTY_CHANGE => {
                    let p = &*(ev.data as *const ffi::mpv_event_property);
                    let value = if p.data.is_null() {
                        Value::None
                    } else {
                        match p.format {
                            ffi::MPV_FORMAT_FLAG => Value::Flag(*(p.data as *const c_int) != 0),
                            ffi::MPV_FORMAT_INT64 => Value::Int64(*(p.data as *const i64)),
                            ffi::MPV_FORMAT_DOUBLE => Value::Double(*(p.data as *const f64)),
                            ffi::MPV_FORMAT_STRING => {
                                Value::String(lossy(*(p.data as *const *const c_char)))
                            }
                            _ => Value::None,
                        }
                    };
                    Event::Property { id: ev.reply_userdata, name: lossy(p.name), value }
                }
                other => Event::Other(other),
            })
        }
    }

    /// Make a blocked `wait_event` return.
    pub fn wakeup(&self) {
        unsafe { ffi::mpv_wakeup(self.handle) }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe { ffi::mpv_terminate_destroy(self.handle) }
    }
}

unsafe fn lossy(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

type ProcAddress = dyn Fn(&CStr) -> *const c_void + Send + Sync;

/// mpv's OpenGL renderer. Create it, use it and drop it on the thread that owns
/// the GL context, and drop it before the `Mpv` it came from.
pub struct RenderContext {
    ctx: *mut ffi::mpv_render_context,
    // Kept alive for as long as mpv may call back into them.
    _get_proc: Box<Box<ProcAddress>>,
    _on_update: Box<Box<dyn Fn() + Send + Sync>>,
}

unsafe extern "C" fn get_proc_trampoline(ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    let f = unsafe { &*(ctx as *const Box<ProcAddress>) };
    f(unsafe { CStr::from_ptr(name) }) as *mut c_void
}

unsafe extern "C" fn update_trampoline(ctx: *mut c_void) {
    let f = unsafe { &*(ctx as *const Box<dyn Fn() + Send + Sync>) };
    f();
}

impl RenderContext {
    /// `get_proc` resolves GL function names; `on_update` runs on an mpv thread
    /// whenever a new frame is ready, and should just schedule a redraw.
    pub fn new(
        mpv: &Mpv,
        get_proc: Box<ProcAddress>,
        on_update: Box<dyn Fn() + Send + Sync>,
    ) -> Result<Self> {
        let get_proc = Box::new(get_proc);
        let on_update = Box::new(on_update);
        let mut init = ffi::mpv_opengl_init_params {
            get_proc_address: Some(get_proc_trampoline),
            get_proc_address_ctx: &*get_proc as *const Box<ProcAddress> as *mut c_void,
        };
        let mut advanced: c_int = 0;
        let mut params = [
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_API_TYPE,
                data: ffi::MPV_RENDER_API_TYPE_OPENGL.as_ptr() as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: &mut init as *mut _ as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_ADVANCED_CONTROL,
                data: &mut advanced as *mut c_int as *mut c_void,
            },
            ffi::mpv_render_param { type_: ffi::MPV_RENDER_PARAM_INVALID, data: ptr::null_mut() },
        ];
        let mut ctx = ptr::null_mut();
        check(
            unsafe { ffi::mpv_render_context_create(&mut ctx, mpv.handle, params.as_mut_ptr()) },
            || "mpv_render_context_create".into(),
        )?;
        unsafe {
            ffi::mpv_render_context_set_update_callback(
                ctx,
                Some(update_trampoline),
                &*on_update as *const Box<dyn Fn() + Send + Sync> as *mut c_void,
            );
        }
        Ok(RenderContext { ctx, _get_proc: get_proc, _on_update: on_update })
    }

    /// True if mpv has a new frame to draw. Call before `render`.
    pub fn needs_frame(&self) -> bool {
        unsafe { ffi::mpv_render_context_update(self.ctx) & ffi::MPV_RENDER_UPDATE_FRAME != 0 }
    }

    /// Draw the current frame into framebuffer `fbo` of `w`x`h` pixels. mpv
    /// letterboxes to fit. `flip_y` is for drawing straight to a window.
    pub fn render(&self, fbo: i32, w: i32, h: i32, flip_y: bool) -> Result<()> {
        let mut target = ffi::mpv_opengl_fbo { fbo, w, h, internal_format: 0 };
        let mut flip: c_int = flip_y as c_int;
        let mut block: c_int = 0;
        let mut params = [
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_OPENGL_FBO,
                data: &mut target as *mut _ as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_FLIP_Y,
                data: &mut flip as *mut c_int as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME,
                data: &mut block as *mut c_int as *mut c_void,
            },
            ffi::mpv_render_param { type_: ffi::MPV_RENDER_PARAM_INVALID, data: ptr::null_mut() },
        ];
        check(unsafe { ffi::mpv_render_context_render(self.ctx, params.as_mut_ptr()) }, || {
            "mpv_render_context_render".into()
        })
    }

    /// Tell mpv a frame was put on screen, which helps its frame timing.
    pub fn report_swap(&self) {
        unsafe { ffi::mpv_render_context_report_swap(self.ctx) }
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        unsafe {
            ffi::mpv_render_context_set_update_callback(self.ctx, None, ptr::null_mut());
            ffi::mpv_render_context_free(self.ctx);
        }
    }
}
