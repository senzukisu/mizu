//! One browser process per profile. A second launch forwards its URLs to the running instance
//! (the engine also requires exclusive access to the profile folder).

use crate::LaunchArgs;
use crate::win::wide;
use mizu_core::paths::ProfilePaths;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, LPARAM, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, ASFW_ANY, FindWindowExW, HWND_MESSAGE, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_COPYDATA,
};

pub const MESSAGE_CLASS: &str = "MizuMessageWindow";
/// Identifies our WM_COPYDATA payloads.
pub const COPYDATA_LAUNCH: usize = 0x4D495A55; // "MIZU"

pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // SAFETY: closing the mutex handle we created.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn mutex_name(paths: &ProfilePaths) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in paths.root.to_string_lossy().to_lowercase().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("Local\\OnibiLab.Mizu.{h:016x}")
}

/// Returns a guard if this is the first instance; otherwise forwards `args` and returns `None`.
pub fn acquire(paths: &ProfilePaths, args: &LaunchArgs) -> Option<InstanceGuard> {
    // SAFETY: creating a named mutex; GetLastError is read immediately after.
    let (handle, existed) = unsafe {
        let h = CreateMutexW(None, true, &wide(&mutex_name(paths))).ok()?;
        (h, GetLastError() == ERROR_ALREADY_EXISTS)
    };
    if !existed {
        return Some(InstanceGuard(handle));
    }
    // SAFETY: closing our handle to the existing mutex.
    unsafe {
        let _ = CloseHandle(handle);
    }
    forward(paths, args);
    None
}

fn forward(paths: &ProfilePaths, args: &LaunchArgs) {
    let payload = serde_json::json!({
        "profile_root": paths.root.to_string_lossy(),
        "urls": args.urls,
        "private": args.private,
    })
    .to_string();
    // SAFETY: FindWindow/SendMessageTimeout with a COPYDATASTRUCT that outlives the call.
    unsafe {
        let Ok(hwnd) = FindWindowExW(Some(HWND_MESSAGE), None, &wide(MESSAGE_CLASS), &wide(&mutex_name(paths))) else {
            return;
        };
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let cds = COPYDATASTRUCT {
            dwData: COPYDATA_LAUNCH,
            cbData: payload.len() as u32,
            lpData: payload.as_ptr() as *mut _,
        };
        let _ = SendMessageTimeoutW(
            hwnd,
            WM_COPYDATA,
            WPARAM(0),
            LPARAM(&raw const cds as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }
}

/// Window title for the message window, so a second instance can find the right profile.
pub fn message_window_title(paths: &ProfilePaths) -> String {
    mutex_name(paths)
}

/// Decodes a forwarded launch.
pub fn decode(data: &[u8]) -> Option<LaunchArgs> {
    let v: serde_json::Value = serde_json::from_slice(data).ok()?;
    Some(LaunchArgs {
        urls: v["urls"]
            .as_array()?
            .iter()
            .filter_map(|u| u.as_str().map(str::to_owned))
            .take(50)
            .collect(),
        private: v["private"].as_bool().unwrap_or(false),
        profile: String::new(),
    })
}
