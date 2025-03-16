use egui::{pos2, vec2, Rect, Sense, Ui};
use crate::piano_gui;

pub fn show(piano: &mut piano_gui::PianoGui, ui: &mut Ui) -> Option<piano_gui::Action> {
    let (action, piano_rect) = piano.show(ui);
    let interval_rect = Rect::from_min_max(pos2(piano_rect.left(), piano_rect.top() - 200.0), pos2(piano_rect.right(), piano_rect.top()));
    ui.allocate_rect(interval_rect, Sense::empty());
    let painter = ui.painter();
    for semi in 0..12 {
        let position = pos2(interval_rect.left() + interval_rect.width() / 12.0 * (semi as f32 + 0.5), interval_rect.bottom());
        painter.line_segment(
            [
                position,
                position - vec2(0.0, 10.0),
            ],
            egui::Stroke::new(1.0, egui::Color32::GRAY),
        );
    }
    action
}