use std::io::{Read, Write};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::play_config::PlayConfig;
use crate::stubs::{KeyInputLog, PatternModifyLog};
use crate::validatable::Validatable;

/// Replay data. Contains key input log, pattern modification info, and gauge type.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReplayData {
    pub player: Option<String>,
    pub sha256: Option<String>,
    pub mode: i32,
    pub keylog: Vec<KeyInputLog>,
    pub keyinput: Option<String>,
    pub gauge: i32,
    pub pattern: Option<Vec<PatternModifyLog>>,
    pub lane_shuffle_pattern: Option<Vec<Vec<i32>>>,
    pub rand: Vec<i32>,
    pub date: i64,
    pub seven_to_nine_pattern: i32,
    pub randomoption: i32,
    pub randomoptionseed: i64,
    pub randomoption2: i32,
    pub randomoption2seed: i64,
    pub doubleoption: i32,
    pub config: Option<PlayConfig>,
}

impl ReplayData {
    pub fn new() -> Self {
        Self {
            randomoptionseed: -1,
            randomoption2seed: -1,
            ..Default::default()
        }
    }

    pub fn shrink(&mut self) {
        if self.keylog.is_empty() {
            return;
        }
        let mut keyinputdata: Vec<u8> = Vec::with_capacity(self.keylog.len() * 9);
        for log in &self.keylog {
            let keycode_byte = ((log.keycode + 1) * if log.pressed { 1 } else { -1 }) as i8 as u8;
            keyinputdata.push(keycode_byte);
            keyinputdata.extend_from_slice(&log.time.to_le_bytes());
        }

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        if encoder.write_all(&keyinputdata).is_err() {
            return;
        }
        if let Ok(compressed) = encoder.finish() {
            self.keyinput = Some(URL_SAFE.encode(&compressed));
            self.keylog = Vec::new();
        }
    }
}

impl Validatable for ReplayData {
    fn validate(&mut self) -> bool {
        if let Some(keyinput) = self.keyinput.take()
            && let Ok(decoded) = URL_SAFE.decode(keyinput.as_bytes())
        {
            let mut gz = GzDecoder::new(&decoded[..]);
            let mut decompressed = Vec::new();
            if gz.read_to_end(&mut decompressed).is_ok() {
                let mut keylogarray = Vec::with_capacity(decompressed.len() / 9);
                let mut pos = 0;
                while pos + 9 <= decompressed.len() {
                    let keycode = decompressed[pos] as i8;
                    pos += 1;
                    let time = i64::from_le_bytes([
                        decompressed[pos],
                        decompressed[pos + 1],
                        decompressed[pos + 2],
                        decompressed[pos + 3],
                        decompressed[pos + 4],
                        decompressed[pos + 5],
                        decompressed[pos + 6],
                        decompressed[pos + 7],
                    ]);
                    pos += 8;
                    keylogarray.push(KeyInputLog {
                        time,
                        keycode: (keycode as i32).unsigned_abs() as i32 - 1,
                        pressed: keycode >= 0,
                    });
                }
                self.keylog = keylogarray;
            }
        }

        self.keylog.retain(|log| log.validate());
        if let Some(ref mut pattern) = self.pattern {
            pattern.retain(|p| p.validate());
        }
        !self.keylog.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_data_new() {
        let rd = ReplayData::new();
        assert!(rd.player.is_none());
        assert!(rd.sha256.is_none());
        assert_eq!(rd.mode, 0);
        assert!(rd.keylog.is_empty());
        assert!(rd.keyinput.is_none());
        assert_eq!(rd.gauge, 0);
        assert!(rd.pattern.is_none());
        assert!(rd.lane_shuffle_pattern.is_none());
        assert!(rd.rand.is_empty());
        assert_eq!(rd.date, 0);
        assert_eq!(rd.seven_to_nine_pattern, 0);
        assert_eq!(rd.randomoption, 0);
        assert_eq!(rd.randomoptionseed, -1);
        assert_eq!(rd.randomoption2, 0);
        assert_eq!(rd.randomoption2seed, -1);
        assert_eq!(rd.doubleoption, 0);
        assert!(rd.config.is_none());
    }

    #[test]
    fn test_replay_data_default() {
        let rd = ReplayData::default();
        // Default doesn't set randomoptionseed to -1 (new() does)
        assert_eq!(rd.randomoptionseed, 0);
        assert_eq!(rd.randomoption2seed, 0);
    }

    #[test]
    fn test_replay_data_serde_round_trip() {
        let mut rd = ReplayData::new();
        rd.player = Some("TestPlayer".to_string());
        rd.sha256 = Some("abc123hash".to_string());
        rd.mode = 7;
        rd.gauge = 3;
        rd.date = 1700000000;
        rd.rand = vec![1, 2, 3];
        rd.randomoption = 5;
        rd.randomoptionseed = 42;

        let json = serde_json::to_string(&rd).unwrap();
        let deserialized: ReplayData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.player.as_deref(), Some("TestPlayer"));
        assert_eq!(deserialized.sha256.as_deref(), Some("abc123hash"));
        assert_eq!(deserialized.mode, 7);
        assert_eq!(deserialized.gauge, 3);
        assert_eq!(deserialized.date, 1700000000);
        assert_eq!(deserialized.rand, vec![1, 2, 3]);
        assert_eq!(deserialized.randomoption, 5);
        assert_eq!(deserialized.randomoptionseed, 42);
    }

    #[test]
    fn test_replay_data_shrink_and_validate_round_trip() {
        let mut rd = ReplayData::new();
        rd.keylog = vec![
            KeyInputLog {
                time: 1000,
                keycode: 0,
                pressed: true,
            },
            KeyInputLog {
                time: 2000,
                keycode: 1,
                pressed: false,
            },
            KeyInputLog {
                time: 3000,
                keycode: 2,
                pressed: true,
            },
        ];

        // Shrink compresses keylog into keyinput string
        rd.shrink();
        assert!(rd.keylog.is_empty());
        assert!(rd.keyinput.is_some());

        // Validate decompresses keyinput back into keylog
        assert!(rd.validate());
        assert_eq!(rd.keylog.len(), 3);
        assert_eq!(rd.keylog[0].time, 1000);
        assert_eq!(rd.keylog[0].keycode, 0);
        assert!(rd.keylog[0].pressed);
        assert_eq!(rd.keylog[1].time, 2000);
        assert_eq!(rd.keylog[1].keycode, 1);
        assert!(!rd.keylog[1].pressed);
        assert_eq!(rd.keylog[2].time, 3000);
        assert_eq!(rd.keylog[2].keycode, 2);
        assert!(rd.keylog[2].pressed);
    }

    #[test]
    fn test_replay_data_validate_empty_keylog() {
        let mut rd = ReplayData::new();
        // No keylog and no keyinput => invalid
        assert!(!rd.validate());
    }

    #[test]
    fn test_replay_data_validate_with_keylog() {
        let mut rd = ReplayData::new();
        rd.keylog = vec![KeyInputLog {
            time: 100,
            keycode: 0,
            pressed: true,
        }];
        assert!(rd.validate());
    }

    #[test]
    fn test_replay_data_with_pattern() {
        let mut rd = ReplayData::new();
        rd.pattern = Some(vec![
            PatternModifyLog {
                old_lane: 0,
                new_lane: 3,
            },
            PatternModifyLog {
                old_lane: 1,
                new_lane: 2,
            },
        ]);

        let json = serde_json::to_string(&rd).unwrap();
        let deserialized: ReplayData = serde_json::from_str(&json).unwrap();
        let pattern = deserialized.pattern.unwrap();
        assert_eq!(pattern.len(), 2);
        assert_eq!(pattern[0].old_lane, 0);
        assert_eq!(pattern[0].new_lane, 3);
    }

    #[test]
    fn test_replay_data_with_config() {
        let mut rd = ReplayData::new();
        rd.config = Some(PlayConfig::default());

        let json = serde_json::to_string(&rd).unwrap();
        let deserialized: ReplayData = serde_json::from_str(&json).unwrap();
        assert!(deserialized.config.is_some());
        assert_eq!(deserialized.config.unwrap().hispeed, 1.0);
    }

    #[test]
    fn test_replay_data_lane_shuffle_pattern() {
        let mut rd = ReplayData::new();
        rd.lane_shuffle_pattern = Some(vec![vec![0, 1, 2], vec![2, 1, 0]]);

        let json = serde_json::to_string(&rd).unwrap();
        let deserialized: ReplayData = serde_json::from_str(&json).unwrap();
        let lsp = deserialized.lane_shuffle_pattern.unwrap();
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0], vec![0, 1, 2]);
        assert_eq!(lsp[1], vec![2, 1, 0]);
    }
}
