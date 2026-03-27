use crate::ir::{ImGuiNotify, LR2Random, Random};

/// LR2 ghost data
///
/// Translated from: LR2GhostData.java
#[derive(Clone, Debug)]
pub struct LR2GhostData {
    random: Random,
    seed: i32,
    lane_order: i32,
    judgements: Vec<i32>,
    pgreat: i32,
    great: i32,
    good: i32,
    bad: i32,
    poor: i32,
}

/// Parameters for constructing LR2GhostData.
struct LR2GhostDataParams {
    pub random: Random,
    pub seed: i32,
    pub lanes: i32,
    pub judgements: Vec<i32>,
    pub pgreat: i32,
    pub great: i32,
    pub good: i32,
    pub bad: i32,
    pub poor: i32,
}

impl LR2GhostData {
    fn new(params: LR2GhostDataParams) -> Self {
        Self {
            random: params.random,
            seed: params.seed,
            lane_order: params.lanes,
            judgements: params.judgements,
            pgreat: params.pgreat,
            great: params.great,
            good: params.good,
            bad: params.bad,
            poor: params.poor,
        }
    }

    pub fn random(&self) -> Random {
        self.random
    }

    pub fn lane_order(&self) -> i32 {
        self.lane_order
    }

    pub fn seed(&self) -> i32 {
        self.seed
    }

    pub fn judgements(&self) -> &[i32] {
        &self.judgements
    }

    pub fn pgreat(&self) -> i32 {
        self.pgreat
    }

    pub fn great(&self) -> i32 {
        self.great
    }

    pub fn good(&self) -> i32 {
        self.good
    }

    pub fn bad(&self) -> i32 {
        self.bad
    }

    pub fn poor(&self) -> i32 {
        self.poor
    }

    pub fn parse(ghost_csv: &str) -> Option<Self> {
        // CSV parsing: format is "name,options,seed,ghost"
        // We parse manually since we don't have apache commons csv
        let lines: Vec<&str> = ghost_csv.lines().collect();
        if lines.is_empty() {
            ImGuiNotify::error("LR2IR returned empty response.");
            return None;
        }

        // Skip header line if present, get first data line
        let data_line = if lines.len() > 1 { lines[1] } else { lines[0] };

        // Parse CSV fields
        let fields: Vec<&str> = data_line.splitn(4, ',').collect();
        if fields.len() < 4 {
            ImGuiNotify::error("Could not parse ghost data response from LR2IR.");
            return None;
        }

        // option field is a 4-digit decimal that encodes options
        // starting with least significant digit: gauge, random 1, random 2, dpflip
        let options: i32 = match fields[1].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                ImGuiNotify::error("Could not parse ghost data response from LR2IR.");
                return None;
            }
        };

        // random: 0 nonrand, 1 mirror, 2 random, 3 sran, 4 hran, 5 converge
        let random_val = (options / 10) % 10;
        // for now, we only support mirror and random, and only SP
        if 3 <= random_val {
            ImGuiNotify::warning(&format!("Unsupported random option: {}", random_val));
            return None;
        }

        let random_option = match random_val {
            1 => Random::Mirror,
            2 => Random::Random,
            _ => Random::Identity,
        };

        // generate a proper lane ordering from the given seed
        let seed: i32 = match fields[2].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                ImGuiNotify::error("Could not parse ghost data response from LR2IR.");
                return None;
            }
        };
        let mut rng = LR2Random::with_seed(seed);
        let mut targets: [i32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        for lane in 1..7 {
            let rand_val = rng.next_int(7 - lane as i32 + 1);
            let swap = ((lane as i32) + rand_val).clamp(1, 7) as usize;
            targets.swap(lane, swap);
        }
        let mut lanes: [i32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        for (i, &target) in targets[1..8].iter().enumerate() {
            if let Some(slot) = lanes.get_mut(target as usize) {
                *slot = (i + 1) as i32;
            }
        }
        // we store the lane order as a decimal where the digits
        // encode the lanes left-to-right, and most to least significant
        let mut encoded_lanes = 0i32;
        for &lane in &lanes[1..8] {
            encoded_lanes = encoded_lanes * 10 + lane;
        }

        let judgements = Self::decode_play_ghost(fields[3].trim());

        let mut pgreat = 0;
        let mut great = 0;
        let mut good = 0;
        let mut bad = 0;
        let mut poor = 0;
        for &judge in &judgements {
            match judge {
                0 => pgreat += 1,
                1 => great += 1,
                2 => good += 1,
                3 => bad += 1,
                _ => poor += 1,
            }
        }

        Some(Self::new(LR2GhostDataParams {
            random: random_option,
            seed,
            lanes: encoded_lanes,
            judgements,
            pgreat,
            great,
            good,
            bad,
            poor,
        }))
    }

    pub fn decode_play_ghost(data: &str) -> Vec<i32> {
        let mut data = data.to_string();
        data = data.replace("q", "XX");
        data = data.replace("r", "X1");
        data = data.replace("s", "X2");
        data = data.replace("t", "X3");
        data = data.replace("u", "X4");
        data = data.replace("v", "X5");
        data = data.replace("w", "X6");
        data = data.replace("x", "X7");
        data = data.replace("y", "X8");
        data = data.replace("z", "X9");

        data = data.replace("F", "E1");
        data = data.replace("G", "E2");
        data = data.replace("H", "E3");
        data = data.replace("I", "E4");
        data = data.replace("J", "E5");
        data = data.replace("K", "E6");
        data = data.replace("L", "E7");
        data = data.replace("M", "E8");
        data = data.replace("N", "E9");
        data = data.replace("P", "EC");
        data = data.replace("Q", "EB");
        data = data.replace("R", "EA");
        data = data.replace("S", "D2");
        data = data.replace("T", "D3");
        data = data.replace("U", "D4");
        data = data.replace("V", "D5");
        data = data.replace("W", "D6");
        data = data.replace("X", "DE");
        data = data.replace("Y", "DC");
        data = data.replace("a", "DB");
        data = data.replace("b", "DA");
        data = data.replace("c", "C2");
        data = data.replace("d", "C3");
        data = data.replace("e", "C4");
        data = data.replace("f", "C5");
        data = data.replace("g", "CE");
        data = data.replace("h", "CD");
        data = data.replace("i", "CB");
        data = data.replace("j", "CA");
        data = data.replace("k", "AB");
        data = data.replace("l", "AC");
        data = data.replace("m", "AD");
        data = data.replace("n", "AE");
        data = data.replace("o", "A2");
        data = data.replace("p", "A3");

        // guard character to slightly simplify the loop
        // (the ghost data already seems to have guards,
        //  so this is just to avoid relying on them)
        data.push('?');

        // after all the substitutions in the first part of this function,
        // the ghost description is now a simple run-length encoded sequence
        // of judgements - for example, ED3CE2 translates to EDDDCEE
        // the following loop performs this decoding
        let mut notes: Vec<char> = Vec::new();
        let mut run_length: i32 = 0;
        let mut current_character: char = '\0';
        for next in data.chars() {
            if next == '?' {
                if current_character != '\0' {
                    if run_length == 0 {
                        run_length = 1;
                    }
                    for _ in 0..run_length {
                        notes.push(current_character);
                    }
                }
                break;
            } else if next.is_ascii_digit() {
                run_length = run_length
                    .saturating_mul(10)
                    .saturating_add(next as i32 - '0' as i32);
            } else if ('@'..='E').contains(&next) {
                if current_character == '\0' {
                    current_character = next;
                    run_length = 0;
                } else {
                    if run_length == 0 {
                        run_length = 1;
                    }
                    for _ in 0..run_length {
                        notes.push(current_character);
                    }
                    current_character = next;
                    run_length = 0;
                }
            } else {
                // we do ignore some characters
            }
        }

        let mut ghost = Vec::new();
        for &ch in &notes {
            match ch {
                'E' => ghost.push(0), // pgreat
                'D' => ghost.push(1), // great
                'C' => ghost.push(2), // good
                'B' => ghost.push(3), // bad
                'A' => ghost.push(4), // poor
                // mash poors ('@') and other chars are skipped
                _ => {}
            }
        }

        ghost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_play_ghost_all_pgreats() {
        // 'E' = pgreat (0), "E3" = 3 pgreats
        let result = LR2GhostData::decode_play_ghost("E3");
        assert_eq!(result, vec![0, 0, 0]);
    }

    #[test]
    fn test_decode_play_ghost_mixed_judgements() {
        // E=pgreat(0), D=great(1), C=good(2), B=bad(3), A=poor(4)
        // "EDCBA" = one of each
        let result = LR2GhostData::decode_play_ghost("EDCBA");
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_decode_play_ghost_run_length_encoding() {
        // "E2D3" = 2 pgreats + 3 greats
        let result = LR2GhostData::decode_play_ghost("E2D3");
        assert_eq!(result, vec![0, 0, 1, 1, 1]);
    }

    #[test]
    fn test_decode_play_ghost_shorthand_substitutions() {
        // 'F' expands to "E1" = 1 pgreat
        // 'S' expands to "D2" = 2 greats
        let result = LR2GhostData::decode_play_ghost("FS");
        // F -> E1 -> E then 1 -> 1 pgreat
        // S -> D2 -> D then 2 -> 2 greats
        assert_eq!(result, vec![0, 1, 1]);
    }

    #[test]
    fn test_decode_play_ghost_empty_input() {
        let result = LR2GhostData::decode_play_ghost("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_play_ghost_at_sign_excluded() {
        // '@' notes (mash poors) are skipped in the final ghost array
        let result = LR2GhostData::decode_play_ghost("@E");
        assert_eq!(result, vec![0]); // only the E (pgreat) should appear
    }

    #[test]
    fn test_parse_valid_csv_identity_random() {
        // CSV format: header line, then data line: "name,options,seed,ghost"
        // options=0 means gauge=0, random1=0 (identity), random2=0, dpflip=0
        let csv = "name,option,seed,ghost\nplayer1,0,12345,E3D2";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_some());
        let ghost = ghost.unwrap();
        assert_eq!(ghost.random(), Random::Identity);
        assert_eq!(ghost.seed(), 12345);
        assert_eq!(ghost.pgreat(), 3);
        assert_eq!(ghost.great(), 2);
        assert_eq!(ghost.good(), 0);
        assert_eq!(ghost.bad(), 0);
        assert_eq!(ghost.poor(), 0);
    }

    #[test]
    fn test_parse_mirror_random_option() {
        // options digit encoding: gauge(1s) random1(10s) random2(100s) dpflip(1000s)
        // random1 = 1 (mirror) means options = 10
        let csv = "name,option,seed,ghost\nplayer1,10,99,E2";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_some());
        assert_eq!(ghost.unwrap().random(), Random::Mirror);
    }

    #[test]
    fn test_parse_random_option() {
        // random1 = 2 (random) means options = 20
        let csv = "name,option,seed,ghost\nplayer1,20,99,E2";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_some());
        assert_eq!(ghost.unwrap().random(), Random::Random);
    }

    #[test]
    fn test_parse_unsupported_random_returns_none() {
        // random1 = 3 (sran) is unsupported
        let csv = "name,option,seed,ghost\nplayer1,30,99,E2";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_none());
    }

    #[test]
    fn test_parse_empty_input_returns_none() {
        let ghost = LR2GhostData::parse("");
        assert!(ghost.is_none());
    }

    #[test]
    fn test_parse_insufficient_fields_returns_none() {
        let csv = "name,option,seed,ghost\nplayer1,10";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_none());
    }

    #[test]
    fn test_parse_judgement_counts() {
        // "EDCBA" = one pgreat, one great, one good, one bad, one poor
        let csv = "name,option,seed,ghost\nplayer1,0,1,EDCBA";
        let ghost = LR2GhostData::parse(csv).unwrap();
        assert_eq!(ghost.pgreat(), 1);
        assert_eq!(ghost.great(), 1);
        assert_eq!(ghost.good(), 1);
        assert_eq!(ghost.bad(), 1);
        assert_eq!(ghost.poor(), 1);
        assert_eq!(ghost.judgements().len(), 5);
    }

    #[test]
    fn test_parse_lane_randomization_stays_in_bounds() {
        // Verify that lane randomization produces valid lane orders for
        // many seeds, including extreme values. Each digit of the encoded
        // lane order must be in [1, 7] and the set must be a permutation
        // of {1..7}.
        let test_seeds: Vec<i32> = (0..200)
            .chain(
                [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1, -2]
                    .iter()
                    .copied(),
            )
            .collect();
        for seed in test_seeds {
            let csv = format!("name,option,seed,ghost\nplayer1,0,{},E2", seed);
            let ghost = LR2GhostData::parse(&csv);
            assert!(ghost.is_some(), "parse failed for seed={}", seed);
            let ghost = ghost.unwrap();
            let mut lane_order = ghost.lane_order();
            let mut digits = Vec::new();
            for _ in 0..7 {
                digits.push(lane_order % 10);
                lane_order /= 10;
            }
            digits.reverse();
            for (i, &d) in digits.iter().enumerate() {
                assert!(
                    (1..=7).contains(&d),
                    "seed={}: lane digit {} is {} (out of [1,7])",
                    seed,
                    i,
                    d
                );
            }
            let mut sorted = digits.clone();
            sorted.sort();
            assert_eq!(
                sorted,
                vec![1, 2, 3, 4, 5, 6, 7],
                "seed={}: lane order is not a valid permutation: {:?}",
                seed,
                digits
            );
        }
    }

    #[test]
    fn test_parse_shift_jis_decoded_csv_with_japanese_name() {
        // Simulate the ghost_data() HTTP response path: server sends Shift_JIS
        // bytes, we decode with encoding_rs::SHIFT_JIS, then parse the CSV.
        // The player name field contains Japanese characters.
        //
        // Shift_JIS encoding of the CSV line with Japanese player name:
        // Header: "name,option,seed,ghost\n"
        // Data:   "<Japanese name>,0,1,E3D2"
        //
        // We encode the CSV as Shift_JIS bytes and decode back to verify
        // the full path produces correct results.
        let csv_utf8 = "name,option,seed,ghost\n\u{30d7}\u{30ec}\u{30a4}\u{30e4}\u{30fc},0,1,E3D2";
        let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(csv_utf8);
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&encoded);
        let ghost = LR2GhostData::parse(&decoded);
        assert!(
            ghost.is_some(),
            "parse should succeed with Shift_JIS decoded Japanese CSV"
        );
        let ghost = ghost.unwrap();
        assert_eq!(ghost.pgreat(), 3);
        assert_eq!(ghost.great(), 2);
    }
}
