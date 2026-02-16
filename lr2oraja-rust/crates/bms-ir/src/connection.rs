use anyhow::Result;
use async_trait::async_trait;

use crate::account::IRAccount;
use crate::chart_data::IRChartData;
use crate::course_data::IRCourseData;
use crate::player_data::IRPlayerData;
use crate::response::IRResponse;
use crate::score_data::IRScoreData;
use crate::table_data::IRTableData;

/// IR connection trait.
///
/// Corresponds to Java `IRConnection` interface.
/// Default implementations return "not supported" errors.
#[async_trait]
pub trait IRConnection: Send + Sync {
    /// Register a new user.
    async fn register(&self, _account: &IRAccount) -> Result<IRResponse<IRPlayerData>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Login to the IR.
    async fn login(&self, _account: &IRAccount) -> Result<IRResponse<IRPlayerData>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Get rival data.
    async fn get_rivals(&self) -> Result<IRResponse<Vec<IRPlayerData>>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Get table data.
    async fn get_table_datas(&self) -> Result<IRResponse<Vec<IRTableData>>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Get play data (scores) for a chart.
    async fn get_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        _chart: &IRChartData,
    ) -> Result<IRResponse<Vec<IRScoreData>>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Get course play data.
    async fn get_course_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        _course: &IRCourseData,
    ) -> Result<IRResponse<Vec<IRScoreData>>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Send play data (score submission).
    async fn send_play_data(
        &self,
        _chart: &IRChartData,
        _score: &IRScoreData,
    ) -> Result<IRResponse<()>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Send course play data.
    async fn send_course_play_data(
        &self,
        _course: &IRCourseData,
        _score: &IRScoreData,
    ) -> Result<IRResponse<()>> {
        Ok(IRResponse::failure("not supported"))
    }

    /// Get song URL.
    async fn get_song_url(&self, _chart: &IRChartData) -> Option<String> {
        None
    }

    /// Get course URL.
    async fn get_course_url(&self, _course: &IRCourseData) -> Option<String> {
        None
    }

    /// Get player URL.
    async fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
        None
    }
}
