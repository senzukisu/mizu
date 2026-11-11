use crate::search::{SearchEngine, builtin_search_engines};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub search: SearchSettings,
    pub privacy: PrivacySettings,
    pub security: SecuritySettings,
    pub downloads: DownloadSettings,
    pub tabs: TabSettings,
    pub performance: PerformanceSettings,
    pub media: MediaSettings,
    pub advanced: AdvancedSettings,
}

impl Settings {
    /// Replaces values that are invalid (hand-edited files, old versions) with defaults.
    pub fn sanitize(&mut self) {
        if !is_hex_color(&self.appearance.accent) {
            self.appearance.accent = AppearanceSettings::default().accent;
        }
        self.tabs.sleep_after_minutes = self.tabs.sleep_after_minutes.clamp(1, 24 * 60);
        self.appearance.zoom_percent = self.appearance.zoom_percent.clamp(25, 500);
        self.search.custom_engines.retain(SearchEngine::is_valid);
        if self.search.find_engine(&self.search.engine).is_none() {
            self.search.engine = SearchSettings::default().engine;
        }
        self.appearance.sidebar_width = self.appearance.sidebar_width.clamp(160, 520);
        if !["top", "side"].contains(&self.appearance.tab_layout.as_str()) {
            self.appearance.tab_layout = "top".into();
        }
        if !["auto", "en", "ru"].contains(&self.general.language.as_str()) {
            self.general.language = "auto".into();
        }
        let home = self.general.home_page.trim();
        let home_ok = ["https://", "http://", "mizu://"]
            .iter()
            .any(|p| home.starts_with(p));
        if !home_ok {
            self.general.home_page = GeneralSettings::default().home_page;
        }
        // Built-in lists that were retired drop out of saved settings too.
        let builtin = builtin_filter_lists();
        self.privacy.filter_lists.retain(|l| {
            if l.builtin {
                builtin.iter().any(|b| b.id == l.id)
            } else {
                l.url.starts_with("https://")
            }
        });
        for list in builtin {
            if !self.privacy.filter_lists.iter().any(|l| l.id == list.id) {
                self.privacy.filter_lists.push(list);
            }
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

// ───────────────────────────── General ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupBehavior {
    NewTab,
    RestoreSession,
    HomePage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// Interface language: "auto" (follow Windows), "en" or "ru".
    pub language: String,
    /// New windows open maximized.
    pub start_maximized: bool,
    /// Check for new Mizu versions once a day and download them in the background.
    pub auto_update: bool,
    pub startup: StartupBehavior,
    pub home_page: String,
    /// Offer to restore tabs after Mizu was not shut down cleanly.
    pub restore_after_crash: bool,
    pub confirm_close_multiple_tabs: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            start_maximized: true,
            auto_update: true,
            startup: StartupBehavior::RestoreSession,
            home_page: "mizu://newtab/".into(),
            restore_after_crash: true,
            confirm_close_multiple_tabs: true,
        }
    }
}

// ───────────────────────────── Appearance ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// Named colour palettes. Each one is a complete set of surface/ink tokens in `mizu-ui`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Palette {
    /// Cool water blues – the signature look.
    Mizu,
    /// Warm paper neutrals.
    Washi,
    /// Soft green tea.
    Matcha,
    /// Ink-black, high contrast.
    Sumi,
    /// Cherry blossom.
    Sakura,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiStyle {
    /// Rounded, floating toolbar.
    Soft,
    /// Flatter, square-ish controls.
    Crisp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub palette: Palette,
    pub accent: String,
    pub style: UiStyle,
    pub density: Density,
    pub animations: bool,
    /// Windows 11 Mica backdrop behind the browser chrome.
    pub translucency: bool,
    pub show_bookmarks_bar: bool,
    pub show_home_button: bool,
    pub zoom_percent: u32,
    /// "top" (tab strip) or "side" (vertical tabs in a sidebar).
    pub tab_layout: String,
    /// Vertical tabs sidebar shows icons only.
    pub sidebar_collapsed: bool,
    /// Vertical tabs sidebar width in CSS pixels (resizable by dragging its edge).
    pub sidebar_width: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            palette: Palette::Mizu,
            accent: "#2f7de1".into(),
            style: UiStyle::Soft,
            density: Density::Comfortable,
            animations: true,
            translucency: true,
            show_bookmarks_bar: true,
            show_home_button: false,
            zoom_percent: 100,
            tab_layout: "top".into(),
            sidebar_collapsed: false,
            sidebar_width: 252,
        }
    }
}

// ───────────────────────────── Search ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchSettings {
    /// Id of the active engine (built-in or custom).
    pub engine: String,
    pub custom_engines: Vec<SearchEngine>,
    /// Suggest matching history and bookmarks while typing (local only, nothing is sent).
    pub local_suggestions: bool,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            engine: "duckduckgo".into(),
            custom_engines: Vec::new(),
            local_suggestions: true,
        }
    }
}

impl SearchSettings {
    pub fn all_engines(&self) -> Vec<SearchEngine> {
        let mut all = builtin_search_engines();
        all.extend(self.custom_engines.iter().cloned());
        all
    }

    pub fn find_engine(&self, id: &str) -> Option<SearchEngine> {
        self.all_engines().into_iter().find(|e| e.id == id)
    }

    pub fn active_engine(&self) -> SearchEngine {
        self.find_engine(&self.engine)
            .unwrap_or_else(|| builtin_search_engines().remove(0))
    }
}

// ───────────────────────────── Privacy ─────────────────────────────

/// Maps 1:1 to the engine's built-in tracking prevention levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingPrevention {
    Off,
    Basic,
    Balanced,
    Strict,
}

/// A content-blocking filter list in uBlock Origin / Adblock Plus syntax.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterListSubscription {
    pub id: String,
    pub name: String,
    /// HTTPS URL the list is downloaded from. Empty for the bundled list.
    #[serde(default)]
    pub url: String,
    pub enabled: bool,
    /// Shipped with Mizu (cannot be removed, only disabled).
    #[serde(default)]
    pub builtin: bool,
}

pub const BUNDLED_LIST_ID: &str = "mizu-shield";

pub fn builtin_filter_lists() -> Vec<FilterListSubscription> {
    let remote = |id: &str, name: &str, url: &str, enabled: bool| FilterListSubscription {
        id: id.into(),
        name: name.into(),
        url: url.into(),
        enabled,
        builtin: true,
    };
    vec![
        FilterListSubscription {
            id: BUNDLED_LIST_ID.into(),
            name: "Mizu Shield – essential trackers".into(),
            url: String::new(),
            enabled: true,
            builtin: true,
        },
        remote("ublock-filters", "uBlock filters", "https://ublockorigin.github.io/uAssets/filters/filters.min.txt", true),
        remote("ublock-privacy", "uBlock filters – Privacy", "https://ublockorigin.github.io/uAssets/filters/privacy.min.txt", true),
        remote("ublock-unbreak", "uBlock filters – Unbreak", "https://ublockorigin.github.io/uAssets/filters/unbreak.min.txt", true),
        remote("ublock-badware", "uBlock filters – Badware risks", "https://ublockorigin.github.io/uAssets/filters/badware.min.txt", true),
        remote("easylist", "EasyList", "https://easylist.to/easylist/easylist.txt", true),
        remote("easyprivacy", "EasyPrivacy", "https://easylist.to/easylist/easyprivacy.txt", true),
        remote("peter-lowe", "Peter Lowe's Ad and tracking server list", "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext", true),
        remote("ublock-annoyances", "uBlock filters – Annoyances", "https://ublockorigin.github.io/uAssets/filters/annoyances-others.txt", false),
        remote("easylist-cookie", "EasyList Cookie (consent banners)", "https://secure.fanboy.co.nz/fanboy-cookiemonster.txt", false),
    ]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacySettings {
    pub tracking_prevention: TrackingPrevention,
    /// Network + cosmetic blocking with the filter lists below.
    pub content_blocking: bool,
    pub filter_lists: Vec<FilterListSubscription>,
    /// User-written rules in uBlock syntax.
    pub custom_rules: String,
    /// Sites where content blocking is disabled (hostnames).
    pub allowlist: Vec<String>,
    /// Send the Global Privacy Control signal (`Sec-GPC: 1`).
    pub global_privacy_control: bool,
    /// Let the engine offer to save passwords. Off by default: Mizu has no password
    /// manager UI yet, and stored passwords would be invisible to the user.
    pub engine_password_saving: bool,
    pub engine_autofill: bool,
    /// Mizu Passwords: offer to save logins and fill them from the address bar.
    pub password_manager: bool,
    pub clear_on_exit: ClearOnExit,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            tracking_prevention: TrackingPrevention::Balanced,
            content_blocking: true,
            filter_lists: builtin_filter_lists(),
            custom_rules: String::new(),
            allowlist: Vec::new(),
            global_privacy_control: true,
            engine_password_saving: false,
            engine_autofill: false,
            password_manager: true,
            clear_on_exit: ClearOnExit::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClearOnExit {
    pub history: bool,
    pub cookies: bool,
    pub cache: bool,
    pub site_storage: bool,
    pub downloads: bool,
}

// ───────────────────────────── Security ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySettings {
    /// Microsoft Defender SmartScreen phishing/malware checks performed by the engine.
    /// Note: URLs are checked against a Microsoft service.
    pub smartscreen: bool,
    /// Block pop-ups that were not triggered by a user gesture.
    pub block_unrequested_popups: bool,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            smartscreen: true,
            block_unrequested_popups: true,
        }
    }
}

// ───────────────────────────── Downloads ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DownloadSettings {
    /// Empty means the system Downloads folder.
    pub directory: String,
    pub ask_where_to_save: bool,
    pub open_panel_on_start: bool,
}

// ───────────────────────────── Tabs ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NewTabPosition {
    AfterCurrent,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TabSettings {
    pub new_tab_position: NewTabPosition,
    /// Suspend background tabs after a period of inactivity (frees CPU and memory).
    pub sleeping_tabs: bool,
    pub sleep_after_minutes: u32,
    pub switch_to_new_tabs_from_links: bool,
}

impl Default for TabSettings {
    fn default() -> Self {
        Self {
            new_tab_position: NewTabPosition::AfterCurrent,
            sleeping_tabs: true,
            sleep_after_minutes: 30,
            switch_to_new_tabs_from_links: false,
        }
    }
}

// ───────────────────────────── Performance ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceSettings {
    /// Ask the engine to keep memory low for tabs that are not visible.
    pub efficiency_mode: bool,
    /// Requires restart.
    pub hardware_acceleration: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            efficiency_mode: true,
            hardware_acceleration: true,
        }
    }
}

// ───────────────────────────── Media ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoplayPolicy {
    /// Engine default: muted autoplay allowed, audible autoplay after interaction.
    Default,
    /// No media plays before the user interacts with the page.
    RequireInteraction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaSettings {
    /// Requires restart.
    pub autoplay: AutoplayPolicy,
    /// Show a picture-in-picture button on videos (and enable Alt+Shift+P).
    pub pip_button: bool,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            autoplay: AutoplayPolicy::Default,
            pip_button: true,
        }
    }
}

// ───────────────────────────── Advanced ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    pub developer_tools: bool,
    /// Load unpacked extensions from the profile (e.g. uBlock Origin Lite). Requires restart.
    pub extensions: bool,
    /// Check installed userscripts for new versions once a day.
    pub userscript_auto_update: bool,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            developer_tools: true,
            extensions: true,
            userscript_auto_update: true,
        }
    }
}
