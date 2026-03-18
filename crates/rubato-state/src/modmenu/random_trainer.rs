use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rubato_types::random_history;
pub use rubato_types::random_history::RandomHistoryEntry;
use rubato_types::sync_utils::lock_or_recover;

static LANE_ORDER: Mutex<String> = Mutex::new(String::new());
static LANES_TO_RANDOM: Mutex<Vec<char>> = Mutex::new(Vec::new());
static BLACK_WHITE_PERMUTE: Mutex<bool> = Mutex::new(false);
static ACTIVE: Mutex<bool> = Mutex::new(false);
static LANE_MASK: Mutex<Vec<bool>> = Mutex::new(Vec::new());
static RANDOM_SEED_MAP: Mutex<Option<HashMap<i32, i64>>> = Mutex::new(None);

fn init_defaults() {
    let mut lane_order = lock_or_recover(&LANE_ORDER);
    if lane_order.is_empty() {
        *lane_order = "1234567".to_string();
    }
    let mut lane_mask = lock_or_recover(&LANE_MASK);
    if lane_mask.is_empty() {
        *lane_mask = vec![false; 7];
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
        init_defaults();
        let mut seed_map = lock_or_recover(&RANDOM_SEED_MAP);
        if seed_map.is_none() {
            // In Java this loads from a serialized resource file "resources/randomtrainer.dat"
            // We stub this as an empty map since the binary resource is not available
            log::info!("RandomTrainer: randomtrainer.dat not found, using empty map");
            *seed_map = Some(HashMap::new());
        }
        RandomTrainer
    }

    pub fn lane_order() -> String {
        init_defaults();
        let mut rng = thread_rng();

        let black_white_permute = *lock_or_recover(&BLACK_WHITE_PERMUTE);
        let mut lane_order = lock_or_recover(&LANE_ORDER);
        let lanes_to_random = lock_or_recover(&LANES_TO_RANDOM);

        if black_white_permute {
            let mut black: Vec<char> = Vec::new();
            let mut white: Vec<char> = Vec::new();
            for c in lane_order.chars() {
                let digit = c.to_digit(10).unwrap_or(0) as i32;
                if digit % 2 == 0 {
                    black.push(c);
                } else {
                    white.push(c);
                }
            }
            black.shuffle(&mut rng);
            white.shuffle(&mut rng);

            let mut new_lane_order: Vec<char> = lane_order.chars().collect();
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
            *lane_order = new_lane_order.into_iter().collect();
        }

        let mut shuffled_lanes: Vec<char> = lanes_to_random.clone();
        shuffled_lanes.shuffle(&mut rng);
        let mut new_lane_order: Vec<char> = lane_order.chars().collect();
        for ch in new_lane_order.iter_mut() {
            if lanes_to_random.contains(ch)
                && let Some(c) = shuffled_lanes.first()
            {
                *ch = *c;
                shuffled_lanes.remove(0);
            }
        }
        *lane_order = new_lane_order.into_iter().collect();
        lane_order.clone()
    }

    pub fn is_lane_to_random(lane: char) -> bool {
        let lanes = lock_or_recover(&LANES_TO_RANDOM);
        lanes.contains(&lane)
    }

    pub fn set_lane_to_random(lane: char) {
        let mut lanes = lock_or_recover(&LANES_TO_RANDOM);
        lanes.push(lane);
    }

    pub fn remove_lane_to_random(lane: char) {
        let mut lanes = lock_or_recover(&LANES_TO_RANDOM);
        if let Some(pos) = lanes.iter().position(|&c| c == lane) {
            lanes.remove(pos);
        }
    }

    pub fn is_active() -> bool {
        *lock_or_recover(&ACTIVE)
    }

    pub fn set_active(active: bool) {
        *lock_or_recover(&ACTIVE) = active;
    }

    pub fn get_random_seed_map() -> Option<HashMap<i32, i64>> {
        lock_or_recover(&RANDOM_SEED_MAP).clone()
    }

    pub fn set_black_white_permute(black_white_permute: bool) {
        *lock_or_recover(&BLACK_WHITE_PERMUTE) = black_white_permute;
    }

    /// Returns the current lane order string without shuffling.
    pub fn get_current_lane_order() -> String {
        init_defaults();
        lock_or_recover(&LANE_ORDER).clone()
    }

    pub fn set_lane_order(number: &str) {
        *lock_or_recover(&LANE_ORDER) = number.to_string();
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
        *LANE_ORDER.lock().unwrap() = "1234567".to_string();
        LANES_TO_RANDOM.lock().unwrap().clear();
        *BLACK_WHITE_PERMUTE.lock().unwrap() = false;
        *ACTIVE.lock().unwrap() = false;
        LANE_MASK.lock().unwrap().clear();
        *LANE_MASK.lock().unwrap() = vec![false; 7];
        *RANDOM_SEED_MAP.lock().unwrap() = Some(HashMap::new());
        guard
    }

    // --- RandomTrainer::new ---

    #[test]
    fn test_new_initializes_defaults() {
        let _g = reset_globals();
        let _trainer = RandomTrainer::new();
        let lane_order = LANE_ORDER.lock().unwrap();
        assert_eq!(*lane_order, "1234567");
        let seed_map = RANDOM_SEED_MAP.lock().unwrap();
        assert!(seed_map.is_some());
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
        let order = LANE_ORDER.lock().unwrap();
        assert_eq!(*order, "7654321");
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
