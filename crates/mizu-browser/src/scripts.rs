//! Per-navigation document scripts: Global Privacy Control, cosmetic filtering, uBlock
//! scriptlets and Mizu Scripts (userscripts); plus the userscript install flow.

use crate::hot::Hot;
use crate::queue::{WorkerResult, WorkerSender};
use mizu_engine::ViewKey;
use mizu_userscripts::{GmMessage, ScriptMeta, build_injection};
use std::hash::{BuildHasher, Hasher};

/// Picture-in-picture: a small button over videos (in a closed shadow root so pages cannot
/// style or script it) and Alt+Shift+P. Clicking or pressing keys is a real user gesture, which
/// the Picture-in-Picture API requires. `disablePictureInPicture` is overridden, like Firefox.
const PIP_SCRIPT: &str = r#"(()=>{const C=__MIZU_PIP__;if(!('requestPictureInPicture' in HTMLVideoElement.prototype))return;
let host=null,btn=null,target=null,hideT=0,raf=0,px=0,py=0;
const NS='http://www.w3.org/2000/svg';
// Built with DOM calls only: sites with Trusted Types (YouTube) forbid innerHTML.
function icon(){const s=document.createElementNS(NS,'svg');const set=(e,a)=>{for(const k in a)e.setAttribute(k,a[k]);return e;};
set(s,{viewBox:'0 0 24 24',width:'16',height:'16',fill:'none',stroke:'currentColor','stroke-width':'1.8','stroke-linejoin':'round'});
s.append(set(document.createElementNS(NS,'rect'),{x:'2.5',y:'4.5',width:'19',height:'15',rx:'2.5'}),set(document.createElementNS(NS,'rect'),{x:'12',y:'11.5',width:'7',height:'5.5',rx:'1.2',fill:'currentColor',stroke:'none'}));return s;}
function ui(){if(host&&host.isConnected)return;host=document.createElement('mizu-pip');const root=host.attachShadow({mode:'closed'});
const st=document.createElement('style');st.textContent=':host{all:initial;position:fixed;z-index:2147483647;display:none}button{all:initial;display:flex;align-items:center;gap:6px;padding:'+(C.compact?'7px':'6px 11px 6px 9px')+';border-radius:999px;background:rgba(18,24,32,.84);color:#fff;font:600 12px/1 "Segoe UI",system-ui,sans-serif;cursor:pointer;-webkit-backdrop-filter:blur(8px);backdrop-filter:blur(8px);box-shadow:0 2px 12px rgba(0,0,0,.4);opacity:.94;transition:opacity .15s,transform .15s}button:hover{opacity:1;transform:scale(1.06)}';
btn=document.createElement('button');btn.title='Picture in picture (Alt+Shift+P)';btn.append(icon());
if(!C.compact){const t=document.createElement('span');t.textContent='PiP';btn.append(t);}
btn.addEventListener('click',e=>{e.preventDefault();e.stopPropagation();toggle(target);},true);
for(const ev of ['mousedown','mouseup','pointerdown','pointerup','dblclick'])btn.addEventListener(ev,e=>e.stopPropagation(),true);
host.addEventListener('mouseenter',()=>clearTimeout(hideT));
root.append(st,btn);(document.body||document.documentElement).appendChild(host);}
function hide(ms){clearTimeout(hideT);hideT=setTimeout(()=>{if(host)host.style.display='none';target=null;},ms);}
function place(v){const r=v.getBoundingClientRect();if(r.width<200||r.height<120){hide(0);return;}ui();clearTimeout(hideT);target=v;host.style.display='block';
const w=btn.offsetWidth||64;host.style.top=Math.max(8,r.top+12)+'px';host.style.left=(C.position==='top-left'?Math.max(8,r.left+12):Math.max(8,r.right-w-12))+'px';}
async function toggle(v){try{if(document.pictureInPictureElement){await document.exitPictureInPicture();return;}if(!v)return;
v.disablePictureInPicture=false;v.removeAttribute('disablepictureinpicture');await v.requestPictureInPicture();}catch(e){console.warn('[Mizu PiP]',e);}}
function best(){const vs=[...document.querySelectorAll('video')].filter(v=>v.readyState>0);vs.sort((a,b)=>(!b.paused)-(!a.paused)||b.clientWidth*b.clientHeight-a.clientWidth*a.clientHeight);return vs[0];}
// Players cover the video with their own controls, so look underneath the pointer.
function videoAt(x,y){for(const el of document.elementsFromPoint(x,y)){if(el.tagName==='VIDEO')return el;if(host&&el===host)return target;}
return [...document.querySelectorAll('video')].find(v=>{const r=v.getBoundingClientRect();return x>=r.left&&x<=r.right&&y>=r.top&&y<=r.bottom;})||null;}
function check(){raf=0;const v=videoAt(px,py);if(v)place(v);else if(target)hide(700);}
if(C.button){document.addEventListener('mousemove',e=>{px=e.clientX;py=e.clientY;if(!raf)raf=requestAnimationFrame(check);},{capture:true,passive:true});
document.addEventListener('scroll',()=>{if(host&&host.style.display!=='none')hide(0);},{capture:true,passive:true});}
document.addEventListener('keydown',e=>{if(e.altKey&&e.shiftKey&&e.code==='KeyP'){e.preventDefault();toggle(best());}},true);
// Buttons in the picture-in-picture window: previous/next video and 10-second skips.
function clickFirst(sels){for(const s of sels){const el=document.querySelector(s);if(el&&el.offsetParent!==null&&!el.disabled&&el.getAttribute('aria-disabled')!=='true'){el.click();return true;}}return false;}
function otherVideo(v,dir){const vs=[...document.querySelectorAll('video')].filter(x=>x.readyState>0&&x!==v);if(!vs.length)return null;const all=[...document.querySelectorAll('video')];const i=all.indexOf(v);
const after=all.filter((x,j)=>vs.includes(x)&&(dir>0?j>i:j<i));return (dir>0?after[0]:after[after.length-1])||null;}
async function switchTo(v,dir){const sels=dir>0?['.ytp-next-button','[aria-label="Next"]','[aria-label="Next video"]','[data-testid="next-button"]','button[title^="Next"]']:['.ytp-prev-button','[aria-label="Previous"]','[aria-label="Previous video"]','[data-testid="previous-button"]','button[title^="Previous"]'];
if(clickFirst(sels))return;const o=otherVideo(v,dir);if(o){try{await o.requestPictureInPicture();o.play().catch(()=>{});}catch(e){console.warn('[Mizu PiP]',e);}}else if(dir<0){v.currentTime=0;}}
function controls(v){if(!C.controls||!('mediaSession' in navigator))return;const ms=navigator.mediaSession;const set=(a,f)=>{try{ms.setActionHandler(a,f);}catch(_){}};
set('previoustrack',()=>switchTo(v,-1));set('nexttrack',()=>switchTo(v,1));
set('seekbackward',d=>{v.currentTime=Math.max(0,v.currentTime-((d&&d.seekOffset)||10));});set('seekforward',d=>{v.currentTime=Math.min(v.duration||1e9,v.currentTime+((d&&d.seekOffset)||10));});}
document.addEventListener('enterpictureinpicture',e=>controls(e.target),true);
})();"#;

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

    // The shortcut always works; the hover button follows the settings.
    parts.push(PIP_SCRIPT.replace("__MIZU_PIP__", &h.pip_config));

    // Mizu Passwords: notice submitted logins (never in private windows).
    if h.password_capture && !h.private.contains(&key) {
        let token = random_token();
        parts.push(crate::passwords::capture_script(&token));
        h.pw_tokens.insert(key, token);
    } else {
        h.pw_tokens.remove(&key);
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

/// A downloaded userscript ready to be installed.
#[derive(Debug, Clone)]
pub struct Fetched {
    pub url: String,
    pub source: String,
    pub requires: String,
}

/// Downloads a script from `url`, trying repository-page rewrites and mirrors in order.
fn fetch_script_text(url: &str) -> Result<(String, String), String> {
    let candidates = mizu_userscripts::install_candidates(url);
    if candidates.is_empty() {
        return Err("unsupported link".into());
    }
    let mut last_err = String::new();
    for c in &candidates {
        match fetch(c) {
            Ok(text) if text.contains("==UserScript==") => return Ok((c.clone(), text)),
            Ok(_) => last_err = "the page does not contain a userscript".into(),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

fn fetch_requires(meta: &ScriptMeta) -> Result<String, String> {
    let mut requires = String::new();
    for req in &meta.requires {
        let (_, text) = fetch_script_text(req)
            .or_else(|_| fetch(req).map(|t| (req.clone(), t)))
            .map_err(|e| format!("@require {req}: {e}"))?;
        requires.push_str(&text);
        requires.push_str("\n;\n");
    }
    Ok(requires)
}

/// Downloads userscripts (and their `@require` libraries) from several links on a worker.
/// A file containing several scripts back to back yields several entries.
pub fn fetch_for_install(key: Option<ViewKey>, urls: Vec<String>, sender: WorkerSender) {
    std::thread::spawn(move || {
        let mut items = Vec::new();
        let mut failures = Vec::new();
        for url in urls.into_iter().filter(|u| !u.trim().is_empty()).take(50) {
            match fetch_script_text(url.trim()) {
                Ok((resolved, text)) => {
                    for source in mizu_userscripts::split_scripts(&text) {
                        match ScriptMeta::parse(&source).map_err(|e| e.to_string()).and_then(|m| fetch_requires(&m)) {
                            Ok(requires) => items.push(Fetched { url: resolved.clone(), source, requires }),
                            Err(e) => failures.push((url.clone(), e)),
                        }
                    }
                }
                Err(e) => failures.push((url, e)),
            }
        }
        sender.send(WorkerResult::ScriptsFetched { key, items, failures });
    });
}

/// Reads local `.user.js` files (already chosen by the user) on a worker.
pub fn read_files_for_install(key: Option<ViewKey>, paths: Vec<String>, sender: WorkerSender) {
    let urls = paths
        .into_iter()
        .filter_map(|p| url::Url::from_file_path(&p).ok().map(|u| u.to_string()))
        .collect();
    fetch_for_install(key, urls, sender);
}

/// Checks scripts for newer versions and downloads the ones that changed.
pub fn check_updates(checks: Vec<mizu_userscripts::UpdateCheck>, reply: Option<(ViewKey, u64)>, sender: WorkerSender) {
    std::thread::spawn(move || {
        let mut updated = Vec::new();
        let mut checked = Vec::new();
        let mut failed = 0;
        for c in checks {
            let result = (|| -> Result<Option<Fetched>, String> {
                let (_, head) = fetch_script_text(&c.check_url)?;
                let remote = ScriptMeta::parse(&head).map_err(|e| e.to_string())?;
                if mizu_userscripts::compare_versions(&remote.version, &c.version) != std::cmp::Ordering::Greater {
                    return Ok(None);
                }
                let (url, source) = if c.check_url == c.download_url {
                    (c.download_url.clone(), head)
                } else {
                    fetch_script_text(&c.download_url)?
                };
                let meta = ScriptMeta::parse(&source).map_err(|e| e.to_string())?;
                if meta.identity() != format!("{}::{}", remote.namespace, remote.name) && meta.name != c.name {
                    return Err("downloaded script does not match the installed one".into());
                }
                let requires = fetch_requires(&meta)?;
                Ok(Some(Fetched { url, source, requires }))
            })();
            match result {
                Ok(Some(f)) => {
                    tracing::info!(script = c.name, "userscript update downloaded");
                    updated.push(f);
                    checked.push(c.id);
                }
                Ok(None) => checked.push(c.id),
                Err(e) => {
                    tracing::warn!(script = c.name, error = %e, "userscript update check failed");
                    failed += 1;
                    checked.push(c.id);
                }
            }
        }
        sender.send(WorkerResult::ScriptUpdates { updated, checked, failed, reply });
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
    if !url.starts_with("https://") {
        return Err("only https:// and local files are supported".into());
    }
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

/// Settings the picture-in-picture script reads (`__MIZU_PIP__`).
pub fn pip_config(m: &mizu_settings::MediaSettings) -> String {
    let position = if m.pip_position == "top-left" { "top-left" } else { "top-right" };
    serde_json::json!({ "button": m.pip_button, "position": position, "compact": m.pip_compact, "controls": m.pip_controls }).to_string()
}
