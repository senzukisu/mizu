//! Internal `mizu://` pages.
//!
//! Internal pages are served by the browser process from embedded assets. They are
//! the only web content allowed to talk to the privileged browser IPC, and only
//! the browser itself may navigate a tab to them (see `mizu-browser::guard`).

use serde::{Deserialize, Serialize};

pub const SCHEME: &str = "mizu";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InternalPage {
    NewTab,
    Settings,
    History,
    Bookmarks,
    Downloads,
    About,
    Media,
    Shield,
    Scripts,
    Blocked,
}

impl InternalPage {
    pub const ALL: [InternalPage; 10] = [
        Self::NewTab,
        Self::Settings,
        Self::History,
        Self::Bookmarks,
        Self::Downloads,
        Self::About,
        Self::Media,
        Self::Shield,
        Self::Scripts,
        Self::Blocked,
    ];

    pub fn host(self) -> &'static str {
        match self {
            Self::NewTab => "newtab",
            Self::Settings => "settings",
            Self::History => "history",
            Self::Bookmarks => "bookmarks",
            Self::Downloads => "downloads",
            Self::About => "about",
            Self::Media => "media",
            Self::Shield => "shield",
            Self::Scripts => "scripts",
            Self::Blocked => "blocked",
        }
    }

    pub fn url(self) -> String {
        format!("{SCHEME}://{}/", self.host())
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::NewTab => "New Tab",
            Self::Settings => "Settings",
            Self::History => "History",
            Self::Bookmarks => "Bookmarks",
            Self::Downloads => "Downloads",
            Self::About => "About Mizu",
            Self::Media => "Media diagnostics",
            Self::Shield => "Mizu Shield",
            Self::Scripts => "Scripts & extensions",
            Self::Blocked => "Page blocked",
        }
    }

    /// Resolves an internal URL such as `mizu://settings/privacy` to its page.
    pub fn from_url(raw: &str) -> Option<Self> {
        let parsed = url::Url::parse(raw).ok()?;
        if parsed.scheme() != SCHEME {
            return None;
        }
        let host = parsed.host_str()?;
        Self::ALL.into_iter().find(|p| p.host() == host)
    }
}

pub fn is_internal_url(raw: &str) -> bool {
    raw.len() > SCHEME.len() + 3
        && raw[..SCHEME.len()].eq_ignore_ascii_case(SCHEME)
        && raw[SCHEME.len()..].starts_with("://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for page in InternalPage::ALL {
            assert_eq!(InternalPage::from_url(&page.url()), Some(page));
        }
        assert_eq!(InternalPage::from_url("mizu://settings/privacy"), Some(InternalPage::Settings));
        assert_eq!(InternalPage::from_url("https://settings/"), None);
        assert_eq!(InternalPage::from_url("mizu://unknown/"), None);
    }

    #[test]
    fn detects_internal() {
        assert!(is_internal_url("mizu://newtab/"));
        assert!(is_internal_url("MIZU://about/"));
        assert!(!is_internal_url("https://mizu.example/"));
        assert!(!is_internal_url("mizu:"));
    }
}
