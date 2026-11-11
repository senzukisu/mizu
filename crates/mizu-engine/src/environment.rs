use crate::util::get_string;
use crate::{EngineError, Result};
use std::path::PathBuf;
use std::sync::mpsc;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    CoreWebView2CustomSchemeRegistration, CoreWebView2EnvironmentOptions,
    CreateCoreWebView2CompositionControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::IStream;
use windows::Win32::UI::Shell::SHCreateMemStream;
use windows_core::{HSTRING, Interface, PCWSTR};

/// Engine-wide options fixed at startup (changing them requires a restart).
#[derive(Debug, Clone)]
pub struct EngineOptions {
    /// Engine user data folder: cookies, cache and site storage live here.
    pub user_data_dir: PathBuf,
    /// BCP-47 UI language for engine-provided UI (context menus, error pages).
    pub language: String,
    /// Chromium switches chosen by Mizu (never user-supplied strings).
    pub browser_args: Vec<String>,
    pub extensions: bool,
    /// Scheme served by the browser for internal pages (`mizu`).
    pub internal_scheme: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Normal,
    /// InPrivate: in-memory cookies/storage, isolated from the normal profile, discarded when
    /// the last private view closes.
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingLevel {
    None,
    Basic,
    Balanced,
    Strict,
}

impl TrackingLevel {
    pub(crate) fn raw(self) -> COREWEBVIEW2_TRACKING_PREVENTION_LEVEL {
        match self {
            Self::None => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_NONE,
            Self::Basic => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BASIC,
            Self::Balanced => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BALANCED,
            Self::Strict => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_STRICT,
        }
    }
}

/// The installed runtime version, or `None` if WebView2 is not installed.
pub fn browser_version() -> Option<String> {
    // SAFETY: plain FFI call with a null folder (use the installed evergreen runtime).
    let v = get_string(|p| unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), p) });
    (!v.is_empty()).then_some(v)
}

/// A WebView2 environment: one browser process tree shared by all views of a profile folder.
pub struct Engine {
    env: ICoreWebView2Environment,
}

impl Engine {
    /// Creates the environment. Blocks while pumping window messages (startup only).
    pub fn create(options: &EngineOptions) -> Result<Self> {
        if browser_version().is_none() {
            return Err(EngineError::RuntimeUnavailable(
                "Microsoft Edge WebView2 Runtime was not found".into(),
            ));
        }
        std::fs::create_dir_all(&options.user_data_dir)
            .map_err(|e| EngineError::RuntimeUnavailable(e.to_string()))?;

        let env_options = CoreWebView2EnvironmentOptions::default();
        // SAFETY: the options object is only accessed from this thread before being passed in.
        unsafe {
            env_options.set_additional_browser_arguments(options.browser_args.join(" "));
            env_options.set_language(options.language.clone());
            env_options.set_are_browser_extensions_enabled(options.extensions);
            env_options.set_enable_tracking_prevention(true);
            // Crash dumps stay on this machine instead of being uploaded.
            env_options.set_is_custom_crash_reporting_enabled(true);
            env_options.set_exclusive_user_data_folder_access(true);

            let scheme = CoreWebView2CustomSchemeRegistration::new(options.internal_scheme.clone());
            scheme.set_treat_as_secure(true);
            scheme.set_has_authority_component(true);
            // No web origin may fetch internal resources.
            scheme.set_allowed_origins(Vec::new());
            env_options.set_scheme_registrations(vec![Some(scheme.into())]);
        }
        let env_options: ICoreWebView2EnvironmentOptions = env_options.into();

        let data_dir = HSTRING::from(options.user_data_dir.as_os_str());
        let (tx, rx) = mpsc::channel();
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |hr, env| {
            let _ = tx.send(hr.and_then(|_| env.ok_or_else(|| windows_core::Error::from(windows::Win32::Foundation::E_POINTER))));
            Ok(())
        }));
        // SAFETY: valid arguments; completion is delivered on this thread via the message pump.
        unsafe {
            CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), &data_dir, &env_options, &handler)?;
        }
        let env = webview2_com::wait_with_pump(rx)??;
        Ok(Self { env })
    }

    /// Asynchronously creates a visually hosted (composition) controller for `parent`.
    pub(crate) fn create_controller(
        &self,
        parent: HWND,
        profile: ProfileKind,
        done: impl FnOnce(Result<ICoreWebView2CompositionController>) + 'static,
    ) {
        let handler = CreateCoreWebView2CompositionControllerCompletedHandler::create(Box::new(move |hr, controller| {
            done(hr.map_err(EngineError::from).and_then(|_| {
                controller.ok_or_else(|| EngineError::WebView2("no controller".into()))
            }));
            Ok(())
        }));
        let result = (|| -> windows_core::Result<()> {
            let env10: ICoreWebView2Environment10 = self.env.cast()?;
            // SAFETY: COM calls on the UI thread with valid arguments.
            unsafe {
                let opts = env10.CreateCoreWebView2ControllerOptions()?;
                let (name, private) = match profile {
                    ProfileKind::Normal => ("Default", false),
                    ProfileKind::Private => ("Private", true),
                };
                opts.SetProfileName(&HSTRING::from(name))?;
                opts.SetIsInPrivateModeEnabled(private)?;
                env10.CreateCoreWebView2CompositionControllerWithOptions(parent, &opts, &handler)
            }
        })();
        if let Err(e) = result {
            tracing::error!(error = %e, "CreateCoreWebView2Controller failed");
        }
    }

    /// Builds a response for intercepted requests (internal pages, blocked trackers).
    pub(crate) fn response(
        &self,
        status: i32,
        reason: &str,
        headers: &str,
        body: Option<&[u8]>,
    ) -> windows_core::Result<ICoreWebView2WebResourceResponse> {
        // SAFETY: SHCreateMemStream copies the buffer; arguments are valid for the call.
        unsafe {
            let stream: Option<IStream> = body.and_then(|b| SHCreateMemStream(Some(b)));
            self.env.CreateWebResourceResponse(
                stream.as_ref(),
                status,
                &HSTRING::from(reason),
                &HSTRING::from(headers),
            )
        }
    }

    pub fn version(&self) -> String {
        // SAFETY: simple getter.
        get_string(|p| unsafe { self.env.BrowserVersionString(p) })
    }
}
