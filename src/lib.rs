#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// Log current process memory usage (`VmRSS`) from `/proc/self/status`.
///
/// Reads the resident set size and logs it at debug level.
/// Only works on Linux; silently does nothing if the file is unreadable.
pub fn log_memory_usage(label: &str) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return;
    };
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            tracing::debug!(label, vmrss = value.trim(), "Memory usage");
            return;
        }
    }
}

pub mod audio;
pub mod config;
pub mod error;
pub mod http;
pub mod postprocess;
pub mod provider;
pub mod template;
pub mod types;
