//! fastmail-cli library crate. Exposes internal modules so integration
//! tests under tests/*.rs can import them.
pub mod caldav;
pub mod carddav;
pub mod commands;
pub mod config;
pub mod error;
pub mod jmap;
pub mod mcp;
pub mod models;
pub mod util;
