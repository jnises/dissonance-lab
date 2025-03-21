use std::sync::LazyLock;

use colorgrad::{BasisGradient, BlendMode, Gradient};
use egui::Color32;

use crate::utils::{colorgrad_to_egui, oklab};

// Background & Base UI
const BACKGROUND_MAIN: Color32 = Color32::from_rgb(18, 18, 18); // #121212
const BACKGROUND_SECONDARY: Color32 = Color32::from_rgb(30, 30, 30); // #1E1E1E
const UI_ELEMENT_BG: Color32 = Color32::from_rgb(45, 45, 45); // #2D2D2D

// Dissonance Visualization Scale (from consonance to dissonance)
const PERFECT_CONSONANCE: Color32 = Color32::from_rgb(74, 144, 226); // #4A90E2
const MILD_CONSONANCE: Color32 = Color32::from_rgb(60, 207, 207); // #3CCFCF
const NEUTRAL_INTERVALS: Color32 = Color32::from_rgb(152, 211, 83); // #98D353
const MILD_DISSONANCE: Color32 = Color32::from_rgb(255, 200, 87); // #FFC857
const MODERATE_DISSONANCE: Color32 = Color32::from_rgb(255, 154, 61); // #FF9A3D
const STRONG_DISSONANCE: Color32 = Color32::from_rgb(255, 107, 107); // #FF6B6B
const MAXIMUM_DISSONANCE: Color32 = Color32::from_rgb(255, 51, 102); // #FF3366

// UI Accents & Interactive Elements
const PRIMARY_ACTION: Color32 = Color32::from_rgb(110, 123, 242); // #6E7BF2
const SECONDARY_ACTION: Color32 = Color32::from_rgb(74, 74, 74); // #4A4A4A
const HOVER_STATE: Color32 = Color32::from_rgb(140, 140, 255); // #8C8CFF
const SELECTION_STATE: LazyLock<Color32> =
    LazyLock::new(|| Color32::from_rgba_unmultiplied(110, 123, 242, 102)); // #6E7BF2 at 40% opacity

// Text & Labels
const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255); // #FFFFFF
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(179, 179, 179); // #B3B3B3
pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(108, 108, 108); // #6C6C6C
const TEXT_LINK: Color32 = Color32::from_rgb(140, 140, 255); // #8C8CFF

// Annotation
const GRID_LINES: Color32 = Color32::from_rgb(51, 51, 51); // #333333
const AXIS_LABELS: Color32 = Color32::from_rgb(179, 179, 179); // #B3B3B3
const IMPORTANT_MARKERS: Color32 = Color32::from_rgb(255, 154, 61); // #FF9A3D

const KEYBOARD_PRIMARY: Color32 = Color32::from_rgb(195, 193, 184); // rgb(195, 193, 184)
// const KEYBOARD_PRIMARY: LazyLock<colorgrad::Color> =
//     LazyLock::new(|| colorgrad::Color::from_oklaba(0.5, 0.0, -0.2, 1.0));
const KEYBOARD_OUTLINES: LazyLock<colorgrad::Color> =
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
