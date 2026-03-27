use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;

use super::super::bar::bar::Bar;
use super::super::bar::grade_bar::GradeBar;
use super::LoaderContext;
use crate::state::select::*;

/// Get a string identifier for a Bar variant (simulates Java getClass())
pub(super) fn bar_class_name(bar: &Bar) -> &'static str {
    match bar {
        Bar::Song(_) => "SongBar",
        Bar::Folder(_) => "FolderBar",
        Bar::Command(_) => "CommandBar",
        Bar::Container(_) => "ContainerBar",
        Bar::Hash(_) => "HashBar",
        Bar::Table(_) => "TableBar",
        Bar::Grade(_) => "GradeBar",
        Bar::RandomCourse(_) => "RandomCourseBar",
        Bar::SearchWord(_) => "SearchWordBar",
        Bar::SameFolder(_) => "SameFolderBar",
        Bar::Executable(_) => "ExecutableBar",
        Bar::Function(_) => "FunctionBar",
        Bar::ContextMenu(_) => "ContextMenuBar",
        Bar::LeaderBoard(_) => "LeaderBoardBar",
    }
}

/// A no-op TableAccessor for course tables.
/// Corresponds to the anonymous TableAccessor in Java BarManager.init()
pub(super) struct CourseTableAccessor;
impl TableAccessor for CourseTableAccessor {
    fn name(&self) -> &str {
        "course"
    }
    fn read(&self) -> Option<TableData> {
        Some(TableData {
            name: "COURSE".to_string(),
            course: CourseDataAccessor::new("course").read_all(),
            ..Default::default()
        })
    }
    fn write(&self, _td: &mut TableData) {
        // No-op for course tables
    }
}

/// Command folder definition (loaded from JSON)
/// Translates: bms.player.beatoraja.select.BarManager.CommandFolder
#[derive(Clone, Debug, Default, Deserialize)]
pub struct CommandFolder {
    pub name: Option<String>,
    #[serde(default)]
    pub folder: Vec<CommandFolder>,
    pub sql: Option<String>,
    #[serde(default)]
    pub rcourse: Vec<RandomCourseData>,
    #[serde(default)]
    pub showall: bool,
}

impl CommandFolder {
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
    pub fn folder(&self) -> &[CommandFolder] {
        &self.folder
    }
    pub fn sql(&self) -> Option<&str> {
        self.sql.as_deref()
    }
    pub fn random_course(&self) -> &[RandomCourseData] {
        &self.rcourse
    }
    pub fn is_showall(&self) -> bool {
        self.showall
    }
}

/// Random folder definition (loaded from JSON)
/// Translates: bms.player.beatoraja.select.BarManager.RandomFolder
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RandomFolder {
    pub name: Option<String>,
    pub filter: Option<HashMap<String, serde_json::Value>>,
}

impl RandomFolder {
    pub fn name(&self) -> String {
        format!("[RANDOM] {}", self.name.as_deref().unwrap_or(""))
    }

    pub fn filter(&self) -> Option<&HashMap<String, serde_json::Value>> {
        self.filter.as_ref()
    }

    pub fn filter_song(&self, score_data: Option<&ScoreData>) -> bool {
        let filter = match &self.filter {
            Some(f) => f,
            None => return true,
        };

        for (key, value) in filter {
            // In Java: uses reflection to call getters on ScoreData
            // This is a simplified version that handles integer comparison
            if let Some(int_value) = value.as_i64() {
                if let Some(score) = score_data {
                    let property_value = score_data_property(score, key);
                    if property_value != int_value {
                        return false;
                    }
                } else if int_value != 0 {
                    return false;
                }
                continue;
            }

            // String filter with comparison operators
            if let Some(str_value) = value.as_str() {
                let parts: Vec<&str> = str_value.split("&&").collect();
                for part in parts {
                    let part = part.trim();
                    if let Some(score) = score_data {
                        let property_value = score_data_property(score, key);
                        if !evaluate_filter_expression(part, property_value) {
                            return false;
                        }
                    } else if !part.is_empty() && !part.starts_with('<') {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub(super) fn score_data_property(score: &ScoreData, key: &str) -> i64 {
    match key {
        // Direct ScoreData fields
        "clear" => score.clear as i64,
        "date" => score.date,
        "playcount" => score.playcount as i64,
        "clearcount" => score.clearcount as i64,
        "combo" => score.maxcombo as i64,
        "mode" => score.mode as i64,
        "notes" => score.notes as i64,
        "passnotes" => score.passnotes as i64,
        "minbp" => score.minbp as i64,
        "state" => score.state as i64,
        // Computed
        "exscore" => score.exscore() as i64,
        // JudgeCounts (flattened)
        "epg" => score.judge_counts.epg as i64,
        "lpg" => score.judge_counts.lpg as i64,
        "egr" => score.judge_counts.egr as i64,
        "lgr" => score.judge_counts.lgr as i64,
        "egd" => score.judge_counts.egd as i64,
        "lgd" => score.judge_counts.lgd as i64,
        "ebd" => score.judge_counts.ebd as i64,
        "lbd" => score.judge_counts.lbd as i64,
        "epr" => score.judge_counts.epr as i64,
        "lpr" => score.judge_counts.lpr as i64,
        "ems" => score.judge_counts.ems as i64,
        "lms" => score.judge_counts.lms as i64,
        // TimingStats (flattened)
        "avgjudge" => score.timing_stats.avgjudge,
        "totalDuration" => score.timing_stats.total_duration,
        // PlayOption (flattened)
        "option" => score.play_option.option as i64,
        "random" => score.play_option.random as i64,
        "seed" => score.play_option.seed,
        "assist" => score.play_option.assist as i64,
        "gauge" => score.play_option.gauge as i64,
        _ => 0,
    }
}

pub(super) fn evaluate_filter_expression(expr: &str, property_value: i64) -> bool {
    if expr.is_empty() {
        return true;
    }
    if let Some(stripped) = expr.strip_prefix(">=") {
        if let Ok(v) = stripped.parse::<i64>() {
            return property_value >= v;
        }
    } else if let Some(stripped) = expr.strip_prefix("<=") {
        if let Ok(v) = stripped.parse::<i64>() {
            return property_value <= v;
        }
    } else if let Some(stripped) = expr.strip_prefix('>') {
        if let Ok(v) = stripped.parse::<i64>() {
            return property_value > v;
        }
    } else if let Some(stripped) = expr.strip_prefix('<')
        && let Ok(v) = stripped.parse::<i64>()
    {
        return property_value < v;
    } else if let Some(val_str) = expr.strip_prefix("==") {
        if let Ok(v) = val_str.parse::<i64>() {
            return property_value == v;
        }
    } else if let Ok(v) = expr.parse::<i64>() {
        return property_value == v;
    }
    log::warn!("Unparseable filter expression: {:?}", expr);
    false
}

/// Random course result
pub(super) struct RandomCourseResult {
    pub course: GradeBar,
    pub dir_string: String,
}

/// Thread for loading score data, banners, and stagefiles for bar contents.
/// Corresponds to Java BarManager.BarContentsLoaderThread
pub struct BarContentsLoaderThread {
    stop: Arc<AtomicBool>,
}

impl BarContentsLoaderThread {
    /// Create a new bar contents loader with a shared stop flag.
    pub fn new(stop: Arc<AtomicBool>) -> Self {
        Self { stop }
    }

    /// Run the loader on the given bars.
    /// Corresponds to Java BarContentsLoaderThread.run()
    pub fn run(&self, bars: &mut [Bar], ctx: &mut LoaderContext) {
        let lnmode = ctx.player_config.play_settings.lnmode;

        // Phase 1: Load scores
        for bar in bars.iter_mut() {
            if self.is_stopped() {
                return;
            }

            // Extract song data to avoid overlapping borrows
            let song_info = bar
                .as_song_bar()
                .filter(|sb| sb.exists_song())
                .map(|sb| sb.song_data().clone());

            if let Some(sd) = song_info {
                // Load player score
                if bar.score().is_none() {
                    let mut score = None;
                    if let Some(ref mut cache) = ctx.score_cache {
                        score = cache.read_score_data(&sd, lnmode).cloned();
                    }
                    if score.is_none()
                        && let Some(read_fn) = ctx.read_score_by_hash_fn
                    {
                        let has_ln = sd.chart.has_undefined_long_note();
                        score = read_fn(&sd.file.sha256, has_ln, lnmode);
                    }
                    bar.set_score(score);
                }

                // Load rival score
                if let Some(ref mut rival) = ctx.rival_cache
                    && bar.rival_score().is_none()
                {
                    let rival_score = rival.read_score_data(&sd, lnmode).cloned();
                    if let Some(mut rs) = rival_score {
                        if let Some(ref name) = ctx.rival_name {
                            rs.player = name.clone();
                        }
                        bar.set_rival_score(Some(rs));
                    }
                }

                // Replay existence check
                // Java: for(int i = 0; i < MusicSelector.REPLAY; i++) { ... }
                if let Some(exists_fn) = ctx.exists_replay_fn {
                    let has_ln = sd.chart.has_undefined_long_note();
                    if let Some(sb) = bar.as_selectable_bar_mut() {
                        for i in 0..super::super::music_selector::REPLAY {
                            sb.set_exists_replay(
                                i as i32,
                                exists_fn(&sd.file.sha256, has_ln, lnmode, i as i32),
                            );
                        }
                    }
                }
            } else if let Some(gb) = bar.as_grade_bar()
                && gb.exists_all_songs()
            {
                // Load grade bar scores via combined hash
                if let Some(read_fn) = &ctx.read_score_by_hash_fn {
                    let lnmode = ctx.player_config.play_settings.lnmode;
                    let songs = gb.song_datas();
                    let combined_hash: String =
                        songs.iter().map(|s| s.file.sha256.as_str()).collect();
                    let has_ln = songs.iter().any(|s| s.chart.has_undefined_long_note());
                    // option 0=normal, 1=mirror, 2=random
                    let mode_base = if has_ln { lnmode } else { 0 };
                    let score = read_fn(&combined_hash, has_ln, mode_base);
                    bar.set_score(score);
                    if let Some(gb_mut) = bar.as_grade_bar_mut() {
                        gb_mut.mscore = read_fn(&combined_hash, has_ln, mode_base + 10);
                        gb_mut.rscore = read_fn(&combined_hash, has_ln, mode_base + 20);
                    }
                }
            }

            // Update folder status
            if ctx.is_folderlamp
                && bar.is_directory_bar()
                && let Some(songdb) = ctx.songdb
            {
                if let Some(fb) = bar.as_folder_bar_mut() {
                    fb.update_folder_status(songdb);
                } else if let Some(hb) = bar.as_hash_bar_mut() {
                    hb.update_folder_status(songdb);
                } else if let Some(swb) = bar.as_search_word_bar_mut() {
                    swb.update_folder_status(songdb);
                } else if let Bar::Command(cb) = bar
                    && let Some(ref cmd_ctx) = ctx.command_bar_ctx
                {
                    cb.update_folder_status(songdb, cmd_ctx);
                }
            }
        }

        // Phase 2: Load song information
        if let Some(info_db) = ctx.song_info_db {
            for bar in bars.iter_mut() {
                if self.is_stopped() {
                    return;
                }
                if let Some(sb) = bar.as_song_bar_mut()
                    && sb.exists_song()
                    && sb.song_data().info.is_none()
                {
                    let sha256 = sb.song_data().file.sha256.clone();
                    if let Some(info) = info_db.information(&sha256) {
                        sb.song_data_mut().info = Some(info);
                    }
                }
            }
        }

        // Phase 3: Load banners and stagefiles
        // Java: for (Bar bar : bars) { if (bar instanceof SongBar && ...) { ... } }
        for bar in bars.iter_mut() {
            if self.is_stopped() {
                return;
            }

            // Extract song data to avoid overlapping borrows (immutable sb → mutable bar)
            let song_info = bar.as_song_bar().filter(|sb| sb.exists_song()).map(|sb| {
                let sd = sb.song_data();
                (
                    sd.file.banner.clone(),
                    sd.file.stagefile.clone(),
                    sd.file.path().map(|s| s.to_string()),
                )
            });

            if let Some((banner_name, stagefile_name, song_path)) = song_info {
                // Load banner
                // Java: Path bannerfile = Paths.get(song.getPath()).getParent().resolve(song.getBanner());
                //        if (song.getBanner().length() > 0 && Files.exists(bannerfile)) {
                //            songbar.setBanner(select.getBannerResource().get(bannerfile.toString()));
                //        }
                if !banner_name.is_empty()
                    && let Some(ref path) = song_path
                    && let Some(parent) = Path::new(path).parent()
                {
                    let banner_path = parent.join(&banner_name);
                    if banner_path.exists() {
                        if let Some(banner_pool) = ctx.banner_resource {
                            let banner_key = banner_path.to_string_lossy().to_string();
                            let pixmap = banner_pool.get_and_use(&banner_key, |p| p.clone());
                            if let Some(pix) = pixmap
                                && let Some(sb) = bar.as_song_bar_mut()
                            {
                                sb.banner = Some(pix);
                            }
                        } else {
                            log::debug!("Banner loading skipped (no pool): {:?}", banner_path);
                        }
                    }
                }

                // Load stagefile
                // Java: Path stagefilefile = Paths.get(song.getPath()).getParent().resolve(song.getStagefile());
                //        if (song.getStagefile().length() > 0 && Files.exists(stagefilefile)) {
                //            songbar.setStagefile(select.getStagefileResource().get(stagefilefile.toString()));
                //        }
                if !stagefile_name.is_empty()
                    && let Some(ref path) = song_path
                    && let Some(parent) = Path::new(path).parent()
                {
                    let stage_path = parent.join(&stagefile_name);
                    if stage_path.exists() {
                        if let Some(stage_pool) = ctx.stagefile_resource {
                            let stage_key = stage_path.to_string_lossy().to_string();
                            let pixmap = stage_pool.get_and_use(&stage_key, |p| p.clone());
                            if let Some(pix) = pixmap
                                && let Some(sb) = bar.as_song_bar_mut()
                            {
                                sb.stagefile = Some(pix);
                            }
                        } else {
                            log::debug!("Stagefile loading skipped (no pool): {:?}", stage_path);
                        }
                    }
                }
            }
        }
    }

    /// Stop the loader.
    pub fn stop_running(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Check if the loader has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}
