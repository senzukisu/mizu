//! Reading and writing cookies of a view's profile (used to keep session cookies when the
//! session is restored, the way Chrome does with "Continue where you left off").

use serde::{Deserialize, Serialize};
use webview2_com::GetCookiesCompletedHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows_core::{HSTRING, Interface};

use crate::util::{get_bool, get_string};
use crate::view::View;

/// A cookie as stored by Mizu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CookieData {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    /// 0 = None, 1 = Lax, 2 = Strict (COREWEBVIEW2_COOKIE_SAME_SITE_KIND).
    pub same_site: i32,
    pub session: bool,
}

fn manager(view: &View) -> Option<ICoreWebView2CookieManager> {
    let wv2: ICoreWebView2_2 = view.core().cast().ok()?;
    // SAFETY: getter on the UI thread.
    unsafe { wv2.CookieManager().ok() }
}

fn to_data(c: &ICoreWebView2Cookie) -> CookieData {
    let mut same = COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX;
    // SAFETY: COM getters on the UI thread.
    unsafe {
        let _ = c.SameSite(&mut same);
        CookieData {
            name: get_string(|p| c.Name(p)),
            value: get_string(|p| c.Value(p)),
            domain: get_string(|p| c.Domain(p)),
            path: get_string(|p| c.Path(p)),
            secure: get_bool(|b| c.IsSecure(b)),
            http_only: get_bool(|b| c.IsHttpOnly(b)),
            same_site: same.0,
            session: get_bool(|b| c.IsSession(b)),
        }
    }
}

/// Lists the cookies a request to `uri` would carry (every cookie of the profile when
/// `uri` is empty) and hands the COM objects to `done` on the UI thread.
fn get_cookies(view: &View, uri: &str, done: impl FnOnce(&ICoreWebView2CookieManager, Vec<ICoreWebView2Cookie>) + 'static) {
    let Some(m) = manager(view) else { return };
    let m2 = m.clone();
    let handler = GetCookiesCompletedHandler::create(Box::new(move |hr, list| {
        let mut out = Vec::new();
        if let (Ok(()), Some(list)) = (hr, list) {
            let mut n = 0u32;
            // SAFETY: COM getters on the UI thread.
            unsafe {
                let _ = list.Count(&mut n);
                out.extend((0..n).filter_map(|i| list.GetValueAtIndex(i).ok()));
            }
        }
        done(&m2, out);
        Ok(())
    }));
    // SAFETY: COM call on the UI thread.
    unsafe {
        let _ = m.GetCookies(&HSTRING::from(uri), &handler);
    }
}

impl View {
    /// All cookies of this view's profile. `done` runs on the UI thread.
    pub fn all_cookies(&self, done: impl FnOnce(Vec<CookieData>) + 'static) {
        self.cookies_for("", done);
    }

    /// Cookies a request to `uri` would carry (the site's own and its parent domain's).
    /// `done` runs on the UI thread; it is not called when the view has no cookie manager.
    pub fn cookies_for(&self, uri: &str, done: impl FnOnce(Vec<CookieData>) + 'static) {
        get_cookies(self, uri, move |_, list| done(list.iter().map(to_data).collect()));
    }

    /// Deletes every cookie a request to `uri` would carry; `done` gets how many.
    pub fn delete_cookies_for(&self, uri: &str, done: impl FnOnce(usize) + 'static) {
        get_cookies(self, uri, move |m, list| {
            for c in &list {
                // SAFETY: COM call on the UI thread with a cookie of the same manager.
                unsafe {
                    let _ = m.DeleteCookie(c);
                }
            }
            done(list.len());
        });
    }

    /// Adds cookies to this view's profile (synchronously, before pages load).
    pub fn add_cookies(&self, cookies: &[CookieData]) -> usize {
        let Some(m) = manager(self) else { return 0 };
        let mut added = 0;
        for c in cookies {
            // SAFETY: COM calls on the UI thread with valid strings.
            let ok = unsafe {
                m.CreateCookie(&HSTRING::from(&c.name), &HSTRING::from(&c.value), &HSTRING::from(&c.domain), &HSTRING::from(&c.path))
                    .and_then(|cookie| {
                        cookie.SetIsSecure(c.secure)?;
                        cookie.SetIsHttpOnly(c.http_only)?;
                        cookie.SetSameSite(COREWEBVIEW2_COOKIE_SAME_SITE_KIND(c.same_site))?;
                        m.AddOrUpdateCookie(&cookie)
                    })
                    .is_ok()
            };
            if ok {
                added += 1;
            }
        }
        added
    }
}
