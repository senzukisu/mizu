//! Address-bar input classification.
//!
//! The omnibox accepts both URLs and search queries. The heuristics below follow
//! what users expect from mainstream browsers: `example.com` navigates,
//! `rust lifetimes` searches, `localhost:8080` navigates, `javascript:` never runs.

use url::Url;

/// What the browser should do with text typed into the address bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmniboxAction {
    Navigate(String),
    Search(String),
}

/// Schemes a user may type explicitly. `javascript:` is deliberately absent: running
/// script typed into the address bar is a classic social-engineering vector.
const TYPABLE_SCHEMES: &[&str] = &["http", "https", "file", "mizu", "about", "data"];

pub fn classify(input: &str) -> Option<OmniboxAction> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }

    if let Some(url) = explicit_scheme(text) {
        return Some(match url {
            Some(u) => OmniboxAction::Navigate(u),
            None => OmniboxAction::Search(text.to_owned()),
        });
    }

    if text.chars().any(char::is_whitespace) {
        return Some(OmniboxAction::Search(text.to_owned()));
    }

    if looks_like_host(text) {
        let scheme = if is_local_host(text) { "http" } else { "https" };
        if let Ok(u) = Url::parse(&format!("{scheme}://{text}")) {
            return Some(OmniboxAction::Navigate(u.to_string()));
        }
    }

    Some(OmniboxAction::Search(text.to_owned()))
}

/// Returns `Some(Some(url))` for input with a typable scheme, `Some(None)` for input whose
/// scheme must not be navigated to, and `None` if the input has no scheme at all.
fn explicit_scheme(text: &str) -> Option<Option<String>> {
    let colon = text.find(':')?;
    let scheme = &text[..colon];
    if scheme.is_empty()
        || !scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        || !scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    {
        return None;
    }
    let lower = scheme.to_ascii_lowercase();
    // `localhost:3000` or `example.com:8080/path` – a port, not a scheme.
    let rest = &text[colon + 1..];
    if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    if TYPABLE_SCHEMES.contains(&lower.as_str()) {
        return Some(Url::parse(text).ok().map(|u| u.to_string()));
    }
    // `javascript:…`, `foo:bar` – never navigated, treated as a search.
    Some(None)
}

fn host_part(text: &str) -> &str {
    let end = text.find(['/', '?', '#']).unwrap_or(text.len());
    let authority = &text[..end];
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    if authority.starts_with('[') {
        return authority.split_once(']').map_or(authority, |(h, _)| h);
    }
    authority.split(':').next().unwrap_or(authority)
}

fn is_local_host(text: &str) -> bool {
    let host = host_part(text).to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".test")
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|ip| ip.is_private() || ip.is_loopback())
}

fn looks_like_host(text: &str) -> bool {
    let host = host_part(text);
    if host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    if let Some(v6) = host.strip_prefix('[') {
        return v6.parse::<std::net::Ipv6Addr>().is_ok();
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return false;
    }
    let valid_label =
        |l: &&str| l.chars().all(|c| c.is_alphanumeric() || c == '-') && !l.starts_with('-');
    if !labels.iter().all(valid_label) {
        return false;
    }
    let tld = labels[labels.len() - 1];
    tld.chars().count() >= 2 && tld.chars().all(char::is_alphabetic)
}

/// Builds a search URL from a template containing `{searchTerms}`.
pub fn search_url(template: &str, query: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    template.replace("{searchTerms}", &encoded)
}

/// Resolves omnibox input to the URL that should be loaded.
pub fn resolve(input: &str, search_template: &str) -> Option<String> {
    Some(match classify(input)? {
        OmniboxAction::Navigate(u) => u,
        OmniboxAction::Search(q) => search_url(search_template, &q),
    })
}

/// A short, human-friendly form of a URL for display (host + path without scheme noise).
pub fn display_url(raw: &str) -> String {
    match Url::parse(raw) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => {
            let host = u.host_str().unwrap_or_default();
            let host = host.strip_prefix("www.").unwrap_or(host);
            let mut out = host.to_owned();
            if u.path() != "/" {
                out.push_str(u.path());
            }
            if let Some(q) = u.query() {
                out.push('?');
                out.push_str(q);
            }
            out
        }
        _ => raw.to_owned(),
    }
}

/// Host of a URL, used for display and grouping. Not a security boundary.
pub fn host_of(raw: &str) -> Option<String> {
    Url::parse(raw).ok()?.host_str().map(str::to_owned)
}

/// Origin (`scheme://host[:port]`) of a URL, used as the key for site permissions.
pub fn origin_of(raw: &str) -> Option<String> {
    let u = Url::parse(raw).ok()?;
    match u.origin() {
        url::Origin::Tuple(..) => Some(u.origin().ascii_serialization()),
        url::Origin::Opaque(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nav(s: &str) -> Option<OmniboxAction> {
        Some(OmniboxAction::Navigate(s.into()))
    }
    fn search(s: &str) -> Option<OmniboxAction> {
        Some(OmniboxAction::Search(s.into()))
    }

    #[test]
    fn classifies_urls() {
        assert_eq!(classify("example.com"), nav("https://example.com/"));
        assert_eq!(
            classify("github.com/rust-lang/rust"),
            nav("https://github.com/rust-lang/rust")
        );
        assert_eq!(
            classify("https://example.com/a?b=c"),
            nav("https://example.com/a?b=c")
        );
        assert_eq!(classify("localhost:8080"), nav("http://localhost:8080/"));
        assert_eq!(classify("192.168.1.1"), nav("http://192.168.1.1/"));
        assert_eq!(classify("8.8.8.8"), nav("https://8.8.8.8/"));
        assert_eq!(classify("example.com:8443/x"), nav("https://example.com:8443/x"));
        assert_eq!(classify("mizu://settings"), nav("mizu://settings"));
        assert_eq!(classify("about:blank"), nav("about:blank"));
        assert_eq!(classify("пример.рф"), nav("https://xn--e1afmkfd.xn--p1ai/"));
    }

    #[test]
    fn classifies_searches() {
        assert_eq!(classify("rust lifetimes"), search("rust lifetimes"));
        assert_eq!(classify("hello"), search("hello"));
        assert_eq!(classify("3.14"), search("3.14"));
        assert_eq!(classify("what is example.com"), search("what is example.com"));
        assert_eq!(classify("c++:templates"), search("c++:templates"));
        assert_eq!(classify("   "), None);
    }

    #[test]
    fn never_runs_javascript() {
        assert_eq!(classify("javascript:alert(1)"), search("javascript:alert(1)"));
        assert_eq!(classify("JavaScript:alert(1)"), search("JavaScript:alert(1)"));
    }

    #[test]
    fn builds_search_urls() {
        assert_eq!(
            search_url("https://duckduckgo.com/?q={searchTerms}", "a b&c"),
            "https://duckduckgo.com/?q=a+b%26c"
        );
    }

    #[test]
    fn display_and_origin() {
        assert_eq!(display_url("https://www.example.com/"), "example.com");
        assert_eq!(display_url("https://example.com/a?b"), "example.com/a?b");
        assert_eq!(display_url("mizu://settings/"), "mizu://settings/");
        assert_eq!(
            origin_of("https://example.com:8443/a").as_deref(),
            Some("https://example.com:8443")
        );
        assert_eq!(origin_of("data:text/plain,hi"), None);
    }
}
