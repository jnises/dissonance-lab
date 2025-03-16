use crate::{
    piano_gui,
    theory::{self, Interval},
    utils::{colorgrad_to_egui, colorous_to_egui},
};
use colorgrad::Gradient;
use egui::{Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

pub fn show(piano: &mut piano_gui::PianoGui, ui: &mut Ui) -> Option<piano_gui::Action> {
    let (action, piano_rect) = piano.show(ui);
    let interval_rect = Rect::from_min_max(
        pos2(piano_rect.left(), piano_rect.top() - 200.0),
        pos2(piano_rect.right(), piano_rect.top()),
    );
    ui.allocate_rect(interval_rect, Sense::empty());
    let painter = ui.painter();
    let key_width = interval_rect.width() / 12.0;
    for (row, selected_semi) in piano
        .selected_keys()
        .iter_ones()
        .map(|i| i8::try_from(i).unwrap())
        .enumerate()
    {
        for semi in 0..12i8 {
            let interval = theory::Interval::from_semitone_wrapping(semi - selected_semi);
            let pos = pos2(
                interval_rect.left() + key_width * (semi as f32 + 0.5),
                interval_rect.bottom(),
            );
            let this_selected = semi == selected_semi;
            let score_center_pos = pos - Vec2::Y * ((row as f32 + 0.5) * (key_width + 4.0) + 10.0);
            if this_selected {
                painter.rect_stroke(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    0.0,
                    Stroke::new(2.0, Color32::WHITE),
                    StrokeKind::Inside,
                );
                painter.text(
                    score_center_pos,
                    Align2::CENTER_CENTER,
                    "♪",
                    FontId::monospace(20.0),
                    Color32::WHITE,
                );
            } else {
                let normalized_dissonance =
                    interval.compound_dissonance() / Interval::Tritone.compound_dissonance();
                painter.rect_filled(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    0.0,
                    colorous_to_egui(
                        colorous::YELLOW_ORANGE_RED.eval_continuous(normalized_dissonance as f64),
                    ),
                );
                painter.text(
                    score_center_pos - vec2(0.0, key_width / 2.0 - 4.0),
                    Align2::CENTER_TOP,
                    interval.just_ratio().to_string(),
                    FontId::monospace(14.0),
                    Color32::BLACK,
                );
                painter.text(
                    score_center_pos - vec2(0.0, key_width / 2.0 - 20.0),
                    Align2::CENTER_TOP,
                    format!("{:+}¢", interval.tempered_just_error_cents() as i32),
                    FontId::monospace(12.0),
                    Color32::from_black_alpha(180),
                );
                painter.text(
                    score_center_pos + vec2(0.0, key_width / 2.0 - 4.0),
                    Align2::CENTER_BOTTOM,
                    interval.to_string(),
                    FontId::proportional(7.0),
                    Color32::from_black_alpha(150),
                );
            }
        }
    }
    action
}
