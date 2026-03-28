use std::sync::Arc;

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::function_bar::{FunctionBar, FunctionBarCallback, STYLE_COURSE, STYLE_TABLE};
use crate::select::*;

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
        let title = song_data.metadata.full_title();
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

        for i in 0..leaderboard.len() {
            let entry = &leaderboard[i];
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
                id + 1,
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
        let rival_score = score_data.convert_to_score_data();
        bar.selectable.bar_data.score = Some(rival_score.clone());
        bar.lamp = score_data.clear.id();

        // Set up ghost/rival action: when the leaderboard entry is selected,
        // create a temporary song bar with the rival score set and start play.
        // This mirrors the Java LR2 ghost battle on click behavior.
        let song = self.song_data.clone();
        let callback: FunctionBarCallback = Arc::new(move |selector| {
            let mut bar = Bar::Song(Box::new(super::song_bar::SongBar::new(song.clone())));
            bar.set_rival_score(Some(rival_score.clone()));
            selector.read_chart(&song, &bar, Some(&BMSPlayerMode::PLAY));
        });
        bar.set_function(callback);
        bar
    }

    fn get_current_player_name(&self) -> String {
        // In Java: StringPropertyFactory.getStringProperty("player").get(state)
        "Player".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ir_score(exscore_pgreats: i32) -> IRScoreData {
        let mut sd = ScoreData::default();
        // exscore = (epg + lpg) * 2 + egr + lgr
        // Set epg so that exscore = exscore_pgreats * 2
        sd.judge_counts.epg = exscore_pgreats;
        IRScoreData::new(&sd)
    }

    fn make_ir_score_with_player(exscore_pgreats: i32, player: &str) -> IRScoreData {
        let mut score = make_ir_score(exscore_pgreats);
        score.player = player.to_string();
        score
    }

    fn make_entry(exscore_pgreats: i32, player: &str) -> LeaderboardEntry {
        LeaderboardEntry::new_entry_primary_ir(make_ir_score_with_player(exscore_pgreats, player))
    }

    fn make_bar() -> LeaderBoardBar {
        LeaderBoardBar::new(SongData::new(), false)
    }

    /// Extract the rank number from a FunctionBar title like "3. Player"
    fn extract_rank(bar: &FunctionBar) -> i32 {
        bar.title()
            .split('.')
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    }

    #[test]
    fn local_score_appended_after_all_entries_gets_correct_rank() {
        // Leaderboard has one entry with exscore=600 (epg=300),
        // local score has exscore=600 (epg=300, equal = not better).
        // The insertion logic: local_score.exscore() (600) is NOT > leaderboard[0].exscore() (600),
        // so the pre-loop branch is skipped.
        // In the loop: entry[0].exscore() (600) > local_score.exscore() (600) is false,
        // so the mid-loop insertion is skipped.
        // After the loop: !inserted is true, local score appended at the end.
        // Expected: leaderboard entry = rank 1, local score = rank 2.
        let bar = make_bar();
        let leaderboard = vec![make_entry(300, "RivalA")];
        let local_score = make_ir_score(300);

        let bars = bar.from_ir_score_data_with_local(&local_score, &leaderboard);
        assert_eq!(bars.len(), 2);
        assert_eq!(extract_rank(&bars[0]), 1); // RivalA
        assert_eq!(extract_rank(&bars[1]), 2); // Local (was incorrectly 1 before fix)
    }

    #[test]
    fn local_score_equal_to_last_entry_appended_after() {
        // Leaderboard: [exscore=500, exscore=400], local: exscore=400 (equal to last).
        // The last entry's exscore (400) is NOT strictly greater than local (400),
        // so the mid-loop insertion condition fails and the !inserted path is taken.
        // Expected: ranks 1, 2, 3.
        let bar = make_bar();
        let leaderboard = vec![make_entry(250, "A"), make_entry(200, "B")];
        let local_score = make_ir_score(200); // exscore=400, equal to B

        let bars = bar.from_ir_score_data_with_local(&local_score, &leaderboard);
        assert_eq!(bars.len(), 3);
        assert_eq!(extract_rank(&bars[0]), 1);
        assert_eq!(extract_rank(&bars[1]), 2);
        assert_eq!(extract_rank(&bars[2]), 3);
    }

    #[test]
    fn local_score_strictly_worse_inserted_after_last() {
        // Leaderboard: [exscore=600, exscore=400], local: exscore=200 (worse than all).
        // For i=0: 600 > 200 yes, but next (400) > 200, so not inserted yet.
        // For i=1 (last): 400 > 200 yes, and i==len-1, so inserted mid-loop.
        // Expected: ranks 1, 2, 3.
        let bar = make_bar();
        let leaderboard = vec![make_entry(300, "A"), make_entry(200, "B")];
        let local_score = make_ir_score(100); // exscore=200, strictly less than B's 400

        let bars = bar.from_ir_score_data_with_local(&local_score, &leaderboard);
        assert_eq!(bars.len(), 3);
        assert_eq!(extract_rank(&bars[0]), 1);
        assert_eq!(extract_rank(&bars[1]), 2);
        assert_eq!(extract_rank(&bars[2]), 3);
    }

    #[test]
    fn local_score_best_gets_rank_1() {
        // Local score beats everyone.
        let bar = make_bar();
        let leaderboard = vec![make_entry(100, "A"), make_entry(50, "B")];
        let local_score = make_ir_score(200); // exscore=400, beats all

        let bars = bar.from_ir_score_data_with_local(&local_score, &leaderboard);
        assert_eq!(bars.len(), 3);
        assert_eq!(extract_rank(&bars[0]), 1); // Local
        assert!(bars[0].title().contains("Player"));
        assert_eq!(extract_rank(&bars[1]), 2); // A
        assert_eq!(extract_rank(&bars[2]), 3); // B
    }

    #[test]
    fn local_score_inserted_mid_leaderboard() {
        // Leaderboard: [exscore=600, exscore=200], local: exscore=400
        // Local should be inserted between the two entries.
        let bar = make_bar();
        let leaderboard = vec![make_entry(300, "Top"), make_entry(100, "Bottom")];
        let local_score = make_ir_score(200); // exscore=400

        let bars = bar.from_ir_score_data_with_local(&local_score, &leaderboard);
        assert_eq!(bars.len(), 3);
        assert_eq!(extract_rank(&bars[0]), 1); // Top
        assert_eq!(extract_rank(&bars[1]), 2); // Local
        assert!(bars[1].title().contains("Player"));
        assert_eq!(extract_rank(&bars[2]), 3); // Bottom
    }

    #[test]
    fn empty_leaderboard_local_score_gets_rank_1() {
        let bar = make_bar();
        let leaderboard: Vec<LeaderboardEntry> = vec![];
        let local_score = make_ir_score(100);

        let bars = bar.from_ir_score_data_with_local(&local_score, &leaderboard);
        assert_eq!(bars.len(), 1);
        assert_eq!(extract_rank(&bars[0]), 1);
        assert!(bars[0].title().contains("Player"));
    }

    #[test]
    fn from_ir_score_data_ranks_are_1_based() {
        let bar = make_bar();
        let leaderboard = vec![
            make_entry(300, "A"),
            make_entry(200, "B"),
            make_entry(100, "C"),
        ];

        let bars = bar.from_ir_score_data(&leaderboard);
        assert_eq!(bars.len(), 3);
        assert_eq!(extract_rank(&bars[0]), 1);
        assert_eq!(extract_rank(&bars[1]), 2);
        assert_eq!(extract_rank(&bars[2]), 3);
    }
}
