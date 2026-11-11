//! Data commands used by internal pages (settings, history, bookmarks, downloads, Shield, …).

use crate::app::App;
use crate::commands::reply;
use crate::hot::{self, Role};
use crate::queue::{self, WorkerResult};
use mizu_core::ids::{DownloadId, WindowId};
use mizu_core::internal::InternalPage;
use mizu_engine::{BrowsingDataKinds, ViewKey};
use mizu_settings::{FilterListSubscription, Settings};
use mizu_storage::{DownloadState, PermissionState, TimeRange};
use mizu_ui::protocol::{Command, DownloadAction};
use serde_json::{Value, json};

type CmdResult = Result<Option<Value>, String>;

fn ok() -> CmdResult {
    Ok(Some(Value::Null))
}

fn data(v: impl serde::Serialize) -> CmdResult {
    Ok(Some(serde_json::to_value(v).map_err(|e| e.to_string())?))
}

fn storage_err(e: mizu_storage::StorageError) -> String {
    e.to_string()
}

/// Sends `json` to every tab showing the internal page `host` (or all internal pages if empty).
pub fn broadcast_to_pages(app: &App, host: &str, json: &str) {
    for w in &app.windows {
        for t in w.strip.tabs() {
            let matches = match InternalPage::from_url(&t.url) {
                Some(p) => host.is_empty() || p.host() == host,
                None => false,
            };
            if matches {
                if let Some(v) = w.tabs.get(&t.id).and_then(|s| s.view.as_ref()) {
                    v.post_json(json);
                }
            }
        }
    }
}

fn is_private(app: &App, window: WindowId) -> bool {
    app.window(window).is_some_and(|w| w.private)
}

fn range(s: &str) -> TimeRange {
    match s {
        "hour" => TimeRange::LastHour,
        "day" => TimeRange::LastDay,
        "week" => TimeRange::LastWeek,
        "month" => TimeRange::LastMonth,
        _ => TimeRange::All,
    }
}

impl App {
    pub fn persist_settings(&mut self) {
        self.settings.sanitize();
        if let Err(e) = mizu_settings::save(&self.paths.settings(), &self.settings) {
            tracing::warn!(error = %e, "could not save settings");
        }
    }

    fn settings_payload(&self) -> Value {
        let store = mizu_privacy::FilterStore::open(&self.paths.filters());
        json!({
            "settings": crate::theme::ui_settings(&self.settings),
            "engines": self.settings.search.all_engines(),
            "lists": store.status(&self.settings.privacy.filter_lists),
            "filters_updating": self.filters_updating,
            "download_dir_default": crate::downloads::default_dir().to_string_lossy(),
            "engine_version": mizu_engine::browser_version().unwrap_or_default(),
        })
    }
}

#[allow(clippy::too_many_lines)]
pub fn handle_data(app: &mut App, window: WindowId, key: ViewKey, cmd: Command) -> CmdResult {
    let private = is_private(app, window);
    match cmd {
        // ── Settings ─────────────────────────────────────────────
        Command::GetSettings => Ok(Some(app.settings_payload())),
        Command::SetSettings { settings } => {
            let mut new: Settings = serde_json::from_value(settings).map_err(|e| format!("invalid settings: {e}"))?;
            new.sanitize();
            let old = std::mem::replace(&mut app.settings, new);
            let lists_changed = old.privacy.filter_lists != app.settings.privacy.filter_lists
                || old.privacy.custom_rules != app.settings.privacy.custom_rules;
            let restart_needed = old.performance.hardware_acceleration != app.settings.performance.hardware_acceleration
                || old.media.autoplay != app.settings.media.autoplay
                || old.advanced.extensions != app.settings.advanced.extensions
                || old.general.language != app.settings.general.language;
            app.persist_settings();
            app.apply_settings_to_surfaces();
            if lists_changed {
                app.filters_updating = true;
                crate::shield::rebuild(&app.paths, &app.settings.privacy, app.worker.clone(), true, false);
            }
            let mut payload = app.settings_payload();
            payload["restart_needed"] = Value::Bool(restart_needed);
            Ok(Some(payload))
        }
        Command::ResetSettings => {
            app.settings = Settings::default();
            app.persist_settings();
            app.apply_settings_to_surfaces();
            crate::shield::rebuild(&app.paths, &app.settings.privacy, app.worker.clone(), true, false);
            Ok(Some(app.settings_payload()))
        }
        Command::ChooseDownloadFolder => {
            let owner = app.window(window).map(|w| w.hwnd).unwrap_or_default();
            let start = app.settings.downloads.directory.clone();
            let rid = current_rid();
            queue::outside(move || {
                let picked = crate::dialogs::pick_folder(owner, "Choose download folder", Some(&start));
                queue::with_app(move |a| {
                    if let Some(p) = &picked {
                        a.settings.downloads.directory = p.clone();
                        a.persist_settings();
                    }
                    if let Some(rid) = rid {
                        reply(key, rid, json!({ "path": picked }));
                    }
                });
            });
            Ok(None)
        }

        // ── History ──────────────────────────────────────────────
        Command::HistoryQuery { query, offset } => {
            if private {
                return data(Vec::<()>::new());
            }
            data(app.storage.history_page(&query, 100, offset).map_err(storage_err)?)
        }
        Command::HistoryDelete { id } => {
            app.storage.delete_history_entry(id).map_err(storage_err)?;
            ok()
        }
        Command::HistoryClear { range: r } => {
            let n = app.storage.clear_history(range(&r)).map_err(storage_err)?;
            data(json!({ "removed": n }))
        }

        // ── Bookmarks ────────────────────────────────────────────
        Command::BookmarksGet => data(app.storage.all_bookmarks().map_err(storage_err)?),
        Command::BookmarkAdd { parent, title, url } => {
            let id = app.storage.add_bookmark(parent, &title, &url).map_err(storage_err)?;
            mark_all_dirty(app);
            data(json!({ "id": id }))
        }
        Command::BookmarkAddFolder { parent, title } => {
            let id = app.storage.add_bookmark_folder(parent, &title).map_err(storage_err)?;
            mark_all_dirty(app);
            data(json!({ "id": id }))
        }
        Command::BookmarkUpdate { id, title, url } => {
            app.storage.update_bookmark(id, &title, url.as_deref()).map_err(storage_err)?;
            mark_all_dirty(app);
            ok()
        }
        Command::BookmarkMove { id, parent, index } => {
            app.storage.move_bookmark(id, parent, index).map_err(storage_err)?;
            mark_all_dirty(app);
            ok()
        }
        Command::BookmarkDelete { id } => {
            app.storage.delete_bookmark(id).map_err(storage_err)?;
            mark_all_dirty(app);
            ok()
        }

        // ── Downloads ────────────────────────────────────────────
        Command::DownloadsGet => Ok(Some(app.downloads.to_json(private))),

        // ── Data clearing & permissions ──────────────────────────
        Command::ClearData(req) => {
            let r = range(&req.range);
            if req.history {
                app.storage.clear_history(r).map_err(storage_err)?;
            }
            if req.downloads {
                app.downloads.clear_finished(Some(&app.storage));
            }
            if req.permissions {
                app.storage.clear_permissions().map_err(storage_err)?;
                app.session_permissions.clear();
            }
            let kinds = BrowsingDataKinds {
                cookies: req.cookies,
                cache: req.cache,
                site_storage: req.site_storage,
                autofill: req.autofill,
                passwords: req.passwords,
            };
            let since = (r != TimeRange::All).then(|| r.since(mizu_storage::now_ms()));
            let view = app
                .windows
                .iter()
                .filter(|w| !w.private)
                .flat_map(|w| w.tabs.values())
                .find_map(|t| t.view.clone());
            let rid = current_rid();
            match view {
                Some(v) if !kinds.is_empty() => {
                    v.clear_browsing_data(kinds, since, move |success| {
                        if let Some(rid) = rid {
                            reply(key, rid, json!({ "engine": success }));
                        }
                    });
                    Ok(None)
                }
                _ => data(json!({ "engine": kinds.is_empty() })),
            }
        }
        Command::PermissionsGet => data(app.storage.all_permissions().map_err(storage_err)?),
        Command::PermissionSet { origin, kind, state } => {
            match state.as_str() {
                "allow" => app.storage.set_permission(&origin, &kind, PermissionState::Allow),
                "block" => app.storage.set_permission(&origin, &kind, PermissionState::Block),
                _ => app.storage.reset_permission(&origin, &kind),
            }
            .map_err(storage_err)?;
            ok()
        }

        // ── Shield ───────────────────────────────────────────────
        Command::FiltersUpdate { force } => {
            app.filters_updating = true;
            crate::shield::rebuild(&app.paths, &app.settings.privacy, app.worker.clone(), true, force);
            ok()
        }
        Command::FilterListAdd { name, url } => {
            if !url.starts_with("https://") {
                return Err("Filter lists must use an https:// address".into());
            }
            let id = format!("custom-{:x}", mizu_storage::now_ms());
            app.settings.privacy.filter_lists.push(FilterListSubscription {
                id,
                name: if name.trim().is_empty() { url.clone() } else { name },
                url,
                enabled: true,
                builtin: false,
            });
            app.persist_settings();
            app.filters_updating = true;
            crate::shield::rebuild(&app.paths, &app.settings.privacy, app.worker.clone(), true, false);
            Ok(Some(app.settings_payload()))
        }
        Command::FilterListRemove { id } => {
            app.settings.privacy.filter_lists.retain(|l| l.builtin || l.id != id);
            mizu_privacy::FilterStore::open(&app.paths.filters()).remove_list(&id);
            app.persist_settings();
            crate::shield::rebuild(&app.paths, &app.settings.privacy, app.worker.clone(), false, false);
            Ok(Some(app.settings_payload()))
        }
        Command::ShieldStats => {
            let (stats, rules) = hot::with(|h| {
                (h.stats.clone(), h.blocker.as_ref().map(|b| b.rule_count()).unwrap_or(0))
            })
            .unwrap_or_default();
            data(json!({
                "session": stats.session_blocked,
                "lifetime": stats.lifetime_blocked,
                "top": stats.top_trackers(15),
                "rules": rules,
                "enabled": app.settings.privacy.content_blocking,
                "tracking_prevention": app.settings.privacy.tracking_prevention,
                "allowlist": app.settings.privacy.allowlist,
            }))
        }
        Command::BlockedProceed { url } => {
            if let Some(host) = mizu_core::omnibox::host_of(&url) {
                hot::with(|h| h.document_exceptions.insert(host.trim_start_matches("www.").to_ascii_lowercase()));
            }
            if url.starts_with("http://") || url.starts_with("https://") {
                if let Some(Role::Tab { window, .. }) = hot::role(key) {
                    app.navigate_active(window, &url);
                }
            }
            ok()
        }

        // ── Misc pages ───────────────────────────────────────────
        Command::MediaProbe => {
            let token = mizu_core::ids::PromptId::next().0;
            if let Some(rid) = current_rid() {
                app.media_waiters.insert(token, (key, rid));
            }
            let sender = app.worker.clone();
            std::thread::spawn(move || {
                let report = mizu_media::probe_system();
                sender.send(WorkerResult::MediaReport(Box::new(report), token));
            });
            Ok(None)
        }
        Command::About => data(json!({
            "name": mizu_core::APP_NAME,
            "vendor": mizu_core::VENDOR,
            "version": mizu_core::VERSION,
            "tagline": mizu_core::TAGLINE,
            "engine": "Microsoft Edge WebView2 (Chromium)",
            "engine_version": mizu_engine::browser_version().unwrap_or_default(),
            "os": os_version(),
            "profile": app.paths.root.to_string_lossy().replace('/', "\\"),
            "user_agent_note": "Mizu uses the engine's standard user agent for compatibility.",
            "update": app.update,
            "installed": crate::update::is_installed_copy(),
            "auto_update": app.settings.general.auto_update,
        })),
        Command::NewtabData => {
            // Speed dial = bookmarks on the bookmarks bar (folders skipped).
            let top: Vec<Value> = app
                .storage
                .bookmark_children(mizu_storage::BOOKMARKS_BAR_ID)
                .unwrap_or_default()
                .into_iter()
                .filter(|b| b.kind == mizu_storage::BookmarkKind::Url)
                .take(24)
                .map(|b| {
                    let url = b.url.unwrap_or_default();
                    json!({ "id": b.id, "url": url, "title": b.title, "host": mizu_core::omnibox::host_of(&url) })
                })
                .collect();
            let engines: Vec<Value> = app
                .settings
                .search
                .all_engines()
                .into_iter()
                .map(|e| json!({ "id": e.id, "name": e.name, "template": e.template }))
                .collect();
            let (session, lifetime) = hot::with(|h| (h.stats.session_blocked, h.stats.lifetime_blocked)).unwrap_or_default();
            data(json!({
                "top": top,
                "private": private,
                "crashed": app.crashed_last_time && app.previous_session.is_some(),
                "restorable": app.previous_session.as_ref().map(|s| s.tab_count()).unwrap_or(0),
                "blocked_session": session,
                "blocked_lifetime": lifetime,
                "engine": app.settings.search.active_engine().name,
                "engine_id": app.settings.search.active_engine().id,
                "engines": engines,
            }))
        }
        Command::UpdateCheck => {
            app.check_for_update(current_rid().map(|r| (key, r)));
            Ok(None)
        }
        Command::UpdateInstall => {
            app.install_update()?;
            ok()
        }
        Command::RestoreSession => {
            app.restore_previous_session();
            ok()
        }

        // ── Userscripts ──────────────────────────────────────────
        Command::ScriptsGet => {
            let scripts = hot::with(|h| serde_json::to_value(h.scripts.scripts()).unwrap_or_default()).unwrap_or_default();
            data(scripts)
        }
        Command::ScriptPending => {
            if app.pending_install.is_empty() && app.pending_failures.is_empty() {
                return Ok(Some(Value::Null));
            }
            let items: Vec<Value> = app
                .pending_install
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let installed = hot::with(|h| h.scripts.installed_version(&p.meta).map(str::to_owned)).flatten();
                    json!({
                        "index": i,
                        "url": p.url,
                        "meta": p.meta,
                        "source": p.source,
                        "requires_bytes": p.requires.len(),
                        "installed_version": installed,
                    })
                })
                .collect();
            let failures: Vec<Value> =
                app.pending_failures.iter().map(|(u, e)| json!({ "url": u, "error": e })).collect();
            data(json!({ "items": items, "failures": failures }))
        }
        Command::ScriptInstallConfirm { indices } => {
            let pending = std::mem::take(&mut app.pending_install);
            app.pending_failures.clear();
            let mut installed = Vec::new();
            for (i, p) in pending.into_iter().enumerate() {
                if indices.as_ref().is_some_and(|ix| !ix.contains(&i)) {
                    continue;
                }
                let r = hot::with(|h| h.scripts.install(&p.source, &p.requires, &p.url)).ok_or("unavailable")?;
                match r {
                    Ok(_) => {
                        tracing::info!(name = p.meta.name, "userscript installed");
                        installed.push(p.meta.name);
                    }
                    Err(e) => return Err(format!("{}: {e}", p.meta.name)),
                }
            }
            data(json!({ "installed": installed }))
        }
        Command::ScriptInstallCancel => {
            app.pending_install.clear();
            app.pending_failures.clear();
            ok()
        }
        Command::ScriptInstallUrls { urls } => {
            let urls: Vec<String> = urls.into_iter().map(|u| u.trim().to_owned()).filter(|u| !u.is_empty()).collect();
            if urls.is_empty() {
                return Err("Paste at least one link".into());
            }
            app.begin_script_install_many(Some(key), urls, false);
            ok()
        }
        Command::ScriptInstallFiles => {
            let owner = app.window(window).map(|w| w.hwnd).unwrap_or_default();
            let sender = app.worker.clone();
            queue::outside(move || {
                let files = crate::dialogs::pick_files(owner, "Install userscripts", "Userscripts", "*.user.js;*.js");
                if !files.is_empty() {
                    crate::scripts::read_files_for_install(Some(key), files, sender);
                }
            });
            ok()
        }
        Command::ScriptsCheckUpdates => {
            let rid = current_rid();
            app.check_script_updates(true, rid.map(|r| (key, r)));
            Ok(None)
        }
        Command::ScriptCreate { source } => {
            let id = hot::with(|h| h.scripts.install(&source, "", ""))
                .ok_or("unavailable")?
                .map_err(|e| e.to_string())?;
            data(json!({ "id": id }))
        }
        Command::ScriptToggle { id, enabled } => {
            hot::with(|h| h.scripts.set_enabled(&id, enabled)).ok_or("unavailable")?.map_err(|e| e.to_string())?;
            ok()
        }
        Command::ScriptSites { id, only, never } => {
            let rules = mizu_userscripts::SiteRules { only, never };
            hot::with(|h| h.scripts.set_sites(&id, rules)).ok_or("unavailable")?.map_err(|e| e.to_string())?;
            let sites = hot::with(|h| h.scripts.get(&id).map(|s| s.sites.clone())).flatten().unwrap_or_default();
            data(json!({ "sites": sites }))
        }
        Command::ScriptRemove { id } => {
            hot::with(|h| h.scripts.remove(&id)).ok_or("unavailable")?.map_err(|e| e.to_string())?;
            ok()
        }
        Command::ScriptSource { id } => {
            let src = hot::with(|h| h.scripts.get(&id).map(|s| s.source.clone())).flatten().ok_or("unknown script")?;
            data(json!({ "source": src }))
        }

        // ── Extensions ───────────────────────────────────────────
        cmd @ (Command::PasswordsGet { .. }
        | Command::PasswordReveal { .. }
        | Command::PasswordSave { .. }
        | Command::PasswordDelete { .. }
        | Command::PasswordNeverRemove { .. }
        | Command::PasswordsImport
        | Command::PasswordsExport
        | Command::PasswordFill { .. }
        | Command::PasswordPromptResponse { .. }) => crate::passwords::handle(app, window, key, current_rid(), cmd),
        Command::ExtensionsGet => crate::extensions::list(app, key, current_rid()),
        Command::ExtensionInstall => crate::extensions::install(app, window, key, current_rid()),
        Command::ExtensionInstallFromStore { url } => crate::extensions::install_from_store(app, key, current_rid(), &url),
        Command::ExtensionToggle { id, enabled } => crate::extensions::toggle(app, key, current_rid(), id, enabled),
        Command::ExtensionRemove { id } => crate::extensions::remove(app, key, current_rid(), id),
        Command::ExtensionOpen { id } => crate::extensions::open(app, window, id),

        other => {
            tracing::warn!(?other, "unhandled command");
            Err("unsupported".into())
        }
    }
}

fn mark_all_dirty(app: &mut App) {
    app.bookmarks_bar = None;
    for w in &mut app.windows {
        w.state_dirty = true;
    }
}

pub fn download_action(app: &mut App, window: WindowId, id: u64, action: DownloadAction) {
    let id = DownloadId(id);
    let Some(item) = app.downloads.get(id) else { return };
    let path = item.path.clone();
    let url = item.url.clone();
    let op = item.op.clone();
    let state = item.state;
    let private = item.private;
    match action {
        DownloadAction::Pause => {
            if let Some(op) = op {
                op.pause();
            }
        }
        DownloadAction::Resume => {
            if let Some(op) = op {
                op.resume();
            }
        }
        DownloadAction::Cancel => {
            if let Some(op) = op {
                op.cancel();
            }
        }
        DownloadAction::Retry => {
            if state.is_active() {
                return;
            }
            let storage = if private { None } else { Some(&app.storage) };
            app.downloads.remove(id, storage);
            if let Some(tab) = app.open_tab(window, url, crate::tabs::OpenOptions::background()) {
                if let Some(t) = app.window_mut(window).and_then(|w| w.tabs.get_mut(&tab)) {
                    t.close_if_download = true;
                }
            }
        }
        DownloadAction::Open => {
            if state == DownloadState::Completed {
                crate::win::shell_open(&path);
            }
        }
        DownloadAction::Show => crate::win::show_in_folder(&path),
        DownloadAction::Remove => {
            let storage = if private { None } else { Some(&app.storage) };
            app.downloads.remove(id, storage);
        }
    }
}

thread_local! {
    static CURRENT_RID: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Request id of the message being handled (for commands that reply asynchronously).
pub fn current_rid() -> Option<u64> {
    CURRENT_RID.with(|c| c.get())
}

pub fn set_current_rid(rid: Option<u64>) {
    CURRENT_RID.with(|c| c.set(rid));
}

fn os_version() -> String {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let read = |name: &str| -> String {
        let mut buf = [0u16; 128];
        let mut size = (buf.len() * 2) as u32;
        // SAFETY: buffer and size are valid; the value is a REG_SZ.
        let r = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                &crate::win::wide(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
                &crate::win::wide(name),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if r.is_err() {
            return String::new();
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    };
    format!("Windows {} (build {})", read("DisplayVersion"), read("CurrentBuildNumber"))
}
