//! Flyouts: menus, panels, prompts and omnibox suggestions.
//!
//! Web content is a native child window, so HTML in the chrome view cannot overlap it. Flyouts
//! are therefore a separate rounded popup window (owned by the browser window) hosting one more
//! privileged view that renders `mizu://flyout/`. The view is created once and reused, so
//! opening a menu is instant.

use crate::win::{dpi_scale, round_corners, wide};
use mizu_core::ids::WindowId;
use mizu_engine::{RECT, View, ViewKey};
use mizu_ui::{Anchor, protocol::FlyoutKind};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const CLASS: &str = "MizuFlyout";

pub struct Flyout {
    pub hwnd: HWND,
    pub view: Option<View>,
    pub key: ViewKey,
    pub ready: bool,
    pub kind: Option<FlyoutKind>,
    /// Screen-space anchor rectangle (physical pixels).
    anchor: RECT,
    activate: bool,
    pub tab_id: Option<u64>,
    /// Permission prompt currently shown, if any.
    pub prompt_id: Option<u64>,
    visible: bool,
}

impl Flyout {
    pub fn new(owner: HWND, window: WindowId) -> Self {
        // SAFETY: creating an owned popup window of a registered class.
        let hwnd = unsafe {
            let instance = GetModuleHandleW(None).unwrap_or_default();
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                &wide(CLASS),
                &wide("Mizu"),
                WS_POPUP | WS_CLIPCHILDREN,
                0,
                0,
                320,
                200,
                Some(owner),
                None,
                Some(instance.into()),
                None,
            )
            .unwrap_or_default();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window.0 as isize);
            hwnd
        };
        round_corners(hwnd);
        Self {
            hwnd,
            view: None,
            key: mizu_core::ids::TabId::next().0,
            ready: false,
            kind: None,
            anchor: RECT::default(),
            activate: true,
            tab_id: None,
            prompt_id: None,
            visible: false,
        }
    }

    /// Prepares the flyout; it becomes visible once the content reports its size.
    pub fn open(&mut self, owner: HWND, chrome_origin_y: i32, kind: FlyoutKind, anchor: Anchor, json: &str) {
        let scale = dpi_scale(owner);
        let mut origin = POINT { x: 0, y: chrome_origin_y };
        // SAFETY: converting a client point of our own window.
        unsafe {
            let _ = ClientToScreen(owner, &mut origin);
        }
        self.anchor = RECT {
            left: origin.x + (anchor.x * scale).round() as i32,
            top: origin.y + (anchor.y * scale).round() as i32,
            right: origin.x + ((anchor.x + anchor.width) * scale).round() as i32,
            bottom: origin.y + ((anchor.y + anchor.height) * scale).round() as i32,
        };
        self.kind = Some(kind);
        self.activate = kind != FlyoutKind::Omnibox;
        if let Some(v) = &self.view {
            v.post_json(json);
        }
    }

    /// Content measured itself: size and place the popup next to the anchor, inside the monitor.
    pub fn resize(&mut self, owner: HWND, width: f64, height: f64) {
        if self.kind.is_none() {
            return;
        }
        let scale = dpi_scale(owner);
        let w = (width * scale).ceil() as i32;
        let h = (height * scale).ceil() as i32;
        let a = self.anchor;
        // SAFETY: monitor queries and window positioning of our own popup.
        unsafe {
            let monitor = MonitorFromPoint(POINT { x: a.left, y: a.bottom }, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoW(monitor, &mut mi);
            let work = mi.rcWork;
            let gap = (4.0 * scale) as i32;
            let mut x = if self.kind == Some(FlyoutKind::Omnibox) || a.right - a.left < w / 2 && a.left + w <= work.right {
                a.left
            } else {
                a.right - w
            };
            if self.kind == Some(FlyoutKind::TabMenu) {
                x = a.left;
            }
            if self.kind == Some(FlyoutKind::Confirm) {
                x = a.left - w / 2;
            }
            x = x.clamp(work.left + gap, (work.right - w - gap).max(work.left));
            let mut y = a.bottom + gap;
            if y + h > work.bottom {
                y = (a.top - h - gap).max(work.top);
            }
            let flags = if self.activate { SWP_SHOWWINDOW } else { SWP_SHOWWINDOW | SWP_NOACTIVATE };
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, w, h, flags);
            if self.activate {
                let _ = SetForegroundWindow(self.hwnd);
            }
        }
        if let Some(v) = &self.view {
            v.set_bounds(RECT { left: 0, top: 0, right: w, bottom: h });
            v.set_visible(true);
            if self.activate {
                v.focus();
            }
        }
        self.visible = true;
    }

    pub fn hide(&mut self) {
        if !self.visible && self.kind.is_none() {
            return;
        }
        self.kind = None;
        self.tab_id = None;
        self.visible = false;
        // SAFETY: hiding our own popup.
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        if let Some(v) = &self.view {
            v.post_json(r#"{"type":"flyout","kind":"menu","data":null}"#);
        }
    }

    pub fn is_open(&self, kind: FlyoutKind) -> bool {
        self.kind == Some(kind)
    }

    pub fn destroy(&mut self) {
        if let Some(v) = self.view.take() {
            v.close();
        }
        crate::window::destroy_native(self.hwnd);
    }
}

pub fn register_class() {
    // SAFETY: class registration with a static window procedure.
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let name = wide(CLASS);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: windows_core::PCWSTR(name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&class);
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: window procedure called by the system with valid arguments.
    unsafe {
        if msg == WM_ACTIVATE && (wparam.0 & 0xffff) == WA_INACTIVE as usize {
            let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if raw != 0 {
                let id = WindowId(raw as u64);
                crate::queue::with_app(move |a| a.flyout_deactivated(id));
            }
        }
        if msg == WM_MOUSEACTIVATE {
            // Omnibox suggestions must not steal focus from the address bar.
            let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if raw != 0 {
                let id = WindowId(raw as u64);
                let no_activate = crate::app::with(|a| {
                    a.window(id).is_some_and(|w| w.flyout.is_open(FlyoutKind::Omnibox))
                })
                .unwrap_or(false);
                if no_activate {
                    return LRESULT(MA_NOACTIVATE as isize);
                }
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
