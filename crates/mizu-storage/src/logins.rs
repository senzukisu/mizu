//! Saved logins for the password manager.
//!
//! This module stores passwords as **opaque encrypted bytes**: encryption and decryption
//! happen in the browser with the operating system's key store (Windows DPAPI), so the
//! database file alone never reveals a password.

use crate::{Result, Storage, StorageError, like_pattern, now_ms};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Login {
    pub id: i64,
    /// `scheme://host[:port]` the login belongs to.
    pub origin: String,
    pub username: String,
    /// Encrypted password (never serialised to UI surfaces).
    #[serde(skip)]
    pub secret: Vec<u8>,
    pub note: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used: i64,
}

const COLUMNS: &str = "id, origin, username, secret, note, created_at, updated_at, last_used";

fn row_to_login(r: &rusqlite::Row<'_>) -> rusqlite::Result<Login> {
    Ok(Login {
        id: r.get(0)?,
        origin: r.get(1)?,
        username: r.get(2)?,
        secret: r.get(3)?,
        note: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        last_used: r.get(7)?,
    })
}

impl Storage {
    /// Inserts a login or, if one exists for the same origin and username, replaces its
    /// password. Returns the id.
    pub fn upsert_login(&self, origin: &str, username: &str, secret: &[u8], note: &str) -> Result<i64> {
        if origin.is_empty() {
            return Err(StorageError::Invalid("login without origin"));
        }
        let now = now_ms();
        self.conn().execute(
            "INSERT INTO logins (origin, username, secret, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(origin, username) DO UPDATE SET
                secret = excluded.secret,
                note = CASE WHEN excluded.note = '' THEN note ELSE excluded.note END,
                updated_at = excluded.updated_at",
            params![origin, username, secret, note, now],
        )?;
        Ok(self.conn().query_row(
            "SELECT id FROM logins WHERE origin = ?1 AND username = ?2",
            params![origin, username],
            |r| r.get(0),
        )?)
    }

    pub fn update_login(&self, id: i64, origin: &str, username: &str, secret: Option<&[u8]>, note: &str) -> Result<()> {
        let n = match secret {
            Some(s) => self.conn().execute(
                "UPDATE logins SET origin = ?2, username = ?3, secret = ?4, note = ?5, updated_at = ?6 WHERE id = ?1",
                params![id, origin, username, s, note, now_ms()],
            )?,
            None => self.conn().execute(
                "UPDATE logins SET origin = ?2, username = ?3, note = ?4, updated_at = ?5 WHERE id = ?1",
                params![id, origin, username, note, now_ms()],
            )?,
        };
        if n == 0 { Err(StorageError::NotFound) } else { Ok(()) }
    }

    pub fn login(&self, id: i64) -> Result<Login> {
        self.conn()
            .query_row(&format!("SELECT {COLUMNS} FROM logins WHERE id = ?1"), [id], row_to_login)
            .optional()?
            .ok_or(StorageError::NotFound)
    }

    /// All logins (optionally filtered by site or username), sorted by site.
    pub fn logins(&self, query: &str) -> Result<Vec<Login>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM logins
             WHERE ?1 = '' OR origin LIKE ?2 ESCAPE '\\' OR username LIKE ?2 ESCAPE '\\' OR note LIKE ?2 ESCAPE '\\'
             ORDER BY origin, username"
        ))?;
        let rows = stmt.query_map(params![query, like_pattern(query)], row_to_login)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Logins usable on `origin`, most recently used first.
    pub fn logins_for_origin(&self, origin: &str) -> Result<Vec<Login>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM logins WHERE origin = ?1 ORDER BY last_used DESC, username"
        ))?;
        let rows = stmt.query_map([origin], row_to_login)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn count_logins_for_origin(&self, origin: &str) -> Result<i64> {
        Ok(self.conn().query_row("SELECT COUNT(*) FROM logins WHERE origin = ?1", [origin], |r| r.get(0))?)
    }

    pub fn touch_login(&self, id: i64) -> Result<()> {
        self.conn().execute("UPDATE logins SET last_used = ?2 WHERE id = ?1", params![id, now_ms()])?;
        Ok(())
    }

    pub fn delete_login(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM logins WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn clear_logins(&self) -> Result<usize> {
        Ok(self.conn().execute("DELETE FROM logins", [])?)
    }

    /// Sites where the user chose "Never save".
    pub fn is_never_save(&self, origin: &str) -> Result<bool> {
        Ok(self
            .conn()
            .query_row("SELECT 1 FROM login_never WHERE origin = ?1", [origin], |_| Ok(()))
            .optional()?
            .is_some())
    }

    pub fn set_never_save(&self, origin: &str, never: bool) -> Result<()> {
        if never {
            self.conn().execute("INSERT OR IGNORE INTO login_never (origin) VALUES (?1)", [origin])?;
        } else {
            self.conn().execute("DELETE FROM login_never WHERE origin = ?1", [origin])?;
        }
        Ok(())
    }

    pub fn never_save_sites(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn().prepare_cached("SELECT origin FROM login_never ORDER BY origin")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_lifecycle() {
        let s = Storage::in_memory().unwrap();
        let id = s.upsert_login("https://a.example", "alice", b"enc1", "").unwrap();
        let id2 = s.upsert_login("https://a.example", "alice", b"enc2", "work").unwrap();
        assert_eq!(id, id2, "same site + user updates in place");
        assert_eq!(s.login(id).unwrap().secret, b"enc2");
        s.upsert_login("https://b.example", "bob", b"x", "").unwrap();
        assert_eq!(s.logins("").unwrap().len(), 2);
        assert_eq!(s.logins("alice").unwrap().len(), 1);
        assert_eq!(s.count_logins_for_origin("https://a.example").unwrap(), 1);
        s.update_login(id, "https://a.example", "alice2", None, "n").unwrap();
        assert_eq!(s.login(id).unwrap().username, "alice2");
        assert_eq!(s.login(id).unwrap().secret, b"enc2");
        s.delete_login(id).unwrap();
        assert!(s.login(id).is_err());

        assert!(!s.is_never_save("https://c.example").unwrap());
        s.set_never_save("https://c.example", true).unwrap();
        assert!(s.is_never_save("https://c.example").unwrap());
        assert_eq!(s.never_save_sites().unwrap(), ["https://c.example"]);
    }
}
