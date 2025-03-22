use egui::{Response, RichText, Ui};
use web_time::{Duration, Instant};

/// Convert a color from colorgrad to egui's Color32
pub fn colorgrad_to_egui(color: &colorgrad::Color) -> egui::Color32 {
    let [r, g, b, a] = color.to_rgba8();
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn oklab(l: f32, a: f32, b: f32, alpha: f32) -> egui::Color32 {
    colorgrad_to_egui(&colorgrad::Color::from_oklaba(l, a, b, alpha))
}

pub struct AttentionButton {
    start_time: Instant,
    duration: Duration,
}

impl AttentionButton {
    pub fn new(duration: Duration) -> Self {
        Self {
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, text: impl Into<RichText>) -> Response {
        let mut rich: RichText = text.into();
        let elapsed = self.start_time.elapsed();
        if elapsed < self.duration {
            ui.ctx().request_repaint();
            if (elapsed.as_secs_f32() * 3.0).rem_euclid(1.0) > 0.5 {
                rich = rich.strong();
            }
        }
        ui.button(rich)
    }
}
