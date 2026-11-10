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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    id: String,
    enabled: bool,
    #[serde(default)]
    source_url: String,
    installed_at: i64,
    updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserScript {
    pub id: String,
    pub enabled: bool,
    pub source_url: String,
    pub installed_at: i64,
    pub updated_at: i64,
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
        let m = &self.meta;
        if m.excludes.iter().any(|p| include_pattern(p, url))
            || m.exclude_matches.iter().any(|p| match_pattern(p, url))
        {
            return false;
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
                source_url: s.source_url.clone(),
                installed_at: s.installed_at,
                updated_at: s.updated_at,
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

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), StoreError> {
        let s = self.scripts.iter_mut().find(|s| s.id == id).ok_or(StoreError::NotFound)?;
        s.enabled = enabled;
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
