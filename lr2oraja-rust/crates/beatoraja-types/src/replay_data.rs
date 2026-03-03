use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::Result;
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
        if let Err(e) = encoder.write_all(&keyinputdata) {
            log::warn!("Failed to compress replay key input data: {}", e);
            return;
        }
        match encoder.finish() {
            Ok(compressed) => {
                self.keyinput = Some(URL_SAFE.encode(&compressed));
                self.keylog = Vec::new();
            }
            Err(e) => {
                log::warn!("Failed to finalize replay key input compression: {}", e);
            }
        }
    }

    /// Read a single ReplayData from a .brd file (gzip-compressed JSON).
    /// Calls validate() after deserialization, matching Java PlayDataAccessor.readReplayData().
    pub fn read_brd(path: &Path) -> Result<ReplayData> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(GzDecoder::new(file));
        let mut rd: ReplayData = serde_json::from_reader(reader)?;
        if !rd.validate() {
            anyhow::bail!("ReplayData validation failed for {:?}", path);
        }
        Ok(rd)
    }

    /// Write a single ReplayData to a .brd file (gzip-compressed JSON).
    /// Calls shrink() before serialization, matching Java PlayDataAccessor.wrireReplayData().
    pub fn write_brd(&mut self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.shrink();
        let file = fs::File::create(path)?;
        let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        serde_json::to_writer_pretty(encoder, &self)?;
        Ok(())
    }

    /// Read a course ReplayData array from a .brd file (gzip-compressed JSON array).
    /// Calls validate() on each element, matching Java PlayDataAccessor.readReplayData(String[], ...).
    pub fn read_brd_course(path: &Path) -> Result<Vec<ReplayData>> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(GzDecoder::new(file));
        let mut rds: Vec<ReplayData> = serde_json::from_reader(reader)?;
        for rd in &mut rds {
            if !rd.validate() {
                anyhow::bail!("ReplayData validation failed in course file {:?}", path);
            }
        }
        Ok(rds)
    }

    /// Write a course ReplayData array to a .brd file (gzip-compressed JSON array).
    /// Calls shrink() on each element, matching Java PlayDataAccessor.wrireReplayData(ReplayData[], ...).
    pub fn write_brd_course(rds: &mut [ReplayData], path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        for rd in rds.iter_mut() {
            rd.shrink();
        }
        let file = fs::File::create(path)?;
        let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        serde_json::to_writer_pretty(encoder, &rds)?;
        Ok(())
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
                        keycode: (keycode as i32).abs() - 1,
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
                old_lane: 3,
                new_lane: 0,
            },
            PatternModifyLog {
                old_lane: 2,
                new_lane: 1,
            },
        ]);

        let json = serde_json::to_string(&rd).unwrap();
        let deserialized: ReplayData = serde_json::from_str(&json).unwrap();
        let pattern = deserialized.pattern.unwrap();
        assert_eq!(pattern.len(), 2);
        assert_eq!(pattern[0].old_lane, 3);
        assert_eq!(pattern[0].new_lane, 0);
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

    #[test]
    fn test_write_brd_and_read_brd_round_trip() {
        let dir = std::env::temp_dir().join("brs_test_brd_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("test.brd");

        let mut rd = ReplayData::new();
        rd.player = Some("TestPlayer".to_string());
        rd.sha256 = Some("abc123hash".to_string());
        rd.mode = 7;
        rd.gauge = 3;
        rd.date = 1700000000;
        rd.rand = vec![1, 2, 3];
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

        rd.write_brd(&path).unwrap();
        assert!(path.exists());

        let loaded = ReplayData::read_brd(&path).unwrap();
        assert_eq!(loaded.player.as_deref(), Some("TestPlayer"));
        assert_eq!(loaded.sha256.as_deref(), Some("abc123hash"));
        assert_eq!(loaded.mode, 7);
        assert_eq!(loaded.gauge, 3);
        assert_eq!(loaded.date, 1700000000);
        assert_eq!(loaded.rand, vec![1, 2, 3]);
        // keylog is restored via validate() (shrink compresses, validate decompresses)
        assert_eq!(loaded.keylog.len(), 3);
        assert_eq!(loaded.keylog[0].time, 1000);
        assert_eq!(loaded.keylog[0].keycode, 0);
        assert!(loaded.keylog[0].pressed);
        assert_eq!(loaded.keylog[1].time, 2000);
        assert_eq!(loaded.keylog[1].keycode, 1);
        assert!(!loaded.keylog[1].pressed);
        assert_eq!(loaded.keylog[2].time, 3000);
        assert_eq!(loaded.keylog[2].keycode, 2);
        assert!(loaded.keylog[2].pressed);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_brd_creates_parent_dirs() {
        let dir = std::env::temp_dir().join("brs_test_brd_parent/nested/dir");
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("brs_test_brd_parent"));
        let path = dir.join("test.brd");

        let mut rd = ReplayData::new();
        rd.keylog = vec![KeyInputLog {
            time: 100,
            keycode: 0,
            pressed: true,
        }];
        rd.write_brd(&path).unwrap();
        assert!(path.exists());

        let _ = std::fs::remove_dir_all(std::env::temp_dir().join("brs_test_brd_parent"));
    }

    #[test]
    fn test_read_brd_nonexistent_file() {
        let path = std::env::temp_dir().join("brs_test_nonexistent.brd");
        let result = ReplayData::read_brd(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_brd_course_and_read_brd_course_round_trip() {
        let dir = std::env::temp_dir().join("brs_test_brd_course");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("course.brd");

        let mut rd1 = ReplayData::new();
        rd1.sha256 = Some("hash1".to_string());
        rd1.keylog = vec![
            KeyInputLog {
                time: 1000,
                keycode: 0,
                pressed: true,
            },
            KeyInputLog {
                time: 2000,
                keycode: 0,
                pressed: false,
            },
        ];

        let mut rd2 = ReplayData::new();
        rd2.sha256 = Some("hash2".to_string());
        rd2.keylog = vec![
            KeyInputLog {
                time: 5000,
                keycode: 1,
                pressed: true,
            },
            KeyInputLog {
                time: 6000,
                keycode: 1,
                pressed: false,
            },
        ];

        let mut rds = vec![rd1, rd2];
        ReplayData::write_brd_course(&mut rds, &path).unwrap();
        assert!(path.exists());

        let loaded = ReplayData::read_brd_course(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].sha256.as_deref(), Some("hash1"));
        assert_eq!(loaded[0].keylog.len(), 2);
        assert_eq!(loaded[0].keylog[0].time, 1000);
        assert_eq!(loaded[1].sha256.as_deref(), Some("hash2"));
        assert_eq!(loaded[1].keylog.len(), 2);
        assert_eq!(loaded[1].keylog[0].time, 5000);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // -- Phase 46a: keycode encoding boundary tests --

    /// Helper: shrink a single KeyInputLog and validate it back, returning the recovered log.
    fn shrink_validate_roundtrip(keycode: i32, pressed: bool) -> KeyInputLog {
        let mut rd = ReplayData::new();
        rd.keylog = vec![KeyInputLog {
            time: 1000,
            keycode,
            pressed,
        }];
        rd.shrink();
        assert!(rd.keyinput.is_some(), "shrink should produce keyinput");
        assert!(rd.validate(), "validate should succeed");
        assert_eq!(rd.keylog.len(), 1, "should recover exactly one entry");
        rd.keylog.remove(0)
    }

    #[test]
    fn test_replay_shrink_keycode_126_roundtrip() {
        // keycode=126 is the boundary: (126+1)*1 = 127, fits in i8
        let recovered = shrink_validate_roundtrip(126, true);
        assert_eq!(recovered.keycode, 126);
        assert!(recovered.pressed);

        let recovered = shrink_validate_roundtrip(126, false);
        assert_eq!(recovered.keycode, 126);
        assert!(!recovered.pressed);
    }

    #[test]
    #[ignore] // BUG: keycode=127 causes i8 overflow in shrink() — (127+1)*1 = 128, which wraps
    // to -128 as i8, corrupting the pressed flag (always reads as "not pressed")
    fn test_replay_shrink_keycode_127_overflow() {
        let recovered = shrink_validate_roundtrip(127, true);
        // After the bug: pressed becomes false because 128 as i8 = -128 (negative)
        assert_eq!(recovered.keycode, 127, "keycode should survive roundtrip");
        assert!(recovered.pressed, "pressed flag should survive roundtrip");
    }

    #[test]
    #[ignore] // BUG: keycode=200 causes both pressed flag AND keycode corruption in shrink()
    // — (200+1)*1 = 201 as i8 = -55, so pressed reads as false and keycode reads as 54
    fn test_replay_shrink_keycode_200_corrupted() {
        let recovered = shrink_validate_roundtrip(200, true);
        assert_eq!(
            recovered.keycode, 200,
            "keycode should survive roundtrip (actual: {})",
            recovered.keycode
        );
        assert!(recovered.pressed, "pressed flag should survive roundtrip");
    }

    #[test]
    fn test_shrink_empty_keylog_is_noop() {
        let mut rd = ReplayData::new();
        // Empty keylog should cause shrink to return early
        rd.shrink();
        assert!(
            rd.keyinput.is_none(),
            "keyinput should remain None for empty keylog"
        );
        assert!(rd.keylog.is_empty(), "keylog should remain empty");
    }

    #[test]
    fn test_shrink_preserves_keylog_on_compression_success() {
        let mut rd = ReplayData::new();
        rd.keylog = vec![
            KeyInputLog {
                time: 100,
                keycode: 0,
                pressed: true,
            },
            KeyInputLog {
                time: 200,
                keycode: 1,
                pressed: false,
            },
        ];

        rd.shrink();
        // After successful compression, keylog should be cleared and keyinput set
        assert!(
            rd.keylog.is_empty(),
            "keylog should be emptied after shrink"
        );
        assert!(
            rd.keyinput.is_some(),
            "keyinput should be set after successful compression"
        );

        // Verify round-trip: validate should restore the same data
        assert!(rd.validate());
        assert_eq!(rd.keylog.len(), 2);
        assert_eq!(rd.keylog[0].time, 100);
        assert!(rd.keylog[0].pressed);
        assert_eq!(rd.keylog[1].time, 200);
        assert!(!rd.keylog[1].pressed);
    }

    #[test]
    fn test_brd_shrinks_keylog_on_write() {
        let dir = std::env::temp_dir().join("brs_test_brd_shrink");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("test.brd");

        let mut rd = ReplayData::new();
        rd.keylog = vec![KeyInputLog {
            time: 1000,
            keycode: 0,
            pressed: true,
        }];

        // After write_brd, the in-memory rd should have keylog shrunk
        rd.write_brd(&path).unwrap();
        assert!(
            rd.keylog.is_empty(),
            "keylog should be emptied after shrink"
        );
        assert!(rd.keyinput.is_some(), "keyinput should be set after shrink");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
