// Leaderboard display — converts IR leaderboard entries to display bars.
//
// Uses `bms_ir::LeaderboardEntry` as the data source and converts entries
// into `Bar::Function` for display in the music select bar list.

use bms_database::SongData;
use bms_ir::LeaderboardEntry;
use bms_rule::ClearType;

use super::bar_manager::{Bar, FunctionAction};

/// Convert leaderboard entries to Function bars for display.
pub fn entries_to_bars(entries: &[LeaderboardEntry], song_data: &SongData) -> Vec<Bar> {
    if entries.is_empty() {
        return vec![Bar::Function {
            title: "No scores found".to_string(),
            subtitle: None,
            display_bar_type: 5,
            action: FunctionAction::None,
            lamp: 0,
        }];
    }
    entries
        .iter()
        .map(|entry| {
            let clear_name = clear_type_name(entry.ir_score.clear);
            Bar::Function {
                title: format!("{} ({})", entry.ir_score.player, clear_name),
                subtitle: Some(format!(
                    "EX: {} | Combo: {} | Miss: {}",
                    entry.ir_score.exscore(),
                    entry.ir_score.maxcombo,
                    entry.ir_score.minbp
                )),
                display_bar_type: 0,
                action: FunctionAction::GhostBattle {
                    song_data: Box::new(song_data.clone()),
                    lr2_id: entry.lr2_id,
                    lane_sequence: 0, // Populated by ghost data fetch when available
                },
                lamp: entry.ir_score.clear.id() as i32,
            }
        })
        .collect()
}

/// Convert a leaderboard error to a single error bar.
pub fn error_to_bars(msg: &str) -> Vec<Bar> {
    vec![Bar::Function {
        title: format!("Error: {msg}"),
        subtitle: None,
        display_bar_type: 5,
        action: FunctionAction::None,
        lamp: 0,
    }]
}

/// Convert a ClearType to a human-readable name.
fn clear_type_name(clear: ClearType) -> &'static str {
    match clear {
        ClearType::NoPlay => "No Play",
        ClearType::Failed => "Failed",
        ClearType::AssistEasy => "Assist",
        ClearType::LightAssistEasy => "LightAssist",
        ClearType::Easy => "Easy",
        ClearType::Normal => "Normal",
        ClearType::Hard => "Hard",
        ClearType::ExHard => "ExHard",
        ClearType::FullCombo => "FullCombo",
        ClearType::Perfect => "Perfect",
        ClearType::Max => "Max",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_ir::IRScoreData;
    use bms_rule::ScoreData;

    fn make_entry(
        player: &str,
        clear: ClearType,
        epg: i32,
        egr: i32,
        maxcombo: i32,
        minbp: i32,
        lr2_id: i64,
    ) -> LeaderboardEntry {
        let sd = ScoreData {
            player: player.to_string(),
            clear,
            epg,
            egr,
            maxcombo,
            minbp,
            ..Default::default()
        };
        let ir_score = IRScoreData::from(&sd);
        LeaderboardEntry::new_lr2(ir_score, lr2_id)
    }

    fn sample_song_data() -> SongData {
        SongData {
            md5: "test_md5".to_string(),
            sha256: "test_sha256".to_string(),
            title: "Test Song".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn entries_to_bars_empty_shows_no_scores() {
        let bars = entries_to_bars(&[], &sample_song_data());
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].bar_name(), "No scores found");
        assert_eq!(bars[0].bar_display_type(), 5);
    }

    #[test]
    fn entries_to_bars_single_entry() {
        let entries = vec![make_entry("Alice", ClearType::Hard, 500, 300, 800, 10, 100)];
        let song = sample_song_data();
        let bars = entries_to_bars(&entries, &song);

        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].bar_name(), "Alice (Hard)");
        match &bars[0] {
            Bar::Function {
                subtitle,
                lamp,
                action,
                ..
            } => {
                // exscore = epg*2 + egr = 1000 + 300 = 1300
                assert_eq!(
                    subtitle.as_deref(),
                    Some("EX: 1300 | Combo: 800 | Miss: 10")
                );
                assert_eq!(*lamp, ClearType::Hard.id() as i32);
                assert!(matches!(
                    action,
                    FunctionAction::GhostBattle { lr2_id: 100, .. }
                ));
            }
            _ => panic!("expected Function bar"),
        }
    }

    #[test]
    fn entries_to_bars_multiple_entries() {
        let entries = vec![
            make_entry("Alice", ClearType::Hard, 500, 300, 800, 10, 100),
            make_entry("Bob", ClearType::Normal, 200, 150, 400, 20, 200),
            make_entry("Charlie", ClearType::FullCombo, 600, 100, 1000, 0, 300),
        ];
        let song = sample_song_data();
        let bars = entries_to_bars(&entries, &song);

        assert_eq!(bars.len(), 3);
        assert_eq!(bars[0].bar_name(), "Alice (Hard)");
        assert_eq!(bars[1].bar_name(), "Bob (Normal)");
        assert_eq!(bars[2].bar_name(), "Charlie (FullCombo)");
    }

    #[test]
    fn entries_to_bars_ghost_battle_action() {
        let entries = vec![make_entry("Player1", ClearType::Easy, 100, 50, 150, 5, 42)];
        let song = sample_song_data();
        let bars = entries_to_bars(&entries, &song);

        match &bars[0] {
            Bar::Function { action, .. } => match action {
                FunctionAction::GhostBattle {
                    song_data, lr2_id, ..
                } => {
                    assert_eq!(song_data.sha256, "test_sha256");
                    assert_eq!(*lr2_id, 42);
                }
                _ => panic!("expected GhostBattle action"),
            },
            _ => panic!("expected Function bar"),
        }
    }

    #[test]
    fn error_to_bars_formatting() {
        let bars = error_to_bars("Connection refused");
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].bar_name(), "Error: Connection refused");
        assert_eq!(bars[0].bar_display_type(), 5);
        match &bars[0] {
            Bar::Function { action, lamp, .. } => {
                assert!(matches!(action, FunctionAction::None));
                assert_eq!(*lamp, 0);
            }
            _ => panic!("expected Function bar"),
        }
    }

    #[test]
    fn clear_type_name_all_variants() {
        assert_eq!(clear_type_name(ClearType::NoPlay), "No Play");
        assert_eq!(clear_type_name(ClearType::Failed), "Failed");
        assert_eq!(clear_type_name(ClearType::AssistEasy), "Assist");
        assert_eq!(clear_type_name(ClearType::LightAssistEasy), "LightAssist");
        assert_eq!(clear_type_name(ClearType::Easy), "Easy");
        assert_eq!(clear_type_name(ClearType::Normal), "Normal");
        assert_eq!(clear_type_name(ClearType::Hard), "Hard");
        assert_eq!(clear_type_name(ClearType::ExHard), "ExHard");
        assert_eq!(clear_type_name(ClearType::FullCombo), "FullCombo");
        assert_eq!(clear_type_name(ClearType::Perfect), "Perfect");
        assert_eq!(clear_type_name(ClearType::Max), "Max");
    }

    #[test]
    fn entries_to_bars_lamp_matches_clear_id() {
        let entries = vec![
            make_entry("A", ClearType::NoPlay, 0, 0, 0, 0, 1),
            make_entry("B", ClearType::Failed, 0, 0, 0, 0, 2),
            make_entry("C", ClearType::Easy, 0, 0, 0, 0, 3),
            make_entry("D", ClearType::Max, 0, 0, 0, 0, 4),
        ];
        let song = sample_song_data();
        let bars = entries_to_bars(&entries, &song);

        let expected_lamps = [0, 1, 4, 10];
        for (bar, expected) in bars.iter().zip(expected_lamps.iter()) {
            match bar {
                Bar::Function { lamp, .. } => assert_eq!(lamp, expected),
                _ => panic!("expected Function bar"),
            }
        }
    }
}
