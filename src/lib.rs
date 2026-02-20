#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::time::Duration;

/// Maximum idle age for pooled HTTP connections.
///
/// Prevents reuse of stale TCP connections that may have been closed
/// by upstream proxies (e.g., Cloudflare) before ureq's default idle timeout.
pub const HTTP_IDLE_TIMEOUT: Duration = Duration::from_secs(5);

pub mod audio;
pub mod config;
pub mod error;
pub mod hotkey;
pub mod insertion;
pub mod postprocess;
pub mod provider;
pub mod types;
