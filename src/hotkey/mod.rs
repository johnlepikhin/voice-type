use std::time::Duration;

use hotkey_listener::{parse_hotkey, HotkeyEvent, HotkeyListenerBuilder, HotkeyListenerHandle};

use crate::error::HotkeyError;
use crate::types::HotkeyBinding;

/// Events sent from the hotkey listener to the main loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HotkeyAction {
    /// The user pressed the hotkey (toggle recording).
    Toggle,
}

/// A running hotkey listener that can be polled for events.
pub struct HotkeyListener {
    handle: HotkeyListenerHandle,
}

impl HotkeyListener {
    /// Start listening for the configured hotkey in a background thread.
    ///
    /// # Errors
    /// Returns `HotkeyError` if the hotkey string is invalid or the listener
    /// cannot be started (e.g., no permission to read `/dev/input`).
    pub fn start(binding: &HotkeyBinding) -> Result<Self, HotkeyError> {
        let hotkey = parse_hotkey(binding.as_str())
            .map_err(|e| HotkeyError::InvalidBinding(e.to_string()))?;

        let handle = HotkeyListenerBuilder::new()
            .add_hotkey(hotkey)
            .build()
            .map_err(|e| HotkeyError::ListenerFailed(e.to_string()))?
            .start()
            .map_err(|e| HotkeyError::ListenerFailed(e.to_string()))?;

        Ok(Self { handle })
    }

    /// Try to receive a hotkey event without blocking.
    #[must_use]
    pub fn try_recv(&self) -> Option<HotkeyAction> {
        match self.handle.try_recv() {
            Ok(HotkeyEvent::Pressed(_)) => Some(HotkeyAction::Toggle),
            Ok(HotkeyEvent::Released(_)) | Err(_) => None,
        }
    }

    /// Wait for a hotkey event with a timeout.
    #[must_use]
    pub fn recv_timeout(&self, timeout: Duration) -> Option<HotkeyAction> {
        match self.handle.recv_timeout(timeout) {
            Ok(HotkeyEvent::Pressed(_)) => Some(HotkeyAction::Toggle),
            Ok(HotkeyEvent::Released(_)) | Err(_) => None,
        }
    }

    /// Stop the hotkey listener.
    pub fn stop(&self) {
        self.handle.stop();
    }

    /// Check if the listener is still running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.handle.is_running()
    }
}

/// Validate that a hotkey binding string is parseable.
///
/// # Errors
/// Returns `HotkeyError::InvalidBinding` if the binding cannot be parsed.
pub fn validate_binding(binding: &str) -> Result<(), HotkeyError> {
    parse_hotkey(binding)
        .map(|_| ())
        .map_err(|e| HotkeyError::InvalidBinding(e.to_string()))
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
    }

    #[test]
    fn validate_bad_bindings() {
        assert!(validate_binding("Super+V").is_err());
        assert!(validate_binding("Ctrl+A").is_err());
        assert!(validate_binding("").is_err());
        assert!(validate_binding("Unknown").is_err());
    }

    #[test]
    fn default_binding_is_valid() {
        let binding = HotkeyBinding::default();
        assert!(validate_binding(binding.as_str()).is_ok());
    }
}
