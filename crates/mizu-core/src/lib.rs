//! Shared primitives used by every Mizu crate.
//!
//! This crate is intentionally dependency-light and platform-independent so that
//! it can be unit-tested without a browser engine.

pub mod ids;
pub mod internal;
pub mod omnibox;
pub mod paths;

pub const APP_NAME: &str = "Mizu";
pub const VENDOR: &str = "Onibi Lab";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TAGLINE: &str = "A new way to browse";
