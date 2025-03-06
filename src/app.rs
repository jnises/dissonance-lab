use egui::{Sense, ThemePreference, Vec2};

use crate::theory::is_key_black;

pub struct TheoryApp {
    pressed: Option<usize>,
}

impl Default for TheoryApp {
    fn default() -> Self {
        Self { pressed: None }
    }
}

impl TheoryApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(ThemePreference::Dark);
        Default::default()
    }
}

impl eframe::App for TheoryApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                for note in 0..12 {
                    if ui.available_width() <= 0f32 {
                        break;
                    }
                    // Calculate semitone difference (if any pressed note exists)
                    let semi_diff_from_pressed = self.pressed.map(|pressed_note| {
                        // Use rem_euclid which properly handles negative numbers
                        // and always returns a positive remainder
                        u8::try_from((note as i32 - pressed_note as i32).rem_euclid(12)).unwrap()
                    });

                    let diff_interval = semi_diff_from_pressed
                        .map(|diff| crate::theory::Interval::from_semitone_interval(diff));

                    let just_interval = diff_interval.map(|diff| diff.just_ratio());

                    let cent_error = diff_interval.map(|diff| diff.just_tempered_error_cents());

                    // Use this value later if needed for display or logic
                    let this_pressed = Some(note) == self.pressed;
                    const KEY_SIZE: Vec2 = Vec2::new(40f32, 80f32);
                    let (key_id, key_rect) = ui.allocate_space(KEY_SIZE);

                    let interact = ui.interact(key_rect, key_id, Sense::click());
                    let painter = ui.painter();
                    painter.rect_filled(
                        key_rect,
                        0f32,
                        if this_pressed {
                            ui.style().visuals.selection.bg_fill
                        } else if is_key_black(note) {
                            egui::Color32::BLACK
                        } else {
                            egui::Color32::WHITE
                        },
                    );

                    // Display just interval and cent error if a note is pressed
                    if let Some(just) = just_interval {
                        if let Some(cents) = cent_error {
                            let text = format!("{:.2}\n{:.1}¢", just, cents);
                            painter.text(
                                key_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                text,
                                egui::FontId::default(),
                                if is_key_black(note) {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::BLACK
                                },
                            );
                        }
                    }

                    if interact.clicked() {
                        self.pressed = Some(note);
                    }
                }
            });
        });
    }
}
