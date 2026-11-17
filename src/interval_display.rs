use crate::{
    interval::{self, Interval},
    piano_gui::{PIANO_WIDTH, PianoGui},
    piano_types::Semitone,
    theme,
    utils::colorgrad_to_egui,
};
use colorgrad::Gradient;
use egui::{
    Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, epaint::PathShape, pos2,
    vec2,
};
use num_rational::Rational32;

pub fn show(piano: &PianoGui, ui: &mut Ui, piano_rect: Rect, just: bool) {
    let mut row_references = piano.held_keys();
    let reference = piano.tuning_reference().filter(|_| just).map(u8::from);
    if just {
        row_references.fill(false);
        if let Some(reference) = reference {
            row_references.set(usize::from(reference % 12), true);
        }
    }
    const INTERVAL_DISPLAY_HEIGHT: f32 = 200.0;
    const TEXT_Y_OFFSET: f32 = 4.0;
    const KEY_RECT_CORNER_RADIUS: f32 = 0.0;
    let bottom = ui.cursor().bottom();
    let interval_rect = Rect::from_min_max(
        pos2(piano_rect.left(), bottom - INTERVAL_DISPLAY_HEIGHT),
        pos2(piano_rect.right(), bottom),
    );
    ui.allocate_rect(interval_rect, Sense::empty());
    let painter = ui.painter();
    const SEMITONES_IN_OCTAVE: f32 = 12.0;
    let key_width = interval_rect.width() / SEMITONES_IN_OCTAVE;
    const PIANO_WIDTH_ADJUSTMENT: f32 = 4.0;
    let font_scale = interval_rect.width() / (PIANO_WIDTH - PIANO_WIDTH_ADJUSTMENT);
    for (row, selected_semi) in row_references
        .iter_ones()
        .map(|i| i8::try_from(i).unwrap())
        .enumerate()
    {
        for semi in 0..12i8 {
            let interval = interval::Interval::from_semitone_wrapping(semi - selected_semi);
            let note = u8::from(piano.note_for_key(Semitone::new(semi as u8)));
            let this_selected = reference.map_or(semi == selected_semi, |root| note == root);
            let pos = pos2(
                interval_rect.left() + key_width * (semi as f32 + 0.5),
                interval_rect.bottom(),
            );
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
                let note_stroke = Stroke::new(1.5, theme::outlines());
                let head = score_center_pos + vec2(-2.0, 4.0) * font_scale;
                let stem = head + vec2(2.5, 0.0) * font_scale;
                let top = stem - vec2(0.0, 11.0) * font_scale;
                painter.circle_stroke(head, 2.5 * font_scale, note_stroke);
                painter.line(
                    vec![stem, top, top + vec2(4.5, 3.5) * font_scale],
                    note_stroke,
                );
            } else {
                let normalized_dissonance = (interval.dissonance()
                    - Interval::PerfectFifth.dissonance())
                    / (Interval::Tritone.dissonance() - Interval::PerfectFifth.dissonance());
                painter.rect_filled(
                    Rect::from_center_size(score_center_pos, Vec2::splat(key_width)),
                    KEY_RECT_CORNER_RADIUS,
                    colorgrad_to_egui(
                        &theme::DISSONANCE_GRADIENT.at(normalized_dissonance.clamp(0.0, 1.0)),
                    ),
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
                let ratio = reference.map_or_else(
                    || interval.just_ratio(),
                    |root| {
                        let (numerator, denominator) = shared_types::just_note_ratio(note, root);
                        Rational32::new(numerator, denominator)
                    },
                );
                let ratio_rect = painter.text(
                    score_center_pos - vec2(0.0, key_width / 2.0 - TEXT_Y_OFFSET),
                    Align2::CENTER_TOP,
                    format!("{}/{}", ratio.numer(), ratio.denom()),
                    FontId::monospace(RATIO_FONT_SIZE * font_scale),
                    Color32::BLACK,
                );
                const CENTS_ERROR_Y_OFFSET: f32 = 2.0;
                const CENTS_ERROR_FONT_SIZE: f32 = 12.0;
                const CENTS_ERROR_ALPHA: u8 = 180;
                painter.text(
                    ratio_rect.center_bottom() + vec2(0.0, CENTS_ERROR_Y_OFFSET),
                    Align2::CENTER_TOP,
                    format!("{:+.0}¢", interval.tempered_just_error_cents()),
                    FontId::monospace(CENTS_ERROR_FONT_SIZE * font_scale),
                    Color32::from_black_alpha(CENTS_ERROR_ALPHA),
                );
                const MIN_FONT_SCALE: f32 = 0.7;
                if font_scale > MIN_FONT_SCALE {
                    const INTERVAL_NAME_FONT_SIZE: f32 = 7.0;
                    const INTERVAL_NAME_ALPHA: u8 = 150;
                    let name = reference.map_or_else(
                        || interval.to_string(),
                        |root| {
                            let distance = note.abs_diff(root);
                            if distance > 12 {
                                return format!("{:+} st", i16::from(note) - i16::from(root));
                            }
                            let interval = Interval::from_semitone_interval(distance);
                            if note < root {
                                format!("↓ {interval}")
                            } else {
                                interval.to_string()
                            }
                        },
                    );
                    painter.text(
                        score_center_pos + vec2(0.0, key_width / 2.0 - TEXT_Y_OFFSET),
                        Align2::CENTER_BOTTOM,
                        name,
                        FontId::proportional(INTERVAL_NAME_FONT_SIZE * font_scale),
                        Color32::from_black_alpha(INTERVAL_NAME_ALPHA),
                    );
                }
            }
        }
    }
}
