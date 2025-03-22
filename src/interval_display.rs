use crate::{
    interval::{self, Interval},
    piano_gui::{self, PIANO_WIDTH},
    theme,
    utils::colorgrad_to_egui,
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
    let font_scale = interval_rect.width() / (PIANO_WIDTH - 4.0);
    for (row, selected_semi) in piano
        .pressed_keys()
        .iter_ones()
        .map(|i| i8::try_from(i).unwrap())
        .enumerate()
    {
        for semi in 0..12i8 {
            let interval = interval::Interval::from_semitone_wrapping(semi - selected_semi);
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
                    Stroke::new(2.0, theme::outlines()),
                    StrokeKind::Inside,
                );
                painter.text(
                    score_center_pos,
                    Align2::CENTER_CENTER,
                    "♪",
                    FontId::monospace(20.0 * font_scale),
                    Color32::WHITE,
                );
            } else {
                let normalized_dissonance = (interval.dissonance()
                    - Interval::PerfectFifth.dissonance())
                    / (Interval::Tritone.dissonance() - Interval::PerfectFifth.dissonance());
                painter.rect_filled(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    0.0,
                    colorgrad_to_egui(&theme::DISSONANCE_GRADIENT.at(normalized_dissonance)),
                );
                let ratio_rect = painter.text(
                    score_center_pos - vec2(0.0, key_width / 2.0 - 4.0),
                    Align2::CENTER_TOP,
                    interval.just_ratio().to_string(),
                    FontId::monospace(14.0 * font_scale),
                    Color32::BLACK,
                );
                painter.text(
                    ratio_rect.center_bottom() + vec2(0.0, 2.0),
                    Align2::CENTER_TOP,
                    format!("{:+}¢", interval.tempered_just_error_cents() as i32),
                    FontId::monospace(12.0 * font_scale),
                    Color32::from_black_alpha(180),
                );
                if font_scale > 0.7 {
                    painter.text(
                        score_center_pos + vec2(0.0, key_width / 2.0 - 4.0),
                        Align2::CENTER_BOTTOM,
                        interval.to_string(),
                        FontId::proportional(7.0 * font_scale),
                        Color32::from_black_alpha(150),
                    );
                }
            }
        }
    }
    if piano.pressed_keys().count_ones() > 1 {
        let chord_dissonances: Vec<_> = (0..12i8)
            .map(|semi| {
                let mut chord: Vec<_> = piano.pressed_keys().iter_ones().collect();
                if !piano.pressed_keys()[semi as usize] {
                    chord.push(semi as usize);
                }
                Interval::chord_dissonance(
                    chord
                        .into_iter()
                        .map(|i| Interval::from_semitone_wrapping(i8::try_from(i).unwrap())),
                )
            })
            .collect();
        let consonant_chord = *chord_dissonances
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let dissonant_chord = *chord_dissonances
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        for (semi, dissonance) in chord_dissonances.into_iter().enumerate() {
            let normalized_dissonance = if (dissonant_chord - consonant_chord).abs() > f32::EPSILON
            {
                (dissonance - consonant_chord) / (dissonant_chord - consonant_chord)
            } else {
                dissonance
            };
            let pos = pos2(
                interval_rect.left() + key_width * (semi as f32 + 0.5),
                interval_rect.bottom(),
            );
            let score_center_pos = pos
                - Vec2::Y
                    * ((piano.pressed_keys().count_ones() as f32 + 0.4) * (key_width + 4.0) - 4.0);
            painter.rect_filled(
                Rect::from_center_size(score_center_pos, vec2(key_width, 8.0)),
                0.0,
                colorgrad_to_egui(&theme::DISSONANCE_GRADIENT.at(normalized_dissonance)),
            );
        }
    }
    action
}
