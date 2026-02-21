pub mod overlay;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gtk4::prelude::*;

use voice_type::audio::{AudioCapture, CaptureConfig};
use voice_type::config::AppConfig;
use voice_type::postprocess::{PipelineProgress, ProcessingPipeline};
use voice_type::provider::TranscribeOptions;

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

/// Shared state for the record flow.
struct RecordState {
    capture: Option<AudioCapture>,
    timer_source: Option<glib::SourceId>,
    confirmed: bool,
}

/// Run the record flow: show overlay, record, transcribe, print to stdout.
///
/// The overlay starts recording immediately. Press Enter to stop and process,
/// Escape to cancel. Final text is printed to stdout.
#[allow(clippy::too_many_lines)]
pub fn run_record(app: &gtk4::Application, config: &AppConfig) {
    app.connect_shutdown(|app| {
        tracing::debug!(
            window_count = app.windows().len(),
            "GTK application shutdown signal"
        );
    });

    let capture_config = CaptureConfig {
        device_name: config.audio.device.clone(),
        sample_rate: config.audio.sample_rate.hz(),
        max_duration: config.audio.max_duration,
    };
    let silence_threshold = config.audio.silence_threshold;

    let (provider, transcribe_options) = config.provider.build_provider();
    let pipeline = Arc::new(ProcessingPipeline::from_configs(&config.post_processing));

    let state = Rc::new(RefCell::new(RecordState {
        capture: None,
        timer_source: None,
        confirmed: false,
    }));

    let overlay = Rc::new(overlay::build_overlay());
    overlay.window().set_application(Some(app));
    tracing::debug!(
        window_visible = overlay.window().is_visible(),
        window_mapped = overlay.window().is_mapped(),
        "Overlay window created"
    );

    // Start recording immediately
    match AudioCapture::start(&capture_config) {
        Ok(capture) => {
            state.borrow_mut().capture = Some(capture);
            overlay.show_recording();
            tracing::info!("Recording started, press Enter to stop or Escape to cancel");
            tracing::debug!(
                window_visible = overlay.window().is_visible(),
                window_mapped = overlay.window().is_mapped(),
                "Overlay shown for recording"
            );
        }
        Err(e) => {
            eprintln!("Failed to start recording: {e}");
            std::process::exit(1);
        }
    }

    // Timer: update elapsed/RMS and auto-stop on max duration
    let state_for_timer = Rc::clone(&state);
    let overlay_for_timer = Rc::clone(&overlay);
    let app_for_timer = app.clone();
    let provider_for_timer = Arc::clone(&provider);
    let opts_for_timer = transcribe_options.clone();
    let pipeline_for_timer = Arc::clone(&pipeline);
    let source = glib::timeout_add_local(Duration::from_millis(100), move || {
        tracing::trace!("Timer tick");
        let borrow = state_for_timer.borrow();
        if let Some(ref cap) = borrow.capture {
            if cap.has_stream_error() {
                drop(borrow);
                eprintln!("Microphone disconnected during recording");
                std::process::exit(1);
            }
            if cap.is_max_duration_reached() {
                drop(borrow);
                stop_and_process(
                    &state_for_timer,
                    silence_threshold,
                    &provider_for_timer,
                    &opts_for_timer,
                    &pipeline_for_timer,
                    &overlay_for_timer,
                    &app_for_timer,
                );
                return glib::ControlFlow::Break;
            }
            overlay_for_timer.update_timer(cap.elapsed());
            overlay_for_timer.update_level(cap.current_rms().value());
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
    state.borrow_mut().timer_source = Some(source);

    // Enter → stop and process
    {
        let state_ref = Rc::clone(&state);
        let overlay_ref = Rc::clone(&overlay);
        let app_ref = app.clone();
        let provider_ref = Arc::clone(&provider);
        let opts_ref = transcribe_options;
        let pipeline_ref = Arc::clone(&pipeline);
        overlay.set_on_confirm(move || {
            stop_and_process(
                &state_ref,
                silence_threshold,
                &provider_ref,
                &opts_ref,
                &pipeline_ref,
                &overlay_ref,
                &app_ref,
            );
        });
    }

    // Escape → cancel
    {
        let app_ref = app.clone();
        overlay.set_on_cancel(move || {
            tracing::debug!("Cancel (Escape) pressed");
            app_ref.quit();
            std::process::exit(1);
        });
    }

    tracing::debug!(
        window_count = app.windows().len(),
        is_registered = app.is_registered(),
        "run_record finished, returning to main loop"
    );
}

/// Stop recording, encode, transcribe, post-process, and print result.
#[allow(clippy::too_many_lines)]
fn stop_and_process(
    state: &Rc<RefCell<RecordState>>,
    silence_threshold: voice_type::types::RmsLevel,
    provider: &Arc<dyn voice_type::provider::TranscriptionProvider>,
    options: &TranscribeOptions,
    pipeline: &Arc<ProcessingPipeline>,
    overlay: &Rc<overlay::OverlayWindow>,
    app: &gtk4::Application,
) {
    tracing::debug!("stop_and_process called");

    // Guard against double-confirm
    {
        let mut s = state.borrow_mut();
        if s.confirmed {
            tracing::debug!("stop_and_process: already confirmed, skipping");
            return;
        }
        s.confirmed = true;
    }

    // Stop timer
    if let Some(source) = state.borrow_mut().timer_source.take() {
        source.remove();
        tracing::debug!("Timer source removed");
    }

    let capture = state.borrow_mut().capture.take();
    let Some(cap) = capture else {
        tracing::warn!("stop_and_process: no capture to stop");
        return;
    };
    let captured = cap.stop();

    tracing::info!(
        duration = ?captured.duration,
        sample_count = captured.samples.len(),
        "Recording stopped"
    );

    // Silence check
    if captured.is_silence(silence_threshold) {
        eprintln!("No speech detected. Speak louder or check your microphone.");
        std::process::exit(1);
    }

    // Encode WAV
    let audio_data = match captured.into_audio_data() {
        Ok(data) => data,
        Err(e) => {
            eprintln!("WAV encoding failed: {e}");
            std::process::exit(1);
        }
    };

    // Show transcribing state
    overlay.show_transcribing();
    tracing::debug!("Spawning transcription thread");

    // Spawn background thread: transcribe → post-process
    let provider = Arc::clone(provider);
    let pipeline = Arc::clone(pipeline);
    let opts = options.clone();
    let (tx, rx) = std::sync::mpsc::channel::<PipelineProgress>();

    std::thread::spawn(move || {
        voice_type::log_memory_usage("transcription_start");

        let result = provider.transcribe(&audio_data, &opts);
        drop(audio_data);

        voice_type::log_memory_usage("transcription_done");

        let transcribed_text = match result {
            Ok(r) => r.text,
            Err(e) => {
                let _ = tx.send(PipelineProgress::Failed {
                    processor_name: "transcription".to_owned(),
                    error: e.to_string(),
                    original_text: String::new(),
                });
                return;
            }
        };

        if pipeline.is_empty() {
            let _ = tx.send(PipelineProgress::Done {
                text: transcribed_text.to_string(),
            });
        } else {
            pipeline.run(transcribed_text.as_str(), &tx);
        }

        voice_type::log_memory_usage("pipeline_done");
    });

    // Poll for progress
    let overlay_ref = Rc::clone(overlay);
    let app_ref = app.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || match rx.try_recv() {
        Ok(PipelineProgress::StepStarted {
            index,
            total,
            ref name,
        }) => {
            overlay_ref.show_processing(index + 1, total, name);
            glib::ControlFlow::Continue
        }
        Ok(PipelineProgress::Done { ref text }) => {
            tracing::debug!(text_len = text.len(), "Pipeline done, printing result");
            println!("{text}");
            app_ref.quit();
            glib::ControlFlow::Break
        }
        Ok(PipelineProgress::Failed {
            ref processor_name,
            ref error,
            ref original_text,
        }) => {
            if processor_name == "transcription" {
                eprintln!("Transcription failed: {error}");
                std::process::exit(1);
            } else {
                // Post-processing failed — print original text
                eprintln!("Post-processing '{processor_name}' failed: {error}");
                println!("{original_text}");
                app_ref.quit();
            }
            glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) | Ok(_) => glib::ControlFlow::Continue,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            eprintln!("Processing thread terminated unexpectedly");
            std::process::exit(1);
        }
    });
}
