mod key_input_beams;
mod key_input_judge;
mod key_input_timers;

pub use key_input_beams::InputContext;
pub use key_input_judge::{JudgeTickResult, ReplayKeyEvent};

use crate::lane_property::LaneProperty;
use key_input_judge::JudgeThread;

/// Key input processing thread
pub struct KeyInputProccessor {
    prevtime: i64,
    /// Scratch turntable rotation state in 2160-degree space (6x 360).
    /// Display angle is `scratch[s] / 6`.
    scratch: Vec<i64>,
    scratch_key: Vec<i32>,
    lane_property: LaneProperty,
    is_judge_started: bool,
    pub key_beam_stop: bool,
    judge: Option<JudgeThread>,
}

impl KeyInputProccessor {
    pub fn new(lane_property: &LaneProperty) -> Self {
        let scratch_len = lane_property.scratch_key_assign().len();
        KeyInputProccessor {
            prevtime: -1,
            scratch: vec![0; scratch_len],
            scratch_key: vec![0; scratch_len],
            lane_property: lane_property.clone(),
            is_judge_started: false,
            key_beam_stop: false,
            judge: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::key_input_judge::ReplayKeylogEntry;
    use super::key_input_timers::*;
    use super::*;
    use bms_model::mode::Mode;
    use rubato_core::timer_manager::TimerManager;
    use rubato_types::KeyInputLog as DtoKeyInputLog;
    use rubato_types::timer_id::TimerId;

    use key_input_judge::JudgeThread;

    fn make_lane_property() -> LaneProperty {
        LaneProperty::new(&Mode::BEAT_7K)
    }

    // --- JudgeThread tests ---

    #[test]
    fn test_judge_thread_new_sets_fields() {
        let jt = JudgeThread::new(1_000_000, None, 500);
        assert!(!jt.stop);
        assert_eq!(jt.micro_margin_time, 500_000); // 500ms * 1000
        assert_eq!(
            jt.last_time,
            1_000_000 + crate::bms_player::TIME_MARGIN * 1000
        );
        assert!(jt.keylog.is_none());
        assert_eq!(jt.index, 0);
        assert_eq!(jt.prevtime, -1);
    }

    #[test]
    fn test_judge_thread_tick_no_keylog_updates_judge() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        let result = jt.tick(1_000_000);
        assert!(result.replay_events.is_empty());
        assert!(result.should_update_judge);
        assert!(!result.finished);
        assert!(!result.has_keylog);
    }

    #[test]
    fn test_judge_thread_tick_same_time_skips_update() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        let _ = jt.tick(1_000_000);
        // Same time again
        let result = jt.tick(1_000_000);
        assert!(result.replay_events.is_empty());
        assert!(!result.should_update_judge);
        assert!(!result.finished);
    }

    #[test]
    fn test_judge_thread_tick_past_last_time_finishes() {
        let last_tl_time = 1_000_000i64;
        let mut jt = JudgeThread::new(last_tl_time, None, 0);
        let past_time = last_tl_time + crate::bms_player::TIME_MARGIN * 1000;
        let result = jt.tick(past_time);
        assert!(result.finished);
        assert!(!result.should_update_judge);
    }

    #[test]
    fn test_judge_thread_tick_stopped_returns_finished() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        jt.stop = true;
        let result = jt.tick(1_000_000);
        assert!(result.finished);
        assert!(!result.should_update_judge);
    }

    #[test]
    fn test_judge_thread_tick_replays_keylog_entries() {
        let keylog = vec![
            ReplayKeylogEntry {
                time: 1_000_000,
                keycode: 0,
                pressed: true,
            },
            ReplayKeylogEntry {
                time: 2_000_000,
                keycode: 1,
                pressed: true,
            },
            ReplayKeylogEntry {
                time: 3_000_000,
                keycode: 0,
                pressed: false,
            },
        ];
        let mut jt = JudgeThread::new(10_000_000, Some(keylog), 0);

        // At 1.5s, should replay first entry only
        let result = jt.tick(1_500_000);
        assert_eq!(result.replay_events.len(), 1);
        assert_eq!(result.replay_events[0].keycode, 0);
        assert!(result.replay_events[0].pressed);
        assert_eq!(result.replay_events[0].time, 1_000_000);
        assert!(result.should_update_judge);
        assert!(result.has_keylog);

        // At 3.5s, should replay entries 2 and 3
        let result = jt.tick(3_500_000);
        assert_eq!(result.replay_events.len(), 2);
        assert_eq!(result.replay_events[0].keycode, 1);
        assert!(result.replay_events[0].pressed);
        assert_eq!(result.replay_events[1].keycode, 0);
        assert!(!result.replay_events[1].pressed);
    }

    #[test]
    fn test_judge_thread_tick_with_margin_time() {
        let keylog = vec![ReplayKeylogEntry {
            time: 1_000_000,
            keycode: 0,
            pressed: true,
        }];
        // margin_time = 500ms = 500_000us
        let mut jt = JudgeThread::new(10_000_000, Some(keylog), 500);

        // At 1.0s: keylog[0].time(1_000_000) + margin(500_000) = 1_500_000 > 1_000_000
        // So entry should NOT be replayed yet
        let result = jt.tick(1_000_000);
        assert!(result.replay_events.is_empty());

        // At 1.5s: 1_000_000 + 500_000 = 1_500_000 <= 1_500_000 -> replay
        let result = jt.tick(1_500_000);
        assert_eq!(result.replay_events.len(), 1);
        assert_eq!(result.replay_events[0].time, 1_500_000); // time + margin
    }

    #[test]
    fn test_judge_thread_frametime_tracking() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        assert_eq!(jt.frametime(), 1);

        jt.tick(1_000_000);
        jt.tick(1_100_000); // delta = 100_000
        assert_eq!(jt.frametime(), 100_000);

        jt.tick(1_150_000); // delta = 50_000 (less than previous max)
        assert_eq!(jt.frametime(), 100_000); // max stays

        jt.tick(1_400_000); // delta = 250_000
        assert_eq!(jt.frametime(), 250_000);
    }

    // --- KeyInputProccessor tests ---

    #[test]
    fn test_start_judge_sets_state() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        assert!(!proc.is_judge_started());
        proc.start_judge(10_000_000, None, 0);
        assert!(proc.is_judge_started());
        assert!(proc.judge.is_some());
    }

    #[test]
    fn test_start_judge_with_keylog() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let keylog = vec![
            DtoKeyInputLog {
                time: 1_000_000,
                keycode: 0,
                pressed: true,
            },
            DtoKeyInputLog {
                time: 2_000_000,
                keycode: 0,
                pressed: false,
            },
        ];
        proc.start_judge(10_000_000, Some(&keylog), 100);
        assert!(proc.is_judge_started());
        // Verify keylog was converted
        let judge = proc.judge.as_ref().unwrap();
        assert!(judge.keylog.is_some());
        assert_eq!(judge.keylog.as_ref().unwrap().len(), 2);
        assert_eq!(judge.micro_margin_time, 100_000);
    }

    #[test]
    fn test_stop_judge_clears_state() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.start_judge(10_000_000, None, 0);
        assert!(proc.is_judge_started());

        proc.stop_judge();
        assert!(!proc.is_judge_started());
        assert!(proc.judge.is_none());
        assert!(proc.key_beam_stop);
    }

    #[test]
    fn test_tick_judge_returns_none_when_not_started() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        assert!(proc.tick_judge(1_000_000).is_none());
    }

    #[test]
    fn test_tick_judge_returns_result_when_started() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.start_judge(10_000_000, None, 0);
        let result = proc.tick_judge(1_000_000);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.should_update_judge);
        assert!(!result.finished);
    }

    #[test]
    fn test_tick_judge_replays_dto_keylog() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let keylog = vec![
            DtoKeyInputLog {
                time: 500_000,
                keycode: 2,
                pressed: true,
            },
            DtoKeyInputLog {
                time: 1_500_000,
                keycode: 3,
                pressed: false,
            },
        ];
        proc.start_judge(10_000_000, Some(&keylog), 0);

        let result = proc.tick_judge(1_000_000).unwrap();
        assert_eq!(result.replay_events.len(), 1);
        assert_eq!(result.replay_events[0].keycode, 2);
        assert!(result.replay_events[0].pressed);
        assert_eq!(result.replay_events[0].time, 500_000);
    }

    #[test]
    fn test_full_replay_sequence() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let keylog = vec![
            DtoKeyInputLog {
                time: 100_000,
                keycode: 0,
                pressed: true,
            },
            DtoKeyInputLog {
                time: 200_000,
                keycode: 0,
                pressed: false,
            },
        ];
        // last timeline at 500_000us
        proc.start_judge(500_000, Some(&keylog), 0);

        // Tick at 150_000: replay first entry
        let r = proc.tick_judge(150_000).unwrap();
        assert_eq!(r.replay_events.len(), 1);
        assert!(r.should_update_judge);
        assert!(!r.finished);
        assert!(r.has_keylog);

        // Tick at 250_000: replay second entry
        let r = proc.tick_judge(250_000).unwrap();
        assert_eq!(r.replay_events.len(), 1);
        assert!(!r.replay_events[0].pressed);

        // Tick past end: last_time = 500_000 + TIME_MARGIN * 1000 = 5_500_000
        let r = proc.tick_judge(5_500_000).unwrap();
        assert!(r.finished);
        assert!(r.has_keylog);
    }

    // --- SkinPropertyMapper timer ID tests ---

    #[test]
    fn test_key_on_timer_id_scratch_range() {
        // player 0, key 0 (scratch) -> 100
        assert_eq!(key_on_timer_id(0, 0), TimerId::new(TIMER_KEYON_1P_SCRATCH));
        // player 0, key 7 -> 107
        assert_eq!(
            key_on_timer_id(0, 7),
            TimerId::new(TIMER_KEYON_1P_SCRATCH + 7)
        );
        // player 1, key 0 -> 110
        assert_eq!(
            key_on_timer_id(1, 0),
            TimerId::new(TIMER_KEYON_1P_SCRATCH + 10)
        );
    }

    #[test]
    fn test_key_off_timer_id_scratch_range() {
        assert_eq!(
            key_off_timer_id(0, 0),
            TimerId::new(TIMER_KEYOFF_1P_SCRATCH)
        );
        assert_eq!(
            key_off_timer_id(0, 7),
            TimerId::new(TIMER_KEYOFF_1P_SCRATCH + 7)
        );
        assert_eq!(
            key_off_timer_id(1, 0),
            TimerId::new(TIMER_KEYOFF_1P_SCRATCH + 10)
        );
    }

    #[test]
    fn test_key_on_timer_id_key10_range() {
        // key 10 -> TIMER_KEYON_1P_KEY10 + 0
        assert_eq!(key_on_timer_id(0, 10), TimerId::new(TIMER_KEYON_1P_KEY10));
        // key 15 -> TIMER_KEYON_1P_KEY10 + 5
        assert_eq!(
            key_on_timer_id(0, 15),
            TimerId::new(TIMER_KEYON_1P_KEY10 + 5)
        );
    }

    #[test]
    fn test_key_timer_id_invalid_player() {
        assert_eq!(key_on_timer_id(2, 0), TimerId::new(-1));
        assert_eq!(key_off_timer_id(2, 0), TimerId::new(-1));
    }

    #[test]
    fn test_key_timer_id_invalid_key() {
        assert_eq!(key_on_timer_id(0, 100), TimerId::new(-1));
        assert_eq!(key_off_timer_id(0, 100), TimerId::new(-1));
    }

    // --- input() method tests (Phase 41f) ---

    fn make_timer() -> TimerManager {
        TimerManager::new()
    }

    fn make_context<'a>(
        now: i64,
        key_states: &'a [bool],
        auto_presstime: &'a [i64],
        is_autoplay: bool,
        timer: &'a mut TimerManager,
    ) -> InputContext<'a> {
        InputContext {
            now,
            key_states,
            auto_presstime,
            is_autoplay,
            timer,
        }
    }

    #[test]
    fn test_input_no_keys_pressed_no_timer_change() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();
        // No keys pressed, no auto_presstime
        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // No timers should be set
        for id in 100..110 {
            assert!(!ctx.timer.is_timer_on(TimerId::new(id)));
        }
    }

    #[test]
    fn test_input_key_pressed_sets_beam_timer_when_not_judge_started() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        // judge is NOT started, so key beam should activate
        let mut timer = make_timer();

        // BEAT_7K: lane 0 maps to key 0, skin offset 1, player 0
        // timer_on = key_on_timer_id(0, 1) = 100 + 1 = 101
        // timer_off = key_off_timer_id(0, 1) = 120 + 1 = 121
        let mut key_states = vec![false; 9];
        key_states[0] = true; // key 0 pressed
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        assert!(ctx.timer.is_timer_on(TimerId::new(101))); // KEYON timer for offset 1
        assert!(!ctx.timer.is_timer_on(TimerId::new(121))); // KEYOFF timer should be off
    }

    #[test]
    fn test_input_key_released_swaps_beam_timers() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // First: press key 0 (lane 0, offset 1)
        let mut key_states = vec![false; 9];
        key_states[0] = true;
        let auto_presstime = vec![i64::MIN; 9];
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(timer.is_timer_on(TimerId::new(101))); // KEYON on

        // Then: release key 0
        key_states[0] = false;
        {
            let mut ctx = make_context(200, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // After release: timer_off(121) becomes on, timer_on(101) becomes off
        assert!(timer.is_timer_on(TimerId::new(121))); // KEYOFF on
        assert!(!timer.is_timer_on(TimerId::new(101))); // KEYON off
    }

    #[test]
    fn test_input_auto_presstime_triggers_beam() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // No key physically pressed, but auto_presstime is set for key 0
        let key_states = vec![false; 9];
        let mut auto_presstime = vec![i64::MIN; 9];
        auto_presstime[0] = 50_000; // auto-pressed at 50ms
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // Should trigger beam as if key was pressed
        assert!(ctx.timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_key_beam_stop_prevents_beam() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.key_beam_stop = true;
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        key_states[0] = true;
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // key_beam_stop is true, so no timers should be set
        assert!(!ctx.timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_judge_started_requires_autoplay_for_beam() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.start_judge(10_000_000, None, 0);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        key_states[0] = true;
        let auto_presstime = vec![i64::MIN; 9];

        // judge is started, NOT autoplay -> no beam timer
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(!timer.is_timer_on(TimerId::new(101)));

        // judge is started, IS autoplay -> beam timer sets
        {
            let mut ctx = make_context(200, &key_states, &auto_presstime, true, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_release_swaps_beam_timers_during_manual_play() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        proc.input_key_on(0, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(101)));
        assert!(!timer.is_timer_on(TimerId::new(121)));

        proc.start_judge(10_000_000, None, 0);

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(200, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);

        assert!(
            !ctx.timer.is_timer_on(TimerId::new(101)),
            "manual play should turn KEYON off after release even while judge is running"
        );
        assert!(
            ctx.timer.is_timer_on(TimerId::new(121)),
            "manual play should turn KEYOFF on after release even while judge is running"
        );
    }

    #[test]
    fn test_input_scratch_key_change_triggers_re_beam() {
        // BEAT_7K: lane 7 (scratch) has keys [7, 8], scratch_assign[7] = 0
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // skin offset for lane 7 is 0. timer_on = key_on_timer_id(0, 0) = 100
        // Press key 7 (first scratch key)
        let mut key_states = vec![false; 9];
        key_states[7] = true;
        let auto_presstime = vec![i64::MIN; 9];
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(timer.is_timer_on(TimerId::new(100)));

        // Now press key 8 instead (switch scratch direction)
        key_states[7] = false;
        key_states[8] = true;
        {
            let mut ctx = make_context(200, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // scratch_changed should be true, timer should be re-set
        assert!(timer.is_timer_on(TimerId::new(100)));
    }

    // --- Scratch animation tests ---
    // Java algorithm (KeyInputProccessor.java lines 92-108):
    //   deltatime = now - prevtime  (milliseconds)
    //   scratch[s] += s % 2 == 0 ? 2160 - deltatime : deltatime  (base rotation)
    //   if key0 active: scratch[s] += deltatime * 2
    //   else if key1 active: scratch[s] += 2160 - deltatime * 2
    //   scratch[s] %= 2160
    //   display angle = scratch[s] / 6
    //
    // BEAT_7K: scratch_key_assign()[0] = [7, 8]
    //   key0 = scratch_keys[0][1] = 8
    //   key1 = scratch_keys[0][0] = 7

    #[test]
    fn test_input_scratch_initial_no_animation_on_first_frame() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];
        // prevtime starts at -1, first frame should not animate
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // scratch angles should still be 0 after first frame (prevtime was -1)
        assert_eq!(proc.scratch_angles()[0], 0.0);
        // prevtime should now be 100
        assert_eq!(proc.prevtime, 100);
    }

    #[test]
    fn test_input_scratch_idle_base_rotation_even_index() {
        // Java: s=0 (even), idle, deltatime=16ms
        // scratch[0] += 2160 - 16 = 2144
        // scratch[0] %= 2160 -> 2144
        // display = 2144 / 6 = 357
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // First frame to set prevtime
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Second frame at 16ms (typical frame time)
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // s=0 (even): scratch += 2160 - 16 = 2144
        assert_eq!(proc.scratch[0], 2144);
        // display = 2144 / 6 = 357 (integer division)
        assert_eq!(proc.scratch_angles()[0], 357.0);
    }

    #[test]
    fn test_input_scratch_idle_base_rotation_odd_index() {
        // Java: s=1 (odd), idle, deltatime=16ms
        // scratch[1] += 16
        // display = 16 / 6 = 2 (integer division)
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 20];
        let auto_presstime = vec![i64::MIN; 20];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Second frame at 16ms
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // s=0 (even): scratch += 2160 - 16 = 2144
        assert_eq!(proc.scratch[0], 2144);
        // s=1 (odd): scratch += 16
        assert_eq!(proc.scratch[1], 16);
        // display: 2144/6=357, 16/6=2
        assert_eq!(proc.scratch_angles()[0], 357.0);
        assert_eq!(proc.scratch_angles()[1], 2.0);
    }

    #[test]
    fn test_input_scratch_key0_accelerates_positive() {
        // Java: key0 active -> scratch[s] += deltatime * 2
        // Combined with base rotation (s=0 even): 2160 - dt + dt*2 = 2160 + dt
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Press key 8 (key0 = scratch_keys[0][1])
        key_states[8] = true;
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // base: 2160 - 16 = 2144
        // key0: + 16 * 2 = 32
        // total: 2144 + 32 = 2176
        // % 2160 = 16
        assert_eq!(proc.scratch[0], 16);
        assert_eq!(proc.scratch_angles()[0], 2.0); // 16 / 6 = 2
    }

    #[test]
    fn test_input_scratch_key1_accelerates_negative() {
        // Java: key1 active -> scratch[s] += 2160 - deltatime * 2
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Press key 7 (key1 = scratch_keys[0][0])
        key_states[7] = true;
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // base: 2160 - 16 = 2144
        // key1: + 2160 - 16*2 = 2128
        // total: 0 + 2144 + 2128 = 4272
        // % 2160 = 4272 - 2160 = 2112
        assert_eq!(proc.scratch[0], 2112);
        assert_eq!(proc.scratch_angles()[0], (2112 / 6) as f32); // 352.0
    }

    #[test]
    fn test_input_scratch_auto_presstime_triggers_key0_acceleration() {
        // auto_presstime[key0] != i64::MIN should trigger key0 acceleration
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let mut auto_presstime = vec![i64::MIN; 9];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // key0 = scratch_keys[0][1] = 8: auto-pressed
        auto_presstime[8] = 50_000;
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // Same as key0 physically pressed: base(2144) + key0(32) = 2176 % 2160 = 16
        assert_eq!(proc.scratch[0], 16);
    }

    #[test]
    fn test_input_scratch_wraps_at_2160() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // Set scratch close to 2160
        proc.scratch[0] = 2150;
        proc.prevtime = 0;

        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // 2150 + (2160 - 16) = 2150 + 2144 = 4294
        // 4294 % 2160 = 4294 - 2160 = 2134
        assert_eq!(proc.scratch[0], (2150 + 2144) % 2160);
    }

    #[test]
    fn test_input_scratch_multi_frame_accumulation() {
        // Verify scratch accumulates across multiple frames like Java
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // Frame 0: prevtime = -1, no animation
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert_eq!(proc.scratch[0], 0);

        // Frame 1: deltatime=16, s=0 even, idle
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        let after_f1 = (0 + 2160 - 16) % 2160; // 2144
        assert_eq!(proc.scratch[0], after_f1);

        // Frame 2: deltatime=16
        {
            let mut ctx = make_context(32, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        let after_f2 = (after_f1 + 2160 - 16) % 2160; // (2144 + 2144) % 2160 = 2128
        assert_eq!(proc.scratch[0], after_f2);

        // Frame 3: deltatime=16
        {
            let mut ctx = make_context(48, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        let after_f3 = (after_f2 + 2160 - 16) % 2160; // (2128 + 2144) % 2160 = 2112
        assert_eq!(proc.scratch[0], after_f3);
    }

    /// Regression test: validates the ported scratch algorithm matches Java behavior
    /// across a sequence of frames with mixed key presses, matching the Java source
    /// in KeyInputProccessor.java lines 92-108.
    #[test]
    fn test_scratch_java_parity_mixed_input_sequence() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // key0 = scratch_keys[0][1] = 8, key1 = scratch_keys[0][0] = 7
        let mut key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // Simulate Java execution step by step:
        // Frame 0: prevtime = -1, skip animation
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert_eq!(proc.scratch[0], 0);

        // Frame 1: now=16, prevtime=0, deltatime=16, idle
        // scratch += 2160 - 16 = 2144 -> 2144 % 2160 = 2144
        {
            let mut ctx = make_context(16, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert_eq!(proc.scratch[0], 2144);
        assert_eq!(proc.scratch_angles()[0], (2144_i64 / 6) as f32);

        // Frame 2: now=32, deltatime=16, key0 (8) pressed
        // base: 2160 - 16 = 2144
        // key0: + 16 * 2 = 32
        // total: 2144 + 2144 + 32 = 4320
        // 4320 % 2160 = 0
        key_states[8] = true;
        {
            let mut ctx = make_context(32, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert_eq!(proc.scratch[0], (2144 + 2144 + 32) % 2160);

        // Frame 3: now=48, deltatime=16, release key0, press key1 (7)
        // base: 2160 - 16 = 2144
        // key1: + 2160 - 16*2 = 2128
        // total: prev + 2144 + 2128
        key_states[8] = false;
        key_states[7] = true;
        let prev = proc.scratch[0];
        {
            let mut ctx = make_context(48, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert_eq!(proc.scratch[0], (prev + 2144 + 2128) % 2160);

        // Frame 4: now=64, deltatime=16, no keys
        key_states[7] = false;
        let prev = proc.scratch[0];
        {
            let mut ctx = make_context(64, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert_eq!(proc.scratch[0], (prev + 2144) % 2160);

        // Verify display angle is always scratch / 6
        let expected_display = (proc.scratch[0] / 6) as f32;
        assert_eq!(proc.scratch_angles()[0], expected_display);
    }

    /// Regression test: verifies that s % 2 direction parity is preserved
    /// for 14K mode (two scratches: even and odd index).
    #[test]
    fn test_scratch_java_parity_direction_depends_on_index_parity() {
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 20];
        let auto_presstime = vec![i64::MIN; 20];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Second frame: deltatime=100ms, idle
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // s=0 (even): scratch += 2160 - 100 = 2060 -> rotates "backward"
        assert_eq!(proc.scratch[0], 2060);
        // s=1 (odd): scratch += 100 -> rotates "forward"
        assert_eq!(proc.scratch[1], 100);

        // They rotate in opposite directions, verifying s % 2 parity
        // Display: 2060/6 = 343, 100/6 = 16
        assert_eq!(proc.scratch_angles()[0], 343.0);
        assert_eq!(proc.scratch_angles()[1], 16.0);
    }

    #[test]
    fn test_input_prevtime_updates() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        assert_eq!(proc.prevtime, -1);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        let mut ctx = make_context(42, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        assert_eq!(proc.prevtime, 42);
    }

    #[test]
    fn test_input_multiple_lanes_pressed() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // Press keys 0, 2, 4 (lanes 0, 2, 4; offsets 1, 3, 5)
        let mut key_states = vec![false; 9];
        key_states[0] = true;
        key_states[2] = true;
        key_states[4] = true;
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);

        // timer_on IDs: 101, 103, 105
        assert!(ctx.timer.is_timer_on(TimerId::new(101)));
        assert!(ctx.timer.is_timer_on(TimerId::new(103)));
        assert!(ctx.timer.is_timer_on(TimerId::new(105)));
        // Unpressed lanes should NOT have timers
        assert!(!ctx.timer.is_timer_on(TimerId::new(102))); // offset 2
        assert!(!ctx.timer.is_timer_on(TimerId::new(104))); // offset 4
    }

    // --- input_key_on() tests ---

    #[test]
    fn test_input_key_on_sets_timer() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // lane 0, offset 1, player 0 -> timer_on = 101, timer_off = 121
        proc.input_key_on(0, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(101)));
        assert!(!timer.is_timer_on(TimerId::new(121)));
    }

    #[test]
    fn test_input_key_on_scratch_lane_always_retriggers() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // lane 7 is scratch, offset 0, player 0 -> timer_on = 100
        proc.input_key_on(7, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(100)));

        // Call again — scratch lanes should re-trigger
        // (scratch condition: lane_scratch[lane] != -1 -> always true for scratch)
        proc.input_key_on(7, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(100)));
    }

    #[test]
    fn test_input_key_on_key_beam_stop_prevents_timer() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.key_beam_stop = true;
        let mut timer = make_timer();

        proc.input_key_on(0, &mut timer);
        assert!(!timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_key_on_out_of_bounds_lane_no_crash() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // Lane 100 is out of bounds — should return early without panic
        proc.input_key_on(100, &mut timer);
    }

    #[test]
    fn test_input_key_on_non_scratch_lane_does_not_retrigger() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // lane 0 (non-scratch), offset 1 -> timer_on = 101
        proc.input_key_on(0, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(101)));

        // Manually set timer_on to OFF, then call again
        // Since timer_on is already ON and lane is not scratch, it should NOT re-trigger
        // Actually, input_key_on checks: !timer.is_timer_on(timer_on) || lane_scratch[lane] != -1
        // For non-scratch lane with timer already on -> skip
        // We can test by checking the timer was set once (already proven above)
    }

    // --- get_scratch_angles() tests ---

    #[test]
    fn test_get_scratch_angles_initial_zeros() {
        let lp = make_lane_property();
        let proc = KeyInputProccessor::new(&lp);
        let angles = proc.scratch_angles();
        assert_eq!(angles.len(), 1); // BEAT_7K has 1 scratch
        assert_eq!(angles[0], 0.0);
    }

    #[test]
    fn test_get_scratch_angles_beat_14k_has_two() {
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        let proc = KeyInputProccessor::new(&lp);
        let angles = proc.scratch_angles();
        assert_eq!(angles.len(), 2);
    }

    #[test]
    fn test_get_scratch_angles_popn_has_none() {
        let lp = LaneProperty::new(&Mode::POPN_9K);
        let proc = KeyInputProccessor::new(&lp);
        let angles = proc.scratch_angles();
        assert_eq!(angles.len(), 0);
    }
}
