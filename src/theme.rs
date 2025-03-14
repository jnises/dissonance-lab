pub fn setup_custom_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Define cool theme colors
    let primary_accent = oklab(0.7, 0.1, 0.3, 1.0); // Purple-ish accent
    let secondary_accent = oklab(0.6, -0.2, 0.3, 1.0); // Teal-ish accent
    let background = oklab(0.15, -0.03, -0.05, 1.0); // Deep dark blue-ish background
    let panel_bg = oklab(0.19, -0.01, -0.03, 1.0); // Slightly lighter panel background

    // Update visuals with our custom colors
    visuals.selection.bg_fill = primary_accent;
    visuals.selection.stroke.color = secondary_accent;
    visuals.widgets.noninteractive.bg_fill = panel_bg;
    visuals.widgets.inactive.bg_fill = panel_bg;
    visuals.widgets.active.bg_fill = oklab(0.22, -0.01, -0.03, 1.0);
    visuals.widgets.hovered.bg_fill = oklab(0.25, 0.0, -0.02, 1.0);

    // Customize window and panel backgrounds
    visuals.window_fill = background;
    visuals.panel_fill = panel_bg;

    // Update stroke colors for better visibility
    visuals.widgets.noninteractive.fg_stroke.color = oklab(0.75, 0.0, 0.0, 1.0);
    visuals.widgets.inactive.fg_stroke.color = oklab(0.65, 0.0, 0.0, 1.0);
    visuals.widgets.active.fg_stroke.color = primary_accent;
    visuals.widgets.hovered.fg_stroke.color = secondary_accent;

    // Add a subtle glow effect to windows
    //visuals.window_shadow.extrusion = 8.0;
    visuals.window_shadow.color = oklab(0.1, 0.1, 0.2, 0.4);

    // Set the custom visuals
    ctx.set_visuals(visuals);
}

/// Convert a color from colorgrad to egui's Color32
fn colorgrad_to_egui(color: colorgrad::Color) -> egui::Color32 {
    let [r, g, b, a] = color.to_rgba8();
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn oklab(l: f32, a: f32, b: f32, alpha: f32) -> egui::Color32 {
    colorgrad_to_egui(colorgrad::Color::from_oklaba(l, a, b, alpha))
}
