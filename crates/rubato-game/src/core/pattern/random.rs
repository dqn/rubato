use bms::model::mode::Mode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomUnit {
    None,
    Lane,
    Note,
    Player,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Random {
    Identity,
    Mirror,
    Random,
    Rotate,
    SRandom,
    Spiral,
    HRandom,
    AllScr,
    MirrorEx,
    RandomEx,
    RotateEx,
    SRandomEx,
    Cross,
    Converge,
    SRandomNoThreshold,
    RandomPlayable,
    SRandomPlayable,
    Flip,
    Battle,
}

impl Random {
    pub fn unit(&self) -> RandomUnit {
        match self {
            Random::Identity => RandomUnit::None,
            Random::Mirror => RandomUnit::Lane,
            Random::Random => RandomUnit::Lane,
            Random::Rotate => RandomUnit::Lane,
            Random::SRandom => RandomUnit::Note,
            Random::Spiral => RandomUnit::Note,
            Random::HRandom => RandomUnit::Note,
            Random::AllScr => RandomUnit::Note,
            Random::MirrorEx => RandomUnit::Lane,
            Random::RandomEx => RandomUnit::Lane,
            Random::RotateEx => RandomUnit::Lane,
            Random::SRandomEx => RandomUnit::Note,
            Random::Cross => RandomUnit::Lane,
            Random::Converge => RandomUnit::Note,
            Random::SRandomNoThreshold => RandomUnit::Note,
            Random::RandomPlayable => RandomUnit::Lane,
            Random::SRandomPlayable => RandomUnit::Note,
            Random::Flip => RandomUnit::Player,
            Random::Battle => RandomUnit::Player,
        }
    }

    pub fn is_scratch_lane_modify(&self) -> bool {
        match self {
            Random::Identity => false,
            Random::Mirror => false,
            Random::Random => false,
            Random::Rotate => false,
            Random::SRandom => false,
            Random::Spiral => false,
            Random::HRandom => false,
            Random::AllScr => true,
            Random::MirrorEx => true,
            Random::RandomEx => true,
            Random::RotateEx => true,
            Random::SRandomEx => true,
            Random::Cross => false,
            Random::Converge => true,
            Random::SRandomNoThreshold => false,
            Random::RandomPlayable => true,
            Random::SRandomPlayable => true,
            Random::Flip => true,
            Random::Battle => true,
        }
    }

    pub fn option_general() -> &'static [Random] {
        &[
            Random::Identity,
            Random::Mirror,
            Random::Random,
            Random::Rotate,
            Random::SRandom,
            Random::Spiral,
            Random::HRandom,
            Random::AllScr,
            Random::RandomEx,
            Random::SRandomEx,
        ]
    }

    pub fn option_pms() -> &'static [Random] {
        &[
            Random::Identity,
            Random::Mirror,
            Random::Random,
            Random::Rotate,
            Random::SRandomNoThreshold,
            Random::Spiral,
            Random::HRandom,
            Random::Converge,
            Random::RandomPlayable,
            Random::SRandomPlayable,
        ]
    }

    pub fn option_double() -> &'static [Random] {
        &[Random::Identity, Random::Flip]
    }

    pub fn option_single() -> &'static [Random] {
        &[Random::Identity, Random::Battle]
    }

    pub fn from_id(id: i32, mode: &Mode) -> Random {
        let randoms = match mode {
            Mode::POPN_5K | Mode::POPN_9K => Random::option_pms(),
            _ => Random::option_general(),
        };
        if id >= 0 && (id as usize) < randoms.len() {
            randoms[id as usize]
        } else {
            Random::Identity
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- RandomUnit --

    #[test]
    fn random_unit_variants_are_distinct() {
        assert_ne!(RandomUnit::None, RandomUnit::Lane);
        assert_ne!(RandomUnit::Lane, RandomUnit::Note);
        assert_ne!(RandomUnit::Note, RandomUnit::Player);
    }

    #[test]
    fn random_unit_clone_and_copy() {
        let u = RandomUnit::Lane;
        let u2 = u;
        assert_eq!(u, u2);
    }

    // -- Random::unit() --

    #[test]
    fn identity_unit_is_none() {
        assert_eq!(Random::Identity.unit(), RandomUnit::None);
    }

    #[test]
    fn lane_type_randoms_have_lane_unit() {
        let lane_randoms = [
            Random::Mirror,
            Random::Random,
            Random::Rotate,
            Random::MirrorEx,
            Random::RandomEx,
            Random::RotateEx,
            Random::Cross,
            Random::RandomPlayable,
        ];
        for r in lane_randoms {
            assert_eq!(r.unit(), RandomUnit::Lane, "{:?} should have Lane unit", r);
        }
    }

    #[test]
    fn note_type_randoms_have_note_unit() {
        let note_randoms = [
            Random::SRandom,
            Random::Spiral,
            Random::HRandom,
            Random::AllScr,
            Random::SRandomEx,
            Random::Converge,
            Random::SRandomNoThreshold,
            Random::SRandomPlayable,
        ];
        for r in note_randoms {
            assert_eq!(r.unit(), RandomUnit::Note, "{:?} should have Note unit", r);
        }
    }

    #[test]
    fn player_type_randoms_have_player_unit() {
        assert_eq!(Random::Flip.unit(), RandomUnit::Player);
        assert_eq!(Random::Battle.unit(), RandomUnit::Player);
    }

    // -- Random::is_scratch_lane_modify() --

    #[test]
    fn non_scratch_modify_variants() {
        let non_scratch = [
            Random::Identity,
            Random::Mirror,
            Random::Random,
            Random::Rotate,
            Random::SRandom,
            Random::Spiral,
            Random::HRandom,
            Random::Cross,
            Random::SRandomNoThreshold,
        ];
        for r in non_scratch {
            assert!(
                !r.is_scratch_lane_modify(),
                "{:?} should not modify scratch",
                r
            );
        }
    }

    #[test]
    fn scratch_modify_variants() {
        let scratch = [
            Random::AllScr,
            Random::MirrorEx,
            Random::RandomEx,
            Random::RotateEx,
            Random::SRandomEx,
            Random::Converge,
            Random::RandomPlayable,
            Random::SRandomPlayable,
            Random::Flip,
            Random::Battle,
        ];
        for r in scratch {
            assert!(r.is_scratch_lane_modify(), "{:?} should modify scratch", r);
        }
    }

    // -- Option lists --

    #[test]
    fn option_general_has_10_elements() {
        assert_eq!(Random::option_general().len(), 10);
    }

    #[test]
    fn option_general_starts_with_identity() {
        assert_eq!(Random::option_general()[0], Random::Identity);
    }

    #[test]
    fn option_pms_has_10_elements() {
        assert_eq!(Random::option_pms().len(), 10);
    }

    #[test]
    fn option_pms_starts_with_identity() {
        assert_eq!(Random::option_pms()[0], Random::Identity);
    }

    #[test]
    fn option_double_contents() {
        assert_eq!(Random::option_double(), &[Random::Identity, Random::Flip]);
    }

    #[test]
    fn option_single_contents() {
        assert_eq!(Random::option_single(), &[Random::Identity, Random::Battle]);
    }

    // -- random --

    #[test]
    fn get_random_id0_is_identity_for_beat7k() {
        assert_eq!(Random::from_id(0, &Mode::BEAT_7K), Random::Identity);
    }

    #[test]
    fn get_random_id1_is_mirror_for_beat7k() {
        assert_eq!(Random::from_id(1, &Mode::BEAT_7K), Random::Mirror);
    }

    #[test]
    fn get_random_out_of_range_returns_identity() {
        assert_eq!(Random::from_id(100, &Mode::BEAT_7K), Random::Identity);
    }

    #[test]
    fn get_random_negative_id_returns_identity() {
        assert_eq!(Random::from_id(-1, &Mode::BEAT_7K), Random::Identity);
    }

    #[test]
    fn get_random_uses_pms_for_popn() {
        // PMS option_pms()[4] = SRandomNoThreshold
        assert_eq!(
            Random::from_id(4, &Mode::POPN_9K),
            Random::SRandomNoThreshold
        );
        // General option_general()[4] = SRandom
        assert_eq!(Random::from_id(4, &Mode::BEAT_7K), Random::SRandom);
    }

    #[test]
    fn get_random_uses_pms_for_popn_5k() {
        assert_eq!(
            Random::from_id(4, &Mode::POPN_5K),
            Random::SRandomNoThreshold
        );
    }

    #[test]
    fn get_random_all_general_ids_valid() {
        for i in 0..10 {
            let r = Random::from_id(i, &Mode::BEAT_7K);
            assert_eq!(r, Random::option_general()[i as usize]);
        }
    }

    #[test]
    fn get_random_all_pms_ids_valid() {
        for i in 0..10 {
            let r = Random::from_id(i, &Mode::POPN_9K);
            assert_eq!(r, Random::option_pms()[i as usize]);
        }
    }
}
