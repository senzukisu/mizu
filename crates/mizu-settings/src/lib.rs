//! User preferences.
//!
//! Settings are stored as human-readable TOML in the profile directory. Every field has a
//! default, so older or partially edited files keep loading after upgrades. Writes are
//! atomic (write to a temporary file, then rename) so a crash never leaves a truncated file.

mod schema;
mod search;

pub use schema::*;
pub use search::{SearchEngine, builtin_search_engines};

use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not serialize settings: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Loads settings. A missing file yields defaults; an unreadable or corrupt file is moved
/// aside to `settings.toml.corrupt` and defaults are used, so the browser always starts.
pub fn load(path: &Path) -> Settings {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Settings::default(),
        Err(e) => {
            tracing::warn!(?path, error = %e, "settings unreadable, using defaults");
            return Settings::default();
        }
    };
    match toml::from_str::<Settings>(&text) {
        Ok(mut s) => {
            s.sanitize();
            s
        }
        Err(e) => {
            tracing::error!(?path, error = %e, "settings corrupt, moving aside and using defaults");
            let aside = path.with_extension("toml.corrupt");
            if let Err(e) = std::fs::rename(path, &aside) {
                tracing::warn!(error = %e, "could not move corrupt settings aside");
            }
            Settings::default()
        }
    }
}

/// Atomically writes settings to `path`.
pub fn save(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    let text = toml::to_string_pretty(settings)?;
    let tmp = path.with_extension("toml.tmp");
    let io = |source| SettingsError::Io {
        path: path.to_owned(),
        source,
    };
    std::fs::write(&tmp, text).map_err(io)?;
    std::fs::rename(&tmp, path).map_err(io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let mut s = Settings::default();
        s.appearance.theme = Theme::Dark;
        s.appearance.accent = "#ff8800".into();
        s.privacy.tracking_prevention = TrackingPrevention::Strict;
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("nope.toml")), Settings::default());
    }

    #[test]
    fn partial_file_fills_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, "[appearance]\ntheme = \"light\"\n").unwrap();
        let s = load(&path);
        assert_eq!(s.appearance.theme, Theme::Light);
        assert_eq!(s.search, SearchSettings::default());
    }

    #[test]
    fn corrupt_file_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, "this is = = not toml").unwrap();
        assert_eq!(load(&path), Settings::default());
        assert!(!path.exists());
        assert!(dir.path().join("settings.toml.corrupt").exists());
    }

    #[test]
    fn sanitizes_invalid_values() {
        let mut s = Settings::default();
        s.appearance.accent = "javascript:alert(1)".into();
        s.tabs.sleep_after_minutes = 0;
        s.search.engine = "does-not-exist".into();
        s.general.home_page = "javascript:alert(1)".into();
        s.sanitize();
        assert_eq!(s.appearance.accent, AppearanceSettings::default().accent);
        assert!(s.tabs.sleep_after_minutes >= 1);
        assert_eq!(s.search.engine, SearchSettings::default().engine);
        assert_eq!(s.general.home_page, GeneralSettings::default().home_page);
    }
}
