use super::bar::{Bar, BarData};
use crate::select::stubs::*;

/// Directory bar shared data
/// Translates: bms.player.beatoraja.select.bar.DirectoryBar
#[derive(Clone, Debug)]
pub struct DirectoryBarData {
    pub bar_data: BarData,
    /// Player clear lamp counts
    pub lamps: [i32; 11],
    /// Rival clear lamp counts
    pub rlamps: [i32; 11],
    /// Player rank counts
    pub ranks: [i32; 28],
    /// Whether to show invisible charts
    pub show_invisible_chart: bool,
    /// Whether this folder can be sorted
    pub sortable: bool,
}

impl Default for DirectoryBarData {
    fn default() -> Self {
        Self {
            bar_data: BarData::default(),
            lamps: [0; 11],
            rlamps: [0; 11],
            ranks: [0; 28],
            show_invisible_chart: false,
            sortable: true,
        }
    }
}

impl DirectoryBarData {
    pub fn new(show_invisible_chart: bool) -> Self {
        Self {
            show_invisible_chart,
            ..Default::default()
        }
    }

    pub fn lamps(&self) -> &[i32; 11] {
        &self.lamps
    }

    pub fn rival_lamps(&self) -> &[i32; 11] {
        &self.rlamps
    }

    pub fn ranks(&self) -> &[i32; 28] {
        &self.ranks
    }

    pub fn lamp(&self, is_player: bool) -> i32 {
        let lamps = if is_player { &self.lamps } else { &self.rlamps };
        if let Some(pos) = lamps.iter().position(|&l| l > 0) {
            return pos as i32;
        }
        0
    }

    pub fn is_show_invisible_chart(&self) -> bool {
        self.show_invisible_chart
    }

    pub fn is_sortable(&self) -> bool {
        self.sortable
    }

    pub fn clear(&mut self) {
        self.lamps.fill(0);
        self.rlamps.fill(0);
        self.ranks.fill(0);
    }

    /// No-op base version.
    /// Corresponds to Java DirectoryBar.updateFolderStatus()
    pub fn update_folder_status(&mut self) {
        // Base implementation is no-op (Java: empty method body)
    }

    /// Update folder lamp/rank status from song data.
    /// Corresponds to Java DirectoryBar.updateFolderStatus(SongData[] songs)
    pub fn update_folder_status_with_songs(
        &mut self,
        songs: &[SongData],
        mode: Option<&bms_model::Mode>,
        score_fn: impl Fn(&SongData) -> Option<ScoreData>,
    ) {
        self.clear();
        for song in songs {
            if song.file.path().is_none() {
                continue;
            }
            if let Some(m) = mode
                && song.chart.mode != 0
                && song.chart.mode != m.id()
            {
                continue;
            }
            let score = score_fn(song);
            if let Some(ref score) = score {
                let clear = score.clear as usize;
                if clear < self.lamps.len() {
                    self.lamps[clear] += 1;
                }
                if score.notes != 0 {
                    let rank = (score.exscore() * 27 / (score.notes * 2)) as usize;
                    let rank = if rank < 28 { rank } else { 27 };
                    self.ranks[rank] += 1;
                } else {
                    self.ranks[0] += 1;
                }
            } else {
                self.lamps[0] += 1;
                self.ranks[0] += 1;
            }
        }
    }

    /// Filter children by mode and same-folder flag.
    /// If mode is set, filters out SongBars whose mode doesn't match.
    /// If contains_same_folder is false, deduplicates SongBars by folder path.
    ///
    /// Translates: Java DirectoryBar.getChildren(Mode, boolean)
    pub fn children_filtered(
        children: &[Bar],
        mode: Option<&bms_model::Mode>,
        contains_same_folder: bool,
    ) -> Vec<Bar> {
        let mut result: Vec<Bar> = Vec::new();
        for b in children {
            // Mode filtering: skip SongBars whose mode doesn't match
            if let Some(mode) = mode
                && let Some(sb) = b.as_song_bar()
            {
                let song_mode = sb.song_data().chart.mode;
                if song_mode != 0 && song_mode != mode.id() {
                    continue;
                }
            }

            // Same-folder deduplication
            let mut add_bar = true;
            if !contains_same_folder && let Some(sb) = b.as_song_bar() {
                let folder = &sb.song_data().folder;
                if !folder.is_empty() {
                    for existing in &result {
                        if let Some(existing_sb) = existing.as_song_bar() {
                            let existing_folder = &existing_sb.song_data().folder;
                            if folder == existing_folder {
                                add_bar = false;
                                break;
                            }
                        }
                    }
                }
            }

            if add_bar {
                result.push(b.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::bar::song_bar::SongBar;

    fn make_song_bar(title: &str, sha256: &str, mode: i32, folder: &str) -> Bar {
        let mut song = SongData::default();
        song.metadata.title = title.to_string();
        song.file.sha256 = sha256.to_string();
        song.chart.mode = mode;
        song.folder = folder.to_string();
        Bar::Song(Box::new(SongBar::new(song)))
    }

    #[test]
    fn get_children_filtered_passes_all_when_no_mode_and_same_folder() {
        let children = vec![
            make_song_bar("A", "sha_a", 0, "/dir1"),
            make_song_bar("B", "sha_b", 0, "/dir2"),
        ];

        let result = DirectoryBarData::children_filtered(&children, None, true);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn get_children_filtered_filters_by_mode() {
        let children = vec![
            make_song_bar("7K Song", "sha_7k", 7, "/dir1"),
            make_song_bar("5K Song", "sha_5k", 5, "/dir2"),
            make_song_bar("Any Mode", "sha_any", 0, "/dir3"),
        ];

        let mode_7k = bms_model::Mode::BEAT_7K;
        let result = DirectoryBarData::children_filtered(&children, Some(&mode_7k), true);

        // 7K Song (mode 7 matches) and Any Mode (mode 0 passes) should remain
        // 5K Song should be filtered out
        assert_eq!(result.len(), 2);
        assert!(result[0].title().contains("7K Song"));
        assert!(result[1].title().contains("Any Mode"));
    }

    #[test]
    fn get_children_filtered_deduplicates_by_folder() {
        let children = vec![
            make_song_bar("Song A", "sha_a", 0, "/same_dir"),
            make_song_bar("Song B", "sha_b", 0, "/same_dir"),
            make_song_bar("Song C", "sha_c", 0, "/other_dir"),
        ];

        let result = DirectoryBarData::children_filtered(&children, None, false);

        // Song A and Song B have same folder, so B should be deduplicated
        assert_eq!(result.len(), 2);
        assert!(result[0].title().contains("Song A"));
        assert!(result[1].title().contains("Song C"));
    }

    #[test]
    fn get_children_filtered_keeps_duplicates_when_contains_same_folder() {
        let children = vec![
            make_song_bar("Song A", "sha_a", 0, "/same_dir"),
            make_song_bar("Song B", "sha_b", 0, "/same_dir"),
        ];

        let result = DirectoryBarData::children_filtered(&children, None, true);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn get_children_filtered_non_song_bars_pass_through() {
        use crate::select::bar::folder_bar::FolderBar;

        let children = vec![
            Bar::Folder(Box::new(FolderBar::new(None, "crc1".to_string()))),
            make_song_bar("Song", "sha_s", 5, "/dir"),
        ];

        let mode_7k = bms_model::Mode::BEAT_7K;
        let result = DirectoryBarData::children_filtered(&children, Some(&mode_7k), true);

        // Folder bar should pass (not a SongBar), Song with mode 5 should be filtered
        assert_eq!(result.len(), 1);
    }
}
