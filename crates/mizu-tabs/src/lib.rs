//! Tab strip model.
//!
//! This crate knows nothing about web views: it owns the *order* and *metadata* of tabs in
//! one window and enforces the invariants (pinned tabs first, exactly one active tab while
//! any exist, bounded closed-tab stack). The browser layer mirrors these decisions onto
//! engine views. Keeping it pure makes the trickiest UI logic fully unit-testable.

mod session;

pub use session::{ClosedTab, SessionSnapshot, TabSnapshot, WindowSnapshot};

use mizu_core::ids::TabId;
use serde::Serialize;
use std::collections::VecDeque;

/// How many closed tabs can be reopened with Ctrl+Shift+T.
pub const CLOSED_TAB_LIMIT: usize = 25;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tab {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub pinned: bool,
}

impl Tab {
    pub fn new(url: impl Into<String>) -> Self {
        Self::with_id(TabId::next(), url)
    }

    /// A tab whose id was allocated earlier (e.g. for a pre-created engine view).
    pub fn with_id(id: TabId, url: impl Into<String>) -> Self {
        Self {
            id,
            url: url.into(),
            title: String::new(),
            pinned: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct TabStrip {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    /// Most recently activated first; used to choose the next tab after a close.
    activation_order: Vec<TabId>,
    closed: VecDeque<ClosedTab>,
}

impl TabStrip {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active(&self) -> Option<TabId> {
        self.active
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    fn pinned_count(&self) -> usize {
        self.tabs.iter().take_while(|t| t.pinned).count()
    }

    /// Inserts a tab. `index` is clamped into the region matching the tab's pinned state.
    pub fn insert(&mut self, mut index: usize, tab: Tab) -> TabId {
        let pinned = self.pinned_count();
        index = if tab.pinned {
            index.min(pinned)
        } else {
            index.clamp(pinned, self.tabs.len())
        };
        let id = tab.id;
        self.tabs.insert(index, tab);
        if self.active.is_none() {
            self.activate(id);
        }
        id
    }

    /// Inserts right after the active tab (or at the end).
    pub fn insert_after_active(&mut self, tab: Tab) -> TabId {
        let at = self
            .active
            .and_then(|a| self.index_of(a))
            .map_or(self.tabs.len(), |i| i + 1);
        self.insert(at, tab)
    }

    pub fn push(&mut self, tab: Tab) -> TabId {
        self.insert(self.tabs.len(), tab)
    }

    pub fn activate(&mut self, id: TabId) -> bool {
        if self.index_of(id).is_none() {
            return false;
        }
        self.active = Some(id);
        self.activation_order.retain(|&t| t != id);
        self.activation_order.insert(0, id);
        true
    }

    /// Closes a tab, remembering it for reopening. Returns the tab that became active, if
    /// the active tab changed.
    pub fn close(&mut self, id: TabId) -> Option<TabId> {
        let index = self.index_of(id)?;
        let tab = self.tabs.remove(index);
        self.activation_order.retain(|&t| t != id);
        if !is_blank(&tab.url) {
            self.closed.push_front(ClosedTab {
                url: tab.url,
                title: tab.title,
                pinned: tab.pinned,
                index,
            });
            self.closed.truncate(CLOSED_TAB_LIMIT);
        }
        if self.active != Some(id) {
            return None;
        }
        self.active = None;
        // Prefer the neighbour to the right, like mainstream browsers; fall back to the left.
        let next = self
            .tabs
            .get(index)
            .or_else(|| index.checked_sub(1).and_then(|i| self.tabs.get(i)))
            .map(|t| t.id);
        if let Some(next) = next {
            self.activate(next);
        }
        next
    }

    /// Reopens the most recently closed tab at its former position and activates it.
    pub fn reopen_closed(&mut self) -> Option<TabId> {
        let closed = self.closed.pop_front()?;
        let mut tab = Tab::new(closed.url);
        tab.title = closed.title;
        tab.pinned = closed.pinned;
        let id = self.insert(closed.index, tab);
        self.activate(id);
        Some(id)
    }

    pub fn closed_tabs(&self) -> impl Iterator<Item = &ClosedTab> {
        self.closed.iter()
    }

    /// Moves a tab to `to`, clamped so pinned and unpinned tabs never interleave.
    pub fn move_tab(&mut self, id: TabId, to: usize) -> bool {
        let Some(from) = self.index_of(id) else { return false };
        let tab = self.tabs.remove(from);
        let pinned = self.pinned_count();
        let to = if tab.pinned {
            to.min(pinned)
        } else {
            to.clamp(pinned, self.tabs.len())
        };
        self.tabs.insert(to, tab);
        from != to
    }

    pub fn set_pinned(&mut self, id: TabId, pinned: bool) -> bool {
        let Some(from) = self.index_of(id) else { return false };
        if self.tabs[from].pinned == pinned {
            return false;
        }
        let mut tab = self.tabs.remove(from);
        tab.pinned = pinned;
        // A newly pinned tab goes to the end of the pinned group; an unpinned one to the
        // start of the normal group. Both are the same boundary index.
        let boundary = self.pinned_count();
        self.tabs.insert(boundary, tab);
        true
    }

    /// Creates a copy of `id` right after it (same URL, same pinned state).
    pub fn duplicate(&mut self, id: TabId) -> Option<TabId> {
        let index = self.index_of(id)?;
        let src = &self.tabs[index];
        let mut tab = Tab::new(src.url.clone());
        tab.title = src.title.clone();
        tab.pinned = src.pinned;
        Some(self.insert(index + 1, tab))
    }

    /// Next/previous tab with wrap-around (Ctrl+Tab / Ctrl+Shift+Tab).
    pub fn cycle(&self, forward: bool) -> Option<TabId> {
        let n = self.tabs.len();
        let current = self.active.and_then(|a| self.index_of(a))?;
        let next = if forward { (current + 1) % n } else { (current + n - 1) % n };
        Some(self.tabs[next].id)
    }

    /// Ctrl+1..8 select by position, Ctrl+9 selects the last tab.
    pub fn by_shortcut(&self, digit: usize) -> Option<TabId> {
        match digit {
            9 => self.tabs.last(),
            1..=8 => self.tabs.get(digit - 1),
            _ => None,
        }
        .map(|t| t.id)
    }

    /// Tabs ordered by how recently they were active (most recent first).
    pub fn recently_active(&self) -> &[TabId] {
        &self.activation_order
    }

    pub fn snapshot(&self) -> WindowSnapshot {
        WindowSnapshot {
            tabs: self
                .tabs
                .iter()
                .map(|t| TabSnapshot {
                    url: t.url.clone(),
                    title: t.title.clone(),
                    pinned: t.pinned,
                })
                .collect(),
            active_index: self.active.and_then(|a| self.index_of(a)).unwrap_or(0),
            closed: self.closed.iter().cloned().collect(),
        }
    }

    /// Rebuilds a strip from a snapshot. Returns the strip; the caller creates views.
    pub fn from_snapshot(snapshot: &WindowSnapshot) -> Self {
        let mut strip = Self::new();
        for t in &snapshot.tabs {
            let mut tab = Tab::new(t.url.clone());
            tab.title = t.title.clone();
            tab.pinned = t.pinned;
            strip.push(tab);
        }
        if let Some(t) = strip.tabs.get(snapshot.active_index).map(|t| t.id) {
            strip.activate(t);
        }
        strip.closed = snapshot.closed.iter().take(CLOSED_TAB_LIMIT).cloned().collect();
        strip
    }
}

fn is_blank(url: &str) -> bool {
    url.is_empty() || url == "about:blank" || url.starts_with("mizu://newtab")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(urls: &[&str]) -> (TabStrip, Vec<TabId>) {
        let mut s = TabStrip::new();
        let ids = urls.iter().map(|u| s.push(Tab::new(*u))).collect();
        (s, ids)
    }

    fn order(s: &TabStrip) -> Vec<String> {
        s.tabs().iter().map(|t| t.url.clone()).collect()
    }

    #[test]
    fn first_tab_becomes_active() {
        let (s, ids) = strip(&["a", "b"]);
        assert_eq!(s.active(), Some(ids[0]));
    }

    #[test]
    fn close_activates_right_then_left() {
        let (mut s, ids) = strip(&["a", "b", "c"]);
        s.activate(ids[1]);
        assert_eq!(s.close(ids[1]), Some(ids[2]));
        assert_eq!(s.close(ids[2]), Some(ids[0]));
        assert_eq!(s.close(ids[0]), None);
        assert!(s.is_empty());
        assert_eq!(s.active(), None);
    }

    #[test]
    fn closing_inactive_tab_keeps_active() {
        let (mut s, ids) = strip(&["a", "b"]);
        assert_eq!(s.close(ids[1]), None);
        assert_eq!(s.active(), Some(ids[0]));
    }

    #[test]
    fn reopen_restores_position() {
        let (mut s, ids) = strip(&["a", "b", "c"]);
        s.close(ids[1]);
        let id = s.reopen_closed().unwrap();
        assert_eq!(order(&s), ["a", "b", "c"]);
        assert_eq!(s.active(), Some(id));
        assert!(s.reopen_closed().is_none());
    }

    #[test]
    fn blank_tabs_are_not_remembered() {
        let (mut s, ids) = strip(&["mizu://newtab/", "a"]);
        s.close(ids[0]);
        assert_eq!(s.closed_tabs().count(), 0);
    }

    #[test]
    fn closed_stack_is_bounded() {
        let mut s = TabStrip::new();
        for i in 0..CLOSED_TAB_LIMIT + 5 {
            let id = s.push(Tab::new(format!("https://{i}.example/")));
            s.close(id);
        }
        assert_eq!(s.closed_tabs().count(), CLOSED_TAB_LIMIT);
    }

    #[test]
    fn pinned_tabs_stay_in_front() {
        let (mut s, ids) = strip(&["a", "b", "c"]);
        s.set_pinned(ids[2], true);
        assert_eq!(order(&s), ["c", "a", "b"]);
        // An unpinned tab cannot be dragged before a pinned one.
        s.move_tab(ids[1], 0);
        assert_eq!(order(&s), ["c", "b", "a"]);
        // A pinned tab cannot be dragged into the unpinned area.
        s.move_tab(ids[2], 10);
        assert_eq!(order(&s), ["c", "b", "a"]);
        s.set_pinned(ids[2], false);
        assert_eq!(order(&s), ["c", "b", "a"]);
        s.move_tab(ids[2], 10);
        assert_eq!(order(&s), ["b", "a", "c"]);
    }

    #[test]
    fn duplicate_inserts_next_to_source() {
        let (mut s, ids) = strip(&["a", "b"]);
        s.duplicate(ids[0]);
        assert_eq!(order(&s), ["a", "a", "b"]);
    }

    #[test]
    fn cycling_and_shortcuts() {
        let (mut s, ids) = strip(&["a", "b", "c"]);
        assert_eq!(s.cycle(false), Some(ids[2]));
        s.activate(ids[2]);
        assert_eq!(s.cycle(true), Some(ids[0]));
        assert_eq!(s.by_shortcut(9), Some(ids[2]));
        assert_eq!(s.by_shortcut(2), Some(ids[1]));
        assert_eq!(s.by_shortcut(7), None);
    }

    #[test]
    fn snapshot_round_trip() {
        let (mut s, ids) = strip(&["a", "b", "c"]);
        s.set_pinned(ids[1], true);
        s.activate(ids[2]);
        s.close(ids[0]);
        let snap = s.snapshot();
        let restored = TabStrip::from_snapshot(&snap);
        assert_eq!(order(&restored), ["b", "c"]);
        assert!(restored.tabs()[0].pinned);
        assert_eq!(restored.active(), Some(restored.tabs()[1].id));
        assert_eq!(restored.closed_tabs().count(), 1);
    }
}
