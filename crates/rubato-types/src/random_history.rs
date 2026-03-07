use std::collections::VecDeque;
use std::sync::Mutex;

static LANE_ORDER_HISTORY: Mutex<VecDeque<RandomHistoryEntry>> = Mutex::new(VecDeque::new());

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Shared random history entry type -- used by both beatoraja-pattern and beatoraja-modmenu.
/// Moved here from beatoraja-modmenu to break circular dependency.
#[derive(Clone, Debug)]
pub struct RandomHistoryEntry {
    pub title: String,
    pub random: String,
}

impl RandomHistoryEntry {
    pub fn new(title: String, random: String) -> Self {
        RandomHistoryEntry { title, random }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn random(&self) -> &str {
        &self.random
    }
}

pub fn add_random_history(entry: RandomHistoryEntry) {
    let mut history = lock_or_recover(&LANE_ORDER_HISTORY);
    history.push_front(entry);
}

pub fn random_history() -> VecDeque<RandomHistoryEntry> {
    lock_or_recover(&LANE_ORDER_HISTORY).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_history_recovers_after_poison() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = LANE_ORDER_HISTORY.lock().expect("mutex poisoned");
            panic!("poison random history");
        }));

        add_random_history(RandomHistoryEntry::new(
            "Song".to_string(),
            "RANDOM".to_string(),
        ));

        let history = random_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].title, "Song");
        assert_eq!(history[0].random, "RANDOM");
    }
}
