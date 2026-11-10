//! Native file dialogs. They are modal and pump messages, so they are only ever called from
//! `queue::outside` closures, never while browser state is borrowed.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, FileSaveDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, IFileOpenDialog, IFileSaveDialog,
    IShellItem, SIGDN_FILESYSPATH, SHCreateItemFromParsingName,
};
use windows_core::{HSTRING, PWSTR};

fn item_path(item: &IShellItem) -> Option<String> {
    // SAFETY: GetDisplayName returns a CoTaskMem string we free via take_pwstr semantics.
    unsafe {
        let p: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
        s
    }
}

fn set_folder(dialog: &windows::Win32::UI::Shell::IFileDialog, folder: Option<&str>) {
    if let Some(f) = folder.filter(|f| !f.is_empty()) {
        // SAFETY: creating a shell item from a path string.
        unsafe {
            if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(f), None) {
                let _ = dialog.SetFolder(&item);
            }
        }
    }
}

pub fn pick_folder(owner: HWND, title: &str, start: Option<&str>) -> Option<String> {
    // SAFETY: standard IFileOpenDialog usage on an STA thread.
    unsafe {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let opts = dialog.GetOptions().ok()?;
        dialog.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        dialog.SetTitle(&HSTRING::from(title)).ok()?;
        set_folder(&dialog.cast().ok()?, start);
        dialog.Show(Some(owner)).ok()?;
        item_path(&dialog.GetResult().ok()?)
    }
}

pub fn pick_file(owner: HWND, title: &str, filter_name: &str, pattern: &str) -> Option<String> {
    // SAFETY: standard IFileOpenDialog usage on an STA thread; strings outlive the call.
    unsafe {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let name = HSTRING::from(filter_name);
        let spec = HSTRING::from(pattern);
        let filters = [COMDLG_FILTERSPEC { pszName: windows_core::PCWSTR(name.as_ptr()), pszSpec: windows_core::PCWSTR(spec.as_ptr()) }];
        dialog.SetFileTypes(&filters).ok()?;
        dialog.SetTitle(&HSTRING::from(title)).ok()?;
        dialog.Show(Some(owner)).ok()?;
        item_path(&dialog.GetResult().ok()?)
    }
}

pub fn save_file(owner: HWND, suggested: &str) -> Option<String> {
    let path = std::path::Path::new(suggested);
    // SAFETY: standard IFileSaveDialog usage on an STA thread.
    unsafe {
        let dialog: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        if let Some(name) = path.file_name() {
            let _ = dialog.SetFileName(&HSTRING::from(name));
        }
        set_folder(&dialog.cast().ok()?, path.parent().and_then(|p| p.to_str()));
        dialog.Show(Some(owner)).ok()?;
        item_path(&dialog.GetResult().ok()?)
    }
}


use windows_core::Interface as _;
