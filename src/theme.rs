use crate::utils::{colorgrad_to_egui, oklab};
use colorgrad::{BasisGradient, BlendMode};
use egui::Color32;
use std::sync::LazyLock;

pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(108, 108, 108); // #6C6C6C
const KEYBOARD_PRIMARY: Color32 = Color32::from_rgb(195, 193, 184); // rgb(195, 193, 184)
static KEYBOARD_OUTLINES: LazyLock<colorgrad::Color> =
    LazyLock::new(|| colorgrad::Color::from_oklaba(1.0, -0.02, 0.01, 1.0));
pub const KEYBOARD_LABEL: Color32 = Color32::from_rgb(179, 179, 179); // #B3B3B3

pub fn outlines() -> Color32 {
    colorgrad_to_egui(&KEYBOARD_OUTLINES)
}

pub fn selected_key() -> Color32 {
    KEYBOARD_PRIMARY
}

pub fn external_selected_key() -> Color32 {
    Color32::from_rgb(150, 148, 140) // rgb(150, 148, 140)
}

pub static DISSONANCE_GRADIENT: LazyLock<BasisGradient> = LazyLock::new(|| {
    colorgrad::GradientBuilder::new()
        .html_colors(&[
            "#4A90E2", "#3CCFCF", "#98D353", "#FFC857", "#FF9A3D", "#FF6B6B", "#FF3366",
        ])
        .mode(BlendMode::Oklab)
        .build::<BasisGradient>()
        .unwrap()
});

pub fn setup_custom_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = oklab(0.19, -0.01, -0.03, 1.0);
    visuals.button_frame = false;

    // Set the custom visuals
    ctx.set_visuals(visuals);
}
