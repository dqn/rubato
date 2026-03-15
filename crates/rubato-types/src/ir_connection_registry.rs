/// Rust equivalent of beatoraja.ir.IRConnectionManager
///
/// Will be turned into a trait in Phase 6.
pub struct IRConnectionManager;

impl IRConnectionManager {
    pub fn all_available_ir_connection_name() -> Vec<String> {
        vec![]
    }

    pub fn ir_connection_class(_name: &str) -> Option<()> {
        Some(())
    }
}
