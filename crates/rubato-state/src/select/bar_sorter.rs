use std::cmp::Ordering;

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
        match name {
            "TITLE" => Some(BarSorter::Title),
            "ARTIST" => Some(BarSorter::Artist),
            "BPM" => Some(BarSorter::Bpm),
            "LENGTH" => Some(BarSorter::Length),
            "LEVEL" => Some(BarSorter::Level),
            "CLEAR" => Some(BarSorter::Clear),
            "SCORE" => Some(BarSorter::Score),
            "MISSCOUNT" => Some(BarSorter::MissCount),
            "DURATION" => Some(BarSorter::Duration),
            "LASTUPDATE" => Some(BarSorter::LastUpdate),
            "RIVALCOMPARE_CLEAR" => Some(BarSorter::RivalCompareClear),
            "RIVALCOMPARE_SCORE" => Some(BarSorter::RivalCompareScore),
            _ => None,
        }
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
                .get_title()
                .to_lowercase()
                .cmp(&s2.song.get_title().to_lowercase());
            if title_cmp == Ordering::Equal {
                return s1.song.get_difficulty().cmp(&s2.song.get_difficulty());
            }
            return title_cmp;
        }

        o1.get_title()
            .to_lowercase()
            .cmp(&o2.get_title().to_lowercase())
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
            .get_artist()
            .to_lowercase()
            .cmp(&s2.song.get_artist().to_lowercase())
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
        s1.song.get_maxbpm().cmp(&s2.song.get_maxbpm())
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
        s1.song.get_length().cmp(&s2.song.get_length())
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
        let level_sort = s1.song.get_level().cmp(&s2.song.get_level());
        if level_sort == Ordering::Equal {
            return s1.song.get_difficulty().cmp(&s2.song.get_difficulty());
        }
        level_sort
    }

    fn compare_clear(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        match (o1.get_score(), o2.get_score()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(s1), Some(s2)) => s1.get_clear().cmp(&s2.get_clear()),
        }
    }

    fn compare_score(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let (s1, s2) = match (o1.get_score(), o2.get_score()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Greater,
            (_, None) => return Ordering::Less,
            (Some(s1), Some(s2)) => (s1, s2),
        };
        let n1 = s1.get_notes();
        let n2 = s2.get_notes();
        if n1 == 0 && n2 == 0 {
            return Ordering::Equal;
        }
        if n1 == 0 {
            return Ordering::Greater;
        }
        if n2 == 0 {
            return Ordering::Less;
        }
        let r1 = s1.get_exscore() as f32 / n1 as f32;
        let r2 = s2.get_exscore() as f32 / n2 as f32;
        r1.partial_cmp(&r2).unwrap_or(Ordering::Equal)
    }

    fn compare_misscount(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        match (o1.get_score(), o2.get_score()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(s1), Some(s2)) => s1.get_minbp().cmp(&s2.get_minbp()),
        }
    }

    fn compare_duration(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let (s1, s2) = match (o1.get_score(), o2.get_score()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Greater,
            (_, None) => return Ordering::Less,
            (Some(s1), Some(s2)) => (s1, s2),
        };
        let exists1 = s1.get_avgjudge() != i64::MAX;
        let exists2 = s2.get_avgjudge() != i64::MAX;
        if !exists1 && !exists2 {
            return Ordering::Equal;
        }
        if !exists1 {
            return Ordering::Greater;
        }
        if !exists2 {
            return Ordering::Less;
        }
        let d = s1.get_avgjudge() - s2.get_avgjudge();
        d.cmp(&0)
    }

    fn compare_lastupdate(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        match (o1.get_score(), o2.get_score()) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(s1), Some(s2)) => {
                let d = s1.get_date() - s2.get_date();
                d.cmp(&0)
            }
        }
    }

    fn compare_rival_clear(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let pair1 = match (o1.get_score(), o1.get_rival_score()) {
            (Some(s), Some(r)) => Some((s, r)),
            _ => None,
        };
        let pair2 = match (o2.get_score(), o2.get_rival_score()) {
            (Some(s), Some(r)) => Some((s, r)),
            _ => None,
        };
        match (pair1, pair2) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some((s1, r1)), Some((s2, r2))) => {
                let d1 = s1.get_clear() - r1.get_clear();
                let d2 = s2.get_clear() - r2.get_clear();
                d1.cmp(&d2)
            }
        }
    }

    fn compare_rival_score(o1: &Bar, o2: &Bar) -> Ordering {
        if o1.as_song_bar().is_none() || o2.as_song_bar().is_none() {
            return Self::compare_title(o1, o2);
        }
        let pair1 = match (o1.get_score(), o1.get_rival_score()) {
            (Some(s), Some(r)) if s.get_notes() > 0 && r.get_notes() > 0 => Some((s, r)),
            _ => None,
        };
        let pair2 = match (o2.get_score(), o2.get_rival_score()) {
            (Some(s), Some(r)) if s.get_notes() > 0 && r.get_notes() > 0 => Some((s, r)),
            _ => None,
        };
        match (pair1, pair2) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some((s1, r1)), Some((s2, r2))) => {
                let v1 = s1.get_exscore() as f32 / s1.get_notes() as f32
                    - r1.get_exscore() as f32 / r1.get_notes() as f32;
                let v2 = s2.get_exscore() as f32 / s2.get_notes() as f32
                    - r2.get_exscore() as f32 / r2.get_notes() as f32;
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
        sd.set_title(title.to_string());
        sd.set_path("/dummy".to_string());
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
        let mut s = ScoreData::default();
        s.epg = epg;
        s.lpg = lpg;
        s.egr = egr;
        s.lgr = lgr;
        s.notes = notes;
        s
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
        let mut score = ScoreData::default();
        score.avgjudge = 100;
        let b1 = song_bar_no_score("A");
        let b2 = song_bar_with_score("B", score);
        assert_eq!(BarSorter::Duration.compare(&b1, &b2), Ordering::Greater);
    }

    #[test]
    fn compare_duration_first_has_score_second_none() {
        let mut score = ScoreData::default();
        score.avgjudge = 100;
        let b1 = song_bar_with_score("A", score);
        let b2 = song_bar_no_score("B");
        assert_eq!(BarSorter::Duration.compare(&b1, &b2), Ordering::Less);
    }

    #[test]
    fn compare_duration_both_have_scores() {
        let mut s1 = ScoreData::default();
        s1.avgjudge = 50;
        let mut s2 = ScoreData::default();
        s2.avgjudge = 100;
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
        let mut score = ScoreData::default();
        score.clear = 5;
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
        let mut score = ScoreData::default();
        score.minbp = 10;
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
        let mut score = ScoreData::default();
        score.date = 1000;
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
        let mut score = ScoreData::default();
        score.clear = 5;
        let mut rival = ScoreData::default();
        rival.clear = 3;
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
        let mut score = ScoreData::default();
        score.clear = 5;
        let mut rival = ScoreData::default();
        rival.clear = 3;
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
        let mut s1 = ScoreData::default();
        s1.clear = 5;
        let mut r1 = ScoreData::default();
        r1.clear = 3;
        let mut b1 = song_bar_with_score("A", s1);
        b1.set_rival_score(Some(r1));

        let mut s2 = ScoreData::default();
        s2.clear = 4;
        let mut r2 = ScoreData::default();
        r2.clear = 4;
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
