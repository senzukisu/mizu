//! Serializable session snapshots.
//!
//! Only the URL, title and pinned state of each tab are restored. The engine does not expose
//! a way to serialise per-tab back/forward history, so restored tabs start with a fresh
//! history stack (documented limitation). Private windows are never snapshotted.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabSnapshot {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedTab {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    /// Position the tab had when it was closed.
    #[serde(default)]
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WindowSnapshot {
    pub tabs: Vec<TabSnapshot>,
    #[serde(default)]
    pub active_index: usize,
    #[serde(default)]
    pub closed: Vec<ClosedTab>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionSnapshot {
    pub version: u32,
    pub windows: Vec<WindowSnapshot>,
    /// Window placement (x, y, width, height, maximized) of the first window.
    #[serde(default)]
    pub placement: Option<(i32, i32, i32, i32, bool)>,
}

impl SessionSnapshot {
    pub const VERSION: u32 = 1;

    pub fn is_empty(&self) -> bool {
        self.windows.iter().all(|w| w.tabs.is_empty())
    }

    pub fn tab_count(&self) -> usize {
        self.windows.iter().map(|w| w.tabs.len()).sum()
    }
}
