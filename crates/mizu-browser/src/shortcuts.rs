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
