//! Document-start bootstrap for userscripts.
//!
//! The browser registers the generated code with the engine so that it runs when the next
//! top-level document is created, before any page script. Each script gets its own closure
//! with a private copy of the `GM_*` API; the page cannot reach those functions.
//!
//! Supported API: `GM_info`, `GM_getValue`, `GM_setValue`, `GM_deleteValue`, `GM_listValues`,
//! `GM_addStyle`, `GM_addElement`, `GM_openInTab`, `GM_setClipboard`, `GM_log`,
//! `GM_registerMenuCommand` (accepted, no menu UI yet), `GM_xmlhttpRequest` (implemented over
//! `fetch`, therefore subject to CORS – unlike Tampermonkey), `unsafeWindow`, and the
//! promise-based `GM.*` equivalents.
//!
//! Persisting values goes through `chrome.webview.postMessage`. Every message carries a
//! random per-document token known only to the bootstrap closure, and the browser accepts
//! value writes only for (token, script) pairs it issued for that tab.

use crate::meta::RunAt;
use crate::store::UserScript;
use serde::Deserialize;

/// Message posted by the bootstrap (`{"mizu_gm": {...}}`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum GmMessage {
    Set {
        token: String,
        sid: String,
        key: String,
        value: serde_json::Value,
    },
    Delete {
        token: String,
        sid: String,
        key: String,
    },
    OpenInTab {
        token: String,
        sid: String,
        url: String,
        #[serde(default)]
        background: bool,
    },
}

impl GmMessage {
    pub fn token(&self) -> &str {
        match self {
            Self::Set { token, .. } | Self::Delete { token, .. } | Self::OpenInTab { token, .. } => token,
        }
    }
    pub fn script_id(&self) -> &str {
        match self {
            Self::Set { sid, .. } | Self::Delete { sid, .. } | Self::OpenInTab { sid, .. } => sid,
        }
    }
}

const RUNTIME: &str = r#"
const __mizuPost = (window.chrome && window.chrome.webview)
  ? window.chrome.webview.postMessage.bind(window.chrome.webview) : () => {};
function __mizuMakeApi(TOKEN, SID, info, initialValues) {
  const values = initialValues;
  const send = (msg) => { msg.token = TOKEN; msg.sid = SID; __mizuPost({ mizu_gm: msg }); };
  const clone = (v) => (v === undefined ? v : JSON.parse(JSON.stringify(v)));
  const api = {
    GM_info: info,
    unsafeWindow: window,
    GM_log: (...a) => console.log('[' + info.script.name + ']', ...a),
    GM_getValue: (k, d) => (Object.prototype.hasOwnProperty.call(values, k) ? clone(values[k]) : d),
    GM_setValue: (k, v) => { k = String(k); values[k] = clone(v); send({ op: 'set', key: k, value: values[k] === undefined ? null : values[k] }); },
    GM_deleteValue: (k) => { k = String(k); delete values[k]; send({ op: 'delete', key: k }); },
    GM_listValues: () => Object.keys(values),
    GM_addStyle: (css) => {
      const s = document.createElement('style');
      s.textContent = css;
      (document.head || document.documentElement).appendChild(s);
      return s;
    },
    GM_addElement: (parent, tag, attrs) => {
      if (typeof parent === 'string') { attrs = tag; tag = parent; parent = document.head || document.documentElement; }
      const el = document.createElement(tag);
      for (const [k, v] of Object.entries(attrs || {})) { if (k === 'textContent') el.textContent = v; else el.setAttribute(k, v); }
      parent.appendChild(el);
      return el;
    },
    GM_openInTab: (url, opts) => {
      const background = typeof opts === 'boolean' ? opts : !!(opts && (opts.active === false || opts.loadInBackground));
      send({ op: 'open_in_tab', url: new URL(url, location.href).href, background });
      return { close() {}, closed: false };
    },
    GM_setClipboard: (data) => {
      const text = typeof data === 'string' ? data : String(data);
      if (navigator.clipboard) navigator.clipboard.writeText(text).catch(() => {});
    },
    GM_registerMenuCommand: (() => { let n = 0; return () => ++n; })(),
    GM_unregisterMenuCommand: () => {},
    GM_notification: (d) => console.info('[' + info.script.name + ']', typeof d === 'string' ? d : (d && d.text)),
    GM_xmlhttpRequest: (d) => {
      const ctrl = new AbortController();
      const resp = (r, text) => ({ status: r.status, statusText: r.statusText, finalUrl: r.url, readyState: 4,
        responseHeaders: [...r.headers].map(([k, v]) => k + ': ' + v).join('\r\n'),
        responseText: text, response: d.responseType === 'json' ? (() => { try { return JSON.parse(text); } catch { return null; } })() : text });
      fetch(d.url, { method: d.method || 'GET', headers: d.headers, body: d.data, signal: ctrl.signal,
                     credentials: d.anonymous ? 'omit' : 'include' })
        .then(async (r) => { const t = await r.text(); const o = resp(r, t); d.onload && d.onload(o); d.onloadend && d.onloadend(o); })
        .catch((e) => { const o = { status: 0, error: String(e), readyState: 4 }; (d.onerror || d.onabort || (() => {}))(o); d.onloadend && d.onloadend(o); });
      return { abort: () => ctrl.abort() };
    },
  };
  api.GM = {
    info,
    getValue: async (k, d) => api.GM_getValue(k, d),
    setValue: async (k, v) => api.GM_setValue(k, v),
    deleteValue: async (k) => api.GM_deleteValue(k),
    listValues: async () => api.GM_listValues(),
    addStyle: async (c) => api.GM_addStyle(c),
    addElement: (...a) => api.GM_addElement(...a),
    openInTab: async (u, o) => api.GM_openInTab(u, o),
    setClipboard: async (t) => api.GM_setClipboard(t),
    notification: async (d) => api.GM_notification(d),
    registerMenuCommand: async (...a) => api.GM_registerMenuCommand(...a),
    xmlHttpRequest: (d) => api.GM_xmlhttpRequest(d),
  };
  return api;
}
function __mizuWhen(runAt, fn) {
  const run = () => { try { fn(); } catch (e) { console.error('[Mizu Scripts]', e); } };
  if (runAt === 'document-start') return run();
  if (runAt === 'document-body') {
    if (document.body) return run();
    const mo = new MutationObserver(() => { if (document.body) { mo.disconnect(); run(); } });
    return mo.observe(document.documentElement, { childList: true, subtree: true });
  }
  const ready = () => (runAt === 'document-idle' ? setTimeout(run, 0) : run());
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', ready, { once: true });
  else ready();
}
"#;

fn js_string(s: &str) -> String {
    // serde_json produces a valid JS string literal; additionally escape U+2028/2029 and
    // `</` for safety when embedded anywhere.
    serde_json::to_string(s)
        .unwrap_or_else(|_| "\"\"".into())
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
        .replace("</", "<\\/")
}

fn run_at(r: RunAt) -> &'static str {
    match r {
        RunAt::DocumentStart => "document-start",
        RunAt::DocumentEnd => "document-end",
        RunAt::DocumentIdle => "document-idle",
        RunAt::DocumentBody => "document-body",
    }
}

/// Builds one document-start script that runs every given userscript on a top-level page
/// whose origin equals `expected_origin`.
pub fn build_injection(
    scripts: &[(&UserScript, serde_json::Map<String, serde_json::Value>)],
    token: &str,
    expected_origin: &str,
) -> String {
    let mut out = String::with_capacity(8192);
    out.push_str("(() => {\n'use strict';\n");
    out.push_str("if (window.top !== window) return;\n");
    out.push_str(&format!("if (location.origin !== {}) return;\n", js_string(expected_origin)));
    out.push_str(RUNTIME);
    for (script, values) in scripts {
        let m = &script.meta;
        let info = serde_json::json!({
            "scriptHandler": "Mizu Scripts",
            "version": env!("CARGO_PKG_VERSION"),
            "script": {
                "name": m.name, "namespace": m.namespace, "version": m.version,
                "description": m.description, "author": m.author,
                "matches": m.matches, "includes": m.includes, "excludes": m.excludes,
                "grant": m.grants, "run-at": run_at(m.run_at),
            },
        });
        let values = serde_json::Value::Object(values.clone());
        out.push_str(&format!(
            "{{ const __api = __mizuMakeApi({token}, {sid}, {info}, {values});\n\
             __mizuWhen({run_at}, () => {{\n\
             (function (GM_info, GM, unsafeWindow, GM_log, GM_getValue, GM_setValue, GM_deleteValue, GM_listValues,\n\
                        GM_addStyle, GM_addElement, GM_openInTab, GM_setClipboard, GM_registerMenuCommand,\n\
                        GM_unregisterMenuCommand, GM_notification, GM_xmlhttpRequest) {{\n\
             {requires}\n{source}\n\
             }}).call(window, __api.GM_info, __api.GM, __api.unsafeWindow, __api.GM_log, __api.GM_getValue,\n\
                        __api.GM_setValue, __api.GM_deleteValue, __api.GM_listValues, __api.GM_addStyle,\n\
                        __api.GM_addElement, __api.GM_openInTab, __api.GM_setClipboard, __api.GM_registerMenuCommand,\n\
                        __api.GM_unregisterMenuCommand, __api.GM_notification, __api.GM_xmlhttpRequest);\n\
             }}); }}\n",
            token = js_string(token),
            sid = js_string(&script.id),
            info = info,
            values = values,
            run_at = js_string(run_at(m.run_at)),
            requires = script.requires,
            source = script.source,
        ));
    }
    out.push_str("})();\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ScriptStore;

    #[test]
    fn builds_bootstrap() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ScriptStore::open(dir.path());
        let id = store
            .install(
                "// ==UserScript==\n// @name T\n// @match https://a.example/*\n// ==/UserScript==\nGM_setValue('x', 1);",
                "",
                "",
            )
            .unwrap();
        let s = store.get(&id).unwrap();
        let js = build_injection(&[(s, store.values(&id))], "tok</script>", "https://a.example");
        assert!(js.contains("GM_setValue('x', 1);"));
        assert!(js.contains("\"https://a.example\""));
        assert!(!js.contains("tok</script>"));
        assert!(js.contains("if (window.top !== window) return;"));
    }

    #[test]
    fn parses_messages() {
        let m: GmMessage = serde_json::from_str(
            r#"{"op":"set","token":"t","sid":"s","key":"k","value":[1,2]}"#,
        )
        .unwrap();
        assert_eq!(m.token(), "t");
        assert_eq!(m.script_id(), "s");
    }
}
