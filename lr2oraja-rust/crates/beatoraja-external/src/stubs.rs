// External dependency stubs for beatoraja-external crate
// These will be replaced with actual implementations when corresponding phases are translated.

use std::collections::HashMap;

// ============================================================
// MainController stub
// ============================================================

/// Stub for bms.player.beatoraja.MainController
pub struct MainController;

impl MainController {
    pub fn get_player_resource(&self) -> &PlayerResource {
        todo!("Phase 8+ dependency: MainController.getPlayerResource")
    }
}

// ============================================================
// PlayerResource stub
// ============================================================

/// Stub for bms.player.beatoraja.PlayerResource
pub struct PlayerResource {
    pub config: Config,
    pub songdata: SongData,
    pub replay_data: ReplayData,
    pub reverse_lookup_levels: Vec<String>,
    pub original_mode: Mode,
}

impl PlayerResource {
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_songdata(&self) -> &SongData {
        &self.songdata
    }

    pub fn get_replay_data(&self) -> &ReplayData {
        &self.replay_data
    }

    pub fn get_reverse_lookup_levels(&self) -> &[String] {
        &self.reverse_lookup_levels
    }

    pub fn get_original_mode(&self) -> &Mode {
        &self.original_mode
    }
}

// ============================================================
// Config stub (fields needed by screenshot/webhook)
// ============================================================

/// Stub for bms.player.beatoraja.Config
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub set_clipboard_when_screenshot: bool,
    pub webhook_option: i32,
    pub webhook_url: Vec<String>,
    pub webhook_name: String,
    pub webhook_avatar: String,
}

impl Config {
    pub fn is_set_clipboard_when_screenshot(&self) -> bool {
        self.set_clipboard_when_screenshot
    }

    pub fn get_webhook_option(&self) -> i32 {
        self.webhook_option
    }

    pub fn get_webhook_url(&self) -> &[String] {
        &self.webhook_url
    }

    pub fn get_webhook_name(&self) -> &str {
        &self.webhook_name
    }

    pub fn get_webhook_avatar(&self) -> &str {
        &self.webhook_avatar
    }
}

// ============================================================
// PlayerConfig stub (fields needed by Twitter exporter)
// ============================================================

/// Stub for bms.player.beatoraja.PlayerConfig
#[derive(Clone, Debug, Default)]
pub struct PlayerConfig {
    pub twitter_consumer_key: String,
    pub twitter_consumer_secret: String,
    pub twitter_access_token: String,
    pub twitter_access_token_secret: String,
}

impl PlayerConfig {
    pub fn get_twitter_consumer_key(&self) -> &str {
        &self.twitter_consumer_key
    }

    pub fn get_twitter_consumer_secret(&self) -> &str {
        &self.twitter_consumer_secret
    }

    pub fn get_twitter_access_token(&self) -> &str {
        &self.twitter_access_token
    }

    pub fn get_twitter_access_token_secret(&self) -> &str {
        &self.twitter_access_token_secret
    }
}

// ============================================================
// SongData stub
// ============================================================

/// Stub for bms.player.beatoraja.song.SongData
#[derive(Clone, Debug, Default)]
pub struct SongData {
    pub sha256: String,
    pub md5: String,
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub subartist: String,
    pub genre: String,
    pub url: Option<String>,
    pub notes: i32,
    pub mode: i32,
}

impl SongData {
    pub fn get_sha256(&self) -> &str {
        &self.sha256
    }
    pub fn set_sha256(&mut self, s: String) {
        self.sha256 = s;
    }
    pub fn get_md5(&self) -> &str {
        &self.md5
    }
    pub fn set_md5(&mut self, s: String) {
        self.md5 = s;
    }
    pub fn get_title(&self) -> &str {
        &self.title
    }
    pub fn set_title(&mut self, s: String) {
        self.title = s;
    }
    pub fn get_artist(&self) -> &str {
        &self.artist
    }
    pub fn set_artist(&mut self, s: String) {
        self.artist = s;
    }
    pub fn get_genre(&self) -> &str {
        &self.genre
    }
    pub fn set_genre(&mut self, s: String) {
        self.genre = s;
    }
    pub fn get_url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    pub fn set_url(&mut self, s: String) {
        self.url = Some(s);
    }
    pub fn get_notes(&self) -> i32 {
        self.notes
    }
    pub fn set_notes(&mut self, n: i32) {
        self.notes = n;
    }
    pub fn get_mode(&self) -> i32 {
        self.mode
    }
    pub fn set_mode(&mut self, m: i32) {
        self.mode = m;
    }
    pub fn get_full_title(&self) -> String {
        if self.subtitle.is_empty() {
            self.title.clone()
        } else {
            format!("{} {}", self.title, self.subtitle)
        }
    }
}

// ============================================================
// SongDatabaseAccessor stub
// ============================================================

/// Stub for bms.player.beatoraja.song.SongDatabaseAccessor
pub struct SongDatabaseAccessor;

impl SongDatabaseAccessor {
    pub fn get_song_datas(&self, _hashes: &[&str]) -> Vec<SongData> {
        todo!("SongDatabaseAccessor.getSongDatas")
    }
}

// ============================================================
// ScoreData stub
// ============================================================

/// Stub for bms.player.beatoraja.ScoreData
#[derive(Clone, Debug, Default)]
pub struct ScoreData {
    pub sha256: String,
    pub mode: i32,
    pub clear: i32,
    pub playcount: i32,
    pub clearcount: i32,
    pub epg: i32,
    pub egr: i32,
    pub egd: i32,
    pub ebd: i32,
    pub epr: i32,
    pub minbp: i32,
    pub notes: i32,
    pub scorehash: String,
    // Full judge fields for judge_count
    pub lpg: i32,
    pub lgr: i32,
    pub lgd: i32,
    pub lbd: i32,
    pub lpr: i32,
}

impl ScoreData {
    pub fn get_sha256(&self) -> &str {
        &self.sha256
    }
    pub fn set_sha256(&mut self, s: String) {
        self.sha256 = s;
    }
    pub fn get_mode(&self) -> i32 {
        self.mode
    }
    pub fn set_mode(&mut self, m: i32) {
        self.mode = m;
    }
    pub fn get_clear(&self) -> i32 {
        self.clear
    }
    pub fn set_clear(&mut self, c: i32) {
        self.clear = c;
    }
    pub fn get_playcount(&self) -> i32 {
        self.playcount
    }
    pub fn set_playcount(&mut self, c: i32) {
        self.playcount = c;
    }
    pub fn get_clearcount(&self) -> i32 {
        self.clearcount
    }
    pub fn set_clearcount(&mut self, c: i32) {
        self.clearcount = c;
    }
    pub fn set_epg(&mut self, v: i32) {
        self.epg = v;
    }
    pub fn set_egr(&mut self, v: i32) {
        self.egr = v;
    }
    pub fn set_egd(&mut self, v: i32) {
        self.egd = v;
    }
    pub fn set_ebd(&mut self, v: i32) {
        self.ebd = v;
    }
    pub fn set_epr(&mut self, v: i32) {
        self.epr = v;
    }
    pub fn set_minbp(&mut self, v: i32) {
        self.minbp = v;
    }
    pub fn get_notes(&self) -> i32 {
        self.notes
    }
    pub fn set_notes(&mut self, n: i32) {
        self.notes = n;
    }
    pub fn get_scorehash(&self) -> &str {
        &self.scorehash
    }
    pub fn set_scorehash(&mut self, s: String) {
        self.scorehash = s;
    }
    pub fn get_exscore(&self) -> i32 {
        (self.epg + self.lpg) * 2 + self.egr + self.lgr
    }

    /// Get judge count for a specific judge type (combined fast+slow).
    /// judge: 0=PG, 1=GR, 2=GD, 3=BD, 4=PR, 5=MS
    pub fn get_judge_count(&self, judge: i32) -> i32 {
        match judge {
            0 => self.epg + self.lpg,
            1 => self.egr + self.lgr,
            2 => self.egd + self.lgd,
            3 => self.ebd + self.lbd,
            4 => self.epr + self.lpr,
            _ => 0,
        }
    }

    pub fn update(&mut self, newscore: &ScoreData) -> bool {
        let mut updated = false;
        if newscore.get_exscore() > self.get_exscore() {
            self.epg = newscore.epg;
            self.lpg = newscore.lpg;
            self.egr = newscore.egr;
            self.lgr = newscore.lgr;
            self.egd = newscore.egd;
            self.lgd = newscore.lgd;
            self.ebd = newscore.ebd;
            self.lbd = newscore.lbd;
            self.epr = newscore.epr;
            self.lpr = newscore.lpr;
            self.minbp = newscore.minbp;
            updated = true;
        }
        if newscore.clear > self.clear {
            self.clear = newscore.clear;
            updated = true;
        }
        self.playcount += newscore.playcount;
        self.clearcount += newscore.clearcount;
        updated
    }
}

// ============================================================
// ScoreDatabaseAccessor stub
// ============================================================

/// Stub for bms.player.beatoraja.ScoreDatabaseAccessor
pub struct ScoreDatabaseAccessor;

impl ScoreDatabaseAccessor {
    pub fn create_table(&self) {
        todo!("ScoreDatabaseAccessor.createTable")
    }

    pub fn get_score_data(&self, _sha256: &str, _mode: i32) -> Option<ScoreData> {
        todo!("ScoreDatabaseAccessor.getScoreData")
    }

    pub fn set_score_data(&self, _scores: &[ScoreData]) {
        todo!("ScoreDatabaseAccessor.setScoreData")
    }
}

// ============================================================
// MainState stub (for ScreenShotExporter)
// ============================================================

/// Stub for bms.player.beatoraja.MainState
pub struct MainState {
    pub main: MainController,
    pub resource: PlayerResource,
}

// ============================================================
// Screen type stubs (for instanceof checks)
// ============================================================

/// Enum to represent the current screen state type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenType {
    MusicSelector,
    MusicDecide,
    BMSPlayer,
    MusicResult,
    CourseResult,
    KeyConfiguration,
    Other,
}

// ============================================================
// AbstractResult stub
// ============================================================

/// Stub for bms.player.beatoraja.result.AbstractResult
pub struct AbstractResult {
    pub new_score: ScoreData,
    pub old_score: ScoreData,
    pub ir_rank: i32,
    pub ir_total_player: i32,
    pub old_ir_rank: i32,
}

impl AbstractResult {
    pub fn get_new_score(&self) -> &ScoreData {
        &self.new_score
    }

    pub fn get_old_score(&self) -> &ScoreData {
        &self.old_score
    }

    pub fn get_ir_rank(&self) -> i32 {
        self.ir_rank
    }

    pub fn get_ir_total_player(&self) -> i32 {
        self.ir_total_player
    }

    pub fn get_old_ir_rank(&self) -> i32 {
        self.old_ir_rank
    }
}

// ============================================================
// ReplayData stub
// ============================================================

/// Stub for bms.player.beatoraja.ReplayData
#[derive(Clone, Debug, Default)]
pub struct ReplayData {
    pub randomoption: i32,
    pub lane_shuffle_pattern: Vec<Vec<i32>>,
}

// ============================================================
// Mode stub
// ============================================================

/// Stub for bms.model.Mode
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mode {
    pub id: i32,
}

impl Default for Mode {
    fn default() -> Self {
        Self { id: 7 }
    }
}

impl Mode {
    pub const BEAT_7K: Mode = Mode { id: 7 };
}

// ============================================================
// TableData and related stubs
// ============================================================

/// Stub for bms.player.beatoraja.TableData
#[derive(Clone, Debug, Default)]
pub struct TableData {
    pub name: String,
    pub folder: Vec<TableFolder>,
}

impl TableData {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, s: String) {
        self.name = s;
    }
    pub fn get_folder(&self) -> &[TableFolder] {
        &self.folder
    }
    pub fn set_folder(&mut self, f: Vec<TableFolder>) {
        self.folder = f;
    }
}

/// Stub for bms.player.beatoraja.TableData.TableFolder
#[derive(Clone, Debug, Default)]
pub struct TableFolder {
    pub name: String,
    pub song: Vec<SongData>,
}

impl TableFolder {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, s: String) {
        self.name = s;
    }
    pub fn get_song(&self) -> &[SongData] {
        &self.song
    }
    pub fn set_song(&mut self, s: Vec<SongData>) {
        self.song = s;
    }
}

// ============================================================
// TableDataAccessor stub
// ============================================================

/// Stub for bms.player.beatoraja.TableDataAccessor
pub struct TableDataAccessor {
    pub tabledir: String,
}

impl TableDataAccessor {
    pub fn new(tabledir: &str) -> Self {
        Self {
            tabledir: tabledir.to_string(),
        }
    }

    pub fn write(&self, _td: &TableData) {
        todo!("TableDataAccessor.write")
    }
}

/// Stub trait for TableDataAccessor.TableAccessor
pub trait TableAccessor {
    fn name(&self) -> &str;
    fn read(&self) -> Option<TableData>;
    fn write(&self, td: &TableData);
}

// ============================================================
// LibGDX stubs
// ============================================================

/// Stub for com.badlogic.gdx.graphics.Pixmap
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
}

impl Pixmap {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    pub fn get_pixels(&self) -> Vec<u8> {
        todo!("Pixmap.getPixels - LibGDX dependency")
    }

    pub fn dispose(&mut self) {
        // stub
    }
}

/// Stub for com.badlogic.gdx.Gdx.graphics
pub struct GdxGraphics;

impl GdxGraphics {
    pub fn get_back_buffer_width() -> i32 {
        todo!("Gdx.graphics.getBackBufferWidth - LibGDX dependency")
    }

    pub fn get_back_buffer_height() -> i32 {
        todo!("Gdx.graphics.getBackBufferHeight - LibGDX dependency")
    }
}

/// Stub for com.badlogic.gdx.utils.BufferUtils
pub struct BufferUtils;

impl BufferUtils {
    pub fn copy(_src: &[u8], _src_offset: usize, _dst: &mut Vec<u8>, _count: usize) {
        todo!("BufferUtils.copy - LibGDX dependency")
    }
}

/// Stub for com.badlogic.gdx.graphics.PixmapIO
pub struct PixmapIO;

impl PixmapIO {
    pub fn write_png(_path: &str, _pixmap: &Pixmap) {
        todo!("PixmapIO.writePNG - LibGDX dependency")
    }
}

// ============================================================
// ImGuiNotify stub
// ============================================================

/// Stub for bms.player.beatoraja.modmenu.ImGuiNotify
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn info(msg: &str, _duration_ms: i32) {
        log::info!("{}", msg);
    }

    pub fn warning(msg: &str) {
        log::warn!("{}", msg);
    }
}

// ============================================================
// SkinConfiguration / KeyConfiguration stubs
// ============================================================

/// Stub for bms.player.beatoraja.config.SkinConfiguration
pub struct SkinConfiguration;

/// Stub for bms.player.beatoraja.config.KeyConfiguration
pub struct KeyConfiguration;

// ============================================================
// IntegerProperty / BooleanProperty / StringProperty stubs
// ============================================================

/// Stub for bms.player.beatoraja.skin.property.IntegerProperty
pub trait IntegerProperty {
    fn get(&self, state: &MainState) -> i32;
}

/// Stub for bms.player.beatoraja.skin.property.BooleanProperty
pub trait BooleanProperty {
    fn get(&self, state: &MainState) -> bool;
}

/// Stub for bms.player.beatoraja.skin.property.StringProperty
pub trait StringProperty {
    fn get(&self, state: &MainState) -> String;
}

/// Stub for IntegerPropertyFactory
pub struct IntegerPropertyFactory;

impl IntegerPropertyFactory {
    pub fn get_integer_property(_id: i32) -> Box<dyn IntegerProperty> {
        todo!("IntegerPropertyFactory.getIntegerProperty - beatoraja-skin dependency")
    }
}

/// Stub for BooleanPropertyFactory
pub struct BooleanPropertyFactory;

impl BooleanPropertyFactory {
    pub fn get_boolean_property(_id: i32) -> Box<dyn BooleanProperty> {
        todo!("BooleanPropertyFactory.getBooleanProperty - beatoraja-skin dependency")
    }
}

/// Stub for StringPropertyFactory
pub struct StringPropertyFactory;

impl StringPropertyFactory {
    pub fn get_string_property(_id: i32) -> Box<dyn StringProperty> {
        todo!("StringPropertyFactory.getStringProperty - beatoraja-skin dependency")
    }
}

// ============================================================
// SkinProperty constants
// ============================================================

pub const NUMBER_CLEAR: i32 = 370;
pub const NUMBER_PLAYLEVEL: i32 = 96;
pub const NUMBER_MAXSCORE: i32 = 72;
pub const OPTION_RESULT_AAA_1P: i32 = 300;
pub const OPTION_RESULT_AA_1P: i32 = 301;
pub const OPTION_RESULT_A_1P: i32 = 302;
pub const OPTION_RESULT_B_1P: i32 = 303;
pub const OPTION_RESULT_C_1P: i32 = 304;
pub const OPTION_RESULT_D_1P: i32 = 305;
pub const OPTION_RESULT_E_1P: i32 = 306;
pub const OPTION_RESULT_F_1P: i32 = 307;
pub const STRING_FULLTITLE: i32 = 12;
pub const STRING_TABLE_NAME: i32 = 1001;
pub const STRING_TABLE_LEVEL: i32 = 1002;

// ============================================================
// Twitter4j stubs (entirely stubbed - no Rust equivalent)
// ============================================================

/// Stub for twitter4j.Twitter
pub struct Twitter;

impl Twitter {
    pub fn upload_media(&self, _name: &str, _input: &[u8]) -> anyhow::Result<UploadedMedia> {
        todo!("twitter4j.Twitter.uploadMedia - no Rust equivalent")
    }

    pub fn update_status(&self, _update: &StatusUpdate) -> anyhow::Result<Status> {
        todo!("twitter4j.Twitter.updateStatus - no Rust equivalent")
    }
}

/// Stub for twitter4j.TwitterFactory
pub struct TwitterFactory;

impl TwitterFactory {
    pub fn new(_config: TwitterConfiguration) -> Self {
        Self
    }

    pub fn get_instance(&self) -> Twitter {
        Twitter
    }
}

/// Stub for twitter4j.conf.ConfigurationBuilder
pub struct TwitterConfigurationBuilder {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

impl Default for TwitterConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitterConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            consumer_key: String::new(),
            consumer_secret: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
        }
    }

    pub fn set_o_auth_consumer_key(mut self, key: &str) -> Self {
        self.consumer_key = key.to_string();
        self
    }

    pub fn set_o_auth_consumer_secret(mut self, secret: &str) -> Self {
        self.consumer_secret = secret.to_string();
        self
    }

    pub fn set_o_auth_access_token(mut self, token: &str) -> Self {
        self.access_token = token.to_string();
        self
    }

    pub fn set_o_auth_access_token_secret(mut self, secret: &str) -> Self {
        self.access_token_secret = secret.to_string();
        self
    }

    pub fn build(self) -> TwitterConfiguration {
        TwitterConfiguration
    }
}

/// Stub for twitter4j.conf.Configuration
pub struct TwitterConfiguration;

/// Stub for twitter4j.UploadedMedia
pub struct UploadedMedia {
    pub media_id: i64,
}

impl std::fmt::Display for UploadedMedia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UploadedMedia(id={})", self.media_id)
    }
}

impl UploadedMedia {
    pub fn get_media_id(&self) -> i64 {
        self.media_id
    }
}

/// Stub for twitter4j.StatusUpdate
pub struct StatusUpdate {
    pub text: String,
    pub media_ids: Vec<i64>,
}

impl StatusUpdate {
    pub fn new(text: String) -> Self {
        Self {
            text,
            media_ids: Vec::new(),
        }
    }

    pub fn set_media_ids(&mut self, ids: Vec<i64>) {
        self.media_ids = ids;
    }
}

/// Stub for twitter4j.Status
pub struct Status;

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status")
    }
}

// ============================================================
// AWT Clipboard stubs
// ============================================================

/// Stub for java.awt.datatransfer.Clipboard + ImageTransferable
/// Clipboard image copy is platform-specific and has no direct Rust equivalent
pub struct ClipboardHelper;

impl ClipboardHelper {
    pub fn copy_image_to_clipboard(_path: &str) -> anyhow::Result<()> {
        todo!("AWT Clipboard image copy - no direct Rust equivalent")
    }
}

// ============================================================
// MainStateListener stub (re-export)
// ============================================================

/// Stub for bms.player.beatoraja.MainStateListener
pub trait MainStateListener {
    fn update(&mut self, state: &MainState, status: i32);
}
