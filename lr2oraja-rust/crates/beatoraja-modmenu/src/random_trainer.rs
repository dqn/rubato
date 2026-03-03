use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use beatoraja_types::random_history;
pub use beatoraja_types::random_history::RandomHistoryEntry;
use rand::seq::SliceRandom;
use rand::thread_rng;

static LANE_ORDER: Mutex<String> = Mutex::new(String::new());
static LANES_TO_RANDOM: Mutex<Vec<char>> = Mutex::new(Vec::new());
static BLACK_WHITE_PERMUTE: Mutex<bool> = Mutex::new(false);
static ACTIVE: Mutex<bool> = Mutex::new(false);
static LANE_MASK: Mutex<Vec<bool>> = Mutex::new(Vec::new());
static RANDOM_SEED_MAP: Mutex<Option<HashMap<i32, i64>>> = Mutex::new(None);

fn init_defaults() {
    let mut lane_order = LANE_ORDER.lock().unwrap();
    if lane_order.is_empty() {
        *lane_order = "1234567".to_string();
    }
    let mut lane_mask = LANE_MASK.lock().unwrap();
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
        let mut seed_map = RANDOM_SEED_MAP.lock().unwrap();
        if seed_map.is_none() {
            // In Java this loads from a serialized resource file "resources/randomtrainer.dat"
            // We stub this as an empty map since the binary resource is not available
            log::info!("RandomTrainer: randomtrainer.dat not found, using empty map");
            *seed_map = Some(HashMap::new());
        }
        RandomTrainer
    }

    pub fn get_lane_order() -> String {
        init_defaults();
        let mut rng = thread_rng();

        let black_white_permute = *BLACK_WHITE_PERMUTE.lock().unwrap();
        let mut lane_order = LANE_ORDER.lock().unwrap();
        let lanes_to_random = LANES_TO_RANDOM.lock().unwrap();

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
            #[allow(clippy::needless_range_loop)]
            for i in 0..lane_order.len() {
                let current = new_lane_order[i];
                let digit = current.to_digit(10).unwrap_or(0) as i32;
                if digit % 2 == 0 {
                    if let Some(c) = black.first() {
                        new_lane_order[i] = *c;
                        black.remove(0);
                    }
                } else if let Some(c) = white.first() {
                    new_lane_order[i] = *c;
                    white.remove(0);
                }
            }
            *lane_order = new_lane_order.into_iter().collect();
        }

        let mut shuffled_lanes: Vec<char> = lanes_to_random.clone();
        shuffled_lanes.shuffle(&mut rng);
        let mut new_lane_order: Vec<char> = lane_order.chars().collect();
        #[allow(clippy::needless_range_loop)]
        for i in 0..lane_order.len() {
            let ch = new_lane_order[i];
            if lanes_to_random.contains(&ch)
                && let Some(c) = shuffled_lanes.first()
            {
                new_lane_order[i] = *c;
                shuffled_lanes.remove(0);
            }
        }
        *lane_order = new_lane_order.into_iter().collect();
        lane_order.clone()
    }

    pub fn is_lane_to_random(lane: char) -> bool {
        let lanes = LANES_TO_RANDOM.lock().unwrap();
        lanes.contains(&lane)
    }

    pub fn set_lane_to_random(lane: char) {
        let mut lanes = LANES_TO_RANDOM.lock().unwrap();
        lanes.push(lane);
    }

    pub fn remove_lane_to_random(lane: char) {
        let mut lanes = LANES_TO_RANDOM.lock().unwrap();
        if let Some(pos) = lanes.iter().position(|&c| c == lane) {
            lanes.remove(pos);
        }
    }

    pub fn is_active() -> bool {
        *ACTIVE.lock().unwrap()
    }

    pub fn set_active(active: bool) {
        *ACTIVE.lock().unwrap() = active;
    }

    pub fn get_random_seed_map() -> Option<HashMap<i32, i64>> {
        RANDOM_SEED_MAP.lock().unwrap().clone()
    }

    pub fn set_black_white_permute(black_white_permute: bool) {
        *BLACK_WHITE_PERMUTE.lock().unwrap() = black_white_permute;
    }

    pub fn set_lane_order(number: &str) {
        *LANE_ORDER.lock().unwrap() = number.to_string();
    }

    pub fn get_random_history() -> VecDeque<RandomHistoryEntry> {
        random_history::get_random_history()
    }

    pub fn add_random_history(hist_entry: RandomHistoryEntry) {
        random_history::add_random_history(hist_entry);
    }
}
