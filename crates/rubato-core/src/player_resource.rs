use std::any::Any;
use std::path::{Path, PathBuf};

use bms_model::bms_model::BMSModel;
use bms_model::bms_model_utils::set_start_note_time;
use bms_model::chart_decoder;
use bms_model::chart_information::ChartInformation;
use rubato_types::player_resource_access::PlayerResourceAccess;

use rubato_render::pixmap::Pixmap;

use crate::bms_player_mode::BMSPlayerMode;
use crate::bms_resource::BMSResource;
use crate::config::Config;
use crate::course_data::{CourseData, CourseDataConstraint};
use crate::player_config::PlayerConfig;
use crate::player_data::PlayerData;
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::stubs::*;

/// FloatArray stub (LibGDX equivalent)
pub type FloatArray = Vec<f32>;

/// PlayerResource - holds game session state for data exchange between components
pub struct PlayerResource {
    /// Margin time
    margin_time: i64,
    /// Current song data
    songdata: Option<SongData>,
    /// Original BMS mode
    orgmode: Option<()>,
    /// Player data
    pub playerdata: PlayerData,
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
    /// Ranking data (type-erased; concrete type is rubato_ir::ranking_data::RankingData)
    ranking: Option<Box<dyn Any + Send + Sync>>,
    /// Whether to update score
    pub update_score: bool,
    /// Whether to update course score
    pub update_course_score: bool,
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
    /// Course BMS models
    course: Option<Vec<BMSModel>>,
    /// Course index
    courseindex: usize,
    /// Course gauge history
    coursegauge: Vec<Vec<FloatArray>>,
    /// Course replay data
    course_replay: Vec<ReplayData>,
    /// Course score
    cscore: Option<ScoreData>,
    /// Combo count (for course play carry-over)
    pub combo: i32,
    /// Max combo count (for course play carry-over)
    pub maxcombo: i32,
    /// Original gauge option
    pub org_gauge_option: i32,
    /// Assist flag
    pub assist: i32,
    /// Table name for current song
    pub tablename: String,
    /// Table level for current song
    tablelevel: String,
    /// Full table name (cached)
    tablefull: Option<String>,
    /// Frequency on
    pub freq_on: bool,
    /// Frequency string
    freq_string: Option<String>,
    /// Force no IR send
    pub force_no_ir_send: bool,
    /// Type-erased BGA processor for reuse across plays.
    /// Concrete type: `Arc<Mutex<BGAProcessor>>` from beatoraja-play.
    /// Stored via Box<dyn Any + Send> to avoid circular dependency (core cannot import play).
    /// Java: BMSResource holds BGAProcessor, reused via PlayerResource.getBGAManager().
    bga_any: Option<Box<dyn Any + Send>>,
}

impl PlayerResource {
    pub fn new(config: Config, pconfig: PlayerConfig) -> Self {
        let org_gauge_option = pconfig.play_settings.gauge;
        let bmsresource = Some(BMSResource::new(&config, &pconfig));
        Self {
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
            bga_any: None,
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

    pub fn set_bms_file(&mut self, f: &Path, mode: BMSPlayerMode) -> bool {
        self.mode = Some(mode);
        self.replay = Some(ReplayData::new());
        let result = Self::load_bms_model(f, self.pconfig.play_settings.lnmode);
        if let Some((model, margin_time)) = result {
            if model.timelines.is_empty() {
                return false;
            }
            self.margin_time = margin_time;
            // Java: if(songdata != null) { songdata.setBMSModel(model); }
            //       else { songdata = new SongData(model, false); }
            // Preserves existing preview/difficulty during course play.
            if let Some(ref mut existing) = self.songdata {
                existing.set_bms_model(model);
            } else {
                self.songdata = Some(SongData::new_from_model(model, false));
            }
            if let Some(ref mut bmsresource) = self.bmsresource {
                bmsresource.set_bms_file(
                    self.songdata
                        .as_ref()
                        .expect("songdata is Some")
                        .bms_model()
                        .expect("get_bms_model"),
                    f,
                    &self.config,
                    self.mode.as_ref().expect("mode is Some"),
                );
            }
            true
        } else {
            log::warn!(
                "chart does not exist or an error occurred during parsing: {}",
                f.display()
            );
            false
        }
    }

    /// Reload the current BMS file from disk.
    /// Preserves tablename and tablelevel across clear().
    /// Java: PlayerResource.reloadBMSFile()
    pub fn reload_bms_file(&mut self) {
        if let Some(path_str) = self.bms_model().and_then(|m| m.path()) {
            let path = PathBuf::from(&path_str);
            if let Some((model, margin_time)) =
                Self::load_bms_model(&path, self.pconfig.play_settings.lnmode)
            {
                self.margin_time = margin_time;
                let songdata = SongData::new_from_model(model, false);
                self.songdata = Some(songdata);
            }
        }
        let name = std::mem::take(&mut self.tablename);
        let lev = std::mem::take(&mut self.tablelevel);
        self.clear();
        self.tablename = name;
        self.tablelevel = lev;
        self.tablefull = None;
    }

    /// Load a BMS model from path, applying start note time and validation.
    /// Returns (model, margin_time).
    /// Java: PlayerResource.loadBMSModel(Path, int lnmode)
    pub fn load_bms_model(path: &Path, lnmode: i32) -> Option<(BMSModel, i64)> {
        let mut decoder = chart_decoder::decoder(path)?;
        let info = ChartInformation::new(
            Some(path.to_path_buf()),
            bms_model::bms_model::LnType::from_i32(lnmode),
            None,
        );
        let mut model = decoder.decode(info)?;
        let margin_time = set_start_note_time(&mut model, 1000);
        rubato_types::bms_player_rule::BMSPlayerRule::validate(&mut model);
        Some((model, margin_time))
    }

    pub fn bms_model(&self) -> Option<&BMSModel> {
        self.songdata.as_ref().and_then(|sd| sd.bms_model())
    }

    pub fn margin_time(&self) -> i64 {
        self.margin_time
    }

    pub fn play_mode(&self) -> Option<&BMSPlayerMode> {
        self.mode.as_ref()
    }

    pub fn set_play_mode(&mut self, mode: BMSPlayerMode) {
        self.mode = Some(mode);
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn player_config(&self) -> &PlayerConfig {
        &self.pconfig
    }

    pub fn media_load_finished(&self) -> bool {
        if let Some(ref bmsresource) = self.bmsresource {
            bmsresource.media_load_finished()
        } else {
            true
        }
    }

    pub fn score_data(&self) -> Option<&ScoreData> {
        self.score.as_ref()
    }

    pub fn set_score_data(&mut self, score: ScoreData) {
        self.score = Some(score);
    }

    pub fn rival_score_data(&self) -> Option<&ScoreData> {
        self.rscore.as_ref()
    }

    pub fn set_rival_score_data(&mut self, rscore: ScoreData) {
        self.rscore = Some(rscore);
    }

    pub fn target_score_data(&self) -> Option<&ScoreData> {
        self.tscore.as_ref()
    }

    pub fn set_target_score_data(&mut self, tscore: ScoreData) {
        self.tscore = Some(tscore);
    }

    pub fn ranking_data_any(&self) -> Option<&dyn Any> {
        self.ranking.as_ref().map(|b| b.as_ref() as &dyn Any)
    }

    pub fn set_ranking_data_any_box(&mut self, ranking: Box<dyn Any + Send + Sync>) {
        self.ranking = Some(ranking);
    }

    pub fn clear_ranking_data(&mut self) {
        self.ranking = None;
    }

    pub fn set_course_bms_files(&mut self, files: &[PathBuf]) -> bool {
        let lnmode = self.pconfig.play_settings.lnmode;
        let mut models = Vec::with_capacity(files.len());
        for f in files {
            match Self::load_bms_model(f, lnmode) {
                Some((model, _margin_time)) => models.push(model),
                None => {
                    log::warn!("failed to load BMS model for course file: {:?}", f);
                    return false;
                }
            }
        }
        self.course = Some(models);
        self.update_course_score = true;
        true
    }

    pub fn course_bms_models(&self) -> Option<&Vec<BMSModel>> {
        self.course.as_ref()
    }

    pub fn set_auto_play_songs(&mut self, paths: Vec<PathBuf>, loop_play: bool) {
        self.bms_paths = Some(paths);
        self.loop_play = loop_play;
    }

    pub fn next_song(&mut self) -> bool {
        let paths_len = match self.bms_paths.as_ref() {
            Some(p) => p.len(),
            None => return false,
        };
        let org_index = self.courseindex;
        loop {
            if self.courseindex == paths_len {
                if self.loop_play {
                    self.courseindex = 0;
                } else {
                    return false;
                }
            }
            self.songdata = None;
            let path =
                self.bms_paths.as_ref().expect("bms_paths is Some")[self.courseindex].clone();
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
        // Load the next course chart (Java: setBMSFile(Paths.get(course[courseindex].getPath()), mode))
        let path = self
            .course
            .as_ref()
            .and_then(|models| models.get(self.courseindex))
            .and_then(|model| model.path());
        let mode = self.mode;
        if let (Some(path_str), Some(mode)) = (path, mode) {
            self.set_bms_file(Path::new(&path_str), mode);
        }
        true
    }

    pub fn course_index(&self) -> usize {
        self.courseindex
    }

    pub fn gauge(&self) -> Option<&Vec<FloatArray>> {
        self.gauge.as_ref()
    }

    pub fn set_gauge(&mut self, gauge: Vec<FloatArray>) {
        self.gauge = Some(gauge);
    }

    pub fn groove_gauge(&self) -> Option<&GrooveGauge> {
        self.groove_gauge.as_ref()
    }

    pub fn set_groove_gauge(&mut self, groove_gauge: GrooveGauge) {
        self.groove_gauge = Some(groove_gauge);
    }

    pub fn replay_data(&self) -> Option<&ReplayData> {
        self.replay.as_ref()
    }

    pub fn set_replay_data(&mut self, replay: ReplayData) {
        self.replay = Some(replay);
    }

    pub fn course_score_data(&self) -> Option<&ScoreData> {
        self.cscore.as_ref()
    }

    pub fn set_course_score_data(&mut self, cscore: ScoreData) {
        self.cscore = Some(cscore);
    }

    pub fn course_data(&self) -> Option<&CourseData> {
        self.coursedata.as_ref()
    }

    pub fn set_course_data(&mut self, coursedata: CourseData) {
        self.coursedata = Some(coursedata);
    }

    pub fn coursetitle(&self) -> Option<&str> {
        self.coursedata.as_ref().map(|cd| cd.name())
    }

    pub fn constraint(&self) -> Vec<CourseDataConstraint> {
        if let Some(ref cd) = self.coursedata {
            cd.constraint.to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn course_replay(&self) -> &[ReplayData] {
        &self.course_replay
    }

    pub fn add_course_replay(&mut self, rd: ReplayData) {
        self.course_replay.push(rd);
    }

    pub fn course_gauge(&self) -> &Vec<Vec<FloatArray>> {
        &self.coursegauge
    }

    pub fn add_course_gauge(&mut self, gauge: Vec<FloatArray>) {
        self.coursegauge.push(gauge);
    }

    pub fn dispose(&mut self) {
        if let Some(mut bmsresource) = self.bmsresource.take() {
            bmsresource.dispose();
        }
    }

    pub fn songdata(&self) -> Option<&SongData> {
        self.songdata.as_ref()
    }

    pub fn set_songdata(&mut self, songdata: SongData) {
        self.songdata = Some(songdata);
    }

    pub fn bms_resource(&self) -> Option<&BMSResource> {
        self.bmsresource.as_ref()
    }

    pub fn set_tablename(&mut self, tablename: &str) {
        self.tablename = tablename.to_string();
        self.tablefull = None;
    }

    pub fn tablelevel(&self) -> &str {
        &self.tablelevel
    }

    pub fn set_tablelevel(&mut self, tablelevel: &str) {
        self.tablelevel = tablelevel.to_string();
        self.tablefull = None;
    }

    pub fn table_fullname(&mut self) -> &str {
        if self.tablefull.is_none() {
            self.tablefull = Some(format!("{}{}", self.tablelevel, self.tablename));
        }
        self.tablefull.as_ref().expect("tablefull is Some")
    }

    pub fn player_data(&self) -> &PlayerData {
        &self.playerdata
    }

    pub fn chart_option(&self) -> Option<&ReplayData> {
        self.chart_option.as_ref()
    }

    pub fn set_chart_option(&mut self, chart_option: ReplayData) {
        self.chart_option = Some(chart_option);
    }

    pub fn original_mode(&self) -> Option<()> {
        self.orgmode
    }

    pub fn set_original_mode(&mut self, orgmode: ()) {
        self.orgmode = Some(orgmode);
    }

    pub fn freq_string(&self) -> Option<&str> {
        self.freq_string.as_deref()
    }

    pub fn set_freq_string(&mut self, freq_string: String) {
        self.freq_string = Some(freq_string);
    }

    pub fn reverse_lookup_data(&self) -> Vec<String> {
        let Some(songdata) = self.songdata.as_ref() else {
            return Vec::new();
        };
        let url_set: std::collections::HashSet<&str> = self
            .config
            .paths
            .table_url
            .iter()
            .map(|s| s.as_str())
            .collect();
        let tdaccessor = crate::table_data_accessor::TableDataAccessor::new(
            self.config.paths.tablepath.as_str(),
        );
        let tds = tdaccessor.read_all();
        let mut result = Vec::new();
        for td in &tds {
            if !url_set.contains(td.url.as_str()) {
                continue;
            }
            for tf in &td.folder {
                let found = tf.songs.iter().any(|ts| {
                    (!ts.md5.is_empty() && ts.md5 == songdata.md5)
                        || (!ts.sha256.is_empty() && ts.sha256 == songdata.sha256)
                });
                if found {
                    result.push(format!("{} {}", td.name, tf.name()));
                    break;
                }
            }
        }
        result
    }

    pub fn reverse_lookup_levels(&self) -> Vec<String> {
        let Some(songdata) = self.songdata.as_ref() else {
            return Vec::new();
        };
        let url_set: std::collections::HashSet<&str> = self
            .config
            .paths
            .table_url
            .iter()
            .map(|s| s.as_str())
            .collect();
        let tdaccessor = crate::table_data_accessor::TableDataAccessor::new(
            self.config.paths.tablepath.as_str(),
        );
        let tds = tdaccessor.read_all();
        let mut result = Vec::new();
        for td in &tds {
            if !url_set.contains(td.url.as_str()) {
                continue;
            }
            for tf in &td.folder {
                let found = tf.songs.iter().any(|ts| {
                    (!ts.md5.is_empty() && ts.md5 == songdata.md5)
                        || (!ts.sha256.is_empty() && ts.sha256 == songdata.sha256)
                });
                if found {
                    result.push(tf.name().to_string());
                    break;
                }
            }
        }
        result
    }
}

impl PlayerResourceAccess for PlayerResource {
    fn into_any_send(self: Box<Self>) -> Box<dyn Any + Send> {
        self
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn player_config(&self) -> &PlayerConfig {
        &self.pconfig
    }

    fn player_config_mut(&mut self) -> Option<&mut PlayerConfig> {
        Some(&mut self.pconfig)
    }

    fn score_data(&self) -> Option<&ScoreData> {
        self.score.as_ref()
    }

    fn rival_score_data(&self) -> Option<&ScoreData> {
        self.rscore.as_ref()
    }

    fn target_score_data(&self) -> Option<&ScoreData> {
        self.tscore.as_ref()
    }

    fn set_target_score_data(&mut self, score: ScoreData) {
        self.tscore = Some(score);
    }

    fn course_score_data(&self) -> Option<&ScoreData> {
        self.cscore.as_ref()
    }

    fn set_course_score_data(&mut self, score: ScoreData) {
        self.cscore = Some(score);
    }

    fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
        self.songdata.as_ref()
    }

    fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
        self.songdata.as_mut()
    }

    fn set_songdata(&mut self, data: Option<rubato_types::song_data::SongData>) {
        self.songdata = data;
    }

    fn replay_data(&self) -> Option<&ReplayData> {
        self.replay.as_ref()
    }

    fn replay_data_mut(&mut self) -> Option<&mut ReplayData> {
        self.replay.as_mut()
    }

    fn course_replay(&self) -> &[ReplayData] {
        &self.course_replay
    }

    fn add_course_replay(&mut self, rd: ReplayData) {
        self.course_replay.push(rd);
    }

    fn course_data(&self) -> Option<&CourseData> {
        self.coursedata.as_ref()
    }

    fn course_index(&self) -> usize {
        self.courseindex
    }

    fn next_course(&mut self) -> bool {
        PlayerResource::next_course(self)
    }

    fn constraint(&self) -> Vec<CourseDataConstraint> {
        PlayerResource::constraint(self)
    }

    fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
        self.gauge.as_ref()
    }

    fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
        self.groove_gauge.as_ref()
    }

    fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        &self.coursegauge
    }

    fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
        self.coursegauge.push(gauge);
    }

    fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
        &mut self.coursegauge
    }

    fn score_data_mut(&mut self) -> Option<&mut ScoreData> {
        self.score.as_mut()
    }

    fn course_replay_mut(&mut self) -> &mut Vec<ReplayData> {
        &mut self.course_replay
    }

    fn maxcombo(&self) -> i32 {
        self.maxcombo
    }

    fn org_gauge_option(&self) -> i32 {
        self.org_gauge_option
    }

    fn set_org_gauge_option(&mut self, val: i32) {
        self.org_gauge_option = val;
    }

    fn assist(&self) -> i32 {
        self.assist
    }

    fn is_update_score(&self) -> bool {
        self.update_score
    }

    fn is_update_course_score(&self) -> bool {
        self.update_course_score
    }

    fn is_force_no_ir_send(&self) -> bool {
        self.force_no_ir_send
    }

    fn is_freq_on(&self) -> bool {
        self.freq_on
    }

    fn reverse_lookup_data(&self) -> Vec<String> {
        PlayerResource::reverse_lookup_data(self)
    }

    fn reverse_lookup_levels(&self) -> Vec<String> {
        PlayerResource::reverse_lookup_levels(self)
    }

    fn clear(&mut self) {
        PlayerResource::clear(self)
    }

    fn set_bms_file(&mut self, path: &Path, mode_type: i32, mode_id: i32) -> bool {
        let mode = match mode_type {
            0 => BMSPlayerMode::new(crate::bms_player_mode::Mode::Play),
            1 => BMSPlayerMode::new(crate::bms_player_mode::Mode::Practice),
            2 => BMSPlayerMode::new(crate::bms_player_mode::Mode::Autoplay),
            3 => BMSPlayerMode::new_with_id(crate::bms_player_mode::Mode::Replay, mode_id),
            _ => BMSPlayerMode::new(crate::bms_player_mode::Mode::Play),
        };
        PlayerResource::set_bms_file(self, path, mode)
    }

    fn set_course_bms_files(&mut self, files: &[PathBuf]) -> bool {
        PlayerResource::set_course_bms_files(self, files)
    }

    fn set_tablename(&mut self, name: &str) {
        PlayerResource::set_tablename(self, name)
    }

    fn set_tablelevel(&mut self, level: &str) {
        PlayerResource::set_tablelevel(self, level)
    }

    fn set_rival_score_data_option(&mut self, score: Option<ScoreData>) {
        self.rscore = score;
    }

    fn set_chart_option_data(&mut self, option: Option<ReplayData>) {
        self.chart_option = option;
    }

    fn set_course_data(&mut self, data: CourseData) {
        PlayerResource::set_course_data(self, data)
    }

    fn clear_course_data(&mut self) {
        self.coursedata = None;
    }

    fn reload_bms_file(&mut self) {
        PlayerResource::reload_bms_file(self)
    }

    fn set_player_config_gauge(&mut self, gauge: i32) {
        self.pconfig.play_settings.gauge = gauge;
    }

    fn set_auto_play_songs(&mut self, paths: Vec<PathBuf>, loop_play: bool) {
        PlayerResource::set_auto_play_songs(self, paths, loop_play)
    }

    fn next_song(&mut self) -> bool {
        PlayerResource::next_song(self)
    }

    fn bms_model(&self) -> Option<&bms_model::bms_model::BMSModel> {
        PlayerResource::bms_model(self)
    }

    fn set_player_data(&mut self, player_data: rubato_types::player_data::PlayerData) {
        self.playerdata = player_data;
    }

    fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
        match self.course_bms_models() {
            Some(models) => models
                .iter()
                .map(|m| {
                    // Build SongData from model metadata without consuming the model
                    let mut sd = rubato_types::song_data::SongData::default();
                    sd.title = m.get_title().to_string();
                    sd.set_subtitle(m.sub_title().to_string());
                    sd.genre = m.genre().to_string();
                    sd.set_artist(m.artist().to_string());
                    sd.set_subartist(m.sub_artist().to_string());
                    if let Some(p) = m.path() {
                        sd.set_path(p);
                    }
                    sd.md5 = m.md5().to_string();
                    sd.sha256 = m.sha256().to_string();
                    sd.notes = m.total_notes();
                    sd.length = m.last_time();
                    sd.mode = m.mode().map(|mode| mode.id()).unwrap_or(0);
                    sd
                })
                .collect(),
            None => vec![],
        }
    }

    fn set_bms_banner_raw(&mut self, data: Option<(i32, i32, Vec<u8>)>) {
        if let Some(res) = &mut self.bmsresource {
            let pixmap = data.map(|(w, h, d)| Pixmap::from_rgba_data(w, h, d));
            res.set_banner(pixmap);
        }
    }

    fn set_bms_stagefile_raw(&mut self, data: Option<(i32, i32, Vec<u8>)>) {
        if let Some(res) = &mut self.bmsresource {
            let pixmap = data.map(|(w, h, d)| Pixmap::from_rgba_data(w, h, d));
            res.set_stagefile(pixmap);
        }
    }

    fn bga_any(&self) -> Option<&(dyn Any + Send)> {
        self.bga_any.as_deref()
    }

    fn set_bga_any(&mut self, bga: Box<dyn Any + Send>) {
        self.bga_any = Some(bga);
    }

    fn set_ranking_data_any(&mut self, data: Option<Box<dyn Any + Send + Sync>>) {
        self.ranking = data;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_bms_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("test-bms")
    }

    #[test]
    fn set_bms_file_loads_model_from_bms() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let bms_path = test_bms_dir().join("minimal_7k.bms");
        assert!(
            bms_path.exists(),
            "test BMS file must exist: {:?}",
            bms_path
        );

        let result = resource.set_bms_file(&bms_path, BMSPlayerMode::PLAY);
        assert!(result, "set_bms_file should return true on success");
        assert!(
            resource.bms_model().is_some(),
            "model should be Some after successful load"
        );
    }

    #[test]
    fn set_bms_file_returns_false_for_nonexistent_file() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let result = resource.set_bms_file(Path::new("/nonexistent/file.bms"), BMSPlayerMode::PLAY);
        assert!(
            !result,
            "set_bms_file should return false for nonexistent file"
        );
        assert!(
            resource.bms_model().is_none(),
            "model should be None after failed load"
        );
    }

    #[test]
    fn set_bms_file_returns_false_for_unsupported_extension() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let result = resource.set_bms_file(Path::new("/some/file.txt"), BMSPlayerMode::PLAY);
        assert!(
            !result,
            "set_bms_file should return false for unsupported extension"
        );
    }

    #[test]
    fn set_bms_file_sets_margin_time() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let bms_path = test_bms_dir().join("minimal_7k.bms");
        resource.set_bms_file(&bms_path, BMSPlayerMode::PLAY);
        // margin_time is set by set_start_note_time (may be 0 if first note >= 1000ms)
        // Just verify it doesn't panic and the field is accessible
        let _margin = resource.margin_time();
    }

    #[test]
    fn set_bms_file_populates_songdata() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let bms_path = test_bms_dir().join("minimal_7k.bms");
        assert!(
            bms_path.exists(),
            "test BMS file must exist: {:?}",
            bms_path
        );

        let result = resource.set_bms_file(&bms_path, BMSPlayerMode::PLAY);
        assert!(result, "set_bms_file should return true on success");

        // songdata() should return Some after loading a BMS file
        let songdata = resource
            .songdata()
            .expect("songdata should be Some after successful set_bms_file");

        // md5 should be populated from the loaded model
        assert!(!songdata.md5.is_empty(), "songdata.md5 should be non-empty");

        // PlayerResourceAccess trait method should also return Some
        let trait_songdata = PlayerResourceAccess::songdata(&resource as &dyn PlayerResourceAccess);
        assert!(
            trait_songdata.is_some(),
            "PlayerResourceAccess::songdata should return Some"
        );
    }

    #[test]
    fn reload_bms_file_preserves_table_info() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let bms_path = test_bms_dir().join("minimal_7k.bms");
        assert!(resource.set_bms_file(&bms_path, BMSPlayerMode::PLAY));

        // Set table info that should be preserved across reload
        resource.set_tablename("insane");
        resource.set_tablelevel("★12");

        resource.reload_bms_file();

        // Model should still be loaded after reload
        assert!(
            resource.bms_model().is_some(),
            "model should be Some after reload"
        );
        // Table info should be preserved
        assert_eq!(resource.tablename.as_str(), "insane");
        assert_eq!(resource.tablelevel(), "★12");
        // Other fields should be cleared
        assert!(resource.score_data().is_none());
    }

    #[test]
    fn reload_bms_file_without_model_just_clears() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        // No model loaded — reload should just clear without panicking
        resource.set_tablename("test");
        resource.set_tablelevel("1");
        resource.reload_bms_file();

        assert_eq!(resource.tablename.as_str(), "test");
        assert_eq!(resource.tablelevel(), "1");
    }

    #[test]
    fn set_course_bms_files_loads_models() {
        let config = Config::default();
        let pconfig = PlayerConfig::default();
        let mut resource = PlayerResource::new(config, pconfig);

        let bms_path = test_bms_dir().join("minimal_7k.bms");
        let files = vec![bms_path];
        let result = resource.set_course_bms_files(&files);
        assert!(result, "set_course_bms_files should return true on success");
        assert!(
            resource.course_bms_models().is_some(),
            "course models should be Some after successful load"
        );
        assert_eq!(
            resource.course_bms_models().unwrap().len(),
            1,
            "should have loaded exactly 1 course model"
        );
    }
}
