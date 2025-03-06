use std::sync::LazyLock;

use colorgrad::{BlendMode, Gradient as _};
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
                    const KEY_SIZE: Vec2 = Vec2::new(50f32, 140f32);
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

                    if let (true, Some(just), Some(cents)) =
                        (!this_pressed, just_interval, cent_error)
                    {
                        // Draw the just ratio
                        painter.text(
                            key_rect.center_top() + Vec2::new(0.0, 50.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.2}", just),
                            egui::FontId::default(),
                            if is_key_black(note) {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::BLACK
                            },
                        );
                        
                        static CENT_ERROR_GRADIENT: LazyLock<colorgrad::LinearGradient> = LazyLock::new(|| {
                            colorgrad::GradientBuilder::new()
                                .colors(&[
                                    colorgrad::Color::new(0.5, 0.5, 0.5, 1.0),
                                    colorgrad::Color::new(1.0, 1.0, 0.0, 1.0),
                                    colorgrad::Color::new(1.0, 0.0, 0.0, 1.0),
                                ])
                                .domain(&[5.0, 10.0, 20.0])
                                .mode(BlendMode::Oklab)
                                .build()
                                .unwrap()
                        });
                        

                        // Draw the cents error
                        painter.text(
                            key_rect.center_top() + Vec2::new(0.0, 80.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.1}¢", cents),
                            egui::FontId::default(),
                            {
                                // Get color based on absolute cent error value
                                let abs_cents = cents.abs();
                                let color = CENT_ERROR_GRADIENT.at(abs_cents);
                                let [r, g, b, a] = color.to_rgba8();
                                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                            },
                        );
                    }

                    if interact.clicked() {
                        self.pressed = Some(note);
                    }
                }
            });
        });
    }
}
