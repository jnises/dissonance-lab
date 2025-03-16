use crate::piano_gui;
use egui::{Color32, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

pub fn show(piano: &mut piano_gui::PianoGui, ui: &mut Ui) -> Option<piano_gui::Action> {
    let (action, piano_rect) = piano.show(ui);
    let interval_rect = Rect::from_min_max(
        pos2(piano_rect.left(), piano_rect.top() - 200.0),
        pos2(piano_rect.right(), piano_rect.top()),
    );
    ui.allocate_rect(interval_rect, Sense::empty());
    let painter = ui.painter();
    let key_width = interval_rect.width() / 12.0;
    for semi in 0..12 {
        let pos = pos2(
            interval_rect.left() + key_width * (semi as f32 + 0.5),
            interval_rect.bottom(),
        );
        let selected = piano.selected_keys()[semi];
        painter.line_segment(
            [pos, pos - vec2(0.0, 10.0)],
            Stroke::new(
                1.0,
                if selected {
                    Color32::BLUE
                } else {
                    Color32::GRAY
                },
            ),
        );
        if selected {
            painter.circle_filled(
                pos - Vec2::Y * (key_width / 2.0 + 10.0),
                key_width / 2.0,
                Color32::BLUE,
            );
        }
    }
    action
}
