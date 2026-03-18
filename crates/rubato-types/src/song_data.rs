use crate::song_information::SongInformation;
use crate::validatable::Validatable;
use bms_model::bms_decoder::convert_hex_string;
use bms_model::bms_model::BMSModel;
use bms_model::note;
use sha2::{Digest, Sha256};

pub const FEATURE_UNDEFINEDLN: i32 = 1;
pub const FEATURE_MINENOTE: i32 = 2;
pub const FEATURE_RANDOM: i32 = 4;
pub const FEATURE_LONGNOTE: i32 = 8;
pub const FEATURE_CHARGENOTE: i32 = 16;
pub const FEATURE_HELLCHARGENOTE: i32 = 32;
pub const FEATURE_STOPSEQUENCE: i32 = 64;
pub const FEATURE_SCROLL: i32 = 128;

pub const CONTENT_TEXT: i32 = 1;
pub const CONTENT_BGA: i32 = 2;
pub const CONTENT_PREVIEW: i32 = 4;
pub const CONTENT_NOKEYSOUND: i32 = 128;

pub const FAVORITE_SONG: i32 = 1;
pub const FAVORITE_CHART: i32 = 2;
pub const INVISIBLE_SONG: i32 = 4;
pub const INVISIBLE_CHART: i32 = 8;

/// Song metadata (title/artist/genre)
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SongMetadata {
    pub title: String,
    pub subtitle: String,
    #[serde(skip)]
    fulltitle: Option<String>,
    pub genre: String,
    pub artist: String,
    pub subartist: String,
    #[serde(skip)]
    fullartist: Option<String>,
    pub tag: String,
}

impl SongMetadata {
    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.fulltitle = None;
    }

    pub fn set_subtitle(&mut self, subtitle: String) {
        self.subtitle = subtitle;
        self.fulltitle = None;
    }

    pub fn full_title_cached(&mut self) -> &str {
        if self.fulltitle.is_none() {
            self.fulltitle = Some(if !self.subtitle.is_empty() {
                format!("{} {}", self.title, self.subtitle)
            } else {
                self.title.clone()
            });
        }
        self.fulltitle.as_ref().expect("fulltitle is Some")
    }

    /// Non-mutating version of get_full_title (computes without caching)
    pub fn full_title(&self) -> String {
        if !self.subtitle.is_empty() {
            format!("{} {}", self.title, self.subtitle)
        } else {
            self.title.clone()
        }
    }

    pub fn set_artist(&mut self, artist: String) {
        self.artist = artist;
        self.fullartist = None;
    }

    pub fn set_subartist(&mut self, subartist: String) {
        self.subartist = subartist;
        self.fullartist = None;
    }

    pub fn full_artist(&mut self) -> &str {
        if self.fullartist.is_none() {
            self.fullartist = Some(if !self.subartist.is_empty() {
                format!("{} {}", self.artist, self.subartist)
            } else {
                self.artist.clone()
            });
        }
        self.fullartist.as_ref().expect("fullartist is Some")
    }
}

/// Chart/timing data (level, mode, BPM, features, etc.)
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ChartInfo {
    pub level: i32,
    pub mode: i32,
    pub feature: i32,
    pub difficulty: i32,
    pub judge: i32,
    pub minbpm: i32,
    pub maxbpm: i32,
    pub length: i32,
    pub content: i32,
    pub notes: i32,
    pub date: i32,
    pub adddate: i32,
}

impl ChartInfo {
    pub fn has_document(&self) -> bool {
        (self.content & CONTENT_TEXT) != 0
    }

    pub fn has_bga(&self) -> bool {
        (self.content & CONTENT_BGA) != 0
    }

    pub fn has_preview(&self) -> bool {
        (self.content & CONTENT_PREVIEW) != 0
    }

    pub fn has_random_sequence(&self) -> bool {
        (self.feature & FEATURE_RANDOM) != 0
    }

    pub fn has_mine_note(&self) -> bool {
        (self.feature & FEATURE_MINENOTE) != 0
    }

    pub fn has_undefined_long_note(&self) -> bool {
        (self.feature & FEATURE_UNDEFINEDLN) != 0
    }

    pub fn has_long_note(&self) -> bool {
        (self.feature & FEATURE_LONGNOTE) != 0
    }

    pub fn has_charge_note(&self) -> bool {
        (self.feature & FEATURE_CHARGENOTE) != 0
    }

    pub fn has_hell_charge_note(&self) -> bool {
        (self.feature & FEATURE_HELLCHARGENOTE) != 0
    }

    pub fn has_any_long_note(&self) -> bool {
        (self.feature
            & (FEATURE_UNDEFINEDLN
                | FEATURE_LONGNOTE
                | FEATURE_CHARGENOTE
                | FEATURE_HELLCHARGENOTE))
            != 0
    }

    pub fn is_bpmstop(&self) -> bool {
        (self.feature & FEATURE_STOPSEQUENCE) != 0
    }

    pub fn has_scroll_change(&self) -> bool {
        (self.feature & FEATURE_SCROLL) != 0
    }
}

/// File paths and hashes
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FileInfo {
    #[serde(skip)]
    path: Vec<String>,
    /// Single path for serialization (first element of path vec)
    #[serde(rename = "path")]
    path_str: String,
    pub md5: String,
    pub sha256: String,
    pub charthash: Option<String>,
    pub org_md5: Option<Vec<String>>,
    pub stagefile: String,
    pub backbmp: String,
    pub banner: String,
    pub preview: String,
}

impl FileInfo {
    pub fn path(&self) -> Option<&str> {
        if !self.path.is_empty() {
            Some(&self.path[0])
        } else if !self.path_str.is_empty() {
            Some(&self.path_str)
        } else {
            None
        }
    }

    pub fn set_path(&mut self, path: String) {
        if self.path.is_empty() {
            self.path.push(path.clone());
        } else {
            self.path[0] = path.clone();
        }
        self.path_str = path;
    }

    /// Clear the path (set to empty)
    pub fn clear_path(&mut self) {
        self.path.clear();
        self.path_str = String::new();
    }

    /// Set path from an Option (compatibility helper)
    pub fn set_path_opt(&mut self, path: Option<String>) {
        match path {
            Some(p) => self.set_path(p),
            None => self.clear_path(),
        }
    }

    pub fn add_another_path(&mut self, path: String) {
        self.path.push(path);
    }

    pub fn all_paths(&self) -> &[String] {
        &self.path
    }

    pub fn org_md5_vec(&self) -> &[String] {
        self.org_md5.as_deref().unwrap_or(&[])
    }
}

/// Song data
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SongData {
    #[serde(flatten)]
    pub metadata: SongMetadata,
    #[serde(flatten)]
    pub chart: ChartInfo,
    #[serde(flatten)]
    pub file: FileInfo,
    pub favorite: i32,
    pub url: Option<String>,
    pub appendurl: Option<String>,
    pub ipfs: Option<String>,
    pub appendipfs: Option<String>,
    pub folder: String,
    pub parent: String,
    /// BMSModel is not Clone/Debug, so skip in derive
    #[serde(skip)]
    pub model: Option<BMSModel>,
    #[serde(skip)]
    pub info: Option<SongInformation>,
}

impl Clone for SongData {
    fn clone(&self) -> Self {
        SongData {
            metadata: self.metadata.clone(),
            chart: self.chart.clone(),
            file: self.file.clone(),
            favorite: self.favorite,
            url: self.url.clone(),
            appendurl: self.appendurl.clone(),
            ipfs: self.ipfs.clone(),
            appendipfs: self.appendipfs.clone(),
            folder: self.folder.clone(),
            parent: self.parent.clone(),
            model: None, // BMSModel is not Clone
            info: self.info.clone(),
        }
    }
}

impl std::fmt::Debug for SongData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SongData")
            .field("title", &self.metadata.title)
            .field("md5", &self.file.md5)
            .field("sha256", &self.file.sha256)
            .field("model", &self.model.is_some())
            .finish()
    }
}

impl SongData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_model(model: BMSModel, contains_txt: bool) -> Self {
        let mut sd = SongData::new();
        sd.chart.content = if contains_txt { CONTENT_TEXT } else { 0 };
        sd.set_bms_model(model);
        sd
    }

    pub fn set_bms_model(&mut self, model: BMSModel) {
        // BMSPlayerRule::validate(&model) - stubbed, no-op
        self.metadata.set_title(model.title.clone());
        self.metadata.set_subtitle(model.sub_title.clone());
        self.metadata.genre = model.genre.clone();
        self.metadata.set_artist(model.artist.clone());
        self.metadata.set_subartist(model.subartist.clone());
        if let Some(p) = model.path() {
            self.file.set_path(p);
        }
        self.file.md5 = model.md5.clone();
        self.file.sha256 = model.sha256.clone();
        self.file.banner = model.banner.clone();

        self.file.stagefile = model.stagefile.clone();
        self.file.backbmp = model.backbmp.clone();
        if self.file.preview.is_empty() {
            self.file.preview = model.preview.clone();
        }

        if let Ok(l) = model.playlevel.parse::<i32>() {
            self.chart.level = l;
        }

        self.chart.mode = model.mode().map(|m| m.id()).unwrap_or(0);
        if self.chart.difficulty == 0 {
            self.chart.difficulty = model.difficulty;
        }
        self.chart.judge = model.judgerank;
        self.chart.minbpm = model.min_bpm() as i32;
        self.chart.maxbpm = model.max_bpm() as i32;
        self.chart.feature = 0;

        let keys = model.mode().map(|m| m.key()).unwrap_or(0);
        for tl in &model.timelines {
            if tl.stop() > 0 {
                self.chart.feature |= FEATURE_STOPSEQUENCE;
            }
            if tl.scroll != 1.0 {
                self.chart.feature |= FEATURE_SCROLL;
            }

            for i in 0..keys {
                if let Some(n) = tl.note(i) {
                    if n.is_long() {
                        match n.long_note_type() {
                            note::TYPE_UNDEFINED => self.chart.feature |= FEATURE_UNDEFINEDLN,
                            note::TYPE_LONGNOTE => self.chart.feature |= FEATURE_LONGNOTE,
                            note::TYPE_CHARGENOTE => self.chart.feature |= FEATURE_CHARGENOTE,
                            note::TYPE_HELLCHARGENOTE => {
                                self.chart.feature |= FEATURE_HELLCHARGENOTE
                            }
                            _ => {}
                        }
                    }
                    if n.is_mine() {
                        self.chart.feature |= FEATURE_MINENOTE;
                    }
                }
            }
        }

        self.chart.length = model.last_time().clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        self.chart.notes = model.total_notes();

        if let Some(random) = model.random()
            && !random.is_empty()
        {
            self.chart.feature |= FEATURE_RANDOM;
        }
        if !model.bgamap.is_empty() {
            self.chart.content |= CONTENT_BGA;
        }
        if self.chart.length >= 30000
            && (model.wavmap.len() as i32) <= (self.chart.length / (50 * 1000)) + 3
        {
            self.chart.content |= CONTENT_NOKEYSOUND;
        }

        self.info = Some(SongInformation::from_model(&model));

        let chart_string = model.to_chart_string();
        let mut hasher = Sha256::new();
        hasher.update(chart_string.as_bytes());
        let result = hasher.finalize();
        self.file.charthash = Some(convert_hex_string(&result));

        self.model = Some(model);
    }

    pub fn bms_model(&self) -> Option<&BMSModel> {
        self.model.as_ref()
    }

    pub fn url(&self) -> &str {
        self.url.as_deref().unwrap_or("")
    }

    pub fn appendurl(&self) -> &str {
        self.appendurl.as_deref().unwrap_or("")
    }

    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    pub fn ipfs_str(&self) -> &str {
        self.ipfs.as_deref().unwrap_or("")
    }

    pub fn append_ipfs_str(&self) -> &str {
        self.appendipfs.as_deref().unwrap_or("")
    }

    pub fn merge(&mut self, song: &SongData) {
        if self.url.as_ref().is_none_or(|u| u.is_empty()) {
            self.url = song.url.clone();
        }
        if self.appendurl.as_ref().is_none_or(|u| u.is_empty()) {
            self.appendurl = song.appendurl.clone();
        }
    }

    pub fn shrink(&mut self) {
        self.metadata.fulltitle = None;
        self.metadata.fullartist = None;
        self.file.clear_path();
        self.chart.date = 0;
        self.chart.adddate = 0;
        self.chart.level = 0;
        self.chart.mode = 0;
        self.chart.feature = 0;
        self.chart.difficulty = 0;
        self.chart.judge = 0;
        self.chart.minbpm = 0;
        self.chart.maxbpm = 0;
        self.chart.notes = 0;
        self.chart.length = 0;
        self.folder = String::new();
        self.parent = String::new();
        self.file.preview = String::new();
    }
}

impl Validatable for SongData {
    fn validate(&mut self) -> bool {
        if self.metadata.title.is_empty() {
            return false;
        }
        if self.file.md5.is_empty() && self.file.sha256.is_empty() {
            return false;
        }
        true
    }
}

impl crate::ipfs_information::IpfsInformation for SongData {
    fn ipfs(&self) -> String {
        self.ipfs.clone().unwrap_or_default()
    }

    fn append_ipfs(&self) -> String {
        self.appendipfs.clone().unwrap_or_default()
    }

    fn title(&self) -> String {
        self.metadata.title.clone()
    }

    fn artist(&self) -> String {
        self.metadata.artist.clone()
    }

    fn org_md5(&self) -> Vec<String> {
        self.file.org_md5.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_construction() {
        let sd = SongData::new();
        assert_eq!(sd.metadata.title, "");
        assert_eq!(sd.metadata.subtitle, "");
        assert_eq!(sd.metadata.genre, "");
        assert_eq!(sd.metadata.artist, "");
        assert_eq!(sd.metadata.subartist, "");
        assert_eq!(sd.file.md5, "");
        assert_eq!(sd.file.sha256, "");
        assert_eq!(sd.favorite, 0);
        assert_eq!(sd.chart.level, 0);
        assert_eq!(sd.chart.mode, 0);
        assert_eq!(sd.chart.difficulty, 0);
        assert_eq!(sd.chart.judge, 0);
        assert_eq!(sd.chart.minbpm, 0);
        assert_eq!(sd.chart.maxbpm, 0);
        assert_eq!(sd.chart.length, 0);
        assert_eq!(sd.chart.notes, 0);
        assert_eq!(sd.chart.content, 0);
        assert_eq!(sd.chart.feature, 0);
        assert_eq!(sd.chart.date, 0);
        assert_eq!(sd.chart.adddate, 0);
        assert!(sd.url.is_none());
        assert!(sd.ipfs.is_none());
        assert!(sd.model.is_none());
        assert!(sd.info.is_none());
        assert!(sd.file.charthash.is_none());
        assert!(sd.file.org_md5.is_none());
    }

    #[test]
    fn test_serde_round_trip() {
        let mut sd = SongData::new();
        sd.metadata.title = "Test Song".to_string();
        sd.metadata.subtitle = "~Extra~".to_string();
        sd.metadata.genre = "Techno".to_string();
        sd.metadata.artist = "DJ Test".to_string();
        sd.metadata.subartist = "feat. Guest".to_string();
        sd.file.md5 = "abc123".to_string();
        sd.file.sha256 = "def456".to_string();
        sd.chart.level = 12;
        sd.chart.mode = 7;
        sd.chart.difficulty = 3;
        sd.chart.judge = 100;
        sd.chart.minbpm = 140;
        sd.chart.maxbpm = 180;
        sd.chart.length = 120000;
        sd.chart.notes = 1500;
        sd.favorite = FAVORITE_SONG;
        sd.url = Some("https://example.com".to_string());

        let json = serde_json::to_string(&sd).unwrap();
        let deserialized: SongData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metadata.title, "Test Song");
        assert_eq!(deserialized.metadata.subtitle, "~Extra~");
        assert_eq!(deserialized.metadata.genre, "Techno");
        assert_eq!(deserialized.metadata.artist, "DJ Test");
        assert_eq!(deserialized.metadata.subartist, "feat. Guest");
        assert_eq!(deserialized.file.md5, "abc123");
        assert_eq!(deserialized.file.sha256, "def456");
        assert_eq!(deserialized.chart.level, 12);
        assert_eq!(deserialized.chart.mode, 7);
        assert_eq!(deserialized.chart.difficulty, 3);
        assert_eq!(deserialized.chart.judge, 100);
        assert_eq!(deserialized.chart.minbpm, 140);
        assert_eq!(deserialized.chart.maxbpm, 180);
        assert_eq!(deserialized.chart.length, 120000);
        assert_eq!(deserialized.chart.notes, 1500);
        assert_eq!(deserialized.favorite, FAVORITE_SONG);
        assert_eq!(deserialized.url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn test_field_accessors() {
        let mut sd = SongData::new();
        sd.metadata.title = "My Title".to_string();
        sd.metadata.set_subtitle("Sub".to_string());
        sd.metadata.genre = "Pop".to_string();
        sd.metadata.set_artist("Artist A".to_string());
        sd.metadata.set_subartist("Sub B".to_string());
        sd.file.md5 = "md5hash".to_string();
        sd.file.sha256 = "sha256hash".to_string();
        sd.set_url("https://url.com".to_string());
        sd.chart.mode = 14;
        sd.favorite = FAVORITE_CHART;

        assert_eq!(sd.metadata.title, "My Title");
        assert_eq!(sd.metadata.subtitle, "Sub");
        assert_eq!(sd.metadata.genre, "Pop");
        assert_eq!(sd.metadata.artist, "Artist A");
        assert_eq!(sd.metadata.subartist, "Sub B");
        assert_eq!(sd.file.md5, "md5hash");
        assert_eq!(sd.file.sha256, "sha256hash");
        assert_eq!(sd.url(), "https://url.com");
        assert_eq!(sd.chart.mode, 14);
        assert_eq!(sd.favorite, FAVORITE_CHART);
    }

    #[test]
    fn test_full_title_with_subtitle() {
        let mut sd = SongData::new();
        sd.metadata.title = "Main".to_string();
        sd.metadata.set_subtitle("Extra".to_string());

        assert_eq!(sd.metadata.full_title(), "Main Extra");
        // Also test the mutable caching version
        assert_eq!(sd.metadata.full_title(), "Main Extra");
    }

    #[test]
    fn test_full_title_without_subtitle() {
        let mut sd = SongData::new();
        sd.metadata.title = "Main".to_string();

        assert_eq!(sd.metadata.full_title(), "Main");
        assert_eq!(sd.metadata.full_title(), "Main");
    }

    #[test]
    fn test_full_title_cache_invalidation() {
        let mut sd = SongData::new();
        sd.metadata.set_title("A".to_string());
        sd.metadata.set_subtitle("B".to_string());
        assert_eq!(sd.metadata.full_title(), "A B");

        // Changing title should invalidate cache
        sd.metadata.set_title("C".to_string());
        assert_eq!(sd.metadata.full_title(), "C B");

        // Changing subtitle should invalidate cache
        sd.metadata.set_subtitle("D".to_string());
        assert_eq!(sd.metadata.full_title(), "C D");
    }

    #[test]
    fn test_full_artist() {
        let mut sd = SongData::new();
        sd.metadata.set_artist("Artist".to_string());
        sd.metadata.set_subartist("Sub".to_string());
        assert_eq!(sd.metadata.full_artist(), "Artist Sub");

        sd.metadata.set_subartist("".to_string());
        assert_eq!(sd.metadata.full_artist(), "Artist");
    }

    #[test]
    fn test_path_operations() {
        let mut sd = SongData::new();
        assert!(sd.file.path().is_none());

        sd.file.set_path("/songs/test.bms".to_string());
        assert_eq!(sd.file.path(), Some("/songs/test.bms"));

        sd.file.add_another_path("/songs/test2.bms".to_string());
        assert_eq!(sd.file.all_paths().len(), 2);
        assert_eq!(sd.file.all_paths()[1], "/songs/test2.bms");

        sd.file.clear_path();
        assert!(sd.file.path().is_none());
        assert!(sd.file.all_paths().is_empty());
    }

    #[test]
    fn test_set_path_opt() {
        let mut sd = SongData::new();
        sd.file.set_path_opt(Some("/songs/a.bms".to_string()));
        assert_eq!(sd.file.path(), Some("/songs/a.bms"));

        sd.file.set_path_opt(None);
        assert!(sd.file.path().is_none());
    }

    #[test]
    fn test_feature_flags() {
        let mut sd = SongData::new();
        sd.chart.feature = FEATURE_LONGNOTE | FEATURE_MINENOTE | FEATURE_STOPSEQUENCE;

        assert!(sd.chart.has_long_note());
        assert!(sd.chart.has_mine_note());
        assert!(sd.chart.is_bpmstop());
        assert!(sd.chart.has_any_long_note());

        assert!(!sd.chart.has_undefined_long_note());
        assert!(!sd.chart.has_charge_note());
        assert!(!sd.chart.has_hell_charge_note());
        assert!(!sd.chart.has_random_sequence());
        assert!(!sd.chart.has_scroll_change());
    }

    #[test]
    fn test_content_flags() {
        let mut sd = SongData::new();
        sd.chart.content = CONTENT_TEXT | CONTENT_BGA;

        assert!(sd.chart.has_document());
        assert!(sd.chart.has_bga());
        assert!(!sd.chart.has_preview());
    }

    #[test]
    fn test_validate() {
        let mut sd = SongData::new();
        // Empty title => invalid
        assert!(!sd.validate());

        sd.metadata.title = "Test".to_string();
        // No md5 and no sha256 => invalid
        assert!(!sd.validate());

        sd.file.md5 = "hash".to_string();
        assert!(sd.validate());

        // sha256 only also valid
        let mut sd2 = SongData::new();
        sd2.metadata.title = "Test".to_string();
        sd2.file.sha256 = "shahash".to_string();
        assert!(sd2.validate());
    }

    #[test]
    fn test_merge() {
        let mut sd1 = SongData::new();
        let mut sd2 = SongData::new();
        sd2.url = Some("https://merged.com".to_string());
        sd2.appendurl = Some("https://append-merged.com".to_string());

        sd1.merge(&sd2);
        assert_eq!(sd1.url(), "https://merged.com");
        assert_eq!(sd1.appendurl(), "https://append-merged.com");

        // If sd1 already has url, merge should not overwrite
        let mut sd3 = SongData::new();
        sd3.url = Some("https://other.com".to_string());
        sd1.merge(&sd3);
        assert_eq!(sd1.url(), "https://merged.com");
    }

    #[test]
    fn test_shrink() {
        let mut sd = SongData::new();
        sd.metadata.title = "Title".to_string();
        sd.metadata.set_subtitle("Sub".to_string());
        sd.file.set_path("/path".to_string());
        sd.chart.level = 10;
        sd.chart.notes = 500;
        sd.file.preview = "preview.ogg".to_string();

        sd.shrink();

        assert!(sd.file.all_paths().is_empty());
        assert_eq!(sd.chart.level, 0);
        assert_eq!(sd.chart.notes, 0);
        assert!(sd.file.preview.is_empty());
        assert!(sd.folder.is_empty());
        // Title should still be there
        assert_eq!(sd.metadata.title, "Title");
    }

    #[test]
    fn test_clone() {
        let mut sd = SongData::new();
        sd.metadata.title = "Clone Test".to_string();
        sd.file.md5 = "md5clone".to_string();
        sd.chart.level = 7;

        let cloned = sd.clone();
        assert_eq!(cloned.metadata.title, "Clone Test");
        assert_eq!(cloned.file.md5, "md5clone");
        assert_eq!(cloned.chart.level, 7);
    }

    #[test]
    fn test_ipfs_accessors() {
        let mut sd = SongData::new();
        assert_eq!(sd.ipfs_str(), "");
        assert_eq!(sd.append_ipfs_str(), "");

        sd.ipfs = Some("Qm123".to_string());
        sd.appendipfs = Some("Qm456".to_string());
        assert_eq!(sd.ipfs_str(), "Qm123");
        assert_eq!(sd.append_ipfs_str(), "Qm456");
    }

    #[test]
    fn test_org_md5_accessor() {
        let sd = SongData::new();
        assert!(sd.file.org_md5_vec().is_empty());

        let mut sd2 = SongData::new();
        sd2.file.org_md5 = Some(vec!["md5a".to_string(), "md5b".to_string()]);
        assert_eq!(sd2.file.org_md5_vec().len(), 2);
        assert_eq!(sd2.file.org_md5_vec()[0], "md5a");
    }

    #[test]
    fn test_favorite_constants() {
        assert_eq!(FAVORITE_SONG, 1);
        assert_eq!(FAVORITE_CHART, 2);
        assert_eq!(INVISIBLE_SONG, 4);
        assert_eq!(INVISIBLE_CHART, 8);
    }
}
