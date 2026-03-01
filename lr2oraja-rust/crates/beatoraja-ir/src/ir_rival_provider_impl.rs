use std::sync::Arc;

use anyhow::bail;
use beatoraja_types::ir_rival_provider::{IRRivalProvider, RivalInfo};
use beatoraja_types::score_data::ScoreData;

use crate::ir_connection::IRConnection;
use crate::ir_player_data::IRPlayerData;

/// IRRivalProvider implementation that wraps an IRConnection.
/// Used by beatoraja-core's RivalDataAccessor to fetch rival data without circular deps.
pub struct IRRivalProviderImpl {
    connection: Arc<dyn IRConnection + Send + Sync>,
    player: IRPlayerData,
    ir_name: String,
    import_scores: bool,
    import_rivals: bool,
}

impl IRRivalProviderImpl {
    pub fn new(
        connection: Arc<dyn IRConnection + Send + Sync>,
        player: IRPlayerData,
        ir_name: String,
        import_scores: bool,
        import_rivals: bool,
    ) -> Self {
        Self {
            connection,
            player,
            ir_name,
            import_scores,
            import_rivals,
        }
    }

    fn convert_scores(ir_scores: &[crate::ir_score_data::IRScoreData]) -> Vec<ScoreData> {
        ir_scores
            .iter()
            .map(|s| s.convert_to_score_data())
            .collect()
    }
}

impl IRRivalProvider for IRRivalProviderImpl {
    fn should_import_scores(&self) -> bool {
        self.import_scores
    }

    fn clear_import_flag(&mut self) {
        self.import_scores = false;
    }

    fn fetch_own_scores(&self) -> anyhow::Result<Vec<ScoreData>> {
        // Java: connection.getPlayData(player, null) — null chart = all scores
        // Since IRConnection.get_play_data requires &IRChartData, we use a default empty chart.
        let empty_chart = crate::ir_chart_data::IRChartData::default();
        let response = self
            .connection
            .get_play_data(Some(&self.player), &empty_chart);
        if response.is_succeeded() {
            match response.data {
                Some(scores) => Ok(Self::convert_scores(&scores)),
                None => Ok(Vec::new()),
            }
        } else {
            bail!("IR score fetch failed: {}", response.get_message())
        }
    }

    fn should_import_rivals(&self) -> bool {
        self.import_rivals
    }

    fn fetch_rival_list(&self) -> anyhow::Result<Vec<RivalInfo>> {
        let response = self.connection.get_rivals();
        if response.is_succeeded() {
            match response.data {
                Some(players) => Ok(players
                    .iter()
                    .map(|p| RivalInfo {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        rank: p.rank.clone(),
                    })
                    .collect()),
                None => Ok(Vec::new()),
            }
        } else {
            bail!("IR rival list fetch failed: {}", response.get_message())
        }
    }

    fn fetch_rival_scores(&self, rival: &RivalInfo) -> anyhow::Result<Vec<ScoreData>> {
        let player = IRPlayerData {
            id: rival.id.clone(),
            name: rival.name.clone(),
            rank: rival.rank.clone(),
        };
        let empty_chart = crate::ir_chart_data::IRChartData::default();
        let response = self.connection.get_play_data(Some(&player), &empty_chart);
        if response.is_succeeded() {
            match response.data {
                Some(scores) => Ok(Self::convert_scores(&scores)),
                None => Ok(Vec::new()),
            }
        } else {
            bail!(
                "IR rival score fetch failed for {}: {}",
                rival.name,
                response.get_message()
            )
        }
    }

    fn ir_name(&self) -> String {
        self.ir_name.clone()
    }

    fn score_hash(&self) -> String {
        self.ir_name.clone()
    }
}
