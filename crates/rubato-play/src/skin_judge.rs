/// Judge display skin object
pub struct SkinJudge {
    /// Judge images present (7 types: PG, GR, GD, BD, PR, MS, PG+MAX)
    judge: [bool; 7],
    /// Judge count numbers present (7 types)
    count: [bool; 7],
    /// Player index
    player: i32,
    /// Whether to shift position based on count length
    shift: bool,
    /// Currently active judge
    #[allow(dead_code)]
    now_judge: Option<usize>,
    /// Currently active count
    #[allow(dead_code)]
    now_count: Option<usize>,
}

impl SkinJudge {
    pub fn new(player: i32, shift: bool) -> Self {
        SkinJudge {
            judge: [false; 7],
            count: [false; 7],
            player,
            shift,
            now_judge: None,
            now_count: None,
        }
    }

    pub fn judge(&self, index: usize) -> bool {
        index < self.judge.len() && self.judge[index]
    }

    pub fn set_judge(&mut self, index: usize) {
        if index < self.judge.len() {
            self.judge[index] = true;
        }
    }

    pub fn judge_count(&self, index: usize) -> bool {
        index < self.count.len() && self.count[index]
    }

    pub fn set_judge_count(&mut self, index: usize) {
        if index < self.count.len() {
            self.count[index] = true;
        }
    }

    pub fn player(&self) -> i32 {
        self.player
    }

    pub fn is_shift(&self) -> bool {
        self.shift
    }

    pub fn prepare(&mut self, _time: i64) {
        // Prepare logic is handled by SkinJudgeObject in beatoraja-skin.
        // The skin-level wrapper accesses JudgeManager/GrooveGauge via MainState
        // trait methods (get_now_judge, get_now_combo, is_gauge_max).
    }

    pub fn draw(&self) {
        // Drawing is handled by SkinJudgeObject in beatoraja-skin.
        // This play-side struct holds state; the skin wrapper holds SkinImage/SkinNumber
        // and delegates drawing via SkinObjectRenderer (which lives in beatoraja-skin).
    }

    pub fn dispose(&mut self) {
        // no GPU resources in Rust translation
    }
}
