use crate::{Result, Storage, now_ms};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionState {
    Allow,
    Block,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PermissionRecord {
    pub origin: String,
    pub kind: String,
    pub state: PermissionState,
    pub updated_at: i64,
}

impl Storage {
    /// Remembered decision for `origin` + `kind`, if any.
    pub fn permission(&self, origin: &str, kind: &str) -> Result<Option<PermissionState>> {
        let state: Option<String> = self
            .conn()
            .query_row(
                "SELECT state FROM permissions WHERE origin = ?1 AND kind = ?2",
                params![origin, kind],
                |r| r.get(0),
            )
            .optional()?;
        Ok(state.map(|s| {
            if s == "allow" {
                PermissionState::Allow
            } else {
                PermissionState::Block
            }
        }))
    }

    pub fn set_permission(&self, origin: &str, kind: &str, state: PermissionState) -> Result<()> {
        let state = match state {
            PermissionState::Allow => "allow",
            PermissionState::Block => "block",
        };
        self.conn().execute(
            "INSERT INTO permissions (origin, kind, state, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(origin, kind) DO UPDATE SET state = excluded.state, updated_at = excluded.updated_at",
            params![origin, kind, state, now_ms()],
        )?;
        Ok(())
    }

    pub fn reset_permission(&self, origin: &str, kind: &str) -> Result<()> {
        self.conn().execute(
            "DELETE FROM permissions WHERE origin = ?1 AND kind = ?2",
            params![origin, kind],
        )?;
        Ok(())
    }

    pub fn all_permissions(&self) -> Result<Vec<PermissionRecord>> {
        let mut stmt = self.conn().prepare_cached(
            "SELECT origin, kind, state, updated_at FROM permissions ORDER BY origin, kind",
        )?;
        let rows = stmt.query_map([], |r| {
            let state: String = r.get(2)?;
            Ok(PermissionRecord {
                origin: r.get(0)?,
                kind: r.get(1)?,
                state: if state == "allow" {
                    PermissionState::Allow
                } else {
                    PermissionState::Block
                },
                updated_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn clear_permissions(&self) -> Result<usize> {
        Ok(self.conn().execute("DELETE FROM permissions", [])?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_decisions() {
        let s = Storage::in_memory().unwrap();
        assert_eq!(s.permission("https://a.example", "camera").unwrap(), None);
        s.set_permission("https://a.example", "camera", PermissionState::Allow).unwrap();
        s.set_permission("https://a.example", "camera", PermissionState::Block).unwrap();
        assert_eq!(
            s.permission("https://a.example", "camera").unwrap(),
            Some(PermissionState::Block)
        );
        assert_eq!(s.all_permissions().unwrap().len(), 1);
        s.reset_permission("https://a.example", "camera").unwrap();
        assert!(s.all_permissions().unwrap().is_empty());
    }
}
