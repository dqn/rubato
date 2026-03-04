// CPU-side pixel buffer (RGBA Vec<u8>) with drawing operations.
// Drop-in replacement for the Pixmap stub in rendering_stubs.rs.

use crate::color::Color;

/// Pixel format enum matching rendering_stubs::PixmapFormat.
#[derive(Clone, Debug)]
pub enum PixmapFormat {
    RGBA8888,
    RGB888,
    Alpha,
}

/// CPU-side RGBA8888 pixel buffer with drawing operations.
/// Replaces the stub Pixmap from rendering_stubs.rs.
#[derive(Clone, Debug)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    pub(crate) data: Vec<u8>,
    current_color: [u8; 4],
}

impl Default for Pixmap {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            data: Vec::new(),
            current_color: [255, 255, 255, 255],
        }
    }
}

impl Pixmap {
    pub fn new(width: i32, height: i32, _format: PixmapFormat) -> Self {
        let w = width.max(0) as usize;
        let h = height.max(0) as usize;
        Self {
            width,
            height,
            data: vec![0u8; w * h * 4],
            current_color: [255, 255, 255, 255],
        }
    }

    /// Load an image file and convert to RGBA8888 Pixmap.
    /// Supports PNG, JPEG, BMP, GIF, and other formats via the `image` crate.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("Failed to load image {}: {}", path, e))?;
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        Ok(Self {
            width: w as i32,
            height: h as i32,
            data: rgba.into_raw(),
            current_color: [255, 255, 255, 255],
        })
    }

    /// Create a Pixmap from raw RGBA8888 data.
    pub fn from_rgba_data(width: i32, height: i32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
            current_color: [255, 255, 255, 255],
        }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    /// Blit source pixmap region into this pixmap with scaling.
    /// Corresponds to Pixmap.drawPixmap(src, sx, sy, sw, sh, dx, dy, dw, dh).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_pixmap(
        &mut self,
        src: &Pixmap,
        sx: i32,
        sy: i32,
        sw: i32,
        sh: i32,
        dx: i32,
        dy: i32,
        dw: i32,
        dh: i32,
    ) {
        if sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0 {
            return;
        }
        for dest_y in 0..dh {
            for dest_x in 0..dw {
                let px = dx + dest_x;
                let py = dy + dest_y;
                // Map destination pixel back to source
                let src_x = sx + (dest_x * sw) / dw;
                let src_y = sy + (dest_y * sh) / dh;
                if src_x >= 0
                    && src_x < src.width
                    && src_y >= 0
                    && src_y < src.height
                    && px >= 0
                    && px < self.width
                    && py >= 0
                    && py < self.height
                {
                    let si = ((src_y as usize) * (src.width as usize) + (src_x as usize)) * 4;
                    let di = ((py as usize) * (self.width as usize) + (px as usize)) * 4;
                    if si + 3 < src.data.len() && di + 3 < self.data.len() {
                        // Simple alpha-over compositing
                        let sa = src.data[si + 3] as u32;
                        if sa == 255 {
                            self.data[di..di + 4].copy_from_slice(&src.data[si..si + 4]);
                        } else if sa > 0 {
                            let da = self.data[di + 3] as u32;
                            let out_a = sa + da * (255 - sa) / 255;
                            if out_a > 0 {
                                for c in 0..3 {
                                    let sc = src.data[si + c] as u32;
                                    let dc = self.data[di + c] as u32;
                                    self.data[di + c] =
                                        ((sc * sa + dc * da * (255 - sa) / 255) / out_a) as u8;
                                }
                                self.data[di + 3] = out_a as u8;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.current_color = [
            (r.clamp(0.0, 1.0) * 255.0) as u8,
            (g.clamp(0.0, 1.0) * 255.0) as u8,
            (b.clamp(0.0, 1.0) * 255.0) as u8,
            (a.clamp(0.0, 1.0) * 255.0) as u8,
        ];
    }

    pub fn set_color(&mut self, color: &Color) {
        self.set_color_rgba(color.r, color.g, color.b, color.a);
    }

    /// Set color from packed RGBA8888 integer.
    pub fn set_color_int(&mut self, color: i32) {
        self.current_color = [
            ((color >> 24) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        ];
    }

    /// Fill entire pixmap with current color.
    pub fn fill(&mut self) {
        let w = self.width;
        let h = self.height;
        self.fill_rectangle(0, 0, w, h);
    }

    /// Fill a rectangle with current color.
    pub fn fill_rectangle(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + width) as usize).min(self.width as usize);
        let y1 = ((y + height) as usize).min(self.height as usize);
        let w = self.width as usize;
        for py in y0..y1 {
            for px in x0..x1 {
                let idx = (py * w + px) * 4;
                if idx + 3 < self.data.len() {
                    self.data[idx..idx + 4].copy_from_slice(&self.current_color);
                }
            }
        }
    }

    /// Draw a line using Bresenham's algorithm with current color.
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let mut x = x1;
        let mut y = y1;
        let dx = (x2 - x1).abs();
        let dy = -(y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set_pixel(x, y);
            if x == x2 && y == y2 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                if x == x2 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y2 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a single pixel with a packed RGBA8888 color value.
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: i32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        if idx + 3 < self.data.len() {
            self.data[idx] = ((color >> 24) & 0xFF) as u8;
            self.data[idx + 1] = ((color >> 16) & 0xFF) as u8;
            self.data[idx + 2] = ((color >> 8) & 0xFF) as u8;
            self.data[idx + 3] = (color & 0xFF) as u8;
        }
    }

    /// Get pixel as packed RGBA8888 integer.
    pub fn get_pixel(&self, x: i32, y: i32) -> i32 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 0;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        if idx + 3 < self.data.len() {
            ((self.data[idx] as i32) << 24)
                | ((self.data[idx + 1] as i32) << 16)
                | ((self.data[idx + 2] as i32) << 8)
                | (self.data[idx + 3] as i32)
        } else {
            0
        }
    }

    /// Fill a triangle using scanline rasterization.
    pub fn fill_triangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) {
        // Sort vertices by y
        let mut verts = [(x1, y1), (x2, y2), (x3, y3)];
        verts.sort_by_key(|v| v.1);
        let (ax, ay) = verts[0];
        let (bx, by) = verts[1];
        let (cx, cy) = verts[2];

        if ay == cy {
            // Degenerate triangle
            return;
        }

        for y in ay..=cy {
            let mut left;
            let mut right;
            if y < by {
                // Upper half
                left = lerp_x(ax, ay, bx, by, y);
                right = lerp_x(ax, ay, cx, cy, y);
            } else {
                // Lower half
                left = lerp_x(bx, by, cx, cy, y);
                right = lerp_x(ax, ay, cx, cy, y);
            }
            if left > right {
                std::mem::swap(&mut left, &mut right);
            }
            for x in left..=right {
                self.set_pixel(x, y);
            }
        }
    }

    pub fn dispose(&mut self) {
        self.data.clear();
        self.width = 0;
        self.height = 0;
    }

    /// Get raw RGBA data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Set a pixel at (x, y) to the current color.
    fn set_pixel(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        if idx + 3 < self.data.len() {
            self.data[idx..idx + 4].copy_from_slice(&self.current_color);
        }
    }
}

/// Interpolate x for a scanline at y along an edge from (x0,y0) to (x1,y1).
fn lerp_x(x0: i32, y0: i32, x1: i32, y1: i32, y: i32) -> i32 {
    if y1 == y0 {
        return x0;
    }
    x0 + (x1 - x0) * (y - y0) / (y1 - y0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pixmap_is_transparent() {
        let p = Pixmap::new(4, 4, PixmapFormat::RGBA8888);
        assert_eq!(p.get_pixel(0, 0), 0);
        assert_eq!(p.get_pixel(3, 3), 0);
    }

    #[test]
    fn test_fill_rectangle() {
        let mut p = Pixmap::new(4, 4, PixmapFormat::RGBA8888);
        p.set_color_rgba(1.0, 0.0, 0.0, 1.0);
        p.fill_rectangle(1, 1, 2, 2);
        // Outside fill area should be 0
        assert_eq!(p.get_pixel(0, 0), 0);
        // Inside fill area should be red (0xFF0000FF)
        let pixel = p.get_pixel(1, 1);
        assert_eq!(pixel, 0xFF0000FFu32 as i32);
    }

    #[test]
    fn test_draw_pixel_and_get_pixel() {
        let mut p = Pixmap::new(4, 4, PixmapFormat::RGBA8888);
        let color = 0x12345678u32 as i32;
        p.draw_pixel(2, 3, color);
        assert_eq!(p.get_pixel(2, 3), color);
    }

    #[test]
    fn test_out_of_bounds_returns_zero() {
        let p = Pixmap::new(4, 4, PixmapFormat::RGBA8888);
        assert_eq!(p.get_pixel(-1, 0), 0);
        assert_eq!(p.get_pixel(0, -1), 0);
        assert_eq!(p.get_pixel(4, 0), 0);
        assert_eq!(p.get_pixel(0, 4), 0);
    }

    #[test]
    fn test_fill_entire() {
        let mut p = Pixmap::new(2, 2, PixmapFormat::RGBA8888);
        p.set_color_rgba(0.0, 1.0, 0.0, 1.0); // green
        p.fill();
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(p.get_pixel(x, y), 0x00FF00FFu32 as i32);
            }
        }
    }

    #[test]
    fn test_set_color_int() {
        let mut p = Pixmap::new(2, 2, PixmapFormat::RGBA8888);
        let color = 0xAABBCCDDu32 as i32;
        p.set_color_int(color);
        p.fill();
        assert_eq!(p.get_pixel(0, 0), color);
    }
}
