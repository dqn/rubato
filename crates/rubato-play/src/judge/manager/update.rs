use super::*;

/// Parameters for the internal judge update (update_micro).
struct UpdateMicroParams<'a> {
    pub lane_idx: usize,
    pub note_idx: usize,
    pub notes: &'a [JudgeNote],
    pub mtime: i64,
    pub judge: i32,
    pub mfast: i64,
    pub judge_vanish: bool,
    pub multi_bad: bool,
    pub gauge: &'a mut GrooveGauge,
}

impl JudgeManager {
    /// Main judge update loop (testable API).
    ///
    /// Translates the Java JudgeManager.update() method verbatim.
    /// Called once per frame with current music time, notes, key states, and gauge.
    pub fn update(
        &mut self,
        mtime: i64,
        notes: &[JudgeNote],
        key_states: &[bool],
        key_changed_times: &[i64],
        gauge: &mut GrooveGauge,
    ) {
        self.judged_lanes.clear();
        self.keysound_play_indices.clear();
        self.keysound_volume_set_indices.clear();
        let lane_count = self.lane_count;

        // --- Pass-through loop ---
        for lane_idx in 0..lane_count {
            self.lane_states[lane_idx]
                .mark((self.prevmtime + self.mjudgestart - 100000) / 1000, notes);
            let mut next_inclease = false;

            // Check if any key assigned to this lane is pressed
            let mut pressed = false;
            for &key in &self.lane_states[lane_idx].laneassign {
                if key < key_states.len() && key_states[key] {
                    pressed = true;
                    break;
                }
            }

            // Iterate notes from prevmtime to mtime
            while let Some(note_idx) = self.lane_states[lane_idx].note() {
                if notes[note_idx].time_us > mtime {
                    break;
                }
                if notes[note_idx].time_us <= self.prevmtime {
                    continue;
                }

                // HCN handling
                if notes[note_idx].is_long() {
                    let ln_type = notes[note_idx].ln_type;
                    let is_end = notes[note_idx].is_long_end();
                    if (ln_type == TYPE_UNDEFINED && self.lntype == LNTYPE_HELLCHARGENOTE)
                        || ln_type == TYPE_HELLCHARGENOTE
                    {
                        if is_end {
                            self.lane_states[lane_idx].passing = None;
                            self.lane_states[lane_idx].mpassingcount = 0;
                        } else {
                            self.lane_states[lane_idx].passing = Some(note_idx);
                        }
                    }
                } else if notes[note_idx].is_mine() && pressed {
                    // Mine note damage
                    gauge.add_value(-(notes[note_idx].damage as f32));
                    // Java line 258: keysound.play(note, keyvolume, 0)
                    self.keysound_play_indices.push(note_idx);
                }

                // Autoplay processing
                if self.autoplay {
                    if notes[note_idx].is_normal() && self.note_states[note_idx].state == 0 {
                        let Some(&first_key) = self.lane_states[lane_idx].laneassign.first() else {
                            continue;
                        };
                        self.auto_presstime[first_key] = mtime;
                        // Java line 265: keysound.play(note, keyvolume, 0)
                        self.keysound_play_indices.push(note_idx);
                        self.update_micro(UpdateMicroParams {
                            lane_idx,
                            note_idx,
                            notes,
                            mtime,
                            judge: 0,
                            mfast: 0,
                            judge_vanish: true,
                            multi_bad: false,
                            gauge,
                        });
                    }
                    if notes[note_idx].is_long() {
                        let ln_type = notes[note_idx].ln_type;
                        if notes[note_idx].is_long_start()
                            && self.note_states[note_idx].state == 0
                            && self.lane_states[lane_idx].processing.is_none()
                        {
                            let Some(&first_key) = self.lane_states[lane_idx].laneassign.first()
                            else {
                                continue;
                            };
                            self.auto_presstime[first_key] = mtime;
                            // Java line 272: keysound.play(note, keyvolume, 0)
                            self.keysound_play_indices.push(note_idx);
                            if (self.lntype == LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                                || ln_type == TYPE_LONGNOTE
                            {
                                self.lane_states[lane_idx].mpassingcount = 0;
                                let player = self.lane_states[lane_idx].player;
                                let offset = self.lane_states[lane_idx].offset;
                                if player < self.judge.len() && offset < self.judge[player].len() {
                                    self.judge[player][offset] = 8;
                                }
                            } else {
                                self.update_micro(UpdateMicroParams {
                                    lane_idx,
                                    note_idx,
                                    notes,
                                    mtime,
                                    judge: 0,
                                    mfast: 0,
                                    judge_vanish: true,
                                    multi_bad: false,
                                    gauge,
                                });
                            }
                            let pair_idx = notes[note_idx].pair_index;
                            self.lane_states[lane_idx].processing = pair_idx;
                        }
                        if notes[note_idx].is_long_end()
                            && self.note_states[note_idx].state == 0
                            && ((self.lntype != LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                                || ln_type == TYPE_CHARGENOTE
                                || ln_type == TYPE_HELLCHARGENOTE)
                        {
                            let sc = self.lane_states[lane_idx].sckey;
                            if sc >= 0 && self.lane_states[lane_idx].laneassign.len() >= 2 {
                                let first_key = self.lane_states[lane_idx].laneassign[0];
                                let second_key = self.lane_states[lane_idx].laneassign[1];
                                self.auto_presstime[first_key] = i64::MIN;
                                self.auto_presstime[second_key] = mtime;
                            }
                            self.update_micro(UpdateMicroParams {
                                lane_idx,
                                note_idx,
                                notes,
                                mtime,
                                judge: 0,
                                mfast: 0,
                                judge_vanish: true,
                                multi_bad: false,
                                gauge,
                            });
                            // Java line 292: keysound.play(state.processing, keyvolume, 0)
                            if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                                self.keysound_play_indices.push(proc_idx);
                            }
                            self.lane_states[lane_idx].processing = None;
                        }
                    }
                }
            }

            // HCN gauge increase/decrease check
            if let Some(passing_idx) = self.lane_states[lane_idx].passing {
                let pair_idx = notes[passing_idx].pair_index;
                let pair_state = pair_idx
                    .filter(|&pi| pi < self.note_states.len())
                    .map(|pi| self.note_states[pi].state)
                    .unwrap_or(0);
                if pressed || (pair_state > 0 && pair_state <= 3) || self.autoplay {
                    next_inclease = true;
                }
            }

            // Autoplay key release timing
            if self.autoplay {
                for &key in &self.lane_states[lane_idx].laneassign {
                    if key < self.auto_presstime.len()
                        && self.auto_presstime[key] != i64::MIN
                        && mtime - self.auto_presstime[key] > self.auto_minduration
                        && self.lane_states[lane_idx].processing.is_none()
                    {
                        self.auto_presstime[key] = i64::MIN;
                    }
                }
            }
            self.lane_states[lane_idx].inclease = next_inclease;
        }

        // --- HCN gauge change loop ---
        for lane_idx in 0..lane_count {
            let passing = self.lane_states[lane_idx].passing;
            if passing.is_none() {
                continue;
            }
            let passing_idx = passing.expect("passing");
            if self.note_states[passing_idx].state == 0 {
                continue;
            }

            if self.lane_states[lane_idx].inclease {
                self.lane_states[lane_idx].mpassingcount += mtime - self.prevmtime;
                if self.lane_states[lane_idx].mpassingcount > HCN_MDURATION {
                    gauge.update_with_rate(1, 0.5);
                    self.lane_states[lane_idx].mpassingcount -= HCN_MDURATION;
                }
                // Java line 333-334: if(state.passing.getPair().getState() > 3)
                //   keysound.setVolume(state.passing, keyvolume)
                let pair_idx = notes[passing_idx].pair_index;
                let pair_state = pair_idx
                    .filter(|&pi| pi < self.note_states.len())
                    .map(|pi| self.note_states[pi].state)
                    .unwrap_or(0);
                if pair_state > 3 {
                    // NaN signals "use key_volume from config" to the caller
                    self.keysound_volume_set_indices
                        .push((passing_idx, f32::NAN));
                }
            } else {
                self.lane_states[lane_idx].mpassingcount -= mtime - self.prevmtime;
                if self.lane_states[lane_idx].mpassingcount < -HCN_MDURATION {
                    gauge.update_with_rate(3, 0.5);
                    self.lane_states[lane_idx].mpassingcount += HCN_MDURATION;
                }
                // Java line 345-346: if(state.passing.getPair().getState() > 3)
                //   keysound.setVolume(state.passing, 0.0f)
                let pair_idx = notes[passing_idx].pair_index;
                let pair_state = pair_idx
                    .filter(|&pi| pi < self.note_states.len())
                    .map(|pi| self.note_states[pi].state)
                    .unwrap_or(0);
                if pair_state > 3 {
                    self.keysound_volume_set_indices.push((passing_idx, 0.0));
                }
            }
        }
        self.prevmtime = mtime;

        // --- Key press/release processing ---
        let mjudge_scratch = self.smjudge.clone();
        let mjudge_note = self.nmjudge.clone();
        for key in 0..self.keyassign.len() {
            let lane = self.keyassign[key];
            if lane == -1 {
                continue;
            }
            let lane_idx = lane as usize;
            if lane_idx >= lane_count {
                continue;
            }
            if key >= key_changed_times.len() {
                continue;
            }
            let pmtime = key_changed_times[key];
            if pmtime == i64::MIN {
                continue;
            }
            self.lane_states[lane_idx].reset();
            let sc = self.lane_states[lane_idx].sckey;

            if key < key_states.len() && key_states[key] {
                // Key pressed
                if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                    let proc_ln_type = notes[proc_idx].ln_type;
                    if (self.lntype != LNTYPE_LONGNOTE && proc_ln_type == TYPE_UNDEFINED)
                        || proc_ln_type == TYPE_CHARGENOTE
                        || proc_ln_type == TYPE_HELLCHARGENOTE
                    {
                        if sc >= 0
                            && (sc as usize) < self.sckey.len()
                            && key as i32 != self.sckey[sc as usize]
                        {
                            // BSS end processing
                            let mjudge = &self.scnendmjudge;
                            let dmtime = notes[proc_idx].time_us - pmtime;
                            let mut j = 0;
                            while j < mjudge.len()
                                && !(dmtime >= mjudge[j][0] && dmtime <= mjudge[j][1])
                            {
                                j += 1;
                            }
                            // Java line 378: keysound.play(state.processing, keyvolume, 0)
                            self.keysound_play_indices.push(proc_idx);
                            let judge = if j >= mjudge.len() {
                                // No window matched: miss-POOR.
                                6
                            } else {
                                j as i32
                            };
                            self.update_micro(UpdateMicroParams {
                                lane_idx,
                                note_idx: proc_idx,
                                notes,
                                mtime,
                                judge,
                                mfast: dmtime,
                                judge_vanish: true,
                                multi_bad: false,
                                gauge,
                            });
                            self.lane_states[lane_idx].processing = None;
                            self.lane_states[lane_idx].releasetime = i64::MIN;
                            self.lane_states[lane_idx].lnend_judge = i32::MIN;
                            self.sckey[sc as usize] = 0;
                        } else {
                            // Re-press: cancel pending release
                            self.lane_states[lane_idx].releasetime = i64::MIN;
                        }
                    } else {
                        // Re-press for LN
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                    }
                } else {
                    // No LN processing - find target note
                    let mjudge = if sc >= 0 {
                        &mjudge_scratch
                    } else {
                        &mjudge_note
                    };
                    self.lane_states[lane_idx].reset();
                    let mut tnote: Option<usize> = None;
                    let mut best_judge: i32 = 0;
                    self.multi_bad.clear();
                    self.multi_bad.set_judge(mjudge);

                    // Scan notes for best match
                    while let Some(note_idx) = self.lane_states[lane_idx].note() {
                        let dmtime = notes[note_idx].time_us - pmtime;
                        if dmtime >= self.mjudgeend {
                            break;
                        }
                        if dmtime < self.mjudgestart {
                            continue;
                        }
                        // Skip mine notes and LN end notes
                        if notes[note_idx].is_mine() || notes[note_idx].is_long_end() {
                            continue;
                        }
                        if self.note_states[note_idx].state == 0 {
                            self.multi_bad.add(note_idx, dmtime);
                        }

                        let tnote_state = tnote.map(|ti| self.note_states[ti].state).unwrap_or(-1);
                        if tnote.is_none()
                            || tnote_state != 0
                            || self.algorithm.compare_times(
                                tnote.map(|ti| notes[ti].time_us).unwrap_or(0),
                                notes[note_idx].time_us,
                                self.note_states[note_idx].state,
                                pmtime,
                                mjudge,
                            )
                        {
                            let note_state = self.note_states[note_idx].state;
                            let note_play_time = self.note_states[note_idx].play_time;
                            // MissCondition::One check
                            if self.miss == MissCondition::One
                                && (note_state != 0
                                    || (note_play_time != 0
                                        && (dmtime > mjudge[2][1] || dmtime < mjudge[2][0])))
                            {
                                continue;
                            }

                            let judge;
                            if note_state != 0 {
                                judge = if mjudge.len() > 4
                                    && dmtime >= mjudge[4][0]
                                    && dmtime <= mjudge[4][1]
                                {
                                    5
                                } else {
                                    6
                                };
                            } else if mjudge.len() > 2
                                && notes[note_idx].is_long_start()
                                && dmtime < mjudge[2][0]
                            {
                                // LR2oraja: Remove late bad for LN
                                judge = 6;
                            } else {
                                let mut j = 0;
                                while j < mjudge.len()
                                    && !(dmtime >= mjudge[j][0] && dmtime <= mjudge[j][1])
                                {
                                    j += 1;
                                }
                                judge = if j >= mjudge.len() {
                                    // No window matched: miss-POOR (6).
                                    6
                                } else if j >= 4 {
                                    j as i32 + 1
                                } else {
                                    j as i32
                                };
                            }

                            if judge < 6 {
                                if judge < 4
                                    || tnote.is_none()
                                    || (tnote.map(|ti| notes[ti].time_us).unwrap_or(0) - pmtime)
                                        .abs()
                                        > (notes[note_idx].time_us - pmtime).abs()
                                {
                                    tnote = Some(note_idx);
                                    best_judge = judge;
                                }
                            } else {
                                tnote = None;
                            }
                        }
                    }
                    self.multi_bad.filter(tnote, notes);

                    if let Some(tnote_idx) = tnote {
                        // Java line 442/473: keysound.play(tnote, keyvolume, 0)
                        // Keysound plays for both LN start and normal note hits.
                        self.keysound_play_indices.push(tnote_idx);

                        // Process multi-bad notes
                        for i in self.multi_bad.array_start..self.multi_bad.size {
                            let bad_idx = self.multi_bad.note_list[i];
                            let bad_time = self.multi_bad.time_list[i];
                            let vanish = self.judge_vanish.get(3).copied().unwrap_or(false);
                            self.update_micro(UpdateMicroParams {
                                lane_idx,
                                note_idx: bad_idx,
                                notes,
                                mtime,
                                judge: 3,
                                mfast: bad_time,
                                judge_vanish: vanish,
                                multi_bad: true,
                                gauge,
                            });
                        }

                        if notes[tnote_idx].is_long_start() {
                            // Long note press processing
                            let dmtime = notes[tnote_idx].time_us - pmtime;
                            let ln_type = notes[tnote_idx].ln_type;
                            if (self.lntype == LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                                || ln_type == TYPE_LONGNOTE
                            {
                                // LN processing
                                if self
                                    .judge_vanish
                                    .get(best_judge as usize)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    self.lane_states[lane_idx].lnstart_judge = best_judge;
                                    self.lane_states[lane_idx].lnstart_duration = dmtime;
                                    self.lane_states[lane_idx].processing =
                                        notes[tnote_idx].pair_index;
                                    self.lane_states[lane_idx].releasetime = i64::MIN;
                                    self.lane_states[lane_idx].lnend_judge = i32::MIN;
                                    if sc >= 0 && (sc as usize) < self.sckey.len() {
                                        self.sckey[sc as usize] = key as i32;
                                    }
                                    let player = self.lane_states[lane_idx].player;
                                    let offset = self.lane_states[lane_idx].offset;
                                    if player < self.judge.len()
                                        && offset < self.judge[player].len()
                                    {
                                        self.judge[player][offset] = 8;
                                    }
                                } else {
                                    self.update_micro(UpdateMicroParams {
                                        lane_idx,
                                        note_idx: tnote_idx,
                                        notes,
                                        mtime,
                                        judge: best_judge,
                                        mfast: dmtime,
                                        judge_vanish: false,
                                        multi_bad: false,
                                        gauge,
                                    });
                                }
                            } else {
                                // CN, HCN press processing
                                if self
                                    .judge_vanish
                                    .get(best_judge as usize)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    self.lane_states[lane_idx].processing =
                                        notes[tnote_idx].pair_index;
                                    self.lane_states[lane_idx].releasetime = i64::MIN;
                                    self.lane_states[lane_idx].lnend_judge = i32::MIN;
                                    if sc >= 0 && (sc as usize) < self.sckey.len() {
                                        self.sckey[sc as usize] = key as i32;
                                    }
                                }
                                let vanish = self
                                    .judge_vanish
                                    .get(best_judge as usize)
                                    .copied()
                                    .unwrap_or(false);
                                self.update_micro(UpdateMicroParams {
                                    lane_idx,
                                    note_idx: tnote_idx,
                                    notes,
                                    mtime,
                                    judge: best_judge,
                                    mfast: dmtime,
                                    judge_vanish: vanish,
                                    multi_bad: false,
                                    gauge,
                                });
                            }
                        } else {
                            // Normal note processing
                            let dmtime = notes[tnote_idx].time_us - pmtime;
                            let vanish = self
                                .judge_vanish
                                .get(best_judge as usize)
                                .copied()
                                .unwrap_or(false);
                            self.update_micro(UpdateMicroParams {
                                lane_idx,
                                note_idx: tnote_idx,
                                notes,
                                mtime,
                                judge: best_judge,
                                mfast: dmtime,
                                judge_vanish: vanish,
                                multi_bad: false,
                                gauge,
                            });
                        }
                    } else {
                        // Empty POOR - no matching note
                        let player = self.lane_states[lane_idx].player;
                        let offset = self.lane_states[lane_idx].offset;
                        if player < self.judge.len() && offset < self.judge[player].len() {
                            self.judge[player][offset] = 0;
                        }
                    }
                }
                // Record this lane for key beam timer triggering by the caller.
                // Java calls main.getKeyinput().inputKeyOn(lane) here -- once per
                // manual key press, unconditionally (even on empty POOR).  Autoplay
                // and miss-POOR paths must NOT trigger key beams.
                self.judged_lanes.push(lane_idx);
            } else {
                // Key released
                if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                    let proc_ln_type = notes[proc_idx].ln_type;
                    let mjudge = if sc >= 0 {
                        &self.scnendmjudge
                    } else {
                        &self.cnendmjudge
                    };
                    let dmtime = notes[proc_idx].time_us - pmtime;
                    let mut judge = 0;
                    while judge < mjudge.len() as i32
                        && !(dmtime >= mjudge[judge as usize][0]
                            && dmtime <= mjudge[judge as usize][1])
                    {
                        judge += 1;
                    }

                    if (self.lntype != LNTYPE_LONGNOTE && proc_ln_type == TYPE_UNDEFINED)
                        || proc_ln_type == TYPE_CHARGENOTE
                        || proc_ln_type == TYPE_HELLCHARGENOTE
                    {
                        // CN, HCN release
                        let mut release = true;
                        if sc >= 0 && (sc as usize) < self.sckey.len() {
                            if judge != 4 || key as i32 != self.sckey[sc as usize] {
                                release = false;
                            } else {
                                self.sckey[sc as usize] = 0;
                            }
                        }
                        if release {
                            if judge >= 3 && dmtime > 0 {
                                self.lane_states[lane_idx].releasetime = mtime;
                                self.lane_states[lane_idx].lnend_judge = judge;
                            } else {
                                // Java line 537: keysound.play(state.processing, keyvolume, 0)
                                self.keysound_play_indices.push(proc_idx);
                                self.update_micro(UpdateMicroParams {
                                    lane_idx,
                                    note_idx: proc_idx,
                                    notes,
                                    mtime,
                                    judge,
                                    mfast: dmtime,
                                    judge_vanish: true,
                                    multi_bad: false,
                                    gauge,
                                });
                                self.lane_states[lane_idx].processing = None;
                                self.lane_states[lane_idx].releasetime = i64::MIN;
                                self.lane_states[lane_idx].lnend_judge = i32::MIN;
                            }
                        }
                    } else {
                        // LN release
                        let mut release = true;
                        if sc >= 0 && (sc as usize) < self.sckey.len() {
                            if key as i32 != self.sckey[sc as usize] {
                                release = false;
                            } else {
                                self.sckey[sc as usize] = 0;
                            }
                        }
                        if release {
                            judge = judge.max(self.lane_states[lane_idx].lnstart_judge);
                            let mut dmtime = dmtime;
                            if self.lane_states[lane_idx].lnstart_duration.abs() > dmtime.abs() {
                                dmtime = self.lane_states[lane_idx].lnstart_duration;
                            }
                            if judge >= 3 && dmtime > 0 {
                                self.lane_states[lane_idx].releasetime = mtime;
                                self.lane_states[lane_idx].lnend_judge = 3;
                            } else {
                                // Get pair of processing note for LN
                                let pair_of_proc = notes[proc_idx].pair_index;
                                let judge_note = pair_of_proc.unwrap_or(proc_idx);
                                // Java line 564: keysound.play(state.processing, keyvolume, 0)
                                // Note: Java plays state.processing (the LN end note),
                                // not the pair used for updateMicro.
                                self.keysound_play_indices.push(proc_idx);
                                self.update_micro(UpdateMicroParams {
                                    lane_idx,
                                    note_idx: judge_note,
                                    notes,
                                    mtime,
                                    judge: judge.min(3),
                                    mfast: dmtime,
                                    judge_vanish: true,
                                    multi_bad: false,
                                    gauge,
                                });
                                self.lane_states[lane_idx].processing = None;
                                self.lane_states[lane_idx].releasetime = i64::MIN;
                                self.lane_states[lane_idx].lnend_judge = i32::MIN;
                            }
                        }
                    }
                }
            }
        }

        // --- Miss POOR and LN end processing ---
        let mjudge_scratch = self.smjudge.clone();
        let mjudge_note = self.nmjudge.clone();
        for lane_idx in 0..lane_count {
            let sc = self.lane_states[lane_idx].sckey;
            let mjudge = if sc >= 0 {
                &mjudge_scratch
            } else {
                &mjudge_note
            };
            let releasemargin = if sc >= 0 {
                self.sreleasemargin
            } else {
                self.nreleasemargin
            };

            // LN end processing
            if let Some(proc_idx) = self.lane_states[lane_idx].processing {
                let proc_ln_type = notes[proc_idx].ln_type;
                if (self.lntype == LNTYPE_LONGNOTE && proc_ln_type == TYPE_UNDEFINED)
                    || proc_ln_type == TYPE_LONGNOTE
                {
                    if self.lane_states[lane_idx].releasetime != i64::MIN
                        && self.lane_states[lane_idx].releasetime + releasemargin <= mtime
                    {
                        let pair_of_proc = notes[proc_idx].pair_index.unwrap_or(proc_idx);
                        let lnend_judge = self.lane_states[lane_idx].lnend_judge;
                        let release_dmtime =
                            notes[proc_idx].time_us - self.lane_states[lane_idx].releasetime;
                        // Java line 586: keysound.setVolume(state.processing.getPair(), 0.0f)
                        self.keysound_volume_set_indices.push((pair_of_proc, 0.0));
                        self.update_micro(UpdateMicroParams {
                            lane_idx,
                            note_idx: pair_of_proc,
                            notes,
                            mtime,
                            judge: lnend_judge,
                            mfast: release_dmtime,
                            judge_vanish: true,
                            multi_bad: false,
                            gauge,
                        });
                        // Java line 588: keysound.play(state.processing, keyvolume, 0)
                        self.keysound_play_indices.push(proc_idx);
                        self.lane_states[lane_idx].processing = None;
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                        self.lane_states[lane_idx].lnend_judge = i32::MIN;
                    } else if notes[proc_idx].time_us < mtime {
                        let pair_of_proc = notes[proc_idx].pair_index.unwrap_or(proc_idx);
                        let lnstart_judge = self.lane_states[lane_idx].lnstart_judge;
                        let lnstart_duration = self.lane_states[lane_idx].lnstart_duration;
                        self.update_micro(UpdateMicroParams {
                            lane_idx,
                            note_idx: pair_of_proc,
                            notes,
                            mtime,
                            judge: lnstart_judge,
                            mfast: lnstart_duration,
                            judge_vanish: true,
                            multi_bad: false,
                            gauge,
                        });
                        // Java line 594: keysound.play(state.processing, keyvolume, 0)
                        self.keysound_play_indices.push(proc_idx);
                        self.lane_states[lane_idx].processing = None;
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                        self.lane_states[lane_idx].lnend_judge = i32::MIN;
                    }
                } else if self.lane_states[lane_idx].releasetime != i64::MIN
                    && self.lane_states[lane_idx].releasetime + releasemargin <= mtime
                {
                    let lnend_judge = self.lane_states[lane_idx].lnend_judge;
                    let release_dmtime =
                        notes[proc_idx].time_us - self.lane_states[lane_idx].releasetime;
                    // Java line 601-602: if(state.lnendJudge >= 3)
                    //   keysound.setVolume(state.processing.getPair(), 0.0f)
                    if lnend_judge >= 3 {
                        let pair_of_proc = notes[proc_idx].pair_index.unwrap_or(proc_idx);
                        self.keysound_volume_set_indices.push((pair_of_proc, 0.0));
                    }
                    self.update_micro(UpdateMicroParams {
                        lane_idx,
                        note_idx: proc_idx,
                        notes,
                        mtime,
                        judge: lnend_judge,
                        mfast: release_dmtime,
                        judge_vanish: true,
                        multi_bad: false,
                        gauge,
                    });
                    // Java line 605: keysound.play(state.processing, keyvolume, 0)
                    self.keysound_play_indices.push(proc_idx);
                    self.lane_states[lane_idx].processing = None;
                    self.lane_states[lane_idx].releasetime = i64::MIN;
                    self.lane_states[lane_idx].lnend_judge = i32::MIN;
                }
            }

            // Miss POOR detection
            self.lane_states[lane_idx].reset();
            while let Some(note_idx) = self.lane_states[lane_idx].note() {
                let miss_window = mjudge.get(3).map(|r| r[0]).unwrap_or(i64::MIN);
                if notes[note_idx].time_us >= mtime + miss_window {
                    break;
                }
                let mjud = notes[note_idx].time_us - mtime;

                if notes[note_idx].is_normal() && self.note_states[note_idx].state == 0 {
                    self.update_micro(UpdateMicroParams {
                        lane_idx,
                        note_idx,
                        notes,
                        mtime,
                        judge: 4,
                        mfast: mjud,
                        judge_vanish: true,
                        multi_bad: false,
                        gauge,
                    });
                } else if notes[note_idx].is_long() {
                    let ln_type = notes[note_idx].ln_type;
                    if notes[note_idx].is_long_start() && self.note_states[note_idx].state == 0 {
                        if (self.lntype != LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                            || ln_type == TYPE_CHARGENOTE
                            || ln_type == TYPE_HELLCHARGENOTE
                        {
                            self.update_micro(UpdateMicroParams {
                                lane_idx,
                                note_idx,
                                notes,
                                mtime,
                                judge: 4,
                                mfast: mjud,
                                judge_vanish: true,
                                multi_bad: false,
                                gauge,
                            });
                            if let Some(pair_idx) = notes[note_idx].pair_index {
                                self.update_micro(UpdateMicroParams {
                                    lane_idx,
                                    note_idx: pair_idx,
                                    notes,
                                    mtime,
                                    judge: 4,
                                    mfast: mjud,
                                    judge_vanish: true,
                                    multi_bad: false,
                                    gauge,
                                });
                            }
                        }
                        if ((self.lntype == LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                            || ln_type == TYPE_LONGNOTE)
                            && self.lane_states[lane_idx].processing != notes[note_idx].pair_index
                        {
                            self.update_micro(UpdateMicroParams {
                                lane_idx,
                                note_idx,
                                notes,
                                mtime,
                                judge: 4,
                                mfast: mjud,
                                judge_vanish: true,
                                multi_bad: false,
                                gauge,
                            });
                        }
                    }
                    if ((self.lntype != LNTYPE_LONGNOTE && ln_type == TYPE_UNDEFINED)
                        || ln_type == TYPE_CHARGENOTE
                        || ln_type == TYPE_HELLCHARGENOTE)
                        && notes[note_idx].is_long_end()
                        && self.note_states[note_idx].state == 0
                    {
                        self.update_micro(UpdateMicroParams {
                            lane_idx,
                            note_idx,
                            notes,
                            mtime,
                            judge: 4,
                            mfast: mjud,
                            judge_vanish: true,
                            multi_bad: false,
                            gauge,
                        });
                        self.lane_states[lane_idx].processing = None;
                        self.lane_states[lane_idx].releasetime = i64::MIN;
                        self.lane_states[lane_idx].lnend_judge = i32::MIN;
                        let sc = self.lane_states[lane_idx].sckey;
                        if sc >= 0 && (sc as usize) < self.sckey.len() {
                            self.sckey[sc as usize] = 0;
                        }
                    }
                }
            }
        }
    }

    /// Internal judge update: records score, combo, ghost, and gauge changes.
    fn update_micro(&mut self, p: UpdateMicroParams<'_>) {
        let lane_idx = p.lane_idx;
        let note_idx = p.note_idx;
        let notes = p.notes;
        let judge = p.judge;
        let mfast = p.mfast;
        let mtime = p.mtime;
        let judge_vanish = p.judge_vanish;
        let multi_bad = p.multi_bad;
        let gauge = p.gauge;
        let _ = notes; // used for type info if needed in future
        // Key beam tracking (judged_lanes) is now handled at the caller site
        // (manual key press block) rather than here, so autoplay and miss-POOR
        // paths do not produce spurious key beam events.
        if note_idx >= self.note_states.len() {
            return;
        }
        if judge_vanish {
            if (self.score.passnotes as usize) < self.ghost.len() {
                self.ghost[self.score.passnotes as usize] = judge;
            }
            self.note_states[note_idx].state = judge + 1;
            self.score.passnotes += 1;
        }
        if self.miss == MissCondition::One
            && judge == 4
            && self.note_states[note_idx].play_time != 0
        {
            return;
        }
        // Guard: only write play_time for first judgments or vanishing notes.
        // Re-hits (judge=5, judge_vanish=false) must not overwrite the original timing.
        if self.note_states[note_idx].state == 0 || judge_vanish {
            self.note_states[note_idx].play_time = mfast;
        }
        self.score.add_judge_count(judge, mfast >= 0, 1);

        if judge < 4 && !self.recent_judges.is_empty() {
            self.recent_judges_index = (self.recent_judges_index + 1) % self.recent_judges.len();
            self.recent_judges[self.recent_judges_index] = mfast / 1000;
            self.micro_recent_judges[self.recent_judges_index] = mfast;
        }

        if (judge as usize) < self.combocond.len() && self.combocond[judge as usize] && judge < 5 {
            self.combo += 1;
            self.score.maxcombo = self.score.maxcombo.max(self.combo);
            self.coursecombo += 1;
            self.coursemaxcombo = self.coursemaxcombo.max(self.coursecombo);
        }
        if (judge as usize) < self.combocond.len() && !self.combocond[judge as usize] {
            self.combo = 0;
            self.coursecombo = 0;
        }

        // Populate per-player judge display fields (Java JudgeManager.update2 lines 724-731)
        if lane_idx < self.lane_states.len() {
            let player = self.lane_states[lane_idx].player;
            let offset = self.lane_states[lane_idx].offset;
            if player < self.judgenow.len() {
                self.judgenow[player] = judge + 1;
            }
            if player < self.judgecombo.len() {
                self.judgecombo[player] = self.coursecombo;
            }
            if player < self.judgefast.len() {
                self.judgefast[player] = mfast / 1000;
            }
            if player < self.mjudgefast.len() {
                self.mjudgefast[player] = mfast;
            }
            self.judged_visual_events.push(JudgeVisualEvent {
                player,
                offset,
                judge,
            });
        }

        if judge != 4 {
            let player = self.lane_states[lane_idx].player;
            let offset = self.lane_states[lane_idx].offset;
            if player < self.judge.len() && offset < self.judge[player].len() {
                self.judge[player][offset] = if judge == 0 {
                    1
                } else {
                    judge * 2 + if mfast >= 0 { 0 } else { 1 }
                };
            }
        }

        if !multi_bad {
            gauge.update(judge);
            // Record judge event for the caller to trigger update_judge() side effects.
            // multi_bad notes are not individually reported (Java behavior).
            self.judged_events.push((judge, mtime));
        }

        // Timing auto-adjust (Java JudgeManager lines 717-725)
        // Java: if (judge <= 2 && mfast >= -150000 && mfast <= 150000) {
        //     player.setJudgetiming(player.getJudgetiming()
        //         - (int)((mfast >= 0 ? mfast + 15000 : mfast - 15000) / 30000));
        // }
        if self.auto_adjust_enabled
            && self.is_play_or_practice
            && judge <= 2
            && (-150_000..=150_000).contains(&mfast)
        {
            let biased = if mfast >= 0 {
                mfast + 15_000
            } else {
                mfast - 15_000
            };
            self.judgetiming_delta -= (biased / 30_000) as i32;
        }
    }
}
