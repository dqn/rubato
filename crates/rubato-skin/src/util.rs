use rubato_render::color::Color;

/// Helper: parse hex color string ("RRGGBBAA" or "RRGGBB").
///
/// Originally in json_skin_object_loader/utilities.rs, moved here so both
/// loader and render code can reach it without cross-boundary imports.
pub fn parse_hex_color(hex: &str, fallback: Color) -> Color {
    // Simple hex color parser: "RRGGBBAA" or "RRGGBB"
    if hex.len() >= 6 && hex.is_ascii() {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f32 / 255.0;
        let a = if hex.len() >= 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
        } else {
            1.0
        };
        Color::new(r, g, b, a)
    } else {
        fallback
    }
}
