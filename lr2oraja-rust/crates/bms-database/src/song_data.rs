use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use bms_model::{BmsDecoder, BmsModel, BmsonDecoder, NoteType, PlayMode};

// Feature flags (matches Java SongData.FEATURE_*)
pub const FEATURE_UNDEFINEDLN: i32 = 1;
pub const FEATURE_MINENOTE: i32 = 2;
pub const FEATURE_RANDOM: i32 = 4;
pub const FEATURE_LONGNOTE: i32 = 8;
pub const FEATURE_CHARGENOTE: i32 = 16;
pub const FEATURE_HELLCHARGENOTE: i32 = 32;
pub const FEATURE_STOPSEQUENCE: i32 = 64;
pub const FEATURE_SCROLL: i32 = 128;

// Content flags (matches Java SongData.CONTENT_*)
pub const CONTENT_TEXT: i32 = 1;
pub const CONTENT_BGA: i32 = 2;
pub const CONTENT_PREVIEW: i32 = 4;
pub const CONTENT_NOKEYSOUND: i32 = 128;

// Favorite flags
pub const FAVORITE_SONG: i32 = 1;
pub const FAVORITE_CHART: i32 = 2;
pub const INVISIBLE_SONG: i32 = 4;
pub const INVISIBLE_CHART: i32 = 8;

/// Song data stored in the song database.
///
/// Corresponds to Java `SongData` with 29 DB columns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongData {
    pub md5: String,
    pub sha256: String,
    pub title: String,
    pub subtitle: String,
    pub genre: String,
    pub artist: String,
    pub subartist: String,
    pub tag: String,
    pub path: String,
    pub folder: String,
    pub stagefile: String,
    pub banner: String,
    pub backbmp: String,
    pub preview: String,
    pub parent: String,
    pub level: i32,
    pub difficulty: i32,
    pub maxbpm: i32,
    pub minbpm: i32,
    pub length: i32,
    pub mode: i32,
    pub judge: i32,
    pub feature: i32,
    pub content: i32,
    pub date: i32,
    pub favorite: i32,
    pub adddate: i32,
    pub notes: i32,
    pub charthash: String,
    /// IPFS CID for the song (runtime-only, populated from IR responses).
    #[serde(default)]
    pub ipfs: String,
    /// IPFS CID for appended data (runtime-only, populated from IR responses).
    #[serde(default)]
    pub appendipfs: String,
}

impl SongData {
    /// Create SongData from a parsed BmsModel.
    pub fn from_model(model: &BmsModel, contains_txt: bool) -> Self {
        let mut content = if contains_txt { CONTENT_TEXT } else { 0 };
        let mut feature = 0i32;

        for note in &model.notes {
            match note.note_type {
                // Java uses lnmode (default 0 = TYPE_UNDEFINED) for LN type in SongData.
                // TYPE_UNDEFINED -> FEATURE_UNDEFINEDLN, TYPE_LONGNOTE -> FEATURE_LONGNOTE.
                // Since lnmode is always forced to 0 in Java, LongNote maps to UNDEFINEDLN.
                NoteType::LongNote => feature |= FEATURE_UNDEFINEDLN,
                NoteType::ChargeNote => feature |= FEATURE_CHARGENOTE,
                NoteType::HellChargeNote => feature |= FEATURE_HELLCHARGENOTE,
                NoteType::Mine => feature |= FEATURE_MINENOTE,
                _ => {}
            }
        }

        if !model.stop_events.is_empty() {
            feature |= FEATURE_STOPSEQUENCE;
        }

        if model.has_random {
            feature |= FEATURE_RANDOM;
        }

        // CONTENT_BGA: check if bmp_defs is non-empty
        if !model.bmp_defs.is_empty() {
            content |= CONTENT_BGA;
        }

        // CONTENT_NOKEYSOUND: length >= 30000ms and few wav defs
        // Use last_event_time_ms() to match Java's getLastTime()
        let length_ms = model.last_event_time_ms();
        if length_ms >= 30000 && model.wav_defs.len() as i32 <= (length_ms / 50000) + 3 {
            content |= CONTENT_NOKEYSOUND;
        }

        Self {
            md5: model.md5.clone(),
            sha256: model.sha256.clone(),
            title: model.title.clone(),
            subtitle: model.subtitle.clone(),
            genre: model.genre.clone(),
            artist: model.artist.clone(),
            subartist: model.sub_artist.clone(),
            tag: String::new(),
            path: String::new(),
            folder: String::new(),
            stagefile: model.stage_file.clone(),
            banner: model.banner.clone(),
            backbmp: model.back_bmp.clone(),
            preview: model.preview.clone(),
            parent: String::new(),
            level: model.play_level,
            difficulty: model.difficulty,
            maxbpm: model.max_bpm() as i32,
            minbpm: model.min_bpm() as i32,
            length: length_ms,
            mode: model.mode.mode_id(),
            judge: model.judge_rank_raw,
            feature,
            content,
            date: 0,
            favorite: 0,
            adddate: 0,
            notes: model.total_notes() as i32,
            charthash: String::new(),
            ipfs: String::new(),
            appendipfs: String::new(),
        }
    }

    /// Create SongData by parsing a BMS/BME/BML/PMS/BMSON file from disk.
    ///
    /// Sets `path`, `folder`, and `date` (file mtime as Unix seconds).
    /// Detects `.txt` companion files for the `CONTENT_TEXT` flag.
    pub fn from_file(file_path: &Path) -> Result<Self> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let model = match ext.as_str() {
            "bmson" => BmsonDecoder::decode(file_path)
                .with_context(|| format!("Failed to decode bmson: {}", file_path.display()))?,
            _ => BmsDecoder::decode(file_path)
                .with_context(|| format!("Failed to decode BMS: {}", file_path.display()))?,
        };

        // Check for companion .txt files
        let contains_txt = file_path
            .parent()
            .map(|dir| {
                dir.read_dir()
                    .map(|entries| {
                        entries.flatten().any(|e| {
                            e.path()
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .map(|s| s.eq_ignore_ascii_case("txt"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        let mut sd = Self::from_model(&model, contains_txt);

        // Set path and folder
        sd.path = file_path.to_string_lossy().to_string();
        if let Some(parent) = file_path.parent() {
            sd.folder = parent.to_string_lossy().to_string();
        }

        // Set date from file modification time (Unix seconds)
        if let Ok(metadata) = std::fs::metadata(file_path)
            && let Ok(mtime) = metadata.modified()
            && let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH)
        {
            sd.date = duration.as_secs() as i32;
        }

        Ok(sd)
    }

    /// Validate that required fields are present.
    pub fn validate(&self) -> bool {
        if self.title.is_empty() {
            return false;
        }
        if self.md5.is_empty() && self.sha256.is_empty() {
            return false;
        }
        true
    }

    pub fn full_title(&self) -> String {
        if self.subtitle.is_empty() {
            self.title.clone()
        } else {
            format!("{} {}", self.title, self.subtitle)
        }
    }

    pub fn has_random_sequence(&self) -> bool {
        self.feature & FEATURE_RANDOM != 0
    }

    pub fn has_mine_note(&self) -> bool {
        self.feature & FEATURE_MINENOTE != 0
    }

    pub fn has_undefined_long_note(&self) -> bool {
        self.feature & FEATURE_UNDEFINEDLN != 0
    }

    pub fn has_long_note(&self) -> bool {
        self.feature & FEATURE_LONGNOTE != 0
    }

    pub fn has_charge_note(&self) -> bool {
        self.feature & FEATURE_CHARGENOTE != 0
    }

    pub fn has_hell_charge_note(&self) -> bool {
        self.feature & FEATURE_HELLCHARGENOTE != 0
    }

    pub fn has_any_long_note(&self) -> bool {
        self.feature
            & (FEATURE_UNDEFINEDLN | FEATURE_LONGNOTE | FEATURE_CHARGENOTE | FEATURE_HELLCHARGENOTE)
            != 0
    }

    pub fn has_stop_sequence(&self) -> bool {
        self.feature & FEATURE_STOPSEQUENCE != 0
    }

    pub fn has_document(&self) -> bool {
        self.content & CONTENT_TEXT != 0
    }

    pub fn has_bga(&self) -> bool {
        self.content & CONTENT_BGA != 0
    }

    pub fn play_mode(&self) -> Option<PlayMode> {
        PlayMode::from_mode_id(self.mode)
    }
}

/// Read a SongData from a rusqlite row.
impl SongData {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            md5: row.get("md5")?,
            sha256: row.get("sha256")?,
            title: row.get("title")?,
            subtitle: row
                .get::<_, Option<String>>("subtitle")?
                .unwrap_or_default(),
            genre: row.get::<_, Option<String>>("genre")?.unwrap_or_default(),
            artist: row.get::<_, Option<String>>("artist")?.unwrap_or_default(),
            subartist: row
                .get::<_, Option<String>>("subartist")?
                .unwrap_or_default(),
            tag: row.get::<_, Option<String>>("tag")?.unwrap_or_default(),
            path: row.get("path")?,
            folder: row.get::<_, Option<String>>("folder")?.unwrap_or_default(),
            stagefile: row
                .get::<_, Option<String>>("stagefile")?
                .unwrap_or_default(),
            banner: row.get::<_, Option<String>>("banner")?.unwrap_or_default(),
            backbmp: row.get::<_, Option<String>>("backbmp")?.unwrap_or_default(),
            preview: row.get::<_, Option<String>>("preview")?.unwrap_or_default(),
            parent: row.get::<_, Option<String>>("parent")?.unwrap_or_default(),
            level: row.get::<_, Option<i32>>("level")?.unwrap_or(0),
            difficulty: row.get::<_, Option<i32>>("difficulty")?.unwrap_or(0),
            maxbpm: row.get::<_, Option<i32>>("maxbpm")?.unwrap_or(0),
            minbpm: row.get::<_, Option<i32>>("minbpm")?.unwrap_or(0),
            length: row.get::<_, Option<i32>>("length")?.unwrap_or(0),
            mode: row.get::<_, Option<i32>>("mode")?.unwrap_or(0),
            judge: row.get::<_, Option<i32>>("judge")?.unwrap_or(0),
            feature: row.get::<_, Option<i32>>("feature")?.unwrap_or(0),
            content: row.get::<_, Option<i32>>("content")?.unwrap_or(0),
            date: row.get::<_, Option<i32>>("date")?.unwrap_or(0),
            favorite: row.get::<_, Option<i32>>("favorite")?.unwrap_or(0),
            adddate: row.get::<_, Option<i32>>("adddate")?.unwrap_or(0),
            notes: row.get::<_, Option<i32>>("notes")?.unwrap_or(0),
            charthash: row
                .get::<_, Option<String>>("charthash")?
                .unwrap_or_default(),
            ipfs: row.get::<_, Option<String>>("ipfs")?.unwrap_or_default(),
            appendipfs: row
                .get::<_, Option<String>>("appendipfs")?
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use bms_model::StopEvent;
    use bms_model::{BmsModel, LnType, Note, PlayMode};

    use super::*;

    // -- from_model: feature flags --

    #[test]
    fn from_model_long_note_sets_feature_undefinedln() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model
            .notes
            .push(Note::long_note(0, 0, 1_000_000, 1, 1, LnType::LongNote));

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.feature & FEATURE_UNDEFINEDLN, 0);
    }

    #[test]
    fn from_model_charge_note_sets_feature_chargenote() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model
            .notes
            .push(Note::long_note(0, 0, 1_000_000, 1, 1, LnType::ChargeNote));

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.feature & FEATURE_CHARGENOTE, 0);
    }

    #[test]
    fn from_model_hell_charge_note_sets_feature_hellchargenote() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.notes.push(Note::long_note(
            0,
            0,
            1_000_000,
            1,
            1,
            LnType::HellChargeNote,
        ));

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.feature & FEATURE_HELLCHARGENOTE, 0);
    }

    #[test]
    fn from_model_mine_note_sets_feature_minenote() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.notes.push(Note::mine(0, 0, 1, 100));

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.feature & FEATURE_MINENOTE, 0);
    }

    #[test]
    fn from_model_stop_events_sets_feature_stopsequence() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.stop_events.push(StopEvent {
            time_us: 1_000_000,
            duration_ticks: 48,
            duration_us: 500_000,
        });

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.feature & FEATURE_STOPSEQUENCE, 0);
    }

    #[test]
    fn from_model_has_random_sets_feature_random() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.has_random = true;

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.feature & FEATURE_RANDOM, 0);
    }

    #[test]
    fn from_model_empty_model_has_no_feature_flags() {
        let model = BmsModel::default();
        let sd = SongData::from_model(&model, false);
        assert_eq!(sd.feature, 0);
    }

    // -- from_model: content flags --

    #[test]
    fn from_model_bmp_defs_sets_content_bga() {
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.bmp_defs.insert(1, PathBuf::from("bg.bmp"));

        let sd = SongData::from_model(&model, false);
        assert_ne!(sd.content & CONTENT_BGA, 0);
    }

    #[test]
    fn from_model_contains_txt_sets_content_text() {
        let model = BmsModel::default();
        let sd = SongData::from_model(&model, true);
        assert_ne!(sd.content & CONTENT_TEXT, 0);
    }

    #[test]
    fn from_model_contains_txt_false_no_content_text() {
        let model = BmsModel::default();
        let sd = SongData::from_model(&model, false);
        assert_eq!(sd.content & CONTENT_TEXT, 0);
    }

    #[test]
    fn from_model_nokeysound_at_threshold() {
        // length_ms >= 30000 and wav_defs.len() <= (length_ms / 50000) + 3
        // With time_us = 30_000_000 (30000ms) and 3 wav_defs:
        //   30000 >= 30000 = true
        //   3 <= (30000 / 50000) + 3 = 0 + 3 = 3 => true
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.notes.push(Note::normal(0, 30_000_000, 1));
        model.wav_defs = HashMap::from([
            (1, PathBuf::from("a.wav")),
            (2, PathBuf::from("b.wav")),
            (3, PathBuf::from("c.wav")),
        ]);

        let sd = SongData::from_model(&model, false);
        assert_ne!(
            sd.content & CONTENT_NOKEYSOUND,
            0,
            "should trigger NOKEYSOUND at 30000ms with 3 wav_defs"
        );
    }

    #[test]
    fn from_model_nokeysound_below_threshold() {
        // time_us = 29_999_000 => 29999ms < 30000 => should NOT trigger
        let mut model = BmsModel::default();
        model.md5 = "abc".into();
        model.title = "t".into();
        model.notes.push(Note::normal(0, 29_999_000, 1));
        model.wav_defs = HashMap::from([
            (1, PathBuf::from("a.wav")),
            (2, PathBuf::from("b.wav")),
            (3, PathBuf::from("c.wav")),
        ]);

        let sd = SongData::from_model(&model, false);
        assert_eq!(
            sd.content & CONTENT_NOKEYSOUND,
            0,
            "should NOT trigger NOKEYSOUND below 30000ms"
        );
    }

    // -- validate --

    #[test]
    fn validate_empty_title_returns_false() {
        let sd = SongData {
            title: String::new(),
            md5: "abc".into(),
            sha256: "def".into(),
            ..Default::default()
        };
        assert!(!sd.validate());
    }

    #[test]
    fn validate_both_hashes_empty_returns_false() {
        let sd = SongData {
            title: "test".into(),
            md5: String::new(),
            sha256: String::new(),
            ..Default::default()
        };
        assert!(!sd.validate());
    }

    #[test]
    fn validate_valid_song_data_returns_true() {
        let sd = SongData {
            title: "test".into(),
            md5: "abc".into(),
            ..Default::default()
        };
        assert!(sd.validate());
    }

    #[test]
    fn validate_sha256_only_returns_true() {
        let sd = SongData {
            title: "test".into(),
            sha256: "def".into(),
            ..Default::default()
        };
        assert!(sd.validate());
    }

    // -- full_title --

    #[test]
    fn full_title_with_subtitle() {
        let sd = SongData {
            title: "title".into(),
            subtitle: "subtitle".into(),
            ..Default::default()
        };
        assert_eq!(sd.full_title(), "title subtitle");
    }

    #[test]
    fn full_title_without_subtitle() {
        let sd = SongData {
            title: "title".into(),
            ..Default::default()
        };
        assert_eq!(sd.full_title(), "title");
    }

    // -- has_* methods via direct flag setting --

    #[test]
    fn has_random_sequence() {
        let sd = SongData {
            feature: FEATURE_RANDOM,
            ..Default::default()
        };
        assert!(sd.has_random_sequence());
    }

    #[test]
    fn has_mine_note() {
        let sd = SongData {
            feature: FEATURE_MINENOTE,
            ..Default::default()
        };
        assert!(sd.has_mine_note());
    }

    #[test]
    fn has_undefined_long_note() {
        let sd = SongData {
            feature: FEATURE_UNDEFINEDLN,
            ..Default::default()
        };
        assert!(sd.has_undefined_long_note());
    }

    #[test]
    fn has_long_note() {
        let sd = SongData {
            feature: FEATURE_LONGNOTE,
            ..Default::default()
        };
        assert!(sd.has_long_note());
    }

    #[test]
    fn has_charge_note() {
        let sd = SongData {
            feature: FEATURE_CHARGENOTE,
            ..Default::default()
        };
        assert!(sd.has_charge_note());
    }

    #[test]
    fn has_hell_charge_note() {
        let sd = SongData {
            feature: FEATURE_HELLCHARGENOTE,
            ..Default::default()
        };
        assert!(sd.has_hell_charge_note());
    }

    #[test]
    fn has_any_long_note_with_multiple_flags() {
        let sd = SongData {
            feature: FEATURE_CHARGENOTE | FEATURE_HELLCHARGENOTE,
            ..Default::default()
        };
        assert!(sd.has_any_long_note());
    }

    #[test]
    fn has_any_long_note_false_without_ln_flags() {
        let sd = SongData {
            feature: FEATURE_MINENOTE,
            ..Default::default()
        };
        assert!(!sd.has_any_long_note());
    }

    #[test]
    fn has_stop_sequence() {
        let sd = SongData {
            feature: FEATURE_STOPSEQUENCE,
            ..Default::default()
        };
        assert!(sd.has_stop_sequence());
    }

    #[test]
    fn has_document() {
        let sd = SongData {
            content: CONTENT_TEXT,
            ..Default::default()
        };
        assert!(sd.has_document());
    }

    #[test]
    fn has_bga() {
        let sd = SongData {
            content: CONTENT_BGA,
            ..Default::default()
        };
        assert!(sd.has_bga());
    }

    // -- play_mode --

    #[test]
    fn play_mode_beat_7k() {
        let sd = SongData {
            mode: 7,
            ..Default::default()
        };
        assert_eq!(sd.play_mode(), Some(PlayMode::Beat7K));
    }

    #[test]
    fn play_mode_beat_5k() {
        let sd = SongData {
            mode: 5,
            ..Default::default()
        };
        assert_eq!(sd.play_mode(), Some(PlayMode::Beat5K));
    }

    #[test]
    fn play_mode_invalid_returns_none() {
        let sd = SongData {
            mode: 999,
            ..Default::default()
        };
        assert_eq!(sd.play_mode(), None);
    }

    // -- from_model: metadata propagation --

    #[test]
    fn from_model_propagates_metadata() {
        let mut model = BmsModel::default();
        model.title = "Song Title".into();
        model.subtitle = "Sub".into();
        model.artist = "Artist".into();
        model.sub_artist = "SubArtist".into();
        model.genre = "Genre".into();
        model.md5 = "md5hash".into();
        model.sha256 = "sha256hash".into();
        model.play_level = 12;
        model.difficulty = 3;

        let sd = SongData::from_model(&model, false);
        assert_eq!(sd.title, "Song Title");
        assert_eq!(sd.subtitle, "Sub");
        assert_eq!(sd.artist, "Artist");
        assert_eq!(sd.subartist, "SubArtist");
        assert_eq!(sd.genre, "Genre");
        assert_eq!(sd.md5, "md5hash");
        assert_eq!(sd.sha256, "sha256hash");
        assert_eq!(sd.level, 12);
        assert_eq!(sd.difficulty, 3);
        // Default mode is Beat7K = 7
        assert_eq!(sd.mode, 7);
    }

    // -- ipfs fields --

    #[test]
    fn serde_ipfs_fields_round_trip() {
        let sd = SongData {
            title: "t".into(),
            md5: "abc".into(),
            ipfs: "QmTestCid123".into(),
            appendipfs: "QmAppendCid456".into(),
            ..Default::default()
        };
        let json = serde_json::to_string(&sd).unwrap();
        let deserialized: SongData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ipfs, "QmTestCid123");
        assert_eq!(deserialized.appendipfs, "QmAppendCid456");
    }

    #[test]
    fn serde_ipfs_fields_default_when_missing() {
        let json = r#"{"md5":"abc","sha256":"","title":"t","subtitle":"","genre":"","artist":"","subartist":"","tag":"","path":"","folder":"","stagefile":"","banner":"","backbmp":"","preview":"","parent":"","level":0,"difficulty":0,"maxbpm":0,"minbpm":0,"length":0,"mode":0,"judge":0,"feature":0,"content":0,"date":0,"favorite":0,"adddate":0,"notes":0,"charthash":""}"#;
        let sd: SongData = serde_json::from_str(json).unwrap();
        assert_eq!(sd.ipfs, "");
        assert_eq!(sd.appendipfs, "");
    }
}
