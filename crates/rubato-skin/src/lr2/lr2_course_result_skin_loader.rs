use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader::{self};
use crate::safe_div_f32;
use crate::skin_gauge_graph_object::SkinGaugeGraphObject;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::skin_object::DestinationParams;
use crate::stubs::{Rectangle, Resolution};

/// LR2 course result skin loader
///
/// Translated from LR2CourseResultSkinLoader.java (87 lines)
/// Loads LR2 course result skins with gauge chart and note chart elements.
///
/// Course result skin loader state
pub struct LR2CourseResultSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,
    pub gauge: Rectangle,
    pub gaugeobj: Option<SkinGaugeGraphObject>,
    pub noteobj: Option<SkinNoteDistributionGraph>,
    /// Rank time (ms) parsed from STARTINPUT str[2]; controls score animation timing.
    pub ranktime: i32,
}

impl LR2CourseResultSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            gauge: Rectangle::default(),
            gaugeobj: None,
            noteobj: None,
            ranktime: 0,
        }
    }

    /// Process course result-specific commands
    pub fn process_course_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "STARTINPUT" => {
                // Delegate input time to base CSV loader (same as non-result skins)
                self.csv.process_csv_command(cmd, str_parts, None);
                // Parse ranktime (course-result-skin-specific; controls score animation timing)
                if str_parts.len() > 2 {
                    self.ranktime = str_parts[2].trim().parse().unwrap_or(0);
                }
            }
            "SRC_GAUGECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let mut obj = SkinGaugeGraphObject::new_default();
                obj.line_width = values[6];
                obj.delay = values[14] - values[13];
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                self.gaugeobj = Some(obj);
            }
            "DST_GAUGECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                if let Some(ref mut obj) = self.gaugeobj {
                    let dstw = safe_div_f32(self.csv.dst.width, self.csv.src.width);
                    let dsth = safe_div_f32(self.csv.dst.height, self.csv.src.height);
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    obj.data.set_destination_with_int_timer_ops(
                        &DestinationParams {
                            time: values[2] as i64,
                            x: self.gauge.x * dstw,
                            y: self.gauge.y * dsth,
                            w: self.gauge.width * dstw,
                            h: self.gauge.height * dsth,
                            acc: values[7],
                            a: values[8],
                            r: values[9],
                            g: values[10],
                            b: values[11],
                            blend: values[12],
                            filter: values[13],
                            angle: values[14],
                            center: values[15],
                            loop_val: values[16],
                        },
                        values[17],
                        &offsets,
                    );
                }
            }
            "SRC_NOTECHART_1P" => {
                lr2_skin_loader::process_src_notechart(
                    str_parts,
                    &mut self.gauge,
                    &mut self.noteobj,
                );
            }
            "DST_NOTECHART_1P" => {
                lr2_skin_loader::process_dst_notechart(
                    str_parts,
                    self.csv.src.height,
                    self.csv.dst.width,
                    self.csv.dst.height,
                    self.csv.src.width,
                    &mut self.gauge,
                    &mut self.noteobj,
                );
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> LR2CourseResultSkinLoaderState {
        let src = Resolution {
            width: 640.0,
            height: 480.0,
        };
        let dst = Resolution {
            width: 1280.0,
            height: 960.0,
        };
        LR2CourseResultSkinLoaderState::new(src, dst, false, String::new())
    }

    fn str_vec(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn startinput_delegates_input_and_parses_ranktime() {
        let mut state = make_state();
        state.process_course_command("STARTINPUT", &str_vec(&["STARTINPUT", "1000", "500"]));
        assert_eq!(state.csv.skin_input, Some(1000));
        assert_eq!(state.ranktime, 500);
    }

    #[test]
    fn startinput_missing_ranktime_defaults_to_zero() {
        let mut state = make_state();
        state.process_course_command("STARTINPUT", &str_vec(&["STARTINPUT", "1000"]));
        assert_eq!(state.csv.skin_input, Some(1000));
        assert_eq!(state.ranktime, 0);
    }

    #[test]
    fn startinput_invalid_ranktime_defaults_to_zero() {
        let mut state = make_state();
        state.process_course_command("STARTINPUT", &str_vec(&["STARTINPUT", "1000", "abc"]));
        assert_eq!(state.csv.skin_input, Some(1000));
        assert_eq!(state.ranktime, 0);
    }

    #[test]
    fn startinput_negative_ranktime() {
        let mut state = make_state();
        state.process_course_command("STARTINPUT", &str_vec(&["STARTINPUT", "1000", "-200"]));
        assert_eq!(state.ranktime, -200);
    }

    #[test]
    fn initial_ranktime_is_zero() {
        let state = make_state();
        assert_eq!(state.ranktime, 0);
    }
}

impl LR2SkinLoaderAccess for LR2CourseResultSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin) {
        use crate::skin::SkinObject;

        if let Some(obj) = self.gaugeobj.take() {
            skin.add(SkinObject::GaugeGraph(obj));
        }
        if let Some(obj) = self.noteobj.take() {
            skin.add(SkinObject::NoteDistributionGraph(obj));
        }
    }
}
