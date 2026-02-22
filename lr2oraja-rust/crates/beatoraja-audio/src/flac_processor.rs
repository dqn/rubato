use std::io::Write;

/// FLAC Processor - writes decoded PCM data to an output stream.
///
/// Translated from: FlacProcessor.java
/// This is a tiny wrapper that implements a PCM processing callback,
/// writing decoded FLAC PCM data to the provided output writer.
pub struct FlacProcessor {
    output: Box<dyn Write>,
}

impl FlacProcessor {
    pub fn new(output: Box<dyn Write>) -> Self {
        FlacProcessor { output }
    }

    /// Processes stream info metadata (no-op in Java).
    ///
    /// Translated from: FlacProcessor.processStreamInfo(StreamInfo)
    pub fn process_stream_info(&mut self) {
        // Empty in Java
    }

    /// Writes PCM data to the output stream.
    ///
    /// Translated from: FlacProcessor.processPCM(ByteData)
    pub fn process_pcm(&mut self, data: &[u8]) {
        if let Err(e) = self.output.write_all(data) {
            log::error!("FlacProcessor: failed to write PCM data: {}", e);
        }
    }
}
