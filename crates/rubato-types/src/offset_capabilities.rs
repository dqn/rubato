/// Describes which offset dimensions (x, y, w, h, r, a) are enabled
/// for a custom skin offset entry.
///
/// Replaces the 6 positional boolean parameters that previously appeared
/// in `CustomOffset::new`, `CustomOffsetData`, and `CustomOffsetDef`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OffsetCapabilities {
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
}

impl OffsetCapabilities {
    /// All capabilities enabled.
    pub fn all() -> Self {
        Self {
            x: true,
            y: true,
            w: true,
            h: true,
            r: true,
            a: true,
        }
    }

    /// Only position (x, y) enabled.
    pub fn position_only() -> Self {
        Self {
            x: true,
            y: true,
            ..Default::default()
        }
    }
}
