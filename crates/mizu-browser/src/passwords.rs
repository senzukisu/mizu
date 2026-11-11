//! Mizu Passwords: the password manager.
//!
//! * Storage: `mizu-storage` keeps only DPAPI-encrypted secrets ([`crate::vault`]).
//! * Saving: a document script notices login form submissions and posts the entered
//!   credentials with a per-page token; the browser offers to save them in a flyout.
//! * Filling: when the current site has saved logins, a key button appears in the address
//!   bar; choosing an account fills the page's login form.
//! * Import/export: CSV compatible with Chrome, Edge, Firefox, Bitwarden and others.

use crate::app::App;
use crate::commands::{reply, reply_err};
use crate::hot::{self, Role};
use crate::queue;
use crate::vault;
use mizu_core::ids::WindowId;
use mizu_engine::ViewKey;
use mizu_ui::protocol::{Command, FlyoutKind};
use serde_json::{Value, json};

type CmdResult = Result<Option<Value>, String>;

/// Credentials captured from a page, waiting for the user's decision.
pub struct PendingPassword {
    pub window: WindowId,
    pub origin: String,
    pub username: String,
    pub password: String,
}

fn js(s: &str) -> String {
    serde_json::to_string(s)
        .unwrap_or_else(|_| "\"\"".into())
        .replace("</", "<\\/")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// Document-start script that reports submitted logins (top frame only).
pub fn capture_script(token: &str) -> String {
    format!(
        r#"(()=>{{const wv=window.chrome&&window.chrome.webview;if(!wv)return;const post=wv.postMessage.bind(wv);const TOKEN={token};let last='';
const visible=(el)=>!!(el&&(el.offsetWidth||el.offsetHeight||el.getClientRects().length));
function userFor(pw,scope){{const inputs=[...scope.querySelectorAll('input')];const i=inputs.indexOf(pw);for(let j=i-1;j>=0;j--){{const t=(inputs[j].type||'text').toLowerCase();if(['text','email','tel'].includes(t)&&inputs[j].value)return inputs[j].value;}}
const hint=scope.querySelector('input[autocomplete="username"],input[type="email"]');return hint&&hint.value||'';}}
function grab(scope){{scope=scope&&scope.querySelectorAll?scope:document;const pws=[...scope.querySelectorAll('input[type="password"]')].filter(p=>p.value);if(!pws.length)return;
const pw=pws.length>1?pws[pws.length-1]:pws[0];const user=userFor(pw,scope);const key=user+'\u0000'+pw.value;if(key===last)return;last=key;post({{mizu_pw:{{token:TOKEN,username:user,password:pw.value,signup:pws.length>1}}}});}}
document.addEventListener('submit',(e)=>grab(e.target),true);
document.addEventListener('click',(e)=>{{const b=e.target&&e.target.closest&&e.target.closest('button,input[type="submit"],input[type="button"],[role="button"]');if(!b)return;const f=b.closest('form');setTimeout(()=>grab(f||document),0);}},true);
document.addEventListener('keydown',(e)=>{{if(e.key==='Enter'&&e.target&&e.target.type==='password')grab(e.target.form||document);}},true);
}})();"#,
        token = js(token)
    )
}

fn fill_script(origin: &str, username: &str, password: &str) -> String {
    format!(
        r#"(()=>{{if(location.origin!=={origin})return;const U={user},P={pass};
const setv=(el,v)=>{{const d=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value');d.set.call(el,v);el.dispatchEvent(new Event('input',{{bubbles:true}}));el.dispatchEvent(new Event('change',{{bubbles:true}}));}};
const vis=(el)=>!!(el&&(el.offsetWidth||el.offsetHeight||el.getClientRects().length));
const pws=[...document.querySelectorAll('input[type="password"]')];const pw=pws.find(vis)||pws[0];
const users=[...document.querySelectorAll('input[autocomplete="username"],input[type="email"],input[type="text"],input[type="tel"],input:not([type])')].filter(vis);
let user=null;if(pw){{const all=[...document.querySelectorAll('input')];const i=all.indexOf(pw);for(let j=i-1;j>=0;j--){{const t=(all[j].type||'text').toLowerCase();if(['text','email','tel'].includes(t)&&vis(all[j])){{user=all[j];break;}}}}}}
if(!user)user=users.find(u=>/user|login|mail|phone|name/i.test((u.name||'')+(u.id||'')+(u.autocomplete||'')))||users[0];
if(user&&U)setv(user,U);if(pw){{setv(pw,P);pw.focus();}}else if(user)user.focus();}})();"#,
        origin = js(origin),
        user = js(username),
        pass = js(password)
    )
}

/// Handles `{"mizu_pw": …}` from web content: verify the token, then offer to save.
pub fn on_capture(app: &mut App, key: ViewKey, value: &Value) {
    let Some(raw) = value.get("mizu_pw") else { return };
    let token = raw["token"].as_str().unwrap_or_default();
    let username = raw["username"].as_str().unwrap_or_default().trim().to_owned();
    let password = raw["password"].as_str().unwrap_or_default().to_owned();
    let (valid, top) = hot::with(|h| {
        (h.pw_tokens.get(&key).is_some_and(|t| t == token), h.top_urls.get(&key).cloned().unwrap_or_default())
    })
    .unwrap_or_default();
    if !valid || password.is_empty() || password.len() > 4096 || username.len() > 512 {
        return;
    }
    let Some(Role::Tab { window, .. }) = hot::role(key) else { return };
    if app.window(window).is_none_or(|w| w.private) {
        return;
    }
    let Some(origin) = mizu_core::omnibox::origin_of(&top) else { return };
    if app.storage.is_never_save(&origin).unwrap_or(false) {
        return;
    }
    // Already saved with the same password: nothing to ask.
    let existing = app.storage.logins_for_origin(&origin).unwrap_or_default();
    let same = existing.iter().find(|l| l.username == username);
    if let Some(l) = same {
        if vault::decrypt(&l.secret).is_ok_and(|p| p == password) {
            let _ = app.storage.touch_login(l.id);
            return;
        }
    }
    let update = same.is_some();
    let host = mizu_core::omnibox::host_of(&origin).unwrap_or_default();
    let data = json!({
        "origin": origin,
        "host": host.trim_start_matches("www."),
        "username": username,
        "password": password,
        "update": update,
        "accounts": existing.len(),
    });
    app.pending_password = Some(PendingPassword { window, origin, username, password });
    let anchor = app.window(window).map(|w| w.omnibox_anchor).unwrap_or_default();
    app.show_flyout(window, FlyoutKind::SavePassword, anchor, data, None);
}

/// Accounts for the address bar's key flyout.
pub fn logins_flyout_data(app: &App, window: WindowId) -> Value {
    let url = app.current_url(window).unwrap_or_default();
    let origin = mizu_core::omnibox::origin_of(&url).unwrap_or_default();
    let logins: Vec<Value> = app
        .storage
        .logins_for_origin(&origin)
        .unwrap_or_default()
        .into_iter()
        .map(|l| json!({ "id": l.id, "username": l.username, "last_used": l.last_used }))
        .collect();
    json!({ "origin": origin, "host": mizu_core::omnibox::host_of(&origin).unwrap_or_default().trim_start_matches("www."), "logins": logins })
}

fn fill(app: &mut App, window: WindowId, id: i64) -> CmdResult {
    let login = app.storage.login(id).map_err(|e| e.to_string())?;
    let url = app.current_url(window).unwrap_or_default();
    if mizu_core::omnibox::origin_of(&url).as_deref() != Some(login.origin.as_str()) {
        return Err("This login belongs to a different site".into());
    }
    let password = vault::decrypt(&login.secret)?;
    if let Some(view) = app.window(window).and_then(|w| w.active_view()) {
        view.execute_script(&fill_script(&login.origin, &login.username, &password));
        view.focus();
    }
    let _ = app.storage.touch_login(id);
    if let Some(w) = app.window_mut(window) {
        w.flyout.hide();
    }
    Ok(Some(Value::Null))
}

fn csv_rows(app: &App) -> Result<Vec<vault::CsvLogin>, String> {
    let mut out = Vec::new();
    for l in app.storage.logins("").map_err(|e| e.to_string())? {
        out.push(vault::CsvLogin { origin: l.origin, username: l.username, password: vault::decrypt(&l.secret)?, note: l.note });
    }
    Ok(out)
}

pub fn handle(app: &mut App, window: WindowId, key: ViewKey, rid: Option<u64>, cmd: Command) -> CmdResult {
    match cmd {
        Command::PasswordsGet { query } => {
            let list: Vec<Value> = app
                .storage
                .logins(&query)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|l| {
                    json!({
                        "id": l.id,
                        "origin": l.origin,
                        "host": mizu_core::omnibox::host_of(&l.origin).unwrap_or_default().trim_start_matches("www."),
                        "username": l.username,
                        "note": l.note,
                        "updated_at": l.updated_at,
                        "last_used": l.last_used,
                    })
                })
                .collect();
            let never = app.storage.never_save_sites().map_err(|e| e.to_string())?;
            Ok(Some(json!({ "logins": list, "never": never, "offer_to_save": app.settings.privacy.password_manager })))
        }
        Command::PasswordReveal { id } => {
            let login = app.storage.login(id).map_err(|e| e.to_string())?;
            Ok(Some(json!({ "password": vault::decrypt(&login.secret)? })))
        }
        Command::PasswordSave { id, origin, username, password, note } => {
            let with_scheme = if origin.contains("://") { origin } else { format!("https://{origin}") };
            let origin = mizu_core::omnibox::origin_of(&with_scheme).ok_or("Enter the site address, e.g. https://example.com")?;
            let secret = match &password {
                Some(p) if p.is_empty() => return Err("The password cannot be empty".into()),
                Some(p) => Some(vault::encrypt(p)?),
                None => None,
            };
            let id = match (id, secret) {
                (Some(id), secret) => {
                    app.storage.update_login(id, &origin, username.trim(), secret.as_deref(), &note).map_err(|e| e.to_string())?;
                    id
                }
                (None, Some(secret)) => app.storage.upsert_login(&origin, username.trim(), &secret, &note).map_err(|e| e.to_string())?,
                (None, None) => return Err("The password cannot be empty".into()),
            };
            mark_dirty(app);
            Ok(Some(json!({ "id": id })))
        }
        Command::PasswordDelete { id } => {
            app.storage.delete_login(id).map_err(|e| e.to_string())?;
            mark_dirty(app);
            Ok(Some(Value::Null))
        }
        Command::PasswordNeverRemove { origin } => {
            app.storage.set_never_save(&origin, false).map_err(|e| e.to_string())?;
            Ok(Some(Value::Null))
        }
        Command::PasswordsImport => {
            let owner = app.window(window).map(|w| w.hwnd).unwrap_or_default();
            queue::outside(move || {
                let Some(path) = crate::dialogs::pick_file(owner, "Import passwords (CSV)", "Password export (CSV)", "*.csv") else {
                    if let Some(rid) = rid {
                        reply(key, rid, Value::Null);
                    }
                    return;
                };
                let parsed = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| vault::import_csv(&t));
                queue::with_app(move |app| {
                    let result = parsed.and_then(|rows| {
                        let mut n = 0;
                        for r in rows {
                            let secret = vault::encrypt(&r.password)?;
                            app.storage.upsert_login(&r.origin, &r.username, &secret, &r.note).map_err(|e| e.to_string())?;
                            n += 1;
                        }
                        Ok(n)
                    });
                    mark_dirty(app);
                    match (result, rid) {
                        (Ok(n), Some(rid)) => reply(key, rid, json!({ "imported": n })),
                        (Err(e), Some(rid)) => reply_err(key, rid, e),
                        _ => {}
                    }
                });
            });
            Ok(None)
        }
        Command::PasswordsExport => {
            let rows = csv_rows(app)?;
            let owner = app.window(window).map(|w| w.hwnd).unwrap_or_default();
            let suggested = crate::downloads::default_dir().join("Mizu Passwords.csv").to_string_lossy().into_owned();
            queue::outside(move || {
                let result = match crate::dialogs::save_file(owner, &suggested) {
                    Some(path) => std::fs::write(&path, vault::export_csv(&rows)).map(|_| Some(path)).map_err(|e| e.to_string()),
                    None => Ok(None),
                };
                if let Some(rid) = rid {
                    match result {
                        Ok(path) => reply(key, rid, json!({ "path": path, "count": rows.len() })),
                        Err(e) => reply_err(key, rid, e),
                    }
                }
            });
            Ok(None)
        }
        Command::PasswordFill { id } => fill(app, window, id),
        Command::PasswordPromptResponse { action, username } => {
            let Some(p) = app.pending_password.take() else { return Ok(Some(Value::Null)) };
            match action.as_str() {
                "save" => {
                    let user = username.unwrap_or(p.username);
                    let secret = vault::encrypt(&p.password)?;
                    app.storage.upsert_login(&p.origin, user.trim(), &secret, "").map_err(|e| e.to_string())?;
                    app.toast_all("Password saved");
                }
                "never" => {
                    app.storage.set_never_save(&p.origin, true).map_err(|e| e.to_string())?;
                }
                _ => {}
            }
            if let Some(w) = app.window_mut(p.window) {
                w.flyout.hide();
            }
            mark_dirty(app);
            Ok(Some(Value::Null))
        }
        _ => Err("unsupported".into()),
    }
}

fn mark_dirty(app: &mut App) {
    for w in &mut app.windows {
        w.state_dirty = true;
    }
}
