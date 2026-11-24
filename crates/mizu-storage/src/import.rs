//! Import from other browsers on this PC: bookmarks, history and search engines.
//!
//! Chromium-based browsers (Chrome, Edge, Yandex, Brave, Vivaldi, Opera) keep bookmarks in a
//! JSON file and history / search engines in SQLite; Firefox keeps both in `places.sqlite`.
//! Databases are copied to a temporary folder first, so a running browser's lock does not
//! matter and its files are never modified. Passwords are not read: browsers encrypt them for
//! themselves, and Mizu imports them only from a CSV the browser exports.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;

use crate::{BOOKMARKS_BAR_ID, OTHER_BOOKMARKS_ID, Result, Storage, now_ms};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Chromium,
    Firefox,
}

/// A browser profile found on this PC.
#[derive(Debug, Clone, Serialize)]
pub struct ImportSource {
    /// Stable id for the UI: browser + profile folder.
    pub id: String,
    pub browser: String,
    pub profile: String,
    pub kind: SourceKind,
    #[serde(skip)]
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Url { title: String, url: String },
    Folder { title: String, children: Vec<Node> },
}

#[derive(Debug, Default)]
pub struct ImportData {
    /// What sat on the source's bookmarks bar.
    pub bar: Vec<Node>,
    /// Everything else (other bookmarks, menu, mobile…).
    pub other: Vec<Node>,
    /// (url, title, visit count, last visit in ms since the Unix epoch)
    pub history: Vec<(String, String, i64, i64)>,
    /// (name, keyword, template with {searchTerms})
    pub engines: Vec<(String, String, String)>,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct ImportCounts {
    pub bookmarks: usize,
    pub history: usize,
    pub engines: usize,
}

/// Most history rows taken from one profile (the most recent ones).
const HISTORY_LIMIT: i64 = 50_000;

fn count(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|n| match n {
            Node::Url { .. } => 1,
            Node::Folder { children, .. } => count(children),
        })
        .sum()
}

impl ImportData {
    pub fn counts(&self) -> ImportCounts {
        ImportCounts { bookmarks: count(&self.bar) + count(&self.other), history: self.history.len(), engines: self.engines.len() }
    }
}

// ───────────────────────────── Finding browsers ─────────────────────────────

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

/// Every browser profile with something to import, Chromium first, then Firefox.
pub fn detect() -> Vec<ImportSource> {
    let mut out = Vec::new();
    let local = env_dir("LOCALAPPDATA");
    let roaming = env_dir("APPDATA");
    let chromium: [(&str, Option<PathBuf>, bool); 8] = [
        ("Google Chrome", local.as_ref().map(|d| d.join(r"Google\Chrome\User Data")), false),
        ("Microsoft Edge", local.as_ref().map(|d| d.join(r"Microsoft\Edge\User Data")), false),
        ("Yandex Browser", local.as_ref().map(|d| d.join(r"Yandex\YandexBrowser\User Data")), false),
        ("Brave", local.as_ref().map(|d| d.join(r"BraveSoftware\Brave-Browser\User Data")), false),
        ("Vivaldi", local.as_ref().map(|d| d.join(r"Vivaldi\User Data")), false),
        ("Chromium", local.as_ref().map(|d| d.join(r"Chromium\User Data")), false),
        // Opera keeps a single profile directly in its folder.
        ("Opera", roaming.as_ref().map(|d| d.join(r"Opera Software\Opera Stable")), true),
        ("Opera GX", roaming.as_ref().map(|d| d.join(r"Opera Software\Opera GX Stable")), true),
    ];
    for (browser, dir, single) in chromium {
        let Some(dir) = dir.filter(|d| d.is_dir()) else { continue };
        if single {
            if has_chromium_data(&dir) {
                out.push(source(browser, "Default", SourceKind::Chromium, dir));
            }
            continue;
        }
        let names = chromium_profile_names(&dir);
        let mut profiles: Vec<PathBuf> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let n = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                (n == "Default" || n.starts_with("Profile ")) && has_chromium_data(p)
            })
            .collect();
        profiles.sort();
        for p in profiles {
            let folder = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_owned();
            let label = names.iter().find(|(f, _)| *f == folder).map_or(folder.clone(), |(_, n)| n.clone());
            out.push(source(browser, &label, SourceKind::Chromium, p));
        }
    }
    if let Some(ff) = roaming.map(|d| d.join(r"Mozilla\Firefox\Profiles")).filter(|d| d.is_dir()) {
        let mut profiles: Vec<PathBuf> = std::fs::read_dir(&ff)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("places.sqlite").is_file())
            .collect();
        profiles.sort();
        for p in profiles {
            // Folder names look like "x1y2z3.default-release": show the part after the dot.
            let folder = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let label = folder.split_once('.').map_or(folder, |(_, n)| n).to_owned();
            out.push(source("Firefox", &label, SourceKind::Firefox, p));
        }
    }
    out
}

fn source(browser: &str, profile: &str, kind: SourceKind, dir: PathBuf) -> ImportSource {
    let folder = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    ImportSource { id: format!("{browser}|{folder}"), browser: browser.into(), profile: profile.into(), kind, dir }
}

fn has_chromium_data(p: &Path) -> bool {
    p.join("Bookmarks").is_file() || p.join("History").is_file()
}

/// Profile display names from `Local State` (folder → name).
fn chromium_profile_names(user_data: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(user_data.join("Local State")) else { return Vec::new() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return Vec::new() };
    v["profile"]["info_cache"]
        .as_object()
        .map(|m| m.iter().filter_map(|(k, p)| Some((k.clone(), p["name"].as_str()?.to_owned()))).collect())
        .unwrap_or_default()
}

// ───────────────────────────── Reading ─────────────────────────────

/// Copies a database (and its WAL, if any) to a fresh temp folder and opens the copy.
fn open_copy(file: &Path) -> Option<(Connection, PathBuf)> {
    let tmp = std::env::temp_dir().join(format!("mizu-import-{}-{}", std::process::id(), now_ms()));
    std::fs::create_dir_all(&tmp).ok()?;
    let name = file.file_name()?;
    let dest = tmp.join(name);
    std::fs::copy(file, &dest).ok()?;
    for ext in ["-wal", "-journal"] {
        let side = PathBuf::from(format!("{}{ext}", file.display()));
        if side.is_file() {
            let _ = std::fs::copy(&side, tmp.join(format!("{}{ext}", name.to_string_lossy())));
        }
    }
    let conn = Connection::open_with_flags(&dest, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()?;
    Some((conn, tmp))
}

fn web_url(u: &str) -> bool {
    u.starts_with("http://") || u.starts_with("https://")
}

pub fn read(src: &ImportSource) -> ImportData {
    match src.kind {
        SourceKind::Chromium => read_chromium(&src.dir),
        SourceKind::Firefox => read_firefox(&src.dir),
    }
}

fn chromium_nodes(v: &serde_json::Value) -> Vec<Node> {
    v["children"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|c| match c["type"].as_str()? {
                    "url" => {
                        let url = c["url"].as_str()?.to_owned();
                        web_url(&url).then(|| Node::Url { title: c["name"].as_str().unwrap_or_default().to_owned(), url })
                    }
                    "folder" => Some(Node::Folder { title: c["name"].as_str().unwrap_or_default().to_owned(), children: chromium_nodes(c) }),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_chromium(dir: &Path) -> ImportData {
    let mut data = ImportData::default();
    if let Ok(text) = std::fs::read_to_string(dir.join("Bookmarks")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            data.bar = chromium_nodes(&v["roots"]["bookmark_bar"]);
            data.other = chromium_nodes(&v["roots"]["other"]);
            let synced = chromium_nodes(&v["roots"]["synced"]);
            if !synced.is_empty() {
                data.other.push(Node::Folder { title: "Mobile bookmarks".into(), children: synced });
            }
        }
    }
    if let Some((conn, tmp)) = open_copy(&dir.join("History")) {
        // Chromium time: microseconds since 1601-01-01.
        let rows = conn
            .prepare("SELECT url, title, visit_count, last_visit_time FROM urls WHERE hidden = 0 ORDER BY last_visit_time DESC LIMIT ?1")
            .and_then(|mut st| {
                st.query_map([HISTORY_LIMIT], |r| {
                    let t: i64 = r.get(3)?;
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1).unwrap_or_default(), r.get::<_, i64>(2)?, t / 1000 - 11_644_473_600_000))
                })
                .map(|it| it.flatten().collect::<Vec<_>>())
            })
            .unwrap_or_default();
        data.history = rows.into_iter().filter(|(u, _, _, t)| web_url(u) && *t > 0).collect();
        drop(conn);
        let _ = std::fs::remove_dir_all(tmp);
    }
    if let Some((conn, tmp)) = open_copy(&dir.join("Web Data")) {
        data.engines = conn
            .prepare("SELECT short_name, keyword, url FROM keywords WHERE url LIKE '%{searchTerms}%' AND url LIKE 'http%' ORDER BY usage_count DESC LIMIT 30")
            .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map(|it| it.flatten().collect()))
            .unwrap_or_default();
        drop(conn);
        let _ = std::fs::remove_dir_all(tmp);
    }
    data
}

/// A row of Firefox's bookmarks: (id, type, parent, title, url, guid).
type FfRow = (i64, i64, i64, String, Option<String>, String);

fn read_firefox(dir: &Path) -> ImportData {
    let mut data = ImportData::default();
    let Some((conn, tmp)) = open_copy(&dir.join("places.sqlite")) else { return data };
    // Bookmarks: (id, type, parent, title, url); type 1 = bookmark, 2 = folder.
    let rows: Vec<FfRow> = conn
        .prepare(
            "SELECT b.id, b.type, b.parent, COALESCE(b.title, ''), p.url, b.guid FROM moz_bookmarks b
             LEFT JOIN moz_places p ON p.id = b.fk ORDER BY b.parent, b.position",
        )
        .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))).map(|it| it.flatten().collect()))
        .unwrap_or_default();
    fn build(parent: i64, rows: &[FfRow], depth: usize) -> Vec<Node> {
        if depth > 32 {
            return Vec::new();
        }
        rows.iter()
            .filter(|r| r.2 == parent)
            .filter_map(|(id, ty, _, title, url, _)| match ty {
                1 => url.clone().filter(|u| web_url(u)).map(|url| Node::Url { title: title.clone(), url }),
                2 => Some(Node::Folder { title: title.clone(), children: build(*id, rows, depth + 1) }),
                _ => None,
            })
            .collect()
    }
    let root = |guid: &str| rows.iter().find(|r| r.5 == guid).map(|r| r.0);
    if let Some(id) = root("toolbar_____") {
        data.bar = build(id, &rows, 0);
    }
    for (guid, title) in [("menu________", "Bookmarks menu"), ("unfiled_____", ""), ("mobile______", "Mobile bookmarks")] {
        if let Some(id) = root(guid) {
            let children = build(id, &rows, 0);
            if children.is_empty() {
                continue;
            }
            if title.is_empty() {
                data.other.extend(children);
            } else {
                data.other.push(Node::Folder { title: title.into(), children });
            }
        }
    }
    // History: last_visit_date is microseconds since the Unix epoch.
    data.history = conn
        .prepare("SELECT url, COALESCE(title, ''), visit_count, last_visit_date FROM moz_places WHERE last_visit_date IS NOT NULL AND hidden = 0 ORDER BY last_visit_date DESC LIMIT ?1")
        .and_then(|mut st| {
            st.query_map([HISTORY_LIMIT], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)? / 1000)))
                .map(|it| it.flatten().filter(|(u, _, _, _)| web_url(u)).collect())
        })
        .unwrap_or_default();
    drop(conn);
    let _ = std::fs::remove_dir_all(tmp);
    data
}

// ───────────────────────────── Writing into Mizu ─────────────────────────────

impl Storage {
    /// Adds bookmarks into a new "Imported from …" folder in Other bookmarks; the source's
    /// bar becomes a subfolder named `bar_name`. Only with `to_bar` do the bar's items go onto
    /// Mizu's own bar (skipping addresses already there): Mizu's bar is never filled unasked.
    /// Returns how many bookmarks were added.
    pub fn import_bookmarks(&mut self, browser: &str, data: &ImportData, to_bar: bool, bar_name: &str) -> Result<usize> {
        let mut added = 0;
        if data.bar.is_empty() && data.other.is_empty() {
            return Ok(0);
        }
        if to_bar {
            let on_bar: Vec<String> = self.bookmark_children(BOOKMARKS_BAR_ID)?.into_iter().filter_map(|b| b.url).collect();
            for n in &data.bar {
                match n {
                    Node::Url { url, .. } if on_bar.contains(url) => {}
                    _ => added += self.add_nodes(BOOKMARKS_BAR_ID, std::slice::from_ref(n))?,
                }
            }
        }
        let folder = self.add_bookmark_folder(OTHER_BOOKMARKS_ID, &format!("Imported from {browser}"))?;
        if !to_bar && !data.bar.is_empty() {
            let bar = self.add_bookmark_folder(folder, if bar_name.trim().is_empty() { "Bookmarks bar" } else { bar_name })?;
            added += self.add_nodes(bar, &data.bar)?;
        }
        added += self.add_nodes(folder, &data.other)?;
        Ok(added)
    }

    fn add_nodes(&mut self, parent: i64, nodes: &[Node]) -> Result<usize> {
        let mut added = 0;
        for n in nodes {
            match n {
                Node::Url { title, url } => {
                    let title = if title.trim().is_empty() { url.as_str() } else { title.as_str() };
                    self.add_bookmark(parent, title, url)?;
                    added += 1;
                }
                Node::Folder { title, children } => {
                    let f = self.add_bookmark_folder(parent, if title.trim().is_empty() { "Folder" } else { title })?;
                    added += self.add_nodes(f, children)?;
                }
            }
        }
        Ok(added)
    }

    /// Merges history rows: new addresses are added, known ones keep the higher visit count
    /// and the later visit. Each address gets one visit at its last visit time.
    pub fn import_history(&mut self, rows: &[(String, String, i64, i64)]) -> Result<usize> {
        let tx = self.conn_mut().transaction()?;
        let mut added = 0;
        {
            let mut up = tx.prepare(
                "INSERT INTO history_urls (url, title, visit_count, last_visit) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(url) DO UPDATE SET
                    visit_count = MAX(visit_count, excluded.visit_count),
                    last_visit = MAX(last_visit, excluded.last_visit),
                    title = CASE WHEN title = '' THEN excluded.title ELSE title END",
            )?;
            let mut visit = tx.prepare(
                "INSERT INTO history_visits (url_id, visited_at)
                 SELECT id, ?2 FROM history_urls WHERE url = ?1
                 AND NOT EXISTS (SELECT 1 FROM history_visits v WHERE v.url_id = history_urls.id AND v.visited_at = ?2)",
            )?;
            for (url, title, visits, last) in rows {
                if url.len() > 4096 {
                    continue;
                }
                up.execute(params![url, title, (*visits).max(1), last])?;
                visit.execute(params![url, last])?;
                added += 1;
            }
        }
        tx.commit()?;
        Ok(added)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chromium_bookmarks_json() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"children":[{"type":"url","name":"Docs","url":"https://docs.rs/"},{"type":"url","name":"JS","url":"javascript:alert(1)"},
            {"type":"folder","name":"Dev","children":[{"type":"url","name":"GH","url":"https://github.com/"}]}]}"#,
        )
        .unwrap();
        let nodes = chromium_nodes(&v);
        assert_eq!(nodes.len(), 2, "javascript: bookmarks are skipped");
        assert_eq!(count(&nodes), 2);
    }

    #[test]
    fn imports_into_bar_and_folder_without_duplicates() {
        let mut s = Storage::in_memory().unwrap();
        s.add_bookmark(BOOKMARKS_BAR_ID, "Docs", "https://docs.rs/").unwrap();
        let data = ImportData {
            bar: vec![Node::Url { title: "Docs".into(), url: "https://docs.rs/".into() }, Node::Url { title: "GH".into(), url: "https://github.com/".into() }],
            other: vec![Node::Folder { title: "Work".into(), children: vec![Node::Url { title: "Mail".into(), url: "https://mail.example.com/".into() }] }],
            ..Default::default()
        };
        // By default Mizu's bar is left alone: everything goes into the folder.
        assert_eq!(s.import_bookmarks("Chrome", &data, false, "Bookmarks bar").unwrap(), 3);
        assert_eq!(s.bookmark_children(BOOKMARKS_BAR_ID).unwrap().len(), 1);
        let folder = s.bookmark_children(OTHER_BOOKMARKS_ID).unwrap();
        assert_eq!(folder[0].title, "Imported from Chrome");
        // Asked to, the bar items join Mizu's bar without duplicates.
        assert_eq!(s.import_bookmarks("Chrome", &data, true, "").unwrap(), 2);
        assert_eq!(s.bookmark_children(BOOKMARKS_BAR_ID).unwrap().len(), 2);
    }

    /// Lists what would be imported on this PC (read-only): cargo test -p mizu-storage detect_on_this_pc -- --ignored --nocapture
    #[test]
    #[ignore]
    fn detect_on_this_pc() {
        for src in detect() {
            let t = std::time::Instant::now();
            let d = read(&src);
            println!("{} / {} ({:?}): {:?} in {} ms", src.browser, src.profile, src.kind, d.counts(), t.elapsed().as_millis());
        }
    }

    #[test]
    fn history_merges() {
        let mut s = Storage::in_memory().unwrap();
        let rows = vec![("https://a.example/".to_owned(), "A".to_owned(), 3, 1_700_000_000_000), ("https://a.example/".to_owned(), "".to_owned(), 5, 1_600_000_000_000)];
        assert_eq!(s.import_history(&rows).unwrap(), 2);
        let h = s.suggest_history("a.example", 5).unwrap();
        assert_eq!(h[0].visit_count, 5);
        assert_eq!(h[0].last_visit, 1_700_000_000_000);
    }
}
