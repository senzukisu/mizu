//! On-disk script storage: `<profile>/scripts/`.
//!
//! ```text
//! scripts/
//! ├── index.json            installed scripts, enabled flags, origins
//! ├── <id>.user.js          script source as installed
//! ├── <id>.require.js       concatenated @require libraries (fetched at install time)
//! └── <id>.values.json      GM_setValue storage
//! ```

use crate::meta::{MetaError, ScriptMeta};
use crate::pattern::{include_pattern, match_pattern};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Per-script limit for `GM_setValue` data, so a runaway script cannot fill the disk.
pub const VALUES_LIMIT_BYTES: usize = 4 * 1024 * 1024;

/// The user's own choice of where a script runs, on top of the script's metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SiteRules {
    /// When not empty, the script runs **only** on these sites (its own `@match`/`@include`
    /// are ignored; its `@exclude` rules still apply).
    #[serde(default)]
    pub only: Vec<String>,
    /// The script never runs on these sites.
    #[serde(default)]
    pub never: Vec<String>,
}

impl SiteRules {
    /// Cleans user input: trims, drops empties and duplicates, caps the list size.
    pub fn normalized(self) -> Self {
        let clean = |v: Vec<String>| {
            let mut out: Vec<String> = Vec::new();
            for s in v {
                let s = s.trim().trim_end_matches('/').to_string();
                if !s.is_empty() && s.len() <= 512 && !out.iter().any(|o| o.eq_ignore_ascii_case(&s)) {
                    out.push(s);
                }
            }
            out.truncate(200);
            out
        };
        Self { only: clean(self.only), never: clean(self.never) }
    }
}

/// Whether a user site rule matches `url`. Accepted forms:
/// - `example.com` – the site and all its subdomains, any page;
/// - `*.example.com` – the same;
/// - `example.com/watch*` – pages whose path starts that way (`*` is a wildcard);
/// - `https://example.com/*` or any `@match`/`@include` pattern – used as is.
pub fn site_rule_matches(rule: &str, url: &str) -> bool {
    let rule = rule.trim();
    if rule.is_empty() {
        return false;
    }
    if rule == "*" || rule == "<all_urls>" {
        return true;
    }
    if rule.contains("://") || (rule.starts_with('/') && rule.ends_with('/') && rule.len() > 2) {
        return match_pattern(rule, url) || include_pattern(rule, url);
    }
    let Ok(parsed) = url::Url::parse(url) else { return false };
    let Some(host) = parsed.host_str() else { return false };
    let (host_rule, path_rule) = match rule.find('/') {
        Some(i) => (&rule[..i], Some(&rule[i..])),
        None => (rule, None),
    };
    let base = host_rule.trim_start_matches("*.").trim_start_matches("www.").to_ascii_lowercase();
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let host_ok = if base.contains('*') {
        crate::pattern::glob(&base, host)
    } else {
        host == base || host.ends_with(&format!(".{base}"))
    };
    if !host_ok {
        return false;
    }
    match path_rule {
        None => true,
        Some(p) => {
            let mut path = parsed.path().to_owned();
            if let Some(q) = parsed.query() {
                path.push('?');
                path.push_str(q);
            }
            let p = if p.ends_with('*') { p.to_owned() } else { format!("{p}*") };
            crate::pattern::glob(&p, &path)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    id: String,
    enabled: bool,
    #[serde(default)]
    sites: SiteRules,
    #[serde(default)]
    source_url: String,
    installed_at: i64,
    updated_at: i64,
    #[serde(default)]
    last_checked: i64,
}

/// What to fetch to check one script for a newer version.
#[derive(Debug, Clone)]
pub struct UpdateCheck {
    pub id: String,
    pub name: String,
    pub version: String,
    /// `@updateURL` (often a small `.meta.js`), else the download URL.
    pub check_url: String,
    /// `@downloadURL`, else the URL the script was installed from.
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserScript {
    pub id: String,
    pub enabled: bool,
    pub source_url: String,
    pub installed_at: i64,
    pub updated_at: i64,
    /// Last automatic or manual update check (0 = never).
    pub last_checked: i64,
    /// Where the user allows or forbids this script, beyond its metadata.
    pub sites: SiteRules,
    pub meta: ScriptMeta,
    #[serde(skip)]
    pub source: String,
    #[serde(skip)]
    pub requires: String,
}

impl UserScript {
    /// Whether this script should run on a top-level document at `url`.
    pub fn applies_to(&self, url: &str) -> bool {
        if !self.enabled || !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")) {
            return false;
        }
        if self.sites.never.iter().any(|r| site_rule_matches(r, url)) {
            return false;
        }
        let m = &self.meta;
        if m.excludes.iter().any(|p| include_pattern(p, url))
            || m.exclude_matches.iter().any(|p| match_pattern(p, url))
        {
            return false;
        }
        if !self.sites.only.is_empty() {
            return self.sites.only.iter().any(|r| site_rule_matches(r, url));
        }
        m.matches.iter().any(|p| match_pattern(p, url)) || m.includes.iter().any(|p| include_pattern(p, url))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Meta(#[from] MetaError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("stored values exceed the {VALUES_LIMIT_BYTES}-byte limit")]
    TooLarge,
    #[error("unknown script")]
    NotFound,
}

pub struct ScriptStore {
    dir: PathBuf,
    scripts: Vec<UserScript>,
    values: BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// FNV-1a: a stable, dependency-free id derived from the script identity.
fn script_id(identity: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in identity.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

impl ScriptStore {
    /// Loads all scripts. Broken entries are skipped with a warning, never fatal.
    pub fn open(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let index: Vec<IndexEntry> = std::fs::read_to_string(dir.join("index.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let mut store = Self {
            dir: dir.to_owned(),
            scripts: Vec::new(),
            values: BTreeMap::new(),
        };
        for entry in index {
            let Ok(source) = std::fs::read_to_string(store.path(&entry.id, "user.js")) else {
                tracing::warn!(id = entry.id, "userscript source missing, skipping");
                continue;
            };
            let Ok(meta) = ScriptMeta::parse(&source) else {
                tracing::warn!(id = entry.id, "userscript metadata unreadable, skipping");
                continue;
            };
            let requires = std::fs::read_to_string(store.path(&entry.id, "require.js")).unwrap_or_default();
            let values = std::fs::read_to_string(store.path(&entry.id, "values.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            store.values.insert(entry.id.clone(), values);
            store.scripts.push(UserScript {
                id: entry.id,
                enabled: entry.enabled,
                source_url: entry.source_url,
                installed_at: entry.installed_at,
                updated_at: entry.updated_at,
                last_checked: entry.last_checked,
                sites: entry.sites,
                meta,
                source,
                requires,
            });
        }
        store
    }

    fn path(&self, id: &str, ext: &str) -> PathBuf {
        self.dir.join(format!("{id}.{ext}"))
    }

    fn save_index(&self) -> std::io::Result<()> {
        let index: Vec<IndexEntry> = self
            .scripts
            .iter()
            .map(|s| IndexEntry {
                id: s.id.clone(),
                enabled: s.enabled,
                sites: s.sites.clone(),
                source_url: s.source_url.clone(),
                installed_at: s.installed_at,
                updated_at: s.updated_at,
                last_checked: s.last_checked,
            })
            .collect();
        let json = serde_json::to_vec_pretty(&index).map_err(std::io::Error::other)?;
        write_atomic(&self.dir.join("index.json"), &json)
    }

    pub fn scripts(&self) -> &[UserScript] {
        &self.scripts
    }

    pub fn get(&self, id: &str) -> Option<&UserScript> {
        self.scripts.iter().find(|s| s.id == id)
    }

    /// Installs or updates (same namespace + name) a script. Returns its id.
    pub fn install(&mut self, source: &str, requires: &str, source_url: &str) -> Result<String, StoreError> {
        let meta = ScriptMeta::parse(source)?;
        let id = script_id(&meta.identity());
        std::fs::write(self.path(&id, "user.js"), source)?;
        std::fs::write(self.path(&id, "require.js"), requires)?;
        let now = now_ms();
        match self.scripts.iter_mut().find(|s| s.id == id) {
            Some(existing) => {
                existing.meta = meta;
                existing.source = source.to_owned();
                existing.requires = requires.to_owned();
                existing.updated_at = now;
                if !source_url.is_empty() {
                    existing.source_url = source_url.to_owned();
                }
            }
            None => {
                self.scripts.push(UserScript {
                    id: id.clone(),
                    enabled: true,
                    source_url: source_url.to_owned(),
                    installed_at: now,
                    updated_at: now,
                    last_checked: now,
                    sites: SiteRules::default(),
                    meta,
                    source: source.to_owned(),
                    requires: requires.to_owned(),
                });
            }
        }
        self.save_index()?;
        Ok(id)
    }

    /// Whether a script with this metadata identity is already installed (update vs install).
    pub fn installed_version(&self, meta: &ScriptMeta) -> Option<&str> {
        let id = script_id(&meta.identity());
        self.get(&id).map(|s| s.meta.version.as_str())
    }

    /// Scripts with a remote source whose last check is older than `max_age_ms` (all of them
    /// when `force`). Only HTTPS sources are ever checked.
    pub fn due_for_update(&self, max_age_ms: i64, force: bool) -> Vec<UpdateCheck> {
        let now = now_ms();
        self.scripts
            .iter()
            .filter(|s| force || now - s.last_checked >= max_age_ms)
            .filter_map(|s| {
                let https = |u: &str| (u.starts_with("https://") || u.starts_with("http://")).then(|| u.to_owned());
                let download = https(&s.meta.download_url).or_else(|| https(&s.source_url))?;
                let check = https(&s.meta.update_url).unwrap_or_else(|| download.clone());
                Some(UpdateCheck {
                    id: s.id.clone(),
                    name: s.meta.name.clone(),
                    version: s.meta.version.clone(),
                    check_url: check,
                    download_url: download,
                })
            })
            .collect()
    }

    pub fn mark_checked(&mut self, ids: &[String]) {
        let now = now_ms();
        for s in self.scripts.iter_mut().filter(|s| ids.contains(&s.id)) {
            s.last_checked = now;
        }
        if let Err(e) = self.save_index() {
            tracing::warn!(error = %e, "could not save script index");
        }
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), StoreError> {
        let s = self.scripts.iter_mut().find(|s| s.id == id).ok_or(StoreError::NotFound)?;
        s.enabled = enabled;
        self.save_index()?;
        Ok(())
    }

    pub fn set_sites(&mut self, id: &str, sites: SiteRules) -> Result<(), StoreError> {
        let s = self.scripts.iter_mut().find(|s| s.id == id).ok_or(StoreError::NotFound)?;
        s.sites = sites.normalized();
        self.save_index()?;
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), StoreError> {
        let before = self.scripts.len();
        self.scripts.retain(|s| s.id != id);
        if before == self.scripts.len() {
            return Err(StoreError::NotFound);
        }
        for ext in ["user.js", "require.js", "values.json"] {
            let _ = std::fs::remove_file(self.path(id, ext));
        }
        self.values.remove(id);
        self.save_index()?;
        Ok(())
    }

    /// Scripts that should run on a top-level document at `url`.
    pub fn matching(&self, url: &str) -> Vec<&UserScript> {
        self.scripts.iter().filter(|s| s.applies_to(url)).collect()
    }

    pub fn values(&self, id: &str) -> serde_json::Map<String, serde_json::Value> {
        self.values.get(id).cloned().unwrap_or_default()
    }

    pub fn set_value(&mut self, id: &str, key: &str, value: Option<serde_json::Value>) -> Result<(), StoreError> {
        if self.get(id).is_none() {
            return Err(StoreError::NotFound);
        }
        let mut map = self.values.get(id).cloned().unwrap_or_default();
        match value {
            Some(v) => {
                map.insert(key.to_owned(), v);
            }
            None => {
                map.remove(key);
            }
        }
        let json = serde_json::to_vec(&map).map_err(std::io::Error::other)?;
        if json.len() > VALUES_LIMIT_BYTES {
            return Err(StoreError::TooLarge);
        }
        write_atomic(&self.path(id, "values.json"), &json)?;
        self.values.insert(id.to_owned(), map);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(name: &str, version: &str) -> String {
        format!(
            "// ==UserScript==\n// @name {name}\n// @namespace test\n// @version {version}\n// @match https://*.example.com/*\n// @exclude https://www.example.com/private/*\n// ==/UserScript==\nconsole.log(1);\n"
        )
    }

    #[test]
    fn install_update_match_persist() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ScriptStore::open(dir.path());
        let id = store.install(&src("A", "1.0"), "", "https://x/a.user.js").unwrap();
        let id2 = store.install(&src("A", "1.1"), "", "").unwrap();
        assert_eq!(id, id2);
        assert_eq!(store.scripts().len(), 1);
        assert_eq!(store.get(&id).unwrap().meta.version, "1.1");
        assert_eq!(store.get(&id).unwrap().source_url, "https://x/a.user.js");

        assert_eq!(store.matching("https://www.example.com/page").len(), 1);
        assert_eq!(store.matching("https://www.example.com/private/x").len(), 0);
        assert_eq!(store.matching("https://example.org/").len(), 0);

        store.set_value(&id, "k", Some(serde_json::json!({"a": 1}))).unwrap();
        store.set_enabled(&id, false).unwrap();
        assert!(store.matching("https://www.example.com/page").is_empty());

        let reopened = ScriptStore::open(dir.path());
        assert_eq!(reopened.scripts().len(), 1);
        assert!(!reopened.get(&id).unwrap().enabled);
        assert_eq!(reopened.values(&id)["k"]["a"], 1);

        let mut reopened = reopened;
        reopened.remove(&id).unwrap();
        assert!(ScriptStore::open(dir.path()).scripts().is_empty());
    }

    #[test]
    fn update_checks() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ScriptStore::open(dir.path());
        let with_urls = "// ==UserScript==\n// @name U\n// @namespace t\n// @version 1.0\n// @updateURL https://x/u.meta.js\n// @downloadURL https://x/u.user.js\n// ==/UserScript==\n";
        let local = "// ==UserScript==\n// @name L\n// @namespace t\n// @version 1.0\n// ==/UserScript==\n";
        let id = store.install(with_urls, "", "").unwrap();
        store.install(local, "", "").unwrap();
        assert!(store.due_for_update(86_400_000, false).is_empty(), "just installed");
        let due = store.due_for_update(0, true);
        assert_eq!(due.len(), 1, "scripts without a remote source are skipped");
        assert_eq!(due[0].check_url, "https://x/u.meta.js");
        assert_eq!(due[0].download_url, "https://x/u.user.js");
        store.mark_checked(&[id.clone()]);
        assert!(ScriptStore::open(dir.path()).get(&id).unwrap().last_checked > 0);
    }

    #[test]
    fn enforces_value_limit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ScriptStore::open(dir.path());
        let id = store.install(&src("B", "1"), "", "").unwrap();
        let big = "x".repeat(VALUES_LIMIT_BYTES + 1);
        assert!(matches!(
            store.set_value(&id, "k", Some(big.into())),
            Err(StoreError::TooLarge)
        ));
    }
}

#[cfg(test)]
mod site_rule_tests {
    use super::*;

    #[test]
    fn plain_domain_covers_subdomains_and_paths() {
        assert!(site_rule_matches("youtube.com", "https://www.youtube.com/watch?v=1"));
        assert!(site_rule_matches("youtube.com", "https://m.youtube.com/"));
        assert!(site_rule_matches("*.youtube.com", "https://youtube.com/"));
        assert!(!site_rule_matches("youtube.com", "https://notyoutube.com/"));
        assert!(!site_rule_matches("youtube.com", "https://youtube.com.evil.net/"));
    }

    #[test]
    fn path_prefix_and_patterns() {
        assert!(site_rule_matches("github.com/senzukisu", "https://github.com/senzukisu/mizu"));
        assert!(!site_rule_matches("github.com/senzukisu", "https://github.com/other"));
        assert!(site_rule_matches("example.com/a*b", "https://example.com/a-x-b/c"));
        assert!(site_rule_matches("https://*.example.org/*", "https://docs.example.org/x"));
        assert!(!site_rule_matches("https://*.example.org/*", "http://docs.example.org/x"));
    }

    #[test]
    fn normalizes_lists() {
        let r = SiteRules { only: vec![" a.com/ ".into(), "A.com".into(), String::new()], never: vec![] }.normalized();
        assert_eq!(r.only, vec!["a.com".to_string()]);
    }
}
