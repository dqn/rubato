/// Surrogate for ImGui ImFloat -- a plain f32 wrapper used in static Mutex statics.
pub struct ImFloat {
    pub value: f32,
}

/// Surrogate for ImGui ImBoolean -- a plain bool wrapper used in static Mutex statics.
pub struct ImBoolean {
    pub value: bool,
}
