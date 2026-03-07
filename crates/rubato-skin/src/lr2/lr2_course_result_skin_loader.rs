use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader::{self};
use crate::skin_gauge_graph_object::SkinGaugeGraphObject;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
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
}

impl LR2CourseResultSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            gauge: Rectangle::default(),
            gaugeobj: None,
            noteobj: None,
        }
    }

    /// Process course result-specific commands
    pub fn process_course_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "STARTINPUT" => {
                // skin.setInput(parseInt(str[1]))
                // skin.setRankTime(parseInt(str[2]))
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
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    obj.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        self.gauge.x * dstw,
                        self.csv.dst.height - (values[4] as f32 + self.gauge.height) * dsth,
                        self.gauge.width * dstw,
                        self.gauge.height * dsth,
                        values[7],
                        values[8],
                        values[9],
                        values[10],
                        values[11],
                        values[12],
                        values[13],
                        values[14],
                        values[15],
                        values[16],
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
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
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
