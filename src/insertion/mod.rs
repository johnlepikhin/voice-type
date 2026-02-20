use std::io::Write;
use std::process::Command;

use crate::error::TextInsertionError;

/// Display session type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    X11,
    Wayland,
}

/// Detect the current display session type.
///
/// # Errors
/// Returns `TextInsertionError::UnsupportedSessionType` if the session
/// type cannot be determined.
pub fn detect_session_type() -> Result<SessionType, TextInsertionError> {
    match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("x11") => Ok(SessionType::X11),
        Ok("wayland") => Ok(SessionType::Wayland),
        _ => Err(TextInsertionError::UnsupportedSessionType),
    }
}

/// Insert text into the previously focused window by:
/// 1. Copying text to clipboard
/// 2. Simulating Ctrl+V paste
///
/// # Errors
/// Returns `TextInsertionError` if clipboard or paste tools are unavailable.
pub fn insert_text(text: &str) -> Result<(), TextInsertionError> {
    let session = detect_session_type()?;
    copy_to_clipboard(text, session)?;

    // Small delay to let clipboard settle
    std::thread::sleep(std::time::Duration::from_millis(50));

    simulate_paste(session)?;
    Ok(())
}

/// Copy text to the system clipboard.
fn copy_to_clipboard(text: &str, session: SessionType) -> Result<(), TextInsertionError> {
    let mut cmd = match session {
        SessionType::X11 => {
            let mut c = Command::new("xclip");
            c.args(["-selection", "clipboard"]);
            c
        }
        SessionType::Wayland => Command::new("wl-copy"),
    };

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| TextInsertionError::ClipboardUnavailable)?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|_| TextInsertionError::ClipboardUnavailable)?;
        // stdin is dropped here, closing the pipe
    }

    let status = child
        .wait()
        .map_err(|_| TextInsertionError::ClipboardUnavailable)?;

    if !status.success() {
        return Err(TextInsertionError::ClipboardUnavailable);
    }

    Ok(())
}

/// Simulate Ctrl+V paste keystroke.
fn simulate_paste(session: SessionType) -> Result<(), TextInsertionError> {
    let status = match session {
        SessionType::X11 => Command::new("xdotool")
            .args(["key", "ctrl+v"])
            .status()
            .map_err(|_| TextInsertionError::PasteSimulationFailed)?,
        SessionType::Wayland => Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v"])
            .status()
            .map_err(|_| TextInsertionError::PasteSimulationFailed)?,
    };

    if !status.success() {
        return Err(TextInsertionError::PasteSimulationFailed);
    }

    Ok(())
}

/// Read the current clipboard content (for save/restore).
///
/// Returns `None` if reading fails (best-effort).
#[must_use]
pub fn read_clipboard() -> Option<String> {
    let session = detect_session_type().ok()?;
    let output = match session {
        SessionType::X11 => Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .ok()?,
        SessionType::Wayland => Command::new("wl-paste")
            .args(["--no-newline"])
            .output()
            .ok()?,
    };

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_session_type_from_env() {
        // This test depends on the environment
        let result = detect_session_type();
        // Either succeeds with x11/wayland or fails with UnsupportedSessionType
        match result {
            Ok(SessionType::X11 | SessionType::Wayland)
            | Err(TextInsertionError::UnsupportedSessionType) => {}
            _ => panic!("Unexpected result"),
        }
    }
}
