//! Automatic updates of Mizu itself.
//!
//! Once a day (and on demand from `mizu://about`) Mizu downloads the signed release manifest
//! from GitHub, verifies its ed25519 signature against the compiled-in Onibi Lab key, and – if
//! the version is newer – downloads the installer and checks its SHA-256 against the signed
//! manifest. Only then is "Restart to update" offered; the installer runs silently, waits for
//! Mizu to exit, replaces the program files and starts Mizu again. The web engine itself is
//! updated by Windows independently.

use crate::app::App;
use crate::queue::{WorkerResult, WorkerSender};
use mizu_engine::ViewKey;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate { checked_at: i64 },
    /// A newer version exists but this copy is not installed (portable/dev build).
    Available { version: String, notes: String },
    /// Installer downloaded and verified; ready to install on restart.
    Ready { version: String, notes: String, #[serde(skip)] path: PathBuf },
    Failed { error: String },
}

pub enum Outcome {
    UpToDate,
    Available { version: String, notes: String },
    Ready { version: String, notes: String, path: PathBuf },
}

/// Only the copy MizuSetup installed (in the folder the user chose, recorded in the
/// uninstall entry) is replaced automatically; portable and development copies are not.
pub fn is_installed_copy() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    let Some(dir) = installed_location() else { return false };
    exe.parent().is_some_and(|p| {
        p.to_string_lossy().trim_end_matches('\\').eq_ignore_ascii_case(dir.trim_end_matches('\\'))
    })
}

fn installed_location() -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    use windows_core::HSTRING;
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: RegGetValueW writes at most `size` bytes into `buf`.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\OnibiLab.Mizu"),
            &HSTRING::from("InstallLocation"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if r.is_err() {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len])).filter(|s| !s.is_empty())
}

fn run(sender: WorkerSender, reply: Option<(ViewKey, u64)>) {
    std::thread::Builder::new()
        .name("mizu-update".into())
        .spawn(move || {
            let result = (|| -> Result<Outcome, String> {
                let manifest = mizu_privacy::fetch_https_bytes(mizu_update::MANIFEST_URL, 64 * 1024).map_err(|e| e.to_string())?;
                let sig = mizu_privacy::fetch_https_bytes(mizu_update::SIGNATURE_URL, 4 * 1024).map_err(|e| e.to_string())?;
                let sig = String::from_utf8(sig).map_err(|_| "bad signature file".to_string())?;
                let m = mizu_update::verify(&manifest, &sig).map_err(|e| e.to_string())?;
                if !mizu_update::is_newer(&m.version, mizu_core::VERSION) {
                    return Ok(Outcome::UpToDate);
                }
                if !is_installed_copy() {
                    return Ok(Outcome::Available { version: m.version, notes: m.notes });
                }
                let installer = mizu_privacy::fetch_https_bytes(&m.url, 400 * 1024 * 1024).map_err(|e| e.to_string())?;
                mizu_update::check_installer(&installer, &m).map_err(|e| e.to_string())?;
                let path = std::env::temp_dir().join(format!("MizuSetup-{}.exe", m.version));
                std::fs::write(&path, &installer).map_err(|e| e.to_string())?;
                Ok(Outcome::Ready { version: m.version, notes: m.notes, path })
            })();
            sender.send(WorkerResult::Update { result, reply });
        })
        .ok();
}

impl App {
    /// Starts an update check unless one is running or an update is already downloaded.
    pub fn check_for_update(&mut self, reply: Option<(ViewKey, u64)>) {
        if matches!(self.update, UpdateState::Checking | UpdateState::Ready { .. }) {
            if let Some((key, rid)) = reply {
                crate::commands::reply(key, rid, serde_json::to_value(&self.update).unwrap_or_default());
            }
            return;
        }
        self.update = UpdateState::Checking;
        run(self.worker.clone(), reply);
    }

    pub fn finish_update_check(&mut self, result: Result<Outcome, String>, reply: Option<(ViewKey, u64)>) {
        self.update = match result {
            Ok(Outcome::UpToDate) => UpdateState::UpToDate { checked_at: mizu_storage::now_ms() },
            Ok(Outcome::Available { version, notes }) => UpdateState::Available { version, notes },
            Ok(Outcome::Ready { version, notes, path }) => {
                tracing::info!(version, "update downloaded and verified");
                self.toast_all(&format!("Mizu {version} is ready — restart to update"));
                UpdateState::Ready { version, notes, path }
            }
            Err(error) => {
                tracing::warn!(error, "update check failed");
                UpdateState::Failed { error }
            }
        };
        for w in &mut self.windows {
            w.state_dirty = true;
        }
        if let Some((key, rid)) = reply {
            crate::commands::reply(key, rid, serde_json::to_value(&self.update).unwrap_or_default());
        }
    }

    /// Runs the verified installer silently and closes Mizu; the installer relaunches it.
    pub fn install_update(&mut self) -> Result<(), String> {
        let UpdateState::Ready { path, .. } = &self.update else {
            return Err("No update is ready".into());
        };
        let pid = std::process::id().to_string();
        std::process::Command::new(path)
            .args(["/S", "--wait-pid", &pid, "--relaunch"])
            .spawn()
            .map_err(|e| format!("could not start the installer: {e}"))?;
        tracing::info!("installing update; closing Mizu");
        let ids: Vec<_> = self.windows.iter().map(|w| w.id).collect();
        for id in ids {
            self.close_window(id);
        }
        Ok(())
    }
}
