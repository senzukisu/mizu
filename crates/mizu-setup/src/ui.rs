//! The setup window: a small frameless window hosting a WebView2 page (`ui.html`) in Mizu's
//! style. The page sends commands (`install`, `browse`, `uninstall`, …) with
//! `chrome.webview.postMessage`; long work runs on a worker thread and reports back through
//! `WM_APP` messages, which are forwarded to the page as JSON.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc;

use serde_json::{Value, json};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    CoreWebView2EnvironmentOptions, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, WebMessageReceivedEventHandler,
};
use windows::Win32::Foundation::{E_POINTER, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName,
    SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_core::{HSTRING, Interface, PCWSTR, PWSTR};

use crate::setup;

const HTML: &str = include_str!("ui.html");
const WM_TO_PAGE: u32 = WM_APP + 1;
const WM_BROWSE: u32 = WM_APP + 2;
const WM_CLOSE_SOON: u32 = WM_APP + 3;

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Install,
    Uninstall,
}

struct State {
    hwnd: HWND,
    mode: Mode,
    controller: Option<ICoreWebView2Controller>,
    core: Option<ICoreWebView2>,
    busy: bool,
    browse_from: String,
    installed_exe: Option<PathBuf>,
    shown: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

fn page_dark() -> bool {
    !setup::reg_dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme").is_some_and(|v| v != 0)
}

pub fn webview_data_dir() -> PathBuf {
    std::env::temp_dir().join("MizuSetup.WebView2")
}

/// Sends a JSON message to the page from any thread.
#[derive(Clone, Copy)]
struct Poster(isize);
impl Poster {
    fn post(self, v: Value) {
        let raw = Box::into_raw(Box::new(v.to_string()));
        // SAFETY: the box is reclaimed by the window procedure; if posting fails we free it.
        unsafe {
            if PostMessageW(Some(HWND(self.0 as *mut _)), WM_TO_PAGE, WPARAM(0), LPARAM(raw as isize)).is_err() {
                drop(Box::from_raw(raw));
            }
        }
    }
}

fn to_page(json: &str) {
    if let Some(Some(core)) = with(|s| s.core.clone()) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = core.PostWebMessageAsJson(&HSTRING::from(json));
        }
    }
}

/// Runs the setup window. Returns `Err` when WebView2 cannot be started (the caller then
/// falls back to plain Windows dialogs).
pub fn run(mode: Mode) -> Result<(), String> {
    let dark = page_dark();
    let bg: [u8; 3] = if dark { [0x12, 0x16, 0x1d] } else { [0xf7, 0xf9, 0xfc] };
    // SAFETY: standard window class registration and creation on this thread.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class = HSTRING::from("MizuSetupWindow");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hIcon: LoadIconW(Some(instance.into()), PCWSTR(1 as _)).unwrap_or_default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: CreateSolidBrush(windows::Win32::Foundation::COLORREF(
                u32::from(bg[0]) | (u32::from(bg[1]) << 8) | (u32::from(bg[2]) << 16),
            )),
            lpszClassName: PCWSTR(class.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let title = HSTRING::from(if mode == Mode::Uninstall { setup::text().title_uninstall } else { setup::text().title });
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            &class,
            &title,
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            600,
            440,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|e| e.to_string())?;
        // Size for the monitor's DPI and centre on the work area.
        let scale = f64::from(GetDpiForWindow(hwnd).max(96)) / 96.0;
        let (w, h) = ((600.0 * scale) as i32, (440.0 * scale) as i32);
        let mut work = RECT::default();
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some((&raw mut work).cast()), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let x = work.left + ((work.right - work.left) - w) / 2;
        let y = work.top + ((work.bottom - work.top) - h) / 2;
        let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_FRAMECHANGED);

        let dark_flag = i32::from(dark);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, (&raw const dark_flag).cast(), 4);
        let corner: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, (&raw const corner).cast(), 4);
        // A 1 px frame margin keeps the system shadow on a frameless window.
        let _ = DwmExtendFrameIntoClientArea(hwnd, &MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 });
        hwnd
    };

    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            hwnd,
            mode,
            controller: None,
            core: None,
            busy: false,
            browse_from: String::new(),
            installed_exe: None,
            shown: false,
        });
    });

    if let Err(e) = create_webview(hwnd, bg) {
        // SAFETY: our own window.
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        STATE.with(|s| s.borrow_mut().take());
        return Err(e);
    }
    // Shown when the page reports ready (no white flash); fallback after 1.5 s.
    // SAFETY: timer on our own window.
    unsafe {
        SetTimer(Some(hwnd), 1, 1500, None);
    }

    // SAFETY: standard message loop.
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    let exe = with(|s| s.installed_exe.take()).flatten();
    STATE.with(|s| s.borrow_mut().take());
    if let Some(exe) = exe {
        setup::launch(&exe);
    }
    Ok(())
}

fn create_webview(hwnd: HWND, bg: [u8; 3]) -> Result<(), String> {
    let data_dir = webview_data_dir();
    let options = CoreWebView2EnvironmentOptions::default();
    // SAFETY: options object used only on this thread before being passed in.
    unsafe {
        options.set_language(setup::lang().to_string());
    }
    let options: ICoreWebView2EnvironmentOptions = options.into();
    let (tx, rx) = mpsc::channel();
    let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |hr, env| {
        let _ = tx.send(hr.and_then(|_| env.ok_or_else(|| windows_core::Error::from(E_POINTER))));
        Ok(())
    }));
    // SAFETY: valid arguments; completion arrives through the message pump.
    unsafe {
        CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), &HSTRING::from(data_dir.as_os_str()), &options, &handler)
            .map_err(|e| format!("WebView2: {e}"))?;
    }
    let env = webview2_com::wait_with_pump(rx).map_err(|e| format!("WebView2: {e:?}"))?.map_err(|e| format!("WebView2: {e}"))?;

    let (tx, rx) = mpsc::channel();
    let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(move |hr, c| {
        let _ = tx.send(hr.and_then(|_| c.ok_or_else(|| windows_core::Error::from(E_POINTER))));
        Ok(())
    }));
    // SAFETY: as above.
    unsafe {
        env.CreateCoreWebView2Controller(hwnd, &handler).map_err(|e| format!("WebView2: {e}"))?;
    }
    let controller = webview2_com::wait_with_pump(rx).map_err(|e| format!("WebView2: {e:?}"))?.map_err(|e| format!("WebView2: {e}"))?;

    // SAFETY: COM calls on the UI thread with valid arguments.
    unsafe {
        if let Ok(c2) = controller.cast::<ICoreWebView2Controller2>() {
            let _ = c2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR { R: bg[0], G: bg[1], B: bg[2], A: 255 });
        }
        let core = controller.CoreWebView2().map_err(|e| e.to_string())?;
        let s = core.Settings().map_err(|e| e.to_string())?;
        let _ = s.SetAreDevToolsEnabled(cfg!(debug_assertions));
        let _ = s.SetAreDefaultContextMenusEnabled(false);
        let _ = s.SetIsStatusBarEnabled(false);
        let _ = s.SetIsZoomControlEnabled(false);
        if let Ok(s3) = s.cast::<ICoreWebView2Settings3>() {
            let _ = s3.SetAreBrowserAcceleratorKeysEnabled(false);
        }
        if let Ok(s9) = s.cast::<ICoreWebView2Settings9>() {
            let _ = s9.SetIsNonClientRegionSupportEnabled(true);
        }
        let mut token = Default::default();
        core.add_WebMessageReceived(
            &WebMessageReceivedEventHandler::create(Box::new(|_, args| {
                if let Some(args) = args {
                    let mut p = PWSTR::null();
                    if args.WebMessageAsJson(&mut p).is_ok() && !p.is_null() {
                        let s = p.to_string().unwrap_or_default();
                        CoTaskMemFree(Some(p.0 as *const _));
                        if let Ok(v) = serde_json::from_str::<Value>(&s) {
                            on_message(&v);
                        }
                    }
                }
                Ok(())
            })),
            &mut token,
        )
        .map_err(|e| e.to_string())?;
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        let _ = controller.SetBounds(rc);
        let _ = controller.SetIsVisible(true);
        core.NavigateToString(&HSTRING::from(HTML)).map_err(|e| e.to_string())?;
        with(|st| {
            st.controller = Some(controller);
            st.core = Some(core);
        });
    }
    Ok(())
}

fn reveal() {
    // Window calls happen outside the state borrow: ShowWindow sends WM_SIZE synchronously.
    let Some(Some((hwnd, controller))) = with(|s| {
        (!std::mem::replace(&mut s.shown, true)).then(|| (s.hwnd, s.controller.clone()))
    }) else {
        return;
    };
    // SAFETY: our own window.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
        if let Some(c) = controller {
            let _ = c.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
        }
    }
}

fn init_message(mode: Mode) -> Value {
    let installed = setup::installed();
    let dir = match mode {
        Mode::Uninstall => setup::uninstall_dir(),
        Mode::Install => installed.as_ref().map_or_else(setup::default_install_dir, |(_, d)| d.clone()),
    };
    json!({
        "type": "init",
        "mode": if mode == Mode::Install { "install" } else { "uninstall" },
        "lang": setup::lang(),
        "version": setup::VERSION,
        "dir": dir.to_string_lossy(),
        "installed": installed.map(|(v, d)| json!({ "version": v, "dir": d.to_string_lossy() })),
        "size_mb": (setup::payload_len() as f64 / 1_048_576.0).round().max(1.0),
        "data_dir": setup::user_data_dir().to_string_lossy(),
    })
}

fn on_message(v: &Value) {
    let cmd = v.get("cmd").and_then(Value::as_str).unwrap_or_default();
    let Some((hwnd, mode, busy)) = with(|s| (s.hwnd, s.mode, s.busy)) else { return };
    let poster = Poster(hwnd.0 as isize);
    match cmd {
        "ready" => {
            to_page(&init_message(mode).to_string());
            reveal();
        }
        "minimize" => {
            // SAFETY: our own window.
            unsafe {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
        "close" if !busy => {
            // Posted so the WebView finishes dispatching this message first.
            // SAFETY: our own window.
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE_SOON, WPARAM(0), LPARAM(0));
            }
        }
        "browse" if !busy => {
            let from = v.get("dir").and_then(Value::as_str).unwrap_or_default().to_string();
            with(|s| s.browse_from = from);
            // SAFETY: our own window.
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_BROWSE, WPARAM(0), LPARAM(0));
            }
        }
        "install" if !busy && mode == Mode::Install => {
            let dir = PathBuf::from(v.get("dir").and_then(Value::as_str).unwrap_or_default());
            let desktop = v.get("desktop").and_then(Value::as_bool).unwrap_or(true);
            with(|s| s.busy = true);
            std::thread::spawn(move || {
                let progress = |pct: u32, step: &str| poster.post(json!({ "type": "progress", "pct": pct, "step": step }));
                match setup::install(&dir, desktop, &progress) {
                    Ok(exe) => poster.post(json!({ "type": "installed", "exe": exe.to_string_lossy() })),
                    Err(e) => poster.post(json!({ "type": "error", "message": e })),
                }
            });
        }
        "uninstall" if !busy && mode == Mode::Uninstall => {
            let remove_data = v.get("remove_data").and_then(Value::as_bool).unwrap_or(false);
            with(|s| s.busy = true);
            std::thread::spawn(move || match setup::uninstall(&setup::uninstall_dir(), remove_data) {
                Ok(()) => poster.post(json!({ "type": "removed", "data_removed": remove_data })),
                Err(e) => poster.post(json!({ "type": "error", "message": e })),
            });
        }
        "open" => {
            let exe = setup::installed().map(|(_, d)| d.join("mizu.exe"));
            with(|s| s.installed_exe = exe);
        }
        "default" => setup::open_default_apps(),
        _ => {}
    }
}

fn browse(hwnd: HWND, from: &str) -> Option<PathBuf> {
    // SAFETY: standard IFileOpenDialog usage on the STA UI thread.
    unsafe {
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let opts = dlg.GetOptions().ok()?;
        let _ = dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM);
        let _ = dlg.SetTitle(&HSTRING::from(setup::text().choose_folder));
        // Start in the parent of the current choice ("…\Programs" rather than "…\Programs\Mizu").
        let start = PathBuf::from(from);
        let start = start.parent().filter(|p| p.exists()).map(PathBuf::from).unwrap_or(start);
        if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(start.as_os_str()), None) {
            let _ = dlg.SetFolder(&item);
        }
        dlg.Show(Some(hwnd)).ok()?;
        let item = dlg.GetResult().ok()?;
        let p = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as *const _));
        (!s.is_empty()).then(|| setup::normalize_install_dir(&PathBuf::from(s)))
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: standard window procedure; pointers come from our own PostMessage calls.
    unsafe {
        match msg {
            WM_NCCALCSIZE if wparam.0 != 0 => LRESULT(0),
            WM_SIZE => {
                if let Some(Some(c)) = with(|s| s.controller.clone()) {
                    let mut rc = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rc);
                    let _ = c.SetBounds(rc);
                }
                LRESULT(0)
            }
            WM_DPICHANGED => {
                let r = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                LRESULT(0)
            }
            WM_TIMER => {
                let _ = KillTimer(Some(hwnd), wparam.0);
                reveal();
                LRESULT(0)
            }
            WM_TO_PAGE => {
                let json = *Box::from_raw(lparam.0 as *mut String);
                if json.contains("\"type\":\"installed\"") || json.contains("\"type\":\"removed\"") || json.contains("\"type\":\"error\"") {
                    with(|s| s.busy = false);
                }
                to_page(&json);
                LRESULT(0)
            }
            WM_BROWSE => {
                let from = with(|s| s.browse_from.clone()).unwrap_or_default();
                if let Some(dir) = browse(hwnd, &from) {
                    to_page(&json!({ "type": "dir", "dir": dir.to_string_lossy() }).to_string());
                }
                LRESULT(0)
            }
            WM_CLOSE_SOON => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_CLOSE => {
                if with(|s| s.busy).unwrap_or(false) {
                    return LRESULT(0);
                }
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                if let Some(Some(c)) = with(|s| {
                    s.core = None;
                    s.controller.take()
                }) {
                    let _ = c.Close();
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
