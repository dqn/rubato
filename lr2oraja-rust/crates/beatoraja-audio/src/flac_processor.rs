/// FLAC Processor (stub - depends on jflac equivalent).
///
/// Translated from: FlacProcessor.java
/// This is a tiny wrapper that just passes decoded PCM data to an output stream.
/// In Rust, this will be replaced by a proper FLAC decoder library.
pub struct FlacProcessor {
    // stub
}

impl FlacProcessor {
    pub fn new() -> Self {
        FlacProcessor {}
    }
}

impl Default for FlacProcessor {
    fn default() -> Self {
        Self::new()
    }
}
