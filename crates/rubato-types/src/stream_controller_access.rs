/// Trait for stream controller access across crate boundaries.
///
/// The concrete implementation lives in beatoraja-stream where the Windows
/// named pipe listener is implemented. MainController holds this as a trait
/// object to avoid depending on beatoraja-stream.
///
/// Translated from: bms.player.beatoraja.stream.StreamController
pub trait StreamControllerAccess: Send {
    /// Start the pipe polling thread.
    fn run(&mut self);

    /// Dispose of resources and stop the polling thread.
    fn dispose(&mut self);
}
