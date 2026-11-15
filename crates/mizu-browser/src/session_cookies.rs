//! Session cookies survive a restart when Mizu restores the session.
//!
//! Chromium drops cookies without an expiry date ("session cookies") when the browser
//! closes. Many sites keep sign-ins and site settings in them, so with "Continue where I
//! left off" Mizu does what Chrome does: it saves them when the last window closes
//! (encrypted with DPAPI, they are often sign-in tokens) and puts them back before the
//! first page of the next session loads. The file is deleted once it has been read.

use crate::app::App;
use mizu_engine::{CookieData, View};
use mizu_settings::StartupBehavior;
use std::path::PathBuf;

fn file(app: &App) -> PathBuf {
    app.paths.database().with_file_name("session-cookies.bin")
}

fn restore_enabled(app: &App) -> bool {
    app.settings.general.startup == StartupBehavior::RestoreSession
}

impl App {
    /// Reads cookies saved by the previous session (startup).
    pub fn load_session_cookies(&mut self) {
        let path = file(self);
        let saved = std::fs::read(&path).ok();
        let _ = std::fs::remove_file(&path);
        if !restore_enabled(self) {
            return;
        }
        let Some(bytes) = saved else { return };
        let cookies: Option<Vec<CookieData>> = crate::vault::decrypt(&bytes).ok().and_then(|json| serde_json::from_str(&json).ok());
        if let Some(c) = cookies.filter(|c| !c.is_empty()) {
            tracing::info!(count = c.len(), "session cookies to restore");
            self.pending_session_cookies = Some(c);
        }
    }

    /// Gives the saved cookies to the profile before its first page loads.
    pub fn apply_session_cookies(&mut self, view: &View, private: bool) {
        if private {
            return;
        }
        if let Some(cookies) = self.pending_session_cookies.take() {
            let added = view.add_cookies(&cookies);
            tracing::info!(added, "session cookies restored");
        }
    }

    /// Before the last normal window closes: collect the session cookies, then call `then`.
    /// Returns false when nothing needs saving (the caller closes right away).
    pub fn save_session_cookies_then(&mut self, view: Option<View>, then: impl FnOnce(&mut App) + 'static) -> bool {
        if !restore_enabled(self) || self.saving_session_cookies {
            return false;
        }
        let Some(view) = view else { return false };
        self.saving_session_cookies = true;
        let path = file(self);
        view.all_cookies(move |cookies| {
            let session: Vec<CookieData> = cookies.into_iter().filter(|c| c.session).collect();
            if session.is_empty() {
                let _ = std::fs::remove_file(&path);
            } else if let Ok(json) = serde_json::to_string(&session) {
                match crate::vault::encrypt(&json) {
                    Ok(bytes) => {
                        let _ = std::fs::write(&path, bytes);
                        tracing::info!(count = session.len(), "session cookies saved");
                    }
                    Err(e) => tracing::warn!(error = %e, "could not save session cookies"),
                }
            }
            crate::queue::with_app(then);
        });
        true
    }
}
