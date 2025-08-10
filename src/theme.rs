use crate::utils::{colorgrad_to_egui, oklab};
use colorgrad::{BasisGradient, BlendMode};
use egui::Color32;
use std::sync::LazyLock;

pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(108, 108, 108); // #6C6C6C
const KEYBOARD_PRIMARY: Color32 = Color32::from_rgb(195, 193, 184); // rgb(195, 193, 184)
const KEYBOARD_OUTLINES_L: f32 = 1.0;
const KEYBOARD_OUTLINES_A: f32 = -0.02;
const KEYBOARD_OUTLINES_B: f32 = 0.01;
const KEYBOARD_OUTLINES_ALPHA: f32 = 1.0;
static KEYBOARD_OUTLINES: LazyLock<colorgrad::Color> = LazyLock::new(|| {
    colorgrad::Color::from_oklaba(
        KEYBOARD_OUTLINES_L,
        KEYBOARD_OUTLINES_A,
        KEYBOARD_OUTLINES_B,
        KEYBOARD_OUTLINES_ALPHA,
    )
});
pub const KEYBOARD_LABEL: Color32 = Color32::from_rgb(179, 179, 179); // #B3B3B3
pub const ATTENTION_TEXT: Color32 = Color32::from_rgb(219, 98, 137); // rgb(219, 98, 137)

pub fn outlines() -> Color32 {
    colorgrad_to_egui(&KEYBOARD_OUTLINES)
}

pub fn pressed_key() -> Color32 {
    KEYBOARD_PRIMARY // Keep the original bright color for actively pressed keys
}

pub fn sustained_key() -> Color32 {
    // Slightly darker/desaturated version of pressed keys
    Color32::from_rgb(175, 173, 164)
}

const EXTERNAL_SELECTED_KEY_R: u8 = 150;
const EXTERNAL_SELECTED_KEY_G: u8 = 148;
const EXTERNAL_SELECTED_KEY_B: u8 = 140;
pub fn external_key() -> Color32 {
    Color32::from_rgb(
        EXTERNAL_SELECTED_KEY_R,
        EXTERNAL_SELECTED_KEY_G,
        EXTERNAL_SELECTED_KEY_B,
    ) // rgb(150, 148, 140)
}

pub fn external_sustained_key() -> Color32 {
    // Even more desaturated version for sustained external keys
    Color32::from_rgb(130, 128, 120)
}

pub static DISSONANCE_GRADIENT: LazyLock<BasisGradient> = LazyLock::new(|| {
    // Create gradient with monotonically increasing darkness (darker = more dissonant)
    // Consonant intervals are bright/light, dissonant intervals are dark/intense
    colorgrad::GradientBuilder::new()
        .html_colors(&[
            "#E6F3FF", // Very light blue for unison (most consonant)
            "#B3D9FF", // Light blue for perfect consonances (perfect fifth/fourth)
            "#80BFFF", // Medium-light blue for low dissonance
            "#4DA6FF", // Medium blue for moderate-low dissonance  
            "#1A8CFF", // Brighter blue for moderate dissonance (thirds/sixths)
            "#0066CC", // Darker blue for tritone (more dissonant than perfect fifth)
            "#004499", // Dark blue for moderate-high dissonance
            "#003366", // Very dark blue for high dissonance
            "#002244", // Very dark blue-black for very high dissonance
            "#000011", // Near black for maximum dissonance (minor second)
        ])
        .mode(BlendMode::Oklab)
        .build::<BasisGradient>()
        .unwrap()
});

pub fn setup_custom_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Force dark mode regardless of system preference
    visuals.dark_mode = true;

    const PANEL_FILL_L: f32 = 0.19;
    const PANEL_FILL_A: f32 = -0.01;
    const PANEL_FILL_B: f32 = -0.03;
    const PANEL_FILL_ALPHA: f32 = 1.0;
    visuals.panel_fill = oklab(PANEL_FILL_L, PANEL_FILL_A, PANEL_FILL_B, PANEL_FILL_ALPHA);
    visuals.button_frame = false;

    // Set the custom visuals
    ctx.set_visuals(visuals);
}
