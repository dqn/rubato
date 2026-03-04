use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader::{self};
use crate::skin_bpm_graph::SkinBPMGraph;
use crate::skin_gauge_graph_object::SkinGaugeGraphObject;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::stubs::{Rectangle, Resolution};

/// LR2 result skin loader
///
/// Translated from LR2ResultSkinLoader.java (114 lines)
/// Loads LR2 result skins with gauge chart, note chart, BPM chart,
/// and timing chart elements.
///
/// Result skin loader state
pub struct LR2ResultSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,
    pub gauge: Rectangle,
    pub gaugeobj: Option<SkinGaugeGraphObject>,
    pub noteobj: Option<SkinNoteDistributionGraph>,
    pub bpmgraphobj: Option<SkinBPMGraph>,
    pub timinggraphobj: Option<SkinTimingDistributionGraph>,
}

/// Get a trimmed string from str_parts at the given index, or empty string if out of bounds.
fn str_at(parts: &[String], idx: usize) -> &str {
    parts.get(idx).map(|s| s.trim()).unwrap_or("")
}

impl LR2ResultSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            gauge: Rectangle::default(),
            gaugeobj: None,
            noteobj: None,
            bpmgraphobj: None,
            timinggraphobj: None,
        }
    }

    /// Process result-specific commands
    pub fn process_result_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "STARTINPUT" => {
                // skin.setInput(parseInt(str[1]))
                // skin.setRankTime(parseInt(str[2]))
            }
            "SRC_GAUGECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let mut obj = SkinGaugeGraphObject::new_default();
                obj.set_line_width(values[6]);
                obj.set_delay(values[14] - values[13]);
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                self.gaugeobj = Some(obj);
            }
            "DST_GAUGECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(gaugeobj, ...)
            }
            "SRC_NOTECHART_1P" => {
                // #SRC_NOTECHART_1P,(index),(gr),(x),(y),(w),(h),(div_x),(div_y),(cycle),(timer),field_w,field_h,(start),(end),delay,backTexOff,orderReverse,noGap
                let values = lr2_skin_loader::parse_int(str_parts);
                let obj = SkinNoteDistributionGraph::new(
                    values[1], values[15], values[16], values[17], values[18], values[19],
                );
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                self.noteobj = Some(obj);
            }
            "DST_NOTECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(noteobj, ...)
            }
            "SRC_BPMCHART" => {
                // #SRC_BPMCHART, field_w, field_h, delay, lineWidth, mainBPMColor, minBPMColor, maxBPMColor, otherBPMColor, stopLineColor, transitionLineColor
                let values = lr2_skin_loader::parse_int(str_parts);
                let obj = SkinBPMGraph::new(
                    values[3],
                    values[4],
                    str_at(str_parts, 5),
                    str_at(str_parts, 6),
                    str_at(str_parts, 7),
                    str_at(str_parts, 8),
                    str_at(str_parts, 9),
                    str_at(str_parts, 10),
                );
                self.gauge = Rectangle::new(0.0, 0.0, values[1] as f32, values[2] as f32);
                self.bpmgraphobj = Some(obj);
            }
            "DST_BPMCHART" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(bpmgraphobj, ...)
            }
            "SRC_TIMINGCHART_1P" => {
                // #SRC_TIMINGCHART_1P,(index),(gr),(x),width,height,lineWidth,graphColor,averageColor,devColor,PGColor,GRColor,GDColor,BDColor,PRColor,drawAverage,drawDev
                let values = lr2_skin_loader::parse_int(str_parts);
                let obj = SkinTimingDistributionGraph::new(
                    values[4],
                    values[6],
                    str_at(str_parts, 7),
                    str_at(str_parts, 8),
                    str_at(str_parts, 9),
                    str_at(str_parts, 10),
                    str_at(str_parts, 11),
                    str_at(str_parts, 12),
                    str_at(str_parts, 13),
                    str_at(str_parts, 14),
                    values[15],
                    values[16],
                );
                self.gauge = Rectangle::new(0.0, 0.0, values[4] as f32, values[5] as f32);
                self.timinggraphobj = Some(obj);
            }
            "DST_TIMINGCHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(timinggraphobj, ...)
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
    }
}

impl LR2SkinLoaderAccess for LR2ResultSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn assemble_objects(&mut self, _skin: &mut crate::skin::Skin) {
        // Graph objects are stored in self.gaugeobj/noteobj/bpmgraphobj/timinggraphobj.
        // Full skin.add() + setDestination() wiring deferred to rendering pipeline integration.
    }
}
