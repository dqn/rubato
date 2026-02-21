// Stub types for Phase 4 dependencies (Config, ResourcePool, etc.)

/// Stub for bms.player.beatoraja.Config
pub struct Config {
    pub song_resource_gen: i32,
    audio_config: AudioConfig,
}

impl Config {
    pub fn get_song_resource_gen(&self) -> i32 {
        self.song_resource_gen
    }

    pub fn get_audio_config(&self) -> &AudioConfig {
        &self.audio_config
    }
}

/// Stub for AudioConfig
pub struct AudioConfig {
    normalize_volume: bool,
    driver_name: String,
    sample_rate: i32,
    device_buffer_size: i32,
    device_simultaneous_sources: i32,
}

impl AudioConfig {
    pub fn is_normalize_volume(&self) -> bool {
        self.normalize_volume
    }

    pub fn get_driver_name(&self) -> &str {
        &self.driver_name
    }

    pub fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    pub fn get_device_buffer_size(&self) -> i32 {
        self.device_buffer_size
    }

    pub fn get_device_simultaneous_sources(&self) -> i32 {
        self.device_simultaneous_sources
    }
}

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
