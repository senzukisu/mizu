//! State needed synchronously inside engine callbacks (navigation and request decisions).
//!
//! It lives apart from [`crate::app::App`] so decisions never wait for, or re-enter, the main
//! browser state. Borrows are always short and never span a call into the engine.

use mizu_core::ids::{TabId, WindowId};
use mizu_engine::{View, ViewKey};
use mizu_privacy::{BlockStats, ContentBlocker};
use mizu_userscripts::ScriptStore;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Tab { window: WindowId, tab: TabId },
    Chrome(WindowId),
    Flyout(WindowId),
}

pub struct Hot {
    pub roles: HashMap<ViewKey, Role>,
    pub views: HashMap<ViewKey, View>,
    pub private: HashSet<ViewKey>,
    /// Top-level URL each tab is showing or loading; the "source" for blocking decisions.
    pub top_urls: HashMap<ViewKey, String>,
    /// Tabs the browser itself is navigating to an internal page.
    pub internal_allowed: HashSet<ViewKey>,

    pub blocker: Option<Arc<ContentBlocker>>,
    pub content_blocking: bool,
    /// Hosts where Mizu Shield is switched off by the user.
    pub allowlist: HashSet<String>,
    pub global_privacy_control: bool,
    /// Theme attributes for internal pages' `<html>` element (see `theme::html_attrs`).
    pub theme_attrs: String,
    /// Hosts the user chose to open despite a document-level block (this session only).
    pub document_exceptions: HashSet<String>,
    /// Requests blocked on the current page of each tab.
    pub page_blocked: HashMap<ViewKey, u32>,
    pub stats: BlockStats,
    pub stats_dirty: bool,

    pub scripts: ScriptStore,
    /// Per tab: the token and script ids of the current userscript injection.
    pub gm_tokens: HashMap<ViewKey, (String, Vec<String>)>,
    /// When the current top-level navigation of each tab started (load-time logging).
    pub nav_started: HashMap<ViewKey, std::time::Instant>,
    /// Per tab: token of the login-capture script on the current page.
    pub pw_tokens: HashMap<ViewKey, String>,
    /// Offer to save passwords (Mizu Passwords enabled).
    pub password_capture: bool,
    /// Picture-in-picture button on videos.
    pub pip_button: bool,
    /// Keyboard shortcuts in effect (defaults plus the user's changes).
    pub bindings: crate::shortcuts::Bindings,
    /// Views recording a new shortcut in Settings: every key goes to the page.
    pub capturing_keys: std::collections::HashSet<ViewKey>,
    /// JSON configuration for the picture-in-picture script.
    pub pip_config: String,
}

thread_local! {
    static HOT: RefCell<Option<Hot>> = const { RefCell::new(None) };
}

pub fn install(hot: Hot) {
    HOT.with(|h| *h.borrow_mut() = Some(hot));
}

/// Runs `f` with the hot state. Returns `None` before initialisation (or on re-entrancy, which
/// the design prevents but we never want to panic on).
pub fn with<R>(f: impl FnOnce(&mut Hot) -> R) -> Option<R> {
    HOT.with(|h| {
        let mut guard = h.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

pub fn view(key: ViewKey) -> Option<View> {
    with(|h| h.views.get(&key).cloned()).flatten()
}

pub fn role(key: ViewKey) -> Option<Role> {
    with(|h| h.roles.get(&key).copied()).flatten()
}

pub fn register(key: ViewKey, role: Role, private: bool) {
    with(|h| {
        h.roles.insert(key, role);
        if private {
            h.private.insert(key);
        }
    });
}

pub fn attach_view(key: ViewKey, view: View) {
    with(|h| {
        h.views.insert(key, view);
    });
}

pub fn forget(key: ViewKey) {
    with(|h| {
        h.roles.remove(&key);
        h.views.remove(&key);
        h.private.remove(&key);
        h.top_urls.remove(&key);
        h.internal_allowed.remove(&key);
        h.page_blocked.remove(&key);
        h.gm_tokens.remove(&key);
        h.pw_tokens.remove(&key);
    });
}

/// Lets the browser navigate `key` to an internal page once.
pub fn allow_internal(key: ViewKey) {
    with(|h| {
        h.internal_allowed.insert(key);
    });
}
