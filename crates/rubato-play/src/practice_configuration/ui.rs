use super::constants::{DPRANDOM, GAUGE, GRAPHTYPESTR, RANDOM};
use super::{PracticeColor, PracticeConfiguration, PracticeDrawCommand};
use crate::gauge_property::GaugeProperty;
use bms_model::mode::Mode;

impl PracticeConfiguration {
    pub fn element_text(&self, index: usize) -> String {
        match index {
            0 => format!(
                "START TIME : {:2}:{:02}.{:1}",
                self.property.starttime / 60000,
                (self.property.starttime / 1000) % 60,
                (self.property.starttime / 100) % 10
            ),
            1 => format!(
                "END TIME : {:2}:{:02}.{:1}",
                self.property.endtime / 60000,
                (self.property.endtime / 1000) % 60,
                (self.property.endtime / 100) % 10
            ),
            2 => format!(
                "GAUGE TYPE : {}",
                GAUGE.get(self.property.gaugetype as usize).unwrap_or(&"?")
            ),
            3 => format!(
                "GAUGE CATEGORY : {}",
                self.property
                    .gaugecategory
                    .map_or("".to_string(), |g| g.name().to_string())
            ),
            4 => format!("GAUGE VALUE : {}", self.property.startgauge),
            5 => format!("JUDGERANK : {}", self.property.judgerank),
            6 => format!("TOTAL : {}", self.property.total as i32),
            7 => format!("FREQUENCY : {}", self.property.freq),
            8 => format!(
                "GRAPHTYPE : {}",
                GRAPHTYPESTR
                    .get(self.property.graphtype as usize)
                    .unwrap_or(&"?")
            ),
            9 => format!(
                "OPTION-1P : {}",
                RANDOM.get(self.property.random as usize).unwrap_or(&"?")
            ),
            10 => format!(
                "OPTION-2P : {}",
                RANDOM.get(self.property.random2 as usize).unwrap_or(&"?")
            ),
            11 => format!(
                "OPTION-DP : {}",
                DPRANDOM
                    .get(self.property.doubleop as usize)
                    .unwrap_or(&"?")
            ),
            _ => String::new(),
        }
    }

    /// Generate draw commands for practice configuration UI.
    ///
    /// Translated from: Java PracticeConfiguration.draw(Rectangle r, SkinObjectRenderer sprite, long time, MainState state)
    ///
    /// Returns a list of `PracticeDrawCommand` that the caller (skin layer)
    /// executes using SkinObjectRenderer.
    ///
    /// `region`: the BGA region rectangle (x, y, width, height)
    /// `judge_counts`: array of [total, fast, slow] for each of 6 judge types
    /// `media_loaded`: whether the audio/BGA has finished loading
    pub fn draw(
        &self,
        region: (f32, f32, f32, f32),
        judge_counts: &[(i32, i32, i32); 6],
        media_loaded: bool,
    ) -> Vec<PracticeDrawCommand> {
        let (rx, ry, rw, rh) = region;
        let x = rx + rw / 8.0;
        let y = ry + rh * 7.0 / 8.0;
        let mut commands = Vec::new();

        // Draw element labels
        for i in 0..Self::ELEMENT_COUNT {
            if self.is_element_visible(i) {
                let color = if self.cursorpos == i {
                    PracticeColor::Yellow
                } else {
                    PracticeColor::Cyan
                };
                commands.push(PracticeDrawCommand::DrawText {
                    text: self.element_text(i),
                    x,
                    y: y - 22.0 * i as f32,
                    color,
                });
            }
        }

        // "PRESS 1KEY TO PLAY" prompt
        if media_loaded {
            commands.push(PracticeDrawCommand::DrawText {
                text: "PRESS 1KEY TO PLAY".to_string(),
                x,
                y: y - 276.0,
                color: PracticeColor::Orange,
            });
        }

        // Judge count table
        let judge_labels = [
            "PGREAT :", "GREAT  :", "GOOD   :", "BAD    :", "POOR   :", "KPOOR  :",
        ];
        for i in 0..6 {
            let (total, fast, slow) = judge_counts[i];
            commands.push(PracticeDrawCommand::DrawText {
                text: format!("{} {} {} {}", judge_labels[i], total, fast, slow),
                x: x + 250.0,
                y: y - (i as f32 * 22.0),
                color: PracticeColor::White,
            });
        }

        // Graph drawing command (graph type, region, time range, frequency)
        commands.push(PracticeDrawCommand::DrawGraph {
            graph_type: self.property.graphtype,
            region: (rx, ry, rw, rh / 4.0),
            start_time: self.property.starttime,
            end_time: self.property.endtime,
            freq: self.property.freq as f32 / 100.0,
        });

        commands
    }

    /// Process input for practice mode elements
    pub fn process_input_action(&mut self, element_index: usize, inc: bool) {
        match element_index {
            0 => {
                // STARTTIME
                if let Some(ref md) = self.model_data {
                    let times = &md.timeline_times;
                    if inc {
                        if !times.is_empty()
                            && self.property.starttime as i64 + 2000
                                <= *times.last().expect("non-empty")
                        {
                            self.property.starttime += 100;
                        }
                        if self.property.starttime + 900 >= self.property.endtime {
                            self.property.endtime += 100;
                        }
                    } else if self.property.starttime >= 100 {
                        self.property.starttime -= 100;
                    }
                }
            }
            1 => {
                // ENDTIME
                if let Some(ref md) = self.model_data {
                    let times = &md.timeline_times;
                    if inc {
                        if !times.is_empty()
                            && (self.property.endtime as i64)
                                <= *times.last().expect("non-empty") + 1000
                        {
                            self.property.endtime += 100;
                        }
                    } else if self.property.endtime > self.property.starttime + 1000 {
                        self.property.endtime -= 100;
                    }
                }
            }
            2 => {
                // GAUGETYPE
                self.property.gaugetype = (self.property.gaugetype + if inc { 1 } else { 8 }) % 9;
                if let Some(ref md) = self.model_data {
                    let mode = &md.mode;
                    if (*mode == Mode::POPN_5K || *mode == Mode::POPN_9K)
                        && self.property.gaugetype >= 3
                        && self.property.startgauge > 100
                    {
                        self.property.startgauge = 100;
                    }
                }
                // Sync startgauge with the current gauge category's element values.
                if let Some(category) = self.property.gaugecategory {
                    let values = category.element_values();
                    if (self.property.gaugetype as usize) < values.len() {
                        self.property.startgauge =
                            values[self.property.gaugetype as usize].init as i32;
                    }
                }
            }
            3 => {
                // GAUGECATEGORY
                let categories = GaugeProperty::values();
                if let Some(current) = self.property.gaugecategory
                    && let Some(i) = categories.iter().position(|&c| c == current)
                {
                    let next = if inc {
                        (i + 1) % categories.len()
                    } else {
                        (i + categories.len() - 1) % categories.len()
                    };
                    self.property.gaugecategory = Some(categories[next]);
                    let values = categories[next].element_values();
                    let idx =
                        (self.property.gaugetype as usize).min(values.len().saturating_sub(1));
                    self.property.startgauge = values[idx].init as i32;
                }
            }
            4 => {
                // GAUGEVALUE
                if let Some(category) = self.property.gaugecategory {
                    let values = category.element_values();
                    let idx =
                        (self.property.gaugetype as usize).min(values.len().saturating_sub(1));
                    let max = values[idx].max as i32;
                    self.property.startgauge =
                        (self.property.startgauge + if inc { 1 } else { -1 }).clamp(1, max);
                }
            }
            5 => {
                // JUDGERANK
                self.property.judgerank =
                    (self.property.judgerank + if inc { 1 } else { -1 }).clamp(1, 400);
            }
            6 => {
                // TOTAL
                self.property.total =
                    (self.property.total + if inc { 10.0 } else { -10.0 }).clamp(20.0, 5000.0);
            }
            7 => {
                // FREQ
                self.property.freq = (self.property.freq + if inc { 5 } else { -5 }).clamp(50, 200);
            }
            8 => {
                // GRAPHTYPE
                self.property.graphtype = (self.property.graphtype + if inc { 1 } else { 2 }) % 3;
            }
            9 => {
                // OPTION1P
                let options = if let Some(ref md) = self.model_data {
                    let mode = &md.mode;
                    if *mode == Mode::POPN_5K || *mode == Mode::POPN_9K {
                        7
                    } else {
                        10
                    }
                } else {
                    10
                };
                self.property.random =
                    (self.property.random + if inc { 1 } else { options - 1 }) % options;
            }
            10 => {
                // OPTION2P
                self.property.random2 = (self.property.random2 + if inc { 1 } else { 9 }) % 10;
            }
            11 => {
                // OPTIONDP
                self.property.doubleop = (self.property.doubleop + 1) % 2;
            }
            _ => {}
        }
    }

    /// Number of practice configuration elements (indices 0..ELEMENT_COUNT).
    pub(super) const ELEMENT_COUNT: usize = 12;

    /// Process input for practice mode navigation.
    ///
    /// Translated from: Java PracticeConfiguration.processInput(BMSPlayerInputProcessor input)
    /// Navigates cursor with UP/DOWN, adjusts values with LEFT/RIGHT.
    ///
    /// `control_up_pressed`: control key UP was pressed this frame
    /// `control_down_pressed`: control key DOWN was pressed this frame
    /// `control_left_held`: control key LEFT is currently held
    /// `control_right_held`: control key RIGHT is currently held
    /// `now_millis`: current time in milliseconds (for repeat logic)
    pub fn process_input(
        &mut self,
        control_up_pressed: bool,
        control_down_pressed: bool,
        control_left_held: bool,
        control_right_held: bool,
        now_millis: i64,
    ) {
        let element_count = Self::ELEMENT_COUNT;

        // Move cursor up (skip invisible elements)
        if control_up_pressed {
            loop {
                self.cursorpos = (self.cursorpos + element_count - 1) % element_count;
                if self.is_element_visible(self.cursorpos) {
                    break;
                }
            }
        }
        // Move cursor down (skip invisible elements)
        if control_down_pressed {
            loop {
                self.cursorpos = (self.cursorpos + 1) % element_count;
                if self.is_element_visible(self.cursorpos) {
                    break;
                }
            }
        }

        // Left: decrement current element (with repeat)
        if control_left_held && (self.presscount == 0 || self.presscount + 10 < now_millis) {
            if self.presscount == 0 {
                self.presscount = now_millis + 500;
            } else {
                self.presscount = now_millis;
            }
            self.process_input_action(self.cursorpos, false);
        } else if control_right_held && (self.presscount == 0 || self.presscount + 10 < now_millis)
        {
            // Right: increment current element (with repeat)
            if self.presscount == 0 {
                self.presscount = now_millis + 500;
            } else {
                self.presscount = now_millis;
            }
            self.process_input_action(self.cursorpos, true);
        } else if !control_left_held && !control_right_held {
            self.presscount = 0;
        }
    }

    /// Get current cursor position.
    pub fn cursor_pos(&self) -> usize {
        self.cursorpos
    }

    pub fn is_element_visible(&self, index: usize) -> bool {
        match index {
            10 | 11 => {
                // OPTION2P, OPTIONDP only visible in DP mode
                if let Some(ref md) = self.model_data {
                    md.mode.player() == 2
                } else {
                    false
                }
            }
            _ => true,
        }
    }
}
