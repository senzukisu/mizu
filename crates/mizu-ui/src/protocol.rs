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
    /// Find in page: search (empty text clears), step through matches, close the bar.
    Find {
        text: String,
        #[serde(default)]
        case_sensitive: bool,
    },
    FindNext {
        #[serde(default = "yes")]
        forward: bool,
    },
    FindClose,
    MoveTab { id: u64, index: usize },
    PinTab { id: u64, pinned: bool },
    /// Split view: show `tab` next to the active tab (the active tab itself: next to a new
    /// tab); swap the sides; end the split.
    SplitOpen { tab: u64 },
    SplitSwap,
    SplitClose,
    /// Spaces: switch, create (optionally moving a tab there), change, delete, move a tab.
    SpaceSwitch { id: u32 },
    SpaceCreate {
        #[serde(default)]
        tab: Option<u64>,
        #[serde(default)]
        name: String,
    },
    SpaceUpdate {
        id: u32,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        icon: Option<String>,
    },
    SpaceDelete { id: u32 },
    /// Media players found on this PC (for "Open in media player").
    MediaPlayers,
    /// An entry of the page context menu was chosen.
    PageMenuSelect { id: i32 },
    /// Per-site zoom back to the default.
    SiteZoomReset { host: String },
    /// Tree tabs: fold or unfold a tab's children; close a tab with all its children.
    TabTreeToggle { id: u64 },
    CloseBranch { id: u64 },
    TabToSpace { tab: u64, space: u32 },
    /// Tab groups: start a group with a tab, move a tab in or out, change, ungroup, close.
    GroupCreate { tab: u64 },
    GroupSet {
        tab: u64,
        #[serde(default)]
        group: Option<u32>,
    },
    GroupUpdate {
        id: u32,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        collapsed: Option<bool>,
    },
    GroupUngroup { id: u32 },
    GroupClose { id: u32 },
    /// Pinned tab: go back to its pinned page, or (`reset`) make the current page its own.
    PinnedHome {
        id: u64,
        #[serde(default)]
        reset: bool,
    },
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
    /// Media hub: `action` is toggle, play, pause, next, prev, volume (`value` 0..1) or focus.
    MediaControl {
        tab: u64,
        action: String,
        #[serde(default)]
        value: Option<f64>,
    },
    MediaList,
    /// Local crash reports: how many there are, and opening their folder.
    CrashReports,
    OpenCrashFolder,
    /// Installed font families, for the interface font setting.
    SystemFonts,
    /// Welcome tour finished; default browser status and the Windows "Default apps" page.
    WelcomeDone,
    DefaultBrowserStatus,
    MakeDefaultBrowser,
    /// Import: browser profiles on this PC with counts, and running an import.
    ImportSources,
    ImportRun {
        id: String,
        #[serde(default)]
        bookmarks: bool,
        #[serde(default)]
        history: bool,
        #[serde(default)]
        engines: bool,
        /// Put the source's bookmarks bar onto Mizu's bar (off unless chosen).
        #[serde(default)]
        to_bar: bool,
        /// Name of the subfolder for the source's bar, in the interface language.
        #[serde(default)]
        bar_name: String,
    },
    /// Opens the folder downloads are saved to.
    DownloadsOpenFolder,

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
    BookmarkMove {
        id: i64,
        parent: i64,
        index: usize,
        /// Chosen in the "Bookmark added" panel: new bookmarks go to this folder from now on.
        #[serde(default)]
        remember: bool,
    },
    BookmarkDelete { id: i64 },
    /// New tab shortcuts (their own list, not bookmarks).
    ShortcutAdd {
        title: String,
        url: String,
        #[serde(default)]
        folder: Option<i64>,
    },
    ShortcutUpdate { id: i64, title: String, url: String },
    ShortcutDelete { id: i64 },
    /// Reorder; with `into` set, move into that folder (or, `into: 0`, out to the top level).
    ShortcutMove {
        id: i64,
        index: usize,
        #[serde(default)]
        into: Option<i64>,
    },
    /// Drop tile `id` onto tile `onto`: they make a folder.
    ShortcutFolder { id: i64, onto: i64, title: String },
    /// New tab look, from its Customize panel.
    NewtabPrefs {
        background: String,
        clock: bool,
        shield_stats: bool,
        #[serde(default = "yes")]
        shortcuts: bool,
        #[serde(default = "yes")]
        now_playing: bool,
        #[serde(default)]
        recent: bool,
        #[serde(default)]
        notes: bool,
        #[serde(default)]
        tile_size: String,
        #[serde(default)]
        name: String,
    },
    DownloadsGet,
    ClearData(ClearDataRequest),
    PermissionsGet,
    PermissionSet { origin: String, kind: String, state: String },
    /// Site page (mizu://site/?host=…): everything Mizu knows and keeps about one site.
    SiteGet { host: String },
    /// Cookies of a site (names, domains), read through any normal tab.
    SiteCookiesFor { host: String },
    SiteShield { host: String, enabled: bool },
    SiteKeepSignedIn { host: String, keep: bool },
    /// Deletes the site's cookies and storage (local storage, IndexedDB, caches, workers).
    SiteForget { host: String },
    /// Cookies the active tab's site can read (names and domains, no values).
    SiteCookies,
    /// Deletes the active tab's site cookies and storage, then reloads it.
    SiteDataClear,
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
    /// Closes Mizu (saving the session) and starts it again, for settings that need it.
    RestartBrowser,
    UpdateCheck,
    UpdateInstall,
    UpdateState,
    ChooseDownloadFolder,
    RestoreSession,
    BlockedProceed { url: String },
    /// HTTPS-First fallback: open the `http://` page; `remember` keeps the site as an exception.
    HttpProceed {
        url: String,
        #[serde(default)]
        remember: bool,
    },
    /// Removes a site from the HTTPS-First exceptions.
    HttpForget { host: String },

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
    /// Media hub panel: what plays in which tab.
    Media,
    /// Menu of a tab group (`tab_id` carries the group id).
    GroupMenu,
    /// Spaces: switch, name, colour, icon, add, delete.
    SpaceMenu,
    /// A web page's context menu, drawn by Mizu.
    PageMenu,
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
    /// In the Windows app for its type, even when Mizu could show it.
    OpenSystem,
    /// The Windows "Open with" dialog: choose the program for this file.
    OpenWith,
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
    /// Media hub list changed.
    Media { items: Value },
    /// Focus the omnibox (Ctrl+L / F6 / new tab).
    FocusOmnibox,
    /// Open the find bar and focus it (Ctrl+F); `step` moves through matches (F3).
    FindShow {
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<bool>,
    },
    /// Leave the address bar and show the page's address again (after a quick action).
    FocusContent,
    /// Copy text to the clipboard and confirm (the address-bar calculator).
    CopyText { text: String },
    /// Find in page result: active match (1-based, 0 = none) and total.
    FindResult { active: i32, count: i32 },
    /// A download started; the chrome may open its downloads panel.
    DownloadStarted,
    Toast { message: String },
}

impl Event {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

fn yes() -> bool {
    true
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
