//! Links that open other applications (`steam://`, `tg://`, `zoom://`, `mailto:` …).
//!
//! Mizu replaces the engine's dialog with its own prompt: "Open Steam?" with the app's name
//! from Windows, and an "Always allow" choice remembered per site and scheme (never in
//! private windows). Schemes that have been used in attacks are never opened.

use crate::app::{App, ConfirmKind};
use mizu_engine::ViewKey;
use windows::Win32::System::Registry::{HKEY_CLASSES_ROOT, RRF_RT_REG_SZ, RegGetValueW};
use windows_core::HSTRING;

/// Schemes that have been abused to run code or reach local files.
pub fn is_dangerous(uri: &str) -> bool {
    let lower = uri.to_ascii_lowercase();
    ["ms-msdt:", "search-ms:", "ms-officecmd:", "ms-cxh", "shell:", "file:", "javascript:", "vbscript:", "ms-appinstaller:"]
        .iter()
        .any(|p| lower.starts_with(p))
}

pub fn scheme_of(uri: &str) -> Option<String> {
    let (scheme, _) = uri.split_once(':')?;
    let valid = !scheme.is_empty() && scheme.len() <= 64 && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c));
    valid.then(|| scheme.to_ascii_lowercase())
}

fn reg_string(key: &str, name: Option<&str>) -> Option<String> {
    let mut buf = vec![0u16; 512];
    let mut size = (buf.len() * 2) as u32;
    let name = name.map(HSTRING::from).unwrap_or_default();
    // SAFETY: RegGetValueW writes at most `size` bytes into `buf`.
    let r = unsafe {
        RegGetValueW(HKEY_CLASSES_ROOT, &HSTRING::from(key), &name, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size))
    };
    if r.is_err() {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len])).filter(|s| !s.trim().is_empty())
}

/// The application's name as Windows knows it ("URL:Steam Protocol" → "Steam"), or `None`
/// when nothing handles the scheme.
pub fn app_name(scheme: &str) -> Option<String> {
    reg_string(scheme, Some("URL Protocol"))
        .or_else(|| reg_string(scheme, None))
        .or_else(|| reg_string(&format!(r"{scheme}\shell\open\command"), None))?;
    let label = reg_string(scheme, None).unwrap_or_default();
    let name = label.trim().trim_start_matches("URL:").trim_start_matches("URL ").trim();
    let name = name.strip_suffix(" Protocol").or_else(|| name.strip_suffix(" protocol")).unwrap_or(name).trim();
    let friendly = match scheme {
        "mailto" => "your email app".to_owned(),
        "tel" | "callto" => "your calling app".to_owned(),
        _ if name.is_empty() => scheme.to_owned(),
        _ => name.to_owned(),
    };
    Some(friendly)
}

pub fn launch(uri: &str) {
    crate::win::shell_open(uri);
}

impl App {
    pub fn on_external_uri(&mut self, key: ViewKey, uri: String, origin: String) {
        let Some(scheme) = scheme_of(&uri) else { return };
        let Some(crate::hot::Role::Tab { window, .. }) = crate::hot::role(key) else { return };
        let private = self.window(window).is_some_and(|w| w.private);
        let Some(app) = app_name(&scheme) else {
            self.toast_all(&format!("No app on this PC opens {scheme}: links"));
            return;
        };
        let allowed = !private && self.settings.privacy.external_apps.iter().any(|r| r.origin == origin && r.scheme == scheme);
        if allowed {
            launch(&uri);
            return;
        }
        let site = mizu_core::omnibox::host_of(&origin).unwrap_or_else(|| "This page".to_owned());
        let data = serde_json::json!({
            "title": format!("Open {app}?"),
            "message": format!("{site} wants to open a link in {app}."),
            "confirm": "Open",
            "cancel": "Cancel",
            "icon": "external",
            "primary": true,
            "can_skip": !private && !origin.is_empty(),
            "skip_label": format!("Always allow {site} to open {app}"),
        });
        self.pending_external = Some((uri, origin, scheme, private));
        self.show_confirm(window, ConfirmKind::OpenExternal, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemes_and_danger() {
        assert_eq!(scheme_of("steam://run/570").as_deref(), Some("steam"));
        assert_eq!(scheme_of("mailto:a@b.c").as_deref(), Some("mailto"));
        assert!(scheme_of("no scheme").is_none());
        assert!(is_dangerous("ms-msdt:/id PCWDiagnostic"));
        assert!(is_dangerous("SEARCH-MS:query=x"));
        assert!(!is_dangerous("tg://resolve?domain=x"));
    }
}
