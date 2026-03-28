/// Trait interface for ImGui overlay renderer access.
///
/// Downstream crates use `Box<dyn ImGuiAccess>` instead of concrete ImGuiRenderer.
/// The real implementation is in beatoraja-modmenu.
pub trait ImGuiAccess: Send {
    /// Toggle the mod menu overlay visibility.
    fn toggle_menu(&mut self);

    /// Dispose resources.
    fn dispose(&mut self) {}
}
