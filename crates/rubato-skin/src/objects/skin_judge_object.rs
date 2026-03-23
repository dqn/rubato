// SkinJudge wrapper for SkinObject enum (Phase 32a)
// Wraps rubato_play::SkinJudge with SkinObjectData for the skin pipeline.
// Translated from: SkinJudge.java

use crate::objects::skin_image::SkinImage;
use crate::objects::skin_number::SkinNumber;
use crate::reexports::MainState;
use crate::types::skin_object::{DestinationParams, SkinObjectData, SkinObjectRenderer};

/// SkinJudge skin object — wraps play-side SkinJudge with SkinObjectData.
///
/// Holds judge images (7 types: PG, GR, GD, BD, PR, MS, PG+MAX) and
/// combo count numbers (7 types). During draw, selects the appropriate
/// judge image and combo number based on the current judge state.
pub struct SkinJudgeObject {
    pub data: SkinObjectData,
    pub inner: rubato_play::skin_judge::SkinJudge,
    /// Judge images (7 types: PG, GR, GD, BD, PR, MS, PG+MAX)
    judge_images: [Option<SkinImage>; 7],
    /// Judge count numbers (7 types)
    judge_counts: [Option<SkinNumber>; 7],
    /// Currently active judge index (set during prepare)
    now_judge_idx: Option<usize>,
    /// Currently active count index (set during prepare)
    now_count_idx: Option<usize>,
}

impl SkinJudgeObject {
    pub fn new(player: i32, shift: bool) -> Self {
        let mut data = SkinObjectData::new();
        // Java: this.setDestination(0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 0, 0, 0, new int[0]);
        data.set_destination_with_int_timer_ops(
            &DestinationParams {
                time: 0,
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                acc: 0,
                a: 0,
                r: 255,
                g: 255,
                b: 255,
                blend: 0,
                filter: 0,
                angle: 0,
                center: 0,
                loop_val: 0,
            },
            0,
            &[],
        );
        Self {
            data,
            inner: rubato_play::skin_judge::SkinJudge::new(player, shift),
            judge_images: Default::default(),
            judge_counts: Default::default(),
            now_judge_idx: None,
            now_count_idx: None,
        }
    }

    /// Read-only access to judge images for wiring validation.
    pub(crate) fn judge_images(&self) -> &[Option<SkinImage>; 7] {
        &self.judge_images
    }

    /// Set a judge image for the given index.
    pub fn set_judge_image(&mut self, index: usize, image: SkinImage) {
        if index < self.judge_images.len() {
            self.judge_images[index] = Some(image);
        }
    }

    /// Set a judge count number for the given index.
    pub fn set_judge_count(&mut self, index: usize, count: SkinNumber) {
        if index < self.judge_counts.len() {
            self.judge_counts[index] = Some(count);
        }
    }

    /// Get mutable reference to a judge image for destination wiring.
    pub fn judge_image_mut(&mut self, index: usize) -> Option<&mut SkinImage> {
        if index < self.judge_images.len() {
            self.judge_images[index].as_mut()
        } else {
            None
        }
    }

    /// Get mutable reference to a judge count number for destination wiring.
    pub fn judge_count_mut(&mut self, index: usize) -> Option<&mut SkinNumber> {
        if index < self.judge_counts.len() {
            self.judge_counts[index].as_mut()
        } else {
            None
        }
    }

    /// Translated from: Java SkinJudge.prepare(long time, MainState state)
    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        let player = self.inner.player();
        let judgenow = state.now_judge(player) - 1;
        if judgenow < 0 {
            self.data.draw = false;
            return;
        }
        self.data.prepare(time, state);

        let judgenow = judgenow as usize;
        if judgenow >= self.judge_images.len() {
            self.data.draw = false;
            return;
        }
        let gauge_is_max = state.is_gauge_max();

        // Select judge image: if PG and gauge is max, use MAX PG (index 6) if available
        let (judge_idx, count_idx) = if judgenow == 0 && gauge_is_max {
            let ji = if self.judge_images[6].is_some() { 6 } else { 0 };
            let ci = if self.judge_counts[6].is_some() {
                Some(6)
            } else if self.judge_counts[0].is_some() {
                Some(0)
            } else {
                None
            };
            (ji, ci)
        } else {
            let ci = if judgenow < 3 { Some(judgenow) } else { None };
            (judgenow, ci)
        };

        // Prepare judge image
        if let Some(ref mut img) = self.judge_images[judge_idx] {
            img.prepare(time, state);
            if !img.data.draw {
                self.data.draw = false;
                return;
            }
        } else {
            self.data.draw = false;
            return;
        }

        self.now_judge_idx = Some(judge_idx);

        // Prepare count number
        if let Some(ci) = count_idx {
            if let Some(ref mut count) = self.judge_counts[ci] {
                let combo = state.now_combo(player);
                let judge_region = &self.judge_images[judge_idx]
                    .as_ref()
                    .expect("judge_images entry is Some")
                    .data
                    .region;
                count.prepare_with_value(time, state, combo, judge_region.x, judge_region.y);
                // Shift judge image by half the count length if shift mode is on
                if self.inner.is_shift()
                    && let Some(ref mut img) = self.judge_images[judge_idx]
                {
                    img.data.region.x -= count.length() / 2.0;
                }
                self.now_count_idx = Some(ci);
            } else {
                self.now_count_idx = None;
            }
        } else {
            self.now_count_idx = None;
        }
    }

    /// Translated from: Java SkinJudge.draw(SkinObjectRenderer sprite)
    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        // Draw count number first (behind judge image)
        if let Some(ci) = self.now_count_idx
            && let Some(ref mut count) = self.judge_counts[ci]
            && count.data.draw
        {
            count.draw(sprite);
        }
        // Draw judge image
        if let Some(ji) = self.now_judge_idx
            && let Some(ref mut img) = self.judge_images[ji]
        {
            img.draw(sprite);
        }
    }

    pub fn dispose(&mut self) {
        for i in self.judge_images.iter_mut().flatten() {
            i.dispose();
        }
        for c in self.judge_counts.iter_mut().flatten() {
            c.dispose();
        }
        self.inner.dispose();
        self.data.set_disposed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::skin_number::NumberDisplayConfig;
    use crate::reexports::{TextureRegion, Timer};

    /// Mock MainState with configurable judge/combo values for testing SkinJudgeObject.
    struct JudgeMockState {
        timer: Timer,
        now_judge: i32,
        now_combo: i32,
        gauge_max: bool,
    }

    impl JudgeMockState {
        fn new(now_judge: i32, now_combo: i32, gauge_max: bool) -> Self {
            Self {
                timer: Timer::default(),
                now_judge,
                now_combo,
                gauge_max,
            }
        }
    }

    impl rubato_types::timer_access::TimerAccess for JudgeMockState {
        fn now_time(&self) -> i64 {
            self.timer.now_time()
        }
        fn now_micro_time(&self) -> i64 {
            self.timer.now_micro_time()
        }
        fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.micro_timer(timer_id)
        }
        fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.timer(timer_id)
        }
        fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.now_time_for(timer_id)
        }
        fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
            self.timer.is_timer_on(timer_id)
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for JudgeMockState {
        fn now_judge(&self, _player: i32) -> i32 {
            self.now_judge
        }
        fn now_combo(&self, _player: i32) -> i32 {
            self.now_combo
        }
        fn is_gauge_max(&self) -> bool {
            self.gauge_max
        }
    }

    impl MainState for JudgeMockState {}

    fn make_test_image() -> SkinImage {
        let mut img = SkinImage::new_with_single(TextureRegion::new());
        // Must add a destination so prepare_region sets draw=true
        img.data.set_destination_with_int_timer_ops(
            &crate::skin_object::DestinationParams {
                time: 0,
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
                acc: 0,
                a: 255,
                r: 255,
                g: 255,
                b: 255,
                blend: 0,
                filter: 0,
                angle: 0,
                center: 0,
                loop_val: 0,
            },
            0,
            &[0],
        );
        img
    }

    fn make_test_number() -> SkinNumber {
        // Minimal SkinNumber: 1 digit, no timer, no zero-padding
        SkinNumber::new_with_int_timer(
            vec![vec![TextureRegion::new(); 10]], // 10 digit images
            None,                                 // no minus images
            0,                                    // timer
            0,                                    // cycle
            NumberDisplayConfig {
                keta: 1,
                zeropadding: 0,
                space: 0,
                align: 0,
            },
            0, // id
        )
    }

    #[test]
    fn test_new_skin_judge_object() {
        let judge = SkinJudgeObject::new(0, false);
        assert!(judge.now_judge_idx.is_none());
        assert!(judge.now_count_idx.is_none());
    }

    #[test]
    fn test_new_has_default_destination() {
        let judge = SkinJudgeObject::new(0, false);
        assert!(
            !judge.data.dst.is_empty(),
            "SkinJudgeObject must have default DST entry"
        );
        assert!(judge.data.fixr.is_some());
    }

    #[test]
    fn test_set_judge_image() {
        let mut judge = SkinJudgeObject::new(0, false);
        assert!(judge.judge_images[0].is_none());
        judge.set_judge_image(0, make_test_image());
        assert!(judge.judge_images[0].is_some());
    }

    #[test]
    fn test_set_judge_image_out_of_bounds() {
        let mut judge = SkinJudgeObject::new(0, false);
        // Should not panic, just no-op
        judge.set_judge_image(7, make_test_image());
    }

    #[test]
    fn test_set_judge_count() {
        let mut judge = SkinJudgeObject::new(0, false);
        assert!(judge.judge_counts[0].is_none());
        judge.set_judge_count(0, make_test_number());
        assert!(judge.judge_counts[0].is_some());
    }

    #[test]
    fn test_prepare_no_judge_sets_draw_false() {
        let mut judge = SkinJudgeObject::new(0, false);
        // now_judge returns 0 (no judge), so judgenow - 1 = -1 → draw = false
        let state = JudgeMockState::new(0, 0, false);
        judge.prepare(1000, &state);
        assert!(!judge.data.draw);
    }

    #[test]
    fn test_prepare_with_judge_no_image_sets_draw_false() {
        let mut judge = SkinJudgeObject::new(0, false);
        // now_judge=1 → judgenow=0 (PG), but no image set → draw = false
        let state = JudgeMockState::new(1, 5, false);
        judge.prepare(1000, &state);
        assert!(!judge.data.draw);
    }

    #[test]
    fn test_prepare_with_judge_and_image() {
        let mut judge = SkinJudgeObject::new(0, false);
        judge.set_judge_image(0, make_test_image());
        // now_judge=1 → judgenow=0 (PG), image at 0 exists
        let state = JudgeMockState::new(1, 5, false);
        judge.prepare(1000, &state);
        assert_eq!(judge.now_judge_idx, Some(0));
    }

    #[test]
    fn test_prepare_pg_max_uses_index_6() {
        let mut judge = SkinJudgeObject::new(0, false);
        judge.set_judge_image(0, make_test_image());
        judge.set_judge_image(6, make_test_image());
        // now_judge=1 → judgenow=0 (PG), gauge_max=true → use index 6
        let state = JudgeMockState::new(1, 10, true);
        judge.prepare(1000, &state);
        assert_eq!(judge.now_judge_idx, Some(6));
    }

    #[test]
    fn test_prepare_pg_max_falls_back_to_index_0() {
        let mut judge = SkinJudgeObject::new(0, false);
        judge.set_judge_image(0, make_test_image());
        // No image at index 6 → falls back to 0
        let state = JudgeMockState::new(1, 10, true);
        judge.prepare(1000, &state);
        assert_eq!(judge.now_judge_idx, Some(0));
    }

    #[test]
    fn test_draw_does_not_panic_with_images() {
        let mut judge = SkinJudgeObject::new(0, false);
        judge.set_judge_image(0, make_test_image());
        let state = JudgeMockState::new(1, 5, false);
        judge.prepare(1000, &state);

        let mut sprite = SkinObjectRenderer::new();
        judge.draw(&mut sprite);
    }

    #[test]
    fn test_draw_without_prepare_is_noop() {
        let mut judge = SkinJudgeObject::new(0, false);
        let mut sprite = SkinObjectRenderer::new();
        // no prepare → no active judge → noop
        judge.draw(&mut sprite);
    }

    #[test]
    fn test_dispose() {
        let mut judge = SkinJudgeObject::new(0, false);
        judge.set_judge_image(0, make_test_image());
        judge.set_judge_count(0, make_test_number());
        judge.dispose();
    }

    #[test]
    fn test_prepare_out_of_bounds_judge_sets_draw_false() {
        // Regression: if now_judge() returns a value > 7, the judgenow index
        // exceeds judge_images.len() (7) and would panic on array access.
        // The fix adds a bounds check that sets draw=false instead.
        let mut judge = SkinJudgeObject::new(0, false);
        judge.set_judge_image(0, make_test_image());
        // now_judge=9 -> judgenow=8, which is out of bounds for [Option<SkinImage>; 7]
        let state = JudgeMockState::new(9, 0, false);
        judge.prepare(1000, &state);
        assert!(
            !judge.data.draw,
            "out-of-bounds judgenow must set draw=false, not panic"
        );
    }
}
