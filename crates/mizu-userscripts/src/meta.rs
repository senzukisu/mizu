//! `// ==UserScript==` metadata parsing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RunAt {
    DocumentStart,
    #[default]
    DocumentEnd,
    DocumentIdle,
    /// Tampermonkey's `document-body` – treated as "when <body> exists".
    DocumentBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScriptMeta {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub icon: String,
    pub matches: Vec<String>,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub exclude_matches: Vec<String>,
    pub grants: Vec<String>,
    pub requires: Vec<String>,
    pub run_at: RunAt,
    pub no_frames: bool,
    pub update_url: String,
    pub download_url: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MetaError {
    #[error("no ==UserScript== metadata block found")]
    Missing,
    #[error("the script has no @name")]
    NoName,
}

impl ScriptMeta {
    pub fn parse(source: &str) -> Result<Self, MetaError> {
        let mut meta = ScriptMeta::default();
        let mut inside = false;
        let mut found = false;
        for line in source.lines() {
            let line = line.trim();
            if !inside {
                if line.starts_with("//") && line.contains("==UserScript==") {
                    inside = true;
                    found = true;
                }
                continue;
            }
            if line.starts_with("//") && line.contains("==/UserScript==") {
                break;
            }
            let Some(rest) = line.strip_prefix("//") else { continue };
            let rest = rest.trim_start();
            let Some(rest) = rest.strip_prefix('@') else { continue };
            let (key, value) = match rest.split_once(char::is_whitespace) {
                Some((k, v)) => (k, v.trim()),
                None => (rest, ""),
            };
            // Localised keys such as `@name:ru` – keep the default-locale value only.
            if key.contains(':') {
                continue;
            }
            match key {
                "name" => meta.name = value.into(),
                "namespace" => meta.namespace = value.into(),
                "version" => meta.version = value.into(),
                "description" => meta.description = value.into(),
                "author" => meta.author = value.into(),
                "icon" | "iconURL" | "defaulticon" => meta.icon = value.into(),
                "match" => meta.matches.push(value.into()),
                "include" => meta.includes.push(value.into()),
                "exclude" => meta.excludes.push(value.into()),
                "exclude-match" => meta.exclude_matches.push(value.into()),
                "grant" => meta.grants.push(value.into()),
                "require" => meta.requires.push(value.into()),
                "noframes" => meta.no_frames = true,
                "updateURL" => meta.update_url = value.into(),
                "downloadURL" => meta.download_url = value.into(),
                "run-at" => {
                    meta.run_at = match value {
                        "document-start" => RunAt::DocumentStart,
                        "document-idle" => RunAt::DocumentIdle,
                        "document-body" => RunAt::DocumentBody,
                        _ => RunAt::DocumentEnd,
                    }
                }
                _ => {}
            }
        }
        if !found {
            return Err(MetaError::Missing);
        }
        if meta.name.trim().is_empty() {
            return Err(MetaError::NoName);
        }
        Ok(meta)
    }

    /// Stable identity used to detect updates of an already installed script.
    pub fn identity(&self) -> String {
        format!("{}::{}", self.namespace, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
// ==UserScript==
// @name         Dark YouTube
// @name:ru      Тёмный YouTube
// @namespace    https://example.org
// @version      1.2.3
// @match        https://*.youtube.com/*
// @include      /^https://music\.youtube\.com/
// @exclude      https://www.youtube.com/embed/*
// @grant        GM_addStyle
// @grant        GM.getValue
// @run-at       document-start
// @noframes
// ==/UserScript==
GM_addStyle('body{background:#000}');
"#;

    #[test]
    fn parses_metadata() {
        let m = ScriptMeta::parse(SRC).unwrap();
        assert_eq!(m.name, "Dark YouTube");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.matches, ["https://*.youtube.com/*"]);
        assert_eq!(m.includes.len(), 1);
        assert_eq!(m.excludes.len(), 1);
        assert_eq!(m.grants, ["GM_addStyle", "GM.getValue"]);
        assert_eq!(m.run_at, RunAt::DocumentStart);
        assert!(m.no_frames);
    }

    #[test]
    fn rejects_non_scripts() {
        assert_eq!(ScriptMeta::parse("console.log(1)"), Err(MetaError::Missing));
        assert_eq!(
            ScriptMeta::parse("// ==UserScript==\n// @version 1\n// ==/UserScript=="),
            Err(MetaError::NoName)
        );
    }
}
