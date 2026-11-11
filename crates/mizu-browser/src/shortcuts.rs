//! Keyboard shortcuts handled by the browser (the engine handles in-page ones such as Ctrl+F,
//! Ctrl+P, zoom and F12 itself).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    NewTab,
    NewWindow,
    NewPrivateWindow,
    CloseTab,
    ReopenClosedTab,
    NextTab,
    PrevTab,
    SelectTab(usize),
    FocusOmnibox,
    Reload,
    HardReload,
    Back,
    Forward,
    Home,
    BookmarkPage,
    ToggleBookmarksBar,
    History,
    Downloads,
    Bookmarks,
    ClearData,
    Fullscreen,
    Escape,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

const VK_TAB: u32 = 0x09;
const VK_ESCAPE: u32 = 0x1B;
const VK_PRIOR: u32 = 0x21;
const VK_NEXT: u32 = 0x22;
const VK_HOME: u32 = 0x24;
const VK_LEFT: u32 = 0x25;
const VK_RIGHT: u32 = 0x27;
const VK_DELETE: u32 = 0x2E;
const VK_F4: u32 = 0x73;
const VK_F5: u32 = 0x74;
const VK_F6: u32 = 0x75;
const VK_F11: u32 = 0x7A;
const VK_BROWSER_BACK: u32 = 0xA6;
const VK_BROWSER_FORWARD: u32 = 0xA7;
const VK_BROWSER_REFRESH: u32 = 0xA8;
const VK_BROWSER_HOME: u32 = 0xAC;
const VK_OEM_PLUS: u32 = 0xBB;
const VK_OEM_MINUS: u32 = 0xBD;
const VK_ADD: u32 = 0x6B;
const VK_SUBTRACT: u32 = 0x6D;

/// `chrome_surface` is true for the browser UI views, where the engine's own zoom handling is
/// disabled so we provide it.
pub fn map(vk: u32, ctrl: bool, shift: bool, alt: bool, chrome_surface: bool) -> Option<Shortcut> {
    use Shortcut::*;
    let key = |c: char| vk == c as u32;
    Some(match (ctrl, shift, alt) {
        (true, false, false) => match vk {
            _ if key('T') => NewTab,
            _ if key('N') => NewWindow,
            _ if key('W') => CloseTab,
            VK_F4 => CloseTab,
            VK_TAB | VK_NEXT => NextTab,
            VK_PRIOR => PrevTab,
            _ if key('L') => FocusOmnibox,
            _ if key('K') || key('E') => FocusOmnibox,
            _ if key('R') => Reload,
            VK_F5 => HardReload,
            _ if key('D') => BookmarkPage,
            _ if key('H') => History,
            _ if key('J') => Downloads,
            0x31..=0x39 => SelectTab((vk - 0x30) as usize),
            VK_OEM_PLUS | VK_ADD if chrome_surface => ZoomIn,
            VK_OEM_MINUS | VK_SUBTRACT if chrome_surface => ZoomOut,
            0x30 if chrome_surface => ZoomReset,
            _ => return None,
        },
        (true, true, false) => match vk {
            _ if key('T') => ReopenClosedTab,
            _ if key('N') || key('P') => NewPrivateWindow,
            VK_TAB => PrevTab,
            _ if key('B') => ToggleBookmarksBar,
            _ if key('O') => Bookmarks,
            _ if key('R') => HardReload,
            VK_DELETE => ClearData,
            _ => return None,
        },
        (false, true, false) if vk == VK_F5 => HardReload,
        (false, false, true) => match vk {
            VK_LEFT => Back,
            VK_RIGHT => Forward,
            VK_HOME => Home,
            _ if key('D') => FocusOmnibox,
            _ => return None,
        },
        (false, false, false) => match vk {
            VK_F5 | VK_BROWSER_REFRESH => Reload,
            VK_F6 => FocusOmnibox,
            VK_F11 => Fullscreen,
            VK_BROWSER_BACK => Back,
            VK_BROWSER_FORWARD => Forward,
            VK_BROWSER_HOME => Home,
            VK_ESCAPE if chrome_surface => Escape,
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_shortcuts() {
        assert_eq!(map('T' as u32, true, false, false, false), Some(Shortcut::NewTab));
        assert_eq!(map('T' as u32, true, true, false, false), Some(Shortcut::ReopenClosedTab));
        assert_eq!(map(0x33, true, false, false, false), Some(Shortcut::SelectTab(3)));
        assert_eq!(map(VK_LEFT, false, false, true, false), Some(Shortcut::Back));
        // Zoom in content is handled by the engine.
        assert_eq!(map(VK_OEM_PLUS, true, false, false, false), None);
        assert_eq!(map(VK_OEM_PLUS, true, false, false, true), Some(Shortcut::ZoomIn));
        assert_eq!(map('F' as u32, true, false, false, false), None);
    }
}

// ───────────────────────────── Customisable shortcuts ─────────────────────────────

/// A browser action whose keys the user can change in Settings → Shortcuts.
pub struct ActionDef {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub action: Shortcut,
    pub defaults: &'static [&'static str],
}

pub const ACTIONS: &[ActionDef] = &[
    ActionDef { id: "new_tab", group: "Tabs", label: "New tab", action: Shortcut::NewTab, defaults: &["Ctrl+T"] },
    ActionDef { id: "close_tab", group: "Tabs", label: "Close tab", action: Shortcut::CloseTab, defaults: &["Ctrl+W", "Ctrl+F4"] },
    ActionDef { id: "reopen_tab", group: "Tabs", label: "Reopen closed tab", action: Shortcut::ReopenClosedTab, defaults: &["Ctrl+Shift+T"] },
    ActionDef { id: "next_tab", group: "Tabs", label: "Next tab", action: Shortcut::NextTab, defaults: &["Ctrl+Tab", "Ctrl+PgDn"] },
    ActionDef { id: "prev_tab", group: "Tabs", label: "Previous tab", action: Shortcut::PrevTab, defaults: &["Ctrl+Shift+Tab", "Ctrl+PgUp"] },
    ActionDef { id: "new_window", group: "Windows", label: "New window", action: Shortcut::NewWindow, defaults: &["Ctrl+N"] },
    ActionDef { id: "new_private_window", group: "Windows", label: "New private window", action: Shortcut::NewPrivateWindow, defaults: &["Ctrl+Shift+N", "Ctrl+Shift+P"] },
    ActionDef { id: "fullscreen", group: "Windows", label: "Full screen", action: Shortcut::Fullscreen, defaults: &["F11"] },
    ActionDef { id: "back", group: "Navigation", label: "Back", action: Shortcut::Back, defaults: &["Alt+Left"] },
    ActionDef { id: "forward", group: "Navigation", label: "Forward", action: Shortcut::Forward, defaults: &["Alt+Right"] },
    ActionDef { id: "home", group: "Navigation", label: "Home page", action: Shortcut::Home, defaults: &["Alt+Home"] },
    ActionDef { id: "reload", group: "Navigation", label: "Reload page", action: Shortcut::Reload, defaults: &["F5", "Ctrl+R"] },
    ActionDef { id: "hard_reload", group: "Navigation", label: "Reload without cache", action: Shortcut::HardReload, defaults: &["Ctrl+F5", "Shift+F5", "Ctrl+Shift+R"] },
    ActionDef { id: "address_bar", group: "Navigation", label: "Address bar", action: Shortcut::FocusOmnibox, defaults: &["Ctrl+L", "Alt+D", "F6", "Ctrl+K", "Ctrl+E"] },
    ActionDef { id: "bookmark_page", group: "Mizu", label: "Bookmark this page", action: Shortcut::BookmarkPage, defaults: &["Ctrl+D"] },
    ActionDef { id: "bookmarks_bar", group: "Mizu", label: "Show or hide bookmarks bar", action: Shortcut::ToggleBookmarksBar, defaults: &["Ctrl+Shift+B"] },
    ActionDef { id: "bookmarks", group: "Mizu", label: "Bookmarks", action: Shortcut::Bookmarks, defaults: &["Ctrl+Shift+O"] },
    ActionDef { id: "history", group: "Mizu", label: "History", action: Shortcut::History, defaults: &["Ctrl+H"] },
    ActionDef { id: "downloads", group: "Mizu", label: "Downloads", action: Shortcut::Downloads, defaults: &["Ctrl+J"] },
    ActionDef { id: "clear_data", group: "Mizu", label: "Clear browsing data", action: Shortcut::ClearData, defaults: &["Ctrl+Shift+Delete"] },
];

const CTRL: u8 = 1;
const SHIFT: u8 = 2;
const ALT: u8 = 4;

/// Parses "Ctrl+Shift+T" into virtual-key codes (some names map to two keys, e.g. Plus)
/// and modifier bits. Unknown names give `None`.
pub fn parse_combo(combo: &str) -> Option<(Vec<u32>, u8)> {
    let mut mods = 0u8;
    let mut key: Option<Vec<u32>> = None;
    for part in combo.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= CTRL,
            "shift" => mods |= SHIFT,
            "alt" => mods |= ALT,
            k => key = Some(key_codes(k)?),
        }
    }
    // "Ctrl++" splits into empty parts: treat a trailing '+' as the Plus key.
    if key.is_none() && combo.trim_end().ends_with("++") {
        key = Some(vec![VK_OEM_PLUS, VK_ADD]);
    }
    Some((key?, mods))
}

fn key_codes(k: &str) -> Option<Vec<u32>> {
    let single = |v: u32| Some(vec![v]);
    if k.len() == 1 {
        let c = k.chars().next()?.to_ascii_uppercase();
        if c.is_ascii_alphanumeric() {
            return single(c as u32);
        }
    }
    if let Some(n) = k.strip_prefix('f').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=24).contains(&n) {
            return single(0x6F + n);
        }
    }
    match k {
        "tab" => single(VK_TAB),
        "esc" | "escape" => single(VK_ESCAPE),
        "pgup" | "pageup" => single(VK_PRIOR),
        "pgdn" | "pagedown" => single(VK_NEXT),
        "home" => single(VK_HOME),
        "end" => single(0x23),
        "left" => single(VK_LEFT),
        "up" => single(0x26),
        "right" => single(VK_RIGHT),
        "down" => single(0x28),
        "insert" | "ins" => single(0x2D),
        "delete" | "del" => single(VK_DELETE),
        "space" => single(0x20),
        "enter" => single(0x0D),
        "backspace" => single(0x08),
        "plus" => Some(vec![VK_OEM_PLUS, VK_ADD]),
        "minus" => Some(vec![VK_OEM_MINUS, VK_SUBTRACT]),
        "comma" => single(0xBC),
        "period" => single(0xBE),
        "slash" => single(0xBF),
        _ => None,
    }
}

/// The active key table: defaults overlaid with the user's changes.
#[derive(Clone, Default)]
pub struct Bindings {
    map: std::collections::HashMap<(u32, u8), Shortcut>,
}

impl Bindings {
    pub fn build(overrides: &std::collections::BTreeMap<String, Vec<String>>) -> Self {
        let mut map = std::collections::HashMap::new();
        for a in ACTIONS {
            let combos: Vec<String> = match overrides.get(a.id) {
                Some(list) => list.clone(),
                None => a.defaults.iter().map(|s| (*s).to_owned()).collect(),
            };
            for c in combos {
                if let Some((vks, mods)) = parse_combo(&c) {
                    for vk in vks {
                        map.insert((vk, mods), a.action);
                    }
                }
            }
        }
        Self { map }
    }

    /// The customisable table first; tab numbers, Escape, zoom in the browser UI and the
    /// keyboard's browser keys keep their fixed meaning.
    pub fn lookup(&self, vk: u32, ctrl: bool, shift: bool, alt: bool, chrome_surface: bool) -> Option<Shortcut> {
        let mods = u8::from(ctrl) * CTRL | u8::from(shift) * SHIFT | u8::from(alt) * ALT;
        if let Some(s) = self.map.get(&(vk, mods)) {
            return Some(*s);
        }
        let fixed = map(vk, ctrl, shift, alt, chrome_surface)?;
        let browser_key = matches!(vk, VK_BROWSER_BACK | VK_BROWSER_FORWARD | VK_BROWSER_REFRESH | VK_BROWSER_HOME);
        match fixed {
            Shortcut::SelectTab(_) | Shortcut::Escape | Shortcut::ZoomIn | Shortcut::ZoomOut | Shortcut::ZoomReset => Some(fixed),
            _ if browser_key => Some(fixed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    #[test]
    fn defaults_match_the_classic_table() {
        let b = Bindings::build(&Default::default());
        assert_eq!(b.lookup('T' as u32, true, false, false, false), Some(Shortcut::NewTab));
        assert_eq!(b.lookup('T' as u32, true, true, false, false), Some(Shortcut::ReopenClosedTab));
        assert_eq!(b.lookup(VK_LEFT, false, false, true, false), Some(Shortcut::Back));
        assert_eq!(b.lookup(0x33, true, false, false, false), Some(Shortcut::SelectTab(3)));
        assert_eq!(b.lookup(VK_OEM_PLUS, true, false, false, true), Some(Shortcut::ZoomIn));
    }

    #[test]
    fn overrides_replace_defaults() {
        let mut o = std::collections::BTreeMap::new();
        o.insert("new_tab".to_string(), vec!["Ctrl+Shift+Y".to_string()]);
        let b = Bindings::build(&o);
        assert_eq!(b.lookup('T' as u32, true, false, false, false), None);
        assert_eq!(b.lookup('Y' as u32, true, true, false, false), Some(Shortcut::NewTab));
    }

    #[test]
    fn parses_names() {
        assert_eq!(parse_combo("Ctrl+PgDn"), Some((vec![VK_NEXT], CTRL)));
        assert_eq!(parse_combo("Alt+F4"), Some((vec![VK_F4], ALT)));
        assert_eq!(parse_combo("Ctrl+Plus").map(|c| c.0.len()), Some(2));
        assert!(parse_combo("Ctrl+Nope").is_none());
    }
}
