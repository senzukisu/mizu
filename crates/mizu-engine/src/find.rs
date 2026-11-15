//! Find in page (Ctrl+F) through the engine's Find API, with Mizu drawing its own bar:
//! the engine's built-in find dialog is suppressed, it only highlights and scrolls.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{FindActiveMatchIndexChangedEventHandler, FindMatchCountChangedEventHandler, FindStartCompletedHandler};
use windows_core::{HSTRING, Interface};

use crate::ViewKey;
use crate::view::View;

/// Receives (active match, 1-based or 0 for none; total matches).
type Report = Box<dyn Fn(i32, i32)>;

thread_local! {
    static REPORTS: RefCell<HashMap<ViewKey, Report>> = RefCell::new(HashMap::new());
    static HOOKED: RefCell<HashSet<ViewKey>> = RefCell::new(HashSet::new());
}

fn find_of(view: &View) -> Option<ICoreWebView2Find> {
    let v: ICoreWebView2_28 = view.core().cast().ok()?;
    // SAFETY: getter on the UI thread.
    unsafe { v.Find().ok() }
}

fn state(find: &ICoreWebView2Find) -> (i32, i32) {
    let (mut active, mut count) = (-1, 0);
    // SAFETY: getters on the UI thread.
    unsafe {
        let _ = find.ActiveMatchIndex(&mut active);
        let _ = find.MatchCount(&mut count);
    }
    // The engine counts from 0 and uses -1 for "none".
    (if active >= 0 { active + 1 } else { 0 }, count.max(0))
}

fn report(key: ViewKey, find: &ICoreWebView2Find) {
    let (active, count) = state(find);
    REPORTS.with(|r| {
        if let Some(f) = r.borrow().get(&key) {
            f(active, count);
        }
    });
}

impl View {
    /// Whether this engine has the Find API (WebView2 Runtime 120+).
    pub fn can_find(&self) -> bool {
        find_of(self).is_some()
    }

    /// Starts a search, highlighting every match and scrolling to the first one. `on_result`
    /// is called on the UI thread whenever the active match or the count changes.
    pub fn find_start(&self, term: &str, case_sensitive: bool, on_result: impl Fn(i32, i32) + 'static) {
        let Some(find) = find_of(self) else { return };
        let key = self.key();
        REPORTS.with(|r| r.borrow_mut().insert(key, Box::new(on_result)));
        if HOOKED.with(|h| h.borrow_mut().insert(key)) {
            let mut token = 0i64;
            // SAFETY: registering handlers on the UI thread; they live as long as the view.
            unsafe {
                let _ = find.add_MatchCountChanged(
                    &FindMatchCountChangedEventHandler::create(Box::new(move |f, _| {
                        if let Some(f) = f {
                            report(key, &f);
                        }
                        Ok(())
                    })),
                    &mut token,
                );
                let _ = find.add_ActiveMatchIndexChanged(
                    &FindActiveMatchIndexChangedEventHandler::create(Box::new(move |f, _| {
                        if let Some(f) = f {
                            report(key, &f);
                        }
                        Ok(())
                    })),
                    &mut token,
                );
            }
        }
        let Some(env) = self.environment15() else { return };
        // SAFETY: COM calls on the UI thread.
        unsafe {
            let Ok(options) = env.CreateFindOptions() else { return };
            let _ = options.SetFindTerm(&HSTRING::from(term));
            let _ = options.SetIsCaseSensitive(case_sensitive);
            let _ = options.SetShouldHighlightAllMatches(true);
            let _ = options.SetSuppressDefaultFindDialog(true);
            let f2 = find.clone();
            let _ = find.Start(
                &options,
                &FindStartCompletedHandler::create(Box::new(move |_| {
                    report(key, &f2);
                    Ok(())
                })),
            );
        }
    }

    pub fn find_next(&self, forward: bool) {
        if let Some(find) = find_of(self) {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = if forward { find.FindNext() } else { find.FindPrevious() };
            }
        }
    }

    /// Clears the highlights and forgets the result callback.
    pub fn find_stop(&self) {
        let key = self.key();
        REPORTS.with(|r| r.borrow_mut().remove(&key));
        if let Some(find) = find_of(self) {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = find.Stop();
            }
        }
    }
}
