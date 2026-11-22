//! Media hub: what is playing in which tab, with play/pause, previous/next and a volume that
//! is remembered per site. A small document script reports the page's media state (and the
//! track details sites publish through the Media Session API) and exposes a control function
//! named with a per-page token; the browser calls it through `ExecuteScript`.

use std::collections::BTreeMap;

use mizu_engine::ViewKey;
use serde::Serialize;
use serde_json::Value;

use crate::app::App;
use crate::hot::{self, Role};

/// The page side. `__TOKEN__` is a random per-document token, `__VOL__` the remembered volume
/// for the site (0..1) or -1. Media Session action handlers registered by the page are kept so
/// the browser can trigger them, as the system media keys would.
const MEDIA_SCRIPT: &str = r#"(()=>{const wv=window.chrome&&window.chrome.webview;if(!wv)return;const post=wv.postMessage.bind(wv);const T=__TOKEN__;let VOL=__VOL__;const H={};const ms=navigator.mediaSession;
if(ms&&ms.setActionHandler){const orig=ms.setActionHandler.bind(ms);try{ms.setActionHandler=function(a,f){H[a]=f;try{return orig(a,f);}catch(e){}};}catch(e){}}
const els=()=>[...document.querySelectorAll('video,audio')];
const active=()=>{const l=els();return l.find(e=>!e.paused&&!e.ended)||l.find(e=>e.currentTime>0&&!e.ended)||null;};
let last='',timer=0,seen=false;
function report(){timer=0;const e=active();const md=ms&&ms.metadata;if(!e&&!seen)return;seen=true;let art='';try{if(md&&md.artwork&&md.artwork.length){art=String(md.artwork[md.artwork.length-1].src||'');}}catch(x){}
const st={token:T,title:String(md&&md.title||''),artist:String(md&&md.artist||''),album:String(md&&md.album||''),artwork:art.slice(0,2048),playing:!!(e&&!e.paused&&!e.ended),prev:!!H.previoustrack,next:!!H.nexttrack,video:!!(e&&e.tagName==='VIDEO'),volume:e?e.volume:1,muted:!!(e&&e.muted),position:e?Math.floor(e.currentTime):0,duration:e&&isFinite(e.duration)?Math.floor(e.duration):0};
const k=JSON.stringify(Object.assign({},st,{position:Math.floor(st.position/10)}));if(k===last)return;last=k;post({mizu_media:st});}
const soon=()=>{if(!timer)timer=setTimeout(report,200);};
for(const ev of ['play','pause','ended','volumechange','loadedmetadata','emptied','seeked'])document.addEventListener(ev,soon,true);
document.addEventListener('play',(ev)=>{const t=ev.target;if(VOL>=0&&t&&typeof t.volume==='number'){try{t.volume=VOL;}catch(x){}}},true);
setInterval(()=>{if(seen)soon();},4000);
const call=(a)=>{const f=H[a];if(!f)return false;try{f({action:a});return true;}catch(x){return false;}};
Object.defineProperty(window,'__mizuMedia_'+T,{value:(cmd,v)=>{const e=active()||els()[0];
if(cmd==='toggle'){if(e&&!e.paused){call('pause')||e.pause();}else{call('play')||(e&&e.play().catch(()=>{}));}}
else if(cmd==='pause'){if(!call('pause'))els().forEach(x=>x.pause());}
else if(cmd==='play'){call('play')||(e&&e.play().catch(()=>{}));}
else if(cmd==='next'){if(!call('nexttrack')&&e&&isFinite(e.duration))e.currentTime=e.duration;}
else if(cmd==='prev'){if(!call('previoustrack')&&e)e.currentTime=0;}
else if(cmd==='volume'){VOL=v;els().forEach(x=>{try{x.volume=v;x.muted=false;}catch(y){}});}
setTimeout(report,180);},enumerable:false,configurable:false});
})();"#;

pub fn script(token: &str, volume: Option<u32>) -> String {
    let vol = volume.map_or("-1".to_owned(), |v| format!("{}", f64::from(v.min(100)) / 100.0));
    MEDIA_SCRIPT
        .replace("__TOKEN__", &serde_json::to_string(token).unwrap_or_default())
        .replace("__VOL__", &vol)
}

/// What one tab is playing.
#[derive(Debug, Clone, Serialize)]
pub struct MediaInfo {
    #[serde(rename = "tab")]
    pub tab_raw: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork: String,
    pub playing: bool,
    pub prev: bool,
    pub next: bool,
    pub video: bool,
    pub volume: f64,
    pub muted: bool,
    pub position: u64,
    pub duration: u64,
    pub site: String,
    pub page_title: String,
    #[serde(skip)]
    pub private: bool,
    /// Order of the last change, newest first in lists.
    #[serde(skip)]
    pub stamp: u64,
}

#[derive(Default)]
pub struct MediaHub {
    pub items: BTreeMap<ViewKey, MediaInfo>,
    next_stamp: u64,
}

impl MediaHub {
    /// Tabs with media for a window kind (private windows only see private tabs), playing
    /// first, then most recently changed.
    pub fn list(&self, private: bool) -> Vec<&MediaInfo> {
        let mut v: Vec<&MediaInfo> = self.items.values().filter(|m| m.private == private).collect();
        v.sort_by(|a, b| b.playing.cmp(&a.playing).then(b.stamp.cmp(&a.stamp)));
        v
    }

    pub fn to_json(&self, private: bool) -> Value {
        serde_json::to_value(self.list(private)).unwrap_or(Value::Array(Vec::new()))
    }

    /// (tabs with media, any playing) for the toolbar button.
    pub fn summary(&self, private: bool) -> (usize, bool) {
        let l = self.list(private);
        (l.len(), l.iter().any(|m| m.playing))
    }

    pub fn forget(&mut self, key: ViewKey) -> bool {
        self.items.remove(&key).is_some()
    }
}

fn clip(v: &Value, key: &str, max: usize) -> String {
    let s = v[key].as_str().unwrap_or_default();
    s.chars().take(max).collect()
}

/// `{"mizu_media": …}` from a page: verify the token, then update the hub.
pub fn on_report(app: &mut App, key: ViewKey, value: &Value) {
    let Some(raw) = value.get("mizu_media") else { return };
    let token = raw["token"].as_str().unwrap_or_default();
    let (valid, top, private) = hot::with(|h| {
        (
            h.media_tokens.get(&key).is_some_and(|t| t == token),
            h.top_urls.get(&key).cloned().unwrap_or_default(),
            h.private.contains(&key),
        )
    })
    .unwrap_or_default();
    if !valid {
        return;
    }
    let Some(Role::Tab { window, tab }) = hot::role(key) else { return };
    let page_title = app.window(window).and_then(|w| w.strip.get(tab)).map(|t| t.title.clone()).unwrap_or_default();
    let artwork = clip(raw, "artwork", 2048);
    let artwork = if artwork.starts_with("https://") || artwork.starts_with("http://") || artwork.starts_with("data:image/") { artwork } else { String::new() };
    let playing = raw["playing"].as_bool().unwrap_or(false);
    let was_playing = app.media.items.get(&key).is_some_and(|m| m.playing);
    app.media.next_stamp += 1;
    let info = MediaInfo {
        tab_raw: tab.0,
        title: clip(raw, "title", 300),
        artist: clip(raw, "artist", 200),
        album: clip(raw, "album", 200),
        artwork,
        playing,
        prev: raw["prev"].as_bool().unwrap_or(false),
        next: raw["next"].as_bool().unwrap_or(false),
        video: raw["video"].as_bool().unwrap_or(false),
        volume: raw["volume"].as_f64().unwrap_or(1.0).clamp(0.0, 1.0),
        muted: raw["muted"].as_bool().unwrap_or(false),
        position: raw["position"].as_u64().unwrap_or(0),
        duration: raw["duration"].as_u64().unwrap_or(0),
        site: mizu_core::omnibox::host_of(&top).unwrap_or_default().trim_start_matches("www.").to_owned(),
        page_title,
        private,
        stamp: app.media.next_stamp,
    };
    app.media.items.insert(key, info);

    // "One sound at a time": a tab that starts playing pauses the others.
    if playing && !was_playing && app.settings.media.pause_others {
        let others: Vec<ViewKey> = app
            .media
            .items
            .iter()
            .filter(|(k, m)| **k != key && m.playing && m.private == private)
            .map(|(k, _)| *k)
            .collect();
        for k in others {
            control(k, "pause", None);
        }
    }
    app.media_dirty = true;
}

/// Runs a control command in a tab's page: toggle, play, pause, next, prev, volume.
pub fn control(key: ViewKey, cmd: &str, value: Option<f64>) {
    let Some(token) = hot::with(|h| h.media_tokens.get(&key).cloned()).flatten() else { return };
    let Some(view) = hot::view(key) else { return };
    let name = serde_json::to_string(&format!("__mizuMedia_{token}")).unwrap_or_default();
    let cmd = serde_json::to_string(cmd).unwrap_or_default();
    let v = value.map_or("null".to_owned(), |v| format!("{v}"));
    view.execute_script(&format!("(()=>{{const f=window[{name}];if(typeof f==='function')f({cmd},{v});}})();"));
}

impl App {
    /// Finds the view key of a tab (for media commands that name a tab id).
    pub fn media_key_for_tab(&self, tab: u64) -> Option<ViewKey> {
        self.media.items.iter().find(|(_, m)| m.tab_raw == tab).map(|(k, _)| *k)
    }

    /// The tab that the media keys and shortcuts act on: the one playing, or the latest.
    pub fn media_current(&self, private: bool) -> Option<ViewKey> {
        let first = self.media.list(private).first().map(|m| m.tab_raw)?;
        self.media_key_for_tab(first)
    }

    /// Remembers a site's volume (normal windows only) and applies it to the tab now.
    pub fn set_media_volume(&mut self, key: ViewKey, volume: f64) {
        let volume = volume.clamp(0.0, 1.0);
        control(key, "volume", Some(volume));
        let (top, private) = hot::with(|h| (h.top_urls.get(&key).cloned().unwrap_or_default(), h.private.contains(&key))).unwrap_or_default();
        if private {
            return;
        }
        if let Some(origin) = mizu_core::omnibox::origin_of(&top) {
            let pct = (volume * 100.0).round() as u32;
            let map = &mut self.settings.media.site_volumes;
            if pct >= 100 {
                map.remove(&origin);
            } else {
                map.insert(origin.clone(), pct);
            }
            let _ = hot::with(|h| {
                if pct >= 100 {
                    h.site_volumes.remove(&origin);
                } else {
                    h.site_volumes.insert(origin, pct);
                }
            });
            self.persist_settings();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::script;

    #[test]
    fn script_carries_token_and_volume() {
        let s = script("abc", Some(40));
        assert!(s.contains("const T=\"abc\""));
        assert!(s.contains("let VOL=0.4"));
        assert!(script("x", None).contains("let VOL=-1"));
    }
}
