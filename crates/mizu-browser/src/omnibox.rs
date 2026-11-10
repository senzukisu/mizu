//! Address-bar suggestions. Everything is computed locally from history, bookmarks and
//! internal pages – nothing typed is sent to a suggestion service.

use mizu_core::internal::InternalPage;
use mizu_core::omnibox::{OmniboxAction, classify, display_url, search_url};
use mizu_settings::Settings;
use mizu_storage::Storage;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    /// `navigate`, `search`, `history`, `bookmark`, `internal`
    pub kind: &'static str,
    pub title: String,
    pub url: String,
    pub display: String,
}

const MAX: usize = 8;

/// Resolves `@keyword query` to an engine-specific search.
pub fn keyword_search(settings: &Settings, input: &str) -> Option<(String, String)> {
    let (kw, rest) = input.trim().split_once(char::is_whitespace)?;
    if !kw.starts_with('@') || rest.trim().is_empty() {
        return None;
    }
    let engine = settings
        .search
        .all_engines()
        .into_iter()
        .find(|e| !e.keyword.is_empty() && e.keyword.eq_ignore_ascii_case(kw))?;
    Some((engine.name, search_url(&engine.template, rest.trim())))
}

/// URL the omnibox should open for raw input.
pub fn resolve_input(settings: &Settings, input: &str) -> Option<String> {
    if let Some((_, url)) = keyword_search(settings, input) {
        return Some(url);
    }
    mizu_core::omnibox::resolve(input, &settings.search.active_engine().template)
}

pub fn suggest(settings: &Settings, storage: Option<&Storage>, input: &str) -> Vec<Suggestion> {
    let text = input.trim();
    let mut out: Vec<Suggestion> = Vec::new();
    if text.is_empty() {
        return out;
    }
    let engine = settings.search.active_engine();

    // 1. What the user typed.
    if let Some((name, url)) = keyword_search(settings, text) {
        out.push(Suggestion { kind: "search", title: format!("Search {name}"), display: text.to_owned(), url });
    } else {
        match classify(text) {
            Some(OmniboxAction::Navigate(url)) => out.push(Suggestion {
                kind: "navigate",
                title: display_url(&url),
                display: url.clone(),
                url,
            }),
            Some(OmniboxAction::Search(q)) => out.push(Suggestion {
                kind: "search",
                title: q.clone(),
                display: format!("Search {}", engine.name),
                url: search_url(&engine.template, &q),
            }),
            None => {}
        }
    }

    // 2. Internal pages by name.
    let lower = text.to_lowercase();
    for page in InternalPage::ALL {
        if matches!(page, InternalPage::Blocked) {
            continue;
        }
        if page.title().to_lowercase().starts_with(&lower) || page.host().starts_with(&lower) {
            out.push(Suggestion {
                kind: "internal",
                title: page.title().to_owned(),
                display: page.url(),
                url: page.url(),
            });
        }
    }

    // 3. Bookmarks and history.
    if settings.search.local_suggestions {
        if let Some(storage) = storage {
            for b in storage.search_bookmarks(text, 3).unwrap_or_default() {
                if let Some(url) = b.url {
                    out.push(Suggestion { kind: "bookmark", title: b.title, display: display_url(&url), url });
                }
            }
            for h in storage.suggest_history(text, MAX).unwrap_or_default() {
                let title = if h.title.is_empty() { display_url(&h.url) } else { h.title };
                out.push(Suggestion { kind: "history", title, display: display_url(&h.url), url: h.url });
            }
        }
    }

    // De-duplicate by URL, keeping the first (highest-priority) entry.
    let mut seen = std::collections::HashSet::new();
    out.retain(|s| seen.insert(s.url.clone()));
    out.truncate(MAX);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_typed_url_and_history() {
        let settings = Settings::default();
        let mut storage = Storage::in_memory().unwrap();
        storage.record_visit("https://example.com/docs", "Example docs").unwrap();
        let s = suggest(&settings, Some(&storage), "example.com");
        assert_eq!(s[0].kind, "navigate");
        assert!(s.iter().any(|x| x.kind == "history"));

        let s = suggest(&settings, Some(&storage), "rust lifetimes");
        assert_eq!(s[0].kind, "search");
        assert!(s[0].url.starts_with("https://duckduckgo.com/"));

        let s = suggest(&settings, None, "sett");
        assert!(s.iter().any(|x| x.url == "mizu://settings/"));
    }

    #[test]
    fn keyword_engines() {
        let settings = Settings::default();
        let url = resolve_input(&settings, "@w Rust (language)").unwrap();
        assert!(url.starts_with("https://en.wikipedia.org/"));
        assert!(resolve_input(&settings, "@unknown x").unwrap().starts_with("https://duckduckgo.com/"));
    }
}
