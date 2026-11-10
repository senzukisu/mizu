//! Native-side theme values: window background and the engine's pre-paint background, which
//! must match the HTML UI to avoid flashes. The full design system lives in `tokens.css`.

use mizu_settings::{Palette, Settings, Theme};

pub fn is_dark(settings: &Settings) -> bool {
    match settings.appearance.theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => crate::win::system_prefers_dark(),
    }
}

/// Opaque background of the browser frame (matches `--frame` in tokens.css).
pub fn frame_rgb(settings: &Settings, private: bool) -> [u8; 3] {
    let dark = is_dark(settings) || private;
    if private {
        return [0x1b, 0x17, 0x26];
    }
    match (settings.appearance.palette, dark) {
        (Palette::Mizu, false) => [0xe9, 0xef, 0xf5],
        (Palette::Mizu, true) => [0x14, 0x1a, 0x22],
        (Palette::Washi, false) => [0xf1, 0xed, 0xe6],
        (Palette::Washi, true) => [0x1d, 0x1b, 0x18],
        (Palette::Matcha, false) => [0xe9, 0xef, 0xe6],
        (Palette::Matcha, true) => [0x16, 0x1c, 0x17],
        (Palette::Sumi, false) => [0xee, 0xee, 0xee],
        (Palette::Sumi, true) => [0x0e, 0x0e, 0x0f],
        (Palette::Sakura, false) => [0xf6, 0xec, 0xee],
        (Palette::Sakura, true) => [0x1f, 0x17, 0x1a],
    }
}

/// Background for web content before it paints: the frame colour, so a tab that is still
/// starting blends with the UI (and with internal pages, which use the same colour).
pub fn page_rgba(settings: &Settings, private: bool) -> [u8; 4] {
    let [r, g, b] = frame_rgb(settings, private);
    [r, g, b, 0xff]
}

/// Browser-UI views are transparent when Mica is active so the backdrop shows through.
pub fn chrome_rgba(settings: &Settings, private: bool) -> [u8; 4] {
    if settings.appearance.translucency && !private {
        [0, 0, 0, 0]
    } else {
        let [r, g, b] = frame_rgb(settings, private);
        [r, g, b, 0xff]
    }
}

/// `<html>` attributes matching the current appearance, injected into internal pages before
/// they are served so they paint in the right theme immediately (no light→dark flash).
pub fn html_attrs(settings: &Settings) -> String {
    let a = &settings.appearance;
    let enum_str = |v: serde_json::Value| v.as_str().unwrap_or_default().to_owned();
    let accent = if a.accent.len() == 7 && a.accent.starts_with('#') { a.accent.as_str() } else { "#2f7de1" };
    format!(
        r#"data-theme="{}" data-palette="{}" data-density="{}" data-style="{}" data-motion="{}" style="--accent:{accent}""#,
        if is_dark(settings) { "dark" } else { "light" },
        enum_str(serde_json::to_value(a.palette).unwrap_or_default()),
        enum_str(serde_json::to_value(a.density).unwrap_or_default()),
        enum_str(serde_json::to_value(a.style).unwrap_or_default()),
        if a.animations { "on" } else { "off" },
    )
}

/// Settings plus derived values, as sent to UI surfaces.
pub fn ui_settings(settings: &Settings) -> serde_json::Value {
    let mut v = serde_json::to_value(settings).unwrap_or_default();
    v["_dark"] = serde_json::Value::Bool(is_dark(settings));
    v["_engine"] = serde_json::Value::String(settings.search.active_engine().name);
    v
}
