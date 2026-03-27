use crate::core::timer_manager::TimerManager;

use super::KeyInputProccessor;
use super::key_input_timers::{key_off_timer_id, key_on_timer_id};

/// Context passed into KeyInputProccessor::input() each frame.
///
/// Bundles the external state needed by the input processing loop,
/// avoiding the need for the processor to hold references to the parent player.
pub struct InputContext<'a> {
    /// Current time in milliseconds (from timer.getNowTime())
    pub now: i64,
    /// Key states array — true if the key is currently pressed
    pub key_states: &'a [bool],
    /// Auto-press timing array from JudgeManager (i64::MIN means not auto-pressed)
    pub auto_presstime: &'a [i64],
    /// Whether the play mode is AUTOPLAY
    pub is_autoplay: bool,
    /// Timer manager for setting key beam timers
    pub timer: &'a mut TimerManager,
}

/// Check if a key is currently active (physically pressed or auto-pressed).
fn is_key_active(key: i32, ctx: &InputContext) -> bool {
    let idx = key as usize;
    (idx < ctx.key_states.len() && ctx.key_states[idx])
        || (idx < ctx.auto_presstime.len() && ctx.auto_presstime[idx] != i64::MIN)
}

impl KeyInputProccessor {
    /// Process key input each frame: key beam flags and scratch turntable animation.
    ///
    /// Translated from Java: KeyInputProccessor.input()
    ///
    /// Returns scratch angle values indexed by scratch index.
    /// The caller should write `result[s]` to `main.getOffset(OFFSET_SCRATCHANGLE_1P + s).r`.
    pub fn input(&mut self, ctx: &mut InputContext) {
        self.update_lane_key_beams(ctx);
        self.update_scratch_animation(ctx);
        self.prevtime = ctx.now;
    }

    /// Update key beam on/off flags for each lane based on pressed keys.
    #[allow(clippy::needless_range_loop)] // Multiple parallel arrays indexed by lane
    fn update_lane_key_beams(&mut self, ctx: &mut InputContext) {
        let lane_offsets = self.lane_property.lane_skin_offset();
        let lane_keys = self.lane_property.lane_key_assign();
        let lane_scratch = self.lane_property.lane_scratch_assign();
        let lane_players = self.lane_property.lane_player();

        for lane in 0..lane_offsets.len() {
            let offset = lane_offsets[lane];
            let mut pressed = false;
            let mut scratch_changed = false;

            if !self.key_beam_stop {
                for &key in &lane_keys[lane] {
                    if is_key_active(key, ctx) {
                        pressed = true;
                        let scratch_idx = lane_scratch[lane];
                        if scratch_idx != -1 {
                            let si = scratch_idx as usize;
                            if si < self.scratch_key.len() && self.scratch_key[si] != key {
                                scratch_changed = true;
                                self.scratch_key[si] = key;
                            }
                        }
                    }
                }
            }

            let timer_on = key_on_timer_id(lane_players[lane], offset);
            let timer_off = key_off_timer_id(lane_players[lane], offset);

            if pressed {
                if (!self.is_judge_started || ctx.is_autoplay)
                    && (!ctx.timer.is_timer_on(timer_on) || scratch_changed)
                {
                    ctx.timer.set_timer_on(timer_on);
                    ctx.timer.set_timer_off(timer_off);
                }
            } else if ctx.timer.is_timer_on(timer_on) {
                ctx.timer.set_timer_on(timer_off);
                ctx.timer.set_timer_off(timer_on);
            }
        }
    }

    /// Update scratch turntable rotation animation.
    ///
    /// Faithfully ports Java's integer-based scratch animation:
    /// - Base rotation: `scratch[s] += s % 2 == 0 ? 2160 - deltatime : deltatime`
    /// - Key0 acceleration: `scratch[s] += deltatime * 2`
    /// - Key1 acceleration: `scratch[s] += 2160 - deltatime * 2`
    /// - Modulo 2160, display angle = scratch[s] / 6
    #[allow(clippy::needless_range_loop)] // Multiple parallel arrays indexed by s
    fn update_scratch_animation(&mut self, ctx: &InputContext) {
        if self.prevtime < 0 {
            return;
        }

        let deltatime = ctx.now - self.prevtime;
        let scratch_keys = self.lane_property.scratch_key_assign();

        for s in 0..self.scratch.len() {
            // Base rotation direction depends on s % 2
            self.scratch[s] += if s % 2 == 0 {
                2160 - deltatime
            } else {
                deltatime
            };

            let key0 = scratch_keys[s][1];
            let key1 = scratch_keys[s][0];
            if is_key_active(key0, ctx) {
                self.scratch[s] += deltatime * 2;
            } else if is_key_active(key1, ctx) {
                self.scratch[s] += 2160 - deltatime * 2;
            }

            self.scratch[s] %= 2160;
        }
    }

    /// Returns the current scratch display angle values (0..360 degrees).
    ///
    /// Java: `main.getOffset(OFFSET_SCRATCHANGLE_1P + s).r = scratch[s] / 6`
    /// The caller should write `angles[s]` to `main.getOffset(OFFSET_SCRATCHANGLE_1P + s).r`.
    pub fn scratch_angles(&self) -> Vec<f32> {
        self.scratch.iter().map(|&v| (v / 6) as f32).collect()
    }

    /// Key beam flag ON — called from judge synchronization.
    ///
    /// Translated from Java: KeyInputProccessor.inputKeyOn(lane)
    pub fn input_key_on(&mut self, lane: usize, timer: &mut TimerManager) {
        let lane_skin_offset = self.lane_property.lane_skin_offset();
        if lane >= lane_skin_offset.len() {
            return;
        }
        if !self.key_beam_stop {
            let offset = lane_skin_offset[lane];
            let player = self.lane_property.lane_player()[lane];
            let timer_on = key_on_timer_id(player, offset);
            let timer_off = key_off_timer_id(player, offset);
            let lane_scratch = self.lane_property.lane_scratch_assign();
            if !timer.is_timer_on(timer_on) || lane_scratch[lane] != -1 {
                timer.set_timer_on(timer_on);
                timer.set_timer_off(timer_off);
            }
        }
    }
}
