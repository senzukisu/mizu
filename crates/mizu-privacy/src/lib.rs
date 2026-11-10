//! Mizu Shield – privacy protection that actually blocks things.
//!
//! * [`blocker`] – network and cosmetic filtering with Brave's `adblock-rust` engine, which
//!   implements the uBlock Origin / Adblock Plus filter syntax (including `$redirect`,
//!   `$removeparam`, scriptlet injection `##+js(...)` and element hiding).
//! * [`lists`] – downloading, caching and refreshing filter lists (uBlock filters, EasyList,
//!   EasyPrivacy, …) plus scriptlet/redirect resources.
//! * [`permissions`] – the catalogue of site permissions and their safe defaults.
//! * [`stats`] – counters behind the Shield panel.

pub mod blocker;
pub mod lists;
pub mod permissions;
pub mod resources;
pub mod stats;

pub use blocker::{ContentBlocker, CosmeticBundle, Verdict};
pub use lists::{FilterStore, ListStatus, fetch_https_bytes, fetch_https_text};
pub use permissions::{PermissionDefault, PermissionKind};
pub use stats::BlockStats;
