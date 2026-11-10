//! Profile-wide operations: data clearing, tracking prevention, extensions.

use crate::util::{get_bool, get_string};
use crate::{TrackingLevel, View};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    BrowserExtensionEnableCompletedHandler, BrowserExtensionRemoveCompletedHandler,
    ClearBrowsingDataCompletedHandler, ProfileAddBrowserExtensionCompletedHandler,
    ProfileGetBrowserExtensionsCompletedHandler,
};
use windows_core::{HSTRING, Interface};

/// Categories for "Clear browsing data".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BrowsingDataKinds {
    pub cookies: bool,
    pub cache: bool,
    /// Local storage, IndexedDB, Web SQL, Cache Storage, File Systems, service workers.
    pub site_storage: bool,
    pub autofill: bool,
    pub passwords: bool,
}

impl BrowsingDataKinds {
    fn raw(self) -> COREWEBVIEW2_BROWSING_DATA_KINDS {
        let mut k = 0;
        if self.cookies {
            k |= COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES.0;
        }
        if self.cache {
            k |= COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE.0;
        }
        if self.site_storage {
            k |= COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_DOM_STORAGE.0
                | COREWEBVIEW2_BROWSING_DATA_KINDS_CACHE_STORAGE.0
                | COREWEBVIEW2_BROWSING_DATA_KINDS_SERVICE_WORKERS.0;
        }
        if self.autofill {
            k |= COREWEBVIEW2_BROWSING_DATA_KINDS_GENERAL_AUTOFILL.0;
        }
        if self.passwords {
            k |= COREWEBVIEW2_BROWSING_DATA_KINDS_PASSWORD_AUTOSAVE.0;
        }
        COREWEBVIEW2_BROWSING_DATA_KINDS(k)
    }

    pub fn is_empty(self) -> bool {
        self.raw().0 == 0
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

fn profile(view: &View) -> windows_core::Result<ICoreWebView2Profile> {
    let wv13: ICoreWebView2_13 = view.core().cast()?;
    // SAFETY: getter on the UI thread.
    unsafe { wv13.Profile() }
}

impl View {
    /// Clears engine-held data for this view's profile. `since_ms` is a Unix timestamp in
    /// milliseconds, or `None` for all time.
    pub fn clear_browsing_data(&self, kinds: BrowsingDataKinds, since_ms: Option<i64>, done: impl FnOnce(bool) + 'static) {
        if kinds.is_empty() {
            done(true);
            return;
        }
        let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |hr| {
            done(hr.is_ok());
            Ok(())
        }));
        let r = (|| -> windows_core::Result<()> {
            let p2: ICoreWebView2Profile2 = profile(self)?.cast()?;
            // SAFETY: COM call on the UI thread.
            unsafe {
                match since_ms {
                    Some(ms) => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs_f64())
                            .unwrap_or(0.0);
                        p2.ClearBrowsingDataInTimeRange(kinds.raw(), ms as f64 / 1000.0, now + 60.0, &handler)
                    }
                    None => p2.ClearBrowsingData(kinds.raw(), &handler),
                }
            }
        })();
        if let Err(e) = r {
            tracing::error!(error = %e, "ClearBrowsingData failed");
        }
    }

    pub fn set_tracking_prevention(&self, level: TrackingLevel) {
        let r = (|| -> windows_core::Result<()> {
            let p3: ICoreWebView2Profile3 = profile(self)?.cast()?;
            // SAFETY: COM call on the UI thread.
            unsafe { p3.SetPreferredTrackingPreventionLevel(level.raw()) }
        })();
        if let Err(e) = r {
            tracing::warn!(error = %e, "tracking prevention not supported by runtime");
        }
    }

    /// Engine-side password saving and form autofill.
    pub fn set_autofill(&self, passwords: bool, general: bool) {
        let r = (|| -> windows_core::Result<()> {
            let p6: ICoreWebView2Profile6 = profile(self)?.cast()?;
            // SAFETY: COM calls on the UI thread.
            unsafe {
                p6.SetIsPasswordAutosaveEnabled(passwords)?;
                p6.SetIsGeneralAutofillEnabled(general)
            }
        })();
        if let Err(e) = r {
            tracing::warn!(error = %e, "autofill settings not supported by runtime");
        }
    }

    /// `Some(true)` dark, `Some(false)` light, `None` follow the system.
    pub fn set_color_scheme(&self, dark: Option<bool>) {
        let scheme = match dark {
            Some(true) => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
            Some(false) => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
            None => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_AUTO,
        };
        if let Ok(p) = profile(self) {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = p.SetPreferredColorScheme(scheme);
            }
        }
    }

    pub fn list_extensions(&self, done: impl FnOnce(Vec<ExtensionInfo>) + 'static) {
        let handler = ProfileGetBrowserExtensionsCompletedHandler::create(Box::new(move |hr, list| {
            let mut out = Vec::new();
            if let (Ok(()), Some(list)) = (hr, list) {
                // SAFETY: iterating a live collection on the UI thread.
                unsafe {
                    let mut n = 0u32;
                    let _ = list.Count(&mut n);
                    for i in 0..n {
                        if let Ok(ext) = list.GetValueAtIndex(i) {
                            out.push(ext_info(&ext));
                        }
                    }
                }
            }
            done(out);
            Ok(())
        }));
        let r = (|| -> windows_core::Result<()> {
            let p7: ICoreWebView2Profile7 = profile(self)?.cast()?;
            // SAFETY: COM call on the UI thread.
            unsafe { p7.GetBrowserExtensions(&handler) }
        })();
        if let Err(e) = r {
            tracing::warn!(error = %e, "extensions not supported by runtime");
        }
    }

    /// Installs an unpacked extension (folder containing `manifest.json`).
    pub fn add_extension(&self, folder: &str, done: impl FnOnce(Result<ExtensionInfo, String>) + 'static) {
        let handler = ProfileAddBrowserExtensionCompletedHandler::create(Box::new(move |hr, ext| {
            done(match (hr, ext) {
                (Ok(()), Some(ext)) => Ok(ext_info(&ext)),
                (Err(e), _) => Err(e.message().to_string()),
                _ => Err("unknown error".into()),
            });
            Ok(())
        }));
        let r = (|| -> windows_core::Result<()> {
            let p7: ICoreWebView2Profile7 = profile(self)?.cast()?;
            // SAFETY: COM call on the UI thread.
            unsafe { p7.AddBrowserExtension(&HSTRING::from(folder), &handler) }
        })();
        if let Err(e) = r {
            tracing::warn!(error = %e, "AddBrowserExtension failed");
        }
    }

    pub fn remove_extension(&self, id: &str, done: impl FnOnce(bool) + 'static) {
        self.with_extension(id, move |ext| {
            let handler = BrowserExtensionRemoveCompletedHandler::create(Box::new(move |hr| {
                done(hr.is_ok());
                Ok(())
            }));
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = ext.Remove(&handler);
            }
        });
    }

    pub fn enable_extension(&self, id: &str, enabled: bool, done: impl FnOnce(bool) + 'static) {
        self.with_extension(id, move |ext| {
            let handler = BrowserExtensionEnableCompletedHandler::create(Box::new(move |hr| {
                done(hr.is_ok());
                Ok(())
            }));
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = ext.Enable(enabled, &handler);
            }
        });
    }

    fn with_extension(&self, id: &str, f: impl FnOnce(ICoreWebView2BrowserExtension) + 'static) {
        let id = id.to_owned();
        let handler = ProfileGetBrowserExtensionsCompletedHandler::create(Box::new(move |hr, list| {
            if let (Ok(()), Some(list)) = (hr, list) {
                // SAFETY: iterating a live collection on the UI thread.
                unsafe {
                    let mut n = 0u32;
                    let _ = list.Count(&mut n);
                    for i in 0..n {
                        if let Ok(ext) = list.GetValueAtIndex(i) {
                            if get_string(|p| ext.Id(p)) == id {
                                f(ext);
                                break;
                            }
                        }
                    }
                }
            }
            Ok(())
        }));
        if let Ok(p7) = profile(self).and_then(|p| p.cast::<ICoreWebView2Profile7>()) {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = p7.GetBrowserExtensions(&handler);
            }
        }
    }
}

fn ext_info(ext: &ICoreWebView2BrowserExtension) -> ExtensionInfo {
    // SAFETY: getters on a live extension object.
    unsafe {
        ExtensionInfo {
            id: get_string(|p| ext.Id(p)),
            name: get_string(|p| ext.Name(p)),
            enabled: get_bool(|b| ext.IsEnabled(b)),
        }
    }
}
