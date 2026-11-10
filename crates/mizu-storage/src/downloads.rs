use crate::{Result, Storage, now_ms};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    InProgress,
    Paused,
    Completed,
    Cancelled,
    Failed,
    /// Was in progress when the browser exited; the transfer cannot continue.
    Interrupted,
}

impl DownloadState {
    fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "failed" => Self::Failed,
            _ => Self::Interrupted,
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::InProgress | Self::Paused)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DownloadRecord {
    pub id: i64,
    pub url: String,
    pub path: String,
    pub mime: String,
    pub total_bytes: i64,
    pub received_bytes: i64,
    pub state: DownloadState,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

impl Storage {
    pub fn insert_download(&self, url: &str, path: &str, mime: &str, total_bytes: i64) -> Result<i64> {
        self.conn().execute(
            "INSERT INTO downloads (url, path, mime, total_bytes, state, started_at)
             VALUES (?1, ?2, ?3, ?4, 'in_progress', ?5)",
            params![url, path, mime, total_bytes, now_ms()],
        )?;
        Ok(self.conn().last_insert_rowid())
    }

    pub fn update_download(
        &self,
        id: i64,
        state: DownloadState,
        received_bytes: i64,
        total_bytes: i64,
        path: &str,
    ) -> Result<()> {
        let finished = (!state.is_active()).then(now_ms);
        self.conn().execute(
            "UPDATE downloads SET state = ?2, received_bytes = ?3, total_bytes = ?4, path = ?5,
                    finished_at = COALESCE(finished_at, ?6)
             WHERE id = ?1",
            params![id, state.as_str(), received_bytes, total_bytes, path, finished],
        )?;
        Ok(())
    }

    pub fn recent_downloads(&self, limit: usize) -> Result<Vec<DownloadRecord>> {
        let mut stmt = self.conn().prepare_cached(
            "SELECT id, url, path, mime, total_bytes, received_bytes, state, started_at, finished_at
             FROM downloads ORDER BY started_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(DownloadRecord {
                id: r.get(0)?,
                url: r.get(1)?,
                path: r.get(2)?,
                mime: r.get(3)?,
                total_bytes: r.get(4)?,
                received_bytes: r.get(5)?,
                state: DownloadState::parse(&r.get::<_, String>(6)?),
                started_at: r.get(7)?,
                finished_at: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Called at startup: transfers that were running when the process died cannot resume.
    pub fn mark_stale_downloads_interrupted(&self) -> Result<usize> {
        Ok(self.conn().execute(
            "UPDATE downloads SET state = 'interrupted', finished_at = ?1
             WHERE state IN ('in_progress', 'paused')",
            [now_ms()],
        )?)
    }

    pub fn delete_download_record(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM downloads WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Removes finished records (the files on disk are untouched).
    pub fn clear_download_history(&self) -> Result<usize> {
        Ok(self.conn().execute(
            "DELETE FROM downloads WHERE state NOT IN ('in_progress', 'paused')",
            [],
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle() {
        let s = Storage::in_memory().unwrap();
        let id = s.insert_download("https://x/f.zip", "C:/d/f.zip", "application/zip", 100).unwrap();
        s.update_download(id, DownloadState::InProgress, 50, 100, "C:/d/f.zip").unwrap();
        let rec = &s.recent_downloads(10).unwrap()[0];
        assert_eq!(rec.received_bytes, 50);
        assert!(rec.finished_at.is_none());

        assert_eq!(s.mark_stale_downloads_interrupted().unwrap(), 1);
        let rec = &s.recent_downloads(10).unwrap()[0];
        assert_eq!(rec.state, DownloadState::Interrupted);

        s.clear_download_history().unwrap();
        assert!(s.recent_downloads(10).unwrap().is_empty());
    }
}
