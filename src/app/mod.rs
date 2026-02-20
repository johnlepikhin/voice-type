pub mod overlay;
pub mod recording_window;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use voice_type::audio::{AudioCapture, CaptureConfig};
use voice_type::config::AppConfig;
use voice_type::hotkey::{HotkeyAction, HotkeyListener};
use voice_type::insertion;
use voice_type::provider::{TranscribeOptions, TranscriptionProvider};

pub use recording_window::build_recording_window;

/// Load the application CSS stylesheet.
pub fn load_css() {
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data(include_str!("../css/style.css"));
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

/// Daemon phase state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonPhase {
    Idle,
    Recording,
    Transcribing,
    AwaitingConfirmation,
}

/// Shared daemon state.
struct DaemonState {
    phase: DaemonPhase,
    capture: Option<AudioCapture>,
    timer_source: Option<glib::SourceId>,
}

/// Build and run the daemon mode GTK application.
///
/// The daemon:
/// 1. Starts a global hotkey listener in a background thread
/// 2. Polls for hotkey events on the glib main loop
/// 3. Toggles recording on hotkey press
/// 4. Shows an overlay for transcription confirmation
/// 5. Inserts confirmed text into the previously focused window
pub fn run_daemon(_app: &gtk4::Application, config: &AppConfig) {
    let capture_config = CaptureConfig {
        device_name: config.audio.device.clone(),
        sample_rate: config.audio.sample_rate.hz(),
        max_duration: config.audio.max_duration,
    };
    let silence_threshold = config.audio.silence_threshold;

    let (provider, transcribe_options) = config.provider.build_provider();

    let state = Rc::new(RefCell::new(DaemonState {
        phase: DaemonPhase::Idle,
        capture: None,
        timer_source: None,
    }));

    // Build overlay with confirm/cancel callbacks
    let state_for_confirm = Rc::clone(&state);
    let state_for_cancel = Rc::clone(&state);

    let overlay = Rc::new(overlay::build_overlay(
        move |text| {
            // Confirm: insert text and return to idle
            tracing::info!("Confirming text insertion: {} chars", text.len());
            if let Err(e) = insertion::insert_text(&text) {
                tracing::error!("Text insertion failed: {e}");
            }
            state_for_confirm.borrow_mut().phase = DaemonPhase::Idle;
        },
        move || {
            // Cancel: return to idle
            tracing::info!("Transcription cancelled");
            state_for_cancel.borrow_mut().phase = DaemonPhase::Idle;
        },
    ));

    // Start hotkey listener
    let hotkey_listener = match HotkeyListener::start(&config.hotkey.binding) {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("Failed to start hotkey listener: {e}");
            tracing::error!("Make sure you have permission to read /dev/input devices");
            return;
        }
    };

    tracing::info!(
        "Daemon started. Press {} to toggle recording.",
        config.hotkey.binding
    );

    // Poll for hotkey events every 50ms
    let overlay_ref = Rc::clone(&overlay);
    glib::timeout_add_local(Duration::from_millis(50), move || {
        if let Some(HotkeyAction::Toggle) = hotkey_listener.try_recv() {
            let current_phase = state.borrow().phase.clone();
            match current_phase {
                DaemonPhase::Idle => {
                    start_daemon_recording(&state, &capture_config, &overlay_ref);
                }
                DaemonPhase::Recording => {
                    stop_daemon_recording(
                        &state,
                        silence_threshold,
                        &provider,
                        &transcribe_options,
                        &overlay_ref,
                    );
                }
                DaemonPhase::Transcribing | DaemonPhase::AwaitingConfirmation => {
                    // Ignore hotkey during transcription/confirmation
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

fn start_daemon_recording(
    state: &Rc<RefCell<DaemonState>>,
    capture_config: &CaptureConfig,
    overlay: &Rc<overlay::OverlayWindow>,
) {
    match AudioCapture::start(capture_config) {
        Ok(capture) => {
            {
                let mut s = state.borrow_mut();
                s.phase = DaemonPhase::Recording;
                s.capture = Some(capture);
            }

            overlay.show_recording();

            // Timer updates + stream error/max duration checks
            let state_ref = Rc::clone(state);
            let overlay_ref = Rc::clone(overlay);
            let source = glib::timeout_add_local(Duration::from_millis(100), move || {
                let borrow = state_ref.borrow();
                if let Some(ref cap) = borrow.capture {
                    if cap.has_stream_error() {
                        tracing::warn!("Audio stream error detected during daemon recording");
                        overlay_ref
                            .show_error("Microphone disconnected. Partial audio may be available.");
                        return glib::ControlFlow::Break;
                    }
                    if cap.is_max_duration_reached() {
                        tracing::info!("Max recording duration reached");
                        return glib::ControlFlow::Break;
                    }
                    overlay_ref.update_timer(cap.elapsed());
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            });
            state.borrow_mut().timer_source = Some(source);

            tracing::info!("Recording started");
        }
        Err(e) => {
            tracing::error!("Failed to start recording: {e}");
            overlay.show_error(&e.to_string());
        }
    }
}

fn stop_daemon_recording(
    state: &Rc<RefCell<DaemonState>>,
    silence_threshold: voice_type::types::RmsLevel,
    provider: &Arc<dyn TranscriptionProvider>,
    options: &TranscribeOptions,
    overlay: &Rc<overlay::OverlayWindow>,
) {
    // Stop timer
    if let Some(source) = state.borrow_mut().timer_source.take() {
        source.remove();
    }

    let capture = state.borrow_mut().capture.take();
    let Some(cap) = capture else { return };
    let captured = cap.stop();

    tracing::info!("Recording stopped, duration: {:?}", captured.duration);

    // Silence check
    if captured.is_silence(silence_threshold) {
        tracing::warn!("Recording was silence");
        overlay.show_error("No speech detected. Speak louder or check your microphone.");
        state.borrow_mut().phase = DaemonPhase::Idle;
        glib::timeout_add_local_once(Duration::from_secs(3), {
            let o = Rc::clone(overlay);
            move || o.hide()
        });
        return;
    }

    // Encode WAV
    let audio_data = match captured.into_audio_data() {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("WAV encoding failed: {e}");
            overlay.show_error(&e.to_string());
            state.borrow_mut().phase = DaemonPhase::Idle;
            return;
        }
    };

    // Start transcription
    state.borrow_mut().phase = DaemonPhase::Transcribing;
    overlay.show_transcribing();

    let provider = Arc::clone(provider);
    let opts = options.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = provider.transcribe(&audio_data, &opts);
        let _ = tx.send(result);
    });

    // Poll for result
    let state_ref = Rc::clone(state);
    let overlay_ref = Rc::clone(overlay);
    glib::timeout_add_local(Duration::from_millis(50), move || match rx.try_recv() {
        Ok(Ok(result)) => {
            tracing::info!(
                "Transcription complete: {} chars",
                result.text.as_str().len()
            );
            state_ref.borrow_mut().phase = DaemonPhase::AwaitingConfirmation;
            overlay_ref.show_result(result.text.as_str());
            glib::ControlFlow::Break
        }
        Ok(Err(e)) => {
            tracing::error!("Transcription failed: {e}");
            overlay_ref.show_error(&e.to_string());
            state_ref.borrow_mut().phase = DaemonPhase::Idle;
            glib::timeout_add_local_once(Duration::from_secs(3), {
                let o = Rc::clone(&overlay_ref);
                move || o.hide()
            });
            glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(_) => {
            tracing::error!("Transcription thread disconnected");
            overlay_ref.show_error("Transcription failed unexpectedly");
            state_ref.borrow_mut().phase = DaemonPhase::Idle;
            glib::ControlFlow::Break
        }
    });
}
