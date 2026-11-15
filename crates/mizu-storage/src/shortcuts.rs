//! New tab shortcuts ("speed dial"): their own list, separate from bookmarks.

use crate::{Result, Storage, StorageError, now_ms};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shortcut {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub position: i64,
}

impl Storage {
    pub fn shortcuts(&self) -> Result<Vec<Shortcut>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, title, url, position FROM shortcuts ORDER BY position, id")?;
        let rows = stmt.query_map([], |r| Ok(Shortcut { id: r.get(0)?, title: r.get(1)?, url: r.get(2)?, position: r.get(3)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn add_shortcut(&self, title: &str, url: &str) -> Result<i64> {
        let conn = self.conn();
        let position: i64 = conn.query_row("SELECT COALESCE(MAX(position) + 1, 0) FROM shortcuts", [], |r| r.get(0))?;
        conn.execute(
            "INSERT INTO shortcuts (title, url, position, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![title, url, position, now_ms()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_shortcut(&self, id: i64, title: &str, url: &str) -> Result<()> {
        let n = self.conn().execute("UPDATE shortcuts SET title = ?2, url = ?3 WHERE id = ?1", params![id, title, url])?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    pub fn delete_shortcut(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM shortcuts WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{BOOKMARKS_BAR_ID, Storage};

    #[test]
    fn shortcuts_are_separate_from_bookmarks() {
        let s = Storage::in_memory().unwrap();
        let id = s.add_shortcut("Twitch", "https://twitch.tv/").unwrap();
        s.add_shortcut("Docs", "https://docs.rs/").unwrap();
        assert_eq!(s.shortcuts().unwrap().len(), 2);
        assert!(s.bookmark_children(BOOKMARKS_BAR_ID).unwrap().is_empty());
        s.update_shortcut(id, "Twitch TV", "https://www.twitch.tv/").unwrap();
        assert_eq!(s.shortcuts().unwrap()[0].title, "Twitch TV");
        s.delete_shortcut(id).unwrap();
        assert_eq!(s.shortcuts().unwrap().len(), 1);
    }
}
