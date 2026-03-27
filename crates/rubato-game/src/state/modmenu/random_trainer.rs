use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rubato_types::random_history;
pub use rubato_types::random_history::RandomHistoryEntry;
use rubato_types::sync_utils::lock_or_recover;

struct RandomTrainerState {
    lane_order: String,
    lanes_to_random: Vec<char>,
    black_white_permute: bool,
    active: bool,
    lane_mask: Vec<bool>,
    random_seed_map: Option<HashMap<i32, i64>>,
}

static STATE: Mutex<RandomTrainerState> = Mutex::new(RandomTrainerState {
    lane_order: String::new(),
    lanes_to_random: Vec::new(),
    black_white_permute: false,
    active: false,
    lane_mask: Vec::new(),
    random_seed_map: None,
});

fn init_defaults(state: &mut RandomTrainerState) {
    if state.lane_order.is_empty() {
        state.lane_order = "1234567".to_string();
    }
    if state.lane_mask.is_empty() {
        state.lane_mask = vec![false; 7];
    }
}

pub struct RandomTrainer;

impl Default for RandomTrainer {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomTrainer {
    pub fn new() -> Self {
        let mut state = lock_or_recover(&STATE);
        init_defaults(&mut state);
        if state.random_seed_map.is_none() {
            // In Java this loads from a serialized resource file "resources/randomtrainer.dat"
            // We stub this as an empty map since the binary resource is not available
            log::info!("RandomTrainer: randomtrainer.dat not found, using empty map");
            state.random_seed_map = Some(HashMap::new());
        }
        RandomTrainer
    }

    pub fn lane_order() -> String {
        let mut rng = thread_rng();
        let mut state = lock_or_recover(&STATE);
        init_defaults(&mut state);

        if state.black_white_permute {
            let mut black: Vec<char> = Vec::new();
            let mut white: Vec<char> = Vec::new();
            for c in state.lane_order.chars() {
                let digit = c.to_digit(10).unwrap_or(0) as i32;
                if digit % 2 == 0 {
                    black.push(c);
                } else {
                    white.push(c);
                }
            }
            black.shuffle(&mut rng);
            white.shuffle(&mut rng);

            let mut new_lane_order: Vec<char> = state.lane_order.chars().collect();
            for ch in new_lane_order.iter_mut() {
                let digit = ch.to_digit(10).unwrap_or(0) as i32;
                if digit % 2 == 0 {
                    if let Some(c) = black.first() {
                        *ch = *c;
                        black.remove(0);
                    }
                } else if let Some(c) = white.first() {
                    *ch = *c;
                    white.remove(0);
                }
            }
            state.lane_order = new_lane_order.into_iter().collect();
        }

        let mut shuffled_lanes: Vec<char> = state.lanes_to_random.clone();
        shuffled_lanes.shuffle(&mut rng);
        let mut new_lane_order: Vec<char> = state.lane_order.chars().collect();
        for ch in new_lane_order.iter_mut() {
            if state.lanes_to_random.contains(ch)
                && let Some(c) = shuffled_lanes.first()
            {
                *ch = *c;
                shuffled_lanes.remove(0);
            }
        }
        state.lane_order = new_lane_order.into_iter().collect();
        state.lane_order.clone()
    }

    pub fn is_lane_to_random(lane: char) -> bool {
        let state = lock_or_recover(&STATE);
        state.lanes_to_random.contains(&lane)
    }

    pub fn set_lane_to_random(lane: char) {
        let mut state = lock_or_recover(&STATE);
        state.lanes_to_random.push(lane);
    }

    pub fn remove_lane_to_random(lane: char) {
        let mut state = lock_or_recover(&STATE);
        if let Some(pos) = state.lanes_to_random.iter().position(|&c| c == lane) {
            state.lanes_to_random.remove(pos);
        }
    }

    pub fn is_active() -> bool {
        lock_or_recover(&STATE).active
    }

    pub fn set_active(active: bool) {
        lock_or_recover(&STATE).active = active;
    }

    pub fn get_random_seed_map() -> Option<HashMap<i32, i64>> {
        lock_or_recover(&STATE).random_seed_map.clone()
    }

    pub fn set_black_white_permute(black_white_permute: bool) {
        lock_or_recover(&STATE).black_white_permute = black_white_permute;
    }

    /// Returns the current lane order string without shuffling.
    pub fn get_current_lane_order() -> String {
        let mut state = lock_or_recover(&STATE);
        init_defaults(&mut state);
        state.lane_order.clone()
    }

    pub fn set_lane_order(number: &str) {
        lock_or_recover(&STATE).lane_order = number.to_string();
    }

    pub fn random_history() -> VecDeque<RandomHistoryEntry> {
        random_history::random_history()
    }

    pub fn add_random_history(hist_entry: RandomHistoryEntry) {
        random_history::add_random_history(hist_entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that touch process-global statics.
    /// Without this, parallel test threads race on the shared Mutex state
    /// and `reset_globals()` in one test can stomp on another mid-execution.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Reset all global statics to a known state and return the test lock guard.
    /// The caller must hold the returned guard for the duration of the test to
    /// prevent other tests from mutating the shared statics concurrently.
    fn reset_globals() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap();
        let mut state = STATE.lock().unwrap();
        state.lane_order = "1234567".to_string();
        state.lanes_to_random.clear();
        state.black_white_permute = false;
        state.active = false;
        state.lane_mask.clear();
        state.lane_mask = vec![false; 7];
        state.random_seed_map = Some(HashMap::new());
        guard
    }

    // --- RandomTrainer::new ---

    #[test]
    fn test_new_initializes_defaults() {
        let _g = reset_globals();
        let _trainer = RandomTrainer::new();
        let state = STATE.lock().unwrap();
        assert_eq!(state.lane_order, "1234567");
        assert!(state.random_seed_map.is_some());
    }

    // --- set/remove lane_to_random ---

    #[test]
    fn test_set_lane_to_random() {
        let _g = reset_globals();
        RandomTrainer::set_lane_to_random('3');
        assert!(RandomTrainer::is_lane_to_random('3'));
        assert!(!RandomTrainer::is_lane_to_random('5'));
    }

    #[test]
    fn test_remove_lane_to_random() {
        let _g = reset_globals();
        RandomTrainer::set_lane_to_random('1');
        RandomTrainer::set_lane_to_random('2');
        RandomTrainer::remove_lane_to_random('1');
        assert!(!RandomTrainer::is_lane_to_random('1'));
        assert!(RandomTrainer::is_lane_to_random('2'));
    }

    #[test]
    fn test_remove_lane_to_random_nonexistent_is_noop() {
        let _g = reset_globals();
        // Removing a lane that was never added should not panic
        RandomTrainer::remove_lane_to_random('9');
        assert!(!RandomTrainer::is_lane_to_random('9'));
    }

    #[test]
    fn test_remove_lane_to_random_removes_only_first() {
        let _g = reset_globals();
        // Add duplicate
        RandomTrainer::set_lane_to_random('5');
        RandomTrainer::set_lane_to_random('5');
        RandomTrainer::remove_lane_to_random('5');
        // One copy should remain
        assert!(RandomTrainer::is_lane_to_random('5'));
    }

    // --- active ---

    #[test]
    fn test_active_default_false() {
        let _g = reset_globals();
        assert!(!RandomTrainer::is_active());
    }

    #[test]
    fn test_set_active() {
        let _g = reset_globals();
        RandomTrainer::set_active(true);
        assert!(RandomTrainer::is_active());
        RandomTrainer::set_active(false);
        assert!(!RandomTrainer::is_active());
    }

    // --- lane_order ---

    #[test]
    fn test_set_lane_order() {
        let _g = reset_globals();
        RandomTrainer::set_lane_order("7654321");
        let state = STATE.lock().unwrap();
        assert_eq!(state.lane_order, "7654321");
    }

    // --- lane_order (shuffling) ---

    #[test]
    fn test_get_lane_order_preserves_length_and_chars() {
        let _g = reset_globals();
        let order = RandomTrainer::lane_order();
        assert_eq!(order.len(), 7);
        // All original digits should be present (possibly reordered)
        let mut sorted: Vec<char> = order.chars().collect();
        sorted.sort();
        assert_eq!(sorted, vec!['1', '2', '3', '4', '5', '6', '7']);
    }

    #[test]
    fn test_get_lane_order_with_random_lanes_preserves_elements() {
        let _g = reset_globals();
        RandomTrainer::set_lane_to_random('1');
        RandomTrainer::set_lane_to_random('3');
        RandomTrainer::set_lane_to_random('5');

        let order = RandomTrainer::lane_order();
        assert_eq!(order.len(), 7);

        // All 7 digits should still be present
        let mut sorted: Vec<char> = order.chars().collect();
        sorted.sort();
        assert_eq!(sorted, vec!['1', '2', '3', '4', '5', '6', '7']);
    }

    #[test]
    fn test_get_lane_order_with_black_white_permute_preserves_parity() {
        let _g = reset_globals();
        RandomTrainer::set_black_white_permute(true);

        let order = RandomTrainer::lane_order();
        let chars: Vec<char> = order.chars().collect();
        assert_eq!(chars.len(), 7);

        // Verify parity is preserved: odd digits stay at positions that originally
        // had odd digits (0,2,4,6), even at even positions (1,3,5)
        let odd_positions: Vec<char> = vec![chars[0], chars[2], chars[4], chars[6]];
        let even_positions: Vec<char> = vec![chars[1], chars[3], chars[5]];

        for c in &odd_positions {
            let digit = c.to_digit(10).unwrap();
            assert!(
                digit % 2 == 1,
                "odd position should have odd digit, got {}",
                digit
            );
        }
        for c in &even_positions {
            let digit = c.to_digit(10).unwrap();
            assert!(
                digit % 2 == 0,
                "even position should have even digit, got {}",
                digit
            );
        }
    }

    // --- random_seed_map ---

    #[test]
    fn test_get_random_seed_map_is_some() {
        let _g = reset_globals();
        let _trainer = RandomTrainer::new();
        let map = RandomTrainer::get_random_seed_map();
        assert!(map.is_some());
        assert!(map.unwrap().is_empty());
    }

    // --- get_current_lane_order ---

    #[test]
    fn test_get_current_lane_order_returns_stable_value() {
        // Regression: lane_order() shuffles on every call, causing per-frame
        // re-randomization when used as a read operation.
        // get_current_lane_order() must return the same value on repeated calls.
        let _g = reset_globals();
        RandomTrainer::set_lane_order("3142567");
        let first = RandomTrainer::get_current_lane_order();
        let second = RandomTrainer::get_current_lane_order();
        assert_eq!(first, "3142567");
        assert_eq!(
            first, second,
            "get_current_lane_order must be stable across calls"
        );
    }

    #[test]
    fn test_get_current_lane_order_reflects_set_lane_order() {
        let _g = reset_globals();
        RandomTrainer::set_lane_order("7654321");
        assert_eq!(RandomTrainer::get_current_lane_order(), "7654321");
    }
}
