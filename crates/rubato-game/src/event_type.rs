/// High-level event types for music select input processing.
///
/// Translated from Java: bms.player.beatoraja.skin.property.EventFactory.EventType
/// (subset used by MusicSelector / MusicSelectInputProcessor)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    Mode,
    Sort,
    Lnmode,
    Option1p,
    Option2p,
    Optiondp,
    Gauge1p,
    Hsfix,
    Target,
    Bga,
    GaugeAutoShift,
    NotesDisplayTiming,
    NotesDisplayTimingAutoAdjust,
    Duration1p,
    Rival,
    OpenDocument,
    OpenWithExplorer,
    OpenIr,
    FavoriteSong,
    FavoriteChart,
    UpdateFolder,
    OpenDownloadSite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_eq() {
        assert_eq!(EventType::Mode, EventType::Mode);
        assert_ne!(EventType::Mode, EventType::Sort);
    }

    #[test]
    fn test_event_type_clone() {
        let e = EventType::Duration1p;
        let e2 = e;
        assert_eq!(e, e2);
    }
}
