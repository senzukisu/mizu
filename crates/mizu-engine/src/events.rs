//! Event types delivered to the browser layer.

use crate::download::Download;
use webview2_com::Microsoft::Web::WebView2::Win32::*;

/// Opaque identifier chosen by the browser when it creates a view.
pub type ViewKey = u64;

#[derive(Debug, Clone)]
pub enum ViewEvent {
    SourceChanged { uri: String },
    TitleChanged { title: String },
    HistoryChanged { can_go_back: bool, can_go_forward: bool },
    ContentLoading,
    NavigationCompleted { success: bool, http_status: i32, web_error: i32 },
    DomContentLoaded,
    /// PNG bytes of the page icon (empty when the page has none).
    FaviconChanged { png: Vec<u8> },
    AudioChanged { playing: bool, muted: bool },
    FullscreenChanged { fullscreen: bool },
    ProcessFailed { unresponsive: bool, description: String },
    /// `window.close()` from script.
    CloseRequested,
    StatusText { text: String },
    ZoomChanged { factor: f64 },
    GotFocus,
}

#[derive(Debug, Clone)]
pub struct NavigationRequest {
    pub uri: String,
    pub navigation_id: u64,
    pub user_initiated: bool,
    pub redirect: bool,
    /// Back/forward or reload of an existing history entry (not a new document).
    pub history_navigation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationDecision {
    Allow,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub uri: String,
    pub method: String,
    /// uBlock-style resource type (`document`, `script`, `image`, …).
    pub resource_type: &'static str,
}

#[derive(Debug, Clone)]
pub enum ResourceDecision {
    Continue,
    /// Continue, adding request headers (e.g. `Sec-GPC: 1`).
    ContinueWithHeaders(Vec<(String, String)>),
    /// Continue with a different URL (tracking parameters stripped).
    Rewrite(String),
    /// Answer locally without touching the network.
    Respond {
        status: i32,
        mime: String,
        body: Vec<u8>,
        extra_headers: String,
    },
    /// Fail the request as blocked.
    Block,
}

/// Engine-level permission kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    FileReadWrite,
    Autoplay,
    OtherSensors,
    Unknown,
}

impl PermissionKind {
    pub(crate) fn from_raw(k: COREWEBVIEW2_PERMISSION_KIND) -> Self {
        match k {
            COREWEBVIEW2_PERMISSION_KIND_CAMERA => Self::Camera,
            COREWEBVIEW2_PERMISSION_KIND_MICROPHONE => Self::Microphone,
            COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION => Self::Geolocation,
            COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS => Self::Notifications,
            COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ => Self::ClipboardRead,
            COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS => Self::MultipleDownloads,
            COREWEBVIEW2_PERMISSION_KIND_MIDI_SYSTEM_EXCLUSIVE_MESSAGES => Self::MidiSysex,
            COREWEBVIEW2_PERMISSION_KIND_WINDOW_MANAGEMENT => Self::WindowManagement,
            COREWEBVIEW2_PERMISSION_KIND_LOCAL_FONTS => Self::LocalFonts,
            COREWEBVIEW2_PERMISSION_KIND_FILE_READ_WRITE => Self::FileReadWrite,
            COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY => Self::Autoplay,
            COREWEBVIEW2_PERMISSION_KIND_OTHER_SENSORS => Self::OtherSensors,
            _ => Self::Unknown,
        }
    }
}

/// A pending permission prompt. The engine waits until [`PermissionRequest::respond`] is
/// called (or the request is dropped, which falls back to the engine default: deny).
pub struct PermissionRequest {
    pub uri: String,
    pub kind: PermissionKind,
    pub user_initiated: bool,
    args: ICoreWebView2PermissionRequestedEventArgs,
    deferral: Option<ICoreWebView2Deferral>,
}

impl PermissionRequest {
    pub(crate) fn new(
        uri: String,
        kind: PermissionKind,
        user_initiated: bool,
        args: ICoreWebView2PermissionRequestedEventArgs,
        deferral: ICoreWebView2Deferral,
    ) -> Self {
        Self {
            uri,
            kind,
            user_initiated,
            args,
            deferral: Some(deferral),
        }
    }

    /// `Some(true)` allow, `Some(false)` deny, `None` engine default.
    pub fn respond(mut self, allow: Option<bool>) {
        let state = match allow {
            Some(true) => COREWEBVIEW2_PERMISSION_STATE_ALLOW,
            Some(false) => COREWEBVIEW2_PERMISSION_STATE_DENY,
            None => COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
        };
        // SAFETY: COM calls on the UI thread on objects we hold references to.
        unsafe {
            let _ = self.args.SetState(state);
            if let Some(d) = self.deferral.take() {
                let _ = d.Complete();
            }
        }
    }
}

impl Drop for PermissionRequest {
    fn drop(&mut self) {
        if let Some(d) = self.deferral.take() {
            // SAFETY: completing an outstanding deferral; deny by default.
            unsafe {
                let _ = self.args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY);
                let _ = d.Complete();
            }
        }
    }
}

/// A pending `window.open` / target=_blank request.
pub struct NewWindowRequest {
    pub uri: String,
    pub user_initiated: bool,
    args: ICoreWebView2NewWindowRequestedEventArgs,
    deferral: Option<ICoreWebView2Deferral>,
}

impl NewWindowRequest {
    pub(crate) fn new(
        uri: String,
        user_initiated: bool,
        args: ICoreWebView2NewWindowRequestedEventArgs,
        deferral: ICoreWebView2Deferral,
    ) -> Self {
        Self {
            uri,
            user_initiated,
            args,
            deferral: Some(deferral),
        }
    }

    /// Hands the request to `target`, which must be a freshly created, not yet navigated view.
    /// The opener relationship (`window.opener`) is preserved, which OAuth pop-ups rely on.
    pub fn complete_with(mut self, target: &crate::View) {
        // SAFETY: COM calls on the UI thread.
        unsafe {
            let _ = self.args.SetNewWindow(target.core());
            let _ = self.args.SetHandled(true);
            if let Some(d) = self.deferral.take() {
                let _ = d.Complete();
            }
        }
    }

    /// Suppresses the pop-up entirely.
    pub fn block(mut self) {
        // SAFETY: COM calls on the UI thread.
        unsafe {
            let _ = self.args.SetHandled(true);
            if let Some(d) = self.deferral.take() {
                let _ = d.Complete();
            }
        }
    }
}

impl Drop for NewWindowRequest {
    fn drop(&mut self) {
        if let Some(d) = self.deferral.take() {
            // SAFETY: completing an outstanding deferral; the request is suppressed.
            unsafe {
                let _ = self.args.SetHandled(true);
                let _ = d.Complete();
            }
        }
    }
}

/// Callbacks from a view. All methods run on the UI thread. Decision methods must return
/// quickly; they block the engine's request pipeline.
pub trait ViewHandler {
    fn on_event(&self, key: ViewKey, event: ViewEvent);

    fn on_navigation_starting(&self, _key: ViewKey, _request: &NavigationRequest) -> NavigationDecision {
        NavigationDecision::Allow
    }

    /// Navigations of sub-frames (iframes).
    fn on_frame_navigation_starting(&self, _key: ViewKey, _uri: &str) -> NavigationDecision {
        NavigationDecision::Allow
    }

    fn on_resource_request(&self, _key: ViewKey, _request: &ResourceRequest) -> ResourceDecision {
        ResourceDecision::Continue
    }

    fn on_new_window(&self, _key: ViewKey, request: NewWindowRequest) {
        request.block();
    }

    fn on_permission(&self, _key: ViewKey, request: PermissionRequest) {
        request.respond(None);
    }

    /// A download is starting. The engine waits until [`Download::accept`] or
    /// [`Download::reject`] is called (or the last clone is dropped, which accepts it with the
    /// engine's suggested path). This allows asking the user where to save first.
    fn on_download(&self, _key: ViewKey, download: Download) {
        download.accept(None);
    }

    /// A JSON message posted by the page with `chrome.webview.postMessage`. `source` is the
    /// URL of the document that sent it – the browser decides how much to trust it.
    fn on_web_message(&self, _key: ViewKey, _source: &str, _json: &str) {}

    /// Keyboard shortcut pressed while the view has focus. Return `true` if handled.
    fn on_accelerator(&self, _key: ViewKey, _vk: u32, _ctrl: bool, _shift: bool, _alt: bool) -> bool {
        false
    }

    /// Link to an external application (`mailto:`, `steam:`…). Return `true` to allow the
    /// engine's own confirmation dialog, `false` to cancel.
    fn on_external_uri(&self, _key: ViewKey, _uri: &str) -> bool {
        true
    }
}

pub(crate) fn resource_type(ctx: COREWEBVIEW2_WEB_RESOURCE_CONTEXT) -> &'static str {
    match ctx {
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT => "document",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_STYLESHEET => "stylesheet",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_IMAGE => "image",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_MEDIA => "media",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FONT => "font",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_SCRIPT => "script",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_XML_HTTP_REQUEST | COREWEBVIEW2_WEB_RESOURCE_CONTEXT_FETCH => {
            "xmlhttprequest"
        }
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_WEBSOCKET => "websocket",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_PING => "ping",
        COREWEBVIEW2_WEB_RESOURCE_CONTEXT_CSP_VIOLATION_REPORT => "csp_report",
        _ => "other",
    }
}
