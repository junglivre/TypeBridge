//! Small reusable UI helpers.

use eframe::egui::{Color32, RichText, Sense, Ui, Vec2};

/// A colored status "badge": a small filled dot plus a strong label.
///
/// The dot is painted (not a font glyph) so it always renders regardless of the
/// active font's coverage.
pub fn status_badge(ui: &mut Ui, label: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, color);
    ui.label(RichText::new(label).color(color).strong());
}
