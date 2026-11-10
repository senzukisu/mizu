//! Browser state and lifecycle.

use crate::downloads::Downloads;
use crate::flyout::Flyout;
use crate::handler::Dispatcher;
use crate::hot::{self, Hot, Role};
use crate::queue::{self, Queued, WM_MIZU_PUMP, WM_MIZU_WORKER, WorkerResult, WorkerSender};
use crate::single_instance;
use crate::window::{self, BrowserWindow};
use crate::LaunchArgs;
use mizu_core::ids::{TabId, WindowId};
use mizu_core::paths::ProfilePaths;
use mizu_engine::{Engine, EngineOptions, ProfileKind, View, ViewConfig, ViewEvent, ViewKey};
use mizu_settings::Settings;
use mizu_storage::Storage;
use mizu_tabs::SessionSnapshot;
use mizu_ui::Event;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

const TIMER_SESSION: usize = 1;
const TIMER_SLEEP: usize = 2;
const TIMER_STATS: usize = 3;
const TIMER_FLUSH: usize = 4;

/// A permission prompt waiting for the user.
pub struct PendingPermission {
    pub window: WindowId,
    pub tab: TabId,
    pub origin: String,
    pub kind: mizu_privacy::PermissionKind,
    pub request: mizu_engine::PermissionRequest,
}

/// A userscript fetched and waiting for the user's confirmation on `mizu://scripts`.
pub struct PendingInstall {
    pub url: String,
    pub source: String,
    pub requires: String,
    pub meta: mizu_userscripts::ScriptMeta,
}

pub struct App {
    pub paths: ProfilePaths,
    pub settings: Settings,
    pub storage: Storage,
    pub engine: Rc<Engine>,
    pub handler: Rc<Dispatcher>,
    pub windows: Vec<BrowserWindow>,
    pub downloads: Downloads,
    pub worker: WorkerSender,
    worker_rx: Receiver<WorkerResult>,
    pub msg_hwnd: HWND,
    pub prompts: HashMap<u64, PendingPermission>,
    /// Permission answers that must not be persisted (private windows, "this time only").
    pub session_permissions: HashMap<(String, String), bool>,
    pub pending_install: Option<PendingInstall>,
    /// Tab that asked to install a userscript (shows the install page when fetched).
    pub script_install_tab: Option<ViewKey>,
    pub crashed_last_time: bool,
    pub previous_session: Option<SessionSnapshot>,
    pub session_dirty: bool,
    pub filters_updating: bool,
    pub startup_notice: Option<String>,
    /// Replies that complete asynchronously: (view key, request id).
    pub media_waiters: HashMap<u64, (ViewKey, u64)>,
    /// Cached bookmarks-bar items for the chrome state (None = needs reload).
    pub bookmarks_bar: Option<Vec<serde_json::Value>>,
    /// Last time download progress was pushed to the UI (progress events are throttled).
    last_downloads_push: std::time::Instant,
    pub quitting: bool,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` with the browser state; `None` if it is unavailable or already borrowed.
pub fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| {
        let mut guard = a.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

fn engine_args(settings: &Settings) -> Vec<String> {
    let mut args = Vec::new();
    if !settings.performance.hardware_acceleration {
        args.push("--disable-gpu".into());
    }
    if settings.media.autoplay == mizu_settings::AutoplayPolicy::RequireInteraction {
        args.push("--autoplay-policy=document-user-activation-required".into());
    }
    args
}

fn create_message_window(paths: &ProfilePaths) -> HWND {
    // SAFETY: registering a class and creating a message-only window.
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let name = crate::win::wide(single_instance::MESSAGE_CLASS);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(message_wndproc),
            hInstance: instance.into(),
            lpszClassName: windows_core::PCWSTR(name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&class);
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            &name,
            &crate::win::wide(&single_instance::message_window_title(paths)),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
        .unwrap_or_default()
    }
}

pub fn start(paths: ProfilePaths, args: LaunchArgs) -> Result<(), String> {
    paths.ensure().map_err(|e| format!("Could not create the profile folder:\n{e}"))?;

    let crashed_last_time = paths.crash_marker().exists();
    let _ = std::fs::write(paths.crash_marker(), std::process::id().to_string());

    let mut settings = mizu_settings::load(&paths.settings());
    settings.sanitize();
    let (storage, outcome) = Storage::open(&paths.database(), &paths.database_backup())
        .map_err(|e| format!("Could not open the profile database:\n{e}"))?;
    let startup_notice = match outcome {
        mizu_storage::OpenOutcome::Clean => None,
        mizu_storage::OpenOutcome::RestoredFromBackup { .. } => {
            Some("Your profile database was damaged and has been restored from the last backup.".to_owned())
        }
        mizu_storage::OpenOutcome::RecreatedEmpty { .. } => {
            Some("Your profile database was damaged and could not be restored; a new one was created.".to_owned())
        }
    };
    let _ = storage.mark_stale_downloads_interrupted();
    let previous_session = storage.load_session::<SessionSnapshot>().ok().flatten();

    let msg_hwnd = create_message_window(&paths);
    queue::set_target(msg_hwnd);
    let (worker, worker_rx) = queue::worker_channel();

    let engine = Engine::create(&EngineOptions {
        user_data_dir: paths.engine_data(),
        language: String::new(),
        browser_args: engine_args(&settings),
        extensions: settings.advanced.extensions,
        internal_scheme: mizu_core::internal::SCHEME.into(),
    })
    .map_err(|e| {
        format!(
            "{e}\n\nMizu uses the Microsoft Edge WebView2 Runtime, which is part of Windows 11 and most \
             Windows 10 installations. You can install it from https://developer.microsoft.com/microsoft-edge/webview2/"
        )
    })?;
    tracing::info!(engine = engine.version(), "engine ready");

    let lifetime_blocked = std::fs::read_to_string(paths.root.join("stats.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v["lifetime_blocked"].as_u64())
        .unwrap_or(0);
    let stats = mizu_privacy::BlockStats { lifetime_blocked, ..Default::default() };

    hot::install(Hot {
        roles: HashMap::new(),
        views: HashMap::new(),
        private: Default::default(),
        top_urls: HashMap::new(),
        internal_allowed: Default::default(),
        blocker: None,
        content_blocking: settings.privacy.content_blocking,
        allowlist: settings.privacy.allowlist.iter().map(|h| crate::shield::normalize_host(h)).collect(),
        global_privacy_control: settings.privacy.global_privacy_control,
        theme_attrs: crate::theme::html_attrs(&settings),
        document_exceptions: Default::default(),
        page_blocked: HashMap::new(),
        stats,
        stats_dirty: false,
        scripts: mizu_userscripts::ScriptStore::open(&paths.root.join("scripts")),
        gm_tokens: HashMap::new(),
    });

    window::register_class();
    crate::flyout::register_class();

    let downloads = Downloads::load(&storage);
    let app = App {
        paths,
        settings,
        storage,
        engine: Rc::new(engine),
        handler: Rc::new(Dispatcher),
        windows: Vec::new(),
        downloads,
        worker,
        worker_rx,
        msg_hwnd,
        prompts: HashMap::new(),
        session_permissions: HashMap::new(),
        pending_install: None,
        script_install_tab: None,
        crashed_last_time,
        previous_session,
        session_dirty: false,
        filters_updating: true,
        startup_notice,
        media_waiters: HashMap::new(),
        bookmarks_bar: None,
        last_downloads_push: std::time::Instant::now(),
        quitting: false,
    };
    APP.with(|a| *a.borrow_mut() = Some(app));

    with(|app| {
        crate::shield::rebuild(&app.paths, &app.settings.privacy, app.worker.clone(), true, false);
        app.open_initial_windows(&args);
        // SAFETY: timers on our message window.
        unsafe {
            SetTimer(Some(app.msg_hwnd), TIMER_SESSION, 3_000, None);
            SetTimer(Some(app.msg_hwnd), TIMER_SLEEP, 60_000, None);
            SetTimer(Some(app.msg_hwnd), TIMER_STATS, 15_000, None);
        }
    });
    Ok(())
}

pub fn shutdown() {
    let app = APP.with(|a| a.borrow_mut().take());
    if let Some(mut app) = app {
        app.save_stats();
        app.clear_on_exit();
        if let Err(e) = mizu_settings::save(&app.paths.settings(), &app.settings) {
            tracing::warn!(error = %e, "could not save settings");
        }
        let _ = std::fs::remove_file(app.paths.crash_marker());
        drop(app);
    }
}

extern "system" fn message_wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MIZU_PUMP => {
            pump();
            LRESULT(0)
        }
        WM_MIZU_WORKER => {
            let results: Vec<WorkerResult> = with(|a| a.worker_rx.try_iter().collect()).unwrap_or_default();
            for r in results {
                queue::with_app(move |a| a.on_worker_result(r));
            }
            LRESULT(0)
        }
        WM_TIMER => {
            let id = wparam.0;
            queue::with_app(move |a| a.on_timer(id));
            LRESULT(0)
        }
        WM_COPYDATA => {
            // SAFETY: lparam points to a COPYDATASTRUCT for the duration of this message.
            let cds = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
            if cds.dwData == single_instance::COPYDATA_LAUNCH && !cds.lpData.is_null() && cds.cbData < 1 << 20 {
                // SAFETY: the sender guarantees cbData bytes at lpData during SendMessage.
                let data = unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as usize) };
                if let Some(args) = single_instance::decode(data) {
                    queue::with_app(move |a| a.on_forwarded_launch(args));
                }
            }
            LRESULT(1)
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Processes queued engine events and UI messages with exclusive access to the state.
fn pump() {
    while let Some(item) = queue::pop() {
        match item {
            Queued::Outside(f) => f(),
            other => {
                if with(|app| app.handle(other)).is_none() {
                    tracing::debug!("queued item dropped: state unavailable");
                }
            }
        }
    }
    with(|app| app.flush());
}

impl App {
    pub fn window(&self, id: WindowId) -> Option<&BrowserWindow> {
        self.windows.iter().find(|w| w.id == id)
    }

    pub fn window_mut(&mut self, id: WindowId) -> Option<&mut BrowserWindow> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    /// The window that should receive new tabs from outside (last active normal window).
    pub fn main_window(&self, private: bool) -> Option<WindowId> {
        self.windows.iter().rev().find(|w| w.private == private && !w.closing).map(|w| w.id)
    }

    pub fn storage_for(&self, private: bool) -> Option<&Storage> {
        (!private).then_some(&self.storage)
    }

    fn handle(&mut self, item: Queued) {
        match item {
            Queued::View(key, event) => self.on_view_event(key, event),
            Queued::Message { key, source, json } => crate::commands::on_message(self, key, &source, &json),
            Queued::NewWindow(key, req) => self.on_new_window_request(key, req),
            Queued::Permission(key, req) => self.on_permission_request(key, req),
            Queued::Download(key, dl) => self.on_download_starting(key, dl),
            Queued::DownloadUpdate(id, info) => {
                let private = self.downloads.get(id).is_some_and(|d| d.private);
                let storage = if private { None } else { Some(&self.storage) };
                self.downloads.update(id, info, storage);
            }
            Queued::Shortcut(key, s) => self.on_shortcut(key, s),
            Queued::InstallScript { key, url } => self.begin_script_install(key, url),
            Queued::ShowBlocked { key, url } => {
                let target = format!("mizu://blocked/?url={}", url::form_urlencoded::byte_serialize(url.as_bytes()).collect::<String>());
                if let Some(view) = hot::view(key) {
                    hot::allow_internal(key);
                    view.navigate(&target);
                }
            }
            Queued::WithApp(f) => f(self),
            Queued::Outside(_) => unreachable!("handled by pump"),
        }
    }

    /// Pushes coalesced state to UI surfaces.
    fn flush(&mut self) {
        let stats_dirty = hot::with(|h| std::mem::take(&mut h.stats_dirty)).unwrap_or(false);
        if stats_dirty {
            for w in &mut self.windows {
                w.state_dirty = true;
            }
        }
        // Progress events arrive many times per second per download; ~4 UI updates per second
        // are enough. State changes (finished, paused, failed) are pushed immediately.
        let throttled = !self.downloads.urgent
            && self.last_downloads_push.elapsed() < std::time::Duration::from_millis(250);
        if self.downloads.dirty && throttled {
            // Make sure the final progress reaches the UI even if no further event arrives.
            // SAFETY: one-shot timer on our message window.
            unsafe {
                SetTimer(Some(self.msg_hwnd), TIMER_FLUSH, 260, None);
            }
        }
        if self.downloads.dirty && !throttled {
            self.downloads.dirty = false;
            self.downloads.urgent = false;
            self.last_downloads_push = std::time::Instant::now();
            let any_private = self.windows.iter().any(|w| w.private);
            let items = self.downloads.to_json(any_private);
            let ev = Event::Downloads { items: items.clone() }.to_json();
            for w in &mut self.windows {
                w.state_dirty = true;
                if w.flyout.is_open(mizu_ui::protocol::FlyoutKind::Downloads) {
                    if let Some(v) = &w.flyout.view {
                        v.post_json(&ev);
                    }
                }
            }
            let normal_ev = Event::Downloads { items: self.downloads.to_json(false) }.to_json();
            crate::internal::broadcast_to_pages(self, "downloads", &normal_ev);
        }
        let dirty: Vec<WindowId> = self.windows.iter().filter(|w| w.state_dirty).map(|w| w.id).collect();
        for id in dirty {
            self.push_window_state(id);
        }
    }

    pub fn on_worker_result(&mut self, result: WorkerResult) {
        match result {
            WorkerResult::BlockerReady(blocker, generation) => {
                if crate::shield::is_current(generation) {
                    hot::with(|h| h.blocker = Some(blocker));
                    tracing::info!("content blocker active");
                }
            }
            WorkerResult::FiltersUpdated => {
                self.filters_updating = false;
                crate::internal::broadcast_to_pages(self, "shield", r#"{"type":"toast","message":"Filter lists updated"}"#);
            }
            WorkerResult::ScriptFetched { url, result } => self.finish_script_fetch(url, result),
            WorkerResult::ExtensionDownloaded { key, rid, result } => {
                crate::extensions::finish_store_install(self, key, rid, result);
            }
            WorkerResult::MediaReport(report, token) => {
                if let Some((key, rid)) = self.media_waiters.remove(&token) {
                    crate::commands::reply(key, rid, serde_json::to_value(&*report).unwrap_or_default());
                }
            }
        }
    }

    fn on_timer(&mut self, id: usize) {
        match id {
            TIMER_SESSION if self.session_dirty => {
                self.session_dirty = false;
                self.save_session();
            }
            TIMER_SLEEP => self.sleep_background_tabs(),
            TIMER_STATS => self.save_stats(),
            TIMER_FLUSH => {
                // SAFETY: stopping our own one-shot timer.
                unsafe {
                    let _ = KillTimer(Some(self.msg_hwnd), TIMER_FLUSH);
                }
                self.downloads.urgent = true;
            }
            _ => {}
        }
    }

    fn save_stats(&self) {
        let lifetime = hot::with(|h| h.stats.lifetime_blocked).unwrap_or(0);
        let json = serde_json::json!({ "lifetime_blocked": lifetime }).to_string();
        let tmp = self.paths.root.join("stats.json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(tmp, self.paths.root.join("stats.json"));
        }
    }

    fn on_forwarded_launch(&mut self, args: LaunchArgs) {
        if args.urls.is_empty() {
            let id = self.main_window(args.private);
            match id {
                Some(id) => {
                    if let Some(w) = self.window(id) {
                        window::focus_window(w.hwnd);
                    }
                    if !args.private {
                        self.new_window(false, None);
                    }
                }
                None => {
                    self.new_window(args.private, None);
                }
            }
            return;
        }
        let urls: Vec<String> = args
            .urls
            .iter()
            .filter_map(|u| crate::omnibox::resolve_input(&self.settings, u))
            .collect();
        match self.main_window(args.private) {
            Some(id) => {
                for u in urls {
                    self.open_tab(id, u, crate::tabs::OpenOptions::foreground());
                }
                if let Some(w) = self.window(id) {
                    window::focus_window(w.hwnd);
                }
            }
            None => {
                self.new_window(args.private, Some(urls));
            }
        }
    }

    fn open_initial_windows(&mut self, args: &LaunchArgs) {
        use mizu_settings::StartupBehavior;
        let urls: Vec<String> = args.urls.iter().filter_map(|u| crate::omnibox::resolve_input(&self.settings, u)).collect();
        if args.private {
            self.new_window(true, Some(urls));
            return;
        }
        let restore = match self.settings.general.startup {
            StartupBehavior::RestoreSession => true,
            _ => self.crashed_last_time && self.settings.general.restore_after_crash,
        };
        let session = if restore { self.previous_session.take() } else { None };
        match session {
            Some(s) if !s.is_empty() => {
                self.restore_session(&s);
                if !urls.is_empty() {
                    if let Some(id) = self.main_window(false) {
                        for u in urls {
                            self.open_tab(id, u, crate::tabs::OpenOptions::foreground());
                        }
                    }
                }
            }
            _ => {
                let initial = if urls.is_empty() {
                    match self.settings.general.startup {
                        StartupBehavior::HomePage => vec![self.settings.general.home_page.clone()],
                        _ => Vec::new(),
                    }
                } else {
                    urls
                };
                self.new_window(false, Some(initial));
            }
        }
    }

    /// Opens a browser window. `urls`: `None` → one new-tab page, `Some([])` → same.
    pub fn new_window(&mut self, private: bool, urls: Option<Vec<String>>) -> WindowId {
        self.new_window_with(private, urls, None, None)
    }

    pub fn new_window_with(
        &mut self,
        private: bool,
        urls: Option<Vec<String>>,
        placement: Option<(i32, i32, i32, i32, bool)>,
        snapshot: Option<&mizu_tabs::WindowSnapshot>,
    ) -> WindowId {
        let id = WindowId::next();
        let title = if private { "Mizu – Private" } else { "Mizu" };
        let hwnd = window::create_native(id, title, placement);
        crate::win::apply_backdrop(hwnd, crate::theme::is_dark(&self.settings) || private, self.settings.appearance.translucency && !private);
        let chrome_key = TabId::next().0;
        hot::register(chrome_key, Role::Chrome(id), private);
        let flyout = Flyout::new(hwnd, id);
        hot::register(flyout.key, Role::Flyout(id), private);

        let mut strip = mizu_tabs::TabStrip::new();
        let mut tabs = HashMap::new();
        if let Some(snap) = snapshot {
            strip = mizu_tabs::TabStrip::from_snapshot(snap);
            for t in strip.tabs() {
                tabs.insert(t.id, window::TabState::new(Some(t.url.clone())));
                hot::register(t.id.0, Role::Tab { window: id, tab: t.id }, private);
            }
        }

        self.windows.push(BrowserWindow {
            id,
            hwnd,
            private,
            strip,
            tabs,
            chrome: None,
            chrome_key,
            chrome_ready: false,
            chrome_height: 86.0,
            omnibox_anchor: mizu_ui::Anchor { x: 120.0, y: 44.0, width: 600.0, height: 36.0 },
            flyout,
            fullscreen: Default::default(),
            suggestions: Vec::new(),
            state_dirty: true,
            closing: false,
        });

        self.create_surface_views(id);

        let urls = urls.unwrap_or_default();
        if snapshot.is_none() {
            if urls.is_empty() {
                self.open_tab(id, mizu_core::internal::InternalPage::NewTab.url(), crate::tabs::OpenOptions::foreground());
            } else {
                for (i, u) in urls.into_iter().enumerate() {
                    let mut o = crate::tabs::OpenOptions::foreground();
                    o.activate = i == 0;
                    self.open_tab(id, u, o);
                }
            }
        } else if let Some(active) = self.window(id).and_then(|w| w.active_tab()) {
            self.activate_tab(id, active);
        }
        window::show_native(hwnd, placement.is_some_and(|p| p.4));
        if let Some(w) = self.window(id) {
            w.layout();
        }
        self.session_dirty = true;
        id
    }

    fn create_surface_views(&mut self, id: WindowId) {
        let Some(w) = self.window(id) else { return };
        let private = w.private;
        let hwnd = w.hwnd;
        let chrome_key = w.chrome_key;
        let flyout_hwnd = w.flyout.hwnd;
        let flyout_key = w.flyout.key;
        let profile = if private { ProfileKind::Private } else { ProfileKind::Normal };
        let base = ViewConfig {
            key: chrome_key,
            profile,
            chrome: true,
            devtools: cfg!(debug_assertions),
            smartscreen: false,
            background: crate::theme::chrome_rgba(&self.settings, private),
            intercept_all: false,
            internal_scheme: mizu_core::internal::SCHEME.into(),
        };
        View::create(&self.engine, hwnd, base.clone(), self.handler.clone(), move |res| {
            queue::with_app(move |app| match res {
                Ok(view) => {
                    hot::attach_view(chrome_key, view.clone());
                    view.navigate("mizu://chrome/");
                    match app.window_mut(id) {
                        Some(w) => {
                            w.chrome = Some(view);
                            w.layout();
                        }
                        None => view.close(),
                    }
                }
                Err(e) => tracing::error!(error = %e, "chrome view failed"),
            });
        });
        let flyout_cfg = ViewConfig {
            key: flyout_key,
            background: {
                let [r, g, b] = crate::theme::frame_rgb(&self.settings, private);
                [r, g, b, 0xff]
            },
            ..base
        };
        View::create(&self.engine, flyout_hwnd, flyout_cfg, self.handler.clone(), move |res| {
            queue::with_app(move |app| match res {
                Ok(view) => {
                    hot::attach_view(flyout_key, view.clone());
                    view.navigate("mizu://flyout/");
                    match app.window_mut(id) {
                        Some(w) => w.flyout.view = Some(view),
                        None => view.close(),
                    }
                }
                Err(e) => tracing::error!(error = %e, "flyout view failed"),
            });
        });
    }

    pub fn on_window_resized(&mut self, id: WindowId) {
        if let Some(w) = self.window_mut(id) {
            w.layout();
            w.flyout.hide();
            w.state_dirty = true;
        }
        self.session_dirty = true;
    }

    pub fn on_window_moved(&mut self, id: WindowId) {
        if let Some(w) = self.window_mut(id) {
            if let Some(c) = &w.chrome {
                c.notify_parent_moved();
            }
            if let Some(v) = w.active_view() {
                v.notify_parent_moved();
            }
            w.flyout.hide();
        }
        self.session_dirty = true;
    }

    pub fn on_window_activated(&mut self, id: WindowId, active: bool) {
        if active {
            // Most recently used window last, so `main_window` finds it.
            if let Some(pos) = self.windows.iter().position(|w| w.id == id) {
                let w = self.windows.remove(pos);
                self.windows.push(w);
            }
            if let Some(w) = self.window(id) {
                if !w.flyout.is_open(mizu_ui::protocol::FlyoutKind::Omnibox) {
                    if let Some(v) = w.active_view() {
                        v.focus();
                    }
                }
            }
        }
    }

    pub fn flyout_deactivated(&mut self, id: WindowId) {
        let pending_prompt = self.window_mut(id).and_then(|w| {
            let p = w.flyout.prompt_id.take();
            w.flyout.hide();
            p
        });
        if let Some(pid) = pending_prompt {
            // Dismissed by clicking elsewhere: deny once, do not remember.
            if let Some(p) = self.prompts.remove(&pid) {
                p.request.respond(Some(false));
            }
            self.show_next_prompt(id);
        }
    }

    pub fn on_system_theme_changed(&mut self) {
        if self.settings.appearance.theme == mizu_settings::Theme::System {
            self.apply_settings_to_surfaces();
        }
    }

    pub fn request_close_window(&mut self, id: WindowId) {
        let Some(w) = self.window(id) else { return };
        let count = w.strip.len();
        let needs_confirm = count > 1
            && self.settings.general.confirm_close_multiple_tabs
            && self.windows.len() == 1
            && w.flyout.ready;
        if !needs_confirm {
            self.close_window(id);
            return;
        }
        let width = crate::win::client_rect(w.hwnd).right as f64 / crate::win::dpi_scale(w.hwnd);
        let restore = !w.private && self.settings.general.startup == mizu_settings::StartupBehavior::RestoreSession;
        let data = serde_json::json!({
            "title": format!("Close {count} tabs?"),
            "message": if w.private {
                "Private tabs and their data will be gone for good.".to_owned()
            } else if restore {
                "They will be reopened the next time you start Mizu.".to_owned()
            } else {
                "You can reopen them from History.".to_owned()
            },
            "confirm": "Close Mizu",
            "can_skip": true,
        });
        let anchor = mizu_ui::Anchor { x: width / 2.0, y: 60.0, width: 0.0, height: 0.0 };
        self.show_flyout(id, mizu_ui::protocol::FlyoutKind::Confirm, anchor, data, None);
    }

    pub fn close_window(&mut self, id: WindowId) {
        let normal_remaining = self.windows.iter().filter(|w| !w.private && w.id != id).count();
        let is_private = self.window(id).is_some_and(|w| w.private);
        if !is_private && normal_remaining == 0 {
            // Last normal window: remember it for "restore session".
            self.save_session();
            self.quitting_session_saved();
        }
        let Some(pos) = self.windows.iter().position(|w| w.id == id) else { return };
        let mut w = self.windows.remove(pos);
        w.closing = true;
        for (tab, state) in w.tabs.drain() {
            if let Some(v) = state.view {
                v.close();
            }
            hot::forget(tab.0);
        }
        if let Some(c) = w.chrome.take() {
            c.close();
        }
        hot::forget(w.chrome_key);
        hot::forget(w.flyout.key);
        w.flyout.destroy();
        window::destroy_native(w.hwnd);
        self.prompts.retain(|_, p| p.window != id);

        if is_private && !self.windows.iter().any(|w| w.private) {
            // InPrivate engine data is discarded by the engine with its last view; drop our
            // in-memory traces too.
            self.session_permissions.clear();
            self.downloads.forget_private();
            tracing::info!("last private window closed, private session discarded");
        }
        if self.windows.is_empty() {
            self.quitting = true;
            // SAFETY: ends the message loop.
            unsafe { PostQuitMessage(0) };
        } else {
            self.session_dirty = true;
        }
    }

    fn quitting_session_saved(&mut self) {
        self.session_dirty = false;
    }

    pub fn on_view_event(&mut self, key: ViewKey, event: ViewEvent) {
        match hot::role(key) {
            Some(Role::Tab { window, tab }) => self.on_tab_event(window, tab, event),
            Some(Role::Chrome(id)) => {
                if let ViewEvent::ProcessFailed { .. } = event {
                    tracing::error!("browser UI process failed, reloading");
                    if let Some(c) = self.window(id).and_then(|w| w.chrome.clone()) {
                        c.navigate("mizu://chrome/");
                    }
                }
            }
            Some(Role::Flyout(id)) => {
                if let ViewEvent::ProcessFailed { .. } = event {
                    if let Some(v) = self.window(id).and_then(|w| w.flyout.view.clone()) {
                        v.navigate("mizu://flyout/");
                    }
                }
            }
            None => {}
        }
    }

    /// Re-applies appearance and engine settings everywhere after a change.
    pub fn apply_settings_to_surfaces(&mut self) {
        let ui = crate::theme::ui_settings(&self.settings);
        let ev = Event::Settings { settings: ui }.to_json();
        let dark = crate::theme::is_dark(&self.settings);
        for w in &mut self.windows {
            crate::win::apply_backdrop(w.hwnd, dark || w.private, self.settings.appearance.translucency && !w.private);
            if let Some(c) = &w.chrome {
                c.set_background(crate::theme::chrome_rgba(&self.settings, w.private));
                c.post_json(&ev);
            }
            if let Some(f) = &w.flyout.view {
                f.post_json(&ev);
            }
            for t in w.tabs.values() {
                if let Some(v) = &t.view {
                    crate::tabs::apply_view_settings(&self.settings, v, w.private);
                }
            }
            w.state_dirty = true;
            // SAFETY: repaint our own window with the new frame colour.
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(w.hwnd), None, true);
            }
        }
        crate::internal::broadcast_to_pages(self, "", &ev);
        hot::with(|h| {
            h.content_blocking = self.settings.privacy.content_blocking;
            h.global_privacy_control = self.settings.privacy.global_privacy_control;
            h.theme_attrs = crate::theme::html_attrs(&self.settings);
            h.allowlist = self.settings.privacy.allowlist.iter().map(|x| crate::shield::normalize_host(x)).collect();
        });
    }

    fn clear_on_exit(&mut self) {
        let c = self.settings.privacy.clear_on_exit;
        if c.history {
            let _ = self.storage.clear_history(mizu_storage::TimeRange::All);
        }
        if c.downloads {
            let _ = self.storage.clear_download_history();
        }
        // Cookies/cache/site storage are cleared by the engine at the next start, because the
        // engine is already shutting down here (see on startup in tabs::apply_view_settings).
        if c.cookies || c.cache || c.site_storage {
            let _ = std::fs::write(self.paths.root.join("clear-on-start"), b"1");
        }
    }
}
