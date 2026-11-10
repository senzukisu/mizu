//! Mizu Scripts – a built-in userscript manager.
//!
//! Scripts use the de-facto standard `// ==UserScript==` metadata block shared by
//! Greasemonkey, Tampermonkey and Violentmonkey, so scripts from Greasy Fork or OpenUserJS
//! install unchanged as long as they only need the supported API subset (see [`inject`]).
//!
//! * [`meta`]     – metadata block parser.
//! * [`pattern`]  – `@match` patterns and `@include`/`@exclude` globs.
//! * [`store`]    – scripts and their `GM_setValue` storage on disk.
//! * [`inject`]   – builds the document-start bootstrap with the `GM_*` API.

pub mod inject;
pub mod meta;
pub mod pattern;
pub mod store;

pub use inject::{GmMessage, build_injection};
pub use meta::{RunAt, ScriptMeta};
pub use store::{ScriptStore, UserScript};

/// Whether a URL points to an installable userscript.
pub fn is_userscript_url(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    (url.starts_with("https://") || url.starts_with("http://") || url.starts_with("file://"))
        && path.to_ascii_lowercase().ends_with(".user.js")
}

#[cfg(test)]
mod tests {
    #[test]
    fn detects_userscript_urls() {
        assert!(super::is_userscript_url("https://greasyfork.org/scripts/1/code/x.user.js"));
        assert!(super::is_userscript_url("https://x/y.USER.JS?v=2"));
        assert!(!super::is_userscript_url("https://x/y.js"));
        assert!(!super::is_userscript_url("javascript:x.user.js"));
    }
}
