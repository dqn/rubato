use crate::{ImGuiNotify, LR2Random, Random};

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

impl LR2GhostData {
    #[allow(clippy::too_many_arguments)]
    fn new(
        random: Random,
        seed: i32,
        lanes: i32,
        judgements: Vec<i32>,
        pgreat: i32,
        great: i32,
        good: i32,
        bad: i32,
        poor: i32,
    ) -> Self {
        Self {
            random,
            seed,
            lane_order: lanes,
            judgements,
            pgreat,
            great,
            good,
            bad,
            poor,
        }
    }

    pub fn get_random(&self) -> Random {
        self.random
    }

    pub fn get_lane_order(&self) -> i32 {
        self.lane_order
    }

    pub fn get_seed(&self) -> i32 {
        self.seed
    }

    pub fn get_judgements(&self) -> &[i32] {
        &self.judgements
    }

    pub fn get_pgreat(&self) -> i32 {
        self.pgreat
    }

    pub fn get_great(&self) -> i32 {
        self.great
    }

    pub fn get_good(&self) -> i32 {
        self.good
    }

    pub fn get_bad(&self) -> i32 {
        self.bad
    }

    pub fn get_poor(&self) -> i32 {
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
            let swap = lane + rng.next_int(7 - lane as i32 + 1) as usize;
            targets.swap(lane, swap);
        }
        let mut lanes: [i32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        for (i, &target) in targets[1..8].iter().enumerate() {
            lanes[target as usize] = (i + 1) as i32;
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

        Some(Self::new(
            random_option,
            seed,
            encoded_lanes,
            judgements,
            pgreat,
            great,
            good,
            bad,
            poor,
        ))
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
                run_length = run_length * 10 + (next as i32 - '0' as i32);
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

        let mut extra = 0;
        for &ch in &notes {
            if ch == '@' {
                extra += 1;
            }
        }

        let note_count = notes.len() - extra;
        let mut ghost = vec![0i32; note_count];
        let mut n = 0;
        for &ch in &notes {
            match ch {
                'E' => {
                    ghost[n] = 0; // pgreat
                    n += 1;
                }
                'D' => {
                    ghost[n] = 1; // great
                    n += 1;
                }
                'C' => {
                    ghost[n] = 2; // good
                    n += 1;
                }
                'B' => {
                    ghost[n] = 3; // bad
                    n += 1;
                }
                'A' => {
                    ghost[n] = 4; // poor
                    n += 1;
                }
                // mash poors
                // '@' => { ghost[n] = 5; n += 1; }
                _ => {
                    continue;
                }
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
        assert_eq!(ghost.get_random(), Random::Identity);
        assert_eq!(ghost.get_seed(), 12345);
        assert_eq!(ghost.get_pgreat(), 3);
        assert_eq!(ghost.get_great(), 2);
        assert_eq!(ghost.get_good(), 0);
        assert_eq!(ghost.get_bad(), 0);
        assert_eq!(ghost.get_poor(), 0);
    }

    #[test]
    fn test_parse_mirror_random_option() {
        // options digit encoding: gauge(1s) random1(10s) random2(100s) dpflip(1000s)
        // random1 = 1 (mirror) means options = 10
        let csv = "name,option,seed,ghost\nplayer1,10,99,E2";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_some());
        assert_eq!(ghost.unwrap().get_random(), Random::Mirror);
    }

    #[test]
    fn test_parse_random_option() {
        // random1 = 2 (random) means options = 20
        let csv = "name,option,seed,ghost\nplayer1,20,99,E2";
        let ghost = LR2GhostData::parse(csv);
        assert!(ghost.is_some());
        assert_eq!(ghost.unwrap().get_random(), Random::Random);
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
        assert_eq!(ghost.get_pgreat(), 1);
        assert_eq!(ghost.get_great(), 1);
        assert_eq!(ghost.get_good(), 1);
        assert_eq!(ghost.get_bad(), 1);
        assert_eq!(ghost.get_poor(), 1);
        assert_eq!(ghost.get_judgements().len(), 5);
    }
}
