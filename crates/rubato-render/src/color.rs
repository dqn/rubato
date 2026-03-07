// Color, Rectangle, Matrix4 — pure data types for rendering.
// Drop-in replacements for the same types in rendering_stubs.rs.

/// RGBA color with float components in [0.0, 1.0].
/// Corresponds to com.badlogic.gdx.graphics.Color.
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl Color {
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const CLEAR: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Parses a hex color string (e.g. "FF0000FF") into a Color.
    /// Corresponds to com.badlogic.gdx.graphics.Color.valueOf(String)
    pub fn value_of(hex: &str) -> Self {
        let hex = hex.trim();
        let len = hex.len();
        if len < 6 {
            return Color::new(1.0, 0.0, 0.0, 1.0); // fallback red
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        let a = if len >= 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
        } else {
            1.0
        };
        Color::new(r, g, b, a)
    }

    /// Alias for `value_of` — parses a hex color string.
    pub fn from_hex(hex: &str) -> Self {
        Self::value_of(hex)
    }

    /// Packs r, g, b, a into an integer (Color.rgba8888 equivalent)
    pub fn rgba8888(r: f32, g: f32, b: f32, a: f32) -> i32 {
        ((255.0 * r) as i32) << 24
            | ((255.0 * g) as i32) << 16
            | ((255.0 * b) as i32) << 8
            | ((255.0 * a) as i32)
    }

    /// Corresponds to com.badlogic.gdx.graphics.Color.toIntBits(a, b, g, r)
    /// Note: LibGDX's toIntBits packs as ABGR
    pub fn to_int_bits(a: i32, b: i32, g: i32, r: i32) -> i32 {
        (a << 24) | (b << 16) | (g << 8) | r
    }

    pub fn set(&mut self, other: &Color) {
        self.r = other.r;
        self.g = other.g;
        self.b = other.b;
        self.a = other.a;
    }

    pub fn set_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }

    pub fn equals(&self, other: &Color) -> bool {
        (self.r - other.r).abs() < f32::EPSILON
            && (self.g - other.g).abs() < f32::EPSILON
            && (self.b - other.b).abs() < f32::EPSILON
            && (self.a - other.a).abs() < f32::EPSILON
    }

    /// Convert to [f32; 4] for GPU uniform/vertex data.
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Axis-aligned rectangle.
/// Corresponds to com.badlogic.gdx.math.Rectangle.
#[derive(Clone, Debug, Default)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rectangle {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn set(&mut self, other: &Rectangle) {
        self.x = other.x;
        self.y = other.y;
        self.width = other.width;
        self.height = other.height;
    }

    pub fn set_xywh(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.x = x;
        self.y = y;
        self.width = w;
        self.height = h;
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= 0.0 && x <= self.width && y >= 0.0 && y <= self.height
    }

    pub fn equals(&self, other: &Rectangle) -> bool {
        (self.x - other.x).abs() < f32::EPSILON
            && (self.y - other.y).abs() < f32::EPSILON
            && (self.width - other.width).abs() < f32::EPSILON
            && (self.height - other.height).abs() < f32::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Color::new ---

    #[test]
    fn test_color_new() {
        let c = Color::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(c.r, 0.1);
        assert_eq!(c.g, 0.2);
        assert_eq!(c.b, 0.3);
        assert_eq!(c.a, 0.4);
    }

    #[test]
    fn test_color_default_is_white() {
        let c = Color::default();
        assert!(c.equals(&Color::WHITE));
    }

    // --- Color::value_of ---

    #[test]
    fn test_value_of_6digit_red() {
        let c = Color::value_of("FF0000");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0); // alpha defaults to 1.0 for 6-digit
    }

    #[test]
    fn test_value_of_6digit_green() {
        let c = Color::value_of("00FF00");
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 1.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_6digit_blue() {
        let c = Color::value_of("0000FF");
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 1.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_8digit_with_alpha() {
        let c = Color::value_of("FF000080");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        // 0x80 = 128, 128/255 ~= 0.50196
        assert!((c.a - 128.0 / 255.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_value_of_8digit_full_alpha() {
        let c = Color::value_of("FF0000FF");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_all_zeros() {
        let c = Color::value_of("000000");
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_all_zeros_8digit() {
        let c = Color::value_of("00000000");
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 0.0);
    }

    #[test]
    fn test_value_of_all_ffs() {
        let c = Color::value_of("FFFFFF");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 1.0);
        assert_eq!(c.b, 1.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_all_ffs_8digit() {
        let c = Color::value_of("FFFFFFFF");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 1.0);
        assert_eq!(c.b, 1.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_short_string_fallback() {
        // Strings shorter than 6 chars return fallback red
        let c = Color::value_of("FFF");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_empty_string_fallback() {
        let c = Color::value_of("");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_value_of_trims_whitespace() {
        let c = Color::value_of("  FF0000  ");
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn test_value_of_mixed_case() {
        let c = Color::value_of("aaBBcc");
        // 0xAA = 170, 0xBB = 187, 0xCC = 204
        assert!((c.r - 170.0 / 255.0).abs() < f32::EPSILON);
        assert!((c.g - 187.0 / 255.0).abs() < f32::EPSILON);
        assert!((c.b - 204.0 / 255.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_from_hex_is_alias_for_value_of() {
        let a = Color::value_of("FF8040");
        let b = Color::from_hex("FF8040");
        assert!(a.equals(&b));
    }

    // --- Color::rgba8888 ---

    #[test]
    fn test_rgba8888_red() {
        let packed = Color::rgba8888(1.0, 0.0, 0.0, 1.0);
        // R=255 << 24 | G=0 << 16 | B=0 << 8 | A=255
        assert_eq!(packed as u32, 0xFF0000FF);
    }

    #[test]
    fn test_rgba8888_green() {
        let packed = Color::rgba8888(0.0, 1.0, 0.0, 1.0);
        assert_eq!(packed as u32, 0x00FF00FF);
    }

    #[test]
    fn test_rgba8888_blue() {
        let packed = Color::rgba8888(0.0, 0.0, 1.0, 1.0);
        assert_eq!(packed as u32, 0x0000FFFF);
    }

    #[test]
    fn test_rgba8888_white_opaque() {
        let packed = Color::rgba8888(1.0, 1.0, 1.0, 1.0);
        assert_eq!(packed as u32, 0xFFFFFFFF);
    }

    #[test]
    fn test_rgba8888_black_transparent() {
        let packed = Color::rgba8888(0.0, 0.0, 0.0, 0.0);
        assert_eq!(packed, 0);
    }

    // --- Color::to_int_bits (ABGR) ---

    #[test]
    fn test_to_int_bits_abgr_layout() {
        // a=255, b=128, g=64, r=32
        let bits = Color::to_int_bits(255, 128, 64, 32);
        assert_eq!(bits as u32, 0xFF804020);
    }

    // --- Roundtrip: Color -> rgba8888 -> verify components ---

    #[test]
    fn test_roundtrip_color_to_packed() {
        let c = Color::new(0.5, 0.25, 0.75, 1.0);
        let packed = Color::rgba8888(c.r, c.g, c.b, c.a) as u32;
        let r = ((packed >> 24) & 0xFF) as f32 / 255.0;
        let g = ((packed >> 16) & 0xFF) as f32 / 255.0;
        let b = ((packed >> 8) & 0xFF) as f32 / 255.0;
        let a = (packed & 0xFF) as f32 / 255.0;
        // Allow tolerance for float->int->float roundtrip
        assert!((c.r - r).abs() < 0.005);
        assert!((c.g - g).abs() < 0.005);
        assert!((c.b - b).abs() < 0.005);
        assert!((c.a - a).abs() < 0.005);
    }

    // --- Color helper methods ---

    #[test]
    fn test_color_set() {
        let mut c = Color::BLACK;
        let src = Color::new(0.1, 0.2, 0.3, 0.4);
        c.set(&src);
        assert!(c.equals(&src));
    }

    #[test]
    fn test_color_set_rgba() {
        let mut c = Color::BLACK;
        c.set_rgba(0.5, 0.6, 0.7, 0.8);
        assert_eq!(c.r, 0.5);
        assert_eq!(c.g, 0.6);
        assert_eq!(c.b, 0.7);
        assert_eq!(c.a, 0.8);
    }

    #[test]
    fn test_color_to_array() {
        let c = Color::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(c.to_array(), [0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn test_color_equals_same() {
        let a = Color::new(0.5, 0.5, 0.5, 0.5);
        let b = Color::new(0.5, 0.5, 0.5, 0.5);
        assert!(a.equals(&b));
    }

    #[test]
    fn test_color_equals_different() {
        let a = Color::new(0.5, 0.5, 0.5, 0.5);
        let b = Color::new(0.5, 0.5, 0.5, 0.6);
        assert!(!a.equals(&b));
    }

    #[test]
    fn test_color_constants() {
        assert!(Color::WHITE.equals(&Color::new(1.0, 1.0, 1.0, 1.0)));
        assert!(Color::BLACK.equals(&Color::new(0.0, 0.0, 0.0, 1.0)));
        assert!(Color::CLEAR.equals(&Color::new(0.0, 0.0, 0.0, 0.0)));
    }
}

/// 4x4 transformation matrix stored column-major.
/// Corresponds to com.badlogic.gdx.math.Matrix4.
#[derive(Clone, Debug)]
pub struct Matrix4 {
    pub values: [f32; 16],
}

impl Default for Matrix4 {
    fn default() -> Self {
        // Identity matrix
        Self {
            values: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

impl Matrix4 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set from translation, quaternion rotation, and scale.
    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &mut self,
        tx: f32,
        ty: f32,
        tz: f32,
        qx: f32,
        qy: f32,
        qz: f32,
        qw: f32,
        sx: f32,
        sy: f32,
        sz: f32,
    ) {
        // Convert quaternion to rotation matrix, apply scale and translation
        let xx = qx * qx;
        let xy = qx * qy;
        let xz = qx * qz;
        let xw = qx * qw;
        let yy = qy * qy;
        let yz = qy * qz;
        let yw = qy * qw;
        let zz = qz * qz;
        let zw = qz * qw;

        // Column-major order (same as LibGDX)
        self.values[0] = sx * (1.0 - 2.0 * (yy + zz));
        self.values[1] = sx * 2.0 * (xy + zw);
        self.values[2] = sx * 2.0 * (xz - yw);
        self.values[3] = 0.0;

        self.values[4] = sy * 2.0 * (xy - zw);
        self.values[5] = sy * (1.0 - 2.0 * (xx + zz));
        self.values[6] = sy * 2.0 * (yz + xw);
        self.values[7] = 0.0;

        self.values[8] = sz * 2.0 * (xz + yw);
        self.values[9] = sz * 2.0 * (yz - xw);
        self.values[10] = sz * (1.0 - 2.0 * (xx + yy));
        self.values[11] = 0.0;

        self.values[12] = tx;
        self.values[13] = ty;
        self.values[14] = tz;
        self.values[15] = 1.0;
    }

    /// Create an orthographic projection matrix.
    pub fn set_to_ortho(
        &mut self,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) {
        let rml = right - left;
        let tmb = top - bottom;
        let fmn = far - near;

        self.values = [0.0; 16];
        self.values[0] = 2.0 / rml;
        self.values[5] = 2.0 / tmb;
        self.values[10] = -2.0 / fmn;
        self.values[12] = -(right + left) / rml;
        self.values[13] = -(top + bottom) / tmb;
        self.values[14] = -(far + near) / fmn;
        self.values[15] = 1.0;
    }
}
