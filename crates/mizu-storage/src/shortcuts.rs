//! New tab shortcuts ("speed dial"): their own list, separate from bookmarks. Tiles can be
//! grouped into folders (one level: a folder holds links, not other folders).

use crate::{Result, Storage, StorageError, now_ms};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shortcut {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub position: i64,
    /// The folder this tile is in (`None` at the top level).
    pub folder: Option<i64>,
    pub is_folder: bool,
}

impl Storage {
    /// Every shortcut and folder, ordered by position within its level.
    pub fn shortcuts(&self) -> Result<Vec<Shortcut>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, title, url, position, folder, kind FROM shortcuts ORDER BY position, id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Shortcut {
                id: r.get(0)?,
                title: r.get(1)?,
                url: r.get(2)?,
                position: r.get(3)?,
                folder: r.get(4)?,
                is_folder: r.get::<_, String>(5)? == "folder",
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn shortcut(&self, id: i64) -> Result<Shortcut> {
        self.shortcuts()?.into_iter().find(|s| s.id == id).ok_or(StorageError::NotFound)
    }

    /// Ids at one level (top level or inside a folder), in order.
    fn level(&self, folder: Option<i64>) -> Result<Vec<i64>> {
        Ok(self.shortcuts()?.into_iter().filter(|s| s.folder == folder).map(|s| s.id).collect())
    }

    fn renumber(&self, ids: &[i64], folder: Option<i64>) -> Result<()> {
        for (pos, id) in ids.iter().enumerate() {
            self.conn().execute("UPDATE shortcuts SET position = ?2, folder = ?3 WHERE id = ?1", params![id, pos as i64, folder])?;
        }
        Ok(())
    }

    pub fn add_shortcut(&self, title: &str, url: &str) -> Result<i64> {
        self.add_shortcut_in(None, title, url)
    }

    /// Adds a link at the end of the top level or of a folder.
    pub fn add_shortcut_in(&self, folder: Option<i64>, title: &str, url: &str) -> Result<i64> {
        if let Some(f) = folder {
            if !self.shortcut(f)?.is_folder {
                return Err(StorageError::NotFound);
            }
        }
        let conn = self.conn();
        let position: i64 = conn.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM shortcuts WHERE folder IS ?1",
            params![folder],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO shortcuts (title, url, position, created_at, folder) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![title, url, position, now_ms(), folder],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_shortcut(&self, id: i64, title: &str, url: &str) -> Result<()> {
        let n = self.conn().execute(
            "UPDATE shortcuts SET title = ?2, url = CASE WHEN kind = 'folder' THEN url ELSE ?3 END WHERE id = ?1",
            params![id, title, url],
        )?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Moves a shortcut to `index` (0-based, clamped) at the top level, or inside `folder`.
    /// Folders stay at the top level. A folder left empty is removed.
    pub fn move_shortcut_to(&self, id: i64, folder: Option<i64>, index: usize) -> Result<()> {
        let s = self.shortcut(id)?;
        if let Some(f) = folder {
            if s.is_folder || f == id || !self.shortcut(f)?.is_folder {
                return Err(StorageError::NotFound);
            }
        }
        let mut from = self.level(s.folder)?;
        from.retain(|&x| x != id);
        let mut to = if folder == s.folder { from.clone() } else { self.level(folder)? };
        to.insert(index.min(to.len()), id);
        if folder != s.folder {
            self.renumber(&from, s.folder)?;
        }
        self.renumber(&to, folder)?;
        if let Some(old) = s.folder.filter(|o| Some(*o) != folder) {
            if self.level(Some(old))?.is_empty() {
                self.conn().execute("DELETE FROM shortcuts WHERE id = ?1", [old])?;
                let top = self.level(None)?;
                self.renumber(&top, None)?;
            }
        }
        Ok(())
    }

    /// Moves a shortcut within its level (kept for callers that only reorder).
    pub fn move_shortcut(&self, id: i64, index: usize) -> Result<()> {
        let folder = self.shortcut(id)?.folder;
        self.move_shortcut_to(id, folder, index)
    }

    /// Drops link `a` onto link `b` (both at the top level): a folder named `title` takes
    /// `b`'s place with `b` and then `a` inside. Returns the folder id.
    pub fn make_shortcut_folder(&self, a: i64, b: i64, title: &str) -> Result<i64> {
        let (sa, sb) = (self.shortcut(a)?, self.shortcut(b)?);
        if a == b || sa.is_folder || sb.is_folder || sb.folder.is_some() {
            return Err(StorageError::NotFound);
        }
        let conn = self.conn();
        conn.execute(
            "INSERT INTO shortcuts (title, url, position, created_at, kind) VALUES (?1, '', ?2, ?3, 'folder')",
            params![title, sb.position, now_ms()],
        )?;
        let folder = conn.last_insert_rowid();
        let mut top = self.level(None)?;
        top.retain(|&x| x != a && x != b && x != folder);
        let at = self.level(None)?.iter().filter(|&&x| x != a && x != folder).position(|&x| x == b).unwrap_or(top.len());
        top.insert(at.min(top.len()), folder);
        self.renumber(&top, None)?;
        self.renumber(&[b, a], Some(folder))?;
        if let Some(old) = sa.folder {
            let rest = self.level(Some(old))?;
            if rest.is_empty() {
                self.conn().execute("DELETE FROM shortcuts WHERE id = ?1", [old])?;
            } else {
                self.renumber(&rest, Some(old))?;
            }
        }
        Ok(folder)
    }

    /// Deletes a link; deleting a folder keeps its links, which take the folder's place.
    pub fn delete_shortcut(&self, id: i64) -> Result<()> {
        let s = self.shortcut(id)?;
        if s.is_folder {
            let inner = self.level(Some(id))?;
            let mut top = self.level(None)?;
            let at = top.iter().position(|&x| x == id).unwrap_or(top.len());
            top.retain(|&x| x != id);
            for (i, x) in inner.iter().enumerate() {
                top.insert((at + i).min(top.len()), *x);
            }
            self.renumber(&top, None)?;
        }
        self.conn().execute("DELETE FROM shortcuts WHERE id = ?1", [id])?;
        if let Some(f) = s.folder {
            if self.level(Some(f))?.is_empty() {
                self.conn().execute("DELETE FROM shortcuts WHERE id = ?1", [f])?;
            }
        }
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
        let third = s.add_shortcut("Mail", "https://mail.example.com/").unwrap();
        s.move_shortcut(third, 0).unwrap();
        assert_eq!(s.shortcuts().unwrap()[0].id, third);
        s.delete_shortcut(id).unwrap();
        assert_eq!(s.shortcuts().unwrap().len(), 2);
    }

    #[test]
    fn folders_of_shortcuts() {
        let s = Storage::in_memory().unwrap();
        let a = s.add_shortcut("A", "https://a/").unwrap();
        let b = s.add_shortcut("B", "https://b/").unwrap();
        let c = s.add_shortcut("C", "https://c/").unwrap();
        // Dropping C onto A makes a folder where A was.
        let f = s.make_shortcut_folder(c, a, "Folder").unwrap();
        let top: Vec<i64> = s.shortcuts().unwrap().into_iter().filter(|x| x.folder.is_none()).map(|x| x.id).collect();
        assert_eq!(top, vec![f, b]);
        let inner: Vec<i64> = s.shortcuts().unwrap().into_iter().filter(|x| x.folder == Some(f)).map(|x| x.id).collect();
        assert_eq!(inner, vec![a, c]);
        // Links go in and out; folders cannot nest.
        s.move_shortcut_to(b, Some(f), 0).unwrap();
        assert!(s.move_shortcut_to(f, Some(f), 0).is_err());
        s.move_shortcut_to(a, None, 5).unwrap();
        assert_eq!(s.shortcuts().unwrap().iter().filter(|x| x.folder == Some(f)).count(), 2);
        s.add_shortcut_in(Some(f), "D", "https://d/").unwrap();
        // Deleting the folder keeps its links at the top level.
        s.delete_shortcut(f).unwrap();
        let all = s.shortcuts().unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.iter().all(|x| x.folder.is_none() && !x.is_folder));
        // A folder emptied by moving its last link out disappears.
        let f2 = s.make_shortcut_folder(b, c, "Two").unwrap();
        s.move_shortcut_to(b, None, 0).unwrap();
        s.move_shortcut_to(c, None, 0).unwrap();
        assert!(s.shortcuts().unwrap().iter().all(|x| x.id != f2));
    }
}
