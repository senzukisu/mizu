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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub space: u32,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

/// A named set of tabs in a window (work, music, …): only the current space's tabs show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Space {
    pub id: u32,
    /// Empty for the first space, which the interface calls "Main".
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub icon: String,
}

impl Space {
    /// The space every window starts with.
    pub fn main() -> Self {
        Self { id: 0, name: String::new(), color: "blue".into(), icon: "home".into() }
    }
}

/// A named, coloured group of neighbouring tabs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabGroup {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub collapsed: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<TabGroup>,
    /// Split view: (left tab index, right tab index, ratio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<(usize, usize, f64)>,
    /// Spaces; empty while the window only has the main one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spaces: Vec<Space>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub active_space: u32,
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
