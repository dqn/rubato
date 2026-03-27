use bms::model::mode::Mode;

#[derive(Clone)]
pub struct LaneProperty {
    /// Key to lane mapping
    key_to_lane: Vec<i32>,
    /// Lane to key(s) mapping
    lane_to_key: Vec<Vec<i32>>,
    /// Lane to scratch index (-1 if not scratch)
    lane_to_scratch: Vec<i32>,
    /// Lane to skin offset mapping
    lane_to_skin_offset: Vec<i32>,
    /// Lane to player number mapping
    lane_to_player: Vec<i32>,
    /// Scratch to key mapping (2 keys per scratch)
    scratch_to_key: Vec<Vec<i32>>,
}

impl LaneProperty {
    pub fn new(mode: &Mode) -> Self {
        let (key_to_lane, lane_to_key, lane_to_scratch, lane_to_skin_offset, scratch_to_key) =
            match mode {
                Mode::BEAT_5K => (
                    vec![0, 1, 2, 3, 4, 5, 5],
                    vec![vec![0], vec![1], vec![2], vec![3], vec![4], vec![5, 6]],
                    vec![-1, -1, -1, -1, -1, 0],
                    vec![1, 2, 3, 4, 5, 0],
                    vec![vec![5, 6]],
                ),
                Mode::BEAT_7K => (
                    vec![0, 1, 2, 3, 4, 5, 6, 7, 7],
                    vec![
                        vec![0],
                        vec![1],
                        vec![2],
                        vec![3],
                        vec![4],
                        vec![5],
                        vec![6],
                        vec![7, 8],
                    ],
                    vec![-1, -1, -1, -1, -1, -1, -1, 0],
                    vec![1, 2, 3, 4, 5, 6, 7, 0],
                    vec![vec![7, 8]],
                ),
                Mode::BEAT_10K => (
                    vec![0, 1, 2, 3, 4, 5, 5, 6, 7, 8, 9, 10, 11, 11],
                    vec![
                        vec![0],
                        vec![1],
                        vec![2],
                        vec![3],
                        vec![4],
                        vec![5, 6],
                        vec![7],
                        vec![8],
                        vec![9],
                        vec![10],
                        vec![11],
                        vec![12, 13],
                    ],
                    vec![-1, -1, -1, -1, -1, 0, -1, -1, -1, -1, -1, 1],
                    vec![1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 0],
                    vec![vec![5, 6], vec![12, 13]],
                ),
                Mode::BEAT_14K => (
                    vec![0, 1, 2, 3, 4, 5, 6, 7, 7, 8, 9, 10, 11, 12, 13, 14, 15, 15],
                    vec![
                        vec![0],
                        vec![1],
                        vec![2],
                        vec![3],
                        vec![4],
                        vec![5],
                        vec![6],
                        vec![7, 8],
                        vec![9],
                        vec![10],
                        vec![11],
                        vec![12],
                        vec![13],
                        vec![14],
                        vec![15],
                        vec![16, 17],
                    ],
                    vec![-1, -1, -1, -1, -1, -1, -1, 0, -1, -1, -1, -1, -1, -1, -1, 1],
                    vec![1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0],
                    vec![vec![7, 8], vec![16, 17]],
                ),
                Mode::POPN_5K | Mode::POPN_9K => (
                    vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                    vec![
                        vec![0],
                        vec![1],
                        vec![2],
                        vec![3],
                        vec![4],
                        vec![5],
                        vec![6],
                        vec![7],
                        vec![8],
                    ],
                    vec![-1, -1, -1, -1, -1, -1, -1, -1, -1],
                    vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
                    vec![],
                ),
                Mode::KEYBOARD_24K => {
                    let mut key_to_lane = vec![0i32; 26];
                    let mut lane_to_key = vec![vec![0i32]; 26];
                    let mut lane_to_scratch = vec![0i32; 26];
                    let mut lane_to_skin_offset = vec![0i32; 26];
                    for i in 0..26 {
                        key_to_lane[i] = i as i32;
                        lane_to_key[i] = vec![i as i32];
                        lane_to_scratch[i] = -1;
                        lane_to_skin_offset[i] = i as i32 + 1;
                    }
                    (
                        key_to_lane,
                        lane_to_key,
                        lane_to_scratch,
                        lane_to_skin_offset,
                        vec![],
                    )
                }
                Mode::KEYBOARD_24K_DOUBLE => {
                    let mut key_to_lane = vec![0i32; 52];
                    let mut lane_to_key = vec![vec![0i32]; 52];
                    let mut lane_to_scratch = vec![0i32; 52];
                    let mut lane_to_skin_offset = vec![0i32; 52];
                    for i in 0..52 {
                        key_to_lane[i] = i as i32;
                        lane_to_key[i] = vec![i as i32];
                        lane_to_scratch[i] = -1;
                        lane_to_skin_offset[i] = (i % 26) as i32 + 1;
                    }
                    (
                        key_to_lane,
                        lane_to_key,
                        lane_to_scratch,
                        lane_to_skin_offset,
                        vec![],
                    )
                }
            };

        let key = mode.key() as usize;
        let player_count = mode.player() as usize;
        debug_assert!(
            player_count > 0 && key.is_multiple_of(player_count),
            "key ({key}) must be evenly divisible by player_count ({player_count})"
        );
        let lane_to_player: Vec<i32> = (0..key)
            .map(|i| (i / (key / player_count)) as i32)
            .collect();

        LaneProperty {
            key_to_lane,
            lane_to_key,
            lane_to_scratch,
            lane_to_skin_offset,
            lane_to_player,
            scratch_to_key,
        }
    }

    pub fn key_lane_assign(&self) -> &[i32] {
        &self.key_to_lane
    }

    pub fn lane_key_assign(&self) -> &[Vec<i32>] {
        &self.lane_to_key
    }

    pub fn lane_scratch_assign(&self) -> &[i32] {
        &self.lane_to_scratch
    }

    pub fn lane_skin_offset(&self) -> &[i32] {
        &self.lane_to_skin_offset
    }

    pub fn lane_player(&self) -> &[i32] {
        &self.lane_to_player
    }

    pub fn scratch_key_assign(&self) -> &[Vec<i32>] {
        &self.scratch_to_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- BEAT_5K lane property tests ---

    #[test]
    fn beat_5k_lane_count() {
        let lp = LaneProperty::new(&Mode::BEAT_5K);
        // 5 keys + 1 scratch = 6 lanes
        assert_eq!(lp.lane_key_assign().len(), 6);
    }

    #[test]
    fn beat_5k_key_to_lane() {
        let lp = LaneProperty::new(&Mode::BEAT_5K);
        assert_eq!(lp.key_lane_assign(), &[0, 1, 2, 3, 4, 5, 5]);
    }

    #[test]
    fn beat_5k_scratch_lane() {
        let lp = LaneProperty::new(&Mode::BEAT_5K);
        let scratch = lp.lane_scratch_assign();
        // Lane 5 is scratch (index 0)
        assert_eq!(scratch[5], 0);
        // Lanes 0-4 are not scratch
        for &s in &scratch[..5] {
            assert_eq!(s, -1);
        }
    }

    #[test]
    fn beat_5k_scratch_keys() {
        let lp = LaneProperty::new(&Mode::BEAT_5K);
        let sk = lp.scratch_key_assign();
        assert_eq!(sk.len(), 1);
        assert_eq!(sk[0], vec![5, 6]);
    }

    #[test]
    fn beat_5k_skin_offset() {
        let lp = LaneProperty::new(&Mode::BEAT_5K);
        let offsets = lp.lane_skin_offset();
        // Scratch is offset 0, keys are 1-5
        assert_eq!(offsets[5], 0); // scratch
        assert_eq!(offsets[0], 1); // key 1
    }

    #[test]
    fn beat_5k_player_mapping() {
        let lp = LaneProperty::new(&Mode::BEAT_5K);
        let players = lp.lane_player();
        // All lanes belong to player 0
        for &p in players {
            assert_eq!(p, 0);
        }
    }

    // --- BEAT_7K lane property tests ---

    #[test]
    fn beat_7k_lane_count() {
        let lp = LaneProperty::new(&Mode::BEAT_7K);
        // 7 keys + 1 scratch = 8 lanes
        assert_eq!(lp.lane_key_assign().len(), 8);
    }

    #[test]
    fn beat_7k_key_to_lane() {
        let lp = LaneProperty::new(&Mode::BEAT_7K);
        assert_eq!(lp.key_lane_assign(), &[0, 1, 2, 3, 4, 5, 6, 7, 7]);
    }

    #[test]
    fn beat_7k_scratch_lane() {
        let lp = LaneProperty::new(&Mode::BEAT_7K);
        let scratch = lp.lane_scratch_assign();
        assert_eq!(scratch[7], 0); // Lane 7 is scratch
        for &s in &scratch[..7] {
            assert_eq!(s, -1);
        }
    }

    #[test]
    fn beat_7k_scratch_keys() {
        let lp = LaneProperty::new(&Mode::BEAT_7K);
        let sk = lp.scratch_key_assign();
        assert_eq!(sk.len(), 1);
        assert_eq!(sk[0], vec![7, 8]);
    }

    #[test]
    fn beat_7k_player_mapping() {
        let lp = LaneProperty::new(&Mode::BEAT_7K);
        let players = lp.lane_player();
        assert_eq!(players.len(), 8);
        for &p in players {
            assert_eq!(p, 0);
        }
    }

    // --- BEAT_10K lane property tests ---

    #[test]
    fn beat_10k_lane_count() {
        let lp = LaneProperty::new(&Mode::BEAT_10K);
        // 5+5 keys + 2 scratches = 12 lanes
        assert_eq!(lp.lane_key_assign().len(), 12);
    }

    #[test]
    fn beat_10k_scratch_lanes() {
        let lp = LaneProperty::new(&Mode::BEAT_10K);
        let scratch = lp.lane_scratch_assign();
        assert_eq!(scratch[5], 0); // Player 1 scratch
        assert_eq!(scratch[11], 1); // Player 2 scratch
    }

    #[test]
    fn beat_10k_two_scratches() {
        let lp = LaneProperty::new(&Mode::BEAT_10K);
        let sk = lp.scratch_key_assign();
        assert_eq!(sk.len(), 2);
        assert_eq!(sk[0], vec![5, 6]);
        assert_eq!(sk[1], vec![12, 13]);
    }

    #[test]
    fn beat_10k_player_mapping() {
        let lp = LaneProperty::new(&Mode::BEAT_10K);
        let players = lp.lane_player();
        assert_eq!(players.len(), 12);
        // First 6 lanes are player 0
        for &p in &players[..6] {
            assert_eq!(p, 0);
        }
        // Last 6 lanes are player 1
        for &p in &players[6..12] {
            assert_eq!(p, 1);
        }
    }

    // --- BEAT_14K lane property tests ---

    #[test]
    fn beat_14k_lane_count() {
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        // 7+7 keys + 2 scratches = 16 lanes
        assert_eq!(lp.lane_key_assign().len(), 16);
    }

    #[test]
    fn beat_14k_scratch_lanes() {
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        let scratch = lp.lane_scratch_assign();
        assert_eq!(scratch[7], 0); // Player 1 scratch
        assert_eq!(scratch[15], 1); // Player 2 scratch
    }

    #[test]
    fn beat_14k_player_mapping() {
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        let players = lp.lane_player();
        assert_eq!(players.len(), 16);
        for &p in &players[..8] {
            assert_eq!(p, 0);
        }
        for &p in &players[8..16] {
            assert_eq!(p, 1);
        }
    }

    // --- POPN lane property tests ---

    #[test]
    fn popn_9k_lane_count() {
        let lp = LaneProperty::new(&Mode::POPN_9K);
        assert_eq!(lp.lane_key_assign().len(), 9);
    }

    #[test]
    fn popn_9k_no_scratch() {
        let lp = LaneProperty::new(&Mode::POPN_9K);
        let scratch = lp.lane_scratch_assign();
        for &s in scratch {
            assert_eq!(s, -1);
        }
        assert!(lp.scratch_key_assign().is_empty());
    }

    #[test]
    fn popn_5k_lane_count() {
        let lp = LaneProperty::new(&Mode::POPN_5K);
        // POPN_5K uses same mapping as POPN_9K (9 lanes)
        assert_eq!(lp.lane_key_assign().len(), 9);
    }

    #[test]
    fn popn_5k_no_scratch() {
        let lp = LaneProperty::new(&Mode::POPN_5K);
        assert!(lp.scratch_key_assign().is_empty());
    }

    // --- KEYBOARD_24K lane property tests ---

    #[test]
    fn keyboard_24k_lane_count() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K);
        assert_eq!(lp.lane_key_assign().len(), 26);
    }

    #[test]
    fn keyboard_24k_no_scratch() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K);
        let scratch = lp.lane_scratch_assign();
        for &s in scratch {
            assert_eq!(s, -1);
        }
        assert!(lp.scratch_key_assign().is_empty());
    }

    #[test]
    fn keyboard_24k_identity_key_to_lane() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K);
        let mapping = lp.key_lane_assign();
        for (i, &lane) in mapping.iter().enumerate() {
            assert_eq!(lane, i as i32);
        }
    }

    #[test]
    fn keyboard_24k_player_mapping() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K);
        let players = lp.lane_player();
        assert_eq!(players.len(), 26);
        for &p in players {
            assert_eq!(p, 0);
        }
    }

    // --- KEYBOARD_24K_DOUBLE lane property tests ---

    #[test]
    fn keyboard_24k_double_lane_count() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K_DOUBLE);
        assert_eq!(lp.lane_key_assign().len(), 52);
    }

    #[test]
    fn keyboard_24k_double_no_scratch() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K_DOUBLE);
        assert!(lp.scratch_key_assign().is_empty());
    }

    #[test]
    fn keyboard_24k_double_player_mapping() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K_DOUBLE);
        let players = lp.lane_player();
        assert_eq!(players.len(), 52);
        for &p in &players[..26] {
            assert_eq!(p, 0);
        }
        for &p in &players[26..52] {
            assert_eq!(p, 1);
        }
    }

    #[test]
    fn keyboard_24k_double_skin_offset_wraps() {
        let lp = LaneProperty::new(&Mode::KEYBOARD_24K_DOUBLE);
        let offsets = lp.lane_skin_offset();
        // Offsets wrap at 26: lane 0 -> 1, lane 26 -> 1
        assert_eq!(offsets[0], 1);
        assert_eq!(offsets[26], 1);
        assert_eq!(offsets[25], 26);
        assert_eq!(offsets[51], 26);
    }

    // --- Clone test ---

    #[test]
    fn lane_property_is_cloneable() {
        let lp = LaneProperty::new(&Mode::BEAT_7K);
        let lp2 = lp.clone();
        assert_eq!(lp.key_lane_assign(), lp2.key_lane_assign());
        assert_eq!(lp.lane_player(), lp2.lane_player());
    }

    /// Regression: verify that key % player_count == 0 holds for all Mode variants,
    /// ensuring the lane_to_player division never panics.
    #[test]
    fn all_modes_satisfy_lane_to_player_invariant() {
        let modes = [
            Mode::BEAT_5K,
            Mode::BEAT_7K,
            Mode::BEAT_10K,
            Mode::BEAT_14K,
            Mode::POPN_5K,
            Mode::POPN_9K,
            Mode::KEYBOARD_24K,
            Mode::KEYBOARD_24K_DOUBLE,
        ];
        for mode in &modes {
            let key = mode.key() as usize;
            let player_count = mode.player() as usize;
            assert!(
                player_count > 0 && key.is_multiple_of(player_count),
                "Mode {:?}: key={key} is not evenly divisible by player_count={player_count}",
                mode
            );
            // Also verify construction succeeds
            let _lp = LaneProperty::new(mode);
        }
    }
}
