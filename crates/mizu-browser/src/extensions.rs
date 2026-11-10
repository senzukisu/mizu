//! Browser extensions through the engine's extension API (Chromium MV3 extensions such as
//! uBlock Origin Lite). Extensions are installed into the normal profile only; private windows
//! run without them.
//!
//! Installation accepts an unpacked folder, a `.zip` or a `.crx` (CRX3) file. The files are
//! copied into `<profile>/extensions/` first, so the extension keeps working if the original is
//! deleted.

use crate::app::App;
use crate::commands::{reply, reply_err};
use crate::queue;
use mizu_core::ids::WindowId;
use mizu_engine::{View, ViewKey};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

type CmdResult = Result<Option<Value>, String>;

/// Any live view of the normal profile (the extension API is per profile).
fn profile_view(app: &App) -> Option<View> {
    app.windows
        .iter()
        .filter(|w| !w.private)
        .flat_map(|w| w.tabs.values())
        .find_map(|t| t.view.clone())
}

fn require_view(app: &App) -> Result<View, String> {
    if !app.settings.advanced.extensions {
        return Err("Extensions are disabled in Settings → Advanced.".into());
    }
    profile_view(app).ok_or_else(|| "Open a normal (non-private) window first.".into())
}

pub fn list(app: &mut App, key: ViewKey, rid: Option<u64>) -> CmdResult {
    let view = require_view(app)?;
    let root = app.paths.extensions();
    view.list_extensions(move |exts| {
        let items: Vec<Value> = exts
            .into_iter()
            .map(|e| {
                let manifest = find_installed_manifest(&root, &e.name);
                json!({
                    "id": e.id,
                    "name": e.name,
                    "enabled": e.enabled,
                    "version": manifest.as_ref().and_then(|m| m["version"].as_str().map(str::to_owned)),
                    "has_options": manifest.as_ref().is_some_and(|m| options_page(m).is_some()),
                })
            })
            .collect();
        if let Some(rid) = rid {
            reply(key, rid, Value::Array(items));
        }
    });
    Ok(None)
}

pub fn install(app: &mut App, window: WindowId, key: ViewKey, rid: Option<u64>) -> CmdResult {
    require_view(app)?;
    let owner = app.window(window).map(|w| w.hwnd).unwrap_or_default();
    let dest_root = app.paths.extensions();
    queue::outside(move || {
        let picked = crate::dialogs::pick_file(owner, "Install extension (.zip, .crx or manifest.json)", "Extensions", "*.zip;*.crx;manifest.json");
        let Some(picked) = picked else {
            if let Some(rid) = rid {
                reply(key, rid, Value::Null);
            }
            return;
        };
        let prepared = prepare(Path::new(&picked), &dest_root);
        queue::with_app(move |app| {
            let folder = match prepared {
                Ok(f) => f,
                Err(e) => {
                    if let Some(rid) = rid {
                        reply_err(key, rid, e);
                    }
                    return;
                }
            };
            let Some(view) = profile_view(app) else { return };
            let folder_str = folder.to_string_lossy().into_owned();
            view.add_extension(&folder_str, move |res| match res {
                Ok(info) => {
                    tracing::info!(name = info.name, "extension installed");
                    if let Some(rid) = rid {
                        reply(key, rid, json!({ "id": info.id, "name": info.name }));
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&folder);
                    if let Some(rid) = rid {
                        reply_err(key, rid, format!("The engine rejected the extension: {e}"));
                    }
                }
            });
        });
    });
    Ok(None)
}

pub fn toggle(app: &mut App, key: ViewKey, rid: Option<u64>, id: String, enabled: bool) -> CmdResult {
    let view = require_view(app)?;
    view.enable_extension(&id, enabled, move |ok| {
        if let Some(rid) = rid {
            reply(key, rid, json!({ "ok": ok }));
        }
    });
    Ok(None)
}

pub fn remove(app: &mut App, key: ViewKey, rid: Option<u64>, id: String) -> CmdResult {
    let view = require_view(app)?;
    view.remove_extension(&id, move |ok| {
        if let Some(rid) = rid {
            reply(key, rid, json!({ "ok": ok }));
        }
    });
    Ok(None)
}

/// Opens the extension's options page (or its popup page) in a tab.
pub fn open(app: &mut App, window: WindowId, id: String) -> CmdResult {
    let view = require_view(app)?;
    let root = app.paths.extensions();
    let id_for_url = id.clone();
    view.list_extensions(move |exts| {
        let Some(ext) = exts.into_iter().find(|e| e.id == id) else { return };
        let page = find_installed_manifest(&root, &ext.name)
            .and_then(|m| options_page(&m))
            .unwrap_or_else(|| "options.html".into());
        let url = format!("chrome-extension://{id_for_url}/{}", page.trim_start_matches('/'));
        queue::with_app(move |a| {
            a.open_tab(window, url, crate::tabs::OpenOptions::foreground());
        });
    });
    Ok(Some(Value::Null))
}

fn options_page(manifest: &Value) -> Option<String> {
    manifest["options_ui"]["page"]
        .as_str()
        .or_else(|| manifest["options_page"].as_str())
        .or_else(|| manifest["action"]["default_popup"].as_str())
        .or_else(|| manifest["browser_action"]["default_popup"].as_str())
        .map(str::to_owned)
}

/// Finds the copied manifest of an installed extension by its display name.
fn find_installed_manifest(root: &Path, name: &str) -> Option<Value> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let m = read_manifest(&entry.path().join("manifest.json"));
        if let Some(m) = m {
            if m["name"].as_str() == Some(name) || resolve_name(&entry.path(), &m).as_deref() == Some(name) {
                return Some(m);
            }
        }
    }
    None
}

fn read_manifest(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// Resolves `__MSG_name__` names through `_locales/<default_locale>/messages.json`.
fn resolve_name(dir: &Path, manifest: &Value) -> Option<String> {
    let raw = manifest["name"].as_str()?;
    let key = raw.strip_prefix("__MSG_")?.strip_suffix("__")?;
    let locale = manifest["default_locale"].as_str().unwrap_or("en");
    let msgs = read_manifest(&dir.join("_locales").join(locale).join("messages.json"))?;
    msgs.as_object()?
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .and_then(|(_, v)| v["message"].as_str().map(str::to_owned))
}

/// Copies or unpacks the chosen extension into the profile and returns the folder.
fn prepare(picked: &Path, dest_root: &Path) -> Result<PathBuf, String> {
    let name = picked.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let stamp = format!("{:x}", mizu_storage::now_ms());
    let dest = dest_root.join(format!("ext-{stamp}"));
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let result = if name == "manifest.json" {
        let src = picked.parent().ok_or("invalid path")?;
        copy_dir(src, &dest, 0)
    } else {
        let bytes = std::fs::read(picked).map_err(|e| e.to_string())?;
        let zip = if name.ends_with(".crx") { strip_crx_header(&bytes)? } else { &bytes[..] };
        unzip(zip, &dest)
    };
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(e);
    }
    // Some archives contain a single top-level folder.
    let root = if dest.join("manifest.json").exists() {
        dest.clone()
    } else {
        let mut dirs = std::fs::read_dir(&dest).map_err(|e| e.to_string())?.flatten().filter(|e| e.path().is_dir());
        match (dirs.next(), dirs.next()) {
            (Some(only), None) if only.path().join("manifest.json").exists() => only.path(),
            _ => {
                let _ = std::fs::remove_dir_all(&dest);
                return Err("No manifest.json found in the extension.".into());
            }
        }
    };
    let manifest = read_manifest(&root.join("manifest.json")).ok_or("manifest.json is not valid JSON")?;
    if manifest["manifest_version"].as_i64() == Some(2) {
        tracing::warn!("installing a Manifest V2 extension; the engine may refuse it");
    }
    Ok(root)
}

/// CRX3: "Cr24" | version u32 | header_size u32 | header | zip.
fn strip_crx_header(bytes: &[u8]) -> Result<&[u8], String> {
    if bytes.len() < 12 || &bytes[..4] != b"Cr24" {
        return Err("Not a CRX file".into());
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| "bad header")?);
    let header_len = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| "bad header")?) as usize;
    let start = match version {
        3 => 12 + header_len,
        2 => {
            // CRX2: key_len at 8, sig_len at 12.
            let sig = u32::from_le_bytes(bytes.get(12..16).ok_or("bad header")?.try_into().map_err(|_| "bad header")?) as usize;
            16 + header_len + sig
        }
        _ => return Err(format!("Unsupported CRX version {version}")),
    };
    bytes.get(start..).ok_or_else(|| "Truncated CRX file".into())
}

fn unzip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    const LIMIT: u64 = 300 * 1024 * 1024;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Invalid archive: {e}"))?;
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        // `enclosed_name` rejects absolute paths and `..` (zip-slip).
        let Some(rel) = file.enclosed_name() else { continue };
        let out = dest.join(rel);
        if file.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut buf = Vec::new();
        (&mut file).take(LIMIT - total).read_to_end(&mut buf).map_err(|e| e.to_string())?;
        total += buf.len() as u64;
        if total >= LIMIT {
            return Err("Extension archive is too large".into());
        }
        std::fs::write(&out, buf).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn copy_dir(src: &Path, dest: &Path, depth: usize) -> Result<(), String> {
    if depth > 16 {
        return Err("Extension folder is nested too deeply".into());
    }
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dest.join(entry.file_name());
        if ty.is_symlink() {
            continue;
        } else if ty.is_dir() {
            copy_dir(&entry.path(), &to, depth + 1)?;
        } else {
            std::fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crx_header() {
        let mut crx = b"Cr24".to_vec();
        crx.extend(3u32.to_le_bytes());
        crx.extend(2u32.to_le_bytes());
        crx.extend([0xAA, 0xBB]);
        crx.extend(b"PK");
        assert_eq!(strip_crx_header(&crx).unwrap(), b"PK");
        assert!(strip_crx_header(b"PK\x03\x04").is_err());
    }

    #[test]
    fn options_page_lookup() {
        let m = json!({"options_ui": {"page": "dashboard.html"}});
        assert_eq!(options_page(&m).as_deref(), Some("dashboard.html"));
        let m = json!({"action": {"default_popup": "popup.html"}});
        assert_eq!(options_page(&m).as_deref(), Some("popup.html"));
    }
}
