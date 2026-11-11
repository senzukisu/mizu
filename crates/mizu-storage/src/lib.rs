//! Profile database.
//!
//! One SQLite file per profile holds everything structured: history, bookmarks, download
//! records, site permissions and the last session. SQLite gives us atomic transactions,
//! crash safety (WAL journal) and an online backup API without writing any of it ourselves.
//!
//! Robustness strategy:
//! * schema changes go through numbered migrations tracked in `PRAGMA user_version`;
//! * on open, `PRAGMA quick_check` validates the file; a corrupt database is moved aside
//!   and the last good backup (`mizu.db.bak`) is restored, or a fresh database is created;
//! * after every successful open a fresh backup is written with SQLite's backup API.

mod bookmarks;
mod downloads;
mod history;
mod logins;
mod migrations;
mod permissions;
mod session;

pub use bookmarks::{BOOKMARKS_BAR_ID, Bookmark, BookmarkKind, OTHER_BOOKMARKS_ID};
pub use downloads::{DownloadRecord, DownloadState};
pub use history::{HistoryEntry, TimeRange};
pub use logins::Login;
pub use permissions::{PermissionRecord, PermissionState};

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(String),
    #[error("not found")]
    NotFound,
    #[error("invalid operation: {0}")]
    Invalid(&'static str),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// What happened while opening the database. Surfaced to the user when not `Clean`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    Clean,
    RestoredFromBackup { corrupt_copy: PathBuf },
    RecreatedEmpty { corrupt_copy: Option<PathBuf> },
}

pub struct Storage {
    conn: Connection,
    path: Option<PathBuf>,
}

impl Storage {
    /// Opens (or creates) the database at `path`, recovering from corruption with `backup`.
    pub fn open(path: &Path, backup: &Path) -> Result<(Self, OpenOutcome)> {
        match Self::try_open(path) {
            Ok(storage) => {
                if let Err(e) = storage.write_backup(backup) {
                    tracing::warn!(error = %e, "could not write database backup");
                }
                Ok((storage, OpenOutcome::Clean))
            }
            Err(e) => {
                tracing::error!(error = %e, ?path, "database unusable, attempting recovery");
                let corrupt_copy = move_aside(path)?;
                if backup.exists() {
                    std::fs::copy(backup, path)?;
                    if let Ok(storage) = Self::try_open(path) {
                        let corrupt_copy = corrupt_copy.unwrap_or_default();
                        return Ok((storage, OpenOutcome::RestoredFromBackup { corrupt_copy }));
                    }
                    move_aside(path)?;
                }
                let storage = Self::try_open(path)?;
                Ok((storage, OpenOutcome::RecreatedEmpty { corrupt_copy }))
            }
        }
    }

    /// In-memory database for tests and private sessions.
    pub fn in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        migrations::run(&mut conn)?;
        Ok(Self { conn, path: None })
    }

    fn try_open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        configure(&conn)?;
        let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(StorageError::Invalid("integrity check failed"));
        }
        migrations::run(&mut conn)?;
        Ok(Self {
            conn,
            path: Some(path.to_owned()),
        })
    }

    pub fn write_backup(&self, backup: &Path) -> Result<()> {
        let tmp = backup.with_extension("bak.tmp");
        {
            let mut dst = Connection::open(&tmp)?;
            let b = rusqlite::backup::Backup::new(&self.conn, &mut dst)?;
            b.run_to_completion(256, std::time::Duration::ZERO, None)?;
        }
        std::fs::rename(&tmp, backup)?;
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    pub(crate) fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Reclaims space after large deletions.
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(())
    }
}

fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 2000;",
    )?;
    Ok(())
}

fn move_aside(path: &Path) -> Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let aside = path.with_extension(format!("corrupt-{}", now_ms()));
    std::fs::rename(path, &aside)?;
    for suffix in ["-wal", "-shm"] {
        let side = PathBuf::from(format!("{}{suffix}", path.display()));
        let _ = std::fs::remove_file(side);
    }
    Ok(Some(aside))
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// Escapes `%`, `_` and `\` for use in a `LIKE … ESCAPE '\'` pattern.
pub(crate) fn like_pattern(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_from_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mizu.db");
        let bak = dir.path().join("mizu.db.bak");
        {
            let (mut s, outcome) = Storage::open(&db, &bak).unwrap();
            assert_eq!(outcome, OpenOutcome::Clean);
            s.record_visit("https://example.com/", "Example").unwrap();
            s.write_backup(&bak).unwrap();
        }
        let _ = std::fs::remove_file(dir.path().join("mizu.db-wal"));
        std::fs::write(&db, b"definitely not a sqlite database, just garbage bytes......").unwrap();

        let (s, outcome) = Storage::open(&db, &bak).unwrap();
        assert!(matches!(outcome, OpenOutcome::RestoredFromBackup { .. }));
        assert_eq!(s.search_history("example", 10).unwrap().len(), 1);
    }

    #[test]
    fn recreates_when_no_backup() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mizu.db");
        std::fs::write(&db, b"garbage garbage garbage garbage garbage garbage garbage!!").unwrap();
        let (_s, outcome) = Storage::open(&db, &dir.path().join("none.bak")).unwrap();
        assert!(matches!(outcome, OpenOutcome::RecreatedEmpty { corrupt_copy: Some(_) }));
    }

    #[test]
    fn like_escaping() {
        assert_eq!(like_pattern("50%_a\\"), "%50\\%\\_a\\\\%");
    }
}
