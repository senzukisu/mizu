//! Mizu's engine layer over Microsoft Edge WebView2.
//!
//! WebView2 embeds the evergreen Chromium-based Edge runtime: current HTML/CSS/JS, WebAssembly,
//! WebGL/WebGPU, WebRTC, HTTP/3, hardware video decoding, H.264/AAC, the Chromium sandbox and
//! site isolation – all serviced by Windows Update independently of Mizu releases.
//!
//! This crate is the *only* place that touches COM. Everything above it (`mizu-browser`) talks
//! to [`Engine`], [`View`] and [`Download`] and receives callbacks through [`ViewHandler`].
//! That seam is what a future CEF or Servo backend would implement (see `docs/ARCHITECTURE.md`).
//!
//! Threading: WebView2 is single-threaded apartment COM. Every call and every callback happens
//! on the UI thread; none of the types here are `Send`.

mod download;
mod drop_target;
pub mod host;
mod environment;
mod events;
mod profile;
mod util;
mod view;

pub use download::{Download, DownloadInfo, DownloadStatus};
pub use environment::{Engine, EngineOptions, ProfileKind, TrackingLevel, browser_version};
pub use events::{
    NavigationDecision, NavigationRequest, NewWindowRequest, PermissionKind, PermissionRequest,
    ResourceDecision, ResourceRequest, ViewEvent, ViewHandler, ViewKey,
};
pub use profile::{BrowsingDataKinds, ExtensionInfo};
pub use view::{View, ViewConfig};

pub use windows::Win32::Foundation::{HWND, RECT};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("WebView2 runtime is not installed or could not be started: {0}")]
    RuntimeUnavailable(String),
    #[error("WebView2 call failed: {0}")]
    Com(#[from] windows_core::Error),
    #[error("WebView2 error: {0}")]
    WebView2(String),
}

impl From<webview2_com::Error> for EngineError {
    fn from(e: webview2_com::Error) -> Self {
        match e {
            webview2_com::Error::WindowsError(e) => Self::Com(e),
            other => Self::WebView2(format!("{other:?}")),
        }
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;
