//! The platform layer: SDL3 (3.4 or later, built from source with the game) for the window, the
//! GL context and input, driven by SDL3's main callbacks (engine-stack design sections 4 and 6).
//!
//! The frame is a callback, never a blocking loop: SDL calls `App::frame` once per frame and it
//! returns, which is what lets the same client run where a browser drives the frame (design 10a).
//! The context is OpenGL ES 3.0 through EGL on Linux and the Pi (SDL's KMS/DRM backend with its
//! atomic path on a Pi 5), desktop OpenGL 4.1 core on Windows and macOS (sokol_gfx's GLCORE).

use sdl3_sys::everything as sdl;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::process::ExitCode;

/// How the window is opened.
#[derive(Clone, Debug)]
pub struct WindowConfig {
    /// The window's title.
    pub title: String,
    /// Width in pixels (ignored full screen, where the display decides).
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Full screen (the Pi client) or a window.
    pub fullscreen: bool,
    /// Draw without showing anything: SDL's offscreen video driver, for captures and tests in a
    /// session with no display (a cloud container renders through Mesa's llvmpipe).
    pub headless: bool,
    /// Wait for the display's refresh between frames.
    pub vsync: bool,
}

/// What the app wants after a callback.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Keep running.
    Continue,
    /// Stop, successfully.
    Done,
    /// Stop with an error.
    Fail,
}

/// SDL keycodes the apps name (SDL3's `SDLK_*`).
pub mod keys {
    use sdl3_sys::everything as sdl;
    /// Escape.
    pub const ESCAPE: u32 = sdl::SDLK_ESCAPE.0;
    /// The 1 key.
    pub const N1: u32 = sdl::SDLK_1.0;
    /// The 2 key.
    pub const N2: u32 = sdl::SDLK_2.0;
    /// The 3 key.
    pub const N3: u32 = sdl::SDLK_3.0;
    /// F12.
    pub const F12: u32 = sdl::SDLK_F12.0;
    /// W.
    pub const W: u32 = sdl::SDLK_W.0;
    /// A.
    pub const A: u32 = sdl::SDLK_A.0;
    /// S.
    pub const S: u32 = sdl::SDLK_S.0;
    /// D.
    pub const D: u32 = sdl::SDLK_D.0;
    /// Q.
    pub const Q: u32 = sdl::SDLK_Q.0;
    /// E.
    pub const E: u32 = sdl::SDLK_E.0;
    /// Space.
    pub const SPACE: u32 = sdl::SDLK_SPACE.0;
    /// C.
    pub const C: u32 = sdl::SDLK_C.0;
    /// Left shift.
    pub const LSHIFT: u32 = sdl::SDLK_LSHIFT.0;
    /// Tab.
    pub const TAB: u32 = sdl::SDLK_TAB.0;
    /// F: walk or fly.
    pub const F: u32 = sdl::SDLK_F.0;
}

/// An input event, as the app sees it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event {
    /// The window was closed or the program was asked to quit.
    Quit,
    /// A key went down: its SDL keycode (not repeated while held).
    KeyDown(u32),
    /// A key came up: its SDL keycode.
    KeyUp(u32),
    /// The mouse moved by this many pixels (raw, not smoothed: CLAUDE.md's mouse rule in Pale-Blue-Dot).
    MouseMotion(f32, f32),
    /// A mouse button went down.
    MouseDown,
}

/// One frame's facts.
#[derive(Copy, Clone, Debug)]
pub struct Frame {
    /// The output's size in pixels.
    pub width: u32,
    /// The output's height in pixels.
    pub height: u32,
    /// Real seconds since the last frame began (0 on the first).
    pub elapsed_s: f64,
    /// Frames begun so far, this one included.
    pub index: u64,
}

/// A program driven by the platform's callbacks.
pub trait App {
    /// Draw one frame and return. The platform shows it afterwards.
    fn frame(&mut self, frame: &Frame) -> Flow;
    /// Handle one input event.
    fn event(&mut self, event: Event) -> Flow {
        if event == Event::Quit {
            Flow::Done
        } else {
            Flow::Continue
        }
    }
}

/// Facts about the GL context, for reports.
#[derive(Clone, Debug)]
pub struct GlInfo {
    /// SDL's video driver (`kmsdrm` on the Pi, `x11`, `wayland`, `offscreen`).
    pub video_driver: String,
    /// GL_VERSION.
    pub version: String,
    /// GL_RENDERER.
    pub renderer: String,
}

type MakeApp = Box<dyn FnOnce(&GlInfo) -> Result<Box<dyn App>, String>>;

struct State {
    window: *mut sdl::SDL_Window,
    context: sdl::SDL_GLContext,
    app: Box<dyn App>,
    last_ns: u64,
    index: u64,
}

thread_local! {
    static PENDING: RefCell<Option<(WindowConfig, MakeApp)>> = const { RefCell::new(None) };
    static WINDOW: std::cell::Cell<*mut sdl::SDL_Window> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// Capture the mouse for looking around (hidden, relative motion) or let it go. Call from the app's
/// callbacks; does nothing before the window exists.
pub fn capture_mouse(on: bool) {
    let w = WINDOW.with(std::cell::Cell::get);
    if !w.is_null() {
        // SAFETY: the window is alive between app_init and app_quit, and this runs on the main thread.
        unsafe { sdl::SDL_SetWindowRelativeMouseMode(w, on) };
    }
}

fn sdl_error() -> String {
    // SAFETY: SDL_GetError returns a valid, nul-terminated string owned by SDL.
    unsafe { CStr::from_ptr(sdl::SDL_GetError()).to_string_lossy().into_owned() }
}

extern "C" {
    fn glGetString(name: u32) -> *const u8;
}

fn gl_string(name: u32) -> String {
    // SAFETY: called with a current context; glGetString returns a static string or null.
    let p = unsafe { glGetString(name) };
    if p.is_null() {
        String::new()
    } else {
        // SAFETY: a non-null result is a nul-terminated string owned by GL.
        unsafe { CStr::from_ptr(p.cast()).to_string_lossy().into_owned() }
    }
}

/// Run `make`'s app under SDL3's main callbacks until it finishes. `make` is called once the
/// window and GL context exist and are current.
pub fn run(cfg: WindowConfig, make: impl FnOnce(&GlInfo) -> Result<Box<dyn App>, String> + 'static) -> ExitCode {
    PENDING.with(|p| *p.borrow_mut() = Some((cfg, Box::new(make))));
    let args: Vec<CString> = std::env::args().map(|a| CString::new(a).unwrap_or_default()).collect();
    let mut argv: Vec<*mut c_char> = args.iter().map(|a| a.as_ptr() as *mut c_char).collect();
    // SAFETY: argv points at nul-terminated strings that outlive the call; the callbacks below
    // match SDL's signatures.
    let code = unsafe {
        sdl::SDL_EnterAppMainCallbacks(
            argv.len() as c_int,
            argv.as_mut_ptr(),
            Some(app_init),
            Some(app_iterate),
            Some(app_event),
            Some(app_quit),
        )
    };
    if code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

unsafe extern "C" fn app_init(appstate: *mut *mut c_void, _argc: c_int, _argv: *mut *mut c_char) -> sdl::SDL_AppResult {
    let Some((cfg, make)) = PENDING.with(|p| p.borrow_mut().take()) else {
        return sdl::SDL_APP_FAILURE;
    };
    match open(&cfg) {
        Ok((window, context, info)) => match make(&info) {
            Ok(app) => {
                // SAFETY: SDL_GetTicksNS has no preconditions once SDL is initialised.
                let last_ns = unsafe { sdl::SDL_GetTicksNS() };
                WINDOW.with(|c| c.set(window));
                let state = Box::new(State { window, context, app, last_ns, index: 0 });
                // SAFETY: appstate is SDL's slot for our state; it hands it back to every callback.
                unsafe { *appstate = Box::into_raw(state).cast() };
                sdl::SDL_APP_CONTINUE
            }
            Err(e) => {
                eprintln!("sc-client: {e}");
                sdl::SDL_APP_FAILURE
            }
        },
        Err(e) => {
            eprintln!("sc-client: {e}");
            sdl::SDL_APP_FAILURE
        }
    }
}

fn open(cfg: &WindowConfig) -> Result<(*mut sdl::SDL_Window, sdl::SDL_GLContext, GlInfo), String> {
    // SAFETY: plain SDL calls in the documented order on the main thread; every pointer is checked.
    unsafe {
        if cfg.headless {
            sdl::SDL_SetHint(sdl::SDL_HINT_VIDEO_DRIVER, c"offscreen".as_ptr());
        }
        // The Pi 5's KMS/DRM needs SDL's atomic path (design section 4).
        sdl::SDL_SetHint(c"SDL_KMSDRM_ATOMIC".as_ptr(), c"1".as_ptr());
        if !sdl::SDL_Init(sdl::SDL_INIT_VIDEO) {
            return Err(format!("SDL_Init: {}", sdl_error()));
        }
        let gl_attr = |a: sdl::SDL_GLAttr, v: c_int| sdl::SDL_GL_SetAttribute(a, v);
        if cfg!(any(target_os = "windows", target_os = "macos")) {
            gl_attr(sdl::SDL_GL_CONTEXT_PROFILE_MASK, sdl::SDL_GL_CONTEXT_PROFILE_CORE.0);
            gl_attr(sdl::SDL_GL_CONTEXT_MAJOR_VERSION, 4);
            gl_attr(sdl::SDL_GL_CONTEXT_MINOR_VERSION, 1);
        } else {
            gl_attr(sdl::SDL_GL_CONTEXT_PROFILE_MASK, sdl::SDL_GL_CONTEXT_PROFILE_ES.0);
            gl_attr(sdl::SDL_GL_CONTEXT_MAJOR_VERSION, 3);
            gl_attr(sdl::SDL_GL_CONTEXT_MINOR_VERSION, 0);
        }
        gl_attr(sdl::SDL_GL_DOUBLEBUFFER, 1);
        gl_attr(sdl::SDL_GL_DEPTH_SIZE, 24);
        gl_attr(sdl::SDL_GL_STENCIL_SIZE, 8);
        let mut flags = sdl::SDL_WINDOW_OPENGL;
        if cfg.fullscreen {
            flags |= sdl::SDL_WINDOW_FULLSCREEN;
        }
        let title = CString::new(cfg.title.clone()).unwrap_or_default();
        let window = sdl::SDL_CreateWindow(title.as_ptr(), cfg.width as c_int, cfg.height as c_int, flags);
        if window.is_null() {
            return Err(format!("SDL_CreateWindow: {}", sdl_error()));
        }
        let context = sdl::SDL_GL_CreateContext(window);
        if context.is_null() {
            return Err(format!("SDL_GL_CreateContext: {}", sdl_error()));
        }
        sdl::SDL_GL_MakeCurrent(window, context);
        sdl::SDL_GL_SetSwapInterval(if cfg.vsync { 1 } else { 0 });
        let driver = sdl::SDL_GetCurrentVideoDriver();
        let video_driver =
            if driver.is_null() { String::new() } else { CStr::from_ptr(driver).to_string_lossy().into_owned() };
        const GL_RENDERER: u32 = 0x1F01;
        const GL_VERSION: u32 = 0x1F02;
        let info = GlInfo { video_driver, version: gl_string(GL_VERSION), renderer: gl_string(GL_RENDERER) };
        Ok((window, context, info))
    }
}

unsafe extern "C" fn app_iterate(appstate: *mut c_void) -> sdl::SDL_AppResult {
    // SAFETY: appstate is the State app_init boxed; SDL calls back on one thread.
    let s = unsafe { &mut *appstate.cast::<State>() };
    let (mut w, mut h) = (0, 0);
    // SAFETY: the window lives until app_quit.
    let now = unsafe {
        sdl::SDL_GetWindowSizeInPixels(s.window, &mut w, &mut h);
        sdl::SDL_GetTicksNS()
    };
    let elapsed_s = if s.index == 0 { 0.0 } else { (now - s.last_ns) as f64 * 1e-9 };
    s.last_ns = now;
    s.index += 1;
    let flow = s.app.frame(&Frame { width: w.max(1) as u32, height: h.max(1) as u32, elapsed_s, index: s.index });
    // SAFETY: as above.
    unsafe { sdl::SDL_GL_SwapWindow(s.window) };
    result(flow)
}

unsafe extern "C" fn app_event(appstate: *mut c_void, event: *mut sdl::SDL_Event) -> sdl::SDL_AppResult {
    // SAFETY: as app_iterate; event is valid for this call.
    let (s, e) = unsafe { (&mut *appstate.cast::<State>(), &*event) };
    // SAFETY: the type field is shared by every member of the union.
    let kind = unsafe { e.r#type };
    let ev = if kind == sdl::SDL_EVENT_QUIT.0 {
        Some(Event::Quit)
    } else if kind == sdl::SDL_EVENT_KEY_DOWN.0 {
        // SAFETY: a key event's payload is the keyboard member.
        let k = unsafe { e.key };
        (!k.repeat).then_some(Event::KeyDown(k.key.0))
    } else if kind == sdl::SDL_EVENT_KEY_UP.0 {
        // SAFETY: as above.
        Some(Event::KeyUp(unsafe { e.key.key.0 }))
    } else if kind == sdl::SDL_EVENT_MOUSE_MOTION.0 {
        // SAFETY: a motion event's payload is the motion member.
        let m = unsafe { e.motion };
        Some(Event::MouseMotion(m.xrel, m.yrel))
    } else if kind == sdl::SDL_EVENT_MOUSE_BUTTON_DOWN.0 {
        Some(Event::MouseDown)
    } else {
        None
    };
    ev.map_or(sdl::SDL_APP_CONTINUE, |ev| result(s.app.event(ev)))
}

unsafe extern "C" fn app_quit(appstate: *mut c_void, _result: sdl::SDL_AppResult) {
    if appstate.is_null() {
        return;
    }
    // SAFETY: appstate is the State app_init boxed; this is the last callback. The app (and with it
    // the renderer) is dropped while the context is still current, then the context goes.
    unsafe {
        WINDOW.with(|c| c.set(std::ptr::null_mut()));
        let s = Box::from_raw(appstate.cast::<State>());
        let (window, context) = (s.window, s.context);
        drop(s);
        sdl::SDL_GL_DestroyContext(context);
        sdl::SDL_DestroyWindow(window);
    }
}

fn result(f: Flow) -> sdl::SDL_AppResult {
    match f {
        Flow::Continue => sdl::SDL_APP_CONTINUE,
        Flow::Done => sdl::SDL_APP_SUCCESS,
        Flow::Fail => sdl::SDL_APP_FAILURE,
    }
}
