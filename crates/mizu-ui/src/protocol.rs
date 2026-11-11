//! JSON protocol between UI surfaces (`chrome`, `flyout`, internal pages) and the browser.
//!
//! UI → browser: `{"rid": 7, "cmd": "history_query", "query": "rust", "offset": 0}`
//! Browser → UI: `{"type": "reply", "rid": 7, "ok": true, "data": [...]}` or push events.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Rectangle in CSS pixels of the sending surface, used to position flyouts.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Default)]
pub struct Anchor {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Envelope {
    /// Request id; when present the browser answers with a `reply` event.
    #[serde(default)]
    pub rid: Option<u64>,
    #[serde(flatten)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    // ── Surfaces ──────────────────────────────────────────────
    Ready,
    Window { action: WindowAction },
    ShowFlyout {
        kind: FlyoutKind,
        anchor: Anchor,
        #[serde(default)]
        tab_id: Option<u64>,
    },
    CloseFlyout,
    FlyoutSize { width: f64, height: f64 },
    /// Collapse/expand the vertical tabs sidebar.
    SidebarToggle,
    /// Live resize of the sidebar (CSS px); `commit` on drag end saves the width.
    SidebarResize {
        width: f64,
        #[serde(default)]
        commit: bool,
    },
    /// Height of the tab strip + toolbar (+ bookmarks bar) and where the omnibox is.
    ChromeMetrics { height: f64, omnibox: Anchor },

    // ── Tabs ─────────────────────────────────────────────────
    NewTab {
        #[serde(default)]
        url: Option<String>,
    },
    NewWindow {
        #[serde(default)]
        private: bool,
    },
    CloseTab { id: u64 },
    ActivateTab { id: u64 },
    MoveTab { id: u64, index: usize },
    PinTab { id: u64, pinned: bool },
    MuteTab { id: u64, muted: bool },
    /// Closes the page of a background tab to free memory; it reloads when opened.
    /// With `others`, every background tab except `id` that isn't playing sound.
    DiscardTab {
        id: u64,
        #[serde(default)]
        others: bool,
    },
    DuplicateTab { id: u64 },
    ReloadTab { id: u64 },
    CloseOtherTabs { id: u64 },
    CloseTabsRight { id: u64 },
    ReopenClosedTab,

    // ── Navigation ───────────────────────────────────────────
    Navigate { input: String },
    OpenUrl {
        url: String,
        #[serde(default)]
        disposition: Disposition,
    },
    Back,
    Forward,
    Reload {
        #[serde(default)]
        bypass_cache: bool,
    },
    Stop,
    Home,
    OmniboxInput { text: String },
    OmniboxHighlight { index: usize },
    OmniboxCommit { index: usize },
    OmniboxClose,
    Zoom { delta: i32 },
    Print,
    Devtools,
    TaskManager,
    ToggleBookmark,

    // ── Prompts & panels ─────────────────────────────────────
    PermissionResponse { prompt_id: u64, allow: bool, remember: bool },
    ConfirmResponse {
        accepted: bool,
        #[serde(default)]
        dont_ask_again: bool,
    },
    ShieldToggleSite { enabled: bool },
    DownloadAction { id: u64, action: DownloadAction },
    DownloadsClear,

    // ── Data for internal pages ──────────────────────────────
    GetSettings,
    SetSettings { settings: Value },
    ResetSettings,
    HistoryQuery {
        #[serde(default)]
        query: String,
        #[serde(default)]
        offset: usize,
    },
    HistoryDelete { id: i64 },
    HistoryClear { range: String },
    BookmarksGet,
    BookmarkAdd { parent: i64, title: String, url: String },
    BookmarkAddFolder { parent: i64, title: String },
    BookmarkUpdate {
        id: i64,
        title: String,
        #[serde(default)]
        url: Option<String>,
    },
    BookmarkMove { id: i64, parent: i64, index: usize },
    BookmarkDelete { id: i64 },
    DownloadsGet,
    ClearData(ClearDataRequest),
    PermissionsGet,
    PermissionSet { origin: String, kind: String, state: String },
    FiltersUpdate {
        #[serde(default)]
        force: bool,
    },
    FilterListAdd { name: String, url: String },
    FilterListRemove { id: String },
    ShieldStats,
    MediaProbe,
    About,
    NewtabData,
    UpdateCheck,
    UpdateInstall,
    UpdateState,
    ChooseDownloadFolder,
    RestoreSession,
    BlockedProceed { url: String },

    // ── Userscripts & extensions ─────────────────────────────
    ScriptsGet,
    ScriptPending,
    /// Install the pending scripts with these indices (all when omitted).
    ScriptInstallConfirm {
        #[serde(default)]
        indices: Option<Vec<usize>>,
    },
    ScriptInstallCancel,
    /// Fetch userscripts from links (GitHub, GitLab, Gist, Greasy Fork, raw URLs…).
    ScriptInstallUrls { urls: Vec<String> },
    /// Pick one or more `.user.js` files.
    ScriptInstallFiles,
    ScriptsCheckUpdates,

    // ── Passwords ────────────────────────────────────────────
    PasswordsGet {
        #[serde(default)]
        query: String,
    },
    PasswordReveal { id: i64 },
    PasswordSave {
        #[serde(default)]
        id: Option<i64>,
        origin: String,
        username: String,
        /// `None` keeps the stored password (editing other fields).
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        note: String,
    },
    PasswordDelete { id: i64 },
    PasswordNeverRemove { origin: String },
    PasswordsImport,
    PasswordsExport,
    PasswordFill { id: i64 },
    PasswordPromptResponse {
        action: String,
        #[serde(default)]
        username: Option<String>,
    },
    ScriptToggle { id: String, enabled: bool },
    /// Settings page is recording a shortcut: let every key reach it.
    ShortcutCapture { on: bool },
    ScriptSites {
        id: String,
        #[serde(default)]
        only: Vec<String>,
        #[serde(default)]
        never: Vec<String>,
    },
    ScriptRemove { id: String },
    ScriptCreate { source: String },
    ScriptSource { id: String },
    ExtensionsGet,
    ExtensionInstall,
    /// Install from a Chrome Web Store / Edge Add-ons page URL or an extension id.
    ExtensionInstallFromStore { url: String },
    ExtensionToggle { id: String, enabled: bool },
    ExtensionRemove { id: String },
    ExtensionOpen { id: String },
}

impl Command {
    /// Commands that only the browser's own UI surfaces (tab strip, flyout) may send. Internal
    /// pages get the data commands; ordinary web pages get nothing.
    pub fn is_chrome_only(&self) -> bool {
        matches!(
            self,
            Self::Window { .. }
                | Self::ShowFlyout { .. }
                | Self::FlyoutSize { .. }
                | Self::SidebarToggle
                | Self::SidebarResize { .. }
                | Self::ChromeMetrics { .. }
                | Self::OmniboxHighlight { .. }
                | Self::PermissionResponse { .. }
                | Self::ConfirmResponse { .. }
                | Self::PasswordFill { .. }
                | Self::PasswordPromptResponse { .. }
                | Self::OmniboxInput { .. }
                | Self::OmniboxCommit { .. }
                | Self::OmniboxClose
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowAction {
    Minimize,
    Maximize,
    Close,
    Fullscreen,
    DragStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlyoutKind {
    Menu,
    TabMenu,
    /// Right-click menu of a bookmarks bar item (`tab_id` carries the bookmark id).
    BookmarkMenu,
    Downloads,
    Shield,
    Bookmark,
    Omnibox,
    SiteInfo,
    Permission,
    Confirm,
    SavePassword,
    Logins,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    #[default]
    Current,
    NewTab,
    Background,
    NewWindow,
    PrivateWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    Open,
    Show,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct ClearDataRequest {
    pub range: String,
    pub history: bool,
    pub cookies: bool,
    pub cache: bool,
    pub site_storage: bool,
    pub downloads: bool,
    pub permissions: bool,
    pub autofill: bool,
    pub passwords: bool,
}

/// Browser → UI messages.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Reply {
        rid: u64,
        ok: bool,
        #[serde(skip_serializing_if = "Value::is_null")]
        data: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Full window state for the tab strip and toolbar.
    State { state: Value },
    Favicon { id: u64, data: String },
    Suggestions { items: Value },
    /// Content for the flyout surface.
    Flyout { kind: FlyoutKind, data: Value },
    /// Appearance and preferences changed (broadcast to every surface).
    Settings { settings: Value },
    Downloads { items: Value },
    /// Focus the omnibox (Ctrl+L / F6 / new tab).
    FocusOmnibox,
    /// A download started; the chrome may open its downloads panel.
    DownloadStarted,
    Toast { message: String },
}

impl Event {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands() {
        let e: Envelope = serde_json::from_str(r#"{"rid":3,"cmd":"history_query","query":"rust"}"#).unwrap();
        assert_eq!(e.rid, Some(3));
        assert_eq!(e.command, Command::HistoryQuery { query: "rust".into(), offset: 0 });

        let e: Envelope = serde_json::from_str(
            r#"{"cmd":"show_flyout","kind":"menu","anchor":{"x":1,"y":2,"width":3,"height":4}}"#,
        )
        .unwrap();
        assert!(e.command.is_chrome_only());

        let e: Envelope = serde_json::from_str(r#"{"cmd":"clear_data","range":"all","cookies":true}"#).unwrap();
        match e.command {
            Command::ClearData(r) => assert!(r.cookies && !r.history),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_commands() {
        assert!(serde_json::from_str::<Envelope>(r#"{"cmd":"rm_rf"}"#).is_err());
        assert!(serde_json::from_str::<Envelope>(r#"{"cmd":"close_tab","id":"x"}"#).is_err());
    }

    #[test]
    fn serialises_events() {
        let j = Event::Reply { rid: 1, ok: true, data: Value::Null, error: None }.to_json();
        assert_eq!(j, r#"{"type":"reply","rid":1,"ok":true}"#);
    }
}
