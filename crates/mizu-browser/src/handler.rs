//! Engine callbacks → browser. Decisions are made here against [`crate::hot`]; everything else
//! is queued for [`crate::app`].

use crate::hot::{self, Role};
use crate::queue::{self, Queued};
use crate::shortcuts;
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

    fn on_navigation_starting(&self, key: ViewKey, req: &NavigationRequest) -> NavigationDecision {
        let uri = req.uri.as_str();
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
                h.gm_tokens.remove(&key);
            });
            if let Some(v) = hot::view(key) {
                v.set_document_script(None);
            }
            return NavigationDecision::Allow;
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

            let script = hot::with(|h| {
                h.top_urls.insert(key, uri.to_owned());
                if !req.redirect {
                    h.page_blocked.insert(key, 0);
                }
                crate::scripts::document_script(h, key, uri)
            })
            .flatten();
            if let Some(v) = hot::view(key) {
                v.set_document_script(script);
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
                h.stats.record(uri);
                h.stats_dirty = true;
                *h.page_blocked.entry(key).or_default() += 1;
            }
            (verdict, gpc)
        });
        let Some((verdict, gpc)) = decision else { return ResourceDecision::Continue };
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
            Verdict::Allow if gpc => ResourceDecision::ContinueWithHeaders(vec![("Sec-GPC".into(), "1".into())]),
            Verdict::Allow => ResourceDecision::Continue,
        }
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
        match shortcuts::map(vk, ctrl, shift, alt, chrome) {
            Some(s) => {
                queue::push(Queued::Shortcut(key, s));
                true
            }
            None => false,
        }
    }

    fn on_external_uri(&self, key: ViewKey, uri: &str) -> bool {
        // Private windows never hand links to other applications silently; the engine shows
        // its confirmation dialog in all cases, we only veto dangerous schemes.
        let lower = uri.to_ascii_lowercase();
        let dangerous = ["ms-msdt:", "search-ms:", "ms-officecmd:", "ms-cxh", "shell:"]
            .iter()
            .any(|p| lower.starts_with(p));
        if dangerous {
            tracing::warn!(key, uri, "blocked dangerous external protocol");
        }
        !dangerous
    }
}

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
