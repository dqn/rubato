// DisplayMode: graphics display mode information (resolution + refresh rate + color depth).

/// Graphics display mode information (resolution + refresh rate + color depth).
///
/// Translated from: com.badlogic.gdx.Graphics.DisplayMode
/// Java fields: width, height, refreshRate, bitsPerPixel
///
/// Populated from winit's VideoMode via `update_monitors_from_winit()`.
/// The `refresh_rate_millihertz` and `bit_depth` fields are used when
/// selecting the best fullscreen mode (Java: highest refreshRate and bitsPerPixel).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: i32,
    pub height: i32,
    /// Refresh rate in millihertz (e.g. 60000 = 60 Hz).
    /// Java: Graphics.DisplayMode.refreshRate (in Hz) -- we store millihertz for precision.
    pub refresh_rate_millihertz: u32,
    /// Color depth in bits per pixel (e.g. 32).
    /// Java: Graphics.DisplayMode.bitsPerPixel
    pub bit_depth: u16,
}

impl DisplayMode {
    /// Get refresh rate in Hz (Java: Graphics.DisplayMode.refreshRate).
    /// Converts from millihertz to Hz.
    pub fn refresh_rate_hz(&self) -> u32 {
        self.refresh_rate_millihertz / 1000
    }
}

impl std::fmt::Display for DisplayMode {
    /// Display format matching Java's logging:
    /// "w - {width} h - {height} refresh - {refreshRate} color bit - {bitsPerPixel}"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.refresh_rate_millihertz > 0 {
            write!(
                f,
                "{}x{} @{}Hz {}bpp",
                self.width,
                self.height,
                self.refresh_rate_hz(),
                self.bit_depth
            )
        } else {
            write!(f, "{}x{}", self.width, self.height)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mode_default() {
        let dm = DisplayMode::default();
        assert_eq!(dm.width, 0);
        assert_eq!(dm.height, 0);
        assert_eq!(dm.refresh_rate_millihertz, 0);
        assert_eq!(dm.bit_depth, 0);
    }

    #[test]
    fn display_mode_refresh_rate_hz() {
        let dm = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_rate_millihertz: 60000,
            bit_depth: 32,
        };
        assert_eq!(dm.refresh_rate_hz(), 60);

        let dm_144 = DisplayMode {
            refresh_rate_millihertz: 144000,
            ..dm
        };
        assert_eq!(dm_144.refresh_rate_hz(), 144);
    }

    #[test]
    fn display_mode_display_with_refresh() {
        let dm = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_rate_millihertz: 60000,
            bit_depth: 32,
        };
        assert_eq!(format!("{}", dm), "1920x1080 @60Hz 32bpp");
    }

    #[test]
    fn display_mode_display_without_refresh() {
        let dm = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_rate_millihertz: 0,
            bit_depth: 0,
        };
        assert_eq!(format!("{}", dm), "1920x1080");
    }

    #[test]
    fn display_mode_equality() {
        let dm1 = DisplayMode {
            width: 1920,
            height: 1080,
            refresh_rate_millihertz: 60000,
            bit_depth: 32,
        };
        let dm2 = dm1;
        assert_eq!(dm1, dm2);

        let dm3 = DisplayMode {
            refresh_rate_millihertz: 144000,
            ..dm1
        };
        assert_ne!(dm1, dm3);
    }
}
