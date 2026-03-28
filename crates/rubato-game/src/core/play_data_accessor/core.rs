use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::core::score_data_log_database_accessor::ScoreDataLogDatabaseAccessor;
use crate::core::score_database_accessor::{ScoreDataCollector, ScoreDatabaseAccessor, SongData};
use crate::core::score_log_database_accessor::{ScoreLog, ScoreLogDatabaseAccessor};
use rubato_types::clear_type::ClearType;
use rubato_types::config::Config;
use rubato_types::course_data::CourseDataConstraint;
use rubato_types::player_data::PlayerData;
use rubato_types::replay_data::ReplayData;
use rubato_types::score_data::{ScoreData, SongTrophy};

use super::{PlayDataAccessor, REPLAY};

/// Context for writing a single-song score entry.
pub struct ScoreWriteContext<'a> {
    pub hash: &'a str,
    pub contains_undefined_ln: bool,
    pub total_notes: i32,
    pub lnmode: i32,
    pub update_score: bool,
    pub last_note_time_us: i64,
}

/// Context for writing a course score entry.
pub struct CourseScoreWriteContext<'a> {
    pub hashes: &'a [&'a str],
    pub total_notes: i32,
    pub ln: bool,
    pub lnmode: i32,
    pub option: i32,
    pub constraint: &'a [CourseDataConstraint],
    pub update_score: bool,
}

impl PlayDataAccessor {
    /// Creates a no-op PlayDataAccessor with no database connections.
    /// All read methods return None/false; all write methods do nothing.
    pub fn null() -> Self {
        Self {
            hashkey: String::new(),
            player: String::new(),
            playerpath: String::new(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        }
    }

    // Java parity: uses config.playername directly; caller is responsible for ensuring it matches the resolved player profile
    pub fn new(config: &Config) -> Self {
        let player = config.playername.clone().unwrap_or_default();
        let playerpath = config.paths.playerpath.clone();

        let sep = std::path::MAIN_SEPARATOR;
        let player_dir = format!("{}{}{}", playerpath, sep, player);
        if let Err(e) = std::fs::create_dir_all(&player_dir) {
            log::error!("Failed to create player directory: {}", e);
        }
        let score_path = format!("{}{}{}{}{}", playerpath, sep, player, sep, "score.db");
        let scorelog_path = format!("{}{}{}{}{}", playerpath, sep, player, sep, "scorelog.db");
        let scoredatalog_path = format!(
            "{}{}{}{}{}",
            playerpath, sep, player, sep, "scoredatalog.db"
        );

        let scoredb = match ScoreDatabaseAccessor::new(&score_path) {
            Ok(db) => {
                if let Err(e) = db.create_table() {
                    log::error!("Failed to create score table: {e}");
                }
                Some(db)
            }
            Err(e) => {
                log::error!("Failed to open score database: {}", e);
                None
            }
        };

        let scorelogdb = match ScoreLogDatabaseAccessor::new(&scorelog_path) {
            Ok(db) => Some(db),
            Err(e) => {
                log::error!("Failed to open score log database: {}", e);
                None
            }
        };

        let scoredatalogdb = match ScoreDataLogDatabaseAccessor::new(&scoredatalog_path) {
            Ok(db) => Some(db),
            Err(e) => {
                log::error!("Failed to open score data log database: {}", e);
                None
            }
        };

        Self {
            hashkey: String::new(),
            player,
            playerpath,
            scoredb,
            scorelogdb,
            scoredatalogdb,
        }
    }

    pub fn read_player_data(&self) -> Option<PlayerData> {
        self.scoredb.as_ref()?.player_data()
    }

    pub fn read_today_player_data(&self) -> Option<PlayerData> {
        let scoredb = self.scoredb.as_ref()?;
        let mut pd = scoredb.player_datas(2);
        if pd.len() > 1 {
            pd[0].playcount = (pd[0].playcount - pd[1].playcount).max(0);
            pd[0].clear = (pd[0].clear - pd[1].clear).max(0);
            pd[0].epg = (pd[0].epg - pd[1].epg).max(0);
            pd[0].lpg = (pd[0].lpg - pd[1].lpg).max(0);
            pd[0].egr = (pd[0].egr - pd[1].egr).max(0);
            pd[0].lgr = (pd[0].lgr - pd[1].lgr).max(0);
            pd[0].egd = (pd[0].egd - pd[1].egd).max(0);
            pd[0].lgd = (pd[0].lgd - pd[1].lgd).max(0);
            pd[0].ebd = (pd[0].ebd - pd[1].ebd).max(0);
            pd[0].lbd = (pd[0].lbd - pd[1].lbd).max(0);
            pd[0].epr = (pd[0].epr - pd[1].epr).max(0);
            pd[0].lpr = (pd[0].lpr - pd[1].lpr).max(0);
            pd[0].ems = (pd[0].ems - pd[1].ems).max(0);
            pd[0].lms = (pd[0].lms - pd[1].lms).max(0);
            pd[0].playtime = (pd[0].playtime - pd[1].playtime).max(0);
            Some(pd.remove(0))
        } else if pd.len() == 1 {
            Some(pd.remove(0))
        } else {
            None
        }
    }

    pub fn update_player_data(&self, score: &ScoreData, time: i64) {
        let scoredb = match &self.scoredb {
            Some(db) => db,
            None => return,
        };
        let mut pd = match scoredb.player_data() {
            Some(p) => p,
            None => return,
        };
        pd.epg += score.judge_counts.epg as i64;
        pd.lpg += score.judge_counts.lpg as i64;
        pd.egr += score.judge_counts.egr as i64;
        pd.lgr += score.judge_counts.lgr as i64;
        pd.egd += score.judge_counts.egd as i64;
        pd.lgd += score.judge_counts.lgd as i64;
        pd.ebd += score.judge_counts.ebd as i64;
        pd.lbd += score.judge_counts.lbd as i64;
        pd.epr += score.judge_counts.epr as i64;
        pd.lpr += score.judge_counts.lpr as i64;
        pd.ems += score.judge_counts.ems as i64;
        pd.lms += score.judge_counts.lms as i64;

        pd.playcount += 1;
        if score.clear > ClearType::Failed.id() {
            pd.clear += 1;
        }
        pd.playtime += time;
        scoredb.set_player_data(&pd);
    }

    pub fn read_score_data_by_hash(&self, hash: &str, ln: bool, lnmode: i32) -> Option<ScoreData> {
        let scoredb = self.scoredb.as_ref()?;
        scoredb.score_data(hash, if ln { lnmode } else { 0 })
    }

    pub fn read_score_datas(
        &self,
        collector: &mut dyn ScoreDataCollector,
        songs: &[SongData],
        lnmode: i32,
    ) {
        if let Some(scoredb) = &self.scoredb {
            scoredb.score_datas_for_songs(collector, songs, lnmode);
        }
    }

    pub fn read_score_datas_sql(&self, sql: &str) -> Option<Vec<ScoreData>> {
        self.scoredb.as_ref()?.score_datas(sql)
    }

    pub fn write_score_data(&self, newscore: &ScoreData, ctx: &ScoreWriteContext<'_>) {
        let scoredb = match &self.scoredb {
            Some(db) => db,
            None => return,
        };

        let hash = ctx.hash;
        let contains_undefined_ln = ctx.contains_undefined_ln;
        let total_notes = ctx.total_notes;
        let lnmode = ctx.lnmode;
        let update_score = ctx.update_score;
        let last_note_time_us = ctx.last_note_time_us;

        let mut score = scoredb
            .score_data(hash, if contains_undefined_ln { lnmode } else { 0 })
            .unwrap_or_else(|| ScoreData {
                mode: if contains_undefined_ln { lnmode } else { 0 },
                ..Default::default()
            });
        score.sha256 = hash.to_string();
        if update_score {
            score.notes = total_notes;
        }

        if newscore.clear > ClearType::Failed.id() {
            score.clearcount += 1;
        }

        let log = self.update_score(&mut score, newscore, hash, update_score);

        // Trophy handling
        let mut trophies: std::collections::BTreeSet<SongTrophy> =
            std::collections::BTreeSet::new();
        for c in score.trophy.chars() {
            if let Some(t) = SongTrophy::trophy(c) {
                trophies.insert(t);
            }
        }

        let mut new_trophies: std::collections::BTreeSet<SongTrophy> =
            std::collections::BTreeSet::new();
        // Clear trophies
        let clear = newscore.clear;
        if newscore.play_option.gauge != -1 {
            if clear >= ClearType::Hard.id() {
                if clear == ClearType::ExHard.id() {
                    new_trophies.insert(SongTrophy::ExHard);
                }
                new_trophies.insert(SongTrophy::Hard);
            } else {
                if clear >= ClearType::Normal.id() {
                    new_trophies.insert(SongTrophy::Groove);
                }
                new_trophies.insert(SongTrophy::Easy);
            }
        }

        // Option trophies
        let option_trophy: &[SongTrophy] = &[
            SongTrophy::Normal,
            SongTrophy::Mirror,
            SongTrophy::Random,
            SongTrophy::RRandom,
            SongTrophy::SRandom,
            SongTrophy::Spiral,
            SongTrophy::HRandom,
            SongTrophy::AllScr,
            SongTrophy::ExRandom,
            SongTrophy::ExSRandom,
        ];

        if clear >= ClearType::Easy.id() {
            let idx = std::cmp::max(
                newscore.play_option.option % 10,
                (newscore.play_option.option / 10) % 10,
            );
            if idx >= 0 && (idx as usize) < option_trophy.len() {
                let idx = idx as usize;
                new_trophies.insert(option_trophy[idx]);
            }
        }

        trophies.extend(&new_trophies);

        let trophy_str: String = trophies.iter().map(|t| t.character()).collect();
        score.trophy = trophy_str;

        score.playcount += 1;
        score.date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        score.scorehash = self.get_score_hash(&score).unwrap_or_default();
        scoredb.set_score_data(&score);

        if log.sha256.is_some()
            && let Some(ref scorelogdb) = self.scorelogdb
        {
            let mut log = log;
            log.mode = score.mode;
            log.date = score.date;
            scorelogdb.set_score_log(&log);
        }

        if let Some(ref scoredatalogdb) = self.scoredatalogdb {
            let new_trophy_str: String = new_trophies.iter().map(|t| t.character()).collect();
            let mut newscore_copy = newscore.clone();
            newscore_copy.trophy = new_trophy_str;
            newscore_copy.mode = score.mode;
            newscore_copy.date = score.date;
            newscore_copy.playcount = score.playcount;
            newscore_copy.clearcount = score.clearcount;
            newscore_copy.scorehash = self.get_score_hash(&newscore_copy).unwrap_or_default();
            scoredatalogdb.set_score_data_log(&newscore_copy);
        }

        // Play time calculation (seconds)
        let time = last_note_time_us / 1000000;
        self.update_player_data(newscore, time);
        log::info!("Score database update completed");
    }

    pub fn write_score_data_for_course(
        &self,
        newscore: &ScoreData,
        ctx: &CourseScoreWriteContext<'_>,
    ) {
        let scoredb = match &self.scoredb {
            Some(db) => db,
            None => return,
        };

        let hashes = ctx.hashes;
        let total_notes = ctx.total_notes;
        let ln = ctx.ln;
        let lnmode = ctx.lnmode;
        let option = ctx.option;
        let constraint = ctx.constraint;
        let update_score = ctx.update_score;

        let hash: String = hashes.join("");
        let (hispeed, judge, gauge) = Self::compute_constraint_values(constraint);
        let mode_val = (if ln { lnmode } else { 0 })
            + option * 10
            + hispeed * 100
            + judge * 1000
            + gauge * 10000;

        let mut score = scoredb
            .score_data(&hash, mode_val)
            .unwrap_or_else(|| ScoreData {
                mode: mode_val,
                ..Default::default()
            });
        score.sha256 = hash.clone();
        score.notes = total_notes;

        if newscore.clear != ClearType::Failed.id() {
            score.clearcount += 1;
        }

        let log = self.update_score(&mut score, newscore, &hash, update_score);

        score.playcount += 1;
        score.date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        score.scorehash = self.get_score_hash(&score).unwrap_or_default();
        scoredb.set_score_data(&score);

        if log.sha256.is_some()
            && let Some(ref scorelogdb) = self.scorelogdb
        {
            let mut log = log;
            log.mode = score.mode;
            log.date = score.date;
            scorelogdb.set_score_log(&log);
        }

        log::info!("Score database update completed");
    }

    pub(super) fn compute_constraint_values(
        constraint: &[CourseDataConstraint],
    ) -> (i32, i32, i32) {
        let mut hispeed = 0;
        let mut judge = 0;
        let mut gauge = 0;
        for c in constraint {
            match c {
                CourseDataConstraint::NoSpeed => hispeed = 1,
                CourseDataConstraint::NoGood => judge = 1,
                CourseDataConstraint::NoGreat => judge = 2,
                CourseDataConstraint::GaugeLr2 => gauge = 1,
                CourseDataConstraint::Gauge5Keys => gauge = 2,
                CourseDataConstraint::Gauge7Keys => gauge = 3,
                CourseDataConstraint::Gauge9Keys => gauge = 4,
                CourseDataConstraint::Gauge24Keys => gauge = 5,
                _ => {}
            }
        }
        (hispeed, judge, gauge)
    }

    pub(super) fn get_score_hash(&self, score: &ScoreData) -> Option<String> {
        let input = format!(
            "{}{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.hashkey,
            score.sha256,
            score.exscore(),
            score.judge_counts.epg,
            score.judge_counts.lpg,
            score.judge_counts.egr,
            score.judge_counts.lgr,
            score.judge_counts.egd,
            score.judge_counts.lgd,
            score.judge_counts.ebd,
            score.judge_counts.lbd,
            score.judge_counts.epr,
            score.judge_counts.lpr,
            score.judge_counts.ems,
            score.judge_counts.lms,
            score.clear,
            score.minbp,
            score.maxcombo,
            score.mode,
            score.clearcount,
            score.playcount,
            score.play_option.option,
            score.play_option.random,
            score.trophy,
            score.date
        );

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
        Some(format!("035{}", hex))
    }

    pub(super) fn update_score(
        &self,
        score: &mut ScoreData,
        newscore: &ScoreData,
        hash: &str,
        update_score: bool,
    ) -> ScoreLog {
        let mut log = ScoreLog::new();

        log.oldclear = score.clear;
        log.clear = score.clear;
        if score.clear < newscore.clear {
            log.sha256 = Some(hash.to_string());
            log.clear = newscore.clear;
        }
        log.oldscore = score.exscore();
        log.score = score.exscore();
        if score.exscore() < newscore.exscore() && update_score {
            log.sha256 = Some(hash.to_string());
            log.score = newscore.exscore();
        }
        log.oldminbp = score.minbp;
        log.minbp = score.minbp;
        if score.minbp > newscore.minbp && update_score {
            log.sha256 = Some(hash.to_string());
            log.minbp = newscore.minbp;
        }
        log.oldcombo = score.maxcombo;
        log.combo = score.maxcombo;
        if score.maxcombo < newscore.maxcombo && update_score {
            log.sha256 = Some(hash.to_string());
            log.combo = newscore.maxcombo;
        }

        score.update(newscore, update_score);

        log
    }

    pub fn delete_score_data(&self, sha256: &str, contains_undefined_ln: bool, lnmode: i32) {
        if let Some(scoredb) = &self.scoredb {
            scoredb.delete_score_data(sha256, if contains_undefined_ln { lnmode } else { 0 });
        }
    }

    pub fn exists_replay_data(&self, hash: &str, ln: bool, lnmode: i32, index: i32) -> bool {
        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path(hash, ln, lnmode, index)
        );
        Path::new(&path).exists()
    }

    pub fn read_replay_data(
        &self,
        hash: &str,
        ln: bool,
        lnmode: i32,
        index: i32,
    ) -> Option<ReplayData> {
        if !self.exists_replay_data(hash, ln, lnmode, index) {
            return None;
        }
        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path(hash, ln, lnmode, index)
        );
        match ReplayData::read_brd(Path::new(&path)) {
            Ok(rd) => Some(rd),
            Err(e) => {
                log::error!("Failed to read replay data: {}", e);
                None
            }
        }
    }

    pub fn write_replay_data(
        &self,
        rd: &mut ReplayData,
        hash: &str,
        ln: bool,
        lnmode: i32,
        index: i32,
    ) -> anyhow::Result<()> {
        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path(hash, ln, lnmode, index)
        );
        rd.write_brd(Path::new(&path))
    }

    pub fn delete_replay_data(&self, hash: &str, ln: bool, lnmode: i32, index: i32) {
        if self.exists_replay_data(hash, ln, lnmode, index) {
            let path = format!(
                "{}.brd",
                self.get_replay_data_file_path(hash, ln, lnmode, index)
            );
            let _ = fs::remove_file(&path);
        }
    }

    pub(super) fn get_replay_data_file_path(
        &self,
        hash: &str,
        ln: bool,
        lnmode: i32,
        index: i32,
    ) -> String {
        let sep = std::path::MAIN_SEPARATOR;
        let prefix = if ln {
            REPLAY.get(lnmode as usize).copied().unwrap_or("")
        } else {
            ""
        };
        let suffix = if index > 0 {
            format!("_{}", index)
        } else {
            String::new()
        };
        format!(
            "{}{}{}{}{}",
            self.get_replay_data_folder(),
            sep,
            prefix,
            hash,
            suffix
        )
    }

    pub(super) fn get_replay_data_folder(&self) -> String {
        let sep = std::path::MAIN_SEPARATOR;
        format!("{}{}{}{}replay", self.playerpath, sep, self.player, sep)
    }

    pub fn scoredb(&self) -> Option<&ScoreDatabaseAccessor> {
        self.scoredb.as_ref()
    }
}
