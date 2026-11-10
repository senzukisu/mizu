use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::resources::Resource;
use std::collections::HashSet;

/// Decision for one network request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Block,
    /// Serve this `data:` URL instead (a neutered script, a transparent pixel, …). Sites that
    /// expect e.g. an analytics object keep working while nothing reaches the tracker.
    Redirect(String),
    /// Load this URL instead (tracking parameters stripped by `$removeparam`).
    Rewrite(String),
}

impl Verdict {
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Block | Self::Redirect(_))
    }
}

/// CSS and scriptlets to apply to a document before page scripts run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CosmeticBundle {
    pub css: String,
    pub script: String,
}

impl CosmeticBundle {
    pub fn is_empty(&self) -> bool {
        self.css.is_empty() && self.script.is_empty()
    }
}

/// A compiled content-blocking engine plus the user's allowlist.
///
/// Building the engine from full lists takes a few hundred milliseconds, so it is done on a
/// worker thread and swapped in atomically by the browser; checks themselves take
/// microseconds and happen on the UI thread inside the engine's request hook.
pub struct ContentBlocker {
    engine: Engine,
    allowlist: HashSet<String>,
    rule_count: usize,
}

impl ContentBlocker {
    /// Compiles the given list texts (uBlock/ABP syntax; hosts files are detected and parsed).
    pub fn build(lists: &[String], resources: Vec<Resource>, allowlist: &[String]) -> Self {
        let mut set = FilterSet::new(false);
        let mut rule_count = 0;
        for text in lists {
            let opts = ParseOptions {
                format: detect_format(text),
                ..ParseOptions::default()
            };
            let before = rule_count;
            rule_count += text
                .lines()
                .filter(|l| {
                    let l = l.trim();
                    !l.is_empty() && !l.starts_with('!') && !l.starts_with('#') && !l.starts_with('[')
                })
                .count();
            let _ = before;
            set.add_filter_list(text.clone(), opts);
        }
        let mut engine = Engine::new_with_filter_set(set);
        engine.use_resources(resources);
        Self {
            engine,
            allowlist: normalize_hosts(allowlist),
            rule_count,
        }
    }

    pub fn empty() -> Self {
        Self::build(&[], Vec::new(), &[])
    }

    /// Approximate number of active rules (for display).
    pub fn rule_count(&self) -> usize {
        self.rule_count
    }

    pub fn set_allowlist(&mut self, hosts: &[String]) {
        self.allowlist = normalize_hosts(hosts);
    }

    /// Whether blocking is disabled for pages on `page_url` (exact host or parent domain).
    pub fn is_allowlisted(&self, page_url: &str) -> bool {
        let Some(host) = url::Url::parse(page_url).ok().and_then(|u| u.host_str().map(str::to_owned)) else {
            return false;
        };
        let mut h = host.as_str();
        loop {
            if self.allowlist.contains(h) {
                return true;
            }
            match h.split_once('.') {
                Some((_, rest)) if rest.contains('.') => h = rest,
                _ => return false,
            }
        }
    }

    /// Checks a sub-resource or document request.
    ///
    /// `request_type` uses uBlock names: `document`, `subdocument`, `script`, `stylesheet`,
    /// `image`, `media`, `font`, `xmlhttprequest`, `websocket`, `ping`, `other`, …
    pub fn check(&self, url: &str, source_url: &str, request_type: &str) -> Verdict {
        if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ws")) {
            return Verdict::Allow;
        }
        if !source_url.is_empty() && self.is_allowlisted(source_url) {
            return Verdict::Allow;
        }
        let Ok(request) = Request::new(url, source_url, request_type, "GET") else {
            return Verdict::Allow;
        };
        let result = self.engine.check_network_request(&request);
        if result.should_block() {
            return match result.redirect {
                Some(data_url) => Verdict::Redirect(data_url),
                None => Verdict::Block,
            };
        }
        match result.rewritten_url {
            Some(u) if u != url => Verdict::Rewrite(u),
            _ => Verdict::Allow,
        }
    }

    /// Element-hiding CSS and scriptlets for a top-level document.
    pub fn cosmetic(&self, page_url: &str) -> CosmeticBundle {
        if self.is_allowlisted(page_url) || !page_url.starts_with("http") {
            return CosmeticBundle::default();
        }
        let res = self.engine.url_cosmetic_resources(page_url);
        let mut selectors: Vec<&String> = res.hide_selectors.iter().collect();
        selectors.sort_unstable();
        // One rule per selector: a single selector the engine does not understand must not
        // invalidate the whole stylesheet.
        let mut css = String::with_capacity(selectors.len() * 48);
        for sel in selectors {
            if sel.contains('{') || sel.contains('}') || sel.contains("</") {
                continue;
            }
            css.push_str(sel);
            css.push_str("{display:none!important}\n");
        }
        CosmeticBundle {
            css,
            script: res.injected_script,
        }
    }
}

fn normalize_hosts(hosts: &[String]) -> HashSet<String> {
    hosts
        .iter()
        .map(|h| h.trim().trim_start_matches("www.").to_ascii_lowercase())
        .filter(|h| !h.is_empty())
        .collect()
}

/// Hosts files (`0.0.0.0 example.com`) are parsed differently from filter syntax.
fn detect_format(text: &str) -> adblock::lists::FilterFormat {
    let sample = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!'))
        .take(20);
    let mut hosts_like = 0;
    let mut total = 0;
    for line in sample {
        total += 1;
        if line.starts_with("0.0.0.0 ") || line.starts_with("127.0.0.1 ") {
            hosts_like += 1;
        }
    }
    if total > 0 && hosts_like * 2 > total {
        adblock::lists::FilterFormat::Hosts
    } else {
        adblock::lists::FilterFormat::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::builtin_resources;

    fn blocker(rules: &str) -> ContentBlocker {
        ContentBlocker::build(&[rules.to_owned()], builtin_resources(), &[])
    }

    #[test]
    fn blocks_third_party_trackers() {
        let b = blocker("||tracker.example^$third-party\n");
        assert_eq!(
            b.check("https://tracker.example/p.gif", "https://news.example/", "image"),
            Verdict::Block
        );
        // First-party use is allowed.
        assert_eq!(
            b.check("https://tracker.example/p.gif", "https://tracker.example/", "image"),
            Verdict::Allow
        );
    }

    #[test]
    fn honours_exceptions() {
        let b = blocker("||ads.example^\n@@||ads.example/allowed.js\n");
        assert_eq!(b.check("https://ads.example/x.js", "https://a.example/", "script"), Verdict::Block);
        assert_eq!(
            b.check("https://ads.example/allowed.js", "https://a.example/", "script"),
            Verdict::Allow
        );
    }

    #[test]
    fn redirects_to_neutered_resources() {
        let b = blocker("||analytics.example/ga.js$script,redirect=noop.js\n");
        match b.check("https://analytics.example/ga.js", "https://a.example/", "script") {
            Verdict::Redirect(data) => assert!(data.starts_with("data:application/javascript;base64,")),
            other => panic!("expected redirect, got {other:?}"),
        }
    }

    #[test]
    fn strips_tracking_parameters() {
        let b = blocker("$removeparam=utm_source\n");
        assert_eq!(
            b.check("https://a.example/page?utm_source=x&id=1", "https://b.example/", "xmlhttprequest"),
            Verdict::Rewrite("https://a.example/page?id=1".into())
        );
    }

    #[test]
    fn allowlist_disables_blocking() {
        let mut b = blocker("||ads.example^\n");
        b.set_allowlist(&["shop.example".into()]);
        assert_eq!(b.check("https://ads.example/a.js", "https://www.shop.example/", "script"), Verdict::Allow);
        assert_eq!(b.check("https://ads.example/a.js", "https://other.example/", "script"), Verdict::Block);
    }

    #[test]
    fn cosmetic_filters() {
        let b = blocker("example.com##.ad-banner\nexample.com##+js(set-constant, adsEnabled, false)\n");
        let c = b.cosmetic("https://example.com/article");
        assert!(c.css.contains(".ad-banner{display:none!important}"));
        assert!(!c.script.is_empty(), "scriptlet should be resolved from bundled resources");
        assert!(b.cosmetic("https://other.com/").css.is_empty());
    }

    #[test]
    fn parses_hosts_files() {
        let b = blocker("# hosts\n0.0.0.0 bad.example\n0.0.0.0 worse.example\n");
        assert_eq!(b.check("https://bad.example/x", "https://a.example/", "script"), Verdict::Block);
    }

    #[test]
    fn bundled_list_compiles_and_blocks() {
        let text = include_str!("../../../assets/filters/mizu-shield.txt");
        let b = blocker(text);
        assert!(b.rule_count() > 100);
        assert_eq!(
            b.check("https://www.google-analytics.com/analytics.js", "https://site.example/", "script"),
            Verdict::Block
        );
        assert_eq!(
            b.check("https://securepubads.g.doubleclick.net/tag/js/gpt.js", "https://site.example/", "script"),
            Verdict::Block
        );
        assert_eq!(b.check("https://example.com/app.js", "https://example.com/", "script"), Verdict::Allow);
    }

    #[test]
    fn ignores_non_web_urls() {
        let b = blocker("*\n");
        assert_eq!(b.check("mizu://settings/", "", "document"), Verdict::Allow);
        assert_eq!(b.check("data:text/plain,hi", "https://a.example/", "image"), Verdict::Allow);
    }
}
