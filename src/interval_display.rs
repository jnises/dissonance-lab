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
    const INTERVAL_DISPLAY_HEIGHT: f32 = 200.0;
    const TEXT_Y_OFFSET: f32 = 4.0;
    const KEY_RECT_CORNER_RADIUS: f32 = 0.0;
    let interval_rect = Rect::from_min_max(
        pos2(
            piano_rect.left(),
            piano_rect.top() - INTERVAL_DISPLAY_HEIGHT,
        ),
        pos2(piano_rect.right(), piano_rect.top()),
    );
    ui.allocate_rect(interval_rect, Sense::empty());
    let painter = ui.painter();
    const SEMITONES_IN_OCTAVE: f32 = 12.0;
    let key_width = interval_rect.width() / SEMITONES_IN_OCTAVE;
    const PIANO_WIDTH_ADJUSTMENT: f32 = 4.0;
    let font_scale = interval_rect.width() / (PIANO_WIDTH - PIANO_WIDTH_ADJUSTMENT);
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
            const SCORE_CENTER_POS_ADJUSTMENT: f32 = 4.0;
            const SCORE_CENTER_POS_OFFSET: f32 = 10.0;
            let score_center_pos = pos
                - Vec2::Y
                    * ((row as f32 + 0.5) * (key_width + SCORE_CENTER_POS_ADJUSTMENT)
                        + SCORE_CENTER_POS_OFFSET);
            const OUTLINE_STROKE_WIDTH: f32 = 2.0;
            if this_selected {
                painter.rect_stroke(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    KEY_RECT_CORNER_RADIUS,
                    Stroke::new(OUTLINE_STROKE_WIDTH, theme::outlines()),
                    StrokeKind::Inside,
                );
                const NOTE_FONT_SIZE: f32 = 20.0;
                painter.text(
                    score_center_pos,
                    Align2::CENTER_CENTER,
                    "♪",
                    FontId::monospace(NOTE_FONT_SIZE * font_scale),
                    Color32::WHITE,
                );
            } else {
                let normalized_dissonance = (interval.dissonance()
                    - Interval::PerfectFifth.dissonance())
                    / (Interval::Tritone.dissonance() - Interval::PerfectFifth.dissonance());
                painter.rect_filled(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    KEY_RECT_CORNER_RADIUS,
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
                            score_center_pos
                                + vec2(
                                    -key_width / 2.0,
                                    key_width / 2.0 - OUTLINE_STROKE_WIDTH / 2.0,
                                ),
                            score_center_pos
                                + vec2(
                                    key_width / 2.0,
                                    key_width / 2.0 - OUTLINE_STROKE_WIDTH / 2.0,
                                ),
                        ],
                        Stroke::new(OUTLINE_STROKE_WIDTH, theme::outlines()),
                    );
                }
                const RATIO_FONT_SIZE: f32 = 14.0;
                let ratio_rect = painter.text(
                    score_center_pos - vec2(0.0, key_width / 2.0 - TEXT_Y_OFFSET),
                    Align2::CENTER_TOP,
                    interval.just_ratio().to_string(),
                    FontId::monospace(RATIO_FONT_SIZE * font_scale),
                    Color32::BLACK,
                );
                const CENTS_ERROR_Y_OFFSET: f32 = 2.0;
                const CENTS_ERROR_FONT_SIZE: f32 = 12.0;
                const CENTS_ERROR_ALPHA: u8 = 180;
                painter.text(
                    ratio_rect.center_bottom() + vec2(0.0, CENTS_ERROR_Y_OFFSET),
                    Align2::CENTER_TOP,
                    format!("{:+}¢", interval.tempered_just_error_cents() as i32),
                    FontId::monospace(CENTS_ERROR_FONT_SIZE * font_scale),
                    Color32::from_black_alpha(CENTS_ERROR_ALPHA),
                );
                const MIN_FONT_SCALE: f32 = 0.7;
                if font_scale > MIN_FONT_SCALE {
                    const INTERVAL_NAME_FONT_SIZE: f32 = 7.0;
                    const INTERVAL_NAME_ALPHA: u8 = 150;
                    painter.text(
                        score_center_pos + vec2(0.0, key_width / 2.0 - TEXT_Y_OFFSET),
                        Align2::CENTER_BOTTOM,
                        interval.to_string(),
                        FontId::proportional(INTERVAL_NAME_FONT_SIZE * font_scale),
                        Color32::from_black_alpha(INTERVAL_NAME_ALPHA),
                    );
                }
            }
        }
    }
    action
}
