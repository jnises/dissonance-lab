
/// Convert a color from colorgrad to egui's Color32
pub fn colorgrad_to_egui(color: colorgrad::Color) -> egui::Color32 {
    let [r, g, b, a] = color.to_rgba8();
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn oklab(l: f32, a: f32, b: f32, alpha: f32) -> egui::Color32 {
    colorgrad_to_egui(colorgrad::Color::from_oklaba(l, a, b, alpha))
}

/// Convert a color from colorous to egui's Color32
pub fn colorous_to_egui(color: colorous::Color) -> egui::Color32 {
    let (r, g, b) = color.into_tuple();
    egui::Color32::from_rgb(r, g, b)
}