//! Engine callbacks → browser. Decisions are made here against [`crate::hot`]; everything else
//! is queued for [`crate::app`].

use crate::hot::{self, Role};
use crate::queue::{self, Queued};
use base64::Engine as _;
use mizu_core::internal::is_internal_url;
use mizu_engine::*;
use mizu_privacy::Verdict;

pub struct Dispatcher;

const CHROME_URL: &str = "mizu://chrome/";
const FLYOUT_URL: &str = "mizu://flyout/";
const SIDEBAR_URL: &str = "mizu://chrome/?sidebar";

fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_ascii_lowercase()))
        .unwrap_or_default()
}

/// Schemes a tab may load besides http(s) and internal pages.
/// Schemes owned by the engine itself; everything else that isn't web or Mizu is an app link.
fn is_external_app_scheme(uri: &str) -> bool {
    let Some(scheme) = crate::external::scheme_of(uri) else { return false };
    !matches!(
        scheme.as_str(),
        "http" | "https" | "mizu" | "about" | "data" | "blob" | "file" | "chrome" | "edge" | "devtools" | "view-source"
            | "chrome-extension" | "extension" | "chrome-untrusted" | "edge-untrusted" | "javascript" | "ws" | "wss"
    )
}

fn allowed_other_scheme(uri: &str) -> bool {
    ["about:", "data:", "blob:", "file:", "chrome-extension:", "extension:"]
        .iter()
        .any(|p| uri.starts_with(p))
}

fn decode_data_url(data_url: &str) -> Option<(String, Vec<u8>)> {
    let rest = data_url.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    let mime = meta.trim_end_matches(";base64").to_owned();
    let body = if meta.ends_with(";base64") {
        base64::prelude::BASE64_STANDARD.decode(payload).ok()?
    } else {
        payload.as_bytes().to_vec()
    };
    Some((mime, body))
}

/// Rewrites the `<html …>` tag of an internal document with the current theme attributes.
fn themed_html(key: ViewKey, body: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(body) else { return body.to_vec() };
    let Some((attrs, private)) = hot::with(|h| (h.theme_attrs.clone(), h.private.contains(&key))) else {
        return body.to_vec();
    };
    let Some(start) = text.find("<html") else { return body.to_vec() };
    let Some(len) = text[start..].find('>') else { return body.to_vec() };
    let private = if private { " data-private" } else { "" };
    format!("{}<html {attrs}{private}>{}", &text[..start], &text[start + len + 1..]).into_bytes()
}

impl ViewHandler for Dispatcher {
    fn on_event(&self, key: ViewKey, event: ViewEvent) {
        queue::push(Queued::View(key, event));
    }

    fn context_items(&self, key: ViewKey, t: &mizu_engine::ContextTarget) -> Vec<mizu_engine::ContextItem> {
        if !matches!(hot::role(key), Some(Role::Tab { .. })) {
            return Vec::new();
        }
        let Some((Some(player), ru)) = hot::with(|h| (h.media_player.clone(), h.ui_ru)) else { return Vec::new() };
        let mut items = Vec::new();
        if t.media && crate::players::media_address(&t.source, &t.page).is_some() {
            let label = if ru { format!("Открыть видео в {}", player.name) } else { format!("Open video in {}", player.name) };
            items.push(mizu_engine::ContextItem { id: 1, label });
        }
        if crate::players::is_media_link(&t.link) {
            let label = if ru { format!("Открыть ссылку в {}", player.name) } else { format!("Open link in {}", player.name) };
            items.push(mizu_engine::ContextItem { id: 2, label });
        }
        items
    }

    fn on_page_menu(&self, key: ViewKey, x: i32, y: i32, items: Vec<mizu_engine::PageMenuItem>, menu: mizu_engine::PageMenu) -> bool {
        let Some(Role::Tab { window, tab }) = hot::role(key) else { return false };
        if !hot::with(|h| h.mizu_page_menu).unwrap_or(false) || items.is_empty() {
            return false;
        }
        queue::with_app(move |app| app.show_page_menu(window, tab, x, y, &items, menu));
        true
    }

    fn on_context_item(&self, _key: ViewKey, id: u32, t: &mizu_engine::ContextTarget) {
        let Some(Some(player)) = hot::with(|h| h.media_player.clone()) else { return };
        let url = match id {
            1 => crate::players::media_address(&t.source, &t.page),
            2 => Some(t.link.clone()),
            _ => None,
        };
        if let Some(url) = url {
            crate::players::open(&player, &url);
        }
    }

    fn on_navigation_starting(&self, key: ViewKey, req: &NavigationRequest) -> NavigationDecision {
        let uri = req.uri.as_str();
        // A shortcut being recorded in Settings ends when the page goes away.
        hot::with(|h| h.capturing_keys.remove(&key));
        match hot::role(key) {
            Some(Role::Chrome(_)) => {
                return if uri == CHROME_URL || uri == SIDEBAR_URL { NavigationDecision::Allow } else { NavigationDecision::Cancel };
            }
            Some(Role::Flyout(_)) => {
                return if uri == FLYOUT_URL { NavigationDecision::Allow } else { NavigationDecision::Cancel };
            }
            Some(Role::Tab { .. }) => {}
            None => return NavigationDecision::Allow,
        }

        // ── Internal pages: only the browser (or history navigation) may open them ──
        if is_internal_url(uri) {
            let host = host_of(uri);
            let allowed = hot::with(|h| {
                let granted = h.internal_allowed.remove(&key);
                let from_internal = h.top_urls.get(&key).is_some_and(|u| is_internal_url(u));
                granted || from_internal || req.history_navigation
            })
            .unwrap_or(false);
            if !allowed || host == "chrome" || host == "flyout" {
                tracing::warn!(uri, "blocked navigation to internal page from web content");
                return NavigationDecision::Cancel;
            }
            hot::with(|h| {
                h.top_urls.insert(key, uri.to_owned());
                h.page_blocked.remove(&key);
                h.fp_report.remove(&key);
                h.gm_tokens.remove(&key);
            });
            if let Some(v) = hot::view(key) {
                v.set_document_script(None);
            }
            return NavigationDecision::Allow;
        }

        if uri.starts_with("http://") {
            let upgrade = hot::with(|h| {
                if !h.https_first {
                    return None;
                }
                let target = crate::https::upgrade_target(uri)?;
                let site = crate::https::site(&host_of(uri));
                if h.http_allowed.contains(&site) {
                    return None;
                }
                // The HTTPS page sent us back to HTTP: the site wants HTTP; allow it this session.
                if req.redirect && h.https_pending.get(&key).is_some_and(|p| crate::https::site(&host_of(p)) == site) {
                    h.https_pending.remove(&key);
                    h.http_allowed.insert(site);
                    return None;
                }
                h.https_pending.insert(key, uri.to_owned());
                Some(target)
            })
            .flatten();
            if let Some(target) = upgrade {
                queue::with_app(move |_| {
                    if let Some(v) = hot::view(key) {
                        v.navigate(&target);
                    }
                });
                return NavigationDecision::Cancel;
            }
        }

        if uri.starts_with("http://") || uri.starts_with("https://") {
            // Userscript installation (Greasy Fork, OpenUserJS, …).
            if mizu_userscripts::is_userscript_url(uri) {
                queue::push(Queued::InstallScript { key, url: uri.to_owned() });
                return NavigationDecision::Cancel;
            }

            // Document-level blocking (badware, known malicious hosts).
            let blocked = hot::with(|h| {
                let blocker = h.blocker.as_ref()?;
                if !h.content_blocking
                    || h.document_exceptions.contains(&host_of(uri))
                    || crate::shield::allowlisted(&h.allowlist, uri)
                {
                    return None;
                }
                (blocker.check(uri, uri, "document") == Verdict::Block).then_some(())
            })
            .flatten()
            .is_some();
            if blocked {
                queue::push(Queued::ShowBlocked { key, url: uri.to_owned() });
                return NavigationDecision::Cancel;
            }

            let t_script = std::time::Instant::now();
            let script = hot::with(|h| {
                h.top_urls.insert(key, uri.to_owned());
                if !req.redirect {
                    h.page_blocked.insert(key, 0);
                    h.fp_report.remove(&key);
                    h.nav_started.insert(key, std::time::Instant::now());
                }
                crate::scripts::document_script(h, key, uri)
            })
            .flatten();
            let len = script.as_ref().map_or(0, String::len);
            if let Some(v) = hot::view(key) {
                v.set_document_script(script);
            }
            let ms = t_script.elapsed().as_millis();
            if ms >= 20 {
                tracing::warn!(host = %host_of(uri), ms, script_kb = len / 1024, "slow document script");
            }
            return NavigationDecision::Allow;
        }

        if allowed_other_scheme(uri) {
            hot::with(|h| {
                h.top_urls.insert(key, uri.to_owned());
                h.gm_tokens.remove(&key);
            });
            if let Some(v) = hot::view(key) {
                v.set_document_script(None);
            }
            return NavigationDecision::Allow;
        }

        // Links to other applications (mailto:, steam:, tg:, zoom: …) go to Mizu's prompt.
        if is_external_app_scheme(uri) {
            if crate::external::is_dangerous(uri) {
                tracing::warn!(uri, "blocked dangerous external protocol");
            } else {
                let origin = hot::with(|h| h.top_urls.get(&key).cloned())
                    .flatten()
                    .and_then(|u| mizu_core::omnibox::origin_of(&u))
                    .unwrap_or_default();
                queue::push(Queued::ExternalUri { key, uri: uri.to_owned(), origin });
            }
            return NavigationDecision::Cancel;
        }

        // edge://, chrome:// and unknown schemes expose engine internals not meant for Mizu.
        tracing::info!(uri, "refused navigation to unsupported scheme");
        NavigationDecision::Cancel
    }

    fn on_frame_navigation_starting(&self, _key: ViewKey, uri: &str) -> NavigationDecision {
        if is_internal_url(uri) {
            NavigationDecision::Cancel
        } else {
            NavigationDecision::Allow
        }
    }

    fn on_resource_request(&self, key: ViewKey, req: &ResourceRequest) -> ResourceDecision {
        let started = std::time::Instant::now();
        let decision = resource_decision(key, req);
        let ms = started.elapsed().as_millis();
        if ms >= 10 {
            tracing::warn!(ms, kind = req.resource_type, "slow request check");
        }
        decision
    }

    fn on_new_window(&self, key: ViewKey, request: NewWindowRequest) {
        queue::push(Queued::NewWindow(key, request));
    }

    fn on_permission(&self, key: ViewKey, request: PermissionRequest) {
        queue::push(Queued::Permission(key, request));
    }

    fn on_download(&self, key: ViewKey, download: Download) {
        queue::push(Queued::Download(key, download));
    }

    fn on_web_message(&self, key: ViewKey, source: &str, json: &str) {
        if json.len() > 8 * 1024 * 1024 {
            tracing::warn!("dropping oversized web message");
            return;
        }
        queue::push(Queued::Message {
            key,
            source: source.to_owned(),
            json: json.to_owned(),
        });
    }

    fn on_accelerator(&self, key: ViewKey, vk: u32, ctrl: bool, shift: bool, alt: bool) -> bool {
        let chrome = matches!(hot::role(key), Some(Role::Chrome(_) | Role::Flyout(_)));
        let found = hot::with(|h| {
            if h.capturing_keys.contains(&key) {
                None
            } else {
                h.bindings.lookup(vk, ctrl, shift, alt, chrome)
            }
        })
        .flatten();
        match found {
            Some(s) => {
                queue::push(Queued::Shortcut(key, s));
                true
            }
            None => false,
        }
    }

    fn on_external_uri(&self, key: ViewKey, uri: &str, origin: &str) -> bool {
        // Mizu asks with its own prompt (and remembers "always allow" per site), so the
        // engine's dialog is always cancelled. Schemes used in attacks are never opened.
        if crate::external::is_dangerous(uri) {
            tracing::warn!(key, uri, "blocked dangerous external protocol");
        } else {
            queue::push(Queued::ExternalUri { key, uri: uri.to_owned(), origin: origin.to_owned() });
        }
        false
    }
}


fn resource_decision(key: ViewKey, req: &ResourceRequest) -> ResourceDecision {
    let uri = req.uri.as_str();


    // ── Internal assets ──
    if is_internal_url(uri) {
        let permitted = match hot::role(key) {
            Some(Role::Chrome(_) | Role::Flyout(_)) => true,
            Some(Role::Tab { .. }) => {
                hot::with(|h| h.top_urls.get(&key).is_some_and(|u| is_internal_url(u))).unwrap_or(false)
            }
            None => false,
        };
        let asset = permitted.then(|| mizu_ui::assets::resolve(uri)).flatten();
        return match asset {
            Some(a) => ResourceDecision::Respond {
                status: 200,
                mime: a.mime.to_owned(),
                body: if a.mime.starts_with("text/html") { themed_html(key, a.body) } else { a.body.to_vec() },
                extra_headers: format!(
                    "Content-Security-Policy: {}\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n",
                    mizu_ui::assets::CSP
                ),
            },
            None => ResourceDecision::Block,
        };
    }

    if !matches!(hot::role(key), Some(Role::Tab { .. })) {
        return ResourceDecision::Continue;
    }

    let decision = hot::with(|h| {
        let source = h.top_urls.get(&key).cloned().unwrap_or_default();
        let gpc = h.global_privacy_control;
        let reduce_hints = h.fingerprinting && {
            let site = crate::https::site(&host_of(&source));
            !site.is_empty() && !h.fp_exceptions.contains(&site)
        };
        let mut verdict = Verdict::Allow;
        if let (true, Some(blocker)) = (h.content_blocking, h.blocker.as_ref()) {
            if !is_internal_url(&source) && !crate::shield::allowlisted(&h.allowlist, &source) {
                // The main document was already checked at navigation start.
                let rtype = if req.resource_type == "document" {
                    if uri == source { "" } else { "subdocument" }
                } else {
                    req.resource_type
                };
                if !rtype.is_empty() {
                    verdict = blocker.check(uri, &source, rtype);
                }
            }
        }
        if verdict.is_blocking() {
            h.stats.record(uri, &source);
            h.stats_dirty = true;
            *h.page_blocked.entry(key).or_default() += 1;
        }
        (verdict, gpc, reduce_hints)
    });
    let Some((verdict, gpc, reduce_hints)) = decision else { return ResourceDecision::Continue };
    match verdict {
        Verdict::Block => ResourceDecision::Block,
        Verdict::Redirect(data_url) => match decode_data_url(&data_url) {
            Some((mime, body)) => ResourceDecision::Respond {
                status: 200,
                mime,
                body,
                extra_headers: String::new(),
            },
            None => ResourceDecision::Block,
        },
        Verdict::Rewrite(u) => ResourceDecision::Rewrite(u),
        Verdict::Allow if gpc || reduce_hints => {
            let mut headers: Vec<(String, String)> = Vec::new();
            if gpc {
                headers.push(("Sec-GPC".into(), "1".into()));
            }
            if reduce_hints {
                for name in HIGH_ENTROPY_HINTS {
                    headers.push(((*name).into(), String::new()));
                }
            }
            ResourceDecision::ContinueWithHeaders(headers)
        }
        Verdict::Allow => ResourceDecision::Continue,
    }
}

/// Client Hints that identify the exact device and build; removed under fingerprinting defence.
const HIGH_ENTROPY_HINTS: &[&str] = &[
    "Sec-CH-UA-Full-Version-List",
    "Sec-CH-UA-Full-Version",
    "Sec-CH-UA-Platform-Version",
    "Sec-CH-UA-Model",
    "Sec-CH-UA-Arch",
    "Sec-CH-UA-Bitness",
    "Sec-CH-UA-WoW64",
    "Sec-CH-UA-Form-Factors",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_data_urls() {
        let (mime, body) = decode_data_url("data:application/javascript;base64,KGZ1bmN0aW9uKCl7fSkoKTs=").unwrap();
        assert_eq!(mime, "application/javascript");
        assert_eq!(body, b"(function(){})();");
        assert!(decode_data_url("https://x").is_none());
    }

    #[test]
    fn schemes() {
        assert!(allowed_other_scheme("about:blank"));
        assert!(allowed_other_scheme("chrome-extension://abc/options.html"));
        assert!(!allowed_other_scheme("edge://settings"));
        assert!(!allowed_other_scheme("chrome://flags"));
    }
}
