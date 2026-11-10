//! Mizu – a new way to browse. Onibi Lab.
//!
//! Process entry point: logging, DPI awareness, COM, single-instance handling, then the Win32
//! message loop. All browser logic runs on this one UI thread (WebView2 requires it); heavy
//! work such as filter-list downloads runs on worker threads that post results back.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod dialogs;
mod downloads;
mod extensions;
mod flyout;
mod handler;
mod hot;
mod internal;
mod omnibox;
mod queue;
mod scripts;
mod session;
mod shield;
mod shortcuts;
mod single_instance;
mod tabs;
mod theme;
mod win;
mod window;

use mizu_core::paths::{ProfilePaths, logs_dir};
use windows::Win32::System::Ole::OleInitialize;
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage};

/// Parsed command line.
#[derive(Debug, Default, Clone)]
pub struct LaunchArgs {
    pub urls: Vec<String>,
    pub private: bool,
    pub profile: String,
}

impl LaunchArgs {
    fn parse(args: impl Iterator<Item = String>) -> Self {
        let mut out = LaunchArgs {
            profile: "Default".into(),
            ..Default::default()
        };
        let mut it = args.skip(1);
        while let Some(a) = it.next() {
            match a.as_str() {
                "--private" | "-private" | "--incognito" => out.private = true,
                "--profile" => {
                    if let Some(p) = it.next() {
                        out.profile = p;
                    }
                }
                s if s.starts_with("--") => {} // unknown switches are ignored, never forwarded
                s => out.urls.push(s.to_owned()),
            }
        }
        out
    }
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dir = logs_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("mizu")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = tracing_subscriber::EnvFilter::try_from_env("MIZU_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    Some(guard)
}

fn main() {
    let _log_guard = init_logging();
    std::panic::set_hook(Box::new(|info| {
        tracing::error!(%info, "panic");
    }));
    tracing::info!(version = mizu_core::VERSION, "starting Mizu");

    // SAFETY: process-wide initialisation before any window exists.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if let Err(e) = OleInitialize(None) {
            win::fatal(&format!("Could not initialise COM: {e}"));
            return;
        }
    }

    let args = LaunchArgs::parse(std::env::args());
    let paths = ProfilePaths::for_profile(&args.profile);

    let Some(_instance) = single_instance::acquire(&paths, &args) else {
        tracing::info!("forwarded launch to the running instance");
        return;
    };

    if let Err(e) = app::start(paths, args) {
        win::fatal(&e);
        return;
    }

    let mut msg = MSG::default();
    // SAFETY: standard Win32 message loop on the UI thread.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    app::shutdown();
    tracing::info!("Mizu exited cleanly");
}

#[cfg(test)]
mod tests {
    use super::LaunchArgs;

    #[test]
    fn parses_args() {
        let a = LaunchArgs::parse(
            ["mizu", "--private", "https://a/", "--profile", "Work", "--evil=1"]
                .into_iter()
                .map(String::from),
        );
        assert!(a.private);
        assert_eq!(a.urls, ["https://a/"]);
        assert_eq!(a.profile, "Work");
    }
}
