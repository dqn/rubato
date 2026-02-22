use std::collections::VecDeque;
use std::sync::Mutex;

static LANE_ORDER_HISTORY: Mutex<VecDeque<RandomHistoryEntry>> = Mutex::new(VecDeque::new());

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

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_random(&self) -> &str {
        &self.random
    }
}

pub fn add_random_history(entry: RandomHistoryEntry) {
    let mut history = LANE_ORDER_HISTORY.lock().unwrap();
    history.push_front(entry);
}

pub fn get_random_history() -> VecDeque<RandomHistoryEntry> {
    LANE_ORDER_HISTORY.lock().unwrap().clone()
}
