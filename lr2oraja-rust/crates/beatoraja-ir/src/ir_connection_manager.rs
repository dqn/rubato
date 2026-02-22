use std::sync::OnceLock;

use log::error;

use crate::ir_connection::IRConnection;

/// Registry entry for an IR connection implementation
pub struct IRConnectionEntry {
    pub name: String,
    pub home: Option<String>,
    pub factory: Box<dyn Fn() -> Box<dyn IRConnection + Send + Sync> + Send + Sync>,
}

/// IR connection manager
///
/// Translated from: IRConnectionManager.java
///
/// In Java, this uses reflection and classpath scanning to discover IRConnection
/// implementations. In Rust, we use a manual registry since there's no reflection.
/// Implementations register themselves via `register_ir_connection`.
static IR_CONNECTIONS: OnceLock<Vec<IRConnectionEntry>> = OnceLock::new();

pub struct IRConnectionManager;

impl IRConnectionManager {
    /// Get all available IR connection names
    pub fn get_all_available_ir_connection_name() -> Vec<String> {
        let entries = Self::get_all_available_ir_connection();
        entries.iter().map(|e| e.name.clone()).collect()
    }

    /// Get an IRConnection instance by name. Returns None if not found.
    pub fn get_ir_connection(name: &str) -> Option<Box<dyn IRConnection + Send + Sync>> {
        if name.is_empty() {
            return None;
        }
        let entries = Self::get_all_available_ir_connection();
        for entry in entries {
            if entry.name == name {
                return Some((entry.factory)());
            }
        }
        None
    }

    /// Get the home URL for an IR by name. Returns None if not found.
    pub fn get_home_url(name: &str) -> Option<String> {
        let entries = Self::get_all_available_ir_connection();
        for entry in entries {
            if entry.name == name {
                return entry.home.clone();
            }
        }
        None
    }

    fn get_all_available_ir_connection() -> &'static Vec<IRConnectionEntry> {
        IR_CONNECTIONS.get_or_init(|| {
            let mut connections = Vec::new();
            // In Java, this scans classpath/JAR files for IRConnection implementations.
            // In Rust, implementations must be registered manually.
            // The LR2IRConnection is not a true IRConnection in Java either,
            // so we don't register it here.
            match Self::discover_ir_connections() {
                Ok(discovered) => connections.extend(discovered),
                Err(e) => {
                    error!("Failed to load ir connections: {}", e);
                }
            }
            connections
        })
    }

    /// Discover IR connections. In Rust, this is a no-op placeholder.
    /// Real implementations should call `register_ir_connection` to add entries.
    fn discover_ir_connections() -> anyhow::Result<Vec<IRConnectionEntry>> {
        // In Java, this scans:
        // 1. ClassPath for classes implementing IRConnection with a NAME field
        // 2. Custom directory JAR files
        // In Rust, we return empty and rely on manual registration.
        Ok(Vec::new())
    }
}

/// Register an IR connection implementation.
/// This must be called before `IRConnectionManager` methods are used.
///
/// Note: Due to OnceLock, registrations after first access will be ignored.
/// All registrations should happen at startup.
pub fn register_ir_connections(entries: Vec<IRConnectionEntry>) {
    let _ = IR_CONNECTIONS.set(entries);
}
