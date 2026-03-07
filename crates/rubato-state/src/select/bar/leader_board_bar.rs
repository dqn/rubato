use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::function_bar::{FunctionBar, STYLE_COURSE, STYLE_TABLE};
use crate::select::stubs::*;

/// Leaderboard display bar
/// Translates: bms.player.beatoraja.select.bar.LeaderBoardBar
#[derive(Clone)]
pub struct LeaderBoardBar {
    pub directory: DirectoryBarData,
    pub song_data: SongData,
    pub title: String,
    pub from_lr2ir: bool,
}

impl LeaderBoardBar {
    pub fn new(song_data: SongData, from_lr2ir: bool) -> Self {
        let title = song_data.full_title();
        Self {
            directory: DirectoryBarData::default(),
            song_data,
            title,
            from_lr2ir,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get children bars for leaderboard display.
    /// Returns empty when IR connection is unavailable.
    /// When IR data is provided via `children_with_ir`, creates FunctionBars.
    ///
    /// Translates: Java LeaderBoardBar.getChildren()
    pub fn children(&self) -> Vec<Bar> {
        // IR connection required - returns empty without it.
        // Use children_with_ir() when IR data is available.
        Vec::new()
    }

    /// Get children bars with pre-fetched IR leaderboard data.
    /// This is the functional path when IR connection is available.
    pub fn children_with_ir(&self, leaderboard: &[LeaderboardEntry]) -> Vec<Bar> {
        self.from_ir_score_data(leaderboard)
            .into_iter()
            .map(|fb| Bar::Function(Box::new(fb)))
            .collect()
    }

    /// Get children bars with local score inserted into leaderboard.
    pub fn children_with_ir_and_local(
        &self,
        local_score: &IRScoreData,
        leaderboard: &[LeaderboardEntry],
    ) -> Vec<Bar> {
        self.from_ir_score_data_with_local(local_score, leaderboard)
            .into_iter()
            .map(|fb| Bar::Function(Box::new(fb)))
            .collect()
    }

    /// Convert IR scores to function bars
    pub fn from_ir_score_data(&self, ir_score_data: &[LeaderboardEntry]) -> Vec<FunctionBar> {
        let mut bars = Vec::with_capacity(ir_score_data.len());
        for (i, entry) in ir_score_data.iter().enumerate() {
            bars.push(self.create_function_bar(
                (i + 1) as i32,
                entry,
                entry.ir_score().player.is_empty(),
            ));
        }
        bars
    }

    /// Convert IR scores to function bars, inserting local score
    pub fn from_ir_score_data_with_local(
        &self,
        local_score: &IRScoreData,
        leaderboard: &[LeaderboardEntry],
    ) -> Vec<FunctionBar> {
        let mut bars = Vec::with_capacity(leaderboard.len() + 1);
        let mut id = 0;
        let mut inserted = false;

        if leaderboard.is_empty() || local_score.exscore() > leaderboard[0].ir_score().exscore() {
            id += 1;
            bars.push(self.create_function_bar(
                id,
                &LeaderboardEntry::new_entry_primary_ir(local_score.clone()),
                true,
            ));
            inserted = true;
        }

        for (i, entry) in leaderboard.iter().enumerate() {
            let score = entry.ir_score();
            bars.push(self.create_function_bar(id + 1, entry, false));
            id += 1;

            if !inserted
                && score.exscore() > local_score.exscore()
                && (i == leaderboard.len() - 1
                    || leaderboard[i + 1].ir_score().exscore() <= local_score.exscore())
            {
                bars.push(self.create_function_bar(
                    id + 1,
                    &LeaderboardEntry::new_entry_primary_ir(local_score.clone()),
                    true,
                ));
                id += 1;
                inserted = true;
            }
        }

        if !inserted {
            bars.push(self.create_function_bar(
                id,
                &LeaderboardEntry::new_entry_primary_ir(local_score.clone()),
                true,
            ));
        }

        bars
    }

    fn create_function_bar(
        &self,
        rank: i32,
        entry: &LeaderboardEntry,
        is_self_score: bool,
    ) -> FunctionBar {
        let score_data = entry.ir_score();
        let title = if is_self_score {
            format!("{}. {}", rank, self.get_current_player_name())
        } else {
            format!("{}. {}", rank, score_data.player)
        };

        let display_type = if is_self_score {
            STYLE_COURSE
        } else {
            STYLE_TABLE
        };

        let mut bar = FunctionBar::new(title, display_type);
        bar.selectable.bar_data.score = Some(score_data.convert_to_score_data());
        bar.lamp = score_data.clear.id();
        // Function callback for ghost battle would go here
        // In Java: sets up LR2 ghost battle on click
        bar
    }

    fn get_current_player_name(&self) -> String {
        // In Java: StringPropertyFactory.getStringProperty("player").get(state)
        "Player".to_string()
    }
}
