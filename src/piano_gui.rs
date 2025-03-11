use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub struct PianoGui;

impl PianoGui {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&mut self, ui: &mut Ui) {
        // Set up the piano dimensions
        let piano_width = ui.available_width().min(600.0);
        let piano_height = 200.0;
        let white_key_count = 14; // Two octaves
        let white_key_width = piano_width / white_key_count as f32;
        let white_key_height = piano_height;
        let black_key_width = white_key_width * 0.6;
        let black_key_height = piano_height * 0.6;

        // Create a canvas to draw on
        let (response, painter) =
            ui.allocate_painter(vec2(piano_width, piano_height), Sense::click_and_drag());
        let rect = response.rect;

        // Draw the piano frame
        painter.rect_filled(rect, 0.0, Color32::from_rgb(120, 120, 120));

        // Draw white keys
        for i in 0..white_key_count {
            let key_rect = Rect::from_min_size(
                pos2(rect.min.x + i as f32 * white_key_width, rect.min.y),
                vec2(white_key_width, white_key_height),
            );

            // Draw white key with a border
            painter.rect_filled(key_rect, 0.0, Color32::WHITE);

            painter.rect_stroke(
                key_rect,
                0.0,
                Stroke::new(1.0, Color32::BLACK),
                StrokeKind::Middle,
            );
        }

        // Draw black keys
        // Pattern of black keys in an octave: after the 1st, 2nd, 4th, 5th, and 6th white keys
        let black_key_positions = [0, 1, 3, 4, 5, 7, 8, 10, 11, 12];

        for &i in &black_key_positions {
            // Position black keys between white keys
            let offset = white_key_width - (black_key_width / 2.0);
            let key_rect = Rect::from_min_size(
                pos2(rect.min.x + i as f32 * white_key_width + offset, rect.min.y),
                vec2(black_key_width, black_key_height),
            );

            painter.rect_filled(key_rect, 0.0, Color32::BLACK);
        }
    }
}
