// BarSorter
// NOTE: This is a simplified stub version (BarSorterEntry with name strings).
// The full enum-based BarSorter is in beatoraja-select/src/bar_sorter.rs.

/// Rust equivalent of beatoraja.select.BarSorter
pub struct BarSorter;

#[derive(Clone, Copy, Debug)]
pub struct BarSorterEntry {
    pub(crate) name: &'static str,
}

impl BarSorterEntry {
    pub fn name(&self) -> &str {
        self.name
    }
}

impl BarSorter {
    pub const DEFAULT_SORTER: &'static [BarSorterEntry] = &[
        BarSorterEntry { name: "TITLE" },
        BarSorterEntry { name: "ARTIST" },
        BarSorterEntry { name: "BPM" },
        BarSorterEntry { name: "LENGTH" },
        BarSorterEntry { name: "LEVEL" },
        BarSorterEntry { name: "CLEAR" },
        BarSorterEntry { name: "SCORE" },
        BarSorterEntry { name: "MISSCOUNT" },
    ];

    pub const ALL_SORTER: &'static [BarSorterEntry] = &[
        BarSorterEntry { name: "TITLE" },
        BarSorterEntry { name: "ARTIST" },
        BarSorterEntry { name: "BPM" },
        BarSorterEntry { name: "LENGTH" },
        BarSorterEntry { name: "LEVEL" },
        BarSorterEntry { name: "CLEAR" },
        BarSorterEntry { name: "SCORE" },
        BarSorterEntry { name: "MISSCOUNT" },
        BarSorterEntry { name: "DURATION" },
        BarSorterEntry { name: "LASTUPDATE" },
        BarSorterEntry {
            name: "RIVALCOMPARECLEAR",
        },
        BarSorterEntry {
            name: "RIVALCOMPARESCORE",
        },
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_sorter_default_sorter_count() {
        assert_eq!(BarSorter::DEFAULT_SORTER.len(), 8);
    }

    #[test]
    fn test_bar_sorter_default_sorter_names() {
        let names: Vec<&str> = BarSorter::DEFAULT_SORTER.iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            vec![
                "TITLE",
                "ARTIST",
                "BPM",
                "LENGTH",
                "LEVEL",
                "CLEAR",
                "SCORE",
                "MISSCOUNT"
            ]
        );
    }

    #[test]
    fn test_bar_sorter_entry_name() {
        let entry = BarSorterEntry { name: "TITLE" };
        assert_eq!(entry.name(), "TITLE");
    }

    #[test]
    fn test_bar_sorter_entry_clone() {
        let entry = BarSorterEntry { name: "CLEAR" };
        let copied = entry;
        assert_eq!(copied.name(), "CLEAR");
    }

    #[test]
    fn test_bar_sorter_all_sorter_count() {
        assert_eq!(BarSorter::ALL_SORTER.len(), 12);
    }

    #[test]
    fn test_bar_sorter_all_sorter_names() {
        let names: Vec<&str> = BarSorter::ALL_SORTER.iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            vec![
                "TITLE",
                "ARTIST",
                "BPM",
                "LENGTH",
                "LEVEL",
                "CLEAR",
                "SCORE",
                "MISSCOUNT",
                "DURATION",
                "LASTUPDATE",
                "RIVALCOMPARECLEAR",
                "RIVALCOMPARESCORE"
            ]
        );
    }

    #[test]
    fn test_bar_sorter_default_has_title_clear_score() {
        // Key entries that must be in DEFAULT_SORTER
        let names: Vec<&str> = BarSorter::DEFAULT_SORTER.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"TITLE"));
        assert!(names.contains(&"CLEAR"));
        assert!(names.contains(&"SCORE"));
    }

    #[test]
    fn test_bar_sorter_all_superset_of_common_entries() {
        // ALL_SORTER has TITLE, CLEAR, SCORE, MISSCOUNT, LEVEL from DEFAULT_SORTER
        let all_names: Vec<&str> = BarSorter::ALL_SORTER.iter().map(|e| e.name()).collect();
        for name in &["TITLE", "CLEAR", "SCORE", "MISSCOUNT", "LEVEL"] {
            assert!(all_names.contains(name), "{} should be in ALL_SORTER", name);
        }
    }

    #[test]
    fn test_bar_sorter_all_has_additional_entries() {
        let all_names: Vec<&str> = BarSorter::ALL_SORTER.iter().map(|e| e.name()).collect();
        assert!(all_names.contains(&"ARTIST"));
        assert!(all_names.contains(&"BPM"));
        assert!(all_names.contains(&"LENGTH"));
        assert!(all_names.contains(&"DURATION"));
        assert!(all_names.contains(&"RIVALCOMPARECLEAR"));
        assert!(all_names.contains(&"RIVALCOMPARESCORE"));
    }

    #[test]
    fn test_bar_sorter_entry_debug() {
        let entry = BarSorterEntry { name: "SCORE" };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("SCORE"));
    }
}
