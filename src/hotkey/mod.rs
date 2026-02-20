mod binding;
mod listener;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::error::HotkeyError;
use crate::types::HotkeyBinding;

pub use binding::parse_hotkey;

/// Events sent from the hotkey listener to the main loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HotkeyAction {
    /// The user pressed the hotkey (toggle recording).
    Toggle,
}

/// A running hotkey listener that can be polled for events.
pub struct HotkeyListener {
    rx: mpsc::Receiver<listener::ListenerEvent>,
    running: Arc<AtomicBool>,
}

impl HotkeyListener {
    /// Start listening for the configured hotkey in a background thread.
    ///
    /// # Errors
    /// Returns `HotkeyError` if the hotkey string is invalid or the listener
    /// cannot be started (e.g., no permission to read `/dev/input`).
    pub fn start(binding: &HotkeyBinding) -> Result<Self, HotkeyError> {
        let hotkey = parse_hotkey(binding.as_str()).map_err(HotkeyError::InvalidBinding)?;

        let (tx, rx) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));

        listener::start_listener(hotkey, tx, Arc::clone(&running))?;

        Ok(Self { rx, running })
    }

    /// Try to receive a hotkey event without blocking.
    #[must_use]
    pub fn try_recv(&self) -> Option<HotkeyAction> {
        match self.rx.try_recv() {
            Ok(listener::ListenerEvent::Pressed) => Some(HotkeyAction::Toggle),
            Ok(listener::ListenerEvent::Released) | Err(_) => None,
        }
    }

    /// Wait for a hotkey event with a timeout.
    #[must_use]
    pub fn recv_timeout(&self, timeout: Duration) -> Option<HotkeyAction> {
        match self.rx.recv_timeout(timeout) {
            Ok(listener::ListenerEvent::Pressed) => Some(HotkeyAction::Toggle),
            Ok(listener::ListenerEvent::Released) | Err(_) => None,
        }
    }

    /// Stop the hotkey listener.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the listener is still running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Validate that a hotkey binding string is parseable.
///
/// # Errors
/// Returns `HotkeyError::InvalidBinding` if the binding cannot be parsed.
pub fn validate_binding(binding: &str) -> Result<(), HotkeyError> {
    parse_hotkey(binding)
        .map(|_| ())
        .map_err(HotkeyError::InvalidBinding)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_good_bindings() {
        assert!(validate_binding("F8").is_ok());
        assert!(validate_binding("Shift+F8").is_ok());
        assert!(validate_binding("Ctrl+Alt+F1").is_ok());
        assert!(validate_binding("Ctrl+Shift+F9").is_ok());
        assert!(validate_binding("Super+F8").is_ok());
        assert!(validate_binding("Meta+F8").is_ok());
        assert!(validate_binding("Super+Shift+F1").is_ok());
        assert!(validate_binding("Win+Delete").is_ok());
        assert!(validate_binding("Super+I").is_ok());
        assert!(validate_binding("Ctrl+A").is_ok());
        assert!(validate_binding("Super+1").is_ok());
    }

    #[test]
    fn validate_bad_bindings() {
        assert!(validate_binding("").is_err());
        assert!(validate_binding("Unknown").is_err());
        assert!(validate_binding("Hyper+F8").is_err());
    }

    #[test]
    fn default_binding_is_valid() {
        let binding = HotkeyBinding::default();
        assert!(validate_binding(binding.as_str()).is_ok());
    }
}
