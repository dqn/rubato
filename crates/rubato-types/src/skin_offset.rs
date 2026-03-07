/// SkinObject.SkinOffset — shared offset applied to skin objects.
///
/// Translated from Java: bms.player.beatoraja.skin.SkinObject.SkinOffset
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SkinOffset {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub a: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_offset_default() {
        let offset = SkinOffset::default();
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, 0.0);
        assert_eq!(offset.w, 0.0);
        assert_eq!(offset.h, 0.0);
        assert_eq!(offset.r, 0.0);
        assert_eq!(offset.a, 0.0);
    }

    #[test]
    fn test_skin_offset_serde() {
        let offset = SkinOffset {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
            r: 5.0,
            a: 0.5,
        };
        let json = serde_json::to_string(&offset).unwrap();
        let deserialized: SkinOffset = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.x, 1.0);
        assert_eq!(deserialized.a, 0.5);
    }
}
