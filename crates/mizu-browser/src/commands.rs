//! Messages from UI surfaces and pages → browser actions.
//!
//! Trust model:
//! * the chrome and flyout views (only ever showing `mizu://chrome/` / `mizu://flyout/`) may
//!   send every command;
//! * internal pages (`mizu://settings/` …) may send data commands but not chrome-only ones;
//! * ordinary web pages can only use the userscript channel, validated by token.

use crate::app::App;
use crate::hot::{self, Role};
use crate::queue;
use crate::shortcuts::Shortcut;
use crate::tabs::OpenOptions;
use mizu_core::ids::{TabId, WindowId};
use mizu_core::internal::{InternalPage, is_internal_url};
use mizu_engine::ViewKey;
use mizu_ui::protocol::{Command, Disposition, Envelope, FlyoutKind, WindowAction};
use mizu_ui::Event;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Chrome,
    Flyout,
    Page,
}

pub fn reply(key: ViewKey, rid: u64, data: Value) {
    if let Some(v) = hot::view(key) {
        v.post_json(&Event::Reply { rid, ok: true, data, error: None }.to_json());
    }
}

pub fn reply_err(key: ViewKey, rid: u64, error: String) {
    if let Some(v) = hot::view(key) {
        v.post_json(&Event::Reply { rid, ok: false, data: Value::Null, error: Some(error) }.to_json());
    }
}

pub fn on_message(app: &mut App, key: ViewKey, source: &str, json: &str) {
    let (window, surface) = match hot::role(key) {
        Some(Role::Chrome(w)) if source.starts_with("mizu://chrome/") => (w, Surface::Chrome),
        Some(Role::Flyout(w)) if source.starts_with("mizu://flyout/") => (w, Surface::Flyout),
        Some(Role::Tab { window, .. }) if is_internal_url(source) => (window, Surface::Page),
        Some(Role::Tab { window, .. }) => {
            // Untrusted web content: userscript channel only.
            let Ok(value) = serde_json::from_str::<Value>(json) else { return };
            if value.get("mizu_pw").is_some() {
                crate::passwords::on_capture(app, key, &value);
                return;
            }
            let action = hot::with(|h| crate::scripts::handle_gm_message(h, key, &value));
            if let Some(crate::scripts::GmAction::OpenTab { url, background }) = action {
                let opts = if background { OpenOptions::background() } else { OpenOptions::foreground() };
                app.open_tab(window, url, opts);
            }
            return;
        }
        _ => return,
    };
    let envelope: Envelope = match serde_json::from_str(json) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "rejected malformed UI message");
            return;
        }
    };
    if surface == Surface::Page && envelope.command.is_chrome_only() {
        tracing::warn!("internal page attempted a chrome-only command");
        return;
    }
    tracing::debug!(?surface, command = ?envelope.command, "ui command");
    crate::internal::set_current_rid(envelope.rid);
    let result = handle(app, window, key, surface, envelope.command);
    crate::internal::set_current_rid(None);
    if let Some(rid) = envelope.rid {
        match result {
            Ok(Some(data)) => reply(key, rid, data),
            Ok(None) => {} // answered asynchronously
            Err(e) => reply_err(key, rid, e),
        }
    }
}

fn ok() -> Result<Option<Value>, String> {
    Ok(Some(Value::Null))
}

fn active_tab(app: &App, window: WindowId) -> Option<TabId> {
    app.window(window).and_then(|w| w.active_tab())
}

fn tab_id(raw: u64) -> TabId {
    TabId(raw)
}

#[allow(clippy::too_many_lines)]
fn handle(app: &mut App, window: WindowId, key: ViewKey, surface: Surface, cmd: Command) -> Result<Option<Value>, String> {
    match cmd {
        // ── Surfaces ──────────────────────────────────────────────
        Command::Ready => {
            let is_sidebar = app.window(window).is_some_and(|w| w.sidebar_key == key);
            if is_sidebar {
                if let Some(w) = app.window_mut(window) {
                    w.sidebar_ready = true;
                    w.layout();
                    w.state_dirty = true;
                }
            }
            match surface {
                Surface::Chrome if is_sidebar => {}
                Surface::Chrome => {
                    if let Some(w) = app.window_mut(window) {
                        w.chrome_ready = true;
                        w.layout();
                        w.state_dirty = true;
                        w.reveal();
                    }
                    app.create_flyout_view(window);
                    app.ensure_sidebar(window);
                    app.schedule_spare();
                    if let Some(notice) = app.startup_notice.take() {
                        app.toast_all(&notice);
                    }
                }
                Surface::Flyout => {
                    if let Some(w) = app.window_mut(window) {
                        w.flyout.ready = true;
                    }
                }
                Surface::Page => {}
            }
            Ok(Some(json!({
                "settings": crate::theme::ui_settings(&app.settings),
                "private": app.window(window).is_some_and(|w| w.private),
                "version": mizu_core::VERSION,
            })))
        }
        Command::ChromeMetrics { height, omnibox } => {
            if let Some(w) = app.window_mut(window).filter(|w| w.sidebar_key != key) {
                if (w.chrome_height - height).abs() > 0.5 {
                    w.chrome_height = height.clamp(30.0, 200.0);
                    w.layout();
                }
                w.omnibox_anchor = omnibox;
            }
            ok()
        }
        Command::Window { action } => {
            match action {
                WindowAction::Fullscreen => {
                    if let Some(w) = app.window_mut(window) {
                        let on = !w.fullscreen.browser;
                        w.set_fullscreen(Some(on), None);
                    }
                }
                other => {
                    if let Some(w) = app.window(window) {
                        crate::window::window_action(w.hwnd, other);
                    }
                }
            }
            ok()
        }
        Command::ShowFlyout { kind, mut anchor, tab_id } => {
            tracing::debug!(?kind, "show flyout");
            // The sidebar sits below the toolbar: convert its coordinates to the chrome's.
            if let Some(w) = app.window(window).filter(|w| w.sidebar_key == key) {
                anchor.y += w.chrome_height;
            }
            // Menus and panels toggle: a second click on their button closes them.
            if kind != FlyoutKind::Omnibox {
                if let Some(w) = app.window_mut(window) {
                    if w.flyout.kind == Some(kind) && w.flyout.tab_id == tab_id {
                        w.flyout.hide();
                        return ok();
                    }
                    if let Some((k, t, at)) = w.flyout.last_closed {
                        if k == kind && t == tab_id && at.elapsed() < std::time::Duration::from_millis(350) {
                            w.flyout.last_closed = None;
                            return ok();
                        }
                    }
                }
            }
            let data = flyout_data(app, window, kind, tab_id);
            app.show_flyout(window, kind, anchor, data, tab_id);
            ok()
        }
        Command::SidebarResize { width, commit } => {
            let width = width.clamp(160.0, 520.0).round();
            for w in &mut app.windows {
                if w.side_tabs {
                    w.sidebar_width = width;
                    w.layout();
                }
            }
            if commit {
                app.settings.appearance.sidebar_width = width as u32;
                app.persist_settings();
            }
            ok()
        }
        Command::SidebarToggle => {
            app.settings.appearance.sidebar_collapsed = !app.settings.appearance.sidebar_collapsed;
            app.persist_settings();
            app.apply_settings_to_surfaces();
            ok()
        }
        Command::CloseFlyout => {
            if app.pending_password.as_ref().is_some_and(|p| p.window == window) {
                app.pending_password = None;
            }
            let prompt = app.window_mut(window).and_then(|w| {
                let p = w.flyout.prompt_id.take();
                w.flyout.hide();
                p
            });
            if let Some(p) = prompt {
                app.answer_prompt(p, false, false);
            }
            if let Some(v) = app.window(window).and_then(|w| w.active_view()) {
                if surface == Surface::Flyout {
                    v.focus();
                }
            }
            ok()
        }
        Command::FlyoutSize { width, height } => {
            tracing::debug!(width, height, "flyout size");
            if let Some(w) = app.window_mut(window) {
                let hwnd = w.hwnd;
                w.flyout.resize(hwnd, width.clamp(80.0, 900.0), height.clamp(20.0, 900.0));
            }
            ok()
        }

        // ── Tabs ─────────────────────────────────────────────────
        Command::NewTab { url } => {
            let url = url
                .and_then(|u| crate::omnibox::resolve_input(&app.settings, &u))
                .unwrap_or_else(|| InternalPage::NewTab.url());
            app.open_tab(window, url, OpenOptions::foreground());
            focus_omnibox(app, window);
            ok()
        }
        Command::NewWindow { private } => {
            app.new_window(private, None);
            ok()
        }
        Command::CloseTab { id } => {
            app.close_tab(window, tab_id(id));
            ok()
        }
        Command::ActivateTab { id } => {
            app.activate_tab(window, tab_id(id));
            app.show_next_prompt(window);
            ok()
        }
        Command::MoveTab { id, index } => {
            if let Some(w) = app.window_mut(window) {
                w.strip.move_tab(tab_id(id), index);
                w.state_dirty = true;
            }
            app.session_dirty = true;
            ok()
        }
        Command::PinTab { id, pinned } => {
            if let Some(w) = app.window_mut(window) {
                w.strip.set_pinned(tab_id(id), pinned);
                w.state_dirty = true;
            }
            app.session_dirty = true;
            ok()
        }
        Command::DiscardTab { id, others } => {
            let targets: Vec<TabId> = match app.window(window) {
                Some(w) if others => w
                    .tabs
                    .iter()
                    .filter(|(t, s)| t.0 != id && !s.audible && s.view.is_some())
                    .map(|(t, _)| *t)
                    .collect(),
                Some(_) => vec![tab_id(id)],
                None => Vec::new(),
            };
            for t in targets {
                app.discard_tab(window, t);
            }
            ok()
        }
        Command::MuteTab { id, muted } => {
            if let Some(v) = app.window(window).and_then(|w| w.tabs.get(&tab_id(id))).and_then(|t| t.view.clone()) {
                v.set_muted(muted);
            }
            ok()
        }
        Command::DuplicateTab { id } => {
            let url = app.window(window).and_then(|w| w.strip.get(tab_id(id))).map(|t| t.url.clone());
            if let Some(url) = url {
                app.open_tab(window, url, OpenOptions { activate: true, popup: false, load_now: true, adjacent: true });
            }
            ok()
        }
        Command::ReloadTab { id } => {
            reload_tab(app, window, tab_id(id), false);
            ok()
        }
        Command::CloseOtherTabs { id } => {
            let others: Vec<TabId> = app
                .window(window)
                .map(|w| w.strip.tabs().iter().filter(|t| t.id.0 != id && !t.pinned).map(|t| t.id).collect())
                .unwrap_or_default();
            for t in others {
                app.close_tab(window, t);
            }
            ok()
        }
        Command::CloseTabsRight { id } => {
            let right: Vec<TabId> = app
                .window(window)
                .map(|w| {
                    let tabs = w.strip.tabs();
                    let pos = tabs.iter().position(|t| t.id.0 == id).unwrap_or(tabs.len());
                    tabs.iter().skip(pos + 1).map(|t| t.id).collect()
                })
                .unwrap_or_default();
            for t in right {
                app.close_tab(window, t);
            }
            ok()
        }
        Command::ReopenClosedTab => {
            app.reopen_closed_tab(window);
            ok()
        }

        // ── Navigation ───────────────────────────────────────────
        Command::Navigate { input } => {
            if let Some(url) = crate::omnibox::resolve_input(&app.settings, &input) {
                app.navigate_active(window, &url);
            }
            ok()
        }
        Command::OpenUrl { url, disposition } => {
            open_url(app, window, url, disposition);
            ok()
        }
        Command::Back => {
            with_active_view(app, window, |key, v| {
                hot::allow_internal(key);
                v.go_back();
            });
            ok()
        }
        Command::Forward => {
            with_active_view(app, window, |key, v| {
                hot::allow_internal(key);
                v.go_forward();
            });
            ok()
        }
        Command::Reload { bypass_cache } => {
            if let Some(t) = active_tab(app, window) {
                reload_tab(app, window, t, bypass_cache);
            }
            ok()
        }
        Command::Stop => {
            with_active_view(app, window, |_, v| v.stop());
            ok()
        }
        Command::Home => {
            let home = app.settings.general.home_page.clone();
            app.navigate_active(window, &home);
            ok()
        }
        Command::OmniboxInput { text } => {
            let private = app.window(window).is_some_and(|w| w.private);
            let items = crate::omnibox::suggest(&app.settings, app.storage_for(private), &text);
            let anchor = app.window(window).map(|w| w.omnibox_anchor).unwrap_or_default();
            let wants_remote = app.settings.search.remote_suggestions
                && !private
                && items.first().is_some_and(|s| s.kind == "search");
            let data = json!({ "items": items, "selected": 0, "width": anchor.width });
            if let Some(w) = app.window_mut(window) {
                w.suggestions = items;
                w.omnibox_query = text.clone();
                w.post_chrome(&Event::Suggestions { items: data["items"].clone() }.to_json());
            }
            if wants_remote {
                if let Some(template) = mizu_settings::suggest_template(&app.settings.search.active_engine().id) {
                    crate::omnibox::fetch_remote(window, text.trim(), template, app.worker.clone());
                }
            }
            if data["items"].as_array().is_some_and(|a| !a.is_empty()) {
                app.show_flyout(window, FlyoutKind::Omnibox, anchor, data, None);
            } else if let Some(w) = app.window_mut(window) {
                if w.flyout.is_open(FlyoutKind::Omnibox) {
                    w.flyout.hide();
                }
            }
            ok()
        }
        Command::OmniboxHighlight { index } => {
            if let Some(w) = app.window(window) {
                if w.flyout.is_open(FlyoutKind::Omnibox) {
                    if let Some(v) = &w.flyout.view {
                        let ev = Event::Flyout {
                            kind: FlyoutKind::Omnibox,
                            data: json!({ "items": w.suggestions, "selected": index, "width": w.omnibox_anchor.width }),
                        };
                        v.post_json(&ev.to_json());
                    }
                }
            }
            ok()
        }
        Command::OmniboxCommit { index } => {
            let url = app.window(window).and_then(|w| w.suggestions.get(index)).map(|s| s.url.clone());
            if let Some(url) = url {
                app.navigate_active(window, &url);
            }
            if let Some(w) = app.window_mut(window) {
                w.flyout.hide();
                w.suggestions.clear();
            }
            ok()
        }
        Command::OmniboxClose => {
            if let Some(w) = app.window_mut(window) {
                if w.flyout.is_open(FlyoutKind::Omnibox) {
                    w.flyout.hide();
                }
            }
            ok()
        }
        Command::Zoom { delta } => {
            zoom(app, window, delta);
            ok()
        }
        Command::Print => {
            with_active_view(app, window, |_, v| v.print());
            ok()
        }
        Command::Devtools => {
            if app.settings.advanced.developer_tools {
                with_active_view(app, window, |_, v| v.open_devtools());
            }
            ok()
        }
        Command::TaskManager => {
            with_active_view(app, window, |_, v| v.open_task_manager());
            ok()
        }
        Command::ToggleBookmark => {
            toggle_bookmark(app, window);
            ok()
        }

        // ── Prompts & panels ─────────────────────────────────────
        Command::PermissionResponse { prompt_id, allow, remember } => {
            app.answer_prompt(prompt_id, allow, remember);
            ok()
        }
        Command::ConfirmResponse { accepted, dont_ask_again } => {
            if let Some(w) = app.window_mut(window) {
                w.flyout.hide();
            }
            let kind = app.pending_confirm.take().filter(|(w, _)| *w == window).map(|(_, k)| k);
            if accepted {
                match kind.unwrap_or(crate::app::ConfirmKind::CloseWindow) {
                    crate::app::ConfirmKind::CloseWindow => {
                        if dont_ask_again {
                            app.settings.general.confirm_close_multiple_tabs = false;
                            app.persist_settings();
                        }
                        app.close_window(window);
                    }
                    crate::app::ConfirmKind::InstallUpdate => {
                        if let Err(e) = app.install_update() {
                            app.toast_all(&e);
                        }
                    }
                    crate::app::ConfirmKind::DownloadUpdate => {
                        app.open_tab(window, crate::update::RELEASES_URL.to_owned(), crate::tabs::OpenOptions::foreground());
                    }
                }
            }
            ok()
        }
        Command::ShieldToggleSite { enabled } => {
            let host = app.current_url(window).and_then(|u| mizu_core::omnibox::host_of(&u)).map(|h| crate::shield::normalize_host(&h));
            if let Some(host) = host {
                app.settings.privacy.allowlist.retain(|h| crate::shield::normalize_host(h) != host);
                if !enabled {
                    app.settings.privacy.allowlist.push(host);
                }
                app.persist_settings();
                app.apply_settings_to_surfaces();
                if let Some(t) = active_tab(app, window) {
                    reload_tab(app, window, t, false);
                }
            }
            ok()
        }
        Command::DownloadAction { id, action } => {
            crate::internal::download_action(app, window, id, action);
            ok()
        }
        Command::DownloadsClear => {
            let storage = Some(&app.storage);
            app.downloads.clear_finished(storage);
            ok()
        }
        other => crate::internal::handle_data(app, window, key, other),
    }
}

fn flyout_data(app: &App, window: WindowId, kind: FlyoutKind, tab_id: Option<u64>) -> Value {
    let private = app.window(window).is_some_and(|w| w.private);
    match kind {
        FlyoutKind::Downloads => app.downloads.to_json(private),
        FlyoutKind::TabMenu => {
            let tab = tab_id.and_then(|id| app.window(window)?.strip.get(TabId(id)).cloned());
            let state = tab_id.and_then(|id| app.window(window)?.tabs.get(&TabId(id)));
            let count = app.window(window).map(|w| w.strip.len()).unwrap_or(0);
            json!({
                "id": tab_id,
                "pinned": tab.as_ref().is_some_and(|t| t.pinned),
                "muted": state.is_some_and(|s| s.muted),
                "count": count,
                "closed": app.window(window).map(|w| w.strip.closed_tabs().count()).unwrap_or(0),
                "active": app.window(window).and_then(|w| w.strip.active()).map(|t| t.0) == tab_id,
                "unloaded": state.is_none_or(|s| s.view.is_none()),
                "others": app.window(window).map_or(0, |w| w.tabs.iter().filter(|(t, s)| Some(t.0) != tab_id && s.view.is_some() && !s.audible).count()),
            })
        }
        FlyoutKind::BookmarkMenu => {
            let b = tab_id.and_then(|id| app.storage.bookmark(id as i64).ok());
            match b {
                Some(b) => json!({
                    "id": b.id,
                    "title": b.title,
                    "url": b.url,
                    "folder": b.kind == mizu_storage::BookmarkKind::Folder,
                    "private": private,
                }),
                None => Value::Null,
            }
        }
        FlyoutKind::Shield | FlyoutKind::SiteInfo => {
            let url = app.current_url(window).unwrap_or_default();
            let key = active_tab(app, window).map(|t| t.0).unwrap_or_default();
            let (blocked, top, allowlisted) = hot::with(|h| {
                (
                    h.page_blocked.get(&key).copied().unwrap_or(0),
                    h.stats.top_trackers(5),
                    crate::shield::allowlisted(&h.allowlist, &url),
                )
            })
            .unwrap_or_default();
            let origin = mizu_core::omnibox::origin_of(&url).unwrap_or_default();
            let permissions: Vec<Value> = if private {
                Vec::new()
            } else {
                app.storage
                    .all_permissions()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|p| p.origin == origin)
                    .map(|p| json!({ "kind": p.kind, "state": p.state }))
                    .collect()
            };
            json!({
                "url": url,
                "host": mizu_core::omnibox::host_of(&url).unwrap_or_default(),
                "origin": origin,
                "secure": url.starts_with("https://"),
                "blocked": blocked,
                "enabled": app.settings.privacy.content_blocking && !allowlisted,
                "global": app.settings.privacy.content_blocking,
                "tracking_prevention": app.settings.privacy.tracking_prevention,
                "top": top,
                "permissions": permissions,
                "private": private,
            })
        }
        FlyoutKind::Bookmark => {
            let url = app.current_url(window).unwrap_or_default();
            let bm = app.storage.find_bookmark_by_url(&url).ok().flatten();
            let folders: Vec<Value> = app
                .storage
                .all_bookmarks()
                .unwrap_or_default()
                .into_iter()
                .filter(|b| b.kind == mizu_storage::BookmarkKind::Folder)
                .map(|b| json!({ "id": b.id, "title": b.title, "parent": b.parent_id }))
                .collect();
            json!({ "bookmark": bm, "folders": folders })
        }
        FlyoutKind::Menu => {
            let zoom = app.window(window).and_then(|w| w.active_tab().and_then(|t| w.tabs.get(&t))).map(|t| t.zoom).unwrap_or(1.0);
            json!({
                "private": private,
                "zoom": (zoom * 100.0).round(),
                "fullscreen": app.window(window).is_some_and(|w| w.fullscreen.browser),
                "devtools": app.settings.advanced.developer_tools,
                "bookmarks_bar": app.settings.appearance.show_bookmarks_bar,
            })
        }
        FlyoutKind::Logins => crate::passwords::logins_flyout_data(app, window),
        FlyoutKind::Omnibox | FlyoutKind::Permission | FlyoutKind::Confirm | FlyoutKind::SavePassword => Value::Null,
    }
}

fn with_active_view(app: &App, window: WindowId, f: impl FnOnce(ViewKey, &mizu_engine::View)) {
    if let Some(w) = app.window(window) {
        if let Some(t) = w.active_tab() {
            if let Some(v) = w.tabs.get(&t).and_then(|s| s.view.as_ref()) {
                f(t.0, v);
            }
        }
    }
}

fn reload_tab(app: &mut App, window: WindowId, tab: TabId, bypass: bool) {
    let Some(w) = app.window_mut(window) else { return };
    let url = w.strip.get(tab).map(|t| t.url.clone()).unwrap_or_default();
    let Some(state) = w.tabs.get_mut(&tab) else { return };
    match &state.view {
        Some(v) if state.crashed => {
            state.crashed = false;
            crate::tabs::load(tab.0, v, &url);
        }
        Some(v) => {
            if is_internal_url(&url) {
                hot::allow_internal(tab.0);
            }
            v.reload(bypass);
        }
        None => {
            state.pending_url = Some(url);
            app.ensure_view(window, tab);
        }
    }
}

fn zoom(app: &mut App, window: WindowId, delta: i32) {
    const STEPS: [f64; 13] = [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];
    with_active_view(app, window, |_, v| {
        let current = v.zoom();
        let next = match delta.signum() {
            0 => 1.0,
            1 => STEPS.iter().copied().find(|s| *s > current + 0.001).unwrap_or(3.0),
            _ => STEPS.iter().rev().copied().find(|s| *s < current - 0.001).unwrap_or(0.25),
        };
        v.set_zoom(next);
    });
}

fn toggle_bookmark(app: &mut App, window: WindowId) {
    let Some(w) = app.window(window) else { return };
    if w.private {
        return;
    }
    let Some(tab) = w.active_tab() else { return };
    let Some(t) = w.strip.get(tab) else { return };
    let (url, title) = (t.url.clone(), t.title.clone());
    if url.is_empty() || url.starts_with("mizu://newtab") {
        return;
    }
    let existing = app.storage.find_bookmark_by_url(&url).ok().flatten();
    if existing.is_none() {
        let title = if title.is_empty() { url.clone() } else { title };
        let _ = app.storage.add_bookmark(mizu_storage::BOOKMARKS_BAR_ID, &title, &url);
    }
    app.bookmarks_bar = None;
    if let Some(w) = app.window_mut(window) {
        w.state_dirty = true;
    }
    for w in &mut app.windows {
        w.state_dirty = true;
    }
}

pub fn open_url(app: &mut App, window: WindowId, url: String, disposition: Disposition) {
    let Some(url) = crate::omnibox::resolve_input(&app.settings, &url) else { return };
    match disposition {
        Disposition::Current => app.navigate_active(window, &url),
        Disposition::NewTab => {
            app.open_tab(window, url, OpenOptions { activate: true, popup: false, load_now: true, adjacent: true });
        }
        Disposition::Background => {
            app.open_tab(window, url, OpenOptions::background());
        }
        Disposition::NewWindow => {
            app.new_window(false, Some(vec![url]));
        }
        Disposition::PrivateWindow => {
            app.new_window(true, Some(vec![url]));
        }
    }
}

fn focus_omnibox(app: &App, window: WindowId) {
    if let Some(w) = app.window(window) {
        if let Some(c) = &w.chrome {
            c.focus();
        }
        w.post_chrome(&Event::FocusOmnibox.to_json());
    }
}

fn open_internal(app: &mut App, window: WindowId, page: InternalPage) {
    // Reuse an existing tab showing the page.
    let existing = app.window(window).and_then(|w| {
        w.strip.tabs().iter().find(|t| InternalPage::from_url(&t.url) == Some(page)).map(|t| t.id)
    });
    match existing {
        Some(t) => app.activate_tab(window, t),
        None => {
            let reuse_newtab = app.current_url(window).is_some_and(|u| u.starts_with("mizu://newtab"));
            if reuse_newtab {
                app.navigate_active(window, &page.url());
            } else {
                app.open_tab(window, page.url(), OpenOptions { activate: true, popup: false, load_now: true, adjacent: true });
            }
        }
    }
}

pub fn run_shortcut(app: &mut App, window: WindowId, s: Shortcut) {
    match s {
        Shortcut::NewTab => {
            app.open_tab(window, InternalPage::NewTab.url(), OpenOptions::foreground());
            focus_omnibox(app, window);
        }
        Shortcut::NewWindow => {
            app.new_window(false, None);
        }
        Shortcut::NewPrivateWindow => {
            app.new_window(true, None);
        }
        Shortcut::CloseTab => {
            if let Some(t) = active_tab(app, window) {
                app.close_tab(window, t);
            }
        }
        Shortcut::ReopenClosedTab => app.reopen_closed_tab(window),
        Shortcut::NextTab | Shortcut::PrevTab => {
            let next = app.window(window).and_then(|w| w.strip.cycle(s == Shortcut::NextTab));
            if let Some(t) = next {
                app.activate_tab(window, t);
            }
        }
        Shortcut::SelectTab(n) => {
            let t = app.window(window).and_then(|w| w.strip.by_shortcut(n));
            if let Some(t) = t {
                app.activate_tab(window, t);
            }
        }
        Shortcut::FocusOmnibox => focus_omnibox(app, window),
        Shortcut::Reload | Shortcut::HardReload => {
            if let Some(t) = active_tab(app, window) {
                reload_tab(app, window, t, s == Shortcut::HardReload);
            }
        }
        Shortcut::Back => with_active_view(app, window, |key, v| {
            hot::allow_internal(key);
            v.go_back();
        }),
        Shortcut::Forward => with_active_view(app, window, |key, v| {
            hot::allow_internal(key);
            v.go_forward();
        }),
        Shortcut::Home => {
            let home = app.settings.general.home_page.clone();
            app.navigate_active(window, &home);
        }
        Shortcut::BookmarkPage => toggle_bookmark(app, window),
        Shortcut::ToggleBookmarksBar => {
            app.settings.appearance.show_bookmarks_bar = !app.settings.appearance.show_bookmarks_bar;
            app.persist_settings();
            app.apply_settings_to_surfaces();
        }
        Shortcut::History => open_internal(app, window, InternalPage::History),
        Shortcut::Downloads => open_internal(app, window, InternalPage::Downloads),
        Shortcut::Bookmarks => open_internal(app, window, InternalPage::Bookmarks),
        Shortcut::ClearData => {
            let url = "mizu://settings/#privacy".to_owned();
            app.open_tab(window, url, OpenOptions::foreground());
        }
        Shortcut::Fullscreen => {
            if let Some(w) = app.window_mut(window) {
                if w.fullscreen.element {
                    w.set_fullscreen(Some(false), Some(false));
                } else {
                    let on = !w.fullscreen.browser;
                    w.set_fullscreen(Some(on), None);
                }
            }
        }
        Shortcut::Escape => {
            if let Some(w) = app.window_mut(window) {
                w.flyout.hide();
            }
            queue::with_app(move |a| {
                if let Some(v) = a.window(window).and_then(|w| w.active_view()) {
                    v.focus();
                }
            });
        }
        Shortcut::ZoomIn => zoom(app, window, 1),
        Shortcut::ZoomOut => zoom(app, window, -1),
        Shortcut::ZoomReset => zoom(app, window, 0),
    }
}
