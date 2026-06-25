//! Small reusable UI helpers.

use eframe::egui::{Color32, RichText, Ui};

/// A colored status "badge": a dot plus a strong label.
pub fn status_badge(ui: &mut Ui, label: &str, color: Color32) {
    ui.label(RichText::new(format!("● {label}")).color(color).strong());
}
