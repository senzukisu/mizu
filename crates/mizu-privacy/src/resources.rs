//! Redirect and scriptlet resources.
//!
//! Filters like `||host/ga.js$redirect=google-analytics_ga.js` or `site##+js(set-constant, …)`
//! refer to named resources. A complete, MPL-2.0 licensed resource bundle is downloaded from
//! Brave's `adblock-resources` project together with the filter lists. The small built-in set
//! below guarantees that the most common neutered resources work offline.

use adblock::resources::{MimeType, Resource, ResourceType};
use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;

/// 1×1 transparent GIF.
const PIXEL_GIF: &[u8] = &[
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xff, 0xff, 0xff, 0x21, 0xf9, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x01, 0x44, 0x00, 0x3b,
];

const SET_CONSTANT: &str = r#"(function () {
  'use strict';
  const chain = '{{1}}';
  const raw = '{{2}}';
  let value;
  switch (raw) {
    case 'true': value = true; break;
    case 'false': value = false; break;
    case 'null': value = null; break;
    case 'undefined': value = undefined; break;
    case 'noopFunc': value = function () {}; break;
    case 'trueFunc': value = function () { return true; }; break;
    case 'falseFunc': value = function () { return false; }; break;
    case '': value = ''; break;
    default:
      if (/^\d+$/.test(raw)) { value = parseInt(raw, 10); } else { return; }
  }
  try {
    const parts = chain.split('.');
    let owner = window;
    for (let i = 0; i < parts.length - 1; i++) {
      if (!(parts[i] in owner)) { owner[parts[i]] = {}; }
      owner = owner[parts[i]];
    }
    Object.defineProperty(owner, parts[parts.length - 1], {
      configurable: false,
      get() { return value; },
      set() {},
    });
  } catch (e) { /* page already locked the property */ }
})();"#;

fn mime(name: &str, aliases: &[&str], kind: MimeType, content: &[u8]) -> Resource {
    Resource {
        name: name.into(),
        aliases: aliases.iter().map(|a| (*a).into()).collect(),
        kind: ResourceType::Mime(kind),
        content: BASE64_STANDARD.encode(content),
        dependencies: Vec::new(),
        permission: Default::default(),
    }
}

pub fn builtin_resources() -> Vec<Resource> {
    vec![
        mime("noop.js", &["noopjs", "noop-0.1s.js"], MimeType::ApplicationJavascript, b"(function(){})();"),
        mime("noop.txt", &["nooptext"], MimeType::TextPlain, b""),
        mime("noop.html", &["noopframe"], MimeType::TextHtml, b"<!DOCTYPE html>"),
        mime("noop.css", &[], MimeType::TextCss, b""),
        mime("noop.json", &[], MimeType::ApplicationJson, b"{}"),
        mime("empty", &[], MimeType::TextPlain, b""),
        mime("1x1.gif", &["1x1-transparent.gif"], MimeType::ImageGif, PIXEL_GIF),
        Resource {
            name: "set-constant.js".into(),
            aliases: vec!["set-constant".into(), "set.js".into()],
            kind: ResourceType::Template,
            content: BASE64_STANDARD.encode(SET_CONSTANT),
            dependencies: Vec::new(),
            permission: Default::default(),
        },
    ]
}

/// Parses a downloaded `resources.json` and merges it over the built-in set (downloaded
/// resources win on name clashes).
pub fn merge_downloaded(json: &str) -> Result<Vec<Resource>, serde_json::Error> {
    let downloaded: Vec<Resource> = serde_json::from_str(json)?;
    let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in &downloaded {
        names.insert(r.name.clone());
        names.extend(r.aliases.iter().cloned());
    }
    let mut all: Vec<Resource> = builtin_resources()
        .into_iter()
        .filter(|r| !names.contains(&r.name) && !r.aliases.iter().any(|a| names.contains(a)))
        .collect();
    all.extend(downloaded);
    Ok(all)
}
