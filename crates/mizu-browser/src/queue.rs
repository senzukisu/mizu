//! Event queue between engine callbacks and the browser state.
//!
//! Engine callbacks may fire while browser state is already borrowed (for example when a
//! modal dialog pumps messages). To make re-entrancy impossible by construction, callbacks
//! never touch [`crate::app::App`] directly: they push a [`Queued`] item and post a message;
//! the item is processed from the message loop with exclusive access to the state.
//! Worker threads use [`WorkerSender`], which is `Send`.

use crate::app::App;
use mizu_core::ids::DownloadId;
use mizu_engine::{Download, DownloadInfo, NewWindowRequest, PermissionRequest, ViewEvent, ViewKey};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::mpsc;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub const WM_MIZU_PUMP: u32 = WM_APP + 1;
pub const WM_MIZU_WORKER: u32 = WM_APP + 2;

pub enum Queued {
    View(ViewKey, ViewEvent),
    Message { key: ViewKey, source: String, json: String },
    NewWindow(ViewKey, NewWindowRequest),
    Permission(ViewKey, PermissionRequest),
    Download(ViewKey, Download),
    DownloadUpdate(DownloadId, DownloadInfo),
    Shortcut(ViewKey, crate::shortcuts::Shortcut),
    InstallScript { key: ViewKey, url: String },
    ShowBlocked { key: ViewKey, url: String },
    /// Runs with exclusive access to the browser state.
    WithApp(Box<dyn FnOnce(&mut App)>),
    /// Runs with no state borrowed (modal dialogs, message boxes).
    Outside(Box<dyn FnOnce()>),
}

impl Queued {
    /// Short description for slow-work diagnostics.
    pub fn label(&self) -> String {
        match self {
            Self::View(_, e) => {
                let s = format!("{e:?}");
                format!("view {}", s.split(['(', ' ', '{']).next().unwrap_or(""))
            }
            Self::Message { json, .. } => {
                let cmd = json.split("\"cmd\":\"").nth(1).and_then(|r| r.split('"').next()).unwrap_or("?");
                format!("ui {cmd}")
            }
            Self::NewWindow(..) => "new window".into(),
            Self::Permission(..) => "permission".into(),
            Self::Download(..) => "download".into(),
            Self::DownloadUpdate(..) => "download update".into(),
            Self::Shortcut(_, s) => format!("shortcut {s:?}"),
            Self::InstallScript { .. } => "install script".into(),
            Self::ShowBlocked { .. } => "show blocked".into(),
            Self::WithApp(_) => "with app".into(),
            Self::Outside(_) => "outside".into(),
        }
    }
}

/// Results from worker threads.
pub enum WorkerResult {
    SearchSuggestions {
        window: mizu_core::ids::WindowId,
        query: String,
        items: Vec<String>,
    },
    BlockerReady(std::sync::Arc<mizu_privacy::ContentBlocker>, u64),
    FiltersUpdated,
    ScriptsFetched {
        key: Option<mizu_engine::ViewKey>,
        items: Vec<crate::scripts::Fetched>,
        failures: Vec<(String, String)>,
    },
    ScriptUpdates {
        updated: Vec<crate::scripts::Fetched>,
        checked: Vec<String>,
        failed: usize,
        reply: Option<(mizu_engine::ViewKey, u64)>,
    },
    MediaReport(Box<mizu_media::SystemMediaReport>, u64),
    Update {
        result: Result<crate::update::Outcome, String>,
        reply: Option<(mizu_engine::ViewKey, u64)>,
    },
    ExtensionDownloaded {
        key: mizu_engine::ViewKey,
        rid: Option<u64>,
        result: Result<Vec<u8>, String>,
    },
}

thread_local! {
    static QUEUE: RefCell<VecDeque<Queued>> = const { RefCell::new(VecDeque::new()) };
    static POSTED: Cell<bool> = const { Cell::new(false) };
    static TARGET: Cell<isize> = const { Cell::new(0) };
}

/// The hidden message window that receives pump/worker messages.
pub fn set_target(hwnd: HWND) {
    TARGET.with(|t| t.set(hwnd.0 as isize));
}

pub fn target() -> HWND {
    HWND(TARGET.with(|t| t.get()) as *mut _)
}

pub fn push(item: Queued) {
    QUEUE.with(|q| q.borrow_mut().push_back(item));
    if !POSTED.with(|p| p.replace(true)) {
        // SAFETY: posting to our own message-only window.
        unsafe {
            let _ = PostMessageW(Some(target()), WM_MIZU_PUMP, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn with_app(f: impl FnOnce(&mut App) + 'static) {
    push(Queued::WithApp(Box::new(f)));
}

pub fn outside(f: impl FnOnce() + 'static) {
    push(Queued::Outside(Box::new(f)));
}

pub fn pop() -> Option<Queued> {
    let item = QUEUE.with(|q| q.borrow_mut().pop_front());
    if item.is_none() {
        POSTED.with(|p| p.set(false));
    }
    item
}

/// Cloneable, `Send` handle for worker threads.
#[derive(Clone)]
pub struct WorkerSender {
    tx: mpsc::Sender<WorkerResult>,
    hwnd: isize,
}

impl WorkerSender {
    pub fn send(&self, result: WorkerResult) {
        if self.tx.send(result).is_ok() {
            // SAFETY: PostMessage is thread-safe; the target is our message window.
            unsafe {
                let _ = PostMessageW(Some(HWND(self.hwnd as *mut _)), WM_MIZU_WORKER, WPARAM(0), LPARAM(0));
            }
        }
    }
}

pub fn worker_channel() -> (WorkerSender, mpsc::Receiver<WorkerResult>) {
    let (tx, rx) = mpsc::channel();
    (
        WorkerSender {
            tx,
            hwnd: TARGET.with(|t| t.get()),
        },
        rx,
    )
}
