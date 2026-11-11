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
        self.begin_script_install_many(Some(key), vec![url], true);
    }

    /// Fetches scripts from links. `navigate_tab`: show the install page in the tab `key`
    /// (link clicks) rather than replying to an internal page that is already open.
    pub fn begin_script_install_many(&mut self, key: Option<ViewKey>, urls: Vec<String>, navigate_tab: bool) {
        tracing::info!(count = urls.len(), "fetching userscripts for installation");
        if let Some(w) = key.and_then(|k| self.locate(k)).and_then(|(w, _)| self.window(w)) {
            w.post_chrome(&mizu_ui::Event::Toast { message: "Downloading userscript…".into() }.to_json());
        }
        self.script_install_tab = if navigate_tab { key } else { None };
        crate::scripts::fetch_for_install(key, urls, self.worker.clone());
    }

    pub fn finish_scripts_fetched(
        &mut self,
        key: Option<ViewKey>,
        items: Vec<crate::scripts::Fetched>,
        failures: Vec<(String, String)>,
    ) {
        let navigate = self.script_install_tab.take();
        self.pending_install = items
            .into_iter()
            .filter_map(|f| {
                let meta = mizu_userscripts::ScriptMeta::parse(&f.source).ok()?;
                Some(PendingInstall { url: f.url, source: f.source, requires: f.requires, meta })
            })
            .collect();
        self.pending_failures = failures;
        if self.pending_install.is_empty() && self.pending_failures.len() == 1 && navigate.is_some() {
            let (url, e) = &self.pending_failures[0];
            self.toast_all(&format!("Could not install {url}: {e}"));
            return;
        }
        let target = "mizu://scripts/#install";
        match navigate.and_then(hot::view) {
            Some(view) => crate::tabs::load(view.key(), &view, target),
            None => match key.and_then(hot::view) {
                // The scripts page asked: tell it to show the install view.
                Some(view) => view.post_json(r#"{"type":"scripts_pending"}"#),
                None => {
                    if let Some(id) = self.main_window(false) {
                        self.open_tab(id, target.into(), crate::tabs::OpenOptions::foreground());
                    }
                }
            },
        }
    }

    /// Starts an update check (automatic: only scripts not checked for a day).
    pub fn check_script_updates(&mut self, force: bool, reply: Option<(ViewKey, u64)>) {
        const DAY_MS: i64 = 24 * 3600 * 1000;
        let checks = hot::with(|h| h.scripts.due_for_update(DAY_MS, force)).unwrap_or_default();
        if checks.is_empty() {
            if let Some((key, rid)) = reply {
                crate::commands::reply(key, rid, serde_json::json!({ "checked": 0, "updated": 0, "failed": 0 }));
            }
            return;
        }
        crate::scripts::check_updates(checks, reply, self.worker.clone());
    }

    pub fn finish_script_updates(
        &mut self,
        updated: Vec<crate::scripts::Fetched>,
        checked: Vec<String>,
        failed: usize,
        reply: Option<(ViewKey, u64)>,
    ) {
        let mut names = Vec::new();
        hot::with(|h| {
            for f in &updated {
                // `install` keeps the enabled flag and stored values of the existing script.
                match h.scripts.install(&f.source, &f.requires, "") {
                    Ok(_) => names.push(mizu_userscripts::ScriptMeta::parse(&f.source).map(|m| m.name).unwrap_or_default()),
                    Err(e) => tracing::warn!(error = %e, "could not apply userscript update"),
                }
            }
            h.scripts.mark_checked(&checked);
        });
        if !names.is_empty() {
            self.toast_all(&format!("Updated: {}", names.join(", ")));
        }
        if let Some((key, rid)) = reply {
            crate::commands::reply(
                key,
                rid,
                serde_json::json!({ "checked": checked.len(), "updated": names.len(), "failed": failed, "names": names }),
            );
        }
    }

    pub fn toast_all(&self, message: &str) {
        let ev = mizu_ui::Event::Toast { message: message.into() }.to_json();
        for w in &self.windows {
            w.post_chrome(&ev);
        }
    }
}
