use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Label, Orientation, Spinner, Window};

type Callback = dyn Fn();

/// Build the compact overlay window for recording status display.
///
/// Layout: status label, spinner, error area, cancel button.
/// Keyboard: Escape to cancel, Enter to confirm.
///
/// Callbacks are set after construction via [`OverlayWindow::set_on_cancel`]
/// and [`OverlayWindow::set_on_confirm`].
pub fn build_overlay() -> OverlayWindow {
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

    let level_value: Rc<Cell<f32>> = Rc::new(Cell::new(0.0));
    let level_area = build_level_meter(&level_value);

    let spinner = Spinner::new();
    spinner.set_visible(false);

    let error_label = Label::new(None);
    error_label.add_css_class("error-message");
    error_label.set_wrap(true);
    error_label.set_visible(false);

    let stop_btn = Button::with_label("Stop");
    stop_btn.add_css_class("stop-button");

    let cancel_btn = Button::with_label("Cancel");
    cancel_btn.add_css_class("cancel-button");

    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::Center);
    button_box.append(&stop_btn);
    button_box.append(&cancel_btn);

    main_box.append(&status_label);
    main_box.append(&timer_label);
    main_box.append(&level_area);
    main_box.append(&spinner);
    main_box.append(&error_label);
    main_box.append(&button_box);

    window.set_child(Some(&main_box));

    // Stop button gets default focus so Enter activates it
    stop_btn.grab_focus();

    let on_cancel: Rc<RefCell<Option<Box<Callback>>>> = Rc::new(RefCell::new(None));
    let on_confirm: Rc<RefCell<Option<Box<Callback>>>> = Rc::new(RefCell::new(None));

    wire_callbacks(&stop_btn, &cancel_btn, &window, &on_cancel, &on_confirm);

    OverlayWindow {
        window,
        status_label,
        timer_label,
        level_area,
        level_value,
        spinner,
        error_label,
        _stop_btn: stop_btn,
        _cancel_btn: cancel_btn,
        on_cancel,
        on_confirm,
    }
}

/// Connect stop/cancel button clicks and keyboard shortcuts to callback slots.
fn wire_callbacks(
    stop_btn: &Button,
    cancel_btn: &Button,
    window: &Window,
    on_cancel: &Rc<RefCell<Option<Box<Callback>>>>,
    on_confirm: &Rc<RefCell<Option<Box<Callback>>>>,
) {
    let cb = Rc::clone(on_confirm);
    stop_btn.connect_clicked(move |_| {
        invoke_callback(&cb);
    });

    let cb = Rc::clone(on_cancel);
    cancel_btn.connect_clicked(move |_| {
        invoke_callback(&cb);
    });

    let key_controller = gtk4::EventControllerKey::new();
    let cancel_cb = Rc::clone(on_cancel);
    let confirm_cb = Rc::clone(on_confirm);
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            invoke_callback(&cancel_cb);
            gtk4::glib::Propagation::Stop
        } else if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
            invoke_callback(&confirm_cb);
            gtk4::glib::Propagation::Stop
        } else {
            gtk4::glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);
}

/// Invoke a stored callback, catching panics.
///
/// Takes the callback from the slot, invokes it, and restores it.
/// If the callback panics, it is NOT restored to prevent re-invoking broken code.
fn invoke_callback(slot: &Rc<RefCell<Option<Box<Callback>>>>) {
    let f = slot.borrow_mut().take();
    if let Some(f) = f {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(&f)).is_ok() {
            *slot.borrow_mut() = Some(f);
        } else {
            eprintln!("PANIC in callback — callback disabled");
        }
    }
}

/// Wrapper for the overlay window with access to its widgets.
pub struct OverlayWindow {
    window: Window,
    status_label: Label,
    timer_label: Label,
    level_area: DrawingArea,
    level_value: Rc<Cell<f32>>,
    spinner: Spinner,
    error_label: Label,
    // Buttons are stored to prevent drop; callbacks are already wired in wire_callbacks().
    _stop_btn: Button,
    _cancel_btn: Button,
    on_cancel: Rc<RefCell<Option<Box<Callback>>>>,
    on_confirm: Rc<RefCell<Option<Box<Callback>>>>,
}

impl OverlayWindow {
    /// Set the callback invoked when the user cancels (button or Escape).
    pub fn set_on_cancel(&self, f: impl Fn() + 'static) {
        *self.on_cancel.borrow_mut() = Some(Box::new(f));
    }

    /// Set the callback invoked when the user confirms (Enter key).
    pub fn set_on_confirm(&self, f: impl Fn() + 'static) {
        *self.on_confirm.borrow_mut() = Some(Box::new(f));
    }

    /// Get a reference to the underlying GTK window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Update the VU meter level.
    pub fn update_level(&self, level: f32) {
        self.level_value.set(level);
        self.level_area.queue_draw();
    }

    /// Show the overlay in recording state.
    pub fn show_recording(&self) {
        self.status_label.set_text("Recording...");
        self.timer_label.set_text("00:00");
        self.timer_label.set_visible(true);
        self.level_value.set(0.0);
        self.level_area.set_visible(true);
        self.level_area.queue_draw();
        self.spinner.set_visible(false);
        self.spinner.set_spinning(false);
        self.error_label.set_visible(false);
        self.window.present();
    }

    /// Switch to transcribing state.
    pub fn show_transcribing(&self) {
        self.status_label.set_text("Transcribing...");
        self.timer_label.set_visible(false);
        self.level_area.set_visible(false);
        self.spinner.set_spinning(true);
        self.spinner.set_visible(true);
    }

    /// Show post-processing progress (e.g., "Step 1/3: Grammar...").
    pub fn show_processing(&self, step: usize, total: usize, name: &str) {
        self.status_label
            .set_text(&format!("Step {step}/{total}: {name}..."));
        self.timer_label.set_visible(false);
        self.level_area.set_visible(false);
        self.spinner.set_spinning(true);
        self.spinner.set_visible(true);
        self.error_label.set_visible(false);
    }

    /// Update the timer display.
    pub fn update_timer(&self, elapsed: std::time::Duration) {
        let secs = elapsed.as_secs();
        self.timer_label
            .set_text(&format!("{:02}:{:02}", secs / 60, secs % 60));
    }
}

/// Number of discrete bars in the VU meter.
const BAR_COUNT: u32 = 16;

/// Gap in pixels between bars.
const BAR_GAP: f64 = 2.0;

/// Build a `DrawingArea`-based VU meter that reads its level from `level_value`.
fn build_level_meter(level_value: &Rc<Cell<f32>>) -> DrawingArea {
    let area = DrawingArea::new();
    area.set_content_width(200);
    area.set_content_height(16);
    area.add_css_class("level-meter");

    let lv = Rc::clone(level_value);
    area.set_draw_func(move |_area, cr, width, height| {
        draw_level_bars(cr, width, height, lv.get());
    });

    area
}

/// Render discrete colored bars into the cairo context.
fn draw_level_bars(cr: &gtk4::cairo::Context, width: i32, height: i32, level: f32) {
    use std::f64::consts::{FRAC_PI_2, PI};

    let total_width = f64::from(width);
    let total_height = f64::from(height);
    let count = f64::from(BAR_COUNT);
    let total_gaps = (count - 1.0) * BAR_GAP;
    let bar_width = (total_width - total_gaps) / count;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let filled = (f64::from(level) * count).round() as u32;

    for idx in 0..BAR_COUNT {
        let bar_x = f64::from(idx) * (bar_width + BAR_GAP);

        // Color by position: green → yellow → red
        let color = if idx < 10 {
            (0.18, 0.8, 0.34) // green
        } else if idx < 13 {
            (0.95, 0.77, 0.06) // yellow
        } else {
            (0.91, 0.30, 0.24) // red
        };

        if idx < filled {
            cr.set_source_rgb(color.0, color.1, color.2);
        } else {
            cr.set_source_rgba(color.0, color.1, color.2, 0.15);
        }

        // Rounded rectangle via arcs (2px radius)
        let radius = 2.0_f64.min(bar_width / 2.0);
        cr.new_path();
        cr.arc(bar_x + bar_width - radius, radius, radius, -FRAC_PI_2, 0.0);
        cr.arc(
            bar_x + bar_width - radius,
            total_height - radius,
            radius,
            0.0,
            FRAC_PI_2,
        );
        cr.arc(bar_x + radius, total_height - radius, radius, FRAC_PI_2, PI);
        cr.arc(bar_x + radius, radius, radius, PI, 3.0 * FRAC_PI_2);
        cr.close_path();
        let _ = cr.fill();
    }
}
