pub use rubato_types::clear_type::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_type_id_mapping() {
        assert_eq!(ClearType::NoPlay.id(), 0);
        assert_eq!(ClearType::Failed.id(), 1);
        assert_eq!(ClearType::AssistEasy.id(), 2);
        assert_eq!(ClearType::LightAssistEasy.id(), 3);
        assert_eq!(ClearType::Easy.id(), 4);
        assert_eq!(ClearType::Normal.id(), 5);
        assert_eq!(ClearType::Hard.id(), 6);
        assert_eq!(ClearType::ExHard.id(), 7);
        assert_eq!(ClearType::FullCombo.id(), 8);
        assert_eq!(ClearType::Perfect.id(), 9);
        assert_eq!(ClearType::Max.id(), 10);
    }

    #[test]
    fn test_clear_type_values_count() {
        let values = ClearType::values();
        assert_eq!(values.len(), 11);
    }

    #[test]
    fn test_clear_type_values_ids_are_sequential() {
        let values = ClearType::values();
        for (i, ct) in values.iter().enumerate() {
            assert_eq!(ct.id(), i as i32, "ClearType at index {} has wrong id", i);
        }
    }

    #[test]
    fn test_get_clear_type_by_id_valid() {
        assert_eq!(ClearType::clear_type_by_id(0), ClearType::NoPlay);
        assert_eq!(ClearType::clear_type_by_id(1), ClearType::Failed);
        assert_eq!(ClearType::clear_type_by_id(5), ClearType::Normal);
        assert_eq!(ClearType::clear_type_by_id(8), ClearType::FullCombo);
        assert_eq!(ClearType::clear_type_by_id(10), ClearType::Max);
    }

    #[test]
    fn test_get_clear_type_by_id_invalid_returns_noplay() {
        assert_eq!(ClearType::clear_type_by_id(-1), ClearType::NoPlay);
        assert_eq!(ClearType::clear_type_by_id(11), ClearType::NoPlay);
        assert_eq!(ClearType::clear_type_by_id(100), ClearType::NoPlay);
    }

    #[test]
    fn test_gaugetype_for_key_clear_types() {
        let empty: &[i32] = &[];
        assert_eq!(ClearType::NoPlay.gaugetype(), empty);
        assert_eq!(ClearType::Failed.gaugetype(), empty);
        assert_eq!(ClearType::AssistEasy.gaugetype(), empty);
        assert_eq!(ClearType::LightAssistEasy.gaugetype(), &[0i32]);
        assert_eq!(ClearType::Easy.gaugetype(), &[1i32]);
        assert_eq!(ClearType::Normal.gaugetype(), &[2i32, 6]);
        assert_eq!(ClearType::Hard.gaugetype(), &[3i32, 7]);
        assert_eq!(ClearType::ExHard.gaugetype(), &[4i32, 8]);
        assert_eq!(ClearType::FullCombo.gaugetype(), &[5i32]);
        assert_eq!(ClearType::Perfect.gaugetype(), empty);
        assert_eq!(ClearType::Max.gaugetype(), empty);
    }

    #[test]
    fn test_get_clear_type_by_gauge_valid() {
        assert_eq!(
            ClearType::clear_type_by_gauge(0),
            Some(ClearType::LightAssistEasy)
        );
        assert_eq!(ClearType::clear_type_by_gauge(1), Some(ClearType::Easy));
        assert_eq!(ClearType::clear_type_by_gauge(2), Some(ClearType::Normal));
        assert_eq!(ClearType::clear_type_by_gauge(3), Some(ClearType::Hard));
        assert_eq!(ClearType::clear_type_by_gauge(4), Some(ClearType::ExHard));
        assert_eq!(
            ClearType::clear_type_by_gauge(5),
            Some(ClearType::FullCombo)
        );
        // Alternate gauge types
        assert_eq!(ClearType::clear_type_by_gauge(6), Some(ClearType::Normal));
        assert_eq!(ClearType::clear_type_by_gauge(7), Some(ClearType::Hard));
        assert_eq!(ClearType::clear_type_by_gauge(8), Some(ClearType::ExHard));
    }

    #[test]
    fn test_get_clear_type_by_gauge_invalid() {
        assert_eq!(ClearType::clear_type_by_gauge(-1), None);
        assert_eq!(ClearType::clear_type_by_gauge(9), None);
        assert_eq!(ClearType::clear_type_by_gauge(100), None);
    }

    #[test]
    fn test_clear_type_serde_roundtrip() {
        for ct in ClearType::values() {
            let json = serde_json::to_string(ct).unwrap();
            let deserialized: ClearType = serde_json::from_str(&json).unwrap();
            assert_eq!(*ct, deserialized);
        }
    }

    #[test]
    fn test_clear_type_clone_and_copy() {
        let ct = ClearType::Hard;
        let cloned = ct;
        let copied = ct;
        assert_eq!(ct, cloned);
        assert_eq!(ct, copied);
    }
}
