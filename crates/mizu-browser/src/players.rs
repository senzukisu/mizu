//! "Open in media player": hands a video or media link to VLC, mpv, PotPlayer or MPC.

use std::path::PathBuf;
use std::sync::OnceLock;
use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows_core::HSTRING;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: &'static str,
    pub name: &'static str,
    pub path: PathBuf,
}

/// (id, name, executable names, folders under Program Files / LocalAppData\Programs)
const KNOWN: &[(&str, &str, &[&str], &[&str])] = &[
    ("vlc", "VLC", &["vlc.exe"], &["VideoLAN\\VLC"]),
    ("mpv", "mpv", &["mpv.exe"], &["mpv", "mpv.net"]),
    ("potplayer", "PotPlayer", &["PotPlayerMini64.exe", "PotPlayerMini.exe"], &["DAUM\\PotPlayer", "PotPlayer"]),
    ("mpc-hc", "MPC-HC", &["mpc-hc64.exe", "mpc-hc.exe"], &["MPC-HC", "K-Lite Codec Pack\\MPC-HC64"]),
    ("mpc-be", "MPC-BE", &["mpc-be64.exe", "mpc-be.exe"], &["MPC-BE x64", "MPC-BE"]),
];

fn app_path(root: HKEY, exe: &str) -> Option<PathBuf> {
    let key = HSTRING::from(format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{exe}"));
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: the buffer and its size in bytes are valid for the call.
    let ok = unsafe { RegGetValueW(root, &key, None, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size)) }.is_ok();
    if !ok {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let p = PathBuf::from(String::from_utf16_lossy(&buf[..len]).trim_matches('"'));
    p.is_file().then_some(p)
}

fn find(exes: &[&str], dirs: &[&str]) -> Option<PathBuf> {
    for exe in exes {
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            if let Some(p) = app_path(root, exe) {
                return Some(p);
            }
        }
    }
    let mut bases: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"].iter().filter_map(|v| std::env::var_os(v).map(PathBuf::from)).collect();
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        bases.push(PathBuf::from(l).join("Programs"));
    }
    for base in &bases {
        for dir in dirs {
            for exe in exes {
                let p = base.join(dir).join(exe);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    let mut extra: Vec<PathBuf> = Vec::new();
    if let Some(u) = std::env::var_os("USERPROFILE") {
        extra.push(PathBuf::from(u).join("scoop\\shims"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        extra.extend(std::env::split_paths(&path));
    }
    for dir in extra {
        for exe in exes {
            let p = dir.join(exe);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Media players installed on this PC (looked up once).
pub fn installed() -> &'static [Player] {
    static FOUND: OnceLock<Vec<Player>> = OnceLock::new();
    FOUND.get_or_init(|| KNOWN.iter().filter_map(|(id, name, exes, dirs)| find(exes, dirs).map(|path| Player { id, name, path })).collect())
}

/// The player chosen in settings ("" = the first one found, "off" = none).
pub fn chosen(setting: &str) -> Option<Player> {
    if setting == "off" {
        return None;
    }
    let all = installed();
    all.iter().find(|p| p.id == setting).or_else(|| all.first()).cloned()
}

const MEDIA_EXT: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v", "wmv", "flv", "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "m3u8", "m3u", "mpd", "ts"];

/// Pages players can open themselves (with their own site support).
fn video_page(url: &str) -> bool {
    let host = mizu_core::omnibox::host_of(url).unwrap_or_default();
    let host = host.trim_start_matches("www.").trim_start_matches("m.");
    let path = url.split_once("://").and_then(|(_, r)| r.find('/').map(|i| &r[i..])).unwrap_or("");
    match host {
        "youtube.com" | "music.youtube.com" => path.starts_with("/watch") || path.starts_with("/shorts/") || path.starts_with("/live/"),
        "youtu.be" | "vimeo.com" | "dailymotion.com" | "soundcloud.com" => path.len() > 1,
        "twitch.tv" => path.len() > 1,
        _ => false,
    }
}

fn is_web(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://")) && url.len() < 4096 && !url.chars().any(|c| c.is_whitespace() || c == '"')
}

/// A link a media player can open.
pub fn is_media_link(url: &str) -> bool {
    if !is_web(url) {
        return false;
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    (MEDIA_EXT.contains(&ext.as_str()) && !ext.contains('/')) || video_page(url)
}

/// The address to hand to the player for a right-clicked video: its file if it has one
/// (not a `blob:` stream), else the page when it is a video page the player understands.
pub fn media_address(source: &str, page: &str) -> Option<String> {
    if is_web(source) {
        return Some(source.to_owned());
    }
    (is_web(page) && video_page(page)).then(|| page.to_owned())
}

pub fn open(player: &Player, url: &str) {
    if !is_web(url) {
        return;
    }
    match std::process::Command::new(&player.path).arg(url).spawn() {
        Ok(_) => tracing::info!(player = player.name, "opened media in player"),
        Err(e) => tracing::warn!(player = player.name, error = %e, "could not start the media player"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_links() {
        assert!(is_media_link("https://cdn.example/a/video.MP4?x=1"));
        assert!(is_media_link("https://www.youtube.com/watch?v=abc"));
        assert!(is_media_link("https://youtu.be/abc"));
        assert!(!is_media_link("https://www.youtube.com/"));
        assert!(!is_media_link("https://example.com/page.html"));
        assert!(!is_media_link("javascript:alert(1)//.mp4"));
        assert_eq!(media_address("blob:https://www.youtube.com/x", "https://www.youtube.com/watch?v=abc").as_deref(), Some("https://www.youtube.com/watch?v=abc"));
        assert_eq!(media_address("blob:https://site/x", "https://site.example/"), None);
    }
}
