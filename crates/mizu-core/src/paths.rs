//! Profile directory layout.
//!
//! Application binaries and user data are never mixed: binaries live wherever Mizu is
//! installed, user data lives under `%LOCALAPPDATA%\Onibi Lab\Mizu\User Data\<profile>`.
//!
//! ```text
//! User Data/
//! ├── logs/
//! └── Default/
//!     ├── mizu.db            history, bookmarks, downloads, permissions, session
//!     ├── mizu.db.bak        last known-good backup (rotated at startup)
//!     ├── settings.toml      user preferences
//!     ├── engine/            engine user data folder (cookies, cache, site storage)
//!     ├── filters/           content-blocking filter lists
//!     ├── extensions/        unpacked extensions installed by the user
//!     └── running.lock       crash marker, present while Mizu runs
//! ```

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProfilePaths {
    pub root: PathBuf,
}

impl ProfilePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Default location for the named profile.
    pub fn for_profile(name: &str) -> Self {
        Self::new(user_data_dir().join(sanitize(name)))
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("mizu.db")
    }
    pub fn database_backup(&self) -> PathBuf {
        self.root.join("mizu.db.bak")
    }
    pub fn settings(&self) -> PathBuf {
        self.root.join("settings.toml")
    }
    pub fn engine_data(&self) -> PathBuf {
        self.root.join("engine")
    }
    pub fn filters(&self) -> PathBuf {
        self.root.join("filters")
    }
    pub fn extensions(&self) -> PathBuf {
        self.root.join("extensions")
    }
    pub fn crash_marker(&self) -> PathBuf {
        self.root.join("running.lock")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [
            self.root.clone(),
            self.engine_data(),
            self.filters(),
            self.extensions(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

/// `%LOCALAPPDATA%\Onibi Lab\Mizu\User Data` (or an XDG-like fallback elsewhere).
/// `MIZU_USER_DATA` overrides it, which tests and portable setups use.
pub fn user_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("MIZU_USER_DATA") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join(crate::VENDOR)
        .join(crate::APP_NAME)
        .join("User Data")
}

/// Logs live next to profiles, not inside one, so they survive profile resets.
pub fn logs_dir() -> PathBuf {
    user_data_dir().join("logs")
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "Default".into()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_profile_names() {
        assert_eq!(sanitize("../evil"), "___evil");
        assert_eq!(sanitize(""), "Default");
        assert_eq!(sanitize("Work"), "Work");
    }
}
