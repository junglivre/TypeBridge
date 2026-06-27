//! Small reusable UI helpers.

use eframe::egui::{self, Align2, Color32, FontId, RichText, Sense, Stroke, Ui, Vec2};

/// A colored status "badge": a small filled dot plus a strong label.
///
/// The dot is painted (not a font glyph) so it always renders regardless of the
/// active font's coverage.
pub fn status_badge(ui: &mut Ui, label: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, color);
    ui.label(RichText::new(label).color(color).strong());
}

/// A bold red warning triangle with an exclamation mark, painted by hand so it
/// always renders (no emoji-font dependency).
pub fn warning_icon(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.46;
    let top = egui::pos2(c.x, c.y - r);
    let left = egui::pos2(c.x - r * 0.95, c.y + r * 0.72);
    let right = egui::pos2(c.x + r * 0.95, c.y + r * 0.72);

    let fill = Color32::from_rgb(220, 60, 60);
    let edge = Color32::from_rgb(110, 16, 16);
    let painter = ui.painter();
    painter.add(egui::Shape::convex_polygon(
        vec![top, left, right],
        fill,
        Stroke::new(2.0, edge),
    ));
    painter.text(
        egui::pos2(c.x, c.y + r * 0.16),
        Align2::CENTER_CENTER,
        "!",
        FontId::proportional(size * 0.52),
        Color32::WHITE,
    );
}

/// A green circle with an upward arrow — the "update available" badge. Painted
/// by hand so it renders without an emoji font.
pub fn update_icon(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size * 0.46;

    let fill = Color32::from_rgb(60, 180, 95);
    let edge = Color32::from_rgb(28, 110, 55);
    let painter = ui.painter();
    painter.circle(c, r, fill, Stroke::new(2.0, edge));

    // Up arrow: a vertical stem plus a chevron head, in white.
    let w = Stroke::new(size * 0.085, Color32::WHITE);
    let top = egui::pos2(c.x, c.y - r * 0.5);
    let bottom = egui::pos2(c.x, c.y + r * 0.5);
    painter.line_segment([top, bottom], w);
    painter.line_segment([top, egui::pos2(c.x - r * 0.42, c.y - r * 0.04)], w);
    painter.line_segment([top, egui::pos2(c.x + r * 0.42, c.y - r * 0.04)], w);
}
