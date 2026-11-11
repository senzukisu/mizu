//! Visual hosting: web views draw into DirectComposition visuals inside our own windows.
//!
//! With classic (HWND) hosting every view owns a child window that belongs to the engine's
//! browser process. Windows ties the input queues of cross-process parent/child windows
//! together, so whenever the engine process was busy (many tabs loading) Mizu's UI thread
//! stalled for seconds inside `SetWindowPos`/`ShowWindow`. With composition controllers
//! there are no foreign windows in our tree: each view is a visual we position, and we
//! forward mouse, wheel, cursor and drag-and-drop input to the view under the pointer.
//!
//! One [`Host`] exists per top-level window that contains views (browser windows, flyouts).
//! Everything runs on the UI thread.

use std::cell::RefCell;
use std::collections::HashMap;

use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_core::Interface;

use crate::events::ViewKey;

const WM_MOUSELEAVE: u32 = 0x02A3;

struct Entry {
    key: ViewKey,
    comp: ICoreWebView2CompositionController,
    visual: IDCompositionVisual,
    bounds: RECT,
    visible: bool,
    /// Draws above other views (browser UI over content where they overlap).
    on_top: bool,
}

struct Host {
    device: IDCompositionDevice,
    _target: IDCompositionTarget,
    root: IDCompositionVisual,
    entries: Vec<Entry>,
    hover: Option<ViewKey>,
    capture: Option<ViewKey>,
    tracking: bool,
    drop_target: Option<ViewKey>,
}

thread_local! {
    static HOSTS: RefCell<HashMap<isize, Host>> = RefCell::new(HashMap::new());
}

fn hkey(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn ensure_host(hwnd: HWND) -> windows_core::Result<()> {
    let exists = HOSTS.with(|h| h.borrow().contains_key(&hkey(hwnd)));
    if exists {
        return Ok(());
    }
    // SAFETY: DirectComposition objects created and used on the UI thread.
    let host = unsafe {
        let device: IDCompositionDevice = DCompositionCreateDevice2(None)?;
        let target = device.CreateTargetForHwnd(hwnd, true)?;
        let root = device.CreateVisual()?;
        target.SetRoot(&root)?;
        device.Commit()?;
        Host { device, _target: target, root, entries: Vec::new(), hover: None, capture: None, tracking: false, drop_target: None }
    };
    HOSTS.with(|h| h.borrow_mut().insert(hkey(hwnd), host));
    crate::drop_target::register(hwnd);
    Ok(())
}

/// Adds a view's visual to `hwnd`'s composition tree.
pub(crate) fn register(hwnd: HWND, key: ViewKey, comp: &ICoreWebView2CompositionController, on_top: bool) -> windows_core::Result<()> {
    ensure_host(hwnd)?;
    HOSTS.with(|h| -> windows_core::Result<()> {
        let mut hosts = h.borrow_mut();
        let host = hosts.get_mut(&hkey(hwnd)).expect("host just created");
        // SAFETY: DirectComposition and WebView2 calls on the UI thread.
        unsafe {
            let visual = host.device.CreateVisual()?;
            if on_top {
                host.root.AddVisual(&visual, true, None)?;
            } else {
                // Content goes underneath any browser UI visuals.
                host.root.AddVisual(&visual, false, None)?;
            }
            comp.SetRootVisualTarget(&visual)?;
            host.device.Commit()?;
            host.entries.push(Entry { key, comp: comp.clone(), visual, bounds: RECT::default(), visible: true, on_top });
        }
        Ok(())
    })
}

pub(crate) fn unregister(hwnd: HWND, key: ViewKey) {
    HOSTS.with(|h| {
        let mut hosts = h.borrow_mut();
        let Some(host) = hosts.get_mut(&hkey(hwnd)) else { return };
        if let Some(i) = host.entries.iter().position(|e| e.key == key) {
            let e = host.entries.remove(i);
            // SAFETY: DirectComposition calls on the UI thread.
            unsafe {
                let _ = e.comp.SetRootVisualTarget(None::<&windows_core::IUnknown>);
                let _ = host.root.RemoveVisual(&e.visual);
                let _ = host.device.Commit();
            }
        }
        for slot in [&mut host.hover, &mut host.capture, &mut host.drop_target] {
            if *slot == Some(key) {
                *slot = None;
            }
        }
    });
}

/// Forgets a window's composition tree (the window is being destroyed).
pub fn release_window(hwnd: HWND) {
    crate::drop_target::revoke(hwnd);
    HOSTS.with(|h| h.borrow_mut().remove(&hkey(hwnd)));
}

pub(crate) fn set_bounds(hwnd: HWND, key: ViewKey, rect: RECT) {
    HOSTS.with(|h| {
        let mut hosts = h.borrow_mut();
        let Some(host) = hosts.get_mut(&hkey(hwnd)) else { return };
        let Some(e) = host.entries.iter_mut().find(|e| e.key == key) else { return };
        if e.bounds == rect {
            return;
        }
        e.bounds = rect;
        // SAFETY: DirectComposition calls on the UI thread.
        unsafe {
            let _ = e.visual.SetOffsetX2(rect.left as f32);
            let _ = e.visual.SetOffsetY2(rect.top as f32);
            let _ = host.device.Commit();
        }
    });
}

pub(crate) fn set_visible(hwnd: HWND, key: ViewKey, visible: bool) {
    HOSTS.with(|h| {
        let mut hosts = h.borrow_mut();
        let Some(host) = hosts.get_mut(&hkey(hwnd)) else { return };
        if let Some(e) = host.entries.iter_mut().find(|e| e.key == key) {
            e.visible = visible;
        }
        if !visible {
            for slot in [&mut host.hover, &mut host.capture] {
                if *slot == Some(key) {
                    *slot = None;
                }
            }
        }
    });
}

/// The view under a client-area point, its composition controller and the point relative to it.
fn view_at(hwnd: HWND, pt: POINT) -> Option<(ViewKey, ICoreWebView2CompositionController, POINT)> {
    HOSTS.with(|h| {
        let hosts = h.borrow();
        let host = hosts.get(&hkey(hwnd))?;
        let inside = |e: &&Entry| e.visible && pt.x >= e.bounds.left && pt.x < e.bounds.right && pt.y >= e.bounds.top && pt.y < e.bounds.bottom;
        let e = host.entries.iter().filter(|e| e.on_top).find(inside).or_else(|| host.entries.iter().filter(|e| !e.on_top).find(inside))?;
        Some((e.key, e.comp.clone(), POINT { x: pt.x - e.bounds.left, y: pt.y - e.bounds.top }))
    })
}

fn view_by_key(hwnd: HWND, key: ViewKey, pt: POINT) -> Option<(ICoreWebView2CompositionController, POINT)> {
    HOSTS.with(|h| {
        let hosts = h.borrow();
        let e = hosts.get(&hkey(hwnd))?.entries.iter().find(|e| e.key == key)?;
        Some((e.comp.clone(), POINT { x: pt.x - e.bounds.left, y: pt.y - e.bounds.top }))
    })
}

fn send(comp: &ICoreWebView2CompositionController, kind: u32, keys: u32, data: u32, pt: POINT) {
    // SAFETY: COM call on the UI thread.
    unsafe {
        let _ = comp.SendMouseInput(
            COREWEBVIEW2_MOUSE_EVENT_KIND(kind as i32),
            COREWEBVIEW2_MOUSE_EVENT_VIRTUAL_KEYS(keys as i32),
            data,
            pt,
        );
    }
}

fn leave_hover(hwnd: HWND) {
    let prev = HOSTS.with(|h| h.borrow_mut().get_mut(&hkey(hwnd)).and_then(|host| host.hover.take()));
    if let Some((comp, _)) = prev.and_then(|k| view_by_key(hwnd, k, POINT::default())) {
        send(&comp, WM_MOUSELEAVE, 0, 0, POINT::default());
    }
}

const BUTTONS: u32 = 0x0001 | 0x0002 | 0x0010 | 0x0020 | 0x0040; // MK_LBUTTON | MK_RBUTTON | MK_MBUTTON | MK_XBUTTON1 | MK_XBUTTON2

/// Routes mouse input of `hwnd` to the view under the pointer. Returns `Some` when handled.
pub fn handle_input(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK
        | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP | WM_XBUTTONDBLCLK | WM_MOUSEWHEEL
        | WM_MOUSEHWHEEL => {}
        WM_MOUSELEAVE => {
            HOSTS.with(|h| {
                if let Some(host) = h.borrow_mut().get_mut(&hkey(hwnd)) {
                    host.tracking = false;
                }
            });
            leave_hover(hwnd);
            return Some(LRESULT(0));
        }
        WM_NCMOUSEMOVE => {
            // Pointer moved onto a draggable caption area: the page must not keep hover state.
            leave_hover(hwnd);
            return None;
        }
        WM_CAPTURECHANGED => {
            HOSTS.with(|h| {
                if let Some(host) = h.borrow_mut().get_mut(&hkey(hwnd)) {
                    host.capture = None;
                }
            });
            return None;
        }
        WM_SETCURSOR => {
            if (lparam.0 & 0xffff) as u32 != HTCLIENT {
                return None;
            }
            let hover = HOSTS.with(|h| h.borrow().get(&hkey(hwnd)).and_then(|host| host.capture.or(host.hover)))?;
            let (comp, _) = view_by_key(hwnd, hover, POINT::default())?;
            let mut cursor = HCURSOR::default();
            // SAFETY: COM getter and SetCursor on the UI thread.
            unsafe {
                if comp.Cursor(&mut cursor).is_ok() && !cursor.is_invalid() {
                    SetCursor(Some(cursor));
                    return Some(LRESULT(1));
                }
            }
            return None;
        }
        _ => return None,
    }

    let mut pt = POINT { x: (lparam.0 & 0xffff) as i16 as i32, y: ((lparam.0 >> 16) & 0xffff) as i16 as i32 };
    if msg == WM_MOUSEWHEEL || msg == WM_MOUSEHWHEEL {
        // Wheel positions are in screen coordinates.
        // SAFETY: valid window and point.
        unsafe {
            let _ = ScreenToClient(hwnd, &mut pt);
        }
    }
    let keys = (wparam.0 & 0xffff) as u32;
    let data = match msg {
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => ((wparam.0 >> 16) & 0xffff) as i16 as i32 as u32,
        WM_XBUTTONDOWN | WM_XBUTTONUP | WM_XBUTTONDBLCLK => ((wparam.0 >> 16) & 0xffff) as u32,
        _ => 0,
    };

    let captured = HOSTS.with(|h| h.borrow().get(&hkey(hwnd)).and_then(|host| host.capture));
    let target = match captured {
        Some(k) => view_by_key(hwnd, k, pt).map(|(c, p)| (k, c, p)),
        None => view_at(hwnd, pt),
    };
    let Some((key, comp, local)) = target else {
        leave_hover(hwnd);
        return None;
    };

    // Hover bookkeeping: tell the previous view the pointer left.
    let prev = HOSTS.with(|h| {
        let mut hosts = h.borrow_mut();
        let host = hosts.get_mut(&hkey(hwnd))?;
        let prev = host.hover.replace(key).filter(|p| *p != key);
        if !host.tracking {
            host.tracking = true;
            let mut tme = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
            // SAFETY: valid structure for our own window.
            unsafe {
                let _ = TrackMouseEvent(&mut tme);
            }
        }
        prev
    });
    if let Some((old, _)) = prev.and_then(|k| view_by_key(hwnd, k, pt)) {
        send(&old, WM_MOUSELEAVE, 0, 0, POINT::default());
    }

    // Keep receiving the drag outside the view (text selection, scrollbars, tab dragging).
    match msg {
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN | WM_LBUTTONDBLCLK | WM_RBUTTONDBLCLK
        | WM_MBUTTONDBLCLK | WM_XBUTTONDBLCLK => {
            // SAFETY: capturing our own window.
            unsafe {
                SetCapture(hwnd);
            }
            HOSTS.with(|h| {
                if let Some(host) = h.borrow_mut().get_mut(&hkey(hwnd)) {
                    host.capture = Some(key);
                }
            });
        }
        _ => {}
    }

    send(&comp, msg, keys, data, local);

    if matches!(msg, WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP) && keys & BUTTONS == 0 {
        HOSTS.with(|h| {
            if let Some(host) = h.borrow_mut().get_mut(&hkey(hwnd)) {
                host.capture = None;
            }
        });
        // SAFETY: releasing our own capture.
        unsafe {
            let _ = ReleaseCapture();
        }
    }
    Some(LRESULT(0))
}

/// For `WM_NCHITTEST`: what the page under a client point declares with CSS `app-region`.
pub fn non_client_region(hwnd: HWND, client: POINT) -> Option<u32> {
    let (_, comp, local) = view_at(hwnd, client)?;
    let comp4 = comp.cast::<ICoreWebView2CompositionController4>().ok()?;
    let mut kind = COREWEBVIEW2_NON_CLIENT_REGION_KIND_NOWHERE;
    // SAFETY: COM call on the UI thread.
    unsafe {
        comp4.GetNonClientRegionAtPoint(local, &mut kind).ok()?;
    }
    (kind == COREWEBVIEW2_NON_CLIENT_REGION_KIND_CAPTION).then_some(HTCAPTION)
}

/// A cursor change in view `key`: apply it right away if the pointer is over that view.
pub(crate) fn cursor_changed(hwnd: HWND, key: ViewKey) {
    let current = HOSTS.with(|h| h.borrow().get(&hkey(hwnd)).and_then(|host| host.capture.or(host.hover)));
    if current != Some(key) {
        return;
    }
    if let Some((comp, _)) = view_by_key(hwnd, key, POINT::default()) {
        let mut cursor = HCURSOR::default();
        // SAFETY: COM getter and SetCursor on the UI thread.
        unsafe {
            if comp.Cursor(&mut cursor).is_ok() && !cursor.is_invalid() {
                SetCursor(Some(cursor));
            }
        }
    }
}

// ── Drag and drop (used by drop_target.rs) ────────────────────────────

pub(crate) fn drag_target_at(hwnd: HWND, screen: POINT) -> Option<(ICoreWebView2CompositionController3, POINT)> {
    let mut pt = screen;
    // SAFETY: valid window and point.
    unsafe {
        let _ = ScreenToClient(hwnd, &mut pt);
    }
    let (key, comp, local) = view_at(hwnd, pt)?;
    let comp3 = comp.cast::<ICoreWebView2CompositionController3>().ok()?;
    HOSTS.with(|h| {
        if let Some(host) = h.borrow_mut().get_mut(&hkey(hwnd)) {
            host.drop_target = Some(key);
        }
    });
    Some((comp3, local))
}

pub(crate) fn current_drag_target(hwnd: HWND) -> Option<ICoreWebView2CompositionController3> {
    let key = HOSTS.with(|h| h.borrow_mut().get_mut(&hkey(hwnd)).and_then(|host| host.drop_target.take()))?;
    let (comp, _) = view_by_key(hwnd, key, POINT::default())?;
    comp.cast::<ICoreWebView2CompositionController3>().ok()
}
