use crate::{
    interval::{self, Interval},
    piano_gui::{self, PIANO_WIDTH},
    theme,
    utils::colorgrad_to_egui,
};
use colorgrad::Gradient;
use egui::{
    Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, epaint::PathShape, pos2,
    vec2,
};

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
            // always consider the pressed key as the base
            // TODO: if we show more than one octave we show the actual base as the root
            let interval = interval::Interval::from_semitone_wrapping(semi - selected_semi);
            let pos = pos2(
                interval_rect.left() + key_width * (semi as f32 + 0.5),
                interval_rect.bottom(),
            );
            let this_selected = semi == selected_semi;
            let score_center_pos = pos - Vec2::Y * ((row as f32 + 0.5) * (key_width + 4.0) + 10.0);
            const OUTLINE_STROKE_WIDTH: f32 = 2.0;
            if this_selected {
                painter.rect_stroke(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    0.0,
                    Stroke::new(OUTLINE_STROKE_WIDTH, theme::outlines()),
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
                // draw triangles to indicate that the pressed key is considered the root
                const TRIANGLE_SIZE: f32 = 1.0 / 6.0;
                painter.add(PathShape::convex_polygon(
                    vec![
                        score_center_pos + vec2(-key_width / 2.0, key_width / 2.0),
                        score_center_pos
                            + vec2(-key_width / 2.0, key_width * (0.5 - TRIANGLE_SIZE)),
                        score_center_pos
                            + vec2(key_width * (-0.5 + TRIANGLE_SIZE), key_width / 2.0),
                    ],
                    theme::outlines(),
                    Stroke::NONE,
                ));
                if (semi + 1).rem_euclid(12) != selected_semi {
                    painter.line_segment(
                        [
                            score_center_pos + vec2(-key_width / 2.0, key_width / 2.0 - OUTLINE_STROKE_WIDTH / 2.0),
                            score_center_pos + vec2(key_width / 2.0, key_width / 2.0- OUTLINE_STROKE_WIDTH / 2.0),
                        ],
                        Stroke::new(OUTLINE_STROKE_WIDTH, theme::outlines()),
                    );
                }
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
    action
}
