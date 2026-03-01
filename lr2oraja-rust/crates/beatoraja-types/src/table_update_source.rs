/// Trait for table update operations.
/// Used by `MainControllerAccess::update_table()` to pass table accessor
/// across crate boundaries without importing `TableAccessor` from beatoraja-core.
pub trait TableUpdateSource: Send + Sync {
    /// Name of the table source (for logging)
    fn source_name(&self) -> String;
    /// Perform read + write cycle (read from remote, write to local cache)
    fn refresh(&self);
}
