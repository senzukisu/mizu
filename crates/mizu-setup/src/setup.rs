//! What setup actually does: paths, registry, files, shortcuts. Used by the setup window, the
//! fallback dialogs and the silent (updater) mode.

use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile};
use windows::Win32::System::Registry::*;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows_core::{HSTRING, Interface, PCWSTR};

const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.bin"));
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\OnibiLab.Mizu";
const CLIENT_KEY: &str = r"Software\Clients\StartMenuInternet\Mizu";

/// Files setup puts into the install folder (plus temporary `.new` copies). Uninstall removes
/// exactly these and then the folder only if it is empty, so choosing a folder that already
/// holds other files can never lose them.
const OWNED_FILES: &[&str] = &["mizu.exe", "mizu.new", "uninstall.exe", "uninstall.new"];

pub fn payload_len() -> usize {
    PAYLOAD.len()
}

// ───────────────────────────── Strings ─────────────────────────────

pub struct Text {
    pub title: &'static str,
    pub title_uninstall: &'static str,
    pub install_heading: &'static str,
    pub install_body: &'static str,
    pub desktop: &'static str,
    pub install_button: &'static str,
    pub done_heading: &'static str,
    pub done_body: &'static str,
    pub launch: &'static str,
    pub default_browser: &'static str,
    pub close: &'static str,
    pub failed: &'static str,
    pub uninstall_heading: &'static str,
    pub uninstall_body: &'static str,
    pub remove_data: &'static str,
    pub uninstall_button: &'static str,
    pub uninstalled: &'static str,
    pub no_payload: &'static str,
    pub choose_folder: &'static str,
    pub no_access: &'static str,
    pub in_use: &'static str,
}

const EN: Text = Text {
    title: "Mizu Setup",
    title_uninstall: "Uninstall Mizu",
    install_heading: "Install Mizu",
    install_body: "Mizu will be installed for your Windows account in\n%PATH%\n\nNo administrator rights are needed. Your data stays on this PC.",
    desktop: "Create a desktop shortcut",
    install_button: "Install",
    done_heading: "Mizu is installed",
    done_body: "Find Mizu in the Start menu. It updates itself automatically when new versions are released.",
    launch: "Launch Mizu",
    default_browser: "Make Mizu the default browser…",
    close: "Close",
    failed: "Installation failed",
    uninstall_heading: "Uninstall Mizu?",
    uninstall_body: "Mizu will be removed from this PC.",
    remove_data: "Also delete my browsing data (history, bookmarks, passwords, settings)",
    uninstall_button: "Uninstall",
    uninstalled: "Mizu has been uninstalled.",
    no_payload: "This installer was built without the Mizu program inside.",
    choose_folder: "Choose where to install Mizu",
    no_access: "Mizu can't be installed in this folder because you don't have permission to write there. Choose a folder in your user profile or on another drive.",
    in_use: "Mizu is running. Close all Mizu windows and try again.",
};

const RU: Text = Text {
    title: "Установка Mizu",
    title_uninstall: "Удаление Mizu",
    install_heading: "Установить Mizu",
    install_body: "Mizu будет установлен для вашей учётной записи Windows в\n%PATH%\n\nПрава администратора не нужны. Ваши данные остаются на этом компьютере.",
    desktop: "Создать ярлык на рабочем столе",
    install_button: "Установить",
    done_heading: "Mizu установлен",
    done_body: "Mizu есть в меню «Пуск». Новые версии устанавливаются автоматически.",
    launch: "Запустить Mizu",
    default_browser: "Сделать Mizu браузером по умолчанию…",
    close: "Закрыть",
    failed: "Не удалось установить",
    uninstall_heading: "Удалить Mizu?",
    uninstall_body: "Mizu будет удалён с этого компьютера.",
    remove_data: "Также удалить мои данные (история, закладки, пароли, настройки)",
    uninstall_button: "Удалить",
    uninstalled: "Mizu удалён.",
    no_payload: "Этот установщик собран без программы Mizu внутри.",
    choose_folder: "Куда установить Mizu",
    no_access: "В эту папку нельзя установить Mizu: нет прав на запись. Выберите папку в своём профиле или на другом диске.",
    in_use: "Mizu сейчас открыт. Закройте все окна Mizu и попробуйте снова.",
};

pub fn lang() -> &'static str {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    // SAFETY: buffer sized LOCALE_NAME_MAX_LENGTH.
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    let tag = String::from_utf16_lossy(&buf[..n.max(1) as usize - 1]).to_ascii_lowercase();
    if tag.starts_with("ru") { "ru" } else { "en" }
}

pub fn text() -> &'static Text {
    if lang() == "ru" { &RU } else { &EN }
}

// ───────────────────────────── Paths ─────────────────────────────

fn known_folder(id: &windows_core::GUID) -> PathBuf {
    // SAFETY: SHGetKnownFolderPath returns a CoTaskMem string we free.
    unsafe {
        match SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) {
            Ok(p) => {
                let s = p.to_string().unwrap_or_default();
                windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
                PathBuf::from(s)
            }
            Err(_) => PathBuf::new(),
        }
    }
}

pub fn default_install_dir() -> PathBuf {
    known_folder(&FOLDERID_LocalAppData).join("Programs").join("Mizu")
}

/// A chosen folder gets its own "Mizu" subfolder unless it already is one.
pub fn normalize_install_dir(chosen: &Path) -> PathBuf {
    if chosen.file_name().is_some_and(|n| n.eq_ignore_ascii_case("mizu")) {
        chosen.to_path_buf()
    } else {
        chosen.join("Mizu")
    }
}

/// The folder to uninstall from: where this uninstaller lives, else the registered location.
pub fn uninstall_dir() -> PathBuf {
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() {
            if dir.join("mizu.exe").exists() || me.file_name().is_some_and(|n| n.eq_ignore_ascii_case("uninstall.exe")) {
                return dir.to_path_buf();
            }
        }
    }
    installed().map_or_else(default_install_dir, |(_, d)| d)
}

fn start_menu_link() -> PathBuf {
    known_folder(&FOLDERID_Programs).join("Mizu.lnk")
}

pub fn desktop_link() -> PathBuf {
    known_folder(&FOLDERID_Desktop).join("Mizu.lnk")
}

pub fn user_data_dir() -> PathBuf {
    known_folder(&FOLDERID_LocalAppData).join("Onibi Lab").join("Mizu")
}

// ───────────────────────────── Registry ─────────────────────────────

fn reg_set(path: &str, name: Option<&str>, value: &str) -> Result<(), String> {
    // SAFETY: standard registry calls with valid strings and buffers.
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None)
            .ok()
            .map_err(|e| format!("registry {path}: {e}"))?;
        let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2);
        let name = name.map(HSTRING::from);
        let r = RegSetValueExW(key, name.as_ref().map_or(PCWSTR::null(), |n| PCWSTR(n.as_ptr())), None, REG_SZ, Some(bytes));
        let _ = RegCloseKey(key);
        r.ok().map_err(|e| format!("registry {path}: {e}"))
    }
}

fn reg_set_dword(path: &str, name: &str, value: u32) {
    // SAFETY: as above.
    unsafe {
        let mut key = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let _ = RegSetValueExW(key, &HSTRING::from(name), None, REG_DWORD, Some(&value.to_le_bytes()));
            let _ = RegCloseKey(key);
        }
    }
}

pub fn reg_dword(path: &str, name: &str) -> Option<u32> {
    let mut value = 0u32;
    let mut size = 4u32;
    // SAFETY: RegGetValueW writes at most `size` bytes into `value`.
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), RRF_RT_REG_DWORD, None, Some((&raw mut value).cast()), Some(&mut size))
    };
    r.is_ok().then_some(value)
}

fn reg_string(path: &str, name: &str) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: RegGetValueW writes at most `size` bytes into `buf`.
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size))
    };
    if r.is_err() {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..len.min(buf.len())]))
}

fn reg_delete_tree(path: &str) {
    // SAFETY: deleting keys we own under HKCU.
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(path));
    }
}

fn reg_delete_value(path: &str, name: &str) {
    // SAFETY: as above.
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, KEY_WRITE, &mut key).is_ok() {
            let _ = RegDeleteValueW(key, &HSTRING::from(name));
            let _ = RegCloseKey(key);
        }
    }
}

/// Installed version and folder, if Mizu is installed for this user.
pub fn installed() -> Option<(String, PathBuf)> {
    // A leftover entry whose program is gone (deleted by hand) does not count.
    let dir = reg_string(UNINSTALL_KEY, "InstallLocation").map(PathBuf::from).filter(|d| d.join("mizu.exe").exists())?;
    let version = reg_string(UNINSTALL_KEY, "DisplayVersion").unwrap_or_default();
    Some((version, dir))
}

/// Registers Mizu as a browser (Default apps), its URL/HTML handlers and the uninstall entry.
fn register(dir: &Path, exe: &Path, uninstaller: &Path) -> Result<(), String> {
    let exe_s = exe.to_string_lossy();
    let icon = format!("{exe_s},0");
    let open = format!("\"{exe_s}\" \"%1\"");

    for (class, desc) in [("MizuURL", "Mizu URL"), ("MizuHTML", "Mizu HTML Document")] {
        let base = format!(r"Software\Classes\{class}");
        reg_set(&base, None, desc)?;
        reg_set(&base, Some("FriendlyTypeName"), desc)?;
        if class == "MizuURL" {
            reg_set(&base, Some("URL Protocol"), "")?;
        }
        reg_set(&format!(r"{base}\DefaultIcon"), None, &icon)?;
        reg_set(&format!(r"{base}\shell\open\command"), None, &open)?;
    }
    reg_set(CLIENT_KEY, None, "Mizu")?;
    reg_set(&format!(r"{CLIENT_KEY}\DefaultIcon"), None, &icon)?;
    reg_set(&format!(r"{CLIENT_KEY}\shell\open\command"), None, &format!("\"{exe_s}\""))?;
    let caps = format!(r"{CLIENT_KEY}\Capabilities");
    reg_set(&caps, Some("ApplicationName"), "Mizu")?;
    reg_set(&caps, Some("ApplicationDescription"), "Mizu – a fast, privacy-focused browser by Onibi Lab")?;
    reg_set(&caps, Some("ApplicationIcon"), &icon)?;
    for scheme in ["http", "https"] {
        reg_set(&format!(r"{caps}\URLAssociations"), Some(scheme), "MizuURL")?;
    }
    for ext in [".htm", ".html", ".xhtml", ".svg", ".pdf"] {
        reg_set(&format!(r"{caps}\FileAssociations"), Some(ext), "MizuHTML")?;
    }
    reg_set(r"Software\RegisteredApplications", Some("Mizu"), &caps)?;

    reg_set(UNINSTALL_KEY, Some("DisplayName"), "Mizu")?;
    reg_set(UNINSTALL_KEY, Some("DisplayVersion"), VERSION)?;
    reg_set(UNINSTALL_KEY, Some("Publisher"), "Onibi Lab")?;
    reg_set(UNINSTALL_KEY, Some("DisplayIcon"), &icon)?;
    reg_set(UNINSTALL_KEY, Some("InstallLocation"), &dir.to_string_lossy())?;
    reg_set(UNINSTALL_KEY, Some("URLInfoAbout"), "https://github.com/senzukisu/mizu")?;
    reg_set(UNINSTALL_KEY, Some("UninstallString"), &format!("\"{}\" --uninstall", uninstaller.to_string_lossy()))?;
    reg_set(UNINSTALL_KEY, Some("QuietUninstallString"), &format!("\"{}\" --uninstall /S", uninstaller.to_string_lossy()))?;
    reg_set_dword(UNINSTALL_KEY, "NoModify", 1);
    reg_set_dword(UNINSTALL_KEY, "NoRepair", 1);
    reg_set_dword(UNINSTALL_KEY, "EstimatedSize", (PAYLOAD.len() / 1024) as u32);
    // SAFETY: notifies Explorer that associations changed.
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
    Ok(())
}

fn unregister() {
    reg_delete_tree(UNINSTALL_KEY);
    reg_delete_tree(CLIENT_KEY);
    reg_delete_tree(r"Software\Classes\MizuURL");
    reg_delete_tree(r"Software\Classes\MizuHTML");
    reg_delete_value(r"Software\RegisteredApplications", "Mizu");
    // SAFETY: as above.
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

// ───────────────────────────── Files & shortcuts ─────────────────────────────

fn create_shortcut(link: &Path, target: &Path) -> Result<(), String> {
    // SAFETY: standard IShellLinkW usage; COM is initialised by the caller's thread.
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED);
        let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
        shell.SetPath(&HSTRING::from(target.as_os_str())).map_err(|e| e.to_string())?;
        shell.SetDescription(&HSTRING::from("Mizu")).map_err(|e| e.to_string())?;
        shell.SetIconLocation(&HSTRING::from(target.as_os_str()), 0).map_err(|e| e.to_string())?;
        if let Some(dir) = target.parent() {
            let _ = shell.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()));
        }
        let file: IPersistFile = shell.cast().map_err(|e| e.to_string())?;
        if let Some(parent) = link.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        file.Save(&HSTRING::from(link.as_os_str()), true).map_err(|e| e.to_string())
    }
}

/// Writes `bytes` to `dest` atomically, retrying for a few seconds while an old copy runs.
fn replace_file(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = dest.with_extension("new");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    for attempt in 0..32 {
        // SAFETY: valid paths; MOVEFILE_REPLACE_EXISTING overwrites the old executable.
        let moved = unsafe {
            MoveFileExW(&HSTRING::from(tmp.as_os_str()), &HSTRING::from(dest.as_os_str()), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)
        };
        if moved.is_ok() {
            return Ok(());
        }
        if attempt == 31 {
            let _ = std::fs::remove_file(&tmp);
            return Err(text().in_use.into());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    unreachable!()
}

pub fn wait_for_pid(pid: u32) {
    // SAFETY: waiting on a process handle we open with SYNCHRONIZE only.
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(h, 60_000) == WAIT_OBJECT_0;
            let _ = CloseHandle(h);
        }
    }
}

fn check_writable(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| text().no_access.to_string())?;
    let probe = dir.join(".mizu-setup-probe");
    std::fs::write(&probe, b"ok").map_err(|_| text().no_access.to_string())?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

/// Removes the files setup owns in `dir`, then `dir` itself if nothing else is left.
fn remove_owned(dir: &Path, keep_uninstaller: bool) -> Result<(), String> {
    let exe = dir.join("mizu.exe");
    if exe.exists() {
        let mut removed = false;
        for _ in 0..8 {
            if std::fs::remove_file(&exe).is_ok() {
                removed = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        if !removed {
            return Err(text().in_use.into());
        }
    }
    for name in OWNED_FILES {
        if keep_uninstaller && name.starts_with("uninstall") {
            continue;
        }
        let _ = std::fs::remove_file(dir.join(name));
    }
    let _ = std::fs::remove_dir(dir); // only succeeds when empty
    Ok(())
}

/// Installs (or updates) Mizu into `dir`. `progress(percent, step)` reports steps for the UI.
pub fn install(dir: &Path, desktop: bool, progress: &dyn Fn(u32, &str)) -> Result<PathBuf, String> {
    if PAYLOAD.is_empty() {
        return Err(text().no_payload.into());
    }
    progress(10, "s_prepare");
    check_writable(dir)?;
    let previous = installed().map(|(_, d)| d);

    progress(35, "s_files");
    let exe = dir.join("mizu.exe");
    replace_file(&exe, PAYLOAD)?;
    // The installer copies itself next to Mizu to serve as the uninstaller.
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let uninstaller = dir.join("uninstall.exe");
    if !me.eq(&uninstaller) {
        let bytes = std::fs::read(&me).map_err(|e| e.to_string())?;
        replace_file(&uninstaller, &bytes)?;
    }

    progress(70, "s_shortcuts");
    create_shortcut(&start_menu_link(), &exe)?;
    if desktop {
        create_shortcut(&desktop_link(), &exe)?;
    } else {
        let _ = std::fs::remove_file(desktop_link());
    }

    progress(88, "s_register");
    register(dir, &exe, &uninstaller)?;

    // Moved to a new folder: remove the old copy (only our files).
    if let Some(old) = previous {
        if !same_dir(&old, dir) {
            let _ = remove_owned(&old, false);
        }
    }
    progress(100, "");
    Ok(exe)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().trim_end_matches('\\').eq_ignore_ascii_case(b.to_string_lossy().trim_end_matches('\\'))
}

/// Removes Mizu from `dir`. The running uninstaller and the then-empty folder are deleted
/// right after this process exits.
pub fn uninstall(dir: &Path, remove_data: bool) -> Result<(), String> {
    remove_owned(dir, true)?;
    let _ = std::fs::remove_file(start_menu_link());
    let _ = std::fs::remove_file(desktop_link());
    unregister();
    if remove_data {
        let data = user_data_dir();
        let _ = std::fs::remove_dir_all(&data);
        if let Some(vendor) = data.parent() {
            let _ = std::fs::remove_dir(vendor); // "Onibi Lab", only if empty
        }
    }
    let mut cmd = String::from("ping 127.0.0.1 -n 3 > nul");
    for name in ["uninstall.exe", "uninstall.new"] {
        cmd.push_str(&format!(" & del /f /q \"{}\"", dir.join(name).display()));
    }
    cmd.push_str(&format!(" & rmdir \"{}\"", dir.display()));
    run_hidden(&cmd);
    Ok(())
}

/// Deletes the setup window's temporary WebView2 folder once this process has exited.
pub fn cleanup_later(path: &Path) {
    if path.exists() {
        run_hidden(&format!("ping 127.0.0.1 -n 4 > nul & rmdir /s /q \"{}\"", path.display()));
    }
}

fn run_hidden(cmd: &str) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("cmd").arg("/c").raw_arg(cmd).creation_flags(CREATE_NO_WINDOW).spawn();
}

pub fn launch(exe: &Path) {
    let mut c = std::process::Command::new(exe);
    if let Some(dir) = exe.parent() {
        c.current_dir(dir);
    }
    let _ = c.spawn();
}

pub fn open_default_apps() {
    // SAFETY: opening a Settings URI.
    unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from("ms-settings:defaultapps?registeredAppUser=Mizu"), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}
