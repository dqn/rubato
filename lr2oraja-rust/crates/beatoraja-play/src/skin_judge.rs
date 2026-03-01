/// Judge display skin object
pub struct SkinJudge {
    /// Judge images (7 types: PG, GR, GD, BD, PR, MS, PG+MAX)
    judge: [Option<()>; 7],
    /// Judge count numbers (7 types)
    count: [Option<()>; 7],
    /// Player index
    player: i32,
    /// Whether to shift position based on count length
    shift: bool,
    /// Currently active judge
    now_judge: Option<usize>,
    /// Currently active count
    now_count: Option<usize>,
}

impl SkinJudge {
    pub fn new(player: i32, shift: bool) -> Self {
        SkinJudge {
            judge: [None; 7],
            count: [None; 7],
            player,
            shift,
            now_judge: None,
            now_count: None,
        }
    }

    pub fn get_judge(&self, index: usize) -> bool {
        index < self.judge.len() && self.judge[index].is_some()
    }

    pub fn set_judge(&mut self, index: usize, _judge: ()) {
        if index < self.judge.len() {
            self.judge[index] = Some(());
        }
    }

    pub fn get_judge_count(&self, index: usize) -> bool {
        index < self.count.len() && self.count[index].is_some()
    }

    pub fn set_judge_count(&mut self, index: usize, _count: ()) {
        if index < self.count.len() {
            self.count[index] = Some(());
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
