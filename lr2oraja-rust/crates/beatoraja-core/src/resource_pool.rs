use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

/// ResourceCacheElement - wraps a resource with a generation counter
struct ResourceCacheElement<V> {
    /// The resource
    resource: V,
    /// Generation counter
    generation: i32,
}

impl<V> ResourceCacheElement<V> {
    fn new(resource: V) -> Self {
        Self {
            resource,
            generation: 0,
        }
    }
}

/// ResourcePool - pools resources with generation-based eviction.
///
/// Resources are loaded on first access and evicted after `maxgen` generations
/// of non-access. This is a generic resource cache for expensive-to-load,
/// explicitly-disposable resources like images and audio.
///
/// Java uses ConcurrentHashMap; Rust uses Mutex<HashMap> for thread safety.
pub struct ResourcePool<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Maximum generation before eviction
    maxgen: i32,
    /// Resource map
    resource_map: Mutex<HashMap<K, ResourceCacheElement<V>>>,
}

impl<K, V> ResourcePool<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new(maxgen: i32) -> Self {
        Self {
            maxgen,
            resource_map: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if the key exists in the pool
    pub fn exists(&self, key: &K) -> bool {
        let map = self.resource_map.lock().unwrap();
        map.contains_key(key)
    }

    /// Get a resource by key. If not in the pool, calls `load_fn` to load it.
    /// Returns None if the resource couldn't be loaded.
    pub fn get<F>(&self, key: &K, load_fn: F) -> Option<()>
    where
        F: FnOnce(&K) -> Option<V>,
    {
        let mut map = self.resource_map.lock().unwrap();
        if let Some(elem) = map.get_mut(key) {
            elem.generation = 0;
            return Some(());
        }

        if let Some(resource) = load_fn(key) {
            map.insert(key.clone(), ResourceCacheElement::new(resource));
            Some(())
        } else {
            None
        }
    }

    /// Get a reference to the resource by key (if it exists in pool).
    /// Does NOT attempt to load.
    pub fn get_cached(&self, key: &K) -> bool {
        let mut map = self.resource_map.lock().unwrap();
        if let Some(elem) = map.get_mut(key) {
            elem.generation = 0;
            true
        } else {
            false
        }
    }

    /// Advance generation counters and dispose resources that have exceeded maxgen.
    /// `dispose_fn` is called for each evicted resource.
    pub fn dispose_old<F>(&self, mut dispose_fn: F)
    where
        F: FnMut(V),
    {
        let mut map = self.resource_map.lock().unwrap();
        let mut removes = Vec::new();

        for (key, value) in map.iter_mut() {
            if value.generation == self.maxgen {
                removes.push(key.clone());
            } else {
                value.generation += 1;
            }
        }

        for key in removes {
            if let Some(elem) = map.remove(&key) {
                dispose_fn(elem.resource);
            }
        }
    }

    /// Returns the number of resources currently in the pool
    pub fn size(&self) -> usize {
        let map = self.resource_map.lock().unwrap();
        map.len()
    }

    /// Dispose all resources
    pub fn dispose<F>(&self, mut dispose_fn: F)
    where
        F: FnMut(V),
    {
        let mut map = self.resource_map.lock().unwrap();
        for (_, elem) in map.drain() {
            dispose_fn(elem.resource);
        }
    }

    /// Access the resource map directly (for subclass patterns)
    pub fn with_resource<F2, R>(&self, key: &K, f: F2) -> Option<R>
    where
        F2: FnOnce(&V) -> R,
    {
        let map = self.resource_map.lock().unwrap();
        map.get(key).map(|elem| f(&elem.resource))
    }

    /// Access the resource map mutably (for subclass patterns)
    pub fn with_resource_mut<F2, R>(&self, key: &K, f: F2) -> Option<R>
    where
        F2: FnOnce(&mut V) -> R,
    {
        let mut map = self.resource_map.lock().unwrap();
        map.get_mut(key).map(|elem| f(&mut elem.resource))
    }
}
