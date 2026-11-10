//! Embedded UI assets served under `mizu://`.
//!
//! URL layout: `mizu://<host>/` serves the host's HTML document, `mizu://<host>/<file>` a file of
//! that surface, and `mizu://<host>/_/<file>` a shared file (design tokens, bridge, icons).
//! Everything is compiled into the binary – nothing is read from disk at runtime, so internal
//! pages cannot be tampered with by modifying files next to the executable.

pub struct Asset {
    pub mime: &'static str,
    pub body: &'static [u8],
}

macro_rules! asset {
    ($path:literal) => {
        include_bytes!(concat!("../../../assets/ui/", $path)) as &'static [u8]
    };
}

fn mime_for(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn shared(file: &str) -> Option<&'static [u8]> {
    Some(match file {
        "tokens.css" => asset!("shared/tokens.css"),
        "pages.css" => asset!("shared/pages.css"),
        "bridge.js" => asset!("shared/bridge.js"),
        "theme.js" => asset!("shared/theme.js"),
        "icons.js" => asset!("shared/icons.js"),
        "logo.svg" => asset!("shared/logo.svg"),
        _ => return None,
    })
}

fn surface(host: &str, file: &str) -> Option<&'static [u8]> {
    Some(match (host, file) {
        ("chrome", "") => asset!("chrome/index.html"),
        ("chrome", "chrome.css") => asset!("chrome/chrome.css"),
        ("chrome", "chrome.js") => asset!("chrome/chrome.js"),
        ("flyout", "") => asset!("flyout/index.html"),
        ("flyout", "flyout.css") => asset!("flyout/flyout.css"),
        ("flyout", "flyout.js") => asset!("flyout/flyout.js"),
        ("newtab", "") => asset!("pages/newtab.html"),
        ("newtab", "page.js") => asset!("pages/newtab.js"),
        ("settings", "") => asset!("pages/settings.html"),
        ("settings", "page.js") => asset!("pages/settings.js"),
        ("history", "") => asset!("pages/history.html"),
        ("history", "page.js") => asset!("pages/history.js"),
        ("bookmarks", "") => asset!("pages/bookmarks.html"),
        ("bookmarks", "page.js") => asset!("pages/bookmarks.js"),
        ("downloads", "") => asset!("pages/downloads.html"),
        ("downloads", "page.js") => asset!("pages/downloads.js"),
        ("about", "") => asset!("pages/about.html"),
        ("about", "page.js") => asset!("pages/about.js"),
        ("media", "") => asset!("pages/media.html"),
        ("media", "page.js") => asset!("pages/media.js"),
        ("shield", "") => asset!("pages/shield.html"),
        ("shield", "page.js") => asset!("pages/shield.js"),
        ("scripts", "") => asset!("pages/scripts.html"),
        ("scripts", "page.js") => asset!("pages/scripts.js"),
        ("blocked", "") => asset!("pages/blocked.html"),
        ("blocked", "page.js") => asset!("pages/blocked.js"),
        _ => return None,
    })
}

/// Resolves a `mizu://host/path` URL to an embedded asset.
pub fn resolve(url: &str) -> Option<Asset> {
    let rest = url.strip_prefix("mizu://")?;
    let rest = rest.split(['?', '#']).next().unwrap_or(rest);
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.to_ascii_lowercase();
    if path.contains("..") || path.contains('\\') {
        return None;
    }
    let (body, name) = if let Some(file) = path.strip_prefix("_/") {
        (shared(file)?, file)
    } else {
        (surface(&host, path)?, if path.is_empty() { "index.html" } else { path })
    };
    Some(Asset { mime: mime_for(name), body })
}

/// Strict Content Security Policy for every internal document: only embedded scripts run, no
/// network access, no framing. Favicons and thumbnails arrive as `data:` URLs from the browser.
pub const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
    img-src 'self' data:; font-src 'self'; media-src 'self' data: blob:; connect-src 'none'; \
    frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_assets() {
        assert!(resolve("mizu://chrome/").unwrap().mime.starts_with("text/html"));
        assert!(resolve("mizu://settings/").is_some());
        assert!(resolve("mizu://settings/page.js").unwrap().mime.starts_with("text/javascript"));
        assert!(resolve("mizu://history/_/tokens.css").unwrap().mime.starts_with("text/css"));
        assert!(resolve("mizu://newtab/?q=1").is_some());
    }

    #[test]
    fn rejects_unknown_and_traversal() {
        assert!(resolve("mizu://settings/../chrome/chrome.js").is_none());
        assert!(resolve("mizu://nope/").is_none());
        assert!(resolve("https://settings/").is_none());
        assert!(resolve("mizu://settings/_/secret").is_none());
    }
}
