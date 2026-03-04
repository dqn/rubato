// Blend mode management for sprite rendering.
// Maps LibGDX GL blend functions to logical blend modes.

/// Logical blend modes corresponding to LibGDX GL blend function combinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// Normal alpha blending: src=SRC_ALPHA, dst=ONE_MINUS_SRC_ALPHA
    #[default]
    Normal,
    /// Additive blending: src=SRC_ALPHA, dst=ONE
    Additive,
    /// Subtractive blending (via GL_FUNC_SUBTRACT): src=ZERO, dst=SRC_COLOR
    Subtractive,
    /// Multiply blending: src=ZERO, dst=SRC_COLOR
    Multiply,
    /// Inversion blending: src=ONE_MINUS_DST_COLOR, dst=ZERO
    Inversion,
}

// GL constants matching rendering_stubs gl11/gl20 modules
pub mod gl11 {
    pub const GL_SRC_ALPHA: i32 = 0x0302;
    pub const GL_ONE: i32 = 1;
    pub const GL_ONE_MINUS_SRC_ALPHA: i32 = 0x0303;
    pub const GL_ZERO: i32 = 0;
    pub const GL_SRC_COLOR: i32 = 0x0300;
    pub const GL_ONE_MINUS_DST_COLOR: i32 = 0x0307;
}

pub mod gl20 {
    pub const GL_FUNC_ADD: i32 = 0x8006;
    pub const GL_FUNC_SUBTRACT: i32 = 0x800A;
}

impl BlendMode {
    /// Determine blend mode from GL src/dst blend factors.
    pub fn from_gl_factors(src: i32, dst: i32) -> Self {
        match (src, dst) {
            (0x0302, 0x0303) => BlendMode::Normal, // SRC_ALPHA, ONE_MINUS_SRC_ALPHA
            (0x0302, 1) => BlendMode::Additive,    // SRC_ALPHA, ONE
            (0, 0x0300) => BlendMode::Multiply,    // ZERO, SRC_COLOR (also subtractive context)
            (0x0307, 0) => BlendMode::Inversion,   // ONE_MINUS_DST_COLOR, ZERO
            _ => BlendMode::Normal,
        }
    }

    /// Get the wgpu blend state for this blend mode.
    pub fn to_wgpu_blend_state(self) -> wgpu::BlendState {
        match self {
            BlendMode::Normal => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            BlendMode::Additive => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            BlendMode::Subtractive => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::Src,
                    operation: wgpu::BlendOperation::ReverseSubtract,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            BlendMode::Multiply => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::Src,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::SrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            BlendMode::Inversion => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::OneMinusDst,
                    dst_factor: wgpu::BlendFactor::Zero,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::Zero,
                    operation: wgpu::BlendOperation::Add,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_mode_default() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }

    #[test]
    fn test_blend_mode_from_gl_factors_normal() {
        // Java: SRC_ALPHA (0x0302), ONE_MINUS_SRC_ALPHA (0x0303)
        assert_eq!(
            BlendMode::from_gl_factors(0x0302, 0x0303),
            BlendMode::Normal
        );
    }

    #[test]
    fn test_blend_mode_from_gl_factors_additive() {
        // Java: SRC_ALPHA (0x0302), ONE (1)
        assert_eq!(BlendMode::from_gl_factors(0x0302, 1), BlendMode::Additive);
    }

    #[test]
    fn test_blend_mode_from_gl_factors_multiply() {
        // Java: ZERO (0), SRC_COLOR (0x0300)
        assert_eq!(BlendMode::from_gl_factors(0, 0x0300), BlendMode::Multiply);
    }

    #[test]
    fn test_blend_mode_from_gl_factors_inversion() {
        // Java: ONE_MINUS_DST_COLOR (0x0307), ZERO (0)
        assert_eq!(BlendMode::from_gl_factors(0x0307, 0), BlendMode::Inversion);
    }

    #[test]
    fn test_blend_mode_from_gl_factors_unknown_defaults_normal() {
        assert_eq!(
            BlendMode::from_gl_factors(0x9999, 0x9999),
            BlendMode::Normal
        );
    }

    #[test]
    fn test_blend_state_normal_has_src_alpha() {
        let state = BlendMode::Normal.to_wgpu_blend_state();
        assert_eq!(state.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(state.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
        assert_eq!(state.color.operation, wgpu::BlendOperation::Add);
    }

    #[test]
    fn test_blend_state_additive_has_one_dst() {
        let state = BlendMode::Additive.to_wgpu_blend_state();
        assert_eq!(state.color.src_factor, wgpu::BlendFactor::SrcAlpha);
        assert_eq!(state.color.dst_factor, wgpu::BlendFactor::One);
    }

    #[test]
    fn test_blend_state_subtractive_has_reverse_subtract() {
        let state = BlendMode::Subtractive.to_wgpu_blend_state();
        assert_eq!(state.color.operation, wgpu::BlendOperation::ReverseSubtract);
    }
}
