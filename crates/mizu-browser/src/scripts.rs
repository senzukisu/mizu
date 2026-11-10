//! Per-navigation document scripts: Global Privacy Control, cosmetic filtering, uBlock
//! scriptlets and Mizu Scripts (userscripts); plus the userscript install flow.

use crate::hot::Hot;
use crate::queue::{WorkerResult, WorkerSender};
use mizu_engine::ViewKey;
use mizu_userscripts::{GmMessage, ScriptMeta, build_injection};
use std::hash::{BuildHasher, Hasher};

/// Unpredictable token for the userscript message channel (SipHash with OS-random keys).
fn random_token() -> String {
    let a = std::collections::hash_map::RandomState::new().build_hasher().finish();
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(a);
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default(),
    );
    format!("{a:016x}{:016x}", h.finish())
}

fn js_str(s: &str) -> String {
    serde_json::to_string(s)
        .unwrap_or_else(|_| "\"\"".into())
        .replace("</", "<\\/")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// Builds the document-start script for a top-level navigation to `uri`, or `None` if nothing
/// needs injecting.
pub fn document_script(h: &mut Hot, key: ViewKey, uri: &str) -> Option<String> {
    let origin = mizu_core::omnibox::origin_of(uri)?;
    let mut parts: Vec<String> = Vec::new();

    if h.global_privacy_control {
        parts.push(
            "try{Object.defineProperty(Navigator.prototype,'globalPrivacyControl',{get:()=>true,configurable:true});}catch(e){}"
                .into(),
        );
    }

    if h.content_blocking && !crate::shield::allowlisted(&h.allowlist, uri) {
        if let Some(blocker) = &h.blocker {
            let cosmetic = blocker.cosmetic(uri);
            if !cosmetic.css.is_empty() {
                parts.push(format!(
                    "(()=>{{const css={css};const add=()=>{{const s=document.createElement('style');s.setAttribute('data-mizu','shield');s.textContent=css;(document.head||document.documentElement).appendChild(s);}};\
                     if(document.documentElement)add();else new MutationObserver((m,o)=>{{if(document.documentElement){{o.disconnect();add();}}}}).observe(document,{{childList:true}});}})();",
                    css = js_str(&cosmetic.css)
                ));
            }
            if !cosmetic.script.is_empty() {
                parts.push(format!("try{{\n{}\n}}catch(e){{}}", cosmetic.script));
            }
        }
    }

    let matching: Vec<_> = h
        .scripts
        .matching(uri)
        .into_iter()
        .map(|s| (s, h.scripts.values(&s.id)))
        .collect();
    if matching.is_empty() {
        h.gm_tokens.remove(&key);
    } else {
        let token = random_token();
        let ids = matching.iter().map(|(s, _)| s.id.clone()).collect();
        parts.push(build_injection(&matching, &token, &origin));
        h.gm_tokens.insert(key, (token, ids));
    }

    if parts.is_empty() {
        return None;
    }
    Some(format!(
        "(()=>{{if(window.top!==window)return;if(location.origin!=={origin})return;\n{}\n}})();",
        parts.join("\n"),
        origin = js_str(&origin)
    ))
}

/// What a userscript message asks the browser to do after validation.
pub enum GmAction {
    None,
    OpenTab { url: String, background: bool },
}

/// Validates and applies a `{"mizu_gm": …}` message from web content.
pub fn handle_gm_message(h: &mut Hot, key: ViewKey, json: &serde_json::Value) -> GmAction {
    let Some(raw) = json.get("mizu_gm") else { return GmAction::None };
    let Ok(msg) = serde_json::from_value::<GmMessage>(raw.clone()) else { return GmAction::None };
    let authorised = h
        .gm_tokens
        .get(&key)
        .is_some_and(|(token, ids)| token == msg.token() && ids.iter().any(|i| i == msg.script_id()));
    if !authorised {
        tracing::warn!(key, "rejected userscript message with invalid token");
        return GmAction::None;
    }
    let sid = msg.script_id().to_owned();
    match msg {
        GmMessage::Set { key: k, value, .. } => {
            if let Err(e) = h.scripts.set_value(&sid, &k, Some(value)) {
                tracing::warn!(error = %e, "GM_setValue failed");
            }
            GmAction::None
        }
        GmMessage::Delete { key: k, .. } => {
            let _ = h.scripts.set_value(&sid, &k, None);
            GmAction::None
        }
        GmMessage::OpenInTab { url, background, .. } => {
            if url.starts_with("https://") || url.starts_with("http://") {
                GmAction::OpenTab { url, background }
            } else {
                GmAction::None
            }
        }
    }
}

/// Downloads a userscript and its `@require` libraries on a worker thread.
pub fn fetch_for_install(url: String, sender: WorkerSender) {
    std::thread::spawn(move || {
        let result = (|| -> Result<(String, String), String> {
            let source = fetch(&url)?;
            let meta = ScriptMeta::parse(&source).map_err(|e| e.to_string())?;
            let mut requires = String::new();
            for req in &meta.requires {
                let text = fetch(req).map_err(|e| format!("@require {req}: {e}"))?;
                requires.push_str(&text);
                requires.push_str("\n;\n");
            }
            Ok((source, requires))
        })();
        sender.send(WorkerResult::ScriptFetched { url, result });
    });
}

fn fetch(url: &str) -> Result<String, String> {
    if let Some(path) = url.strip_prefix("file:///") {
        let path = url::Url::parse(url)
            .ok()
            .and_then(|u| u.to_file_path().ok())
            .unwrap_or_else(|| path.into());
        return std::fs::read_to_string(path).map_err(|e| e.to_string());
    }
    let url = if url.starts_with("http://") {
        // Upgrade: scripts run with access to pages, so they must not be fetched in clear text.
        url.replacen("http://", "https://", 1)
    } else {
        url.to_owned()
    };
    mizu_privacy::fetch_https_text(&url).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique() {
        assert_ne!(random_token(), random_token());
        assert_eq!(random_token().len(), 32);
    }
}
