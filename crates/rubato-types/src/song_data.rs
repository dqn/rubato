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

/// Song data
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SongData {
    pub title: String,
    pub subtitle: String,
    #[serde(skip)]
    fulltitle: Option<String>,
    pub genre: String,
    pub artist: String,
    pub subartist: String,
    #[serde(skip)]
    fullartist: Option<String>,
    pub favorite: i32,
    #[serde(skip)]
    path: Vec<String>,
    /// Single path for serialization (first element of path vec)
    #[serde(rename = "path")]
    path_str: String,
    pub tag: String,
    pub md5: String,
    pub sha256: String,
    pub url: Option<String>,
    pub appendurl: Option<String>,
    pub ipfs: Option<String>,
    pub appendipfs: Option<String>,
    pub date: i32,
    pub adddate: i32,
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
    pub stagefile: String,
    pub backbmp: String,
    pub banner: String,
    pub preview: String,
    pub folder: String,
    pub parent: String,
    /// BMSModel is not Clone/Debug, so skip in derive
    #[serde(skip)]
    pub model: Option<BMSModel>,
    #[serde(skip)]
    pub info: Option<SongInformation>,
    pub charthash: Option<String>,
    pub org_md5: Option<Vec<String>>,
}

impl Clone for SongData {
    fn clone(&self) -> Self {
        SongData {
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
            fulltitle: self.fulltitle.clone(),
            genre: self.genre.clone(),
            artist: self.artist.clone(),
            subartist: self.subartist.clone(),
            fullartist: self.fullartist.clone(),
            favorite: self.favorite,
            path: self.path.clone(),
            path_str: self.path_str.clone(),
            tag: self.tag.clone(),
            md5: self.md5.clone(),
            sha256: self.sha256.clone(),
            url: self.url.clone(),
            appendurl: self.appendurl.clone(),
            ipfs: self.ipfs.clone(),
            appendipfs: self.appendipfs.clone(),
            date: self.date,
            adddate: self.adddate,
            level: self.level,
            mode: self.mode,
            feature: self.feature,
            difficulty: self.difficulty,
            judge: self.judge,
            minbpm: self.minbpm,
            maxbpm: self.maxbpm,
            length: self.length,
            content: self.content,
            notes: self.notes,
            stagefile: self.stagefile.clone(),
            backbmp: self.backbmp.clone(),
            banner: self.banner.clone(),
            preview: self.preview.clone(),
            folder: self.folder.clone(),
            parent: self.parent.clone(),
            model: None, // BMSModel is not Clone
            info: self.info.clone(),
            charthash: self.charthash.clone(),
            org_md5: self.org_md5.clone(),
        }
    }
}

impl std::fmt::Debug for SongData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SongData")
            .field("title", &self.title)
            .field("md5", &self.md5)
            .field("sha256", &self.sha256)
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
        sd.content = if contains_txt { CONTENT_TEXT } else { 0 };
        sd.set_bms_model(model);
        sd
    }

    pub fn set_bms_model(&mut self, model: BMSModel) {
        // BMSPlayerRule::validate(&model) - stubbed, no-op
        self.set_title(model.get_title().to_string());
        self.set_subtitle(model.get_sub_title().to_string());
        self.genre = model.get_genre().to_string();
        self.set_artist(model.get_artist().to_string());
        self.set_subartist(model.get_sub_artist().to_string());
        if let Some(p) = model.get_path() {
            self.path.push(p);
        }
        self.md5 = model.get_md5().to_string();
        self.sha256 = model.get_sha256().to_string();
        self.banner = model.get_banner().to_string();

        self.stagefile = model.get_stagefile().to_string();
        self.backbmp = model.get_backbmp().to_string();
        if self.preview.is_empty() {
            self.preview = model.get_preview().to_string();
        }

        if let Ok(l) = model.get_playlevel().parse::<i32>() {
            self.level = l;
        }

        self.mode = model.get_mode().map(|m| m.id()).unwrap_or(0);
        if self.difficulty == 0 {
            self.difficulty = model.get_difficulty();
        }
        self.judge = model.get_judgerank();
        self.minbpm = model.get_min_bpm() as i32;
        self.maxbpm = model.get_max_bpm() as i32;
        self.feature = 0;

        let keys = model.get_mode().map(|m| m.key()).unwrap_or(0);
        for tl in model.get_all_time_lines() {
            if tl.get_stop() > 0 {
                self.feature |= FEATURE_STOPSEQUENCE;
            }
            if tl.get_scroll() != 1.0 {
                self.feature |= FEATURE_SCROLL;
            }

            for i in 0..keys {
                if let Some(n) = tl.get_note(i) {
                    if n.is_long() {
                        match n.get_long_note_type() {
                            note::TYPE_UNDEFINED => self.feature |= FEATURE_UNDEFINEDLN,
                            note::TYPE_LONGNOTE => self.feature |= FEATURE_LONGNOTE,
                            note::TYPE_CHARGENOTE => self.feature |= FEATURE_CHARGENOTE,
                            note::TYPE_HELLCHARGENOTE => self.feature |= FEATURE_HELLCHARGENOTE,
                            _ => {}
                        }
                    }
                    if n.is_mine() {
                        self.feature |= FEATURE_MINENOTE;
                    }
                }
            }
        }

        self.length = model.get_last_time();
        self.notes = model.get_total_notes();

        if let Some(random) = model.get_random()
            && !random.is_empty()
        {
            self.feature |= FEATURE_RANDOM;
        }
        if !model.get_bga_list().is_empty() {
            self.content |= CONTENT_BGA;
        }
        if self.length >= 30000
            && (model.get_wav_list().len() as i32) <= (self.length / (50 * 1000)) + 3
        {
            self.content |= CONTENT_NOKEYSOUND;
        }

        self.info = Some(SongInformation::from_model(&model));

        let chart_string = model.to_chart_string();
        let mut hasher = Sha256::new();
        hasher.update(chart_string.as_bytes());
        let result = hasher.finalize();
        self.charthash = Some(convert_hex_string(&result));

        self.model = Some(model);
    }

    pub fn get_bms_model(&self) -> Option<&BMSModel> {
        self.model.as_ref()
    }

    pub fn get_path(&self) -> Option<&str> {
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

    pub fn get_all_paths(&self) -> &[String] {
        &self.path
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.fulltitle = None;
    }

    pub fn set_subtitle(&mut self, subtitle: String) {
        self.subtitle = subtitle;
        self.fulltitle = None;
    }

    pub fn get_full_title(&mut self) -> &str {
        if self.fulltitle.is_none() {
            self.fulltitle = Some(if !self.subtitle.is_empty() {
                format!("{} {}", self.title, self.subtitle)
            } else {
                self.title.clone()
            });
        }
        self.fulltitle.as_ref().unwrap()
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

    pub fn get_full_artist(&mut self) -> &str {
        if self.fullartist.is_none() {
            self.fullartist = Some(if !self.subartist.is_empty() {
                format!("{} {}", self.artist, self.subartist)
            } else {
                self.artist.clone()
            });
        }
        self.fullartist.as_ref().unwrap()
    }

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

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_subtitle(&self) -> &str {
        &self.subtitle
    }

    pub fn get_genre(&self) -> &str {
        &self.genre
    }

    pub fn get_artist(&self) -> &str {
        &self.artist
    }

    pub fn get_subartist(&self) -> &str {
        &self.subartist
    }

    pub fn get_md5(&self) -> &str {
        &self.md5
    }

    pub fn get_sha256(&self) -> &str {
        &self.sha256
    }

    pub fn get_url(&self) -> &str {
        self.url.as_deref().unwrap_or("")
    }

    pub fn get_appendurl(&self) -> &str {
        self.appendurl.as_deref().unwrap_or("")
    }

    pub fn get_level(&self) -> i32 {
        self.level
    }

    pub fn get_judge(&self) -> i32 {
        self.judge
    }

    pub fn get_minbpm(&self) -> i32 {
        self.minbpm
    }

    pub fn get_maxbpm(&self) -> i32 {
        self.maxbpm
    }

    pub fn get_notes(&self) -> i32 {
        self.notes
    }

    pub fn get_mode(&self) -> i32 {
        self.mode
    }

    pub fn get_difficulty(&self) -> i32 {
        self.difficulty
    }

    pub fn get_favorite(&self) -> i32 {
        self.favorite
    }

    pub fn set_favorite(&mut self, favorite: i32) {
        self.favorite = favorite;
    }

    pub fn get_feature(&self) -> i32 {
        self.feature
    }

    pub fn get_content(&self) -> i32 {
        self.content
    }

    pub fn get_length(&self) -> i32 {
        self.length
    }

    pub fn get_date(&self) -> i32 {
        self.date
    }

    pub fn get_adddate(&self) -> i32 {
        self.adddate
    }

    pub fn get_tag(&self) -> &str {
        &self.tag
    }

    pub fn get_folder(&self) -> &str {
        &self.folder
    }

    pub fn get_parent(&self) -> &str {
        &self.parent
    }

    pub fn get_stagefile(&self) -> &str {
        &self.stagefile
    }

    pub fn get_backbmp(&self) -> &str {
        &self.backbmp
    }

    pub fn get_banner(&self) -> &str {
        &self.banner
    }

    pub fn get_preview(&self) -> &str {
        &self.preview
    }

    pub fn get_charthash(&self) -> Option<&str> {
        self.charthash.as_deref()
    }

    pub fn get_song_information(&self) -> Option<&SongInformation> {
        self.info.as_ref()
    }

    pub fn set_genre(&mut self, genre: String) {
        self.genre = genre;
    }

    pub fn set_md5(&mut self, md5: String) {
        self.md5 = md5;
    }

    pub fn set_sha256(&mut self, sha256: String) {
        self.sha256 = sha256;
    }

    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    pub fn set_appendurl(&mut self, appendurl: String) {
        self.appendurl = Some(appendurl);
    }

    pub fn set_mode(&mut self, mode: i32) {
        self.mode = mode;
    }

    pub fn get_ipfs_str(&self) -> &str {
        self.ipfs.as_deref().unwrap_or("")
    }

    pub fn get_append_ipfs_str(&self) -> &str {
        self.appendipfs.as_deref().unwrap_or("")
    }

    pub fn get_org_md5_vec(&self) -> &[String] {
        self.org_md5.as_deref().unwrap_or(&[])
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
        self.fulltitle = None;
        self.fullartist = None;
        self.path.clear();
        self.date = 0;
        self.adddate = 0;
        self.level = 0;
        self.mode = 0;
        self.feature = 0;
        self.difficulty = 0;
        self.judge = 0;
        self.minbpm = 0;
        self.maxbpm = 0;
        self.notes = 0;
        self.length = 0;
        self.folder = String::new();
        self.parent = String::new();
        self.preview = String::new();
    }
}

impl Validatable for SongData {
    fn validate(&mut self) -> bool {
        if self.title.is_empty() {
            return false;
        }
        if self.md5.is_empty() && self.sha256.is_empty() {
            return false;
        }
        true
    }
}

impl crate::ipfs_information::IpfsInformation for SongData {
    fn get_ipfs(&self) -> String {
        self.ipfs.clone().unwrap_or_default()
    }

    fn get_append_ipfs(&self) -> String {
        self.appendipfs.clone().unwrap_or_default()
    }

    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn get_artist(&self) -> String {
        self.artist.clone()
    }

    fn get_org_md5(&self) -> Vec<String> {
        self.org_md5.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_construction() {
        let sd = SongData::new();
        assert_eq!(sd.title, "");
        assert_eq!(sd.subtitle, "");
        assert_eq!(sd.genre, "");
        assert_eq!(sd.artist, "");
        assert_eq!(sd.subartist, "");
        assert_eq!(sd.md5, "");
        assert_eq!(sd.sha256, "");
        assert_eq!(sd.favorite, 0);
        assert_eq!(sd.level, 0);
        assert_eq!(sd.mode, 0);
        assert_eq!(sd.difficulty, 0);
        assert_eq!(sd.judge, 0);
        assert_eq!(sd.minbpm, 0);
        assert_eq!(sd.maxbpm, 0);
        assert_eq!(sd.length, 0);
        assert_eq!(sd.notes, 0);
        assert_eq!(sd.content, 0);
        assert_eq!(sd.feature, 0);
        assert_eq!(sd.date, 0);
        assert_eq!(sd.adddate, 0);
        assert!(sd.url.is_none());
        assert!(sd.ipfs.is_none());
        assert!(sd.model.is_none());
        assert!(sd.info.is_none());
        assert!(sd.charthash.is_none());
        assert!(sd.org_md5.is_none());
    }

    #[test]
    fn test_serde_round_trip() {
        let mut sd = SongData::new();
        sd.title = "Test Song".to_string();
        sd.subtitle = "~Extra~".to_string();
        sd.genre = "Techno".to_string();
        sd.artist = "DJ Test".to_string();
        sd.subartist = "feat. Guest".to_string();
        sd.md5 = "abc123".to_string();
        sd.sha256 = "def456".to_string();
        sd.level = 12;
        sd.mode = 7;
        sd.difficulty = 3;
        sd.judge = 100;
        sd.minbpm = 140;
        sd.maxbpm = 180;
        sd.length = 120000;
        sd.notes = 1500;
        sd.favorite = FAVORITE_SONG;
        sd.url = Some("https://example.com".to_string());

        let json = serde_json::to_string(&sd).unwrap();
        let deserialized: SongData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.title, "Test Song");
        assert_eq!(deserialized.subtitle, "~Extra~");
        assert_eq!(deserialized.genre, "Techno");
        assert_eq!(deserialized.artist, "DJ Test");
        assert_eq!(deserialized.subartist, "feat. Guest");
        assert_eq!(deserialized.md5, "abc123");
        assert_eq!(deserialized.sha256, "def456");
        assert_eq!(deserialized.level, 12);
        assert_eq!(deserialized.mode, 7);
        assert_eq!(deserialized.difficulty, 3);
        assert_eq!(deserialized.judge, 100);
        assert_eq!(deserialized.minbpm, 140);
        assert_eq!(deserialized.maxbpm, 180);
        assert_eq!(deserialized.length, 120000);
        assert_eq!(deserialized.notes, 1500);
        assert_eq!(deserialized.favorite, FAVORITE_SONG);
        assert_eq!(deserialized.url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn test_field_accessors() {
        let mut sd = SongData::new();
        sd.set_title("My Title".to_string());
        sd.set_subtitle("Sub".to_string());
        sd.set_genre("Pop".to_string());
        sd.set_artist("Artist A".to_string());
        sd.set_subartist("Sub B".to_string());
        sd.set_md5("md5hash".to_string());
        sd.set_sha256("sha256hash".to_string());
        sd.set_url("https://url.com".to_string());
        sd.set_appendurl("https://append.com".to_string());
        sd.set_mode(14);
        sd.set_favorite(FAVORITE_CHART);

        assert_eq!(sd.get_title(), "My Title");
        assert_eq!(sd.get_subtitle(), "Sub");
        assert_eq!(sd.get_genre(), "Pop");
        assert_eq!(sd.get_artist(), "Artist A");
        assert_eq!(sd.get_subartist(), "Sub B");
        assert_eq!(sd.get_md5(), "md5hash");
        assert_eq!(sd.get_sha256(), "sha256hash");
        assert_eq!(sd.get_url(), "https://url.com");
        assert_eq!(sd.get_appendurl(), "https://append.com");
        assert_eq!(sd.get_mode(), 14);
        assert_eq!(sd.get_favorite(), FAVORITE_CHART);
    }

    #[test]
    fn test_full_title_with_subtitle() {
        let mut sd = SongData::new();
        sd.set_title("Main".to_string());
        sd.set_subtitle("Extra".to_string());

        assert_eq!(sd.full_title(), "Main Extra");
        // Also test the mutable caching version
        assert_eq!(sd.get_full_title(), "Main Extra");
    }

    #[test]
    fn test_full_title_without_subtitle() {
        let mut sd = SongData::new();
        sd.set_title("Main".to_string());

        assert_eq!(sd.full_title(), "Main");
        assert_eq!(sd.get_full_title(), "Main");
    }

    #[test]
    fn test_full_title_cache_invalidation() {
        let mut sd = SongData::new();
        sd.set_title("A".to_string());
        sd.set_subtitle("B".to_string());
        assert_eq!(sd.get_full_title(), "A B");

        // Changing title should invalidate cache
        sd.set_title("C".to_string());
        assert_eq!(sd.get_full_title(), "C B");

        // Changing subtitle should invalidate cache
        sd.set_subtitle("D".to_string());
        assert_eq!(sd.get_full_title(), "C D");
    }

    #[test]
    fn test_full_artist() {
        let mut sd = SongData::new();
        sd.set_artist("Artist".to_string());
        sd.set_subartist("Sub".to_string());
        assert_eq!(sd.get_full_artist(), "Artist Sub");

        sd.set_subartist("".to_string());
        assert_eq!(sd.get_full_artist(), "Artist");
    }

    #[test]
    fn test_path_operations() {
        let mut sd = SongData::new();
        assert!(sd.get_path().is_none());

        sd.set_path("/songs/test.bms".to_string());
        assert_eq!(sd.get_path(), Some("/songs/test.bms"));

        sd.add_another_path("/songs/test2.bms".to_string());
        assert_eq!(sd.get_all_paths().len(), 2);
        assert_eq!(sd.get_all_paths()[1], "/songs/test2.bms");

        sd.clear_path();
        assert!(sd.get_path().is_none());
        assert!(sd.get_all_paths().is_empty());
    }

    #[test]
    fn test_set_path_opt() {
        let mut sd = SongData::new();
        sd.set_path_opt(Some("/songs/a.bms".to_string()));
        assert_eq!(sd.get_path(), Some("/songs/a.bms"));

        sd.set_path_opt(None);
        assert!(sd.get_path().is_none());
    }

    #[test]
    fn test_feature_flags() {
        let mut sd = SongData::new();
        sd.feature = FEATURE_LONGNOTE | FEATURE_MINENOTE | FEATURE_STOPSEQUENCE;

        assert!(sd.has_long_note());
        assert!(sd.has_mine_note());
        assert!(sd.is_bpmstop());
        assert!(sd.has_any_long_note());

        assert!(!sd.has_undefined_long_note());
        assert!(!sd.has_charge_note());
        assert!(!sd.has_hell_charge_note());
        assert!(!sd.has_random_sequence());
        assert!(!sd.has_scroll_change());
    }

    #[test]
    fn test_content_flags() {
        let mut sd = SongData::new();
        sd.content = CONTENT_TEXT | CONTENT_BGA;

        assert!(sd.has_document());
        assert!(sd.has_bga());
        assert!(!sd.has_preview());
    }

    #[test]
    fn test_validate() {
        let mut sd = SongData::new();
        // Empty title => invalid
        assert!(!sd.validate());

        sd.title = "Test".to_string();
        // No md5 and no sha256 => invalid
        assert!(!sd.validate());

        sd.md5 = "hash".to_string();
        assert!(sd.validate());

        // sha256 only also valid
        let mut sd2 = SongData::new();
        sd2.title = "Test".to_string();
        sd2.sha256 = "shahash".to_string();
        assert!(sd2.validate());
    }

    #[test]
    fn test_merge() {
        let mut sd1 = SongData::new();
        let mut sd2 = SongData::new();
        sd2.url = Some("https://merged.com".to_string());
        sd2.appendurl = Some("https://append-merged.com".to_string());

        sd1.merge(&sd2);
        assert_eq!(sd1.get_url(), "https://merged.com");
        assert_eq!(sd1.get_appendurl(), "https://append-merged.com");

        // If sd1 already has url, merge should not overwrite
        let mut sd3 = SongData::new();
        sd3.url = Some("https://other.com".to_string());
        sd1.merge(&sd3);
        assert_eq!(sd1.get_url(), "https://merged.com");
    }

    #[test]
    fn test_shrink() {
        let mut sd = SongData::new();
        sd.set_title("Title".to_string());
        sd.set_subtitle("Sub".to_string());
        sd.set_path("/path".to_string());
        sd.level = 10;
        sd.notes = 500;
        sd.preview = "preview.ogg".to_string();

        sd.shrink();

        assert!(sd.get_all_paths().is_empty());
        assert_eq!(sd.level, 0);
        assert_eq!(sd.notes, 0);
        assert!(sd.preview.is_empty());
        assert!(sd.folder.is_empty());
        // Title should still be there
        assert_eq!(sd.get_title(), "Title");
    }

    #[test]
    fn test_clone() {
        let mut sd = SongData::new();
        sd.set_title("Clone Test".to_string());
        sd.md5 = "md5clone".to_string();
        sd.level = 7;

        let cloned = sd.clone();
        assert_eq!(cloned.get_title(), "Clone Test");
        assert_eq!(cloned.get_md5(), "md5clone");
        assert_eq!(cloned.get_level(), 7);
    }

    #[test]
    fn test_ipfs_accessors() {
        let mut sd = SongData::new();
        assert_eq!(sd.get_ipfs_str(), "");
        assert_eq!(sd.get_append_ipfs_str(), "");

        sd.ipfs = Some("Qm123".to_string());
        sd.appendipfs = Some("Qm456".to_string());
        assert_eq!(sd.get_ipfs_str(), "Qm123");
        assert_eq!(sd.get_append_ipfs_str(), "Qm456");
    }

    #[test]
    fn test_org_md5_accessor() {
        let sd = SongData::new();
        assert!(sd.get_org_md5_vec().is_empty());

        let mut sd2 = SongData::new();
        sd2.org_md5 = Some(vec!["md5a".to_string(), "md5b".to_string()]);
        assert_eq!(sd2.get_org_md5_vec().len(), 2);
        assert_eq!(sd2.get_org_md5_vec()[0], "md5a");
    }

    #[test]
    fn test_favorite_constants() {
        assert_eq!(FAVORITE_SONG, 1);
        assert_eq!(FAVORITE_CHART, 2);
        assert_eq!(INVISIBLE_SONG, 4);
        assert_eq!(INVISIBLE_CHART, 8);
    }
}
