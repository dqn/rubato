use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use super::bar::bar::Bar;

/// Bar sorting algorithms
/// Translates: bms.player.beatoraja.select.BarSorter
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarSorter {
    Title,
    Artist,
    Bpm,
    Length,
    Level,
    Clear,
    Score,
    MissCount,
    Duration,
    LastUpdate,
    RivalCompareClear,
    RivalCompareScore,
}

impl FromStr for BarSorter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TITLE" => Ok(Self::Title),
            "ARTIST" => Ok(Self::Artist),
            "BPM" => Ok(Self::Bpm),
            "LENGTH" => Ok(Self::Length),
            "LEVEL" => Ok(Self::Level),
            "CLEAR" => Ok(Self::Clear),
            "SCORE" => Ok(Self::Score),
            "MISSCOUNT" => Ok(Self::MissCount),
            "DURATION" => Ok(Self::Duration),
            "LASTUPDATE" => Ok(Self::LastUpdate),
            "RIVALCOMPARE_CLEAR" => Ok(Self::RivalCompareClear),
            "RIVALCOMPARE_SCORE" => Ok(Self::RivalCompareScore),
            _ => anyhow::bail!("unknown BarSorter: {}", s),
        }
    }
}

impl fmt::Display for BarSorter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Title => write!(f, "TITLE"),
            Self::Artist => write!(f, "ARTIST"),
            Self::Bpm => write!(f, "BPM"),
            Self::Length => write!(f, "LENGTH"),
            Self::Level => write!(f, "LEVEL"),
            Self::Clear => write!(f, "CLEAR"),
            Self::Score => write!(f, "SCORE"),
            Self::MissCount => write!(f, "MISSCOUNT"),
            Self::Duration => write!(f, "DURATION"),
            Self::LastUpdate => write!(f, "LASTUPDATE"),
            Self::RivalCompareClear => write!(f, "RIVALCOMPARE_CLEAR"),
            Self::RivalCompareScore => write!(f, "RIVALCOMPARE_SCORE"),
        }
    }
}

impl BarSorter {
    pub const DEFAULT_SORTER: &'static [BarSorter] = &[
        BarSorter::Title,
        BarSorter::Artist,
        BarSorter::Bpm,
        BarSorter::Length,
        BarSorter::Level,
        BarSorter::Clear,
        BarSorter::Score,
        BarSorter::MissCount,
    ];

    pub const ALL_SORTER: &'static [BarSorter] = &[
        BarSorter::Title,
        BarSorter::Artist,
        BarSorter::Bpm,
        BarSorter::Length,
        BarSorter::Level,
        BarSorter::Clear,
        BarSorter::Score,
        BarSorter::MissCount,
        BarSorter::Duration,
        BarSorter::LastUpdate,
        BarSorter::RivalCompareClear,
        BarSorter::RivalCompareScore,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            BarSorter::Title => "TITLE",
            BarSorter::Artist => "ARTIST",
            BarSorter::Bpm => "BPM",
            BarSorter::Length => "LENGTH",
            BarSorter::Level => "LEVEL",
            BarSorter::Clear => "CLEAR",
            BarSorter::Score => "SCORE",
            BarSorter::MissCount => "MISSCOUNT",
            BarSorter::Duration => "DURATION",
            BarSorter::LastUpdate => "LASTUPDATE",
            BarSorter::RivalCompareClear => "RIVALCOMPARE_CLEAR",
            BarSorter::RivalCompareScore => "RIVALCOMPARE_SCORE",
        }
    }

    pub fn value_of(name: &str) -> Option<BarSorter> {
        name.parse().ok()
    }

    pub fn compare(&self, o1: &Bar, o2: &Bar) -> Ordering {
        match self {
            BarSorter::Title => Self::compare_title(o1, o2),
            BarSorter::Artist => Self::compare_artist(o1, o2),
            BarSorter::Bpm => Self::compare_bpm(o1, o2),
            BarSorter::Length => Self::compare_length(o1, o2),
            BarSorter::Level => Self::compare_level(o1, o2),
            BarSorter::Clear => Self::compare_clear(o1, o2),
            BarSorter::Score => Self::compare_score(o1, o2),
            BarSorter::MissCount => Self::compare_misscount(o1, o2),
            BarSorter::Duration => Self::compare_duration(o1, o2),
            BarSorter::LastUpdate => Self::compare_lastupdate(o1, o2),
            BarSorter::RivalCompareClear => Self::compare_rival_clear(o1, o2),
            BarSorter::RivalCompareScore => Self::compare_rival_score(o1, o2),
        }
    }

    fn is_song_or_folder(bar: &Bar) -> bool {
        bar.as_song_bar().is_some() || bar.as_folder_bar().is_some()
    }

    fn compare_title(o1: &Bar, o2: &Bar) -> Ordering {
        if !Self::is_song_or_folder(o1) && !Self::is_song_or_folder(o2) {
            return Ordering::Equal;
        }
        if !Self::is_song_or_folder(o1) {
            return Ordering::Greater;
        }
        if !Self::is_song_or_folder(o2) {
            return Ordering::Less;
        }

        if let (Some(s1), Some(s2)) = (o1.as_song_bar(), o2.as_song_bar()) {
            let title_cmp = s1
                .song
                .metadata
                .title
                .to_lowercase()
                .cmp(&s2.song.metadata.title.to_lowercase());
            if title_cmp == Ordering::Equal {
                return s1.song.chart.difficulty.cmp(&s2.song.chart.difficulty);
            }
            return title_cmp;
        }

        o1.title().to_lowercase().cmp(&o2.title().to_lowercase())
    }

    fn compare_artist(o1: &Bar, o2: &Bar) -> Ordering {
        let (s1, s2) = match (o1.as_song_bar(), o2.as_song_bar()) {
            (Some(s1), Some(s2)) => (s1, s2),
            _ => return Self::compare_title(o1, o2),
        };
        if !s1.exists_song() && !s2.exists_song() {
            return Ordering::Equal;
        }
        if !s1.exists_song() {
            return Ordering::Greater;
        }
        if !s2.exists_song() {
            return Ordering::Less;
        }
        s1.song
            .metadata
            .artist
            .to_lowercase()
            .cmp(&s2.song.metadata.artist.to_lowercase())
    }

    fn compare_bpm(o1: &Bar, o2: &Bar) -> Ordering {
        let (s1, s2) = match (o1.as_song_bar(), o2.as_song_bar()) {
            (Some(s1), Some(s2)) => (s1, s2),
            _ => return Self::compare_title(o1, o2),
        };
        if !s1.exists_song() && !s2.exists_song() {
            return Ordering::Equal;
        }
        if !s1.exists_song() {
            return Ordering::Greater;
        }
        if !s2.exists_song() {
            return Ordering::Less;
        }
        s1.song.chart.maxbpm.cmp(&s2.song.chart.maxbpm)
    }

    fn compare_length(o1: &Bar, o2: &Bar) -> Ordering {
        let (s1, s2) = match (o1.as_song_bar(), o2.as_song_bar()) {
            (Some(s1), Some(s2)) => (s1, s2),
            _ => return Self::compare_title(o1, o2),
        };
        if !s1.exists_song() && !s2.exists_song() {
            return Ordering::Equal;
        }
        if !s1.exists_song() {
            return Ordering::Greater;
        }
        if !s2.exists_song() {
            return Ordering::Less;
        }
        s1.song.chart.length.cmp(&s2.song.chart.length)
    }

    fn compare_level(o1: &Bar, o2: &Bar) -> Ordering {
        let (s1, s2) = match (o1.as_song_bar(), o2.as_song_bar()) {
            (Some(s1), Some(s2)) => (s1, s2),
            _ => return Self::compare_title(o1, o2),
        };
        if !s1.exists_song() && !s2.exists_song() {
            return Ordering::Equal;
        }
        if !s1.exists_song() {
            return Ordering::Greater;
        }
        if !s2.exists_song() {
            return Ordering::Less;
        }
        let level_sort = s1.song.chart.level.cmp(&s2.song.chart.level);
        if level_sort == Ordering::Equal {
            return s1.song.chart.difficulty.cmp(&s2.song.chart.difficulty);
        }
        level_sort
    }

    fn compare_clear(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        match (o1.score(), o2.score()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(s1), Some(s2)) => s1.clear.cmp(&s2.clear),
        }
    }

    fn compare_score(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let (s1, s2) = match (o1.score(), o2.score()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Greater,
            (_, None) => return Ordering::Less,
            (Some(s1), Some(s2)) => (s1, s2),
        };
        let n1 = s1.notes;
        let n2 = s2.notes;
        if n1 == 0 && n2 == 0 {
            return Ordering::Equal;
        }
        if n1 == 0 {
            return Ordering::Greater;
        }
        if n2 == 0 {
            return Ordering::Less;
        }
        let r1 = s1.exscore() as f32 / n1 as f32;
        let r2 = s2.exscore() as f32 / n2 as f32;
        r1.partial_cmp(&r2).unwrap_or(Ordering::Equal)
    }

    fn compare_misscount(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        match (o1.score(), o2.score()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(s1), Some(s2)) => s1.minbp.cmp(&s2.minbp),
        }
    }

    fn compare_duration(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let (s1, s2) = match (o1.score(), o2.score()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Greater,
            (_, None) => return Ordering::Less,
            (Some(s1), Some(s2)) => (s1, s2),
        };
        let exists1 = s1.timing_stats.avgjudge != i64::MAX;
        let exists2 = s2.timing_stats.avgjudge != i64::MAX;
        if !exists1 && !exists2 {
            return Ordering::Equal;
        }
        if !exists1 {
            return Ordering::Greater;
        }
        if !exists2 {
            return Ordering::Less;
        }
        let d = s1.timing_stats.avgjudge - s2.timing_stats.avgjudge;
        d.cmp(&0)
    }

    fn compare_lastupdate(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        match (o1.score(), o2.score()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(s1), Some(s2)) => {
                let d = s1.date - s2.date;
                d.cmp(&0)
            }
        }
    }

    fn compare_rival_clear(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let pair1 = match (o1.score(), o1.rival_score()) {
            (Some(s), Some(r)) => Some((s, r)),
            _ => None,
        };
        let pair2 = match (o2.score(), o2.rival_score()) {
            (Some(s), Some(r)) => Some((s, r)),
            _ => None,
        };
        match (pair1, pair2) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some((s1, r1)), Some((s2, r2))) => {
                let d1 = s1.clear - r1.clear;
                let d2 = s2.clear - r2.clear;
                d1.cmp(&d2)
            }
        }
    }

    fn compare_rival_score(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let pair1 = match (o1.score(), o1.rival_score()) {
            (Some(s), Some(r)) if s.notes > 0 && r.notes > 0 => Some((s, r)),
            _ => None,
        };
        let pair2 = match (o2.score(), o2.rival_score()) {
            (Some(s), Some(r)) if s.notes > 0 && r.notes > 0 => Some((s, r)),
            _ => None,
        };
        match (pair1, pair2) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some((s1, r1)), Some((s2, r2))) => {
                let v1 =
                    s1.exscore() as f32 / s1.notes as f32 - r1.exscore() as f32 / r1.notes as f32;
                let v2 =
                    s2.exscore() as f32 / s2.notes as f32 - r2.exscore() as f32 / r2.notes as f32;
                v1.partial_cmp(&v2).unwrap_or(Ordering::Equal)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::bar::song_bar::SongBar;
    use crate::select::stubs::{ScoreData, SongData};

    /// Create a SongBar with no score
    fn song_bar_no_score(title: &str) -> Bar {
        let mut sd = SongData::default();
        sd.metadata.title = title.to_string();
        sd.file.set_path("/dummy".to_string());
        Bar::Song(Box::new(SongBar::new(sd)))
    }

    /// Create a SongBar with a score
    fn song_bar_with_score(title: &str, score: ScoreData) -> Bar {
        let mut bar = song_bar_no_score(title);
        bar.set_score(Some(score));
        bar
    }

    /// Create a ScoreData with specific exscore components and notes
    fn make_score(epg: i32, lpg: i32, egr: i32, lgr: i32, notes: i32) -> ScoreData {
        let mut sd = ScoreData::default();
        sd.judge_counts.epg = epg;
        sd.judge_counts.lpg = lpg;
        sd.judge_counts.egr = egr;
        sd.judge_counts.lgr = lgr;
        sd.notes = notes;
        sd
    }

    // ---- compare_score: None score handling ----

    #[test]
    fn compare_score_both_none_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::Score.compare(&b1, &b2), Ordering::Equal);
    }

    #[test]
    fn compare_score_first_none_second_has_score() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_with_score("B", make_score(10, 10, 5, 5, 100));
        assert_eq!(BarSorter::Score.compare(&b1, &b2), Ordering::Greater);
    }

    #[test]
    fn compare_score_first_has_score_second_none() {
        let b1 = song_bar_with_score("A", make_score(10, 10, 5, 5, 100));
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::Score.compare(&b1, &b2), Ordering::Less);
    }

    #[test]
    fn compare_score_both_have_scores() {
        // s1: exscore = (10+10)*2 + 5+5 = 50, rate = 50/100 = 0.5
        let b1 = song_bar_with_score("A", make_score(10, 10, 5, 5, 100));
        // s2: exscore = (20+20)*2 + 10+10 = 100, rate = 100/100 = 1.0
        let b2 = song_bar_with_score("B", make_score(20, 20, 10, 10, 100));
        assert_eq!(BarSorter::Score.compare(&b1, &b2), Ordering::Less);
    }

    // ---- compare_duration: None score handling ----

    #[test]
    fn compare_duration_both_none_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::Duration.compare(&b1, &b2), Ordering::Equal);
    }

    #[test]
    fn compare_duration_first_none_second_has_score() {
        let score = {
            let mut sd = ScoreData::default();
            sd.timing_stats.avgjudge = 100;
            sd
        };
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_with_score("B", score);
        assert_eq!(BarSorter::Duration.compare(&b1, &b2), Ordering::Greater);
    }

    #[test]
    fn compare_duration_first_has_score_second_none() {
        let score = {
            let mut sd = ScoreData::default();
            sd.timing_stats.avgjudge = 100;
            sd
        };
        let b1 = song_bar_with_score("A", score);
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::Duration.compare(&b1, &b2), Ordering::Less);
    }

    #[test]
    fn compare_duration_both_have_scores() {
        let s1 = {
            let mut sd = ScoreData::default();
            sd.timing_stats.avgjudge = 50;
            sd
        };
        let s2 = {
            let mut sd = ScoreData::default();
            sd.timing_stats.avgjudge = 100;
            sd
        };
        let b1 = song_bar_with_score("A", s1);
        let b2 = song_bar_with_score("B", s2);
        assert_eq!(BarSorter::Duration.compare(&b1, &b2), Ordering::Less);
    }

    // ---- compare_clear: None score handling ----

    #[test]
    fn compare_clear_both_none_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::Clear.compare(&b1, &b2), Ordering::Equal);
    }

    #[test]
    fn compare_clear_first_none() {
        let score = ScoreData {
            clear: 5,
            ..Default::default()
        };
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_with_score("B", score);
        assert_eq!(BarSorter::Clear.compare(&b1, &b2), Ordering::Greater);
    }

    // ---- compare_misscount: None score handling ----

    #[test]
    fn compare_misscount_both_none_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::MissCount.compare(&b1, &b2), Ordering::Equal);
    }

    #[test]
    fn compare_misscount_first_none() {
        let score = ScoreData {
            minbp: 10,
            ..Default::default()
        };
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_with_score("B", score);
        assert_eq!(BarSorter::MissCount.compare(&b1, &b2), Ordering::Greater);
    }

    // ---- compare_lastupdate: None score handling ----

    #[test]
    fn compare_lastupdate_both_none_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::LastUpdate.compare(&b1, &b2), Ordering::Equal);
    }

    #[test]
    fn compare_lastupdate_first_none() {
        let score = ScoreData {
            date: 1000,
            ..Default::default()
        };
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_with_score("B", score);
        assert_eq!(BarSorter::LastUpdate.compare(&b1, &b2), Ordering::Greater);
    }

    // ---- compare_rival_clear: None score handling ----

    #[test]
    fn compare_rival_clear_both_no_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(
            BarSorter::RivalCompareClear.compare(&b1, &b2),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_rival_clear_first_no_score_second_has_both() {
        let score = ScoreData {
            clear: 5,
            ..Default::default()
        };
        let rival = ScoreData {
            clear: 3,
            ..Default::default()
        };
        let b1 = song_bar_no_score("A");
        let mut b2 = song_bar_with_score("B", score);
        b2.set_rival_score(Some(rival));
        assert_eq!(
            BarSorter::RivalCompareClear.compare(&b1, &b2),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_rival_clear_first_has_score_but_no_rival() {
        let b1 = song_bar_with_score("A", make_score(10, 10, 5, 5, 100));
        let score = ScoreData {
            clear: 5,
            ..Default::default()
        };
        let rival = ScoreData {
            clear: 3,
            ..Default::default()
        };
        let mut b2 = song_bar_with_score("B", score);
        b2.set_rival_score(Some(rival));
        // b1 has score but no rival, so pair1 = None
        assert_eq!(
            BarSorter::RivalCompareClear.compare(&b1, &b2),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_rival_clear_both_have_pairs() {
        let s1 = ScoreData {
            clear: 5,
            ..Default::default()
        };
        let r1 = ScoreData {
            clear: 3,
            ..Default::default()
        };
        let mut b1 = song_bar_with_score("A", s1);
        b1.set_rival_score(Some(r1));

        let s2 = ScoreData {
            clear: 4,
            ..Default::default()
        };
        let r2 = ScoreData {
            clear: 4,
            ..Default::default()
        };
        let mut b2 = song_bar_with_score("B", s2);
        b2.set_rival_score(Some(r2));

        // d1 = 5-3 = 2, d2 = 4-4 = 0, so b1 > b2
        assert_eq!(
            BarSorter::RivalCompareClear.compare(&b1, &b2),
            Ordering::Greater
        );
    }

    // ---- compare_rival_score: None score handling ----

    #[test]
    fn compare_rival_score_both_no_scores() {
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_no_score("B");
        assert_eq!(
            BarSorter::RivalCompareScore.compare(&b1, &b2),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_rival_score_first_no_score_second_has_both() {
        let mut b2 = song_bar_with_score("B", make_score(20, 20, 10, 10, 100));
        b2.set_rival_score(Some(make_score(10, 10, 5, 5, 100)));
        let b1 = song_bar_no_score("A");
        assert_eq!(
            BarSorter::RivalCompareScore.compare(&b1, &b2),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_rival_score_first_has_score_but_no_rival() {
        let b1 = song_bar_with_score("A", make_score(10, 10, 5, 5, 100));
        let mut b2 = song_bar_with_score("B", make_score(20, 20, 10, 10, 100));
        b2.set_rival_score(Some(make_score(10, 10, 5, 5, 100)));
        // b1 has score but no rival, pair1 = None
        assert_eq!(
            BarSorter::RivalCompareScore.compare(&b1, &b2),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_rival_score_first_has_score_with_zero_notes() {
        let mut b1 = song_bar_with_score("A", make_score(10, 10, 5, 5, 0));
        b1.set_rival_score(Some(make_score(5, 5, 3, 3, 100)));
        let mut b2 = song_bar_with_score("B", make_score(20, 20, 10, 10, 100));
        b2.set_rival_score(Some(make_score(10, 10, 5, 5, 100)));
        // b1 score notes = 0, so pair1 = None
        assert_eq!(
            BarSorter::RivalCompareScore.compare(&b1, &b2),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_rival_score_both_have_pairs() {
        // s1 exscore = (20+20)*2+10+10 = 100, rate = 1.0
        // r1 exscore = (10+10)*2+5+5 = 50, rate = 0.5
        // v1 = 1.0 - 0.5 = 0.5
        let mut b1 = song_bar_with_score("A", make_score(20, 20, 10, 10, 100));
        b1.set_rival_score(Some(make_score(10, 10, 5, 5, 100)));

        // s2 exscore = (10+10)*2+5+5 = 50, rate = 0.5
        // r2 exscore = (10+10)*2+5+5 = 50, rate = 0.5
        // v2 = 0.5 - 0.5 = 0.0
        let mut b2 = song_bar_with_score("B", make_score(10, 10, 5, 5, 100));
        b2.set_rival_score(Some(make_score(10, 10, 5, 5, 100)));

        // v1 (0.5) > v2 (0.0)
        assert_eq!(
            BarSorter::RivalCompareScore.compare(&b1, &b2),
            Ordering::Greater
        );
    }
}
