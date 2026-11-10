use crate::{Result, Storage, StorageError, now_ms};
use rusqlite::{OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

impl Storage {
    /// Persists the session snapshot (any serializable type, stored as JSON).
    pub fn save_session<T: Serialize>(&self, session: &T) -> Result<()> {
        let data = serde_json_like::to_string(session)?;
        self.conn().execute(
            "INSERT INTO session (id, data, saved_at) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data, saved_at = excluded.saved_at",
            params![data, now_ms()],
        )?;
        Ok(())
    }

    /// Loads the last snapshot. A snapshot that no longer parses is discarded, not fatal.
    pub fn load_session<T: DeserializeOwned>(&self) -> Result<Option<T>> {
        let data: Option<String> = self
            .conn()
            .query_row("SELECT data FROM session WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        let Some(data) = data else { return Ok(None) };
        match serde_json_like::from_str(&data) {
            Ok(v) => Ok(Some(v)),
            Err(e) => {
                tracing::warn!(error = %e, "discarding unreadable session snapshot");
                Ok(None)
            }
        }
    }

    pub fn clear_session(&self) -> Result<()> {
        self.conn().execute("DELETE FROM session", [])?;
        Ok(())
    }
}

/// Thin wrapper so the storage crate's error type stays independent of the JSON library.
mod serde_json_like {
    use super::StorageError;
    use serde::{Serialize, de::DeserializeOwned};

    pub fn to_string<T: Serialize>(v: &T) -> Result<String, StorageError> {
        serde_json::to_string(v).map_err(|e| StorageError::Json(e.to_string()))
    }

    pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, StorageError> {
        serde_json::from_str(s).map_err(|e| StorageError::Json(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Snap {
        tabs: Vec<String>,
    }

    #[test]
    fn save_and_load() {
        let s = Storage::in_memory().unwrap();
        assert_eq!(s.load_session::<Snap>().unwrap(), None);
        let snap = Snap { tabs: vec!["https://a/".into()] };
        s.save_session(&snap).unwrap();
        assert_eq!(s.load_session::<Snap>().unwrap(), Some(snap));
        s.clear_session().unwrap();
        assert_eq!(s.load_session::<Snap>().unwrap(), None);
    }
}
