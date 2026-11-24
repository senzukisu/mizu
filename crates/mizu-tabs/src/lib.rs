//! Tab strip model.
//!
//! This crate knows nothing about web views: it owns the *order* and *metadata* of tabs in
//! one window and enforces the invariants (pinned tabs first, exactly one active tab while
//! any exist, bounded closed-tab stack). The browser layer mirrors these decisions onto
//! engine views. Keeping it pure makes the trickiest UI logic fully unit-testable.

mod session;

pub use session::{ClosedTab, SessionSnapshot, Space, TabGroup, TabSnapshot, WindowSnapshot};

/// Space colours and icons the interface offers.
pub const SPACE_COLORS: [&str; 8] = ["blue", "green", "orange", "pink", "purple", "teal", "yellow", "grey"];
pub const SPACE_ICONS: [&str; 12] = ["home", "briefcase", "music", "code", "gamepad", "book", "star", "globe", "sparkle", "wave", "user", "moon"];

/// Group colours, in the order new groups take them.
pub const GROUP_COLORS: [&str; 8] = ["blue", "green", "orange", "pink", "purple", "teal", "yellow", "grey"];

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
    /// The page a pinned tab belongs to. Browsing elsewhere keeps it; "back to pinned
    /// page" returns there.
    pub pinned_url: Option<String>,
    /// The group this tab belongs to (never for pinned tabs).
    pub group: Option<u32>,
    /// The space this tab belongs to.
    pub space: u32,
    /// Tree tabs: the tab this one was opened from (the sidebar shows it underneath).
    pub parent: Option<TabId>,
    /// Tree tabs: this tab's children are folded away.
    pub tree_collapsed: bool,
}

impl Tab {
    /// A pinned tab that has been navigated away from its pinned page (another site).
    pub fn away_from_pin(&self) -> bool {
        match (&self.pinned_url, self.pinned) {
            (Some(home), true) => !is_blank(&self.url) && site_of(home) != site_of(&self.url),
            _ => false,
        }
    }
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
            pinned_url: None,
            group: None,
            space: 0,
            parent: None,
            tree_collapsed: false,
        }
    }
}

#[derive(Debug)]
pub struct TabStrip {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    /// Most recently activated first; used to choose the next tab after a close.
    activation_order: Vec<TabId>,
    closed: VecDeque<ClosedTab>,
    groups: Vec<TabGroup>,
    /// Never empty: the main space is always there.
    spaces: Vec<Space>,
    active_space: u32,
}

impl Default for TabStrip {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            activation_order: Vec::new(),
            closed: VecDeque::new(),
            groups: Vec::new(),
            spaces: vec![Space::main()],
            active_space: 0,
        }
    }
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

    /// Inserts a tab into the current space. `index` is clamped into the region matching the
    /// tab's pinned state.
    pub fn insert(&mut self, index: usize, mut tab: Tab) -> TabId {
        tab.space = self.active_space;
        self.insert_raw(index, tab)
    }

    fn insert_raw(&mut self, mut index: usize, tab: Tab) -> TabId {
        let pinned = self.pinned_count();
        index = if tab.pinned {
            index.min(pinned)
        } else {
            index.clamp(pinned, self.tabs.len())
        };
        let id = tab.id;
        let space = tab.space;
        self.tabs.insert(index, tab);
        if self.active.is_none() && space == self.active_space {
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
        let Some(space) = self.get(id).map(|t| t.space) else { return false };
        self.active_space = space;
        self.active = Some(id);
        // A tab inside a folded branch unfolds it.
        let ancestors = self.ancestors(id);
        for a in ancestors {
            if let Some(t) = self.get_mut(a) {
                t.tree_collapsed = false;
            }
        }
        self.activation_order.retain(|&t| t != id);
        self.activation_order.insert(0, id);
        true
    }

    /// Closes a tab, remembering it for reopening. Returns the tab that became active, if
    /// the active tab changed.
    pub fn close(&mut self, id: TabId) -> Option<TabId> {
        let index = self.index_of(id)?;
        self.promote_children(id);
        let tab = self.tabs.remove(index);
        self.prune_groups();
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
        // Only tabs of the same space qualify: with none left, the space is empty (the
        // browser opens a new tab there).
        let space = tab.space;
        let next = self.tabs[index..]
            .iter()
            .find(|t| t.space == space)
            .or_else(|| self.tabs[..index].iter().rev().find(|t| t.space == space))
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
        self.promote_children(id);
        let mut tab = self.tabs.remove(from);
        tab.parent = None;
        let pinned = self.pinned_count();
        let to = if tab.pinned {
            to.min(pinned)
        } else {
            to.clamp(pinned, self.tabs.len())
        };
        self.tabs.insert(to, tab);
        self.regroup_after_move(to);
        self.prune_groups();
        from != to
    }

    // ───────────────────────────── Groups ─────────────────────────────

    pub fn groups(&self) -> &[TabGroup] {
        &self.groups
    }

    pub fn group(&self, id: u32) -> Option<&TabGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn group_mut(&mut self, id: u32) -> Option<&mut TabGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    pub fn group_tabs(&self, id: u32) -> Vec<TabId> {
        self.tabs.iter().filter(|t| t.group == Some(id)).map(|t| t.id).collect()
    }

    /// Starts a new group with `tab` in it. Pinned tabs cannot be grouped.
    pub fn create_group(&mut self, tab: TabId, name: &str) -> Option<u32> {
        if self.get(tab).is_none_or(|t| t.pinned) {
            return None;
        }
        let id = self.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1;
        let color = GROUP_COLORS[(id as usize - 1) % GROUP_COLORS.len()].to_owned();
        self.groups.push(TabGroup { id, name: name.to_owned(), color, collapsed: false });
        self.set_group(tab, Some(id));
        Some(id)
    }

    /// Puts a tab into a group (next to its last tab) or, with `None`, takes it out (just
    /// after the group, so groups stay contiguous).
    pub fn set_group(&mut self, tab: TabId, group: Option<u32>) -> bool {
        let Some(from) = self.index_of(tab) else { return false };
        if self.tabs[from].pinned || self.tabs[from].group == group {
            return false;
        }
        if let Some(g) = group {
            if self.group(g).is_none() {
                return false;
            }
        }
        let old = self.tabs[from].group;
        let mut t = self.tabs.remove(from);
        t.group = group;
        let anchor = group.or(old);
        let last = anchor.and_then(|g| self.tabs.iter().rposition(|x| x.group == Some(g)));
        let to = match (group, last) {
            (Some(_), Some(i)) | (None, Some(i)) => i + 1,
            (Some(_), None) | (None, None) => from.min(self.tabs.len()),
        };
        let pinned = self.pinned_count();
        self.tabs.insert(to.clamp(pinned, self.tabs.len()), t);
        if let Some(g) = group.and_then(|g| self.group_mut(g)) {
            g.collapsed = false;
        }
        self.prune_groups();
        true
    }

    /// Removes the group; its tabs stay where they are.
    pub fn ungroup(&mut self, id: u32) {
        for t in &mut self.tabs {
            if t.group == Some(id) {
                t.group = None;
            }
        }
        self.groups.retain(|g| g.id != id);
    }

    /// A tab dropped at `at` joins the group around it, or leaves a group it was taken out of.
    fn regroup_after_move(&mut self, at: usize) {
        let left = at.checked_sub(1).and_then(|i| self.tabs.get(i)).and_then(|t| t.group);
        let right = self.tabs.get(at + 1).and_then(|t| t.group);
        let own = self.tabs[at].group;
        if self.tabs[at].pinned {
            self.tabs[at].group = None;
            return;
        }
        self.tabs[at].group = match (left, right) {
            (Some(l), Some(r)) if l == r => Some(l),
            _ if own.is_some() && (left == own || right == own) => own,
            _ => None,
        };
    }

    /// Drops groups without tabs.
    fn prune_groups(&mut self) {
        let tabs = &self.tabs;
        self.groups.retain(|g| tabs.iter().any(|t| t.group == Some(g.id)));
    }

    pub fn set_pinned(&mut self, id: TabId, pinned: bool) -> bool {
        let Some(from) = self.index_of(id) else { return false };
        if self.tabs[from].pinned == pinned {
            return false;
        }
        self.promote_children(id);
        let mut tab = self.tabs.remove(from);
        tab.parent = None;
        tab.pinned = pinned;
        tab.pinned_url = (pinned && !is_blank(&tab.url)).then(|| tab.url.clone());
        if pinned {
            tab.group = None;
        }
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
        tab.pinned_url = src.pinned_url.clone();
        tab.group = src.group;
        tab.space = src.space;
        tab.parent = src.parent;
        let after = self.subtree_end(index);
        Some(self.insert_raw(after, tab))
    }

    // ───────────────────────────── Tree tabs ─────────────────────────────

    /// Inserts a tab opened from `parent` after the parent's branch, as its child (same space
    /// and group). Pinned parents get no children: the tab goes after the pinned tabs.
    pub fn insert_child(&mut self, parent: TabId, mut tab: Tab) -> TabId {
        let Some(pi) = self.index_of(parent) else { return self.insert_after_active(tab) };
        let p = &self.tabs[pi];
        if p.pinned {
            let at = self.pinned_count();
            return self.insert(at, tab);
        }
        tab.parent = Some(parent);
        tab.group = p.group;
        tab.space = p.space;
        let at = self.subtree_end(pi);
        self.insert_raw(at, tab)
    }

    /// Index just past the branch that starts at `index` (its descendants follow it).
    fn subtree_end(&self, index: usize) -> usize {
        let root = self.tabs[index].id;
        let mut end = index + 1;
        while end < self.tabs.len() && self.is_descendant(self.tabs[end].id, root) {
            end += 1;
        }
        end
    }

    /// Parent, grandparent, … of a tab (nearest first).
    pub fn ancestors(&self, id: TabId) -> Vec<TabId> {
        let mut out = Vec::new();
        let mut cur = self.get(id).and_then(|t| t.parent);
        while let Some(p) = cur {
            if out.contains(&p) || out.len() > 64 {
                break;
            }
            out.push(p);
            cur = self.get(p).and_then(|t| t.parent);
        }
        out
    }

    pub fn is_descendant(&self, id: TabId, ancestor: TabId) -> bool {
        self.ancestors(id).contains(&ancestor)
    }

    /// How deep a tab sits in its tree (0 for a top-level tab).
    pub fn depth(&self, id: TabId) -> usize {
        self.ancestors(id).len()
    }

    pub fn children_count(&self, id: TabId) -> usize {
        self.tabs.iter().filter(|t| t.parent == Some(id)).count()
    }

    /// The tab is inside a folded branch.
    pub fn tree_hidden(&self, id: TabId) -> bool {
        self.ancestors(id).iter().any(|a| self.get(*a).is_some_and(|t| t.tree_collapsed))
    }

    /// Folds or unfolds a tab's children. Folding the branch of the active tab activates the
    /// branch's top tab (returned).
    pub fn toggle_tree(&mut self, id: TabId) -> Option<TabId> {
        if self.children_count(id) == 0 {
            return None;
        }
        let t = self.get_mut(id)?;
        t.tree_collapsed = !t.tree_collapsed;
        let folded = t.tree_collapsed;
        if folded && self.active.is_some_and(|a| self.is_descendant(a, id)) {
            self.active = Some(id);
            self.activation_order.retain(|&t| t != id);
            self.activation_order.insert(0, id);
            return Some(id);
        }
        None
    }

    /// A tab leaves the tree: its children take its parent.
    fn promote_children(&mut self, id: TabId) {
        let parent = self.get(id).and_then(|t| t.parent);
        for t in &mut self.tabs {
            if t.parent == Some(id) {
                t.parent = parent;
            }
        }
    }

    /// The tabs of the current space, in order.
    pub fn space_tabs(&self) -> Vec<&Tab> {
        self.tabs.iter().filter(|t| t.space == self.active_space).collect()
    }

    /// Next/previous tab of the current space with wrap-around (Ctrl+Tab / Ctrl+Shift+Tab).
    pub fn cycle(&self, forward: bool) -> Option<TabId> {
        let tabs = self.space_tabs();
        let n = tabs.len();
        let current = tabs.iter().position(|t| Some(t.id) == self.active)?;
        let next = if forward { (current + 1) % n } else { (current + n - 1) % n };
        Some(tabs[next].id)
    }

    /// Ctrl+1..8 select by position in the current space, Ctrl+9 selects its last tab.
    pub fn by_shortcut(&self, digit: usize) -> Option<TabId> {
        let tabs = self.space_tabs();
        match digit {
            9 => tabs.last().copied(),
            1..=8 => tabs.get(digit - 1).copied(),
            _ => None,
        }
        .map(|t| t.id)
    }

    // ───────────────────────────── Spaces ─────────────────────────────

    pub fn spaces(&self) -> &[Space] {
        &self.spaces
    }

    pub fn active_space(&self) -> u32 {
        self.active_space
    }

    pub fn space_mut(&mut self, id: u32) -> Option<&mut Space> {
        self.spaces.iter_mut().find(|s| s.id == id)
    }

    pub fn space_tab_count(&self, id: u32) -> usize {
        self.tabs.iter().filter(|t| t.space == id).count()
    }

    /// Adds a space (not switched to). Returns its id.
    pub fn create_space(&mut self, name: &str) -> u32 {
        let id = self.spaces.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        let used: Vec<&str> = self.spaces.iter().map(|s| s.color.as_str()).collect();
        let color = SPACE_COLORS.iter().find(|c| !used.contains(c)).unwrap_or(&SPACE_COLORS[id as usize % SPACE_COLORS.len()]);
        let icon = SPACE_ICONS[(id as usize) % SPACE_ICONS.len()];
        self.spaces.push(Space { id, name: name.to_owned(), color: (*color).to_owned(), icon: icon.to_owned() });
        id
    }

    /// Makes `id` the current space and activates its most recent tab. Returns that tab, or
    /// `None` when the space has no tabs yet (the caller opens one; the old tab stays active
    /// until then).
    pub fn switch_space(&mut self, id: u32) -> Option<TabId> {
        if !self.set_active_space(id) {
            return None;
        }
        let tab = self.space_last_tab(id);
        if let Some(t) = tab {
            self.activate(t);
        }
        tab
    }

    /// The tab a space shows when switched to: its most recently active one.
    pub fn space_last_tab(&self, id: u32) -> Option<TabId> {
        self.activation_order
            .iter()
            .copied()
            .find(|t| self.get(*t).is_some_and(|t| t.space == id))
            .or_else(|| self.tabs.iter().find(|t| t.space == id).map(|t| t.id))
    }

    /// Makes `id` the current space without changing the active tab (new tabs go there).
    pub fn set_active_space(&mut self, id: u32) -> bool {
        if !self.spaces.iter().any(|s| s.id == id) {
            return false;
        }
        self.active_space = id;
        true
    }

    /// Removes a space; its tabs move to the nearest other space, which becomes current if the
    /// removed one was. The last space cannot be removed.
    pub fn delete_space(&mut self, id: u32) -> bool {
        let Some(i) = self.spaces.iter().position(|s| s.id == id) else { return false };
        if self.spaces.len() < 2 {
            return false;
        }
        self.spaces.remove(i);
        let to = self.spaces[i.saturating_sub(1).min(self.spaces.len() - 1)].id;
        for t in &mut self.tabs {
            if t.space == id {
                t.space = to;
            }
        }
        if self.active_space == id {
            self.active_space = to;
        }
        true
    }

    /// Moves a tab to another space (out of its group). If it was the active tab, the most
    /// recent other tab of the current space becomes active; returns that tab, or `None` when
    /// the current space is left without tabs.
    pub fn move_to_space(&mut self, tab: TabId, space: u32) -> Option<TabId> {
        if !self.spaces.iter().any(|s| s.id == space) {
            return self.active;
        }
        let Some(t) = self.get_mut(tab) else { return self.active };
        if t.space == space {
            return self.active;
        }
        t.space = space;
        t.group = None;
        t.parent = None;
        self.promote_children(tab);
        self.prune_groups();
        if self.active != Some(tab) {
            return self.active;
        }
        let current = self.active_space;
        let next = self
            .activation_order
            .iter()
            .copied()
            .find(|t| *t != tab && self.get(*t).is_some_and(|t| t.space == current));
        if let Some(n) = next {
            self.activate(n);
        } else {
            // Keep the moved tab as the active tab until the caller opens one here.
            self.active = Some(tab);
        }
        next
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
                    pinned_url: t.pinned_url.clone(),
                    group: t.group,
                    space: t.space,
                    parent: t.parent.and_then(|p| self.index_of(p)),
                    tree_collapsed: t.tree_collapsed,
                })
                .collect(),
            active_index: self.active.and_then(|a| self.index_of(a)).unwrap_or(0),
            closed: self.closed.iter().cloned().collect(),
            groups: self.groups.clone(),
            split: None,
            spaces: if self.spaces.len() == 1 && self.spaces[0] == Space::main() { Vec::new() } else { self.spaces.clone() },
            active_space: self.active_space,
        }
    }

    /// Rebuilds a strip from a snapshot. Returns the strip; the caller creates views.
    pub fn from_snapshot(snapshot: &WindowSnapshot) -> Self {
        let mut strip = Self::new();
        if !snapshot.spaces.is_empty() {
            strip.spaces = snapshot.spaces.clone();
        }
        let first_space = strip.spaces[0].id;
        for t in &snapshot.tabs {
            let mut tab = Tab::new(t.url.clone());
            tab.title = t.title.clone();
            tab.pinned = t.pinned;
            if t.pinned {
                tab.pinned_url = Some(t.pinned_url.clone().unwrap_or_else(|| t.url.clone()));
            }
            tab.group = t.group.filter(|g| !t.pinned && snapshot.groups.iter().any(|x| x.id == *g));
            tab.space = if strip.spaces.iter().any(|s| s.id == t.space) { t.space } else { first_space };
            tab.tree_collapsed = t.tree_collapsed;
            let at = strip.tabs.len();
            strip.insert_raw(at, tab);
        }
        // Parents by index; a link to itself or to a later tab is dropped (trees go forward).
        let ids: Vec<TabId> = strip.tabs.iter().map(|t| t.id).collect();
        for (i, t) in snapshot.tabs.iter().enumerate() {
            let parent = t.parent.filter(|p| *p < i && !strip.tabs[*p].pinned && strip.tabs[*p].space == strip.tabs[i].space).map(|p| ids[p]);
            if !strip.tabs[i].pinned {
                strip.tabs[i].parent = parent;
            }
        }
        strip.active_space = if strip.spaces.iter().any(|s| s.id == snapshot.active_space) { snapshot.active_space } else { first_space };
        if let Some(t) = strip.tabs.get(snapshot.active_index).map(|t| t.id) {
            strip.activate(t);
        }
        strip.closed = snapshot.closed.iter().take(CLOSED_TAB_LIMIT).cloned().collect();
        strip.groups = snapshot.groups.clone();
        strip.prune_groups();
        strip
    }
}

fn is_blank(url: &str) -> bool {
    url.is_empty() || url == "about:blank" || url.starts_with("mizu://newtab")
}

/// Scheme + host without a leading "www." (a pinned mail tab may wander within its site).
fn site_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.strip_prefix("www.").unwrap_or(host)
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
    fn groups_stay_together_and_survive_sessions() {
        let (mut s, ids) = strip(&["a", "b", "c", "d"]);
        let g = s.create_group(ids[0], "Work").unwrap();
        // Adding "c" moves it right after the group's last tab.
        s.set_group(ids[2], Some(g));
        assert_eq!(order(&s), ["a", "c", "b", "d"]);
        assert_eq!(s.group_tabs(g), vec![ids[0], ids[2]]);
        // Dropping "d" between two grouped tabs puts it in the group.
        s.move_tab(ids[3], 1);
        assert_eq!(s.get(ids[3]).unwrap().group, Some(g));
        // Dragging it out to the end takes it out.
        s.move_tab(ids[3], 3);
        assert_eq!(s.get(ids[3]).unwrap().group, None);
        let restored = TabStrip::from_snapshot(&s.snapshot());
        assert_eq!(restored.groups().len(), 1);
        assert_eq!(restored.group_tabs(g).len(), 2);
        // Closing every tab of a group removes the group.
        s.close(ids[0]);
        s.close(ids[2]);
        assert!(s.groups().is_empty());
    }

    #[test]
    fn ungroup_keeps_tabs() {
        let (mut s, ids) = strip(&["a", "b"]);
        let g = s.create_group(ids[1], "").unwrap();
        s.ungroup(g);
        assert!(s.groups().is_empty());
        assert_eq!(s.len(), 2);
        s.set_pinned(ids[0], true);
        assert!(s.create_group(ids[0], "x").is_none(), "pinned tabs are not grouped");
    }

    #[test]
    fn pinned_tab_remembers_its_page() {
        let (mut s, ids) = strip(&["https://mail.example.com/inbox", "b"]);
        s.set_pinned(ids[0], true);
        let t = s.get_mut(ids[0]).unwrap();
        assert_eq!(t.pinned_url.as_deref(), Some("https://mail.example.com/inbox"));
        t.url = "https://mail.example.com/thread/1".into();
        assert!(!t.away_from_pin());
        t.url = "https://news.example.org/".into();
        assert!(t.away_from_pin());
        let restored = TabStrip::from_snapshot(&s.snapshot());
        assert_eq!(restored.tabs()[0].pinned_url.as_deref(), Some("https://mail.example.com/inbox"));
        s.set_pinned(ids[0], false);
        assert_eq!(s.get(ids[0]).unwrap().pinned_url, None);
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

    #[test]
    fn spaces_keep_their_own_tabs() {
        let (mut s, ids) = strip(&["https://a/", "https://b/"]);
        let work = s.create_space("Work");
        assert_eq!(s.switch_space(work), None, "a new space has no tabs");
        let w1 = s.insert_after_active(Tab::new("https://w1/"));
        s.activate(w1);
        assert_eq!(s.active_space(), work);
        assert_eq!(s.space_tabs().len(), 1);
        assert_eq!(s.cycle(true), Some(w1), "cycling stays in the space");
        assert_eq!(s.by_shortcut(9), Some(w1));
        // Back to the main space: its last active tab returns.
        s.activate(ids[1]);
        s.activate(w1);
        assert_eq!(s.switch_space(0), Some(ids[1]));
        // Closing the space's only tab leaves no active tab in it.
        s.switch_space(work);
        assert_eq!(s.close(w1), None);
        assert_eq!(s.active(), None);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn moving_and_deleting_spaces() {
        let (mut s, ids) = strip(&["https://a/", "https://b/", "https://c/"]);
        let music = s.create_space("Music");
        s.activate(ids[0]);
        s.activate(ids[2]);
        // Moving the active tab away activates the previous one in this space.
        assert_eq!(s.move_to_space(ids[2], music), Some(ids[0]));
        assert_eq!(s.get(ids[2]).unwrap().space, music);
        assert_eq!(s.space_tab_count(music), 1);
        // Deleting a space moves its tabs back.
        assert!(s.delete_space(music));
        assert_eq!(s.get(ids[2]).unwrap().space, 0);
        assert!(!s.delete_space(0), "the last space stays");
    }

    #[test]
    fn spaces_survive_a_snapshot() {
        let (mut s, _) = strip(&["https://a/"]);
        let work = s.create_space("Work");
        s.switch_space(work);
        let w = s.push(Tab::new("https://w/"));
        s.activate(w);
        let snap = s.snapshot();
        assert_eq!(snap.spaces.len(), 2);
        let r = TabStrip::from_snapshot(&snap);
        assert_eq!(r.active_space(), work);
        assert_eq!(r.space_tab_count(work), 1);
        assert_eq!(r.get(r.active().unwrap()).unwrap().url, "https://w/");
        // A window with only the main space stores nothing extra.
        let (plain, _) = strip(&["https://a/"]);
        assert!(plain.snapshot().spaces.is_empty());
    }

    #[test]
    fn tree_tabs_nest_and_promote() {
        let (mut s, ids) = strip(&["https://a/", "https://b/"]);
        let c1 = s.insert_child(ids[0], Tab::new("https://a/1"));
        let c2 = s.insert_child(ids[0], Tab::new("https://a/2"));
        let g = s.insert_child(c1, Tab::new("https://a/1/x"));
        // Order: a, a/1, a/1/x, a/2, b
        let order: Vec<TabId> = s.tabs().iter().map(|t| t.id).collect();
        assert_eq!(order, vec![ids[0], c1, g, c2, ids[1]]);
        assert_eq!(s.depth(g), 2);
        assert_eq!(s.children_count(ids[0]), 2);
        // Folding hides the branch; activating a hidden tab unfolds it.
        s.activate(g);
        assert_eq!(s.toggle_tree(ids[0]), Some(ids[0]), "the active tab was inside");
        assert!(s.tree_hidden(g));
        s.activate(g);
        assert!(!s.tree_hidden(g));
        // Closing a parent moves its children up.
        s.close(c1);
        assert_eq!(s.get(g).unwrap().parent, Some(ids[0]));
        // A snapshot keeps the tree.
        let r = TabStrip::from_snapshot(&s.snapshot());
        let rid: Vec<TabId> = r.tabs().iter().map(|t| t.id).collect();
        assert_eq!(r.get(rid[1]).unwrap().parent, Some(rid[0]));
        assert_eq!(r.depth(rid[1]), 1);
    }
}
