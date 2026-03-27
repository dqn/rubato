use std::collections::HashMap;
use std::hash::Hash;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, MutexGuard};

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

/// Acquire a mutex lock, recovering from poison if a thread panicked while holding it.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
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
        let map = lock_or_recover(&self.resource_map);
        map.contains_key(key)
    }

    /// Get a resource by key. If not in the pool, calls `load_fn` to load it.
    /// Returns None if the resource couldn't be loaded.
    ///
    /// The lock is released before calling `load_fn` so that blocking I/O
    /// (e.g. disk reads) does not hold other threads. A double-check after
    /// re-acquiring the lock handles the race where another thread loaded the
    /// same key concurrently. When another thread wins the race, the losing
    /// thread's resource is passed to `race_dispose_fn` for cleanup (e.g.
    /// GPU memory release).
    pub fn get<F, D>(&self, key: &K, load_fn: F, race_dispose_fn: D) -> Option<()>
    where
        F: FnOnce(&K) -> Option<V>,
        D: FnOnce(V),
    {
        // Fast path: cache hit under lock.
        {
            let mut map = lock_or_recover(&self.resource_map);
            if let Some(elem) = map.get_mut(key) {
                elem.generation = 0;
                return Some(());
            }
        }
        // Lock released here -- call load_fn outside the lock.

        let resource = load_fn(key)?;

        // Re-acquire and double-check: another thread may have inserted.
        let mut map = lock_or_recover(&self.resource_map);
        if let Some(elem) = map.get_mut(key) {
            // Another thread loaded it while we were loading; reset generation
            // and discard our copy. Dispose the losing resource to prevent
            // leaking resources that require explicit cleanup (e.g. GPU memory).
            elem.generation = 0;
            drop(map);
            race_dispose_fn(resource);
        } else {
            map.insert(key.clone(), ResourceCacheElement::new(resource));
        }
        Some(())
    }

    /// Get a reference to the resource by key (if it exists in pool).
    /// Does NOT attempt to load.
    pub fn get_cached(&self, key: &K) -> bool {
        let mut map = lock_or_recover(&self.resource_map);
        if let Some(elem) = map.get_mut(key) {
            elem.generation = 0;
            true
        } else {
            false
        }
    }

    /// Advance generation counters and dispose resources that have exceeded maxgen.
    /// `dispose_fn` is called for each evicted resource. If `dispose_fn` panics on
    /// one resource, the panic is caught and logged so that remaining resources
    /// still get their `dispose_fn` called.
    pub fn dispose_old<F>(&self, mut dispose_fn: F)
    where
        F: FnMut(V),
    {
        let evicted: Vec<V> = {
            let mut map = lock_or_recover(&self.resource_map);
            let mut removes = Vec::new();

            for (key, value) in map.iter_mut() {
                if value.generation == self.maxgen {
                    removes.push(key.clone());
                } else {
                    value.generation += 1;
                }
            }

            removes
                .into_iter()
                .filter_map(|key| map.remove(&key).map(|elem| elem.resource))
                .collect()
        };

        for resource in evicted {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| dispose_fn(resource))) {
                log::error!("ResourcePool::dispose_old: dispose_fn panicked: {:?}", e);
            }
        }
    }

    /// Returns the number of resources currently in the pool
    pub fn size(&self) -> usize {
        let map = lock_or_recover(&self.resource_map);
        map.len()
    }

    /// Dispose all resources. If `dispose_fn` panics on one resource, the panic
    /// is caught and logged so that remaining resources still get disposed.
    pub fn dispose<F>(&self, mut dispose_fn: F)
    where
        F: FnMut(V),
    {
        let drained: Vec<V> = {
            let mut map = lock_or_recover(&self.resource_map);
            map.drain().map(|(_, elem)| elem.resource).collect()
        };

        for resource in drained {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| dispose_fn(resource))) {
                log::error!("ResourcePool::dispose: dispose_fn panicked: {:?}", e);
            }
        }
    }

    /// Access the resource map directly (for subclass patterns)
    pub fn with_resource<F2, R>(&self, key: &K, f: F2) -> Option<R>
    where
        F2: FnOnce(&V) -> R,
    {
        let map = lock_or_recover(&self.resource_map);
        map.get(key).map(|elem| f(&elem.resource))
    }

    /// Access the resource map mutably (for subclass patterns)
    pub fn with_resource_mut<F2, R>(&self, key: &K, f: F2) -> Option<R>
    where
        F2: FnOnce(&mut V) -> R,
    {
        let mut map = lock_or_recover(&self.resource_map);
        map.get_mut(key).map(|elem| f(&mut elem.resource))
    }

    /// Load a resource if not cached, then apply a function to it.
    /// Combines `get` (load-on-miss) with `with_resource` (callback access).
    /// Returns None if the resource couldn't be loaded.
    ///
    /// The lock is released before calling `load_fn` so that blocking I/O
    /// does not hold other threads. A double-check after re-acquiring the
    /// lock handles the race where another thread loaded the same key
    /// concurrently. When another thread wins the race, the losing thread's
    /// resource is passed to `race_dispose_fn` for cleanup.
    pub fn get_or_load<L, F2, D, R>(
        &self,
        key: &K,
        load_fn: L,
        f: F2,
        race_dispose_fn: D,
    ) -> Option<R>
    where
        L: FnOnce(&K) -> Option<V>,
        F2: FnOnce(&V) -> R,
        D: FnOnce(V),
    {
        // Fast path: cache hit under lock.
        {
            let mut map = lock_or_recover(&self.resource_map);
            if let Some(elem) = map.get_mut(key) {
                elem.generation = 0;
                return Some(f(&elem.resource));
            }
        }
        // Lock released here -- call load_fn outside the lock.

        let resource = load_fn(key)?;

        // Re-acquire and double-check: another thread may have inserted.
        let mut map = lock_or_recover(&self.resource_map);
        if let Some(elem) = map.get_mut(key) {
            // Another thread loaded it; use the already-cached value and
            // dispose our copy to prevent leaking resources that require
            // explicit cleanup (e.g. GPU memory).
            elem.generation = 0;
            let result = f(&elem.resource);
            drop(map);
            race_dispose_fn(resource);
            Some(result)
        } else {
            let result = f(&resource);
            map.insert(key.clone(), ResourceCacheElement::new(resource));
            Some(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    #[test]
    fn test_get_loads_on_miss() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result = pool.get(&key, |_| Some(42), |_| {});
        assert_eq!(result, Some(()));
        assert!(pool.exists(&key));
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_returns_none_when_load_fails() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result = pool.get(&key, |_| None, |_| {});
        assert_eq!(result, None);
        assert!(!pool.exists(&key));
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_get_caches_on_hit() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();
        let load_count = AtomicI32::new(0);

        pool.get(
            &key,
            |_| {
                load_count.fetch_add(1, Ordering::SeqCst);
                Some(42)
            },
            |_| {},
        );
        pool.get(
            &key,
            |_| {
                load_count.fetch_add(1, Ordering::SeqCst);
                Some(99)
            },
            |_| {},
        );

        assert_eq!(load_count.load(Ordering::SeqCst), 1);
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_or_load_returns_value() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result = pool.get_or_load(&key, |_| Some(42), |v| *v, |_| {});
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
            |_| {},
        );
        let result = pool.get_or_load(
            &key,
            |_| {
                load_count.fetch_add(1, Ordering::SeqCst);
                Some(99)
            },
            |v| *v,
            |_| {},
        );

        assert_eq!(result, Some(42));
        assert_eq!(load_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_get_or_load_returns_none_when_load_fails() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        let result: Option<i32> = pool.get_or_load(&key, |_| None, |v| *v, |_| {});
        assert_eq!(result, None);
        assert!(!pool.exists(&key));
    }

    #[test]
    fn test_dispose_old_evicts_after_maxgen() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let key = "key1".to_string();

        pool.get(&key, |_| Some(42), |_| {});
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

        pool.get(&key, |_| Some(42), |_| {});
        pool.dispose_old(|_| {}); // gen 0 -> 1

        // Access resets generation to 0
        pool.get(&key, |_| unreachable!(), |_| {});
        pool.dispose_old(|_| {}); // gen 0 -> 1 (not evicted)
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_dispose_all() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        pool.get(&"a".to_string(), |_| Some(1), |_| {});
        pool.get(&"b".to_string(), |_| Some(2), |_| {});
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
        pool.get(&key, |_| Some(42), |_| {});

        let result = pool.with_resource(&key, |v| *v * 2);
        assert_eq!(result, Some(84));
    }

    #[test]
    fn test_with_resource_returns_none_when_missing() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        let result = pool.with_resource(&"missing".to_string(), |v| *v);
        assert_eq!(result, None);
    }

    #[test]
    fn test_dispose_fn_panic_does_not_poison_lock() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        pool.get(&"a".to_string(), |_| Some(1), |_| {});
        pool.get(&"b".to_string(), |_| Some(2), |_| {});

        pool.dispose(|_v| {
            panic!("intentional panic in dispose_fn");
        });

        assert_eq!(pool.size(), 0);
        assert!(!pool.resource_map.is_poisoned());
        pool.get(&"c".to_string(), |_| Some(3), |_| {});
        assert!(pool.exists(&"c".to_string()));
    }

    #[test]
    fn test_dispose_old_fn_panic_does_not_poison_lock() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(0);
        pool.get(&"x".to_string(), |_| Some(10), |_| {});
        pool.get(&"y".to_string(), |_| Some(20), |_| {});

        pool.dispose_old(|_v| {
            panic!("intentional panic in dispose_old callback");
        });

        assert_eq!(pool.size(), 0);
        assert!(!pool.resource_map.is_poisoned());
        pool.get(&"z".to_string(), |_| Some(30), |_| {});
        assert!(pool.exists(&"z".to_string()));
    }

    #[test]
    fn test_lock_or_recover_after_poison() {
        let pool: ResourcePool<String, i32> = ResourcePool::new(1);
        pool.get(&"key".to_string(), |_| Some(42), |_| {});

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = pool.resource_map.lock().expect("lock");
            panic!("intentional poison");
        }));
        assert!(pool.resource_map.is_poisoned());

        assert!(pool.exists(&"key".to_string()));
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_does_not_hold_lock_during_load_fn() {
        let pool = Arc::new(ResourcePool::<String, i32>::new(1));
        pool.get(&"existing".to_string(), |_| Some(100), |_| {});

        let loading = Arc::new(AtomicBool::new(false));
        let accessed = Arc::new(AtomicBool::new(false));

        let pool2 = Arc::clone(&pool);
        let loading2 = Arc::clone(&loading);
        let accessed2 = Arc::clone(&accessed);

        let t1 = std::thread::spawn(move || {
            pool2.get(
                &"slow_key".to_string(),
                |_| {
                    loading2.store(true, Ordering::SeqCst);
                    while !accessed2.load(Ordering::SeqCst) {
                        std::thread::yield_now();
                    }
                    Some(42)
                },
                |_| {},
            );
        });

        while !loading.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        assert!(pool.exists(&"existing".to_string()));
        accessed.store(true, Ordering::SeqCst);

        t1.join().unwrap();
        assert!(pool.exists(&"slow_key".to_string()));
    }

    #[test]
    fn test_get_concurrent_load_same_key_uses_first_value() {
        let pool = Arc::new(ResourcePool::<String, i32>::new(1));

        let barrier = Arc::new(std::sync::Barrier::new(2));
        let load_count = Arc::new(AtomicI32::new(0));

        let handles: Vec<_> = (0..2)
            .map(|i| {
                let pool = Arc::clone(&pool);
                let barrier = Arc::clone(&barrier);
                let load_count = Arc::clone(&load_count);
                std::thread::spawn(move || {
                    pool.get(
                        &"same_key".to_string(),
                        |_| {
                            barrier.wait();
                            load_count.fetch_add(1, Ordering::SeqCst);
                            Some(i)
                        },
                        |_| {},
                    );
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert!(pool.exists(&"same_key".to_string()));
        assert_eq!(pool.size(), 1);
        assert_eq!(load_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_get_or_load_does_not_hold_lock_during_load_fn() {
        let pool = Arc::new(ResourcePool::<String, i32>::new(1));
        pool.get(&"existing".to_string(), |_| Some(100), |_| {});

        let loading = Arc::new(AtomicBool::new(false));
        let accessed = Arc::new(AtomicBool::new(false));

        let pool2 = Arc::clone(&pool);
        let loading2 = Arc::clone(&loading);
        let accessed2 = Arc::clone(&accessed);

        let t1 = std::thread::spawn(move || {
            pool2.get_or_load(
                &"slow_key".to_string(),
                |_| {
                    loading2.store(true, Ordering::SeqCst);
                    while !accessed2.load(Ordering::SeqCst) {
                        std::thread::yield_now();
                    }
                    Some(42)
                },
                |v| *v,
                |_| {},
            )
        });

        while !loading.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        assert!(pool.exists(&"existing".to_string()));
        accessed.store(true, Ordering::SeqCst);

        let result = t1.join().unwrap();
        assert_eq!(result, Some(42));
        assert!(pool.exists(&"slow_key".to_string()));
    }

    #[test]
    fn test_get_race_calls_dispose_on_loser() {
        let pool = Arc::new(ResourcePool::<String, i32>::new(1));
        let dispose_count = Arc::new(AtomicI32::new(0));

        let barrier = Arc::new(std::sync::Barrier::new(2));

        let handles: Vec<_> = (0..2)
            .map(|i| {
                let pool = Arc::clone(&pool);
                let barrier = Arc::clone(&barrier);
                let dispose_count = Arc::clone(&dispose_count);
                std::thread::spawn(move || {
                    pool.get(
                        &"race_key".to_string(),
                        |_| {
                            barrier.wait();
                            Some(i)
                        },
                        |_| {
                            dispose_count.fetch_add(1, Ordering::SeqCst);
                        },
                    );
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(pool.size(), 1);
        assert_eq!(
            dispose_count.load(Ordering::SeqCst),
            1,
            "race-losing resource must be disposed exactly once"
        );
    }

    #[test]
    fn test_get_or_load_race_calls_dispose_on_loser() {
        let pool = Arc::new(ResourcePool::<String, i32>::new(1));
        let dispose_count = Arc::new(AtomicI32::new(0));

        let barrier = Arc::new(std::sync::Barrier::new(2));

        let handles: Vec<_> = (0..2)
            .map(|i| {
                let pool = Arc::clone(&pool);
                let barrier = Arc::clone(&barrier);
                let dispose_count = Arc::clone(&dispose_count);
                std::thread::spawn(move || {
                    pool.get_or_load(
                        &"race_key".to_string(),
                        |_| {
                            barrier.wait();
                            Some(i)
                        },
                        |v| *v,
                        |_| {
                            dispose_count.fetch_add(1, Ordering::SeqCst);
                        },
                    )
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(pool.size(), 1);
        assert_eq!(
            dispose_count.load(Ordering::SeqCst),
            1,
            "race-losing resource must be disposed exactly once"
        );
    }
}
