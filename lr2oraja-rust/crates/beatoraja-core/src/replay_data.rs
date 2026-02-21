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
