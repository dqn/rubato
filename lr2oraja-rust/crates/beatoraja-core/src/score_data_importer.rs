use beatoraja_types::score_data::ScoreData;

use crate::score_database_accessor::ScoreDatabaseAccessor;

/// Score data importer
/// Translates: bms.player.beatoraja.external.ScoreDataImporter
pub struct ScoreDataImporter<'a> {
    scoredb: &'a ScoreDatabaseAccessor,
}

impl<'a> ScoreDataImporter<'a> {
    pub fn new(scoredb: &'a ScoreDatabaseAccessor) -> Self {
        Self { scoredb }
    }

    /// Import scores from IR, merging with existing local scores.
    /// Updates existing records if new score is better, inserts if new.
    /// Translates: ScoreDataImporter.importScores(ScoreData[], String)
    pub fn import_scores(&self, scores: &[ScoreData], scorehash: &str) {
        let mut result: Vec<ScoreData> = Vec::new();

        for score in scores {
            let sha256 = score.get_sha256();
            let mode = score.get_mode();
            let mut oldsd = match self.scoredb.get_score_data(sha256, mode) {
                Some(existing) => existing,
                None => ScoreData {
                    playcount: score.playcount,
                    clearcount: score.clearcount,
                    sha256: sha256.to_string(),
                    mode,
                    notes: score.notes,
                    ..Default::default()
                },
            };
            oldsd.scorehash = scorehash.to_string();
            if oldsd.update(score, true) {
                result.push(oldsd);
            }
        }

        if !result.is_empty() {
            let refs: Vec<&ScoreData> = result.iter().collect();
            self.scoredb.set_score_data_batch(&refs);
        }
        log::info!("Score import complete - imported: {}", result.len());
    }
}
