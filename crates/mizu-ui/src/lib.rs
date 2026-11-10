//! Browser UI surfaces and their protocol.
//!
//! Mizu draws its own interface (tab strip, toolbar, menus, internal pages) with HTML/CSS in
//! dedicated engine views that only ever load the embedded assets in this crate. Those views
//! talk to the browser process with JSON messages defined in [`protocol`]; every message is
//! deserialised into a closed Rust enum, so anything unexpected is rejected at the boundary.

pub mod assets;
pub mod protocol;

pub use protocol::{Anchor, Command, Envelope, Event};
