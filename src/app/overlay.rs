use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, Orientation, ScrolledWindow, Spinner, TextView, Window, WrapMode,
};

/// Build the compact overlay window for daemon mode transcription confirmation.
///
/// Layout: status label, spinner/result area, editable text, confirm/cancel buttons.
/// Keyboard: Enter to confirm, Escape to cancel.
pub fn build_overlay(
    on_confirm: impl Fn(String) + 'static,
    on_cancel: impl Fn() + 'static,
) -> OverlayWindow {
    let window = Window::builder()
        .title("Voice Type")
        .default_width(400)
        .default_height(200)
        .resizable(true)
        .decorated(true)
        .build();

    window.add_css_class("overlay-window");

    let main_box = GtkBox::new(Orientation::Vertical, 8);
    main_box.set_margin_top(12);
    main_box.set_margin_bottom(12);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);

    let status_label = Label::new(Some("Recording..."));
    status_label.add_css_class("recording-indicator");

    let timer_label = Label::new(Some("00:00"));
    timer_label.add_css_class("elapsed-time");

    let spinner = Spinner::new();
    spinner.set_visible(false);

    let text_view = TextView::new();
    text_view.set_wrap_mode(WrapMode::WordChar);
    text_view.set_editable(true);
    text_view.add_css_class("transcription-text");
    text_view.set_visible(false);

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&text_view)
        .build();

    let error_label = Label::new(None);
    error_label.add_css_class("error-message");
    error_label.set_wrap(true);
    error_label.set_visible(false);

    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    let confirm_btn = Button::with_label("Confirm");
    confirm_btn.add_css_class("confirm-button");
    confirm_btn.set_visible(false);

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("cancel-button");

    button_box.append(&cancel_btn);
    button_box.append(&confirm_btn);

    main_box.append(&status_label);
    main_box.append(&timer_label);
    main_box.append(&spinner);
    main_box.append(&scrolled);
    main_box.append(&error_label);
    main_box.append(&button_box);

    window.set_child(Some(&main_box));

    // Confirm button
    let text_buf = text_view.buffer();
    {
        let buf = text_buf.clone();
        confirm_btn.connect_clicked(move |_| {
            let (start, end) = buf.bounds();
            let text = buf.text(&start, &end, false);
            on_confirm(text.to_string());
        });
    }

    // Cancel button
    {
        let on_cancel_ref = std::rc::Rc::new(on_cancel);
        let oc = on_cancel_ref.clone();
        cancel_btn.connect_clicked(move |_| oc());

        // Escape key to cancel
        let key_controller = gtk4::EventControllerKey::new();
        let oc2 = on_cancel_ref;
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                oc2();
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        window.add_controller(key_controller);
    }

    OverlayWindow {
        window,
        status_label,
        timer_label,
        spinner,
        text_view,
        scrolled,
        error_label,
        confirm_btn,
        _cancel_btn: cancel_btn,
        text_buf,
    }
}

/// Wrapper for the overlay window with access to its widgets.
pub struct OverlayWindow {
    pub window: Window,
    pub status_label: Label,
    pub timer_label: Label,
    pub spinner: Spinner,
    pub text_view: TextView,
    pub scrolled: ScrolledWindow,
    pub error_label: Label,
    pub confirm_btn: Button,
    pub _cancel_btn: Button,
    pub text_buf: gtk4::TextBuffer,
}

impl OverlayWindow {
    /// Show the overlay in recording state.
    pub fn show_recording(&self) {
        self.status_label.set_text("Recording...");
        self.timer_label.set_text("00:00");
        self.timer_label.set_visible(true);
        self.spinner.set_visible(false);
        self.spinner.set_spinning(false);
        self.scrolled.set_visible(false);
        self.text_view.set_visible(false);
        self.error_label.set_visible(false);
        self.confirm_btn.set_visible(false);
        self.window.present();
    }

    /// Switch to transcribing state.
    pub fn show_transcribing(&self) {
        self.status_label.set_text("Transcribing...");
        self.timer_label.set_visible(false);
        self.spinner.set_spinning(true);
        self.spinner.set_visible(true);
    }

    /// Show the transcription result for confirmation.
    pub fn show_result(&self, text: &str) {
        self.status_label.set_text("Review and confirm:");
        self.spinner.set_spinning(false);
        self.spinner.set_visible(false);
        self.text_buf.set_text(text);
        self.text_view.set_visible(true);
        self.scrolled.set_visible(true);
        self.confirm_btn.set_visible(true);
        self.text_view.grab_focus();
    }

    /// Show an error.
    pub fn show_error(&self, message: &str) {
        self.status_label.set_text("Error");
        self.spinner.set_spinning(false);
        self.spinner.set_visible(false);
        self.error_label.set_text(message);
        self.error_label.set_visible(true);
    }

    /// Update the timer display.
    pub fn update_timer(&self, elapsed: std::time::Duration) {
        let secs = elapsed.as_secs();
        self.timer_label
            .set_text(&format!("{:02}:{:02}", secs / 60, secs % 60));
    }

    /// Hide the overlay.
    pub fn hide(&self) {
        self.window.set_visible(false);
    }
}
