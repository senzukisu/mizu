//! Site permission catalogue.
//!
//! Defaults are conservative: nothing powerful is granted without asking, and the answer is
//! remembered per origin only when the user ticks "remember".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    Camera,
    Microphone,
    Geolocation,
    Notifications,
    ClipboardRead,
    MultipleDownloads,
    MidiSysex,
    WindowManagement,
    LocalFonts,
    FileEditing,
    Autoplay,
    OtherSensors,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDefault {
    Ask,
    Allow,
    Block,
}

impl PermissionKind {
    pub const ALL: [PermissionKind; 12] = [
        Self::Camera,
        Self::Microphone,
        Self::Geolocation,
        Self::Notifications,
        Self::ClipboardRead,
        Self::MultipleDownloads,
        Self::MidiSysex,
        Self::WindowManagement,
        Self::LocalFonts,
        Self::FileEditing,
        Self::Autoplay,
        Self::OtherSensors,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::Geolocation => "geolocation",
            Self::Notifications => "notifications",
            Self::ClipboardRead => "clipboard_read",
            Self::MultipleDownloads => "multiple_downloads",
            Self::MidiSysex => "midi_sysex",
            Self::WindowManagement => "window_management",
            Self::LocalFonts => "local_fonts",
            Self::FileEditing => "file_editing",
            Self::Autoplay => "autoplay",
            Self::OtherSensors => "other_sensors",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|k| k.as_str() == s).unwrap_or(Self::Unknown)
    }

    /// Phrase used in the permission prompt: "<site> wants to <description>".
    pub fn description(self) -> &'static str {
        match self {
            Self::Camera => "use your camera",
            Self::Microphone => "use your microphone",
            Self::Geolocation => "know your location",
            Self::Notifications => "show notifications",
            Self::ClipboardRead => "see text and images copied to the clipboard",
            Self::MultipleDownloads => "download multiple files",
            Self::MidiSysex => "control MIDI devices",
            Self::WindowManagement => "manage windows on all your displays",
            Self::LocalFonts => "use fonts installed on this device",
            Self::FileEditing => "edit files on this device",
            Self::Autoplay => "play media automatically",
            Self::OtherSensors => "use motion and light sensors",
            Self::Unknown => "use a device feature",
        }
    }

    pub fn default_policy(self) -> PermissionDefault {
        match self {
            // Harmless and expected by many sites (audio already gated by engine policy).
            Self::Autoplay => PermissionDefault::Allow,
            // Requests we cannot describe accurately are refused rather than guessed.
            Self::Unknown => PermissionDefault::Block,
            _ => PermissionDefault::Ask,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        for k in PermissionKind::ALL {
            assert_eq!(PermissionKind::parse(k.as_str()), k);
        }
        assert_eq!(PermissionKind::parse("bogus"), PermissionKind::Unknown);
        assert_eq!(PermissionKind::Camera.default_policy(), PermissionDefault::Ask);
        assert_eq!(PermissionKind::Unknown.default_policy(), PermissionDefault::Block);
    }
}
