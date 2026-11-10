//! Filter list storage and updates.
//!
//! Lists are cached in `<profile>/filters/<id>.txt`, with metadata in `meta.json`. Downloads
//! only use HTTPS, are size-limited, are sanity-checked (an HTML error page is not a filter
//! list) and are written atomically, so a failed update never destroys the previous copy.

use crate::resources::{builtin_resources, merge_downloaded};
use adblock::resources::Resource;
use mizu_settings::{BUNDLED_LIST_ID, FilterListSubscription};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const BUNDLED_LIST: &str = include_str!("../../../assets/filters/mizu-shield.txt");

/// Brave's MPL-2.0 scriptlet and redirect resources (uBlock-compatible names).
pub const RESOURCES_URL: &str =
    "https://cdn.jsdelivr.net/gh/brave/adblock-resources@master/dist/resources.json";
const RESOURCES_ID: &str = "resources";

/// Lists older than this are refreshed in the background.
pub const MAX_AGE: Duration = Duration::from_secs(4 * 24 * 3600);
const MAX_DOWNLOAD_BYTES: u64 = 25 * 1024 * 1024;
const USER_AGENT: &str = concat!("Mizu/", env!("CARGO_PKG_VERSION"), " (filter list updater)");

#[derive(Debug, thiserror::Error)]
pub enum ListError {
    #[error("only https:// list URLs are allowed")]
    InsecureUrl,
    #[error("download failed: {0}")]
    Http(String),
    #[error("the downloaded file does not look like a filter list")]
    NotAFilterList,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ListMeta {
    last_updated: Option<i64>,
    last_error: Option<String>,
    bytes: u64,
}

/// What the Shield settings page shows for each list.
#[derive(Debug, Clone, Serialize)]
pub struct ListStatus {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub builtin: bool,
    pub downloaded: bool,
    pub last_updated: Option<i64>,
    pub last_error: Option<String>,
    pub bytes: u64,
}

pub struct FilterStore {
    dir: PathBuf,
    meta: HashMap<String, ListMeta>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// List ids come from settings; never let one escape the filters directory.
fn safe_id(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

impl FilterStore {
    pub fn open(dir: &Path) -> Self {
        let meta = std::fs::read_to_string(dir.join("meta.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self {
            dir: dir.to_owned(),
            meta,
        }
    }

    fn list_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.txt", safe_id(id)))
    }

    fn save_meta(&self) {
        let Ok(text) = serde_json::to_string_pretty(&self.meta) else { return };
        let tmp = self.dir.join("meta.json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(tmp, self.dir.join("meta.json"));
        }
    }

    /// Texts of all enabled lists that are available locally (bundled list always is).
    pub fn enabled_texts(&self, subs: &[FilterListSubscription], custom_rules: &str) -> Vec<String> {
        let mut out = Vec::new();
        for sub in subs.iter().filter(|s| s.enabled) {
            if sub.id == BUNDLED_LIST_ID {
                out.push(BUNDLED_LIST.to_owned());
            } else if let Ok(text) = std::fs::read_to_string(self.list_path(&sub.id)) {
                out.push(text);
            }
        }
        if !custom_rules.trim().is_empty() {
            out.push(custom_rules.to_owned());
        }
        out
    }

    pub fn resources(&self) -> Vec<Resource> {
        let path = self.dir.join("resources.json");
        match std::fs::read_to_string(&path) {
            Ok(json) => merge_downloaded(&json).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "resources.json unreadable, using built-in resources");
                builtin_resources()
            }),
            Err(_) => builtin_resources(),
        }
    }

    fn is_stale(&self, id: &str) -> bool {
        let fresh = self
            .meta
            .get(id)
            .and_then(|m| m.last_updated)
            .is_some_and(|t| now_ms() - t < MAX_AGE.as_millis() as i64);
        !fresh || !self.list_path(id).exists() && id != RESOURCES_ID
    }

    /// Subscriptions that should be downloaded now (enabled, remote, missing or stale).
    pub fn due_for_update<'a>(
        &self,
        subs: &'a [FilterListSubscription],
        force: bool,
    ) -> Vec<&'a FilterListSubscription> {
        subs.iter()
            .filter(|s| s.enabled && !s.url.is_empty())
            .filter(|s| force || self.is_stale(&s.id))
            .collect()
    }

    pub fn resources_due(&self, force: bool) -> bool {
        force || self.is_stale(RESOURCES_ID) || !self.dir.join("resources.json").exists()
    }

    /// Downloads one list. Blocking; call from a worker thread.
    pub fn update_list(&mut self, sub: &FilterListSubscription) -> Result<u64, ListError> {
        let result = fetch_https_text(&sub.url).and_then(|text| {
            if !looks_like_filter_list(&text) {
                return Err(ListError::NotAFilterList);
            }
            write_atomic(&self.list_path(&sub.id), text.as_bytes())?;
            Ok(text.len() as u64)
        });
        self.record(&sub.id, &result);
        result
    }

    pub fn update_resources(&mut self) -> Result<u64, ListError> {
        let result = fetch_https_text(RESOURCES_URL).and_then(|json| {
            merge_downloaded(&json).map_err(|_| ListError::NotAFilterList)?;
            write_atomic(&self.dir.join("resources.json"), json.as_bytes())?;
            Ok(json.len() as u64)
        });
        self.record(RESOURCES_ID, &result);
        result
    }

    fn record(&mut self, id: &str, result: &Result<u64, ListError>) {
        let meta = self.meta.entry(id.to_owned()).or_default();
        match result {
            Ok(bytes) => {
                meta.last_updated = Some(now_ms());
                meta.last_error = None;
                meta.bytes = *bytes;
            }
            Err(e) => {
                tracing::warn!(list = id, error = %e, "filter list update failed");
                meta.last_error = Some(e.to_string());
            }
        }
        self.save_meta();
    }

    pub fn remove_list(&mut self, id: &str) {
        let _ = std::fs::remove_file(self.list_path(id));
        self.meta.remove(id);
        self.save_meta();
    }

    pub fn status(&self, subs: &[FilterListSubscription]) -> Vec<ListStatus> {
        subs.iter()
            .map(|s| {
                let meta = self.meta.get(&s.id).cloned().unwrap_or_default();
                let bundled = s.id == BUNDLED_LIST_ID;
                ListStatus {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    url: s.url.clone(),
                    enabled: s.enabled,
                    builtin: s.builtin,
                    downloaded: bundled || self.list_path(&s.id).exists(),
                    last_updated: meta.last_updated,
                    last_error: meta.last_error,
                    bytes: if bundled { BUNDLED_LIST.len() as u64 } else { meta.bytes },
                }
            })
            .collect()
    }
}

/// Fetches a text resource over HTTPS with a size cap (filter lists, userscripts).
pub fn fetch_https_text(url: &str) -> Result<String, ListError> {
    let bytes = fetch_https_bytes(url, MAX_DOWNLOAD_BYTES)?;
    String::from_utf8(bytes).map_err(|_| ListError::Http("response is not UTF-8 text".into()))
}

/// Fetches a binary resource over HTTPS (redirects must stay on HTTPS), up to `limit` bytes.
pub fn fetch_https_bytes(url: &str, limit: u64) -> Result<Vec<u8>, ListError> {
    if !url.starts_with("https://") {
        return Err(ListError::InsecureUrl);
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .https_only(true)
        .user_agent(USER_AGENT)
        // Windows' own TLS stack (SChannel): system certificate store and revocation checks.
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| ListError::Http(e.to_string()))?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|e| ListError::Http(e.to_string()))?;
    Ok(bytes)
}

fn looks_like_filter_list(text: &str) -> bool {
    let head = text.trim_start();
    if head.is_empty() || head.starts_with('<') || head.starts_with('{') {
        return false;
    }
    text.lines().take(200).any(|l| {
        let l = l.trim();
        l.starts_with("||") || l.contains("##") || l.starts_with('!') || l.starts_with("0.0.0.0") || l.starts_with("@@")
    })
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(id: &str, url: &str, enabled: bool) -> FilterListSubscription {
        FilterListSubscription {
            id: id.into(),
            name: id.into(),
            url: url.into(),
            enabled,
            builtin: false,
        }
    }

    #[test]
    fn validates_list_content() {
        assert!(looks_like_filter_list("! Title: x\n||ads.example^\n"));
        assert!(looks_like_filter_list("0.0.0.0 ads.example\n"));
        assert!(!looks_like_filter_list("<!DOCTYPE html><html>"));
        assert!(!looks_like_filter_list(""));
    }

    #[test]
    fn rejects_insecure_urls() {
        assert!(matches!(fetch_https_text("http://example.com/list.txt"), Err(ListError::InsecureUrl)));
    }

    #[test]
    fn ids_cannot_escape_directory() {
        assert_eq!(safe_id("../../evil"), "______evil");
    }

    #[test]
    fn reads_enabled_lists_and_reports_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = FilterStore::open(dir.path());
        std::fs::write(dir.path().join("a.txt"), "||a.example^\n").unwrap();
        let subs = vec![
            sub(BUNDLED_LIST_ID, "", true),
            sub("a", "https://x/a.txt", true),
            sub("b", "https://x/b.txt", false),
        ];
        let texts = store.enabled_texts(&subs, "||custom.example^");
        assert_eq!(texts.len(), 3);
        let due: Vec<_> = store.due_for_update(&subs, false).iter().map(|s| s.id.clone()).collect();
        assert_eq!(due, ["a"]); // never updated, so stale; b is disabled; bundled is local
        let status = store.status(&subs);
        assert!(status[0].downloaded && status[1].downloaded && !status[2].downloaded);
    }
}
