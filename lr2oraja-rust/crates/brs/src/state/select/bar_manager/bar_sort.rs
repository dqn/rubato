// Bar sorting — sort and filter methods for BarManager.
//
// Java parity: all sort modes (except Default) fall back to title_cmp()
// when either bar is not a Song. TITLE sort treats Song and Folder as
// "sortable" bar types; others go to end.

use std::cmp::Ordering;
use std::collections::HashMap;

use bms_rule::ScoreData;

use bms_database::song_data::{INVISIBLE_CHART, INVISIBLE_SONG};

use super::bar_types::Bar;
use super::{BarManager, SortMode};

/// TITLE sort comparator.
///
/// Sorts all bars by their display name (case-insensitive). When both are
/// Song with equal titles, sub-sort by difficulty.
fn title_cmp(a: &Bar, b: &Bar) -> Ordering {
    // Special case: both are Song with equal titles → sub-sort by difficulty
    if let (Bar::Song(sa), Bar::Song(sb)) = (a, b) {
        let title_ord = sa.title.to_lowercase().cmp(&sb.title.to_lowercase());
        if title_ord == Ordering::Equal {
            return sa.difficulty.cmp(&sb.difficulty);
        }
        return title_ord;
    }

    // Otherwise, compare by display name
    a.bar_name()
        .to_lowercase()
        .cmp(&b.bar_name().to_lowercase())
}

/// Fallback comparator for non-TITLE sort modes.
///
/// Java parity: when either bar is not a Song, falls back to TITLE sort
/// (case-insensitive display name comparison). Returns `None` when both
/// bars are Song so the caller can apply sort-mode-specific logic.
fn title_fallback_cmp(a: &Bar, b: &Bar) -> Option<Ordering> {
    if matches!(a, Bar::Song(_)) && matches!(b, Bar::Song(_)) {
        None
    } else {
        Some(title_cmp(a, b))
    }
}

/// Compute (player_clear - rival_clear) for a song.
///
/// Java parity: `BarSorter.RivalCompareClear` sorts by the difference
/// between player and rival clear types (descending).
fn player_rival_clear_diff(
    song: &bms_database::SongData,
    player: &HashMap<String, ScoreData>,
    rival: &HashMap<String, ScoreData>,
) -> i32 {
    let p = player
        .get(&song.sha256)
        .map(|s| s.clear.id() as i32)
        .unwrap_or(0);
    let r = rival
        .get(&song.sha256)
        .map(|s| s.clear.id() as i32)
        .unwrap_or(0);
    p - r
}

/// Compute (player_exscore - rival_exscore) for a song.
///
/// Java parity: `BarSorter.RivalCompareScore` sorts by the difference
/// between player and rival EX scores (descending).
fn player_rival_exscore_diff(
    song: &bms_database::SongData,
    player: &HashMap<String, ScoreData>,
    rival: &HashMap<String, ScoreData>,
) -> i32 {
    let p = player.get(&song.sha256).map(|s| s.exscore()).unwrap_or(0);
    let r = rival.get(&song.sha256).map(|s| s.exscore()).unwrap_or(0);
    p - r
}

/// Returns a stable identity key for a bar, used for cursor restoration.
///
/// Song bars use sha256; other bars use display_type + bar_name.
fn bar_identity(bar: &Bar) -> String {
    match bar {
        Bar::Song(s) => format!("song:{}", s.sha256),
        other => format!("{}:{}", other.bar_display_type(), other.bar_name()),
    }
}

impl BarManager {
    /// Restore cursor to the bar matching the given identity key.
    ///
    /// Java parity: `BarManager` L426-450. After sorting or filtering,
    /// the cursor is restored to the same bar the user was looking at.
    pub fn restore_cursor(&mut self, identity: &str) {
        if let Some(pos) = self.bars.iter().position(|b| bar_identity(b) == identity) {
            self.cursor = pos;
        }
        // If not found, keep cursor at 0 (already reset by sort/filter)
    }

    /// Sort bars by the given mode.
    ///
    /// Follows Java BarSorter parity: non-Song bars fall back to TITLE sort
    /// for all modes except Default. Score-dependent modes use the
    /// `score_cache` keyed by SHA-256.
    pub fn sort(&mut self, mode: SortMode, score_cache: &HashMap<String, ScoreData>) {
        // Save current bar identity for cursor restoration
        let saved_identity = self.bars.get(self.cursor).map(bar_identity);

        match mode {
            SortMode::Default => {} // Keep original order
            SortMode::Title => {
                self.bars.sort_by(title_cmp);
            }
            SortMode::Artist => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    sa.artist.to_lowercase().cmp(&sb.artist.to_lowercase())
                });
            }
            SortMode::Level => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let level_ord = sa.level.cmp(&sb.level);
                    if level_ord == Ordering::Equal {
                        return sa.difficulty.cmp(&sb.difficulty);
                    }
                    level_ord
                });
            }
            SortMode::Bpm => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    sa.maxbpm.cmp(&sb.maxbpm)
                });
            }
            SortMode::Length => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    sa.length.cmp(&sb.length)
                });
            }
            SortMode::Clear => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let score_a = score_cache.get(&sa.sha256);
                    let score_b = score_cache.get(&sb.sha256);
                    match (score_a, score_b) {
                        (None, None) => Ordering::Equal,
                        (None, Some(_)) => Ordering::Greater,
                        (Some(_), None) => Ordering::Less,
                        (Some(a), Some(b)) => (a.clear.id() as i32).cmp(&(b.clear.id() as i32)),
                    }
                });
            }
            SortMode::Score => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let n1 = score_cache.get(&sa.sha256).map(|sd| sd.notes).unwrap_or(0);
                    let n2 = score_cache.get(&sb.sha256).map(|sd| sd.notes).unwrap_or(0);
                    if n1 == 0 && n2 == 0 {
                        return Ordering::Equal;
                    }
                    if n1 == 0 {
                        return Ordering::Greater;
                    }
                    if n2 == 0 {
                        return Ordering::Less;
                    }
                    let r1 = score_cache.get(&sa.sha256).unwrap().exscore() as f32 / n1 as f32;
                    let r2 = score_cache.get(&sb.sha256).unwrap().exscore() as f32 / n2 as f32;
                    r1.partial_cmp(&r2).unwrap_or(Ordering::Equal)
                });
            }
            SortMode::MissCount => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let score_a = score_cache.get(&sa.sha256);
                    let score_b = score_cache.get(&sb.sha256);
                    match (score_a, score_b) {
                        (None, None) => Ordering::Equal,
                        (None, Some(_)) => Ordering::Greater,
                        (Some(_), None) => Ordering::Less,
                        (Some(a), Some(b)) => a.minbp.cmp(&b.minbp),
                    }
                });
            }
            SortMode::Duration => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let exists_a = score_cache
                        .get(&sa.sha256)
                        .is_some_and(|sd| sd.avgjudge != i64::MAX);
                    let exists_b = score_cache
                        .get(&sb.sha256)
                        .is_some_and(|sd| sd.avgjudge != i64::MAX);
                    if !exists_a && !exists_b {
                        return Ordering::Equal;
                    }
                    if !exists_a {
                        return Ordering::Greater;
                    }
                    if !exists_b {
                        return Ordering::Less;
                    }
                    let aj_a = score_cache.get(&sa.sha256).unwrap().avgjudge;
                    let aj_b = score_cache.get(&sb.sha256).unwrap().avgjudge;
                    aj_a.cmp(&aj_b)
                });
            }
            SortMode::LastUpdate => {
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let score_a = score_cache.get(&sa.sha256);
                    let score_b = score_cache.get(&sb.sha256);
                    match (score_a, score_b) {
                        (None, None) => Ordering::Equal,
                        (None, Some(_)) => Ordering::Greater,
                        (Some(_), None) => Ordering::Less,
                        (Some(a), Some(b)) => a.date.cmp(&b.date),
                    }
                });
            }
            SortMode::RivalCompareClear => {
                let rival_scores = &self.rival_scores;
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let diff_a = player_rival_clear_diff(sa, score_cache, rival_scores);
                    let diff_b = player_rival_clear_diff(sb, score_cache, rival_scores);
                    diff_b.cmp(&diff_a) // descending: higher diff = player ahead
                });
            }
            SortMode::RivalCompareScore => {
                let rival_scores = &self.rival_scores;
                self.bars.sort_by(|a, b| {
                    if let Some(ord) = title_fallback_cmp(a, b) {
                        return ord;
                    }
                    let sa = a.as_song().unwrap();
                    let sb = b.as_song().unwrap();
                    let diff_a = player_rival_exscore_diff(sa, score_cache, rival_scores);
                    let diff_b = player_rival_exscore_diff(sb, score_cache, rival_scores);
                    diff_b.cmp(&diff_a) // descending: higher diff = player ahead
                });
            }
        }
        // Restore cursor to the previously selected bar
        if let Some(identity) = saved_identity {
            self.restore_cursor(&identity);
        } else {
            self.cursor = 0;
        }
    }

    /// Filter bars to retain only songs matching the given mode ID.
    /// Non-Song bars are always retained.
    pub fn filter_by_mode(&mut self, mode: Option<i32>) {
        if let Some(mode_id) = mode {
            self.bars.retain(|bar| match bar {
                Bar::Song(s) => s.mode == mode_id,
                _ => true,
            });
            self.cursor = 0;
        }
    }

    /// Remove Song bars with INVISIBLE_SONG or INVISIBLE_CHART flags set.
    ///
    /// Java parity: `BarManager` L351-359. Non-Song bars are always retained.
    pub fn filter_invisible(&mut self) {
        let invisible_mask = INVISIBLE_SONG | INVISIBLE_CHART;
        self.bars.retain(|bar| match bar {
            Bar::Song(s) => s.favorite & invisible_mask == 0,
            _ => true,
        });
    }

    /// Remove Song bars whose files don't exist on disk (empty path).
    ///
    /// Java parity: `BarManager` L332-341. Called when
    /// `config.show_no_song_existing_bar` is false.
    pub fn filter_non_existing(&mut self) {
        if !self.show_no_song_existing_bar {
            self.bars.retain(|bar| match bar {
                Bar::Song(s) => !s.path.is_empty(),
                _ => true,
            });
        }
    }
}
