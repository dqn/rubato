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

    /// Load a resource if not cached, then apply a function to it.
    /// Combines `get` (load-on-miss) with `with_resource` (callback access).
    /// Returns None if the resource couldn't be loaded.
    pub fn get_or_load<L, F2, R>(&self, key: &K, load_fn: L, f: F2) -> Option<R>
    where
        L: FnOnce(&K) -> Option<V>,
        F2: FnOnce(&V) -> R,
    {
        let mut map = self.resource_map.lock().unwrap();
        if let Some(elem) = map.get_mut(key) {
            elem.generation = 0;
            return Some(f(&elem.resource));
        }

        if let Some(resource) = load_fn(key) {
            let result = f(&resource);
            map.insert(key.clone(), ResourceCacheElement::new(resource));
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn test_get_loads_on_miss() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result = pool.get(&key, |_| Some(42));
        assert_eq!(result, Some(()));
        assert!(pool.exists(&key));
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_returns_none_when_load_fails() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result = pool.get(&key, |_| None);
        assert_eq!(result, None);
        assert!(!pool.exists(&key));
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_get_caches_on_hit() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();
        let load_count = AtomicI32::new(0);

        pool.get(&key, |_| {
            load_count.fetch_add(1, Ordering::SeqCst);
            Some(42)
        });
        pool.get(&key, |_| {
            load_count.fetch_add(1, Ordering::SeqCst);
            Some(99)
        });

        assert_eq!(load_count.load(Ordering::SeqCst), 1);
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_or_load_returns_value() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result = pool.get_or_load(&key, |_| Some(42), |v| *v);
        assert_eq!(result, Some(42));
        assert!(pool.exists(&key));
    }

    #[test]
    fn test_get_or_load_cache_hit() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();
        let load_count = AtomicI32::new(0);

        pool.get_or_load(
            &key,
            |_| {
                load_count.fetch_add(1, Ordering::SeqCst);
                Some(42)
            },
            |v| *v,
        );
        let result = pool.get_or_load(
            &key,
            |_| {
                load_count.fetch_add(1, Ordering::SeqCst);
                Some(99)
            },
            |v| *v,
        );

        assert_eq!(result, Some(42));
        assert_eq!(load_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_get_or_load_returns_none_when_load_fails() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result: Option<i32> = pool.get_or_load(&key, |_| None, |v| *v);
        assert_eq!(result, None);
        assert!(!pool.exists(&key));
    }

    #[test]
    fn test_dispose_old_evicts_after_maxgen() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        pool.get(&key, |_| Some(42));
        assert_eq!(pool.size(), 1);

        // First dispose_old: generation 0 -> 1 (not evicted yet)
        pool.dispose_old(|_| {});
        assert_eq!(pool.size(), 1);

        // Second dispose_old: generation 1 == maxgen(1), evict
        pool.dispose_old(|_| {});
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_access_resets_generation() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        pool.get(&key, |_| Some(42));
        pool.dispose_old(|_| {}); // gen 0 -> 1

        // Access resets generation to 0
        pool.get(&key, |_| unreachable!());
        pool.dispose_old(|_| {}); // gen 0 -> 1 (not evicted)
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_dispose_all() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        pool.get(&"a".to_string(), |_| Some(1));
        pool.get(&"b".to_string(), |_| Some(2));
        assert_eq!(pool.size(), 2);

        let mut disposed = Vec::new();
        pool.dispose(|v| disposed.push(v));
        assert_eq!(pool.size(), 0);
        disposed.sort();
        assert_eq!(disposed, vec![1, 2]);
    }

    #[test]
    fn test_with_resource() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();
        pool.get(&key, |_| Some(42));

        let result = pool.with_resource(&key, |v| *v * 2);
        assert_eq!(result, Some(84));
    }

    #[test]
    fn test_with_resource_returns_none_when_missing() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let result = pool.with_resource(&"missing".to_string(), |v| *v);
        assert_eq!(result, None);
    }
}
