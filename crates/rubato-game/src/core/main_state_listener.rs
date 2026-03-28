/// MainStateListener - re-exported from beatoraja-types for unified API.
///
/// Previously defined locally with `&dyn MainState`, now uses `&dyn MainStateAccess`
/// from beatoraja-types so that external listeners (DiscordListener, ObsListener)
/// and core listeners share the same trait.
///
/// **Deprecated**: Use `AppEvent` channel via `MainController::add_event_sender()` instead.
#[allow(deprecated)]
pub use crate::main_state_access::MainStateListener;
