use std::path::{Path, PathBuf};

use crate::bms_player_mode::BMSPlayerMode;
use crate::bms_resource::BMSResource;
use crate::config::Config;
use crate::course_data::{CourseData, CourseDataConstraint};
use crate::player_config::PlayerConfig;
use crate::player_data::PlayerData;
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::stubs::*;

/// SongData stub (Phase 5+ dependency: beatoraja.song)
pub struct SongData {
    md5: String,
    sha256: String,
    path: String,
    title: String,
    subtitle: String,
}

impl Default for SongData {
    fn default() -> Self {
        Self::new()
    }
}

impl SongData {
    pub fn new() -> Self {
        Self {
            md5: String::new(),
            sha256: String::new(),
            path: String::new(),
            title: String::new(),
            subtitle: String::new(),
        }
    }

    pub fn get_md5(&self) -> &str {
        &self.md5
    }

    pub fn get_sha256(&self) -> &str {
        &self.sha256
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_subtitle(&self) -> &str {
        &self.subtitle
    }
}

/// RankingData stub (Phase 5+ dependency: beatoraja.ir)
pub struct RankingData;

/// FloatArray stub (LibGDX equivalent)
pub type FloatArray = Vec<f32>;

/// PlayerResource - holds game session state for data exchange between components
#[allow(dead_code)]
pub struct PlayerResource {
    /// Current BMS model (Phase 5+ stub)
    model: Option<()>,
    /// Margin time
    margin_time: i64,
    /// Current song data
    songdata: Option<SongData>,
    /// Original BMS mode
    orgmode: Option<()>,
    /// Player data
    playerdata: PlayerData,
    /// Config reference
    config: Config,
    /// Player config reference
    pconfig: PlayerConfig,
    /// Play mode
    mode: Option<BMSPlayerMode>,
    /// BMS resource
    bmsresource: Option<BMSResource>,
    /// Score
    score: Option<ScoreData>,
    /// Rival score
    rscore: Option<ScoreData>,
    /// Target score
    tscore: Option<ScoreData>,
    /// Ranking data
    ranking: Option<RankingData>,
    /// Whether to update score
    update_score: bool,
    /// Whether to update course score
    update_course_score: bool,
    /// Groove gauge (Phase 5+ stub)
    groove_gauge: Option<GrooveGauge>,
    /// Gauge transition log
    gauge: Option<Vec<FloatArray>>,
    /// Replay data
    replay: Option<ReplayData>,
    /// Chart option
    chart_option: Option<ReplayData>,
    /// BMS paths for autoplay
    bms_paths: Option<Vec<PathBuf>>,
    /// Loop autoplay
    loop_play: bool,
    /// Course data
    coursedata: Option<CourseData>,
    /// Course BMS models (Phase 5+ stub)
    course: Option<Vec<()>>,
    /// Course index
    courseindex: usize,
    /// Course gauge history
    coursegauge: Vec<Vec<FloatArray>>,
    /// Course replay data
    course_replay: Vec<ReplayData>,
    /// Course score
    cscore: Option<ScoreData>,
    /// Combo count (for course play carry-over)
    combo: i32,
    /// Max combo count (for course play carry-over)
    maxcombo: i32,
    /// Original gauge option
    org_gauge_option: i32,
    /// Assist flag
    assist: i32,
    /// Table name for current song
    tablename: String,
    /// Table level for current song
    tablelevel: String,
    /// Full table name (cached)
    tablefull: Option<String>,
    /// Frequency on
    freq_on: bool,
    /// Frequency string
    freq_string: Option<String>,
    /// Force no IR send
    force_no_ir_send: bool,
    /// Reverse lookup data
    reverse_lookup: Vec<String>,
}

impl PlayerResource {
    pub fn new(config: Config, pconfig: PlayerConfig) -> Self {
        let org_gauge_option = pconfig.gauge;
        let bmsresource = Some(BMSResource::new(&config, &pconfig));
        Self {
            model: None,
            margin_time: 0,
            songdata: None,
            orgmode: None,
            playerdata: PlayerData::new(),
            config,
            pconfig,
            mode: None,
            bmsresource,
            score: None,
            rscore: None,
            tscore: None,
            ranking: None,
            update_score: true,
            update_course_score: true,
            groove_gauge: None,
            gauge: None,
            replay: None,
            chart_option: None,
            bms_paths: None,
            loop_play: false,
            coursedata: None,
            course: None,
            courseindex: 0,
            coursegauge: Vec::new(),
            course_replay: Vec::new(),
            cscore: None,
            combo: 0,
            maxcombo: 0,
            org_gauge_option,
            assist: 0,
            tablename: String::new(),
            tablelevel: String::new(),
            tablefull: None,
            freq_on: false,
            freq_string: None,
            force_no_ir_send: false,
            reverse_lookup: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.course = None;
        self.courseindex = 0;
        self.cscore = None;
        self.score = None;
        // rscore is intentionally not cleared (commented out in Java)
        self.tscore = None;
        self.gauge = None;
        self.course_replay.clear();
        self.coursegauge.clear();
        self.combo = 0;
        self.maxcombo = 0;
        self.bms_paths = None;
        self.set_tablename("");
        self.set_tablelevel("");
    }

    pub fn set_bms_file(&mut self, _f: &Path, mode: BMSPlayerMode) -> bool {
        self.mode = Some(mode);
        self.replay = Some(ReplayData::new());
        // model = loadBMSModel(f, pconfig.getLnmode())
        // Phase 5+ dependency: ChartDecoder, BMSModel
        todo!("Phase 5+ dependency: loadBMSModel")
    }

    pub fn get_bms_model(&self) -> Option<()> {
        self.model
    }

    pub fn get_margin_time(&self) -> i64 {
        self.margin_time
    }

    pub fn get_play_mode(&self) -> Option<&BMSPlayerMode> {
        self.mode.as_ref()
    }

    pub fn set_play_mode(&mut self, mode: BMSPlayerMode) {
        self.mode = Some(mode);
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_player_config(&self) -> &PlayerConfig {
        &self.pconfig
    }

    pub fn media_load_finished(&self) -> bool {
        if let Some(ref bmsresource) = self.bmsresource {
            bmsresource.media_load_finished()
        } else {
            true
        }
    }

    pub fn get_score_data(&self) -> Option<&ScoreData> {
        self.score.as_ref()
    }

    pub fn set_score_data(&mut self, score: ScoreData) {
        self.score = Some(score);
    }

    pub fn get_rival_score_data(&self) -> Option<&ScoreData> {
        self.rscore.as_ref()
    }

    pub fn set_rival_score_data(&mut self, rscore: ScoreData) {
        self.rscore = Some(rscore);
    }

    pub fn get_target_score_data(&self) -> Option<&ScoreData> {
        self.tscore.as_ref()
    }

    pub fn set_target_score_data(&mut self, tscore: ScoreData) {
        self.tscore = Some(tscore);
    }

    pub fn get_ranking_data(&self) -> Option<&RankingData> {
        self.ranking.as_ref()
    }

    pub fn set_ranking_data(&mut self, ranking: RankingData) {
        self.ranking = Some(ranking);
    }

    pub fn set_course_bms_files(&mut self, _files: &[PathBuf]) -> bool {
        // Phase 5+ dependency: loadBMSModel for each file
        self.update_course_score = true;
        todo!("Phase 5+ dependency: loadBMSModel for course files")
    }

    pub fn get_course_bms_models(&self) -> Option<&Vec<()>> {
        self.course.as_ref()
    }

    pub fn set_auto_play_songs(&mut self, paths: Vec<PathBuf>, loop_play: bool) {
        self.bms_paths = Some(paths);
        self.loop_play = loop_play;
    }

    pub fn next_song(&mut self) -> bool {
        if self.bms_paths.is_none() {
            return false;
        }
        let paths = self.bms_paths.as_ref().unwrap().clone();
        let org_index = self.courseindex;
        loop {
            if self.courseindex == paths.len() {
                if self.loop_play {
                    self.courseindex = 0;
                } else {
                    return false;
                }
            }
            self.songdata = None;
            let path = paths[self.courseindex].clone();
            self.courseindex += 1;
            if self.set_bms_file(&path, BMSPlayerMode::AUTOPLAY) {
                return true;
            }
            if org_index == self.courseindex {
                break;
            }
        }
        false
    }

    pub fn next_course(&mut self) -> bool {
        self.courseindex += 1;
        if let Some(ref course) = self.course
            && self.courseindex == course.len()
        {
            return false;
        }
        self.songdata = None;
        // Phase 5+ dependency: setBMSFile with course model path
        true
    }

    pub fn get_course_index(&self) -> usize {
        self.courseindex
    }

    pub fn get_gauge(&self) -> Option<&Vec<FloatArray>> {
        self.gauge.as_ref()
    }

    pub fn set_gauge(&mut self, gauge: Vec<FloatArray>) {
        self.gauge = Some(gauge);
    }

    pub fn get_groove_gauge(&self) -> Option<&GrooveGauge> {
        self.groove_gauge.as_ref()
    }

    pub fn set_groove_gauge(&mut self, groove_gauge: GrooveGauge) {
        self.groove_gauge = Some(groove_gauge);
    }

    pub fn get_replay_data(&self) -> Option<&ReplayData> {
        self.replay.as_ref()
    }

    pub fn set_replay_data(&mut self, replay: ReplayData) {
        self.replay = Some(replay);
    }

    pub fn get_course_score_data(&self) -> Option<&ScoreData> {
        self.cscore.as_ref()
    }

    pub fn set_course_score_data(&mut self, cscore: ScoreData) {
        self.cscore = Some(cscore);
    }

    pub fn is_update_score(&self) -> bool {
        self.update_score
    }

    pub fn set_update_score(&mut self, b: bool) {
        self.update_score = b;
    }

    pub fn is_update_course_score(&self) -> bool {
        self.update_course_score
    }

    pub fn set_update_course_score(&mut self, update: bool) {
        self.update_course_score = update;
    }

    pub fn get_course_data(&self) -> Option<&CourseData> {
        self.coursedata.as_ref()
    }

    pub fn set_course_data(&mut self, coursedata: CourseData) {
        self.coursedata = Some(coursedata);
    }

    pub fn get_coursetitle(&self) -> Option<String> {
        self.coursedata.as_ref().map(|cd| cd.get_name().to_string())
    }

    pub fn get_constraint(&self) -> Vec<CourseDataConstraint> {
        if let Some(ref cd) = self.coursedata {
            cd.get_constraint().to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn get_course_replay(&self) -> &[ReplayData] {
        &self.course_replay
    }

    pub fn add_course_replay(&mut self, rd: ReplayData) {
        self.course_replay.push(rd);
    }

    pub fn get_course_gauge(&self) -> &Vec<Vec<FloatArray>> {
        &self.coursegauge
    }

    pub fn add_course_gauge(&mut self, gauge: Vec<FloatArray>) {
        self.coursegauge.push(gauge);
    }

    pub fn get_combo(&self) -> i32 {
        self.combo
    }

    pub fn set_combo(&mut self, combo: i32) {
        self.combo = combo;
    }

    pub fn get_maxcombo(&self) -> i32 {
        self.maxcombo
    }

    pub fn set_maxcombo(&mut self, maxcombo: i32) {
        self.maxcombo = maxcombo;
    }

    pub fn dispose(&mut self) {
        if let Some(mut bmsresource) = self.bmsresource.take() {
            bmsresource.dispose();
        }
    }

    pub fn get_songdata(&self) -> Option<&SongData> {
        self.songdata.as_ref()
    }

    pub fn set_songdata(&mut self, songdata: SongData) {
        self.songdata = Some(songdata);
    }

    pub fn get_bms_resource(&self) -> Option<&BMSResource> {
        self.bmsresource.as_ref()
    }

    pub fn get_org_gauge_option(&self) -> i32 {
        self.org_gauge_option
    }

    pub fn set_org_gauge_option(&mut self, org_gauge_option: i32) {
        self.org_gauge_option = org_gauge_option;
    }

    pub fn get_assist(&self) -> i32 {
        self.assist
    }

    pub fn set_assist(&mut self, assist: i32) {
        self.assist = assist;
    }

    pub fn get_tablename(&self) -> &str {
        &self.tablename
    }

    pub fn set_tablename(&mut self, tablename: &str) {
        self.tablename = tablename.to_string();
        self.tablefull = None;
    }

    pub fn get_tablelevel(&self) -> &str {
        &self.tablelevel
    }

    pub fn set_tablelevel(&mut self, tablelevel: &str) {
        self.tablelevel = tablelevel.to_string();
        self.tablefull = None;
    }

    pub fn get_table_fullname(&mut self) -> &str {
        if self.tablefull.is_none() {
            self.tablefull = Some(format!("{}{}", self.tablelevel, self.tablename));
        }
        self.tablefull.as_ref().unwrap()
    }

    pub fn get_player_data(&self) -> &PlayerData {
        &self.playerdata
    }

    pub fn set_player_data(&mut self, playerdata: PlayerData) {
        self.playerdata = playerdata;
    }

    pub fn get_chart_option(&self) -> Option<&ReplayData> {
        self.chart_option.as_ref()
    }

    pub fn set_chart_option(&mut self, chart_option: ReplayData) {
        self.chart_option = Some(chart_option);
    }

    pub fn get_original_mode(&self) -> Option<()> {
        self.orgmode
    }

    pub fn set_original_mode(&mut self, orgmode: ()) {
        self.orgmode = Some(orgmode);
    }

    pub fn is_freq_on(&self) -> bool {
        self.freq_on
    }

    pub fn set_freq_on(&mut self, freq_on: bool) {
        self.freq_on = freq_on;
    }

    pub fn get_freq_string(&self) -> Option<&str> {
        self.freq_string.as_deref()
    }

    pub fn set_freq_string(&mut self, freq_string: String) {
        self.freq_string = Some(freq_string);
    }

    pub fn is_force_no_ir_send(&self) -> bool {
        self.force_no_ir_send
    }

    pub fn set_force_no_ir_send(&mut self, force: bool) {
        self.force_no_ir_send = force;
    }

    pub fn get_reverse_lookup_data(&self) -> Vec<String> {
        // Phase 5+ dependency: TableDataAccessor, SongData matching
        todo!("Phase 5+ dependency: getReverseLookupData")
    }

    pub fn get_reverse_lookup_levels(&self) -> Vec<String> {
        // Phase 5+ dependency: TableDataAccessor, SongData matching
        todo!("Phase 5+ dependency: getReverseLookupLevels")
    }
}
