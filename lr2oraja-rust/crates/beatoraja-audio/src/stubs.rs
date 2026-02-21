// Config types re-exported from beatoraja-types
pub use beatoraja_types::audio_config::AudioConfig;
pub use beatoraja_types::config::Config;

/// Stub for bms.player.beatoraja.ResourcePool<K, V>
pub struct ResourcePool<K, V> {
    maxgen: i32,
    resources: std::collections::HashMap<u64, ResourceEntry<V>>,
    _phantom: std::marker::PhantomData<K>,
}

struct ResourceEntry<V> {
    resource: V,
    generation: i32,
}

impl<K: std::hash::Hash + Eq, V> ResourcePool<K, V> {
    pub fn new(maxgen: i32) -> Self {
        ResourcePool {
            maxgen,
            resources: std::collections::HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn size(&self) -> usize {
        self.resources.len()
    }

    pub fn dispose_old(&mut self) {
        // Stub: remove old resources
        self.resources.clear();
    }
}

/// Stub for PerformanceMetrics
pub struct PerformanceMetrics;

impl PerformanceMetrics {
    pub fn get() -> Self {
        PerformanceMetrics
    }

    #[allow(non_snake_case)]
    pub fn Event(&self, _name: &str) -> PerformanceEvent {
        PerformanceEvent
    }
}

pub struct PerformanceEvent;

impl Drop for PerformanceEvent {
    fn drop(&mut self) {}
}
