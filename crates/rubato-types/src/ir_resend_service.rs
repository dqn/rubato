/// Service that manages the IR score resend background thread.
///
/// The concrete implementation lives in beatoraja-result where IRSendStatusMain
/// and the resend loop are defined. MainController holds this as a trait object
/// to avoid depending on beatoraja-result.
///
/// Translated from: MainController.java lines 518-548 (daemon thread)
pub trait IrResendService: Send + Sync {
    /// Start the background retry thread.
    fn start(&self);

    /// Stop the background retry thread.
    fn stop(&self);
}
