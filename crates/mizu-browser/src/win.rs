//! Thin Win32 helpers: DPI, DWM (Mica, dark title, rounded corners), shell, message boxes.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmExtendFrameIntoClientArea,
    DwmSetWindowAttribute,
};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, MB_ICONERROR, MB_OK, MessageBoxW, SW_SHOWNORMAL,
};
use windows_core::{HSTRING, PCWSTR};

pub fn wide(s: &str) -> HSTRING {
    HSTRING::from(s)
}

pub fn dpi_scale(hwnd: HWND) -> f64 {
    // SAFETY: valid window handle or 0 (returns system DPI).
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { f64::from(dpi) / 96.0 }
}

pub fn client_rect(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    // SAFETY: rc is a valid out pointer.
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    rc
}

/// Applies dark/light caption colours and the Mica backdrop (Windows 11; ignored elsewhere).
pub fn apply_backdrop(hwnd: HWND, dark: bool, mica: bool) {
    // SAFETY: DWM attribute calls with correctly sized values.
    unsafe {
        let dark_flag: i32 = i32::from(dark);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark_flag).cast(),
            size_of::<i32>() as u32,
        );
        let backdrop: DWM_SYSTEMBACKDROP_TYPE = if mica { DWMSBT_MAINWINDOW } else { DWMSBT_NONE };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&raw const backdrop).cast(),
            size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        );
        let margins = if mica {
            MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 }
        } else {
            MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 }
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        // With the frame extended over the whole window, Windows draws its own title, icon
        // and minimise/maximise/close buttons into the glass, where they show through Mizu's
        // own buttons. Mizu draws all of these itself, so tell the theme not to.
        use windows::Win32::UI::Controls::{SetWindowThemeAttribute, WTA_NONCLIENT, WTA_OPTIONS, WTNCA_NODRAWCAPTION, WTNCA_NODRAWICON, WTNCA_NOSYSMENU};
        let flags = WTNCA_NODRAWCAPTION | WTNCA_NODRAWICON | WTNCA_NOSYSMENU;
        let opts = WTA_OPTIONS { dwFlags: flags, dwMask: flags };
        let _ = SetWindowThemeAttribute(hwnd, WTA_NONCLIENT, (&raw const opts).cast(), size_of::<WTA_OPTIONS>() as u32);
    }
}

pub fn round_corners(hwnd: HWND) {
    let pref: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
    // SAFETY: DWM attribute call with a correctly sized value.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const pref).cast(),
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}

/// Opens a file with its associated application.
pub fn shell_open(path: &str) {
    // SAFETY: ShellExecuteW with valid strings.
    unsafe {
        ShellExecuteW(None, &wide("open"), &wide(path), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

/// Opens Explorer with the file selected.
pub fn show_in_folder(path: &str) {
    let args = format!("/select,\"{}\"", path.replace('"', ""));
    // SAFETY: ShellExecuteW with valid strings.
    unsafe {
        ShellExecuteW(None, &wide("open"), &wide("explorer.exe"), &wide(&args), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

pub fn fatal(message: &str) {
    tracing::error!(message, "fatal error");
    // SAFETY: modal message box with valid strings.
    unsafe {
        MessageBoxW(None, &wide(message), &wide("Mizu"), MB_OK | MB_ICONERROR);
    }
}

/// The Windows UI language as a BCP-47 tag, e.g. "ru-RU".
pub fn system_language() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    // SAFETY: buffer of LOCALE_NAME_MAX_LENGTH characters.
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if n <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..(n as usize - 1)]).to_ascii_lowercase()
}

/// Whether Windows apps are set to dark mode.
pub fn system_prefers_dark() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let mut value: u32 = 1;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: value/size are valid out pointers sized for a DWORD.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            &wide("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&mut size),
        )
    };
    r.is_ok() && value == 0
}

/// Restores and activates a browser window (switching to a tab in another window).
pub fn bring_to_front(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};
    // SAFETY: plain window calls on a window this process owns.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Font families installed on this PC (for the interface font setting), sorted, without
/// vertical "@" variants.
pub fn font_families() -> Vec<String> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::Graphics::Gdi::{DEFAULT_CHARSET, EnumFontFamiliesExW, GetDC, LOGFONTW, ReleaseDC, TEXTMETRICW};

    unsafe extern "system" fn collect(lf: *const LOGFONTW, _: *const TEXTMETRICW, _: u32, lparam: LPARAM) -> i32 {
        // SAFETY: lparam is the Vec passed below; lf is valid for the duration of the call.
        unsafe {
            let out = &mut *(lparam.0 as *mut Vec<String>);
            let face = &(*lf).lfFaceName;
            let len = face.iter().position(|&c| c == 0).unwrap_or(face.len());
            let name = String::from_utf16_lossy(&face[..len]);
            if !name.is_empty() && !name.starts_with('@') {
                out.push(name);
            }
        }
        1
    }

    let mut out: Vec<String> = Vec::new();
    // SAFETY: screen DC released below; the callback only touches `out`.
    unsafe {
        let dc = GetDC(None);
        let lf = LOGFONTW { lfCharSet: DEFAULT_CHARSET, ..Default::default() };
        EnumFontFamiliesExW(dc, &lf, Some(collect), LPARAM(&raw mut out as isize), 0);
        ReleaseDC(None, dc);
    }
    out.sort_by_key(|s| s.to_lowercase());
    out.dedup();
    out
}
