//! Small COM helpers.

use windows::Win32::System::Com::CoTaskMemFree;
use windows_core::{BOOL, PWSTR};

/// Takes ownership of a CoTaskMem-allocated string returned by WebView2.
pub fn take_pwstr(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: WebView2 returns a valid, NUL-terminated, CoTaskMemAlloc'd UTF-16 string that the
    // caller owns; we read it once and free it exactly once.
    unsafe {
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    }
}

/// Calls a WebView2 getter of the form `fn(&self, *mut PWSTR) -> Result<()>`.
pub fn get_string(f: impl FnOnce(*mut PWSTR) -> windows_core::Result<()>) -> String {
    let mut p = PWSTR::null();
    match f(&mut p) {
        Ok(()) => take_pwstr(p),
        Err(_) => String::new(),
    }
}

pub fn get_bool(f: impl FnOnce(*mut BOOL) -> windows_core::Result<()>) -> bool {
    let mut b = BOOL::default();
    f(&mut b).map(|_| b.as_bool()).unwrap_or(false)
}

/// Reads an entire COM stream (favicons, small responses).
pub fn read_stream(stream: &windows::Win32::System::Com::IStream, limit: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        let mut read = 0u32;
        // SAFETY: buf is a valid writable buffer of the given length.
        let hr = unsafe { stream.Read(buf.as_mut_ptr().cast(), buf.len() as u32, Some(&mut read)) };
        if hr.is_err() || read == 0 {
            break;
        }
        out.extend_from_slice(&buf[..read as usize]);
        if out.len() > limit {
            break;
        }
    }
    out
}
