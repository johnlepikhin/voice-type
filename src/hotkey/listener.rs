//! Background evdev listener for global hotkeys.
//!
//! Scans `/dev/input/event*` for keyboard devices, reads key events in a
//! background thread, tracks modifier state (including Super/Meta), and
//! sends press/release events over an `mpsc` channel.

use std::collections::HashSet;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use evdev::Device;
use nix::fcntl::{fcntl, FcntlArg, OFlag};

use super::binding::{Modifiers, ParsedHotkey};
use crate::error::HotkeyError;

/// Interval between periodic scans for newly connected keyboards.
const DEVICE_SCAN_INTERVAL: Duration = Duration::from_secs(5);

/// Minimum interval between keyboard rescans after an error.
const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// Sleep between poll iterations (non-blocking reads).
const POLL_SLEEP: Duration = Duration::from_millis(10);

/// Events emitted by the listener thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListenerEvent {
    /// The hotkey was pressed.
    Pressed,
    /// The hotkey was released.
    Released,
}

/// Start the background listener thread for the given hotkey.
///
/// Returns an `Arc<AtomicBool>` that can be set to `false` to stop the
/// thread.
///
/// # Errors
/// Returns `HotkeyError` if no keyboard devices are found or permissions
/// are insufficient.
pub(super) fn start_listener(
    hotkey: ParsedHotkey,
    tx: Sender<ListenerEvent>,
    running: Arc<AtomicBool>,
) -> Result<(), HotkeyError> {
    let keyboards = find_keyboards()?;
    set_nonblocking(&keyboards)?;

    let evdev_key = hotkey.key.to_evdev();
    let hotkey_mods = hotkey.modifiers;

    std::thread::spawn(move || {
        run_event_loop(keyboards, evdev_key, hotkey_mods, &tx, &running);
    });

    Ok(())
}

// ── Event loop ──────────────────────────────────────────────────────────────

/// Main event loop: reads from keyboards, tracks modifiers, fires events.
#[allow(clippy::too_many_lines)]
fn run_event_loop(
    initial_keyboards: Vec<Device>,
    evdev_key: evdev::Key,
    hotkey_mods: Modifiers,
    tx: &Sender<ListenerEvent>,
    running: &AtomicBool,
) {
    let mut keyboards = initial_keyboards;
    let mut current_mods = Modifiers::default();
    let mut known_paths: HashSet<PathBuf> = get_keyboard_paths();
    let mut last_rescan = Instant::now();
    let mut last_device_scan = Instant::now();
    let mut had_error = false;

    while running.load(Ordering::Relaxed) {
        // ── Reconnect after errors ──────────────────────────────────────
        if had_error && last_rescan.elapsed() >= RESCAN_INTERVAL {
            if let Some(new_keyboards) = try_rescan_keyboards() {
                keyboards = new_keyboards;
                current_mods = Modifiers::default();
                had_error = false;
                known_paths = get_keyboard_paths();
                last_device_scan = Instant::now();
            }
            last_rescan = Instant::now();
        }

        // ── Hot-plug detection ──────────────────────────────────────────
        if last_device_scan.elapsed() >= DEVICE_SCAN_INTERVAL {
            add_new_keyboards(&mut keyboards, &mut known_paths);
            last_device_scan = Instant::now();
        }

        // ── Read events ─────────────────────────────────────────────────
        let mut any_error = false;

        for device in &mut keyboards {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if let evdev::InputEventKind::Key(key) = event.kind() {
                            let pressed = event.value() == 1;
                            let released = event.value() == 0;

                            update_modifiers(&mut current_mods, key, pressed, released);

                            if key == evdev_key && current_mods == hotkey_mods {
                                let event = if pressed {
                                    ListenerEvent::Pressed
                                } else if released {
                                    ListenerEvent::Released
                                } else {
                                    continue;
                                };
                                // Receiver dropped → stop.
                                if tx.send(event).is_err() {
                                    running.store(false, Ordering::Relaxed);
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.raw_os_error() != Some(libc::EAGAIN)
                        && e.raw_os_error() != Some(libc::EWOULDBLOCK)
                    {
                        tracing::debug!("Keyboard read error: {e}");
                        any_error = true;
                    }
                }
            }
        }

        if any_error {
            had_error = true;
        }

        std::thread::sleep(POLL_SLEEP);
    }
}

// ── Modifier tracking ───────────────────────────────────────────────────────

/// Update the live modifier state based on a key event.
fn update_modifiers(mods: &mut Modifiers, key: evdev::Key, pressed: bool, released: bool) {
    let flag = match key {
        evdev::Key::KEY_LEFTSHIFT | evdev::Key::KEY_RIGHTSHIFT => &mut mods.shift,
        evdev::Key::KEY_LEFTCTRL | evdev::Key::KEY_RIGHTCTRL => &mut mods.ctrl,
        evdev::Key::KEY_LEFTALT | evdev::Key::KEY_RIGHTALT => &mut mods.alt,
        evdev::Key::KEY_LEFTMETA | evdev::Key::KEY_RIGHTMETA => &mut mods.super_,
        _ => return,
    };
    if pressed {
        *flag = true;
    } else if released {
        *flag = false;
    }
}

// ── Device discovery ────────────────────────────────────────────────────────

/// Find all keyboard devices under `/dev/input`.
fn find_keyboards() -> Result<Vec<Device>, HotkeyError> {
    let mut keyboards = Vec::new();

    let entries = std::fs::read_dir("/dev/input")
        .map_err(|e| HotkeyError::ListenerFailed(format!("Cannot read /dev/input: {e}")))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_event_device(&path) {
            continue;
        }
        if let Ok(device) = Device::open(&path) {
            if is_keyboard(&device) {
                tracing::debug!("Found keyboard: {:?} at {}", device.name(), path.display());
                keyboards.push(device);
            }
        }
    }

    if keyboards.is_empty() {
        Err(HotkeyError::ListenerFailed(
            "No keyboard devices found. Ensure user is in the 'input' group or running as root"
                .to_owned(),
        ))
    } else {
        Ok(keyboards)
    }
}

/// Set non-blocking mode on all devices.
fn set_nonblocking(keyboards: &[Device]) -> Result<(), HotkeyError> {
    for device in keyboards {
        let fd = device.as_raw_fd();
        let flags = fcntl(fd, FcntlArg::F_GETFL)
            .map_err(|e| HotkeyError::ListenerFailed(format!("F_GETFL failed: {e}")))?;
        let flags = OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK;
        fcntl(fd, FcntlArg::F_SETFL(flags))
            .map_err(|e| HotkeyError::ListenerFailed(format!("F_SETFL failed: {e}")))?;
    }
    Ok(())
}

/// Drain stale events from devices (important after reconnection).
fn drain_events(keyboards: &mut [Device]) {
    for device in keyboards {
        while let Ok(events) = device.fetch_events() {
            if events.count() == 0 {
                break;
            }
        }
    }
}

/// Attempt a full keyboard rescan, returning new devices if successful.
fn try_rescan_keyboards() -> Option<Vec<Device>> {
    tracing::info!("Rescanning keyboard devices...");
    let mut keyboards = find_keyboards().ok()?;

    // Give devices time to initialise (especially Bluetooth).
    std::thread::sleep(Duration::from_millis(100));

    if set_nonblocking(&keyboards).is_err() {
        return None;
    }

    drain_events(&mut keyboards);
    tracing::info!("Keyboards reconnected: {} device(s)", keyboards.len());
    Some(keyboards)
}

/// Discover and add newly connected keyboards.
fn add_new_keyboards(keyboards: &mut Vec<Device>, known_paths: &mut HashSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir("/dev/input") else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !is_event_device(&path) || known_paths.contains(&path) {
            continue;
        }
        if let Ok(device) = Device::open(&path) {
            if is_keyboard(&device) {
                tracing::info!(
                    "New keyboard detected: {:?} at {}",
                    device.name(),
                    path.display()
                );
                // Give device time to initialise.
                std::thread::sleep(Duration::from_millis(100));

                if set_nonblocking(std::slice::from_ref(&device)).is_ok() {
                    known_paths.insert(path);
                    let mut devs = vec![device];
                    drain_events(&mut devs);
                    keyboards.extend(devs);
                }
            }
        }
    }
}

/// Collect paths of all currently detectable keyboard devices.
fn get_keyboard_paths() -> HashSet<PathBuf> {
    let mut paths = HashSet::new();
    let Ok(entries) = std::fs::read_dir("/dev/input") else {
        return paths;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_event_device(&path) {
            continue;
        }
        if let Ok(device) = Device::open(&path) {
            if is_keyboard(&device) {
                paths.insert(path);
            }
        }
    }
    paths
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Check if a path looks like `/dev/input/eventN`.
fn is_event_device(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("event"))
}

/// Check if a device is a keyboard (supports `KEY_A`).
fn is_keyboard(device: &Device) -> bool {
    device
        .supported_keys()
        .is_some_and(|keys| keys.contains(evdev::Key::KEY_A))
}
