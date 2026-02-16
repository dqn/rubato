use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use crate::connection::IRConnection;
use crate::lr2ir::LR2IRConnection;

/// Registry entry for an IR connection.
struct IrEntry {
    factory: fn() -> Box<dyn IRConnection>,
    home_url: Option<&'static str>,
}

/// Global IR connection registry.
///
/// Pre-populated with LR2IR. Additional IR implementations can be
/// registered at runtime via `IRConnectionManager::register()`.
static REGISTRY: LazyLock<RwLock<HashMap<&'static str, IrEntry>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "LR2IR",
        IrEntry {
            factory: || Box::new(LR2IRConnection::new()),
            home_url: Some("http://dream-pro.info/~lavalse/LR2IR/2"),
        },
    );
    RwLock::new(map)
});

/// IR connection manager.
///
/// Corresponds to Java `IRConnectionManager`.
/// Uses a dynamic registry pattern so that external IR plugins can be registered at runtime.
pub struct IRConnectionManager;

impl IRConnectionManager {
    /// Get all available IR connection names.
    pub fn available_names() -> Vec<String> {
        let registry = REGISTRY.read().unwrap();
        registry.keys().map(|k| k.to_string()).collect()
    }

    /// Create an IR connection by name.
    pub fn create(name: &str) -> Option<Box<dyn IRConnection>> {
        let registry = REGISTRY.read().unwrap();
        registry.get(name).map(|entry| (entry.factory)())
    }

    /// Get the home URL for an IR by name.
    pub fn home_url(name: &str) -> Option<&'static str> {
        let registry = REGISTRY.read().unwrap();
        registry.get(name).and_then(|entry| entry.home_url)
    }

    /// Register a new IR connection type.
    ///
    /// If an entry with the same name already exists, it is replaced.
    pub fn register(
        name: &'static str,
        factory: fn() -> Box<dyn IRConnection>,
        home_url: Option<&'static str>,
    ) {
        let mut registry = REGISTRY.write().unwrap();
        registry.insert(name, IrEntry { factory, home_url });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_names_contains_lr2ir() {
        let names = IRConnectionManager::available_names();
        assert!(names.iter().any(|n| n == "LR2IR"));
    }

    #[test]
    fn create_lr2ir() {
        let conn = IRConnectionManager::create("LR2IR");
        assert!(conn.is_some());
    }

    #[test]
    fn create_unknown_returns_none() {
        let conn = IRConnectionManager::create("UnknownIR");
        assert!(conn.is_none());
    }

    #[test]
    fn home_url_lr2ir() {
        let url = IRConnectionManager::home_url("LR2IR");
        assert_eq!(url, Some("http://dream-pro.info/~lavalse/LR2IR/2"));
    }

    #[test]
    fn home_url_unknown_returns_none() {
        let url = IRConnectionManager::home_url("UnknownIR");
        assert!(url.is_none());
    }

    #[test]
    fn register_and_create_custom_ir() {
        use anyhow::Result;
        use async_trait::async_trait;

        use crate::chart_data::IRChartData;
        use crate::player_data::IRPlayerData;
        use crate::response::IRResponse;
        use crate::score_data::IRScoreData;

        struct TestIR;

        #[async_trait]
        impl IRConnection for TestIR {
            async fn get_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _chart: &IRChartData,
            ) -> Result<IRResponse<Vec<IRScoreData>>> {
                Ok(IRResponse::success(vec![]))
            }
        }

        IRConnectionManager::register("TestIR", || Box::new(TestIR), Some("https://test.ir"));

        let names = IRConnectionManager::available_names();
        assert!(names.iter().any(|n| n == "TestIR"));

        let conn = IRConnectionManager::create("TestIR");
        assert!(conn.is_some());

        let url = IRConnectionManager::home_url("TestIR");
        assert_eq!(url, Some("https://test.ir"));
    }
}
