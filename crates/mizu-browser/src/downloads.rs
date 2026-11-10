//! Download manager: tracks live engine downloads, computes speed/ETA, persists records.

use mizu_core::ids::DownloadId;
use mizu_engine::{Download, DownloadInfo, DownloadStatus};
use mizu_storage::{DownloadState, Storage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct DownloadItem {
    pub id: DownloadId,
    pub db_id: Option<i64>,
    pub op: Option<Download>,
    pub url: String,
    pub path: String,
    pub mime: String,
    pub received: i64,
    pub total: i64,
    pub state: DownloadState,
    pub can_resume: bool,
    pub private: bool,
    /// Bytes per second (exponentially smoothed).
    pub speed: f64,
    sample: (Instant, i64),
}

#[derive(Default)]
pub struct Downloads {
    pub items: Vec<DownloadItem>,
    pub dirty: bool,
}

fn map_state(s: DownloadStatus) -> DownloadState {
    match s {
        DownloadStatus::InProgress => DownloadState::InProgress,
        DownloadStatus::Paused => DownloadState::Paused,
        DownloadStatus::Completed => DownloadState::Completed,
        DownloadStatus::Cancelled => DownloadState::Cancelled,
        DownloadStatus::Failed => DownloadState::Failed,
    }
}

fn state_str(s: DownloadState) -> &'static str {
    match s {
        DownloadState::InProgress => "in_progress",
        DownloadState::Paused => "paused",
        DownloadState::Completed => "completed",
        DownloadState::Cancelled => "cancelled",
        DownloadState::Failed => "failed",
        DownloadState::Interrupted => "interrupted",
    }
}

/// The user's Downloads folder.
pub fn default_dir() -> PathBuf {
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    // SAFETY: SHGetKnownFolderPath returns a CoTaskMem string that we free.
    unsafe {
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None) {
            let s = p.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
            if !s.is_empty() {
                return PathBuf::from(s);
            }
        }
    }
    std::env::var_os("USERPROFILE").map(|h| Path::new(&h).join("Downloads")).unwrap_or_else(std::env::temp_dir)
}

/// Picks a non-existing path in `dir` for `file_name` ("name (1).ext", "name (2).ext", …).
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let clean: String = file_name
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let clean = if clean.trim().is_empty() { "download".to_owned() } else { clean };
    let candidate = dir.join(&clean);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match clean.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_owned(), format!(".{e}")),
        _ => (clean.clone(), String::new()),
    };
    (1..10_000)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(candidate)
}

impl Downloads {
    pub fn load(storage: &Storage) -> Self {
        let items = storage
            .recent_downloads(200)
            .unwrap_or_default()
            .into_iter()
            .map(|r| DownloadItem {
                id: DownloadId::next(),
                db_id: Some(r.id),
                op: None,
                url: r.url,
                path: r.path,
                mime: r.mime,
                received: r.received_bytes,
                total: r.total_bytes,
                state: r.state,
                can_resume: false,
                private: false,
                speed: 0.0,
                sample: (Instant::now(), r.received_bytes),
            })
            .collect();
        Self { items, dirty: true }
    }

    pub fn start(&mut self, op: Download, private: bool, storage: Option<&Storage>) -> DownloadId {
        let info = op.info();
        let id = DownloadId::next();
        let db_id = if private {
            None
        } else {
            storage.and_then(|s| s.insert_download(&info.uri, &info.path, &info.mime, info.total).ok())
        };
        op.subscribe(move |info| crate::queue::push(crate::queue::Queued::DownloadUpdate(id, info)));
        self.items.insert(
            0,
            DownloadItem {
                id,
                db_id,
                op: Some(op),
                url: info.uri,
                path: info.path,
                mime: info.mime,
                received: info.received,
                total: info.total,
                state: DownloadState::InProgress,
                can_resume: info.can_resume,
                private,
                speed: 0.0,
                sample: (Instant::now(), info.received),
            },
        );
        self.dirty = true;
        id
    }

    pub fn update(&mut self, id: DownloadId, info: DownloadInfo, storage: Option<&Storage>) {
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else { return };
        let now = Instant::now();
        let dt = now.duration_since(item.sample.0).as_secs_f64();
        if dt >= 0.25 {
            let instant = (info.received - item.sample.1).max(0) as f64 / dt;
            item.speed = if item.speed == 0.0 { instant } else { item.speed * 0.7 + instant * 0.3 };
            item.sample = (now, info.received);
        }
        let new_state = map_state(info.status);
        let state_changed = new_state != item.state;
        item.received = info.received;
        item.total = info.total;
        item.path = info.path;
        item.can_resume = info.can_resume;
        item.state = new_state;
        if !new_state.is_active() {
            item.speed = 0.0;
        }
        if state_changed || !new_state.is_active() {
            if let (Some(db), Some(storage)) = (item.db_id, storage) {
                let _ = storage.update_download(db, item.state, item.received, item.total, &item.path);
            }
        }
        if state_changed && new_state != DownloadState::InProgress && new_state != DownloadState::Paused {
            tracing::info!(state = state_str(new_state), "download finished");
        }
        self.dirty = true;
    }

    pub fn get(&self, id: DownloadId) -> Option<&DownloadItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn remove(&mut self, id: DownloadId, storage: Option<&Storage>) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id && !i.state.is_active()) {
            let item = self.items.remove(pos);
            if let (Some(db), Some(s)) = (item.db_id, storage) {
                let _ = s.delete_download_record(db);
            }
            self.dirty = true;
        }
    }

    pub fn clear_finished(&mut self, storage: Option<&Storage>) {
        self.items.retain(|i| i.state.is_active());
        if let Some(s) = storage {
            let _ = s.clear_download_history();
        }
        self.dirty = true;
    }

    /// Drops records from a private session (called when the last private window closes).
    pub fn forget_private(&mut self) {
        self.items.retain(|i| !i.private || i.state.is_active());
        self.dirty = true;
    }

    pub fn to_json(&self, include_private: bool) -> Value {
        Value::Array(
            self.items
                .iter()
                .filter(|i| include_private || !i.private)
                .map(|i| {
                    let file = Path::new(&i.path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                    let eta = if i.speed > 1.0 && i.total > 0 { ((i.total - i.received) as f64 / i.speed).round() } else { -1.0 };
                    json!({
                        "id": i.id.0,
                        "url": i.url,
                        "file": file,
                        "path": i.path,
                        "mime": i.mime,
                        "received": i.received,
                        "total": i.total,
                        "state": state_str(i.state),
                        "speed": i.speed.round(),
                        "eta": eta,
                        "can_resume": i.can_resume,
                        "live": i.op.is_some(),
                        "exists": i.state == DownloadState::Completed && Path::new(&i.path).exists(),
                        "private": i.private,
                    })
                })
                .collect(),
        )
    }

    /// (active downloads, overall progress 0..1 or -1 if unknown)
    pub fn summary(&self) -> (usize, f64) {
        let active: Vec<_> = self.items.iter().filter(|i| i.state == DownloadState::InProgress).collect();
        let total: i64 = active.iter().map(|i| i.total.max(0)).sum();
        let received: i64 = active.iter().map(|i| i.received).sum();
        let progress = if total > 0 { received as f64 / total as f64 } else { -1.0 };
        (active.len(), progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names() {
        let dir = std::env::temp_dir().join(format!("mizu-dl-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p1 = unique_path(&dir, "a.txt");
        std::fs::write(&p1, "x").unwrap();
        let p2 = unique_path(&dir, "a.txt");
        assert_eq!(p2.file_name().unwrap(), "a (1).txt");
        assert_eq!(unique_path(&dir, "bad:name?.zip").file_name().unwrap(), "bad_name_.zip");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
