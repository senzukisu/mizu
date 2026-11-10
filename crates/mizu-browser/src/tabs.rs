//! Tab lifecycle, tab events, pop-ups, permissions, downloads and shortcuts.

use crate::app::{App, PendingPermission};
use crate::hot::{self, Role};
use crate::queue;
use crate::shortcuts::Shortcut;
use crate::window::TabState;
use base64::Engine as _;
use mizu_core::ids::{PromptId, TabId, WindowId};
use mizu_core::internal::{InternalPage, is_internal_url};
use mizu_engine::{Download, NewWindowRequest, PermissionRequest, ProfileKind, View, ViewConfig, ViewEvent, ViewKey};
use mizu_privacy::{PermissionDefault, PermissionKind};
use mizu_settings::{NewTabPosition, Settings, TrackingPrevention};
use mizu_storage::PermissionState;
use mizu_tabs::Tab;
use mizu_ui::Event;
use mizu_ui::protocol::FlyoutKind;
use serde_json::json;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct OpenOptions {
    pub activate: bool,
    /// Create the engine view now even if the tab stays in the background.
    pub load_now: bool,
    /// Insert next to the active tab rather than at the end.
    pub adjacent: bool,
}

impl OpenOptions {
    pub fn foreground() -> Self {
        Self { activate: true, load_now: true, adjacent: false }
    }
    pub fn background() -> Self {
        Self { activate: false, load_now: true, adjacent: true }
    }
}

/// Engine settings that follow user preferences.
pub fn apply_view_settings(settings: &Settings, view: &View, private: bool) {
    view.apply_live_settings(settings.advanced.developer_tools, settings.security.smartscreen);
    let level = match settings.privacy.tracking_prevention {
        TrackingPrevention::Off => mizu_engine::TrackingLevel::None,
        TrackingPrevention::Basic => mizu_engine::TrackingLevel::Basic,
        TrackingPrevention::Balanced => mizu_engine::TrackingLevel::Balanced,
        TrackingPrevention::Strict => mizu_engine::TrackingLevel::Strict,
    };
    view.set_tracking_prevention(level);
    view.set_autofill(
        settings.privacy.engine_password_saving && !private,
        settings.privacy.engine_autofill && !private,
    );
    view.set_color_scheme(match settings.appearance.theme {
        mizu_settings::Theme::Dark => Some(true),
        mizu_settings::Theme::Light => Some(false),
        mizu_settings::Theme::System => None,
    });
    view.set_zoom(f64::from(settings.appearance.zoom_percent) / 100.0);
}

fn png_data_url(png: &[u8]) -> Option<String> {
    (!png.is_empty()).then(|| format!("data:image/png;base64,{}", base64::prelude::BASE64_STANDARD.encode(png)))
}

impl App {
    // ───────────────────────────── Opening & closing ─────────────────────────────

    pub fn open_tab(&mut self, window: WindowId, url: String, opts: OpenOptions) -> Option<TabId> {
        let position = self.settings.tabs.new_tab_position;
        let w = self.window_mut(window)?;
        let tab = Tab::new(url.clone());
        let id = tab.id;
        if opts.adjacent && position == NewTabPosition::AfterCurrent {
            w.strip.insert_after_active(tab);
        } else {
            w.strip.push(tab);
        }
        w.tabs.insert(id, TabState::new(Some(url)));
        hot::register(id.0, Role::Tab { window, tab: id }, w.private);
        w.state_dirty = true;
        if opts.activate {
            self.activate_tab(window, id);
        } else if opts.load_now {
            self.ensure_view(window, id);
        }
        self.session_dirty = true;
        Some(id)
    }

    pub fn activate_tab(&mut self, window: WindowId, tab: TabId) {
        let efficiency = self.settings.performance.efficiency_mode;
        let Some(w) = self.window_mut(window) else { return };
        let previous = w.strip.active();
        if !w.strip.activate(tab) {
            return;
        }
        if let Some(prev) = previous.filter(|p| *p != tab) {
            if let Some(v) = w.tabs.get(&prev).and_then(|t| t.view.clone()) {
                v.set_visible(false);
                if efficiency {
                    v.set_low_memory(true);
                }
            }
        }
        let content = w.content_rect();
        if let Some(state) = w.tabs.get_mut(&tab) {
            state.last_active = Instant::now();
            if let Some(v) = &state.view {
                if state.sleeping {
                    v.resume();
                    state.sleeping = false;
                }
                v.set_low_memory(false);
                v.set_bounds(content);
                v.set_visible(true);
                v.focus();
            }
        }
        w.flyout.hide();
        w.state_dirty = true;
        let needs_view = w.tabs.get(&tab).is_some_and(|t| t.view.is_none());
        let title = w.strip.get(tab).map(|t| t.title.clone()).unwrap_or_default();
        let hwnd = w.hwnd;
        crate::window::set_title(hwnd, &if title.is_empty() { "Mizu".to_owned() } else { format!("{title} – Mizu") });
        if needs_view {
            self.ensure_view(window, tab);
        }
        self.session_dirty = true;
    }

    /// Creates the engine view for a tab if it does not exist yet (lazy loading).
    pub fn ensure_view(&mut self, window: WindowId, tab: TabId) {
        let (hwnd, private) = {
            let Some(w) = self.window_mut(window) else { return };
            let Some(state) = w.tabs.get_mut(&tab) else { return };
            if state.view.is_some() || state.creating {
                return;
            }
            state.creating = true;
            (w.hwnd, w.private)
        };
        let settings_bg = crate::theme::page_rgba(&self.settings, private);
        let config = ViewConfig {
            key: tab.0,
            profile: if private { ProfileKind::Private } else { ProfileKind::Normal },
            chrome: false,
            devtools: self.settings.advanced.developer_tools,
            smartscreen: self.settings.security.smartscreen,
            background: settings_bg,
            intercept_all: true,
            internal_scheme: mizu_core::internal::SCHEME.into(),
        };
        View::create(&self.engine, hwnd, config, self.handler.clone(), move |res| {
            queue::with_app(move |app| app.on_tab_view_created(window, tab, res));
        });
    }

    fn on_tab_view_created(&mut self, window: WindowId, tab: TabId, res: mizu_engine::Result<View>) {
        let view = match res {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "could not create tab view");
                if let Some(t) = self.window_mut(window).and_then(|w| w.tabs.get_mut(&tab)) {
                    t.creating = false;
                    t.crashed = true;
                }
                return;
            }
        };
        let settings = self.settings.clone();
        let clear_flag = self.paths.root.join("clear-on-start");
        let Some(w) = self.window_mut(window) else {
            view.close();
            return;
        };
        let active = w.strip.active() == Some(tab);
        let content = w.content_rect();
        let Some(state) = w.tabs.get_mut(&tab) else {
            view.close();
            return;
        };
        hot::attach_view(tab.0, view.clone());
        apply_view_settings(&settings, &view, w.private);
        if !w.private && clear_flag.exists() {
            let _ = std::fs::remove_file(&clear_flag);
            let c = settings.privacy.clear_on_exit;
            view.clear_browsing_data(
                mizu_engine::BrowsingDataKinds { cookies: c.cookies, cache: c.cache, site_storage: c.site_storage, ..Default::default() },
                None,
                |_| tracing::info!("cleared browsing data on startup (clear on exit)"),
            );
        }
        state.creating = false;
        state.view = Some(view.clone());
        if active {
            view.set_bounds(content);
            view.set_visible(true);
            view.focus();
        } else {
            view.set_visible(false);
            if settings.performance.efficiency_mode {
                view.set_low_memory(true);
            }
        }
        if let Some(req) = state.pending_new_window.take() {
            state.pending_url = None;
            req.complete_with(&view);
        } else if let Some(url) = state.pending_url.take() {
            load(tab.0, &view, &url);
        }
        w.state_dirty = true;
    }

    pub fn close_tab(&mut self, window: WindowId, tab: TabId) {
        let Some(w) = self.window_mut(window) else { return };
        let Some(state) = w.tabs.remove(&tab) else { return };
        if let Some(v) = state.view {
            v.close();
        }
        hot::forget(tab.0);
        let next = w.strip.close(tab);
        w.state_dirty = true;
        let empty = w.strip.is_empty();
        self.prompts.retain(|_, p| p.tab != tab);
        if empty {
            self.close_window(window);
            return;
        }
        if let Some(next) = next {
            self.activate_tab(window, next);
        }
        self.session_dirty = true;
    }

    pub fn reopen_closed_tab(&mut self, window: WindowId) {
        let Some(w) = self.window_mut(window) else { return };
        let Some(id) = w.strip.reopen_closed() else { return };
        let url = w.strip.get(id).map(|t| t.url.clone()).unwrap_or_default();
        w.tabs.insert(id, TabState::new(Some(url)));
        hot::register(id.0, Role::Tab { window, tab: id }, w.private);
        self.activate_tab(window, id);
    }

    pub fn navigate_active(&mut self, window: WindowId, url: &str) {
        let Some(w) = self.window_mut(window) else { return };
        let Some(tab) = w.active_tab() else { return };
        w.flyout.hide();
        if let Some(t) = w.strip.get_mut(tab) {
            t.url = url.to_owned();
        }
        match w.tabs.get_mut(&tab) {
            Some(state) if state.view.is_some() => {
                state.crashed = false;
                let v = state.view.clone().unwrap_or_else(|| unreachable!());
                load(tab.0, &v, url);
                v.focus();
            }
            Some(state) => {
                state.pending_url = Some(url.to_owned());
                self.ensure_view(window, tab);
            }
            None => {}
        }
    }

    /// Finds the window and tab that own a view key.
    pub fn locate(&self, key: ViewKey) -> Option<(WindowId, TabId)> {
        match hot::role(key)? {
            Role::Tab { window, tab } => Some((window, tab)),
            _ => None,
        }
    }

    // ───────────────────────────── Tab events ─────────────────────────────

    pub fn on_tab_event(&mut self, window: WindowId, tab: TabId, event: ViewEvent) {
        let private = self.window(window).is_some_and(|w| w.private);
        let Some(w) = self.window_mut(window) else { return };
        let active = w.strip.active() == Some(tab);
        let mut record: Option<(String, String)> = None;
        let mut session_dirty = false;
        let mut title_update: Option<(String, String)> = None;
        {
            let Some(state) = w.tabs.get_mut(&tab) else { return };
            match event {
                ViewEvent::SourceChanged { uri } => {
                    if let Some(t) = w.strip.get_mut(tab) {
                        t.url = uri.clone();
                    }
                    if active && !w.flyout.is_open(FlyoutKind::Omnibox) {
                        w.flyout.hide();
                    }
                    session_dirty = true;
                }
                ViewEvent::TitleChanged { title } => {
                    let url = w.strip.get(tab).map(|t| t.url.clone()).unwrap_or_default();
                    if let Some(t) = w.strip.get_mut(tab) {
                        t.title = title.clone();
                    }
                    if active {
                        crate::window::set_title(w.hwnd, &format!("{title} – Mizu"));
                    }
                    title_update = Some((url, title));
                }
                ViewEvent::HistoryChanged { can_go_back, can_go_forward } => {
                    state.can_go_back = can_go_back;
                    state.can_go_forward = can_go_forward;
                }
                ViewEvent::ContentLoading => {
                    state.loading = true;
                    state.crashed = false;
                    state.favicon = None;
                }
                ViewEvent::NavigationCompleted { success, http_status, .. } => {
                    state.loading = false;
                    state.http_status = http_status;
                    let url = w.strip.get(tab).map(|t| t.url.clone()).unwrap_or_default();
                    let title = w.strip.get(tab).map(|t| t.title.clone()).unwrap_or_default();
                    if success && !private {
                        record = Some((url, title));
                    }
                }
                ViewEvent::DomContentLoaded => {}
                ViewEvent::FaviconChanged { png } => {
                    state.favicon = png_data_url(&png);
                    let ev = Event::Favicon { id: tab.0, data: state.favicon.clone().unwrap_or_default() }.to_json();
                    w.post_chrome(&ev);
                }
                ViewEvent::AudioChanged { playing, muted } => {
                    state.audible = playing;
                    state.muted = muted;
                }
                ViewEvent::FullscreenChanged { fullscreen } => {
                    if active {
                        w.set_fullscreen(None, Some(fullscreen));
                    }
                }
                ViewEvent::ProcessFailed { unresponsive, description } => {
                    tracing::warn!(tab = tab.0, unresponsive, description, "tab process failed");
                    state.crashed = true;
                    state.loading = false;
                }
                ViewEvent::CloseRequested => {
                    queue::with_app(move |a| a.close_tab(window, tab));
                }
                ViewEvent::StatusText { .. } | ViewEvent::GotFocus => {}
                ViewEvent::ZoomChanged { factor } => state.zoom = factor,
            }
        }
        w.state_dirty = true;
        if session_dirty {
            self.session_dirty = true;
        }
        if let Some((url, title)) = record {
            if !url.is_empty() {
                let _ = self.storage.record_visit(&url, &title);
            }
        }
        if let Some((url, title)) = title_update {
            if !private {
                let _ = self.storage.update_history_title(&url, &title);
            }
        }
    }

    // ───────────────────────────── State for the chrome ─────────────────────────────

    pub fn push_window_state(&mut self, id: WindowId) {
        let bookmarks: Vec<serde_json::Value> = if self.settings.appearance.show_bookmarks_bar {
            self.storage
                .bookmark_children(mizu_storage::BOOKMARKS_BAR_ID)
                .unwrap_or_default()
                .into_iter()
                .map(|b| json!({ "id": b.id, "title": b.title, "url": b.url, "kind": b.kind }))
                .collect()
        } else {
            Vec::new()
        };
        let (dl_active, dl_progress) = self.downloads.summary();
        let session_blocked = hot::with(|h| h.stats.session_blocked).unwrap_or(0);
        let Some(w) = self.window(id) else { return };
        let active = w.active_tab();
        let tabs: Vec<serde_json::Value> = w
            .strip
            .tabs()
            .iter()
            .map(|t| {
                let s = w.tabs.get(&t.id);
                json!({
                    "id": t.id.0,
                    "title": if t.title.is_empty() { tab_fallback_title(&t.url) } else { t.title.clone() },
                    "url": t.url,
                    "pinned": t.pinned,
                    "loading": s.is_some_and(|s| s.loading),
                    "audible": s.is_some_and(|s| s.audible),
                    "muted": s.is_some_and(|s| s.muted),
                    "crashed": s.is_some_and(|s| s.crashed),
                    "sleeping": s.is_some_and(|s| s.sleeping || (s.view.is_none() && !s.creating)),
                    "has_favicon": s.is_some_and(|s| s.favicon.is_some()),
                    "internal": is_internal_url(&t.url),
                })
            })
            .collect();
        let nav = active.and_then(|tab| {
            let t = w.strip.get(tab)?;
            let s = w.tabs.get(&tab)?;
            let key = tab.0;
            let blocked = hot::with(|h| h.page_blocked.get(&key).copied().unwrap_or(0)).unwrap_or(0);
            let shield_on = self.settings.privacy.content_blocking
                && !hot::with(|h| crate::shield::allowlisted(&h.allowlist, &t.url)).unwrap_or(false);
            let security = if is_internal_url(&t.url) {
                "internal"
            } else if t.url.starts_with("https://") {
                "secure"
            } else if t.url.starts_with("http://") {
                "insecure"
            } else if t.url.starts_with("file:") {
                "file"
            } else {
                "other"
            };
            let bookmarked = !w.private && self.storage.find_bookmark_by_url(&t.url).ok().flatten().is_some();
            let display = if t.url.starts_with("mizu://newtab") { String::new() } else { t.url.clone() };
            Some(json!({
                "url": t.url,
                "display": display,
                "host": mizu_core::omnibox::host_of(&t.url).unwrap_or_default(),
                "can_go_back": s.can_go_back,
                "can_go_forward": s.can_go_forward,
                "loading": s.loading,
                "security": security,
                "bookmarked": bookmarked,
                "zoom": (s.zoom * 100.0).round(),
                "blocked": blocked,
                "shield": shield_on,
                "crashed": s.crashed,
            }))
        });
        let state = json!({
            "private": w.private,
            "maximized": w.is_maximized(),
            "fullscreen": w.fullscreen.active(),
            "active": active.map(|t| t.0),
            "tabs": tabs,
            "nav": nav,
            "bookmarks": bookmarks,
            "closed": w.strip.closed_tabs().count(),
            "downloads": { "active": dl_active, "progress": dl_progress, "any": !self.downloads.items.is_empty() },
            "shield": { "session": session_blocked },
        });
        let ev = Event::State { state }.to_json();
        w.post_chrome(&ev);
        if let Some(w) = self.window_mut(id) {
            w.state_dirty = false;
        }
    }

    // ───────────────────────────── Background tabs ─────────────────────────────

    pub fn sleep_background_tabs(&mut self) {
        if !self.settings.tabs.sleeping_tabs {
            return;
        }
        let after = Duration::from_secs(u64::from(self.settings.tabs.sleep_after_minutes) * 60);
        for w in &mut self.windows {
            let active = w.strip.active();
            let mut changed = false;
            for (id, s) in w.tabs.iter_mut() {
                if Some(*id) == active || s.sleeping || s.audible || s.loading {
                    continue;
                }
                if let Some(v) = &s.view {
                    if s.last_active.elapsed() >= after {
                        v.suspend();
                        s.sleeping = true;
                        changed = true;
                    }
                }
            }
            if changed {
                w.state_dirty = true;
            }
        }
    }

    // ───────────────────────────── Pop-ups ─────────────────────────────

    pub fn on_new_window_request(&mut self, key: ViewKey, req: NewWindowRequest) {
        let Some((window, _tab)) = self.locate(key) else {
            req.block();
            return;
        };
        if !req.user_initiated && self.settings.security.block_unrequested_popups {
            tracing::info!(uri = req.uri, "blocked pop-up without user gesture");
            if let Some(w) = self.window(window) {
                w.post_chrome(&Event::Toast { message: "Pop-up blocked".into() }.to_json());
            }
            req.block();
            return;
        }
        let activate = self.settings.tabs.switch_to_new_tabs_from_links || req.uri.is_empty() || req.uri == "about:blank";
        let opts = OpenOptions { activate, load_now: true, adjacent: true };
        let url = req.uri.clone();
        if let Some(tab) = self.open_tab(window, url, opts) {
            if let Some(state) = self.window_mut(window).and_then(|w| w.tabs.get_mut(&tab)) {
                match &state.view {
                    Some(v) => req.complete_with(v),
                    None => state.pending_new_window = Some(req),
                }
            }
        }
    }

    // ───────────────────────────── Permissions ─────────────────────────────

    pub fn on_permission_request(&mut self, key: ViewKey, request: PermissionRequest) {
        let Some((window, tab)) = self.locate(key) else {
            request.respond(Some(false));
            return;
        };
        let kind = map_permission(request.kind);
        let origin = mizu_core::omnibox::origin_of(&request.uri).unwrap_or_default();
        let private = self.window(window).is_some_and(|w| w.private);

        if is_internal_url(&request.uri) || origin.is_empty() {
            request.respond(Some(false));
            return;
        }
        match kind.default_policy() {
            PermissionDefault::Allow => return request.respond(Some(true)),
            PermissionDefault::Block => return request.respond(Some(false)),
            PermissionDefault::Ask => {}
        }
        if let Some(&allow) = self.session_permissions.get(&(origin.clone(), kind.as_str().to_owned())) {
            return request.respond(Some(allow));
        }
        if !private {
            if let Ok(Some(state)) = self.storage.permission(&origin, kind.as_str()) {
                return request.respond(Some(state == PermissionState::Allow));
            }
        }
        let id = PromptId::next().0;
        self.prompts.insert(id, PendingPermission { window, tab, origin, kind, request });
        self.show_next_prompt(window);
    }

    pub fn show_next_prompt(&mut self, window: WindowId) {
        let Some(w) = self.window(window) else { return };
        if w.flyout.prompt_id.is_some() {
            return;
        }
        let active = w.active_tab();
        let next = self
            .prompts
            .iter()
            .filter(|(_, p)| p.window == window && Some(p.tab) == active)
            .map(|(id, _)| *id)
            .min();
        let Some(id) = next else { return };
        let p = &self.prompts[&id];
        let data = json!({
            "prompt_id": id,
            "origin": p.origin,
            "host": mizu_core::omnibox::host_of(&p.origin).unwrap_or_default(),
            "kind": p.kind.as_str(),
            "description": p.kind.description(),
            "private": w.private,
        });
        let anchor = w.omnibox_anchor;
        self.show_flyout(window, FlyoutKind::Permission, mizu_ui::Anchor { width: 1.0, ..anchor }, data, None);
        if let Some(w) = self.window_mut(window) {
            w.flyout.prompt_id = Some(id);
        }
    }

    pub fn answer_prompt(&mut self, prompt_id: u64, allow: bool, remember: bool) {
        let Some(p) = self.prompts.remove(&prompt_id) else { return };
        let private = self.window(p.window).is_some_and(|w| w.private);
        if remember && !private {
            let state = if allow { PermissionState::Allow } else { PermissionState::Block };
            let _ = self.storage.set_permission(&p.origin, p.kind.as_str(), state);
        } else if remember {
            self.session_permissions.insert((p.origin.clone(), p.kind.as_str().to_owned()), allow);
        }
        p.request.respond(Some(allow));
        let window = p.window;
        if let Some(w) = self.window_mut(window) {
            w.flyout.prompt_id = None;
            w.flyout.hide();
        }
        self.show_next_prompt(window);
    }

    // ───────────────────────────── Downloads ─────────────────────────────

    pub fn on_download_starting(&mut self, key: ViewKey, download: Download) {
        let located = self.locate(key);
        let private = located.and_then(|(w, _)| self.window(w)).is_some_and(|w| w.private);
        let dir = if self.settings.downloads.directory.is_empty() {
            crate::downloads::default_dir()
        } else {
            std::path::PathBuf::from(&self.settings.downloads.directory)
        };
        let _ = std::fs::create_dir_all(&dir);
        let suggested = download.suggested_path();
        let file_name = std::path::Path::new(&suggested)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".into());
        let target = crate::downloads::unique_path(&dir, &file_name);
        let target = target.to_string_lossy().into_owned();

        // Tabs opened only to retry a download close themselves.
        if let Some((window, tab)) = located {
            let close = self
                .window(window)
                .and_then(|w| w.tabs.get(&tab))
                .is_some_and(|t| t.close_if_download && !t.can_go_back);
            if close {
                queue::with_app(move |a| a.close_tab(window, tab));
            }
        }

        if self.settings.downloads.ask_where_to_save {
            let owner = located.and_then(|(w, _)| self.window(w)).map(|w| w.hwnd).unwrap_or_default();
            queue::outside(move || match crate::dialogs::save_file(owner, &target) {
                Some(path) => {
                    download.accept(Some(&path));
                    queue::with_app(move |a| a.track_download(download, private));
                }
                None => download.reject(),
            });
            return;
        }
        download.accept(Some(&target));
        self.track_download(download, private);
    }

    fn track_download(&mut self, download: Download, private: bool) {
        let storage = if private { None } else { Some(&self.storage) };
        self.downloads.start(download, private, storage);
        // The chrome opens its downloads panel itself when `open_panel_on_start` is set,
        // because only it knows where the downloads button is.
        if self.settings.downloads.open_panel_on_start {
            let ev = Event::DownloadStarted.to_json();
            if let Some(w) = self.main_window(private).and_then(|id| self.window(id)) {
                w.post_chrome(&ev);
            }
        }
    }

    // ───────────────────────────── Shortcuts ─────────────────────────────

    pub fn on_shortcut(&mut self, key: ViewKey, s: Shortcut) {
        let window = match hot::role(key) {
            Some(Role::Tab { window, .. } | Role::Chrome(window) | Role::Flyout(window)) => window,
            None => return,
        };
        crate::commands::run_shortcut(self, window, s);
    }


    pub fn show_flyout(
        &mut self,
        window: WindowId,
        kind: FlyoutKind,
        anchor: mizu_ui::Anchor,
        data: serde_json::Value,
        tab_id: Option<u64>,
    ) {
        let scale_origin = self.window(window).map(|w| {
            if w.is_maximized() || w.fullscreen.active() { 0 } else { (4.0 * crate::win::dpi_scale(w.hwnd)).round() as i32 }
        });
        let Some(w) = self.window_mut(window) else { return };
        let json = Event::Flyout { kind, data }.to_json();
        let hwnd = w.hwnd;
        w.flyout.open(hwnd, scale_origin.unwrap_or(0), kind, anchor, &json);
        w.flyout.tab_id = tab_id;
    }

    pub fn current_url(&self, window: WindowId) -> Option<String> {
        let w = self.window(window)?;
        w.strip.get(w.active_tab()?).map(|t| t.url.clone())
    }
}

/// Navigates a view, granting internal-page access when the target is internal.
pub fn load(key: ViewKey, view: &View, url: &str) {
    if is_internal_url(url) {
        hot::allow_internal(key);
    }
    view.navigate(url);
}

fn tab_fallback_title(url: &str) -> String {
    if let Some(p) = InternalPage::from_url(url) {
        return p.title().to_owned();
    }
    mizu_core::omnibox::host_of(url).unwrap_or_else(|| "New Tab".into())
}

fn map_permission(k: mizu_engine::PermissionKind) -> PermissionKind {
    use mizu_engine::PermissionKind as E;
    match k {
        E::Camera => PermissionKind::Camera,
        E::Microphone => PermissionKind::Microphone,
        E::Geolocation => PermissionKind::Geolocation,
        E::Notifications => PermissionKind::Notifications,
        E::ClipboardRead => PermissionKind::ClipboardRead,
        E::MultipleDownloads => PermissionKind::MultipleDownloads,
        E::MidiSysex => PermissionKind::MidiSysex,
        E::WindowManagement => PermissionKind::WindowManagement,
        E::LocalFonts => PermissionKind::LocalFonts,
        E::FileReadWrite => PermissionKind::FileEditing,
        E::Autoplay => PermissionKind::Autoplay,
        E::OtherSensors => PermissionKind::OtherSensors,
        E::Unknown => PermissionKind::Unknown,
    }
}
