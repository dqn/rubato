// BarSorter (stubs version) - moved from stubs.rs (Phase 30a)
// NOTE: This is a simplified stub version (BarSorterEntry with name strings).
// The full enum-based BarSorter is in beatoraja-select/src/bar_sorter.rs.

/// Stub for beatoraja.select.BarSorter
pub struct BarSorter;

#[derive(Clone, Debug)]
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
        BarSorterEntry { name: "CLEAR" },
        BarSorterEntry { name: "SCORE" },
        BarSorterEntry { name: "MISSCOUNT" },
        BarSorterEntry { name: "DATE" },
        BarSorterEntry { name: "LEVEL" },
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
        assert_eq!(BarSorter::DEFAULT_SORTER.len(), 6);
    }

    #[test]
    fn test_bar_sorter_default_sorter_names() {
        let names: Vec<&str> = BarSorter::DEFAULT_SORTER.iter().map(|e| e.name()).collect();
        assert_eq!(
            names,
            vec!["TITLE", "CLEAR", "SCORE", "MISSCOUNT", "DATE", "LEVEL"]
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
        let cloned = entry.clone();
        assert_eq!(cloned.name(), "CLEAR");
    }
}
