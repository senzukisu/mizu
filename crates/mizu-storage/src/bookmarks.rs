use crate::{Result, Storage, StorageError, like_pattern, now_ms};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

pub const BOOKMARKS_BAR_ID: i64 = 1;
pub const OTHER_BOOKMARKS_ID: i64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BookmarkKind {
    Folder,
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bookmark {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub kind: BookmarkKind,
    pub title: String,
    pub url: Option<String>,
    pub position: i64,
}

const COLUMNS: &str = "id, parent_id, kind, title, url, position";

fn row_to_bookmark(r: &rusqlite::Row<'_>) -> rusqlite::Result<Bookmark> {
    let kind: String = r.get(2)?;
    Ok(Bookmark {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        kind: if kind == "folder" {
            BookmarkKind::Folder
        } else {
            BookmarkKind::Url
        },
        title: r.get(3)?,
        url: r.get(4)?,
        position: r.get(5)?,
    })
}

fn is_root(id: i64) -> bool {
    id == BOOKMARKS_BAR_ID || id == OTHER_BOOKMARKS_ID
}

impl Storage {
    pub fn add_bookmark(&self, parent_id: i64, title: &str, url: &str) -> Result<i64> {
        self.insert_bookmark(parent_id, "url", title, Some(url))
    }

    pub fn add_bookmark_folder(&self, parent_id: i64, title: &str) -> Result<i64> {
        self.insert_bookmark(parent_id, "folder", title, None)
    }

    fn insert_bookmark(&self, parent_id: i64, kind: &str, title: &str, url: Option<&str>) -> Result<i64> {
        self.ensure_folder(parent_id)?;
        let position: i64 = self.conn().query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM bookmarks WHERE parent_id = ?1",
            [parent_id],
            |r| r.get(0),
        )?;
        self.conn().execute(
            "INSERT INTO bookmarks (parent_id, kind, title, url, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![parent_id, kind, title, url, position, now_ms()],
        )?;
        Ok(self.conn().last_insert_rowid())
    }

    fn ensure_folder(&self, id: i64) -> Result<()> {
        let kind: Option<String> = self
            .conn()
            .query_row("SELECT kind FROM bookmarks WHERE id = ?1", [id], |r| r.get(0))
            .optional()?;
        match kind.as_deref() {
            Some("folder") => Ok(()),
            Some(_) => Err(StorageError::Invalid("parent is not a folder")),
            None => Err(StorageError::NotFound),
        }
    }

    pub fn bookmark(&self, id: i64) -> Result<Bookmark> {
        self.conn()
            .query_row(
                &format!("SELECT {COLUMNS} FROM bookmarks WHERE id = ?1"),
                [id],
                row_to_bookmark,
            )
            .optional()?
            .ok_or(StorageError::NotFound)
    }

    pub fn bookmark_children(&self, parent_id: i64) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM bookmarks WHERE parent_id = ?1 ORDER BY position, id"
        ))?;
        let rows = stmt.query_map([parent_id], row_to_bookmark)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every bookmark and folder, for the bookmarks manager tree.
    pub fn all_bookmarks(&self) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM bookmarks ORDER BY parent_id, position, id"
        ))?;
        let rows = stmt.query_map([], row_to_bookmark)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn find_bookmark_by_url(&self, url: &str) -> Result<Option<Bookmark>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLUMNS} FROM bookmarks WHERE url = ?1 LIMIT 1"),
                [url],
                row_to_bookmark,
            )
            .optional()?)
    }

    pub fn search_bookmarks(&self, query: &str, limit: usize) -> Result<Vec<Bookmark>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM bookmarks
             WHERE kind = 'url' AND (title LIKE ?1 ESCAPE '\\' OR url LIKE ?1 ESCAPE '\\')
             ORDER BY title LIMIT ?2"
        ))?;
        let rows = stmt.query_map(params![like_pattern(query), limit as i64], row_to_bookmark)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_bookmark(&self, id: i64, title: &str, url: Option<&str>) -> Result<()> {
        let existing = self.bookmark(id)?;
        if is_root(id) {
            return Err(StorageError::Invalid("root folders cannot be edited"));
        }
        let url = match existing.kind {
            BookmarkKind::Url => url.or(existing.url.as_deref()),
            BookmarkKind::Folder => None,
        };
        self.conn().execute(
            "UPDATE bookmarks SET title = ?2, url = ?3 WHERE id = ?1",
            params![id, title, url],
        )?;
        Ok(())
    }

    /// Moves a bookmark into `parent_id` at `position` (clamped), renumbering siblings.
    pub fn move_bookmark(&mut self, id: i64, parent_id: i64, position: usize) -> Result<()> {
        if is_root(id) {
            return Err(StorageError::Invalid("root folders cannot be moved"));
        }
        self.ensure_folder(parent_id)?;
        // Refuse to move a folder into itself or a descendant.
        let mut cursor = Some(parent_id);
        while let Some(c) = cursor {
            if c == id {
                return Err(StorageError::Invalid("cannot move a folder into itself"));
            }
            cursor = self.bookmark(c)?.parent_id;
        }
        let mut siblings: Vec<i64> = self
            .bookmark_children(parent_id)?
            .into_iter()
            .map(|b| b.id)
            .filter(|&b| b != id)
            .collect();
        siblings.insert(position.min(siblings.len()), id);
        let tx = self.conn_mut().transaction()?;
        tx.execute("UPDATE bookmarks SET parent_id = ?2 WHERE id = ?1", params![id, parent_id])?;
        for (pos, sid) in siblings.iter().enumerate() {
            tx.execute("UPDATE bookmarks SET position = ?2 WHERE id = ?1", params![sid, pos as i64])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Deletes a bookmark or a folder with everything inside it.
    pub fn delete_bookmark(&self, id: i64) -> Result<()> {
        if is_root(id) {
            return Err(StorageError::Invalid("root folders cannot be deleted"));
        }
        self.conn().execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crud_and_folders() {
        let mut s = Storage::in_memory().unwrap();
        let folder = s.add_bookmark_folder(BOOKMARKS_BAR_ID, "Dev").unwrap();
        let a = s.add_bookmark(folder, "Rust", "https://rust-lang.org/").unwrap();
        let b = s.add_bookmark(BOOKMARKS_BAR_ID, "Docs", "https://docs.rs/").unwrap();

        assert_eq!(s.bookmark_children(BOOKMARKS_BAR_ID).unwrap().len(), 2);
        s.update_bookmark(a, "Rust lang", None).unwrap();
        assert_eq!(s.bookmark(a).unwrap().title, "Rust lang");
        assert_eq!(s.bookmark(a).unwrap().url.as_deref(), Some("https://rust-lang.org/"));

        assert_eq!(s.search_bookmarks("docs", 10).unwrap().len(), 1);
        assert!(s.find_bookmark_by_url("https://docs.rs/").unwrap().is_some());

        s.move_bookmark(b, folder, 0).unwrap();
        let kids = s.bookmark_children(folder).unwrap();
        assert_eq!(kids[0].id, b);
        assert_eq!(kids[1].id, a);

        assert!(s.move_bookmark(folder, folder, 0).is_err());
        assert!(s.delete_bookmark(BOOKMARKS_BAR_ID).is_err());
        assert!(s.add_bookmark(a, "x", "https://x/").is_err());

        s.delete_bookmark(folder).unwrap();
        assert!(s.bookmark(a).is_err());
    }
}
