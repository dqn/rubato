use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};

use crate::clear_type::ClearType;
use crate::config::Config;
use crate::course_data::CourseData;
use crate::player_data::PlayerData;
use crate::replay_data::ReplayData;
use crate::score_data::{ScoreData, SongTrophy};
use crate::score_data_log_database_accessor::ScoreDataLogDatabaseAccessor;
use crate::score_database_accessor::{ScoreDataCollector, ScoreDatabaseAccessor, SongData};
use crate::score_log_database_accessor::{ScoreLog, ScoreLogDatabaseAccessor};
use crate::validatable::Validatable;

static REPLAY: &[&str] = &["", "C", "H"];

/// Play data accessor.
/// Translated from Java: PlayDataAccessor
pub struct PlayDataAccessor {
    hashkey: String,
    player: String,
    playerpath: String,
    scoredb: Option<ScoreDatabaseAccessor>,
    scorelogdb: Option<ScoreLogDatabaseAccessor>,
    scoredatalogdb: Option<ScoreDataLogDatabaseAccessor>,
}

impl PlayDataAccessor {
    pub fn new(config: &Config) -> Self {
        let player = config.playername.clone().unwrap_or_default();
        let playerpath = config.playerpath.clone();

        let sep = std::path::MAIN_SEPARATOR;
        let score_path = format!("{}{}{}{}{}", playerpath, sep, player, sep, "score.db");
        let scorelog_path = format!("{}{}{}{}{}", playerpath, sep, player, sep, "scorelog.db");
        let scoredatalog_path = format!(
            "{}{}{}{}{}",
            playerpath, sep, player, sep, "scoredatalog.db"
        );

        let scoredb = match ScoreDatabaseAccessor::new(&score_path) {
            Ok(db) => {
                db.create_table();
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
        self.scoredb.as_ref()?.get_player_data()
    }

    pub fn read_today_player_data(&self) -> Option<PlayerData> {
        let scoredb = self.scoredb.as_ref()?;
        let mut pd = scoredb.get_player_datas(2);
        if pd.len() > 1 {
            pd[0].playcount -= pd[1].playcount;
            pd[0].clear -= pd[1].clear;
            pd[0].epg -= pd[1].epg;
            pd[0].lpg -= pd[1].lpg;
            pd[0].egr -= pd[1].egr;
            pd[0].lgr -= pd[1].lgr;
            pd[0].egd -= pd[1].egd;
            pd[0].lgd -= pd[1].lgd;
            pd[0].ebd -= pd[1].ebd;
            pd[0].lbd -= pd[1].lbd;
            pd[0].epr -= pd[1].epr;
            pd[0].lpr -= pd[1].lpr;
            pd[0].ems -= pd[1].ems;
            pd[0].lms -= pd[1].lms;
            pd[0].playtime -= pd[1].playtime;
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
        let mut pd = match scoredb.get_player_data() {
            Some(p) => p,
            None => return,
        };
        pd.epg += score.epg as i64;
        pd.lpg += score.lpg as i64;
        pd.egr += score.egr as i64;
        pd.lgr += score.lgr as i64;
        pd.egd += score.egd as i64;
        pd.lgd += score.lgd as i64;
        pd.ebd += score.ebd as i64;
        pd.lbd += score.lbd as i64;
        pd.epr += score.epr as i64;
        pd.lpr += score.lpr as i64;
        pd.ems += score.ems as i64;
        pd.lms += score.lms as i64;

        pd.playcount += 1;
        if score.clear > ClearType::Failed.id() {
            pd.clear += 1;
        }
        pd.playtime += time;
        scoredb.set_player_data(&pd);
    }

    pub fn read_score_data_by_hash(&self, hash: &str, ln: bool, lnmode: i32) -> Option<ScoreData> {
        let scoredb = self.scoredb.as_ref()?;
        scoredb.get_score_data(hash, if ln { lnmode } else { 0 })
    }

    pub fn read_score_datas(
        &self,
        collector: &mut dyn ScoreDataCollector,
        songs: &[SongData],
        lnmode: i32,
    ) {
        if let Some(scoredb) = &self.scoredb {
            scoredb.get_score_datas_for_songs(collector, songs, lnmode);
        }
    }

    pub fn read_score_datas_sql(&self, sql: &str) -> Option<Vec<ScoreData>> {
        self.scoredb.as_ref()?.get_score_datas(sql)
    }

    #[allow(clippy::too_many_arguments, clippy::field_reassign_with_default)]
    pub fn write_score_data(
        &self,
        newscore: &ScoreData,
        hash: &str,
        contains_undefined_ln: bool,
        total_notes: i32,
        lnmode: i32,
        update_score: bool,
        last_note_time_us: i64,
    ) {
        let scoredb = match &self.scoredb {
            Some(db) => db,
            None => return,
        };

        let mut score = scoredb
            .get_score_data(hash, if contains_undefined_ln { lnmode } else { 0 })
            .unwrap_or_else(|| {
                let mut s = ScoreData::default();
                s.mode = if contains_undefined_ln { lnmode } else { 0 };
                s
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
        let mut trophies: std::collections::HashSet<SongTrophy> = std::collections::HashSet::new();
        for c in score.trophy.chars() {
            if let Some(t) = SongTrophy::get_trophy(c) {
                trophies.insert(t);
            }
        }

        let mut new_trophies: std::collections::HashSet<SongTrophy> =
            std::collections::HashSet::new();
        // Clear trophies
        let clear = newscore.clear;
        if newscore.gauge != -1 {
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
            let idx = std::cmp::max(newscore.option % 10, (newscore.option / 10) % 10) as usize;
            if idx < option_trophy.len() {
                new_trophies.insert(option_trophy[idx]);
            }
        }

        trophies.extend(&new_trophies);

        let trophy_str: String = trophies.iter().map(|t| t.character()).collect();
        score.trophy = trophy_str;

        score.playcount += 1;
        score.date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
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

    #[allow(clippy::too_many_arguments, clippy::field_reassign_with_default)]
    pub fn write_score_data_for_course(
        &self,
        newscore: &ScoreData,
        hashes: &[&str],
        total_notes: i32,
        ln: bool,
        lnmode: i32,
        option: i32,
        constraint: &[CourseData],
        update_score: bool,
    ) {
        let scoredb = match &self.scoredb {
            Some(db) => db,
            None => return,
        };

        let hash: String = hashes.join("");
        let (hispeed, judge, gauge) = Self::compute_constraint_values(constraint);
        let mode_val = (if ln { lnmode } else { 0 })
            + option * 10
            + hispeed * 100
            + judge * 1000
            + gauge * 10000;

        let mut score = scoredb.get_score_data(&hash, mode_val).unwrap_or_else(|| {
            let mut s = ScoreData::default();
            s.mode = mode_val;
            s
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
            .unwrap()
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

    fn compute_constraint_values(_constraint: &[CourseData]) -> (i32, i32, i32) {
        // TODO: implement constraint parsing when CourseData constraints are fully available
        (0, 0, 0)
    }

    fn get_score_hash(&self, score: &ScoreData) -> Option<String> {
        let input = format!(
            "{}{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.hashkey,
            score.sha256,
            score.get_exscore(),
            score.epg,
            score.lpg,
            score.egr,
            score.lgr,
            score.egd,
            score.lgd,
            score.ebd,
            score.lbd,
            score.epr,
            score.lpr,
            score.ems,
            score.lms,
            score.clear,
            score.minbp,
            score.combo,
            score.mode,
            score.clearcount,
            score.playcount,
            score.option,
            score.random,
            score.trophy,
            score.date
        );

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
        Some(format!("035{}", hex))
    }

    fn update_score(
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
        log.oldscore = score.get_exscore();
        log.score = score.get_exscore();
        if score.get_exscore() < newscore.get_exscore() && update_score {
            log.sha256 = Some(hash.to_string());
            log.score = newscore.get_exscore();
        }
        log.oldminbp = score.minbp;
        log.minbp = score.minbp;
        if score.minbp > newscore.minbp && update_score {
            log.sha256 = Some(hash.to_string());
            log.minbp = newscore.minbp;
        }
        log.oldcombo = score.combo;
        log.combo = score.combo;
        if score.combo < newscore.combo && update_score {
            log.sha256 = Some(hash.to_string());
            log.combo = newscore.combo;
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
        match fs::File::open(&path) {
            Ok(file) => {
                let reader = BufReader::new(GzDecoder::new(file));
                match serde_json::from_reader::<_, ReplayData>(reader) {
                    Ok(mut rd) => {
                        if rd.validate() {
                            Some(rd)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read replay data: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to open replay file: {}", e);
                None
            }
        }
    }

    pub fn write_replay_data(
        &self,
        rd: &ReplayData,
        hash: &str,
        ln: bool,
        lnmode: i32,
        index: i32,
    ) {
        let replay_dir = self.get_replay_data_folder();
        if let Err(e) = fs::create_dir_all(&replay_dir) {
            log::error!("Failed to create replay directory: {}", e);
            return;
        }

        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path(hash, ln, lnmode, index)
        );
        match fs::File::create(&path) {
            Ok(file) => {
                let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
                if let Err(e) = serde_json::to_writer_pretty(encoder, rd) {
                    log::error!("Failed to write replay data: {}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to create replay file: {}", e);
            }
        }
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

    fn get_replay_data_file_path(&self, hash: &str, ln: bool, lnmode: i32, index: i32) -> String {
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

    fn get_replay_data_folder(&self) -> String {
        let sep = std::path::MAIN_SEPARATOR;
        format!("{}{}{}{}replay", self.playerpath, sep, self.player, sep)
    }

    pub fn get_scoredb(&self) -> Option<&ScoreDatabaseAccessor> {
        self.scoredb.as_ref()
    }
}
