//! Session save/restore and userscript installation flow.

use crate::app::{App, PendingInstall};
use crate::hot;
use mizu_engine::ViewKey;
use mizu_tabs::SessionSnapshot;

impl App {
    /// Snapshot of all normal (non-private) windows. Called periodically and on exit.
    pub fn save_session(&mut self) {
        let windows: Vec<_> = self
            .windows
            .iter()
            .filter(|w| !w.private && !w.closing)
            .map(|w| w.strip.snapshot())
            .filter(|s| !s.tabs.is_empty())
            .collect();
        if windows.is_empty() {
            return;
        }
        let placement = self.windows.iter().find(|w| !w.private).map(|w| crate::window::placement(w.hwnd));
        let snapshot = SessionSnapshot { version: SessionSnapshot::VERSION, windows, placement };
        if let Err(e) = self.storage.save_session(&snapshot) {
            tracing::warn!(error = %e, "could not save session");
        }
    }

    pub fn restore_session(&mut self, snapshot: &SessionSnapshot) {
        for (i, w) in snapshot.windows.iter().enumerate() {
            if w.tabs.is_empty() {
                continue;
            }
            let placement = if i == 0 { snapshot.placement } else { None };
            self.new_window_with(false, None, placement, Some(w));
        }
        if self.windows.is_empty() {
            self.new_window(false, None);
        }
        tracing::info!(tabs = snapshot.tab_count(), "session restored");
    }

    /// Restores the previous session on demand (button on the new-tab page after a crash).
    pub fn restore_previous_session(&mut self) {
        if let Some(s) = self.previous_session.take() {
            self.crashed_last_time = false;
            self.restore_session(&s);
        }
    }

    // ───────────────────────────── Userscripts ─────────────────────────────

    pub fn begin_script_install(&mut self, key: ViewKey, url: String) {
        tracing::info!(url, "fetching userscript for installation");
        if let Some(w) = self.locate(key).and_then(|(w, _)| self.window(w)) {
            w.post_chrome(&mizu_ui::Event::Toast { message: "Downloading userscript…".into() }.to_json());
        }
        self.pending_install = None;
        self.script_install_tab = Some(key);
        crate::scripts::fetch_for_install(url, self.worker.clone());
    }

    pub fn finish_script_fetch(&mut self, url: String, result: Result<(String, String), String>) {
        let key = self.script_install_tab.take();
        match result {
            Ok((source, requires)) => match mizu_userscripts::ScriptMeta::parse(&source) {
                Ok(meta) => {
                    self.pending_install = Some(PendingInstall { url, source, requires, meta });
                    let target = "mizu://scripts/#install";
                    match key.and_then(hot::view) {
                        Some(view) => crate::tabs::load(view.key(), &view, target),
                        None => {
                            if let Some(id) = self.main_window(false) {
                                self.open_tab(id, target.into(), crate::tabs::OpenOptions::foreground());
                            }
                        }
                    }
                }
                Err(e) => self.toast_all(&format!("Not a valid userscript: {e}")),
            },
            Err(e) => self.toast_all(&format!("Could not download the userscript: {e}")),
        }
    }

    pub fn toast_all(&self, message: &str) {
        let ev = mizu_ui::Event::Toast { message: message.into() }.to_json();
        for w in &self.windows {
            w.post_chrome(&ev);
        }
    }
}
