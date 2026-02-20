use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box as GtkBox, Button, Label, LevelBar, Orientation, ScrolledWindow,
    Spinner, TextView, WrapMode,
};

use voice_type::audio::{AudioCapture, CaptureConfig};
use voice_type::config::AppConfig;
use voice_type::provider::{TranscribeOptions, TranscriptionProvider};

/// Recording window state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Ready,
    Recording,
    Transcribing,
    Result,
    Error,
}

/// Shared mutable state.
struct Inner {
    state: State,
    capture: Option<AudioCapture>,
    timer_source: Option<glib::SourceId>,
}

/// Grouped UI widgets for the recording window.
struct Widgets {
    status_label: Label,
    timer_label: Label,
    level_bar: LevelBar,
    spinner: Spinner,
    scrolled: ScrolledWindow,
    error_label: Label,
    copy_btn: Button,
    text_buf: gtk4::TextBuffer,
}

/// Build a one-shot recording window for the `record` command.
#[allow(clippy::too_many_lines)]
pub fn build_recording_window(app: &gtk4::Application, config: &AppConfig) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Voice Type — Record")
        .default_width(420)
        .default_height(320)
        .resizable(true)
        .build();

    let (widgets, record_btn) = build_widgets(&window);

    // --- State and config ---
    let inner = Rc::new(RefCell::new(Inner {
        state: State::Ready,
        capture: None,
        timer_source: None,
    }));

    let capture_config = CaptureConfig {
        device_name: config.audio.device.clone(),
        sample_rate: config.audio.sample_rate.hz(),
        max_duration: config.audio.max_duration,
    };
    let silence_threshold = config.audio.silence_threshold;

    let (provider, transcribe_options) = config.provider.build_provider();

    // --- Record button ---
    record_btn.connect_clicked(move |btn| {
        let current = inner.borrow().state.clone();

        match current {
            State::Ready | State::Result | State::Error => {
                start_recording(&inner, &capture_config, btn, &widgets);
            }
            State::Recording => {
                stop_recording_and_transcribe(
                    &inner,
                    silence_threshold,
                    &provider,
                    &transcribe_options,
                    btn,
                    &widgets,
                );
            }
            State::Transcribing => {
                // Ignore clicks while transcribing
            }
        }
    });

    window
}

/// Build all UI widgets and assemble the window layout.
fn build_widgets(window: &ApplicationWindow) -> (Widgets, Button) {
    let main_box = GtkBox::new(Orientation::Vertical, 12);
    main_box.set_margin_top(16);
    main_box.set_margin_bottom(16);
    main_box.set_margin_start(16);
    main_box.set_margin_end(16);

    let status_label = Label::new(Some("Ready to record"));
    status_label.add_css_class("recording-indicator");

    let timer_label = Label::new(Some("00:00"));
    timer_label.add_css_class("elapsed-time");
    timer_label.set_visible(false);

    let level_bar = LevelBar::new();
    level_bar.set_min_value(0.0);
    level_bar.set_max_value(1.0);
    level_bar.set_value(0.0);
    level_bar.add_css_class("rms-level");
    level_bar.set_visible(false);

    let spinner = Spinner::new();
    spinner.set_visible(false);

    let text_view = TextView::new();
    text_view.set_wrap_mode(WrapMode::WordChar);
    text_view.set_editable(false);
    text_view.add_css_class("transcription-text");

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&text_view)
        .build();
    scrolled.set_visible(false);

    let error_label = Label::new(None);
    error_label.add_css_class("error-message");
    error_label.set_wrap(true);
    error_label.set_visible(false);

    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::Center);

    let record_btn = Button::with_label("Start Recording");
    record_btn.add_css_class("suggested-action");

    let copy_btn = Button::with_label("Copy");
    copy_btn.set_visible(false);

    let close_btn = Button::with_label("Close");

    button_box.append(&record_btn);
    button_box.append(&copy_btn);
    button_box.append(&close_btn);

    main_box.append(&status_label);
    main_box.append(&timer_label);
    main_box.append(&level_bar);
    main_box.append(&spinner);
    main_box.append(&scrolled);
    main_box.append(&error_label);
    main_box.append(&button_box);

    window.set_child(Some(&main_box));

    // Close button
    {
        let w = window.clone();
        close_btn.connect_clicked(move |_| w.close());
    }

    // Copy button
    let text_buf = text_view.buffer();
    {
        let buf = text_buf.clone();
        let w = window.clone();
        copy_btn.connect_clicked(move |_| {
            let (start, end) = buf.bounds();
            let text = buf.text(&start, &end, false);
            let display = gtk4::prelude::WidgetExt::display(&w);
            display.clipboard().set_text(&text);
        });
    }

    let widgets = Widgets {
        status_label,
        timer_label,
        level_bar,
        spinner,
        scrolled,
        error_label,
        copy_btn,
        text_buf,
    };

    (widgets, record_btn)
}

fn start_recording(
    inner: &Rc<RefCell<Inner>>,
    capture_config: &CaptureConfig,
    btn: &Button,
    w: &Widgets,
) {
    match AudioCapture::start(capture_config) {
        Ok(capture) => {
            {
                let mut s = inner.borrow_mut();
                s.state = State::Recording;
                s.capture = Some(capture);
            }

            w.status_label.set_text("Recording...");
            w.timer_label.set_text("00:00");
            w.timer_label.set_visible(true);
            w.level_bar.set_value(0.0);
            w.level_bar.set_visible(true);
            w.spinner.set_visible(false);
            w.scrolled.set_visible(false);
            w.error_label.set_visible(false);
            w.copy_btn.set_visible(false);
            btn.set_label("Stop Recording");
            btn.remove_css_class("suggested-action");
            btn.add_css_class("destructive-action");

            // Periodic timer/RMS updates + stream error/max duration checks
            let inner_ref = Rc::clone(inner);
            let tl = w.timer_label.clone();
            let lb = w.level_bar.clone();
            let sl = w.status_label.clone();
            let el = w.error_label.clone();
            let source = glib::timeout_add_local(Duration::from_millis(100), move || {
                let borrow = inner_ref.borrow();
                if let Some(ref cap) = borrow.capture {
                    // Check for stream error (microphone disconnect)
                    if cap.has_stream_error() {
                        drop(borrow);
                        sl.set_text("Microphone error");
                        el.set_text("Microphone disconnected. Partial audio may be available.");
                        el.set_visible(true);
                        return glib::ControlFlow::Break;
                    }
                    // Check max duration
                    if cap.is_max_duration_reached() {
                        drop(borrow);
                        sl.set_text("Max duration reached");
                        return glib::ControlFlow::Break;
                    }
                    let elapsed = cap.elapsed();
                    let secs = elapsed.as_secs();
                    tl.set_text(&format!("{:02}:{:02}", secs / 60, secs % 60));
                    lb.set_value(f64::from(cap.current_rms().value()));
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            });
            inner.borrow_mut().timer_source = Some(source);
        }
        Err(e) => {
            w.status_label.set_text("Error");
            w.error_label.set_text(&e.to_string());
            w.error_label.set_visible(true);
            inner.borrow_mut().state = State::Error;
        }
    }
}

fn stop_recording_and_transcribe(
    inner: &Rc<RefCell<Inner>>,
    silence_threshold: voice_type::types::RmsLevel,
    provider: &Arc<dyn TranscriptionProvider>,
    options: &TranscribeOptions,
    btn: &Button,
    w: &Widgets,
) {
    // Stop timer
    if let Some(source) = inner.borrow_mut().timer_source.take() {
        source.remove();
    }

    let capture = inner.borrow_mut().capture.take();
    let Some(cap) = capture else { return };
    let captured = cap.stop();

    // Silence check
    if captured.is_silence(silence_threshold) {
        show_error_state(
            inner,
            btn,
            w,
            "No speech detected",
            "The recording appears to be silence. Speak louder or check your microphone.",
        );
        return;
    }

    // Encode WAV
    let audio_data = match captured.into_audio_data() {
        Ok(data) => data,
        Err(e) => {
            show_error_state(inner, btn, w, "Error", &e.to_string());
            return;
        }
    };

    // Switch to transcribing state
    inner.borrow_mut().state = State::Transcribing;
    w.status_label.set_text("Transcribing...");
    w.timer_label.set_visible(false);
    w.level_bar.set_visible(false);
    w.spinner.set_spinning(true);
    w.spinner.set_visible(true);
    btn.set_sensitive(false);
    btn.set_label("Transcribing...");
    btn.remove_css_class("destructive-action");

    // Spawn transcription thread
    let provider = Arc::clone(provider);
    let opts = options.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = provider.transcribe(&audio_data, &opts);
        let _ = tx.send(result);
    });

    // Poll for result on main loop
    let inner_ref = Rc::clone(inner);
    let sl = w.status_label.clone();
    let sp = w.spinner.clone();
    let sc = w.scrolled.clone();
    let el = w.error_label.clone();
    let b = btn.clone();
    let cp = w.copy_btn.clone();
    let tb = w.text_buf.clone();

    glib::timeout_add_local(Duration::from_millis(50), move || match rx.try_recv() {
        Ok(Ok(result)) => {
            inner_ref.borrow_mut().state = State::Result;
            sl.set_text("Transcription complete");
            sp.set_spinning(false);
            sp.set_visible(false);
            tb.set_text(result.text.as_str());
            sc.set_visible(true);
            cp.set_visible(true);
            b.set_label("Record Again");
            b.add_css_class("suggested-action");
            b.set_sensitive(true);
            glib::ControlFlow::Break
        }
        Ok(Err(e)) => {
            inner_ref.borrow_mut().state = State::Error;
            sl.set_text("Transcription failed");
            sp.set_spinning(false);
            sp.set_visible(false);
            el.set_text(&e.to_string());
            el.set_visible(true);
            b.set_label("Try Again");
            b.add_css_class("suggested-action");
            b.set_sensitive(true);
            glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(_) => {
            inner_ref.borrow_mut().state = State::Error;
            sl.set_text("Error");
            sp.set_spinning(false);
            sp.set_visible(false);
            el.set_text("Transcription thread terminated unexpectedly");
            el.set_visible(true);
            b.set_label("Try Again");
            b.add_css_class("suggested-action");
            b.set_sensitive(true);
            glib::ControlFlow::Break
        }
    });
}

/// Helper to show an error state and reset recording UI.
fn show_error_state(
    inner: &Rc<RefCell<Inner>>,
    btn: &Button,
    w: &Widgets,
    status: &str,
    message: &str,
) {
    w.status_label.set_text(status);
    w.error_label.set_text(message);
    w.error_label.set_visible(true);
    w.timer_label.set_visible(false);
    w.level_bar.set_visible(false);
    btn.set_label("Try Again");
    btn.remove_css_class("destructive-action");
    btn.add_css_class("suggested-action");
    inner.borrow_mut().state = State::Error;
}
