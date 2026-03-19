use crate::lr2::lr2_skin_csv_loader::LR2SkinCSVLoaderState;
use crate::objects::skin_hidden::SkinHidden;
use crate::reexports::{Rectangle, Resolution, TextureRegion};
use crate::skin_bpm_graph::SkinBPMGraph;
use crate::skin_image::SkinImage;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;

/// LR2 play skin loader
///
/// Translated from LR2PlaySkinLoader.java (1025 lines)
/// Loads LR2 play skins with notes, judge, BGA, gauge, and other play-specific elements.
///
/// Note source data
#[derive(Clone, Debug, Default)]
pub struct SkinSourceData {
    pub images: Option<Vec<TextureRegion>>,
    pub timer: i32,
    pub cycle: i32,
}

/// Parsed PM character entry for deferred assembly.
#[derive(Clone, Debug)]
pub enum PmCharaEntry {
    /// DST_PM_CHARA: side, imagefile, color, dstx, dsty, dstw, dsth
    Chara {
        side: i32,
        imagefile: String,
        color: i32,
        dst: Rectangle,
    },
    /// DST_PM_CHARA_ANIMATION: load_type, imagefile, color, dst values, timer, ops
    Animation {
        load_type: i32,
        imagefile: String,
        color: i32,
        dst: Rectangle,
        timer: i32,
    },
    /// SRC_PM_CHARA_IMAGE: load_type, imagefile, color
    SrcImage {
        load_type: i32,
        imagefile: String,
        color: i32,
    },
    /// DST_PM_CHARA_IMAGE: destination rectangle
    DstImage { dst: Rectangle },
}

/// Play skin loader state
pub struct LR2PlaySkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,

    pub skin_type: crate::skin_type::SkinType,
    pub mode: Option<bms_model::mode::Mode>,

    pub note: Vec<Option<SkinSourceData>>,
    pub lnstart: Vec<Option<SkinSourceData>>,
    pub lnend: Vec<Option<SkinSourceData>>,
    pub lnbody: Vec<Option<SkinSourceData>>,
    pub lnbodya: Vec<Option<SkinSourceData>>,
    pub hcnstart: Vec<Option<SkinSourceData>>,
    pub hcnend: Vec<Option<SkinSourceData>>,
    pub hcnbody: Vec<Option<SkinSourceData>>,
    pub hcnbodya: Vec<Option<SkinSourceData>>,
    pub hcnbodyd: Vec<Option<SkinSourceData>>,
    pub hcnbodyr: Vec<Option<SkinSourceData>>,
    pub mine: Vec<Option<SkinSourceData>>,
    pub laner: Vec<Option<Rectangle>>,
    pub scale: Vec<f32>,
    pub dstnote2: Vec<i32>,
    pub linevalues: [Option<Vec<String>>; 2],

    pub srcw: f32,
    pub srch: f32,
    pub dstw: f32,
    pub dsth: f32,

    pub gauge: Rectangle,
    pub noteobj: Option<SkinNoteDistributionGraph>,
    pub bpmgraphobj: Option<SkinBPMGraph>,
    pub playerr: Vec<Option<Rectangle>>,
    pub hidden: Option<SkinHidden>,
    pub lanerender: bool,
    pub judgeline: Option<SkinImage>,
    pub bga: bool,

    // Accumulated play skin property values (applied by caller)
    /// Close time (ms) — set by CLOSE command
    pub play_close: Option<i32>,
    /// Playstart time (ms) — set by PLAYSTART command
    pub play_playstart: Option<i32>,
    /// Loadstart time (ms) — set by LOADSTART command
    pub play_loadstart: Option<i32>,
    /// Loadend time (ms) — set by LOADEND command
    pub play_loadend: Option<i32>,
    /// Finish margin time (ms) — set by FINISHMARGIN command
    pub play_finish_margin: Option<i32>,
    /// Judge timer condition — set by JUDGETIMER command
    pub play_judgetimer: Option<i32>,
    /// Note expansion rate [w%, h%] — set by DST_NOTE_EXPANSION_RATE command
    pub play_note_expansion_rate: Option<[i32; 2]>,

    /// SkinImage per line index (SRC_LINE/DST_LINE)
    pub line_images: Vec<Option<SkinImage>>,

    /// SkinJudge objects per player (SRC_NOWJUDGE/DST_NOWJUDGE)
    pub judge_objects: [Option<crate::skin_judge_object::SkinJudgeObject>; 3],
    /// Whether DST_NOWJUDGE detail has been added per player
    pub judge_detail_added: [bool; 3],

    /// Parsed PomyuChara entries for deferred assembly.
    pub pmchara_entries: Vec<PmCharaEntry>,

    /// Computed judge region count (set by load_skin post-processing)
    pub computed_judge_reg: Option<i32>,
    /// Computed line count (set by load_skin post-processing)
    pub computed_line_count: Option<usize>,

    /// Lane cover Y position from the last DST_HIDDEN destination.
    /// -1.0 means no lane cover was defined.
    pub lane_cover_dst_y: f32,
}

impl LR2PlaySkinLoaderState {
    pub fn new(
        skin_type: crate::skin_type::SkinType,
        src: Resolution,
        dst: Resolution,
        usecim: bool,
        skinpath: String,
    ) -> Self {
        let srcw = src.width;
        let srch = src.height;
        let dstw = dst.width;
        let dsth = dst.height;

        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            skin_type,
            mode: None,
            note: vec![None; 8],
            lnstart: vec![None; 8],
            lnend: vec![None; 8],
            lnbody: vec![None; 8],
            lnbodya: vec![None; 8],
            hcnstart: vec![None; 8],
            hcnend: vec![None; 8],
            hcnbody: vec![None; 8],
            hcnbodya: vec![None; 8],
            hcnbodyd: vec![None; 8],
            hcnbodyr: vec![None; 8],
            mine: vec![None; 8],
            laner: vec![None; 8],
            scale: vec![0.0; 8],
            dstnote2: vec![0; 8],
            linevalues: [None, None],
            srcw,
            srch,
            dstw,
            dsth,
            gauge: Rectangle::default(),
            noteobj: None,
            bpmgraphobj: None,
            playerr: Vec::new(),
            hidden: None,
            lanerender: false,
            judgeline: None,
            bga: false,
            play_close: None,
            play_playstart: None,
            play_loadstart: None,
            play_loadend: None,
            play_finish_margin: None,
            play_judgetimer: None,
            play_note_expansion_rate: None,
            line_images: vec![None, None, None, None, None, None, None, None],
            judge_objects: [None, None, None],
            judge_detail_added: [false; 3],
            pmchara_entries: Vec::new(),
            computed_judge_reg: None,
            computed_line_count: None,
            lane_cover_dst_y: -1.0,
        }
    }
}

mod command_parser;
mod object_builder;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::Texture;

    fn make_state() -> LR2PlaySkinLoaderState {
        LR2PlaySkinLoaderState::new(
            crate::skin_type::SkinType::Play7Keys,
            Resolution {
                width: 640.0,
                height: 480.0,
            },
            Resolution {
                width: 1920.0,
                height: 1080.0,
            },
            false,
            "/tmp/test_skin".to_string(),
        )
    }

    fn str_vec(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    // Helper: build a str_parts array with enough elements for parse_int (needs 22+ entries)
    fn make_parts(cmd: &str, vals: &[i32]) -> Vec<String> {
        let mut parts = vec![cmd.to_string()];
        for v in vals {
            parts.push(v.to_string());
        }
        // Pad to at least 22 elements
        while parts.len() < 22 {
            parts.push("0".to_string());
        }
        parts
    }

    // ===== Scalar commands =====

    #[test]
    fn test_close_command() {
        let mut state = make_state();
        state.process_play_command("CLOSE", &str_vec(&["CLOSE", "500"]));
        assert_eq!(state.play_close, Some(500));
    }

    #[test]
    fn test_playstart_command() {
        let mut state = make_state();
        state.process_play_command("PLAYSTART", &str_vec(&["PLAYSTART", "1000"]));
        assert_eq!(state.play_playstart, Some(1000));
    }

    #[test]
    fn test_loadstart_command() {
        let mut state = make_state();
        state.process_play_command("LOADSTART", &str_vec(&["LOADSTART", "200"]));
        assert_eq!(state.play_loadstart, Some(200));
    }

    #[test]
    fn test_loadend_command() {
        let mut state = make_state();
        state.process_play_command("LOADEND", &str_vec(&["LOADEND", "3000"]));
        assert_eq!(state.play_loadend, Some(3000));
    }

    #[test]
    fn test_finishmargin_command() {
        let mut state = make_state();
        state.process_play_command("FINISHMARGIN", &str_vec(&["FINISHMARGIN", "2000"]));
        assert_eq!(state.play_finish_margin, Some(2000));
    }

    #[test]
    fn test_judgetimer_command() {
        let mut state = make_state();
        state.process_play_command("JUDGETIMER", &str_vec(&["JUDGETIMER", "1"]));
        assert_eq!(state.play_judgetimer, Some(1));
    }

    #[test]
    fn test_scalar_invalid_parse_returns_none() {
        let mut state = make_state();
        state.process_play_command("CLOSE", &str_vec(&["CLOSE", "xyz"]));
        assert_eq!(state.play_close, None);
    }

    #[test]
    fn test_scalar_empty_parts_no_panic() {
        let mut state = make_state();
        state.process_play_command("CLOSE", &str_vec(&["CLOSE"]));
        assert_eq!(state.play_close, None);
    }

    // ===== DST_NOTE_EXPANSION_RATE =====

    #[test]
    fn test_dst_note_expansion_rate() {
        let mut state = make_state();
        state.process_play_command(
            "DST_NOTE_EXPANSION_RATE",
            &str_vec(&["DST_NOTE_EXPANSION_RATE", "150", "200"]),
        );
        assert_eq!(state.play_note_expansion_rate, Some([150, 200]));
    }

    #[test]
    fn test_dst_note_expansion_rate_too_few_parts() {
        let mut state = make_state();
        // Only 2 parts, needs 3 — should not set
        state.process_play_command(
            "DST_NOTE_EXPANSION_RATE",
            &str_vec(&["DST_NOTE_EXPANSION_RATE", "150"]),
        );
        assert_eq!(state.play_note_expansion_rate, None);
    }

    // ===== SRC_BGA / DST_BGA =====

    #[test]
    fn test_src_bga_sets_flag() {
        let mut state = make_state();
        assert!(!state.bga);
        state.process_play_command("SRC_BGA", &str_vec(&["SRC_BGA"]));
        assert!(state.bga);
    }

    #[test]
    fn test_dst_bga_without_src_no_panic() {
        let mut state = make_state();
        state.process_play_command("DST_BGA", &make_parts("DST_BGA", &[0; 21]));
        // No panic, bga still false
        assert!(!state.bga);
    }

    // ===== SRC_NOWJUDGE / DST_NOWJUDGE =====

    #[test]
    fn test_src_nowjudge_creates_judge_object() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // SRC_NOWJUDGE_1P: values[1]=0 (judge type PG), values[2]=0 (image index)
        let parts = make_parts("SRC_NOWJUDGE_1P", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &parts);
        assert!(state.judge_objects[0].is_some());
    }

    #[test]
    fn test_src_nowjudge_2p_creates_at_index_1() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let parts = make_parts("SRC_NOWJUDGE_2P", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_2P", &parts);
        assert!(state.judge_objects[0].is_none());
        assert!(state.judge_objects[1].is_some());
    }

    #[test]
    fn test_src_nowjudge_sets_judge_image() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1]=5 -> judge_idx = 5-5 = 0 (PG)
        let parts = make_parts("SRC_NOWJUDGE_1P", &[5, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &parts);
        let judge = state.judge_objects[0].as_ref().unwrap();
        assert!(judge.inner.judge(0));
    }

    #[test]
    fn test_src_nowjudge_index_mapping() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1]=3 -> judge_idx = 5-3 = 2
        let parts = make_parts("SRC_NOWJUDGE_1P", &[3, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &parts);
        let judge = state.judge_objects[0].as_ref().unwrap();
        assert!(judge.inner.judge(2));
        // Other indices should be unset
        assert!(!judge.inner.judge(0));
        assert!(!judge.inner.judge(1));
    }

    // ===== SRC_NOWCOMBO =====

    #[test]
    fn test_src_nowcombo_sets_judge_count() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // First create judge object
        let judge_parts = make_parts("SRC_NOWJUDGE_1P", &[5, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &judge_parts);

        // Now set combo count for PG (values[1]=5 -> idx=0)
        let combo_parts = make_parts("SRC_NOWCOMBO_1P", &[5, 0, 0, 0, 10, 10, 10, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWCOMBO_1P", &combo_parts);
        let judge = state.judge_objects[0].as_ref().unwrap();
        assert!(judge.inner.judge_count(0));
    }

    // ===== SRC_JUDGELINE / DST_JUDGELINE =====

    #[test]
    fn test_src_judgeline_creates_skin_image() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let parts = make_parts("SRC_JUDGELINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_JUDGELINE", &parts);
        assert!(state.judgeline.is_some());
    }

    #[test]
    fn test_dst_judgeline_without_src_no_effect() {
        let mut state = make_state();
        assert!(state.judgeline.is_none());
        let parts = make_parts("DST_JUDGELINE", &[0, 0, 100, 200, 50, 30, 0, 0, 0, 0, 0]);
        state.process_play_command("DST_JUDGELINE", &parts);
        assert!(state.judgeline.is_none());
    }

    #[test]
    fn test_dst_judgeline_sets_destination() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let src_parts = make_parts("SRC_JUDGELINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_JUDGELINE", &src_parts);
        assert!(state.judgeline.is_some());

        let dst_parts = make_parts(
            "DST_JUDGELINE",
            &[
                0, 0, 100, 200, 50, 30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_JUDGELINE", &dst_parts);
        let jl = state.judgeline.as_ref().unwrap();
        assert!(!jl.data.dst.is_empty());
    }

    // ===== SRC_LINE / DST_LINE =====

    #[test]
    fn test_src_line_creates_skin_image_at_index() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1] = 0 (line index)
        let parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &parts);
        assert!(state.line_images[0].is_some());
        assert!(state.line_images[1].is_none());
    }

    #[test]
    fn test_src_line_second_index() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1] = 3 (line index)
        let parts = make_parts("SRC_LINE", &[3, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &parts);
        assert!(state.line_images[0].is_none());
        assert!(state.line_images[3].is_some());
    }

    #[test]
    fn test_dst_line_sets_destination() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let src_parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);

        let dst_parts = make_parts(
            "DST_LINE",
            &[
                0, 0, 100, 200, 50, 30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_LINE", &dst_parts);
        let li = state.line_images[0].as_ref().unwrap();
        assert!(!li.data.dst.is_empty());
    }

    #[test]
    fn test_dst_line_negative_width_correction() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let src_parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);

        // Negative width/height: values[5]=-50, values[6]=-30
        let dst_parts = make_parts(
            "DST_LINE",
            &[
                0, 0, 100, 200, -50, -30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_LINE", &dst_parts);
        let li = state.line_images[0].as_ref().unwrap();
        assert!(!li.data.dst.is_empty());
    }

    // ===== PM_CHARA commands =====

    #[test]
    fn test_dst_pm_chara_1p_no_panic() {
        let mut state = make_state();
        let mut parts = make_parts("DST_PM_CHARA_1P", &[100, 200, 50, 30, 1, 0, 0, 0, 0, 0, 0]);
        if parts.len() > 7 {
            parts[7] = "chara_folder".to_string();
        }
        state.process_play_command("DST_PM_CHARA_1P", &parts);
    }

    #[test]
    fn test_dst_pm_chara_animation_no_panic() {
        let mut state = make_state();
        let mut parts = make_parts(
            "DST_PM_CHARA_ANIMATION",
            &[100, 200, 50, 30, 1, 3, 0, 0, 0, 0, 0, 0],
        );
        while parts.len() <= 12 {
            parts.push("0".to_string());
        }
        parts[12] = "chara_anim_folder".to_string();
        state.process_play_command("DST_PM_CHARA_ANIMATION", &parts);
    }

    #[test]
    fn test_dst_pm_chara_animation_invalid_type_skipped() {
        let mut state = make_state();
        // values[6] = 10, which is outside 0-9 range
        let parts = make_parts(
            "DST_PM_CHARA_ANIMATION",
            &[100, 200, 50, 30, 1, 10, 0, 0, 0, 0, 0, 0],
        );
        state.process_play_command("DST_PM_CHARA_ANIMATION", &parts);
    }

    #[test]
    fn test_src_pm_chara_image_no_panic() {
        let mut state = make_state();
        let mut parts = make_parts("SRC_PM_CHARA_IMAGE", &[1, 2, 0]);
        if parts.len() > 3 {
            parts[3] = "chara_image_folder".to_string();
        }
        state.process_play_command("SRC_PM_CHARA_IMAGE", &parts);
    }

    #[test]
    fn test_src_pm_chara_image_invalid_type_skipped() {
        let mut state = make_state();
        // values[2] = 5, which is outside 0-4 range
        let parts = make_parts("SRC_PM_CHARA_IMAGE", &[1, 5, 0]);
        state.process_play_command("SRC_PM_CHARA_IMAGE", &parts);
    }

    #[test]
    fn test_dst_pm_chara_image_no_panic() {
        let mut state = make_state();
        let parts = make_parts(
            "DST_PM_CHARA_IMAGE",
            &[
                0, 0, 100, 200, 50, 30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_PM_CHARA_IMAGE", &parts);
    }

    // ===== apply_to_play_skin =====

    #[test]
    fn test_apply_to_play_skin_all_values() {
        let mut state = make_state();
        state.play_close = Some(500);
        state.play_playstart = Some(1000);
        state.play_loadstart = Some(200);
        state.play_loadend = Some(3000);
        state.play_finish_margin = Some(2000);
        state.play_judgetimer = Some(1);
        state.play_note_expansion_rate = Some([150, 200]);

        let mut play_skin = rubato_play::play_skin::PlaySkin::new();
        state.apply_to_play_skin(&mut play_skin);
        assert_eq!(play_skin.close, 500);
        assert_eq!(play_skin.playstart, 1000);
        assert_eq!(play_skin.loadstart, 200);
        assert_eq!(play_skin.loadend, 3000);
        assert_eq!(play_skin.finish_margin, 2000);
        assert_eq!(play_skin.judgetimer, 1);
        assert_eq!(play_skin.note_expansion_rate, [150, 200]);
    }

    #[test]
    fn test_apply_to_play_skin_none_values_preserved() {
        let state = make_state();
        let mut play_skin = rubato_play::play_skin::PlaySkin::new();
        let orig_close = play_skin.close;
        state.apply_to_play_skin(&mut play_skin);
        assert_eq!(play_skin.close, orig_close);
    }

    // ===== Unknown command delegation =====

    #[test]
    fn test_unknown_command_delegates_to_csv_loader() {
        let mut state = make_state();
        // STARTINPUT should be handled by the CSV base loader via delegation
        state.process_play_command("STARTINPUT", &str_vec(&["STARTINPUT", "750"]));
        assert_eq!(state.csv.skin_input, Some(750));
    }

    // ===== load_skin CSV pipeline integration =====

    /// Helper: write content to a temp file and return the path.
    fn write_temp_csv(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("lr2_play_skin_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_skin_parses_csv_commands() {
        let csv_content = "#CLOSE,500\n#PLAYSTART,1000\n#STARTINPUT,750\n";
        let path = write_temp_csv("test_load_skin_cmds.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        assert_eq!(state.play_close, Some(500));
        assert_eq!(state.play_playstart, Some(1000));
        // STARTINPUT is delegated to the CSV loader
        assert_eq!(state.csv.skin_input, Some(750));
    }

    #[test]
    fn test_dst_note_empty_playerr_does_not_panic() {
        // Regression: DST_NOTE divides by playerr.len() without .max(1) guard.
        // With empty playerr this would divide by zero.
        let mut state = make_state();
        state.mode = Some(bms_model::mode::Mode::BEAT_7K);
        // Initialize laner with 8 keys (matching BEAT_7K) but leave playerr empty
        state.laner = vec![None; 8];
        state.scale = vec![0.0; 8];
        state.dstnote2 = vec![i32::MIN; 8];
        assert!(state.playerr.is_empty());

        // DST_NOTE with lane=1 (non-scratch lane) triggers the division
        let parts = make_parts(
            "DST_NOTE",
            &[
                0, 1, 0, 0, 0, 64, 480, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        // Should not panic
        state.process_play_command("DST_NOTE", &parts);
    }

    #[test]
    fn test_load_skin_initializes_arrays_from_mode() {
        let csv_content = "";
        let path = write_temp_csv("test_load_skin_init.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        // Play7Keys mode has 8 keys
        assert_eq!(state.note.len(), 8);
        assert_eq!(state.laner.len(), 8);
        assert_eq!(state.scale.len(), 8);
        assert_eq!(state.dstnote2.len(), 8);
        // All dstnote2 should be i32::MIN
        assert!(state.dstnote2.iter().all(|&v| v == i32::MIN));
    }

    #[test]
    fn test_load_skin_computes_judge_reg_default() {
        let csv_content = "";
        let path = write_temp_csv("test_load_skin_judge.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        // No judge objects created, default judge_reg = 1
        assert_eq!(state.computed_judge_reg, Some(1));
    }

    #[test]
    fn test_load_skin_computes_line_count_zero() {
        let csv_content = "";
        let path = write_temp_csv("test_load_skin_lines.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        // No line images created
        assert_eq!(state.computed_line_count, Some(0));
    }

    #[test]
    fn test_load_skin_file_not_found_returns_error() {
        let mut state = make_state();
        let result = state.load_skin(std::path::Path::new("/nonexistent/skin.lr2skin"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_skin_applies_to_play_skin_with_computed_values() {
        let csv_content = "#CLOSE,500\n#JUDGETIMER,2\n";
        let path = write_temp_csv("test_load_skin_apply.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        let mut play_skin = rubato_play::play_skin::PlaySkin::new();
        state.apply_to_play_skin(&mut play_skin);

        assert_eq!(play_skin.close, 500);
        assert_eq!(play_skin.judgetimer, 2);
        assert_eq!(play_skin.judgeregion, 1); // default
        // Lane region should be set (8 default rectangles)
        assert!(play_skin.lane_region().is_some());
    }

    // ===== make_default_line / default line images =====

    /// Helper: set up a judge line at index 0 with SRC_LINE + DST_LINE and stored linevalues.
    fn setup_judge_line(state: &mut LR2PlaySkinLoaderState) {
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // SRC_LINE: index=0, imageID=0, x=0, y=0, w=10, h=10, divx=1, divy=1, cycle=0, timer=0
        let src_parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);
        // DST_LINE: index=0, time=0, x=100, y=200, w=500, h=2, acc=0, a=255, r=255, g=255, b=255, ...
        let dst_parts = make_parts(
            "DST_LINE",
            &[
                0, 0, 100, 200, 500, 2, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_LINE", &dst_parts);
    }

    #[test]
    fn test_make_default_line_creates_time_line_when_judge_exists() {
        let mut state = make_state();
        setup_judge_line(&mut state);
        assert!(state.line_images[0].is_some()); // judge line exists
        assert!(state.line_images[6].is_none()); // time line missing

        // Simulate what load_skin does: create default time line
        state.make_default_line(6, 1, 64, 192, 192);

        assert!(
            state.line_images[6].is_some(),
            "time line should be created"
        );
        let li = state.line_images[6].as_ref().unwrap();
        assert!(!li.data.dst.is_empty(), "destination should be set");
    }

    #[test]
    fn test_make_default_line_creates_bpm_line() {
        let mut state = make_state();
        setup_judge_line(&mut state);
        assert!(state.line_images[2].is_none()); // BPM line missing

        state.make_default_line(2, 2, 0, 192, 0);

        assert!(state.line_images[2].is_some(), "BPM line should be created");
        let li = state.line_images[2].as_ref().unwrap();
        assert!(!li.data.dst.is_empty(), "destination should be set");
    }

    #[test]
    fn test_make_default_line_creates_stop_line() {
        let mut state = make_state();
        setup_judge_line(&mut state);
        assert!(state.line_images[4].is_none()); // stop line missing

        state.make_default_line(4, 2, 192, 192, 0);

        assert!(
            state.line_images[4].is_some(),
            "stop line should be created"
        );
        let li = state.line_images[4].as_ref().unwrap();
        assert!(!li.data.dst.is_empty(), "destination should be set");
    }

    #[test]
    fn test_make_default_line_skips_when_no_linevalues() {
        let mut state = make_state();
        // No DST_LINE processed, so linevalues is [None, None]
        assert!(state.linevalues[0].is_none());

        state.make_default_line(6, 1, 64, 192, 192);

        // Should not create anything since linevalues is unavailable
        assert!(state.line_images[6].is_none());
    }

    #[test]
    fn test_load_skin_creates_default_lines_when_judge_exists() {
        let mut state = make_state();
        setup_judge_line(&mut state);

        // Simulate load_skin post-processing: count lines then create defaults
        let line_count = if state.line_images[0].is_some() {
            if state.line_images[1].is_some() { 2 } else { 1 }
        } else {
            0
        };
        assert_eq!(line_count, 1);

        for i in 0..line_count {
            if state.line_images[i + 6].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 6, 1, 64, 192, 192);
            }
            if state.line_images[i + 2].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 2, 2, 0, 192, 0);
            }
            if state.line_images[i + 4].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 4, 2, 192, 192, 0);
            }
        }

        // Time, BPM, stop lines should be created
        assert!(state.line_images[6].is_some(), "time line at [6]");
        assert!(state.line_images[2].is_some(), "BPM line at [2]");
        assert!(state.line_images[4].is_some(), "stop line at [4]");
    }

    #[test]
    fn test_load_skin_no_defaults_when_line_count_zero() {
        let mut state = make_state();
        // No judge lines at all
        assert!(state.line_images[0].is_none());

        let line_count = if state.line_images[0].is_some() {
            if state.line_images[1].is_some() { 2 } else { 1 }
        } else {
            0
        };
        assert_eq!(line_count, 0);

        // Loop body never executes when line_count == 0
        for i in 0..line_count {
            state.make_default_line(i + 6, 1, 64, 192, 192);
            state.make_default_line(i + 2, 2, 0, 192, 0);
            state.make_default_line(i + 4, 2, 192, 192, 0);
        }

        // Nothing should be created
        for slot in &state.line_images {
            assert!(slot.is_none());
        }
    }

    #[test]
    fn test_existing_line_images_not_overwritten() {
        let mut state = make_state();
        setup_judge_line(&mut state);

        // Manually create a line image at index 6 (time line)
        // values[1]=6 means line index 6 in SRC_LINE
        let src_parts = make_parts("SRC_LINE", &[6, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);
        assert!(state.line_images[6].is_some(), "pre-existing time line");

        // The condition `line_images[i + 6].is_none()` should prevent overwriting
        let line_count = 1;
        for i in 0..line_count {
            if state.line_images[i + 6].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 6, 1, 64, 192, 192);
            }
        }

        // The existing line image should still be there, not replaced
        let li = state.line_images[6].as_ref().unwrap();
        // The pre-existing image was created via SRC_LINE, so it has no destinations
        // (DST_LINE was never called for index 6). A default line would have a destination.
        assert!(
            li.data.dst.is_empty(),
            "existing image should not be overwritten"
        );
    }

    #[test]
    fn test_make_default_line_height_multiplier() {
        let mut state = make_state();
        setup_judge_line(&mut state);

        // Create two default lines with different height multipliers
        state.make_default_line(6, 1, 64, 192, 192); // h=1 (time)
        state.make_default_line(2, 2, 0, 192, 0); // h=2 (BPM)

        let time_line = state.line_images[6].as_ref().unwrap();
        let bpm_line = state.line_images[2].as_ref().unwrap();

        // Both should have destinations
        assert!(!time_line.data.dst.is_empty());
        assert!(!bpm_line.data.dst.is_empty());

        // The BPM line (h=2) should have double the height of the time line (h=1)
        // DST_LINE had h=2 (values[6]), scaled by dsth/srch = 1080/480 = 2.25
        // Time: 2 * 2.25 * 1 = 4.5; BPM: 2 * 2.25 * 2 = 9.0
        let time_dst = &time_line.data.dst[0];
        let bpm_dst = &bpm_line.data.dst[0];
        let time_h = time_dst.region.height;
        let bpm_h = bpm_dst.region.height;
        assert!(
            (bpm_h - time_h * 2.0).abs() < 0.01,
            "BPM height ({}) should be 2x time height ({})",
            bpm_h,
            time_h
        );
    }
}
