//! Browser windows: Win32 frame, layout and the per-window tab model.
//!
//! The window has no system title bar. Its client area is split into the *chrome* view (tab
//! strip, toolbar and bookmarks bar, rendered from `mizu://chrome/`) and the active tab's view.
//! Dragging and the caption buttons are handled by the chrome view via CSS `app-region`.

use crate::app;
use crate::flyout::Flyout;
use crate::queue::{self, Queued};
use crate::win::{self, dpi_scale, wide};
use mizu_core::ids::{TabId, WindowId};
use mizu_engine::{NewWindowRequest, RECT, View, ViewKey};
use mizu_tabs::TabStrip;
use std::collections::HashMap;
use std::time::Instant;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, HBRUSH, PAINTSTRUCT, ScreenToClient,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const CLASS: &str = "MizuBrowserWindow";
/// Thin strip above the chrome view that lets the user resize from the top edge.
const TOP_RESIZE_BORDER: f64 = 4.0;

pub struct TabState {
    pub view: Option<View>,
    pub creating: bool,
    /// URL to load once the view exists (lazy tabs, restored sessions).
    pub pending_url: Option<String>,
    pub pending_new_window: Option<NewWindowRequest>,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub favicon: Option<String>,
    pub audible: bool,
    pub muted: bool,
    pub crashed: bool,
    pub sleeping: bool,
    /// Engine memory target lowered (efficiency mode, long-idle background tab).
    pub low_memory: bool,
    pub zoom: f64,
    pub last_active: Instant,
    /// Opened only to retry a download: close it once the download starts.
    pub close_if_download: bool,
    pub http_status: i32,
}

impl TabState {
    pub fn new(pending_url: Option<String>) -> Self {
        Self {
            view: None,
            creating: false,
            pending_url,
            pending_new_window: None,
            loading: false,
            can_go_back: false,
            can_go_forward: false,
            favicon: None,
            audible: false,
            muted: false,
            crashed: false,
            sleeping: false,
            low_memory: false,
            zoom: 1.0,
            last_active: Instant::now(),
            close_if_download: false,
            http_status: 0,
        }
    }
}

#[derive(Default)]
pub struct FullscreenState {
    /// F11 browser fullscreen.
    pub browser: bool,
    /// Page element fullscreen (video players).
    pub element: bool,
    pub saved_rect: Option<windows::Win32::Foundation::RECT>,
    pub saved_style: isize,
    pub was_maximized: bool,
}

impl FullscreenState {
    pub fn active(&self) -> bool {
        self.browser || self.element
    }
}

pub struct BrowserWindow {
    pub id: WindowId,
    pub hwnd: HWND,
    pub private: bool,
    pub strip: TabStrip,
    pub tabs: HashMap<TabId, TabState>,
    pub chrome: Option<View>,
    pub chrome_key: ViewKey,
    pub chrome_ready: bool,
    /// Height of the chrome view in CSS pixels, reported by the chrome itself.
    pub chrome_height: f64,
    pub omnibox_anchor: mizu_ui::Anchor,
    pub flyout: Flyout,
    pub fullscreen: FullscreenState,
    pub suggestions: Vec<crate::omnibox::Suggestion>,
    /// Text the suggestions were made for (remote results for older text are dropped).
    pub omnibox_query: String,
    pub state_dirty: bool,
    pub closing: bool,
    /// The native window is shown once the browser UI has loaded (no empty frame on start).
    pub shown: bool,
    pub show_maximized: bool,
    /// Last WM_ACTIVATE state, so focus is moved only on a real activation.
    pub activated: bool,
    pub created_at: std::time::Instant,
    /// Pre-created, hidden engine view handed to the next new tab (makes new tabs instant).
    pub spare: Option<View>,
    pub spare_creating: bool,
    /// Vertical tabs: a second browser-UI view (`mizu://chrome/?sidebar`) left of the page.
    pub sidebar: Option<View>,
    pub sidebar_key: ViewKey,
    pub sidebar_ready: bool,
    pub sidebar_creating: bool,
    pub side_tabs: bool,
    pub sidebar_collapsed: bool,
    /// Expanded sidebar width in CSS pixels.
    pub sidebar_width: f64,
}

/// Collapsed (icons only) sidebar width in CSS pixels.
const SIDEBAR_COLLAPSED: f64 = 56.0;

impl BrowserWindow {
    /// Shows the native window if it is still hidden.
    pub fn reveal(&mut self) {
        if !self.shown {
            self.shown = true;
            show_native(self.hwnd, self.show_maximized);
            self.layout();
        }
    }
}

impl BrowserWindow {
    pub fn active_tab(&self) -> Option<TabId> {
        self.strip.active()
    }

    pub fn active_view(&self) -> Option<View> {
        self.active_tab().and_then(|t| self.tabs.get(&t)).and_then(|t| t.view.clone())
    }

    pub fn is_maximized(&self) -> bool {
        // SAFETY: valid window handle.
        unsafe { IsZoomed(self.hwnd).as_bool() }
    }

    /// Space above the browser UI. With visual hosting our window procedure sees the top
    /// resize border itself (WM_NCHITTEST), so the UI can start at the very top; a gap here
    /// showed the translucent frame as a strip above the toolbar.
    fn top_inset(&self) -> i32 {
        0
    }

    /// Width of the vertical tabs sidebar in physical pixels (0 when not shown).
    fn sidebar_px(&self) -> i32 {
        if !self.side_tabs || !self.sidebar_ready || self.fullscreen.active() {
            return 0;
        }
        let css = if self.sidebar_collapsed { SIDEBAR_COLLAPSED } else { self.sidebar_width };
        (css * dpi_scale(self.hwnd)).round() as i32
    }

    /// Positions the chrome, the sidebar and the active tab view.
    pub fn layout(&self) {
        let rc = win::client_rect(self.hwnd);
        let scale = dpi_scale(self.hwnd);
        let fullscreen = self.fullscreen.active();
        let top = self.top_inset();
        let chrome_px = if fullscreen { 0 } else { (self.chrome_height * scale).round() as i32 };
        if let Some(chrome) = &self.chrome {
            chrome.set_visible(!fullscreen && self.chrome_ready);
            chrome.set_bounds(RECT { left: 0, top, right: rc.right, bottom: top + chrome_px });
        }
        let side = self.sidebar_px();
        if let Some(sidebar) = &self.sidebar {
            sidebar.set_visible(side > 0);
            sidebar.set_bounds(RECT { left: 0, top: top + chrome_px, right: side, bottom: rc.bottom });
        }
        if let Some(view) = self.active_view() {
            view.set_bounds(self.content_rect());
        }
    }

    pub fn content_rect(&self) -> RECT {
        let rc = win::client_rect(self.hwnd);
        let scale = dpi_scale(self.hwnd);
        let chrome_px = if self.fullscreen.active() { 0 } else { (self.chrome_height * scale).round() as i32 };
        let top = self.top_inset() + chrome_px;
        RECT { left: self.sidebar_px(), top, right: rc.right, bottom: rc.bottom }
    }

    pub fn post_chrome(&self, json: &str) {
        if let (Some(c), true) = (&self.chrome, self.chrome_ready) {
            c.post_json(json);
        }
        if let (Some(s), true) = (&self.sidebar, self.sidebar_ready) {
            s.post_json(json);
        }
    }

    /// Enters or leaves fullscreen (F11 or page element) by resizing to the monitor.
    pub fn set_fullscreen(&mut self, browser: Option<bool>, element: Option<bool>) {
        let was = self.fullscreen.active();
        if let Some(b) = browser {
            self.fullscreen.browser = b;
        }
        if let Some(e) = element {
            self.fullscreen.element = e;
        }
        let now = self.fullscreen.active();
        if was == now {
            self.layout();
            return;
        }
        // SAFETY: window style/position calls on our own window.
        unsafe {
            if now {
                let mut rc = Default::default();
                let _ = GetWindowRect(self.hwnd, &mut rc);
                self.fullscreen.saved_rect = Some(rc);
                self.fullscreen.was_maximized = self.is_maximized();
                self.fullscreen.saved_style = GetWindowLongPtrW(self.hwnd, GWL_STYLE);
                let monitor = windows::Win32::Graphics::Gdi::MonitorFromWindow(
                    self.hwnd,
                    windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
                );
                let mut mi = windows::Win32::Graphics::Gdi::MONITORINFO {
                    cbSize: size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
                    ..Default::default()
                };
                let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut mi);
                let style = self.fullscreen.saved_style & !((WS_CAPTION.0 | WS_THICKFRAME.0) as isize);
                SetWindowLongPtrW(self.hwnd, GWL_STYLE, style);
                let r = mi.rcMonitor;
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOP),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_FRAMECHANGED | SWP_NOOWNERZORDER,
                );
            } else {
                SetWindowLongPtrW(self.hwnd, GWL_STYLE, self.fullscreen.saved_style);
                if let Some(r) = self.fullscreen.saved_rect.take() {
                    let _ = SetWindowPos(
                        self.hwnd,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER,
                    );
                }
                if self.fullscreen.was_maximized {
                    let _ = ShowWindow(self.hwnd, SW_MAXIMIZE);
                }
            }
        }
        self.layout();
        self.state_dirty = true;
    }
}

pub fn register_class() {
    // SAFETY: class registration with a static window procedure.
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let icon = LoadIconW(Some(instance.into()), windows_core::PCWSTR(1 as _)).unwrap_or_default();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hIcon: icon,
            hIconSm: icon,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: windows_core::PCWSTR(wide(CLASS).as_ptr()),
            ..Default::default()
        };
        let name = wide(CLASS);
        let class = WNDCLASSEXW { lpszClassName: windows_core::PCWSTR(name.as_ptr()), ..class };
        RegisterClassExW(&class);
    }
}

/// Creates the native window (hidden until the first layout).
pub fn create_native(id: WindowId, title: &str, placement: Option<(i32, i32, i32, i32, bool)>) -> HWND {
    // SAFETY: CreateWindowExW with a registered class; the id is stored for routing.
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let (x, y, w, h) = match placement {
            Some((x, y, w, h, _)) if w >= 400 && h >= 300 => (x, y, w, h),
            _ => (CW_USEDEFAULT, CW_USEDEFAULT, 1280, 820),
        };
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            &wide(CLASS),
            &wide(title),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .unwrap_or_default();
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, id.0 as isize);
        // Force WM_NCCALCSIZE so the custom frame takes effect immediately.
        let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);
        hwnd
    }
}

pub fn show_native(hwnd: HWND, maximized: bool) {
    // SAFETY: valid window handle.
    unsafe {
        let _ = ShowWindow(hwnd, if maximized { SW_SHOWMAXIMIZED } else { SW_SHOWNORMAL });
        let _ = windows::Win32::Graphics::Gdi::UpdateWindow(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
}

fn window_id(hwnd: HWND) -> Option<WindowId> {
    // SAFETY: reading our own user data slot.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    (raw != 0).then_some(WindowId(raw as u64))
}

thread_local! {
    static FRAME_BRUSH: std::cell::Cell<(u32, isize)> = const { std::cell::Cell::new((u32::MAX, 0)) };
}

/// Brush with the current frame colour (cached; recreated when the colour changes).
fn frame_brush(rgb: [u8; 3]) -> HBRUSH {
    let colour = u32::from(rgb[0]) | (u32::from(rgb[1]) << 8) | (u32::from(rgb[2]) << 16);
    FRAME_BRUSH.with(|c| {
        let (cached, brush) = c.get();
        if cached == colour {
            return HBRUSH(brush as *mut _);
        }
        // SAFETY: creating/deleting GDI brushes we own.
        unsafe {
            if brush != 0 {
                let _ = DeleteObject(HBRUSH(brush as *mut _).into());
            }
            let b = CreateSolidBrush(windows::Win32::Foundation::COLORREF(colour));
            c.set((colour, b.0 as isize));
            b
        }
    })
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let id = window_id(hwnd);
    // Views are visually hosted: mouse input arrives here and goes to the view under it.
    if let Some(r) = mizu_engine::host::handle_input(hwnd, msg, wparam, lparam) {
        return r;
    }
    // SAFETY: window procedure called by the system with valid arguments.
    unsafe {
        match msg {
            WM_NCDESTROY => {
                mizu_engine::host::release_window(hwnd);
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            WM_NCCALCSIZE if wparam.0 != 0 => {
                // Remove the title bar but keep the system resize borders on the other edges.
                let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
                let original_top = params.rgrc[0].top;
                let r = DefWindowProcW(hwnd, msg, wparam, lparam);
                params.rgrc[0].top = original_top;
                if IsZoomed(hwnd).as_bool() {
                    let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd);
                    let frame = GetSystemMetricsForDpi(SM_CYFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                    params.rgrc[0].top += frame;
                }
                return r;
            }
            WM_NCHITTEST => {
                let r = DefWindowProcW(hwnd, msg, wparam, lparam);
                if r.0 as u32 == HTCLIENT {
                    let mut pt = POINT {
                        x: (lparam.0 & 0xffff) as i16 as i32,
                        y: ((lparam.0 >> 16) & 0xffff) as i16 as i32,
                    };
                    let _ = ScreenToClient(hwnd, &mut pt);
                    let border = (TOP_RESIZE_BORDER * dpi_scale(hwnd)).round() as i32;
                    if pt.y < border && !IsZoomed(hwnd).as_bool() {
                        return LRESULT(HTTOP as isize);
                    }
                    // `app-region: drag` areas of the browser UI move the window.
                    if let Some(ht) = mizu_engine::host::non_client_region(hwnd, pt) {
                        return LRESULT(ht as isize);
                    }
                }
                return r;
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let (rgb, mica) = app::with(|a| {
                    let private = id.and_then(|i| a.window(i)).is_some_and(|w| w.private);
                    (
                        crate::theme::frame_rgb(&a.settings, private),
                        a.settings.appearance.translucency && !private,
                    )
                })
                .unwrap_or(([0x14, 0x1a, 0x22], false));
                let brush = if mica {
                    HBRUSH(windows::Win32::Graphics::Gdi::GetStockObject(windows::Win32::Graphics::Gdi::BLACK_BRUSH).0)
                } else {
                    frame_brush(rgb)
                };
                FillRect(hdc, &ps.rcPaint, brush);
                let _ = EndPaint(hwnd, &ps);
                return LRESULT(0);
            }
            WM_ERASEBKGND => return LRESULT(1),
            // Only real size changes: WM_WINDOWPOSCHANGED also fires for z-order/activation
            // changes (e.g. when a flyout opens), which must not close the flyout.
            WM_SIZE => {
                if let Some(id) = id {
                    if app::with(|a| a.on_window_resized(id)).is_none() {
                        queue::with_app(move |a| a.on_window_resized(id));
                    }
                }
                return LRESULT(0);
            }
            WM_MOVE => {
                if let Some(id) = id {
                    queue::with_app(move |a| a.on_window_moved(id));
                }
                return LRESULT(0);
            }
            WM_ACTIVATE => {
                if let Some(id) = id {
                    let active = (wparam.0 & 0xffff) != WA_INACTIVE as usize;
                    queue::with_app(move |a| a.on_window_activated(id, active));
                }
                return LRESULT(0);
            }
            WM_GETMINMAXINFO => {
                let info = &mut *(lparam.0 as *mut MINMAXINFO);
                let s = dpi_scale(hwnd);
                info.ptMinTrackSize.x = (520.0 * s) as i32;
                info.ptMinTrackSize.y = (360.0 * s) as i32;
                return LRESULT(0);
            }
            WM_DPICHANGED => {
                let r = &*(lparam.0 as *const windows::Win32::Foundation::RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                return LRESULT(0);
            }
            WM_SETTINGCHANGE => {
                queue::with_app(|a| a.on_system_theme_changed());
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            WM_CLOSE => {
                if let Some(id) = id {
                    queue::push(Queued::WithApp(Box::new(move |a| a.request_close_window(id))));
                }
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

pub fn destroy_native(hwnd: HWND) {
    // SAFETY: destroying our own window; user data cleared first so late messages are ignored.
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        let _ = DestroyWindow(hwnd);
    }
}

pub fn set_title(hwnd: HWND, title: &str) {
    // SAFETY: valid window and string.
    unsafe {
        let _ = SetWindowTextW(hwnd, &wide(title));
    }
}

pub fn placement(hwnd: HWND) -> (i32, i32, i32, i32, bool) {
    let mut wp = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    // SAFETY: wp is properly sized.
    unsafe {
        let _ = GetWindowPlacement(hwnd, &mut wp);
    }
    let r = wp.rcNormalPosition;
    (r.left, r.top, r.right - r.left, r.bottom - r.top, wp.showCmd == SW_SHOWMAXIMIZED.0 as u32)
}

pub fn window_action(hwnd: HWND, action: mizu_ui::protocol::WindowAction) {
    use mizu_ui::protocol::WindowAction;
    // SAFETY: standard window commands on our own window.
    unsafe {
        match action {
            WindowAction::Minimize => {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
            WindowAction::Maximize => {
                let _ = ShowWindow(hwnd, if IsZoomed(hwnd).as_bool() { SW_RESTORE } else { SW_MAXIMIZE });
            }
            WindowAction::Close => {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            WindowAction::DragStart | WindowAction::Fullscreen => {}
        }
    }
}

pub fn focus_window(hwnd: HWND) {
    // SAFETY: valid window handle.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}
