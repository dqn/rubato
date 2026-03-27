/// BMS player mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BMSPlayerMode {
    pub mode: Mode,
    pub id: i32,
}

impl BMSPlayerMode {
    pub const PLAY: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Play,
        id: 0,
    };
    pub const PRACTICE: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Practice,
        id: 0,
    };
    pub const AUTOPLAY: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Autoplay,
        id: 0,
    };
    pub const REPLAY_1: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Replay,
        id: 0,
    };
    pub const REPLAY_2: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Replay,
        id: 1,
    };
    pub const REPLAY_3: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Replay,
        id: 2,
    };
    pub const REPLAY_4: BMSPlayerMode = BMSPlayerMode {
        mode: Mode::Replay,
        id: 3,
    };

    pub fn new(mode: Mode) -> Self {
        Self { mode, id: 0 }
    }

    pub fn new_with_id(mode: Mode, id: i32) -> Self {
        Self { mode, id }
    }

    pub fn replay_mode(index: i32) -> Option<&'static BMSPlayerMode> {
        match index {
            0 => Some(&BMSPlayerMode::REPLAY_1),
            1 => Some(&BMSPlayerMode::REPLAY_2),
            2 => Some(&BMSPlayerMode::REPLAY_3),
            3 => Some(&BMSPlayerMode::REPLAY_4),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Play,
    Practice,
    Autoplay,
    Replay,
}
