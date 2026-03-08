use std::path::Path;

use rubato_types::player_information::PlayerInformation;
use rubato_types::score_data_cache::ScoreDataCache;

use crate::main_controller::MainController;
use crate::score_data_importer::ScoreDataImporter;
use crate::score_database_accessor::ScoreDatabaseAccessor;

/// Rival data accessor.
/// Translated from Java: RivalDataAccessor
#[derive(Default)]
pub struct RivalDataAccessor {
    rivals: Vec<PlayerInformation>,
    rivalcaches: Vec<ScoreDataCache>,
}

impl RivalDataAccessor {
    pub fn new() -> Self {
        Self {
            rivals: Vec::new(),
            rivalcaches: Vec::new(),
        }
    }

    pub fn rival_information(&self, index: usize) -> Option<&PlayerInformation> {
        self.rivals.get(index)
    }

    /// Get rival score data cache by index.
    ///
    /// Translated from: RivalDataAccessor.getRivalScoreDataCache(int)
    pub fn rival_score_data_cache(&self, index: usize) -> Option<&ScoreDataCache> {
        self.rivalcaches.get(index)
    }

    pub fn rival_score_data_cache_mut(&mut self, index: usize) -> Option<&mut ScoreDataCache> {
        self.rivalcaches.get_mut(index)
    }

    pub fn rival_count(&self) -> usize {
        self.rivals.len()
    }

    /// Update rival data from IR.
    /// Translates: RivalDataAccessor.update(MainController)
    pub fn update(&mut self, main: &mut MainController) {
        let ir_status = main.ir_status();
        if ir_status.is_empty() {
            return;
        }

        let provider = match ir_status[0].rival_provider.as_ref() {
            Some(p) => p,
            None => {
                log::debug!("No IR rival provider configured");
                return;
            }
        };

        // Step 1: Import own scores if configured
        if provider.should_import_scores() {
            let config = main.config();
            let player_name = config.playername().unwrap_or("player1");
            let score_db_path = format!("{}/{}/score.db", config.paths.playerpath, player_name);
            match provider.fetch_own_scores() {
                Ok(scores) => {
                    if let Ok(scoredb) = ScoreDatabaseAccessor::new(&score_db_path) {
                        let importer = ScoreDataImporter::new(&scoredb);
                        let score_hash = provider.score_hash();
                        importer.import_scores(&scores, &score_hash);
                        log::info!("IR score import complete");
                    } else {
                        log::warn!("Failed to open score database: {}", score_db_path);
                    }
                }
                Err(e) => {
                    log::warn!("IR score import failed: {}", e);
                }
            }
            // Clear import flag via mutable access
            if let Some(p) = main
                .ir_status_mut()
                .get_mut(0)
                .and_then(|s| s.rival_provider.as_mut())
            {
                p.clear_import_flag();
            }
        }

        // Re-borrow provider after mutable access
        let provider = match main
            .ir_status()
            .first()
            .and_then(|s| s.rival_provider.as_ref())
        {
            Some(p) => p,
            None => return,
        };

        // Step 2: Fetch rivals from IR
        let ir_name = provider.ir_name();
        let should_import_rivals = provider.should_import_rivals();

        let mut rivals = Vec::new();
        let mut rivalcaches = Vec::new();

        if should_import_rivals {
            match provider.fetch_rival_list() {
                Ok(rival_list) => {
                    // Create rival/ directory if needed
                    let rival_dir = Path::new("rival");
                    if !rival_dir.exists()
                        && let Err(e) = std::fs::create_dir_all(rival_dir)
                    {
                        log::warn!("Failed to create rival directory: {}", e);
                    }

                    for rival_info in &rival_list {
                        let info = rival_info.to_player_information();
                        let db_path = format!("rival/{}{}.db", ir_name, rival_info.id);

                        // Fetch rival scores in background thread
                        let rival_info_clone = rival_info.clone();
                        let db_path_clone = db_path.clone();
                        let info_clone = info.clone();

                        // Re-borrow provider for fetch
                        let provider_ref = main
                            .ir_status()
                            .first()
                            .and_then(|s| s.rival_provider.as_ref());
                        if let Some(prov) = provider_ref {
                            match prov.fetch_rival_scores(&rival_info_clone) {
                                Ok(scores) => {
                                    if let Ok(scoredb) = ScoreDatabaseAccessor::new(&db_path_clone)
                                    {
                                        scoredb.create_table();
                                        scoredb.set_information(&info_clone);
                                        let refs: Vec<&rubato_types::score_data::ScoreData> =
                                            scores.iter().collect();
                                        scoredb.set_score_data_batch(&refs);
                                        log::info!(
                                            "Rival score fetch complete: {}",
                                            info_clone.name()
                                        );
                                    }
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Rival score fetch failed for {}: {}",
                                        info_clone.name(),
                                        e
                                    );
                                }
                            }
                        }

                        // Create cache backed by the rival score database
                        let cache = Self::create_score_cache_for_db(&db_path);
                        rivals.push(info);
                        rivalcaches.push(cache);
                    }
                }
                Err(e) => {
                    log::warn!("IR rival list fetch failed: {}", e);
                }
            }
        }

        // Step 3: Scan rival/ directory for existing .db files not in IR list
        let rival_dir = Path::new("rival");
        if rival_dir.exists()
            && let Ok(entries) = std::fs::read_dir(rival_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "db") {
                    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

                    // Skip if already loaded from IR
                    let already_loaded = rivals.iter().any(|info| {
                        let expected =
                            format!("{}{}.db", ir_name, info.id.as_deref().unwrap_or(""));
                        file_name == expected
                    });
                    if already_loaded {
                        continue;
                    }

                    let path_str = path.to_string_lossy().to_string();
                    if let Ok(scoredb) = ScoreDatabaseAccessor::new(&path_str)
                        && let Some(info) = scoredb.information()
                    {
                        let cache = Self::create_score_cache_for_db(&path_str);
                        log::info!("Local rival score loaded: {}", info.name());
                        rivals.push(info);
                        rivalcaches.push(cache);
                    }
                }
            }
        }

        self.rivals = rivals;
        self.rivalcaches = rivalcaches;
    }

    /// Create a ScoreDataCache backed by a score database file.
    fn create_score_cache_for_db(db_path: &str) -> ScoreDataCache {
        let db_path_single = db_path.to_string();
        let db_path_multi = db_path.to_string();

        ScoreDataCache::new(
            Box::new(move |song, lnmode| {
                let sha256 = &song.file.sha256;
                let mode = if song.has_undefined_long_note() {
                    lnmode
                } else {
                    0
                };
                ScoreDatabaseAccessor::new(&db_path_single)
                    .ok()
                    .and_then(|db| db.score_data(sha256, mode))
            }),
            Box::new(move |collector, songs, lnmode| {
                if let Ok(db) = ScoreDatabaseAccessor::new(&db_path_multi) {
                    for song in songs {
                        let sha256 = &song.file.sha256;
                        let mode = if song.has_undefined_long_note() {
                            lnmode
                        } else {
                            0
                        };
                        let score = db.score_data(sha256, mode);
                        collector(song, score.as_ref());
                    }
                }
            }),
        )
    }
}
