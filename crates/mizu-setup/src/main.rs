//! MizuSetup – installs Mizu for the current user (no administrator rights needed).
//!
//! ```text
//! MizuSetup.exe                                   interactive install (choose the folder)
//! MizuSetup.exe /S [--wait-pid N] [--relaunch]    silent install/update (used by Mizu's updater)
//! uninstall.exe  |  MizuSetup.exe --uninstall [/S]   remove Mizu
//! ```
//!
//! The interactive installer and uninstaller are a small WebView2 window in Mizu's style
//! (`ui.rs`); if WebView2 cannot start, plain Windows dialogs are used instead. Browsing data
//! lives in `%LOCALAPPDATA%\Onibi Lab\Mizu` and is only removed on uninstall if the user asks.

#![windows_subsystem = "windows"]

mod setup;
mod ui;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Controls::*;
use windows_core::{HSTRING, PCWSTR};

fn main() {
    // SAFETY: COM for shell links, dialogs and WebView2 on this (UI) thread.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |f: &str| args.iter().any(|a| a.eq_ignore_ascii_case(f));
    let silent = has("/S") || has("--silent");

    if let Some(pid) = args.iter().position(|a| a == "--wait-pid").and_then(|i| args.get(i + 1)).and_then(|p| p.parse().ok()) {
        setup::wait_for_pid(pid);
    }

    // Started as uninstall.exe (from Settings, or double-clicked) means uninstall.
    let named_uninstall = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().eq_ignore_ascii_case("uninstall")))
        .unwrap_or(false);

    if has("--uninstall") || named_uninstall {
        if silent {
            let _ = setup::uninstall(&setup::uninstall_dir(), false);
        } else {
            if ui::run(ui::Mode::Uninstall).is_err() {
                fallback_uninstall();
            }
            setup::cleanup_later(&ui::webview_data_dir());
        }
        return;
    }

    if silent {
        let dir = setup::installed().map_or_else(setup::default_install_dir, |(_, d)| d);
        let keep_desktop = setup::desktop_link().exists();
        match setup::install(&dir, keep_desktop, &|_, _| {}) {
            Ok(exe) if has("--relaunch") => setup::launch(&exe),
            Ok(_) => {}
            Err(e) => {
                let _ = std::fs::write(std::env::temp_dir().join("MizuSetup-error.txt"), e);
                std::process::exit(1);
            }
        }
        return;
    }

    if ui::run(ui::Mode::Install).is_err() {
        fallback_install();
    }
    setup::cleanup_later(&ui::webview_data_dir());
}

// ───────────────────────────── Fallback dialogs (no WebView2) ─────────────────────────────

struct Dialog<'a> {
    heading: &'a str,
    body: String,
    buttons: &'a [(i32, &'a str)],
    verification: Option<(&'a str, bool)>,
    error: bool,
}

/// Shows a task dialog; returns (button id, verification checked).
fn dialog(d: Dialog) -> (i32, bool) {
    let t = setup::text();
    let title = HSTRING::from(t.title);
    let heading = HSTRING::from(d.heading);
    let body = HSTRING::from(d.body.as_str());
    let labels: Vec<HSTRING> = d.buttons.iter().map(|(_, l)| HSTRING::from(*l)).collect();
    let buttons: Vec<TASKDIALOG_BUTTON> = d
        .buttons
        .iter()
        .zip(&labels)
        .map(|((id, _), l)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: PCWSTR(l.as_ptr()) })
        .collect();
    let verify = d.verification.map(|(v, _)| HSTRING::from(v));
    let mut flags = TDF_ALLOW_DIALOG_CANCELLATION | TDF_USE_COMMAND_LINKS | TDF_POSITION_RELATIVE_TO_WINDOW;
    if d.verification.is_some_and(|(_, on)| on) {
        flags |= TDF_VERIFICATION_FLAG_CHECKED;
    }
    let mut config = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: HWND::default(),
        dwFlags: flags,
        pszWindowTitle: PCWSTR(title.as_ptr()),
        pszMainInstruction: PCWSTR(heading.as_ptr()),
        pszContent: PCWSTR(body.as_ptr()),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: d.buttons.first().map_or(0, |b| b.0),
        pszVerificationText: verify.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr())),
        ..Default::default()
    };
    config.Anonymous1.pszMainIcon = if d.error { TD_ERROR_ICON } else { PCWSTR(1 as _) };
    // SAFETY: hInstance for the custom icon resource id 1.
    config.hInstance = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or_default().into() };
    let mut button = 0i32;
    let mut checked = windows_core::BOOL::default();
    // SAFETY: config and all strings outlive the modal call.
    let _ = unsafe { TaskDialogIndirect(&config, Some(&mut button), None, Some(&mut checked)) };
    (button, checked.as_bool())
}

const ID_INSTALL: i32 = 100;
const ID_LAUNCH: i32 = 101;
const ID_DEFAULT: i32 = 102;
const ID_UNINSTALL: i32 = 103;
const ID_CLOSE: i32 = 2; // IDCANCEL

fn fallback_uninstall() {
    let t = setup::text();
    let (button, remove_data) = dialog(Dialog {
        heading: t.uninstall_heading,
        body: t.uninstall_body.into(),
        buttons: &[(ID_UNINSTALL, t.uninstall_button), (ID_CLOSE, t.close)],
        verification: Some((t.remove_data, false)),
        error: false,
    });
    if button != ID_UNINSTALL {
        return;
    }
    match setup::uninstall(&setup::uninstall_dir(), remove_data) {
        Ok(()) => {
            dialog(Dialog { heading: t.uninstalled, body: String::new(), buttons: &[(ID_CLOSE, t.close)], verification: None, error: false });
        }
        Err(e) => {
            dialog(Dialog { heading: t.failed, body: e, buttons: &[(ID_CLOSE, t.close)], verification: None, error: true });
        }
    }
}

fn fallback_install() {
    let t = setup::text();
    let dir = setup::installed().map_or_else(setup::default_install_dir, |(_, d)| d);
    let body = t.install_body.replace("%PATH%", &dir.to_string_lossy());
    let (button, desktop) = dialog(Dialog {
        heading: &format!("{} {}", t.install_heading, setup::VERSION),
        body,
        buttons: &[(ID_INSTALL, t.install_button), (ID_CLOSE, t.close)],
        verification: Some((t.desktop, true)),
        error: false,
    });
    if button != ID_INSTALL {
        return;
    }
    match setup::install(&dir, desktop, &|_, _| {}) {
        Ok(exe) => loop {
            let (b, _) = dialog(Dialog {
                heading: t.done_heading,
                body: t.done_body.into(),
                buttons: &[(ID_LAUNCH, t.launch), (ID_DEFAULT, t.default_browser), (ID_CLOSE, t.close)],
                verification: None,
                error: false,
            });
            match b {
                ID_LAUNCH => {
                    setup::launch(&exe);
                    break;
                }
                ID_DEFAULT => setup::open_default_apps(),
                _ => break,
            }
        },
        Err(e) => {
            dialog(Dialog { heading: t.failed, body: e, buttons: &[(ID_CLOSE, t.close)], verification: None, error: true });
        }
    }
}
