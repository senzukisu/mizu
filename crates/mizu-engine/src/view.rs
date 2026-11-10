//! A single web view (one tab, or one browser-UI surface).

use crate::download::Download;
use crate::environment::{Engine, ProfileKind};
use crate::events::*;
use crate::util::{get_bool, get_string, read_stream};
use crate::Result;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::*;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
use windows_core::{HSTRING, Interface};

#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub key: ViewKey,
    pub profile: ProfileKind,
    /// Browser UI surface (toolbar, menus): no context menu, zoom, swipe or engine shortcuts,
    /// and CSS `app-region: drag` support for the custom title bar.
    pub chrome: bool,
    pub devtools: bool,
    pub smartscreen: bool,
    /// RGBA background shown before the first paint (avoids white flashes in dark mode).
    pub background: [u8; 4],
    /// Route every request through [`ViewHandler::on_resource_request`] (content blocking).
    pub intercept_all: bool,
    pub internal_scheme: String,
}

struct Inner {
    engine: Rc<Engine>,
    controller: ICoreWebView2Controller,
    core: ICoreWebView2,
    key: ViewKey,
    doc_script: RefCell<Option<String>>,
    /// Bumped on every `set_document_script` so late completions can be discarded.
    doc_script_gen: Cell<u64>,
    closed: Cell<bool>,
}

/// Handle to a web view. Cheap to clone; all clones refer to the same view.
#[derive(Clone)]
pub struct View {
    inner: Rc<Inner>,
}

fn key_down(vk: u16) -> bool {
    // SAFETY: GetKeyState has no preconditions.
    unsafe { GetKeyState(vk as i32) < 0 }
}

impl View {
    /// Creates a view inside `parent`. `done` runs on the UI thread once it is ready.
    pub fn create(
        engine: &Rc<Engine>,
        parent: HWND,
        config: ViewConfig,
        handler: Rc<dyn ViewHandler>,
        done: impl FnOnce(Result<View>) + 'static,
    ) {
        let engine_rc = engine.clone();
        engine.create_controller(parent, config.profile, move |result| {
            let view = result.and_then(|controller| {
                // SAFETY: getter on a freshly created controller.
                let core = unsafe { controller.CoreWebView2()? };
                let view = View {
                    inner: Rc::new(Inner {
                        engine: engine_rc,
                        controller,
                        core,
                        key: config.key,
                        doc_script: RefCell::new(None),
                        doc_script_gen: Cell::new(0),
                        closed: Cell::new(false),
                    }),
                };
                view.configure(&config)?;
                view.attach(handler, &config)?;
                Ok(view)
            });
            done(view);
        });
    }

    pub fn key(&self) -> ViewKey {
        self.inner.key
    }

    pub(crate) fn core(&self) -> &ICoreWebView2 {
        &self.inner.core
    }

    fn configure(&self, config: &ViewConfig) -> Result<()> {
        // SAFETY: settings calls on the UI thread on a live view.
        unsafe {
            let s = self.inner.core.Settings()?;
            s.SetIsWebMessageEnabled(true)?;
            s.SetAreDevToolsEnabled(config.devtools)?;
            // The browser UI suppresses its own menus in script, except in text fields.
            s.SetAreDefaultContextMenusEnabled(true)?;
            s.SetIsZoomControlEnabled(!config.chrome)?;
            s.SetIsStatusBarEnabled(!config.chrome)?;
            s.SetIsBuiltInErrorPageEnabled(true)?;
            s.SetAreHostObjectsAllowed(false)?;
            if let Ok(s3) = s.cast::<ICoreWebView2Settings3>() {
                s3.SetAreBrowserAcceleratorKeysEnabled(!config.chrome)?;
            }
            if let Ok(s6) = s.cast::<ICoreWebView2Settings6>() {
                s6.SetIsSwipeNavigationEnabled(!config.chrome)?;
            }
            if let Ok(s8) = s.cast::<ICoreWebView2Settings8>() {
                s8.SetIsReputationCheckingRequired(config.smartscreen)?;
            }
            if config.chrome {
                if let Ok(s9) = s.cast::<ICoreWebView2Settings9>() {
                    s9.SetIsNonClientRegionSupportEnabled(true)?;
                }
            }
            self.set_background(config.background);
        }
        Ok(())
    }

    pub fn set_background(&self, rgba: [u8; 4]) {
        if let Ok(c2) = self.inner.controller.cast::<ICoreWebView2Controller2>() {
            // SAFETY: setter on the UI thread.
            unsafe {
                let _ = c2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                    R: rgba[0],
                    G: rgba[1],
                    B: rgba[2],
                    A: rgba[3],
                });
            }
        }
    }

    /// Updates settings that can change at runtime.
    pub fn apply_live_settings(&self, devtools: bool, smartscreen: bool) {
        // SAFETY: settings calls on the UI thread.
        unsafe {
            if let Ok(s) = self.inner.core.Settings() {
                let _ = s.SetAreDevToolsEnabled(devtools);
                if let Ok(s8) = s.cast::<ICoreWebView2Settings8>() {
                    let _ = s8.SetIsReputationCheckingRequired(smartscreen);
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn attach(&self, handler: Rc<dyn ViewHandler>, config: &ViewConfig) -> Result<()> {
        let core = &self.inner.core;
        let key = self.inner.key;
        let mut token = 0i64;
        let weak = Rc::downgrade(&self.inner);

        // SAFETY: all registrations happen on the UI thread on live COM objects; callbacks are
        // invoked by WebView2 on the same thread.
        unsafe {
            // ── Navigation ───────────────────────────────────────────────
            let h = handler.clone();
            core.add_NavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let mut id = 0u64;
                    let _ = args.NavigationId(&mut id);
                    let req = NavigationRequest {
                        uri: get_string(|p| args.Uri(p)),
                        navigation_id: id,
                        user_initiated: get_bool(|b| args.IsUserInitiated(b)),
                        redirect: get_bool(|b| args.IsRedirected(b)),
                        history_navigation: args
                            .cast::<ICoreWebView2NavigationStartingEventArgs3>()
                            .map(|a3| {
                                let mut kind = COREWEBVIEW2_NAVIGATION_KIND_NEW_DOCUMENT;
                                let _ = a3.NavigationKind(&mut kind);
                                kind != COREWEBVIEW2_NAVIGATION_KIND_NEW_DOCUMENT
                            })
                            .unwrap_or(false),
                    };
                    if h.on_navigation_starting(key, &req) == NavigationDecision::Cancel {
                        args.SetCancel(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_FrameNavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let uri = get_string(|p| args.Uri(p));
                    if h.on_frame_navigation_starting(key, &uri) == NavigationDecision::Cancel {
                        args.SetCancel(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_SourceChanged(
                &SourceChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(s) = sender {
                        h.on_event(key, ViewEvent::SourceChanged { uri: get_string(|p| s.Source(p)) });
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_DocumentTitleChanged(
                &DocumentTitleChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(s) = sender {
                        h.on_event(key, ViewEvent::TitleChanged { title: get_string(|p| s.DocumentTitle(p)) });
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_HistoryChanged(
                &HistoryChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(s) = sender {
                        h.on_event(
                            key,
                            ViewEvent::HistoryChanged {
                                can_go_back: get_bool(|b| s.CanGoBack(b)),
                                can_go_forward: get_bool(|b| s.CanGoForward(b)),
                            },
                        );
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_ContentLoading(
                &ContentLoadingEventHandler::create(Box::new(move |_, _| {
                    h.on_event(key, ViewEvent::ContentLoading);
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_NavigationCompleted(
                &NavigationCompletedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let mut web_error = COREWEBVIEW2_WEB_ERROR_STATUS_UNKNOWN;
                    let _ = args.WebErrorStatus(&mut web_error);
                    let mut http_status = 0;
                    if let Ok(a2) = args.cast::<ICoreWebView2NavigationCompletedEventArgs2>() {
                        let _ = a2.HttpStatusCode(&mut http_status);
                    }
                    h.on_event(
                        key,
                        ViewEvent::NavigationCompleted {
                            success: get_bool(|b| args.IsSuccess(b)),
                            http_status,
                            web_error: web_error.0,
                        },
                    );
                    Ok(())
                })),
                &mut token,
            )?;

            if let Ok(wv2) = core.cast::<ICoreWebView2_2>() {
                let h = handler.clone();
                wv2.add_DOMContentLoaded(
                    &DOMContentLoadedEventHandler::create(Box::new(move |_, _| {
                        h.on_event(key, ViewEvent::DomContentLoaded);
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // ── Requests ─────────────────────────────────────────────────
            let scheme_filter = HSTRING::from(format!("{}:*", config.internal_scheme));
            let all = HSTRING::from("*");
            let filters: &[&HSTRING] = if config.intercept_all { &[&all] } else { &[&scheme_filter] };
            for filter in filters {
                let added = core
                    .cast::<ICoreWebView2_22>()
                    .and_then(|wv22| {
                        wv22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                            *filter,
                            COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                            COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                        )
                    })
                    .is_ok();
                if !added {
                    core.AddWebResourceRequestedFilter(*filter, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?;
                }
            }
            if config.intercept_all {
                // `*` does not cover custom schemes; register ours explicitly as well.
                core.AddWebResourceRequestedFilter(&scheme_filter, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?;
            }

            let h = handler.clone();
            let w = weak.clone();
            core.add_WebResourceRequested(
                &WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                    let (Some(args), Some(inner)) = (args, w.upgrade()) else { return Ok(()) };
                    let request = args.Request()?;
                    let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_OTHER;
                    let _ = args.ResourceContext(&mut ctx);
                    let req = ResourceRequest {
                        uri: get_string(|p| request.Uri(p)),
                        method: get_string(|p| request.Method(p)),
                        resource_type: resource_type(ctx),
                    };
                    match h.on_resource_request(key, &req) {
                        ResourceDecision::Continue => {}
                        ResourceDecision::ContinueWithHeaders(headers) => {
                            let hs = request.Headers()?;
                            for (k, v) in headers {
                                let _ = hs.SetHeader(&HSTRING::from(k), &HSTRING::from(v));
                            }
                        }
                        ResourceDecision::Rewrite(uri) => {
                            request.SetUri(&HSTRING::from(uri))?;
                        }
                        ResourceDecision::Respond { status, mime, body, extra_headers } => {
                            let reason = if status == 200 { "OK" } else { "Error" };
                            let headers = format!("Content-Type: {mime}\r\n{extra_headers}");
                            let resp = inner.engine.response(status, reason, &headers, Some(&body))?;
                            args.SetResponse(&resp)?;
                        }
                        ResourceDecision::Block => {
                            let resp = inner.engine.response(403, "Blocked by Mizu Shield", "", None)?;
                            args.SetResponse(&resp)?;
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            // ── Pop-ups, permissions, downloads ─────────────────────────
            let h = handler.clone();
            core.add_NewWindowRequested(
                &NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let deferral = args.GetDeferral()?;
                    let req = NewWindowRequest::new(
                        get_string(|p| args.Uri(p)),
                        get_bool(|b| args.IsUserInitiated(b)),
                        args,
                        deferral,
                    );
                    h.on_new_window(key, req);
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_PermissionRequested(
                &PermissionRequestedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    // Mizu stores decisions itself, so the engine must not remember them.
                    if let Ok(a3) = args.cast::<ICoreWebView2PermissionRequestedEventArgs3>() {
                        let _ = a3.SetSavesInProfile(false);
                    }
                    let mut kind = COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION;
                    let _ = args.PermissionKind(&mut kind);
                    let deferral = args.GetDeferral()?;
                    let req = PermissionRequest::new(
                        get_string(|p| args.Uri(p)),
                        PermissionKind::from_raw(kind),
                        get_bool(|b| args.IsUserInitiated(b)),
                        args,
                        deferral,
                    );
                    h.on_permission(key, req);
                    Ok(())
                })),
                &mut token,
            )?;

            if let Ok(wv4) = core.cast::<ICoreWebView2_4>() {
                let h = handler.clone();
                wv4.add_DownloadStarting(
                    &DownloadStartingEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let op = args.DownloadOperation()?;
                        let deferral = args.GetDeferral()?;
                        h.on_download(key, Download::new(op, args, deferral));
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // ── Messages & misc state ───────────────────────────────────
            let h = handler.clone();
            core.add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let source = get_string(|p| args.Source(p));
                    let json = get_string(|p| args.WebMessageAsJson(p));
                    h.on_web_message(key, &source, &json);
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_ContainsFullScreenElementChanged(
                &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(s) = sender {
                        let fullscreen = get_bool(|b| s.ContainsFullScreenElement(b));
                        h.on_event(key, ViewEvent::FullscreenChanged { fullscreen });
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_WindowCloseRequested(
                &WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
                    h.on_event(key, ViewEvent::CloseRequested);
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            core.add_ProcessFailed(
                &ProcessFailedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND_UNKNOWN_PROCESS_EXITED;
                    let _ = args.ProcessFailedKind(&mut kind);
                    let unresponsive = kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE;
                    let crashed_page = matches!(
                        kind,
                        COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
                            | COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE
                            | COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED
                    );
                    if crashed_page {
                        h.on_event(
                            key,
                            ViewEvent::ProcessFailed {
                                unresponsive,
                                description: format!("process failure kind {}", kind.0),
                            },
                        );
                    } else {
                        tracing::warn!(kind = kind.0, "auxiliary engine process exited (recovered by engine)");
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            if let Ok(wv15) = core.cast::<ICoreWebView2_15>() {
                let h = handler.clone();
                wv15.add_FaviconChanged(
                    &FaviconChangedEventHandler::create(Box::new(move |sender, _| {
                        let Some(s) = sender.and_then(|s| s.cast::<ICoreWebView2_15>().ok()) else {
                            return Ok(());
                        };
                        let h = h.clone();
                        let _ = s.GetFavicon(
                            COREWEBVIEW2_FAVICON_IMAGE_FORMAT_PNG,
                            &GetFaviconCompletedHandler::create(Box::new(move |hr, stream| {
                                let png = match (hr, stream) {
                                    (Ok(()), Some(stream)) => read_stream(&stream, 256 * 1024),
                                    _ => Vec::new(),
                                };
                                h.on_event(key, ViewEvent::FaviconChanged { png });
                                Ok(())
                            })),
                        );
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            if let Ok(wv8) = core.cast::<ICoreWebView2_8>() {
                let audio = {
                    let h = handler.clone();
                    move |s: Option<ICoreWebView2>| {
                        if let Some(s) = s.and_then(|s| s.cast::<ICoreWebView2_8>().ok()) {
                            h.on_event(
                                key,
                                ViewEvent::AudioChanged {
                                    playing: get_bool(|b| s.IsDocumentPlayingAudio(b)),
                                    muted: get_bool(|b| s.IsMuted(b)),
                                },
                            );
                        }
                    }
                };
                let a2 = audio.clone();
                wv8.add_IsDocumentPlayingAudioChanged(
                    &IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |s, _| {
                        audio(s);
                        Ok(())
                    })),
                    &mut token,
                )?;
                wv8.add_IsMutedChanged(
                    &IsMutedChangedEventHandler::create(Box::new(move |s, _| {
                        a2(s);
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            if let Ok(wv12) = core.cast::<ICoreWebView2_12>() {
                let h = handler.clone();
                wv12.add_StatusBarTextChanged(
                    &StatusBarTextChangedEventHandler::create(Box::new(move |sender, _| {
                        if let Some(s) = sender.and_then(|s| s.cast::<ICoreWebView2_12>().ok()) {
                            h.on_event(key, ViewEvent::StatusText { text: get_string(|p| s.StatusBarText(p)) });
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            if let Ok(wv18) = core.cast::<ICoreWebView2_18>() {
                let h = handler.clone();
                wv18.add_LaunchingExternalUriScheme(
                    &LaunchingExternalUriSchemeEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let uri = get_string(|p| args.Uri(p));
                        if !h.on_external_uri(key, &uri) {
                            args.SetCancel(true)?;
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // ── Controller events ───────────────────────────────────────
            let controller = &self.inner.controller;
            let h = handler.clone();
            controller.add_AcceleratorKeyPressed(
                &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    let mut kind = COREWEBVIEW2_KEY_EVENT_KIND_KEY_UP;
                    args.KeyEventKind(&mut kind)?;
                    if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN {
                        return Ok(());
                    }
                    let mut vk = 0u32;
                    args.VirtualKey(&mut vk)?;
                    let handled = h.on_accelerator(
                        key,
                        vk,
                        key_down(VK_CONTROL.0),
                        key_down(VK_SHIFT.0),
                        key_down(VK_MENU.0),
                    );
                    if handled {
                        args.SetHandled(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler.clone();
            controller.add_ZoomFactorChanged(
                &ZoomFactorChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(c) = sender {
                        let mut f = 1.0;
                        let _ = c.ZoomFactor(&mut f);
                        h.on_event(key, ViewEvent::ZoomChanged { factor: f });
                    }
                    Ok(())
                })),
                &mut token,
            )?;

            let h = handler;
            controller.add_GotFocus(
                &FocusChangedEventHandler::create(Box::new(move |_, _| {
                    h.on_event(key, ViewEvent::GotFocus);
                    Ok(())
                })),
                &mut token,
            )?;
        }
        Ok(())
    }

    // ───────────────────────── Commands ─────────────────────────

    pub fn navigate(&self, uri: &str) {
        // SAFETY: COM call on the UI thread.
        if let Err(e) = unsafe { self.inner.core.Navigate(&HSTRING::from(uri)) } {
            tracing::warn!(uri, error = %e, "navigate failed");
        }
    }

    pub fn navigate_to_string(&self, html: &str) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.NavigateToString(&HSTRING::from(html));
        }
    }

    pub fn reload(&self, bypass_cache: bool) {
        // SAFETY: COM calls on the UI thread.
        unsafe {
            if bypass_cache {
                let _ = self.inner.core.CallDevToolsProtocolMethod(
                    &HSTRING::from("Page.reload"),
                    &HSTRING::from(r#"{"ignoreCache":true}"#),
                    &CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_, _| Ok(()))),
                );
            } else {
                let _ = self.inner.core.Reload();
            }
        }
    }

    pub fn stop(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.Stop();
        }
    }

    pub fn go_back(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.GoBack();
        }
    }

    pub fn go_forward(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.GoForward();
        }
    }

    pub fn source(&self) -> String {
        // SAFETY: getter on the UI thread.
        get_string(|p| unsafe { self.inner.core.Source(p) })
    }

    pub fn title(&self) -> String {
        // SAFETY: getter on the UI thread.
        get_string(|p| unsafe { self.inner.core.DocumentTitle(p) })
    }

    pub fn can_go_back(&self) -> bool {
        // SAFETY: getter on the UI thread.
        get_bool(|b| unsafe { self.inner.core.CanGoBack(b) })
    }

    pub fn can_go_forward(&self) -> bool {
        // SAFETY: getter on the UI thread.
        get_bool(|b| unsafe { self.inner.core.CanGoForward(b) })
    }

    pub fn set_bounds(&self, rect: RECT) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.controller.SetBounds(rect);
        }
    }

    pub fn set_visible(&self, visible: bool) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.controller.SetIsVisible(visible);
        }
    }

    pub fn focus(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
        }
    }

    /// Must be called when the host window moves so pop-ups (select boxes) stay aligned.
    pub fn notify_parent_moved(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.controller.NotifyParentWindowPositionChanged();
        }
    }

    pub fn post_json(&self, json: &str) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.PostWebMessageAsJson(&HSTRING::from(json));
        }
    }

    pub fn execute_script(&self, js: &str) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.ExecuteScript(
                &HSTRING::from(js),
                &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
            );
        }
    }

    /// Script that runs at creation of every following document (replaces the previous one).
    /// Used for per-navigation cosmetic filtering, scriptlets and userscripts.
    pub fn set_document_script(&self, js: Option<String>) {
        let generation = self.inner.doc_script_gen.get() + 1;
        self.inner.doc_script_gen.set(generation);
        if let Some(old) = self.inner.doc_script.borrow_mut().take() {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = self.inner.core.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(old));
            }
        }
        let Some(js) = js else { return };
        let weak = Rc::downgrade(&self.inner);
        let handler = AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(move |hr, id| {
            if let (Ok(()), Some(inner)) = (hr, weak.upgrade()) {
                if inner.doc_script_gen.get() == generation {
                    *inner.doc_script.borrow_mut() = Some(id);
                } else {
                    // Superseded while registering: remove immediately.
                    // SAFETY: COM call on the UI thread.
                    unsafe {
                        let _ = inner.core.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id));
                    }
                }
            }
            Ok(())
        }));
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.AddScriptToExecuteOnDocumentCreated(&HSTRING::from(js), &handler);
        }
    }

    pub fn zoom(&self) -> f64 {
        let mut f = 1.0;
        // SAFETY: getter on the UI thread.
        unsafe {
            let _ = self.inner.controller.ZoomFactor(&mut f);
        }
        f
    }

    pub fn set_zoom(&self, factor: f64) {
        // SAFETY: setter on the UI thread.
        unsafe {
            let _ = self.inner.controller.SetZoomFactor(factor.clamp(0.25, 5.0));
        }
    }

    pub fn set_muted(&self, muted: bool) {
        if let Ok(wv8) = self.inner.core.cast::<ICoreWebView2_8>() {
            // SAFETY: setter on the UI thread.
            unsafe {
                let _ = wv8.SetIsMuted(muted);
            }
        }
    }

    /// Suspends script timers and rendering of a hidden view to save CPU and memory.
    pub fn suspend(&self) {
        if let Ok(wv3) = self.inner.core.cast::<ICoreWebView2_3>() {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = wv3.TrySuspend(&TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(()))));
            }
        }
    }

    pub fn resume(&self) {
        if let Ok(wv3) = self.inner.core.cast::<ICoreWebView2_3>() {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = wv3.Resume();
            }
        }
    }

    pub fn is_suspended(&self) -> bool {
        match self.inner.core.cast::<ICoreWebView2_3>() {
            // SAFETY: getter on the UI thread.
            Ok(wv3) => get_bool(|b| unsafe { wv3.IsSuspended(b) }),
            Err(_) => false,
        }
    }

    /// Hint the engine to trim memory of background views.
    pub fn set_low_memory(&self, low: bool) {
        if let Ok(wv19) = self.inner.core.cast::<ICoreWebView2_19>() {
            let level = if low {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            };
            // SAFETY: setter on the UI thread.
            unsafe {
                let _ = wv19.SetMemoryUsageTargetLevel(level);
            }
        }
    }

    pub fn print(&self) {
        if let Ok(wv16) = self.inner.core.cast::<ICoreWebView2_16>() {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = wv16.ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_BROWSER);
            }
        }
    }

    pub fn open_devtools(&self) {
        // SAFETY: COM call on the UI thread.
        unsafe {
            let _ = self.inner.core.OpenDevToolsWindow();
        }
    }

    pub fn open_task_manager(&self) {
        if let Ok(wv6) = self.inner.core.cast::<ICoreWebView2_6>() {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = wv6.OpenTaskManagerWindow();
            }
        }
    }

    pub fn close(&self) {
        if self.inner.closed.replace(true) {
            return;
        }
        // SAFETY: COM call on the UI thread; the view is not used afterwards.
        unsafe {
            let _ = self.inner.controller.Close();
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.get()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if !self.closed.get() {
            // SAFETY: final release on the UI thread.
            unsafe {
                let _ = self.controller.Close();
            }
        }
    }
}
