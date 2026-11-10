//! Downloads. Mizu replaces the engine's download bubble with its own manager.

use crate::util::{get_bool, get_string};
use std::cell::RefCell;
use std::rc::Rc;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{BytesReceivedChangedEventHandler, StateChangedEventHandler};
use windows_core::HSTRING;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    InProgress,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone)]
pub struct DownloadInfo {
    pub uri: String,
    pub path: String,
    pub mime: String,
    pub received: i64,
    /// -1 when the server did not send a length.
    pub total: i64,
    pub status: DownloadStatus,
    pub can_resume: bool,
    /// Engine interrupt reason (for diagnostics), 0 = none.
    pub reason: i32,
}

struct Starting {
    args: ICoreWebView2DownloadStartingEventArgs,
    deferral: ICoreWebView2Deferral,
}

impl Starting {
    fn finish(self, path: Option<&str>, cancel: bool) {
        // SAFETY: setters and Complete on live objects on the UI thread; called exactly once.
        unsafe {
            if cancel {
                let _ = self.args.SetCancel(true);
            } else if let Some(p) = path {
                let _ = self.args.SetResultFilePath(&HSTRING::from(p));
            }
            // Hide the engine's own download UI; Mizu shows its manager instead.
            let _ = self.args.SetHandled(true);
            let _ = self.deferral.Complete();
        }
    }
}

struct Pending(RefCell<Option<Starting>>);

impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(s) = self.0.borrow_mut().take() {
            s.finish(None, false);
        }
    }
}

/// A live download. Cloning shares the same operation.
#[derive(Clone)]
pub struct Download {
    op: ICoreWebView2DownloadOperation,
    pending: Rc<Pending>,
}

impl Download {
    pub(crate) fn new(
        op: ICoreWebView2DownloadOperation,
        args: ICoreWebView2DownloadStartingEventArgs,
        deferral: ICoreWebView2Deferral,
    ) -> Self {
        Self {
            op,
            pending: Rc::new(Pending(RefCell::new(Some(Starting { args, deferral })))),
        }
    }

    /// The path the engine suggests (meaningful before `accept`).
    pub fn suggested_path(&self) -> String {
        match &*self.pending.0.borrow() {
            // SAFETY: getter on a live args object.
            Some(s) => get_string(|p| unsafe { s.args.ResultFilePath(p) }),
            None => self.info().path,
        }
    }

    /// Lets the download proceed, optionally to a different path.
    pub fn accept(&self, path: Option<&str>) {
        if let Some(s) = self.pending.0.borrow_mut().take() {
            s.finish(path, false);
        }
    }

    /// Cancels the download before it begins.
    pub fn reject(&self) {
        if let Some(s) = self.pending.0.borrow_mut().take() {
            s.finish(None, true);
        }
    }

    pub fn info(&self) -> DownloadInfo {
        let op = &self.op;
        // SAFETY: getters on a live operation, called on the UI thread.
        unsafe {
            let mut received = 0i64;
            let _ = op.BytesReceived(&mut received);
            let mut total = -1i64;
            let _ = op.TotalBytesToReceive(&mut total);
            let mut state = COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS;
            let _ = op.State(&mut state);
            let mut reason = COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NONE;
            let _ = op.InterruptReason(&mut reason);
            let status = match state {
                COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED => DownloadStatus::Completed,
                COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED => match reason {
                    COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED => DownloadStatus::Paused,
                    COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED => DownloadStatus::Cancelled,
                    _ => DownloadStatus::Failed,
                },
                _ => DownloadStatus::InProgress,
            };
            DownloadInfo {
                uri: get_string(|p| op.Uri(p)),
                path: get_string(|p| op.ResultFilePath(p)),
                mime: get_string(|p| op.MimeType(p)),
                received,
                total,
                status,
                can_resume: get_bool(|b| op.CanResume(b)),
                reason: reason.0,
            }
        }
    }

    /// Calls `on_update` whenever progress or state changes.
    pub fn subscribe(&self, on_update: impl Fn(DownloadInfo) + 'static) {
        let on_update = Rc::new(on_update);
        let me = self.clone();
        let cb = on_update.clone();
        let mut token = 0i64;
        // SAFETY: registering COM event handlers on the UI thread.
        unsafe {
            let _ = self.op.add_BytesReceivedChanged(
                &BytesReceivedChangedEventHandler::create(Box::new(move |_, _| {
                    cb(me.info());
                    Ok(())
                })),
                &mut token,
            );
            let me = self.clone();
            let _ = self.op.add_StateChanged(
                &StateChangedEventHandler::create(Box::new(move |_, _| {
                    on_update(me.info());
                    Ok(())
                })),
                &mut token,
            );
        }
    }

    pub fn pause(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.op.Pause();
        }
    }

    pub fn resume(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.op.Resume();
        }
    }

    pub fn cancel(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.op.Cancel();
        }
    }
}
