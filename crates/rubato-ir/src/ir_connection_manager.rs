use std::sync::Mutex;

use log::{error, warn};

use crate::ir_connection::IRConnection;
use rubato_types::sync_utils::lock_or_recover;

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
/// Implementations register themselves via `register_ir_connections`.
///
/// Uses Mutex instead of OnceLock so that connections can be registered at any
/// time (including after first access). This matches the Java pattern where
/// classpath scanning can discover JARs added at runtime.
static IR_CONNECTIONS: Mutex<Vec<IRConnectionEntry>> = Mutex::new(Vec::new());

pub struct IRConnectionManager;

impl IRConnectionManager {
    /// Get all available IR connection names
    pub fn all_available_ir_connection_name() -> Vec<String> {
        let entries = lock_or_recover(&IR_CONNECTIONS);
        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        if names.is_empty() {
            warn!("No IR connections registered. IR features are disabled.");
        }
        names
    }

    /// Get an IRConnection instance by name. Returns None if not found.
    pub fn ir_connection(name: &str) -> Option<Box<dyn IRConnection + Send + Sync>> {
        if name.is_empty() {
            return None;
        }
        let entries = lock_or_recover(&IR_CONNECTIONS);
        for entry in entries.iter() {
            if entry.name == name {
                return Some((entry.factory)());
            }
        }
        None
    }

    /// Get the home URL for an IR by name. Returns None if not found.
    pub fn home_url(name: &str) -> Option<String> {
        let entries = lock_or_recover(&IR_CONNECTIONS);
        for entry in entries.iter() {
            if entry.name == name {
                return entry.home.clone();
            }
        }
        None
    }
}

/// Register IR connection implementations.
///
/// Can be called at any time, including after `IRConnectionManager` methods
/// have already been used. New entries are appended to the existing registry.
///
/// Duplicate names are allowed (first match wins in lookups).
/// No warning is logged for duplicates; current callers never register the same name twice.
pub fn register_ir_connections(entries: Vec<IRConnectionEntry>) {
    match IR_CONNECTIONS.lock() {
        Ok(mut connections) => {
            for entry in &entries {
                log::info!("Registering IR connection: {}", entry.name);
            }
            connections.extend(entries);
        }
        Err(e) => {
            error!("Failed to register IR connections (lock poisoned): {}", e);
        }
    }
}
