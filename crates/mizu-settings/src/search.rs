use serde::{Deserialize, Serialize};

/// A search provider. `template` contains `{searchTerms}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchEngine {
    pub id: String,
    pub name: String,
    pub template: String,
    /// Short keyword to trigger this engine from the address bar, e.g. `@w`.
    #[serde(default)]
    pub keyword: String,
}

impl SearchEngine {
    fn builtin(id: &str, name: &str, template: &str, keyword: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            template: template.into(),
            keyword: keyword.into(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.id.is_empty()
            && !self.name.trim().is_empty()
            && self.template.contains("{searchTerms}")
            && (self.template.starts_with("https://") || self.template.starts_with("http://"))
    }
}

/// OpenSearch suggestion endpoint (`["query", ["suggestion", …]]`) of a built-in engine.
/// Engines without one simply get no remote suggestions; we never ask a different provider.
pub fn suggest_template(engine_id: &str) -> Option<&'static str> {
    Some(match engine_id {
        "duckduckgo" => "https://duckduckgo.com/ac/?q={searchTerms}&type=list",
        "google" => "https://suggestqueries.google.com/complete/search?client=firefox&ie=utf-8&oe=utf-8&q={searchTerms}",
        "bing" => "https://api.bing.com/osjson.aspx?query={searchTerms}",
        "brave" => "https://search.brave.com/api/suggest?q={searchTerms}",
        "ecosia" => "https://ac.ecosia.org/autocomplete?type=list&q={searchTerms}",
        "yandex" => "https://suggest.yandex.ru/suggest-ff.cgi?part={searchTerms}",
        "wikipedia" => "https://en.wikipedia.org/w/api.php?action=opensearch&format=json&limit=6&search={searchTerms}",
        _ => return None,
    })
}

/// Engines shipped with Mizu. DuckDuckGo is the default because it does not build
/// search profiles; the others are offered without any commercial arrangement.
pub fn builtin_search_engines() -> Vec<SearchEngine> {
    vec![
        SearchEngine::builtin("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q={searchTerms}", "@ddg"),
        SearchEngine::builtin("google", "Google", "https://www.google.com/search?q={searchTerms}", "@g"),
        SearchEngine::builtin("bing", "Bing", "https://www.bing.com/search?q={searchTerms}", "@b"),
        SearchEngine::builtin("brave", "Brave Search", "https://search.brave.com/search?q={searchTerms}", "@brave"),
        SearchEngine::builtin("startpage", "Startpage", "https://www.startpage.com/do/search?q={searchTerms}", "@sp"),
        SearchEngine::builtin("ecosia", "Ecosia", "https://www.ecosia.org/search?q={searchTerms}", "@eco"),
        SearchEngine::builtin("yandex", "Yandex", "https://yandex.ru/search/?text={searchTerms}", "@ya"),
        SearchEngine::builtin("wikipedia", "Wikipedia", "https://en.wikipedia.org/w/index.php?search={searchTerms}", "@w"),
        SearchEngine::builtin("github", "GitHub", "https://github.com/search?q={searchTerms}", "@gh"),
    ]
}
