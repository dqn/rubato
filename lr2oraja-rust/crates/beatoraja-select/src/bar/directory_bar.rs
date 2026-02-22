use super::bar::{Bar, BarData};
use crate::stubs::*;

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

    pub fn get_lamps(&self) -> &[i32; 11] {
        &self.lamps
    }

    pub fn get_rival_lamps(&self) -> &[i32; 11] {
        &self.rlamps
    }

    pub fn get_ranks(&self) -> &[i32; 28] {
        &self.ranks
    }

    pub fn get_lamp(&self, is_player: bool) -> i32 {
        let lamps = if is_player { &self.lamps } else { &self.rlamps };
        for i in 0..lamps.len() {
            if lamps[i] > 0 {
                return i as i32;
            }
        }
        0
    }

    pub fn is_show_invisible_chart(&self) -> bool {
        self.show_invisible_chart
    }

    pub fn is_sortable(&self) -> bool {
        self.sortable
    }

    pub fn set_sortable(&mut self, val: bool) {
        self.sortable = val;
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
            if song.get_path().is_none() {
                continue;
            }
            if let Some(m) = mode
                && song.get_mode() != 0
                && song.get_mode() != m.id()
            {
                continue;
            }
            let score = score_fn(song);
            if let Some(ref score) = score {
                let clear = score.get_clear() as usize;
                if clear < self.lamps.len() {
                    self.lamps[clear] += 1;
                }
                if score.get_notes() != 0 {
                    let rank = (score.get_exscore() * 27 / (score.get_notes() * 2)) as usize;
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

    /// Filter children by mode and same-folder flag
    pub fn get_children_filtered(
        children: &[Bar],
        mode: Option<&bms_model::Mode>,
        contains_same_folder: bool,
    ) -> Vec<Bar> {
        // NOTE: This method creates new Bar values which would require Clone.
        // In practice this is called on the Java side via getChildren(Mode, boolean).
        // For now we stub it since we cannot Clone enum-of-Box easily.
        log::warn!("not yet implemented: DirectoryBar.getChildren(mode, containsSameFolder)");
        Vec::new()
    }
}
