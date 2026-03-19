use std::collections::HashMap;

use crate::graphs::skin_graph::SkinGraph;
use crate::lr2::lr2_skin_loader::LR2SkinLoaderState;
use crate::objects::skin_number::SkinNumber;
use crate::objects::skin_slider::SkinSlider;
use crate::reexports::{Resolution, Texture, TextureRegion};
use crate::skin::SkinObject;
use crate::skin_gauge::SkinGauge;
use crate::skin_image::SkinImage;
use crate::skin_text_image::SkinTextImageSource;

/// LR2 CSV skin loader base
///
/// Translated from LR2SkinCSVLoader.java
/// Base class for all LR2 CSV-based skin loaders.
/// Provides IMAGE, LR2FONT, SRC_IMAGE, DST_IMAGE, SRC_NUMBER, DST_NUMBER,
/// SRC_TEXT, DST_TEXT, SRC_SLIDER, DST_SLIDER, SRC_BARGRAPH, DST_BARGRAPH,
/// SRC_BUTTON, DST_BUTTON, SRC_ONMOUSE, DST_ONMOUSE, SRC_GROOVEGAUGE, DST_GROOVEGAUGE,
/// INCLUDE, STARTINPUT, SCENETIME, FADEOUT, STRETCH commands.
///
/// Image list entry (can be Texture or MovieSource)
pub enum ImageListEntry {
    TextureEntry(Texture),
    Movie(String),
    Null,
}

/// State for CSV loader
pub struct LR2SkinCSVLoaderState {
    pub base: LR2SkinLoaderState,
    pub imagelist: Vec<ImageListEntry>,
    pub fontlist: Vec<Option<SkinTextImageSource>>,

    /// Source resolution
    pub src: Resolution,
    /// Destination resolution
    pub dst: Resolution,
    pub usecim: bool,
    pub skinpath: String,

    pub filemap: HashMap<String, String>,

    // Accumulated skin property values (applied to Skin by caller)
    pub stretch: i32,
    /// Input start time (ms) — set by STARTINPUT command
    pub skin_input: Option<i32>,
    /// Scene time (ms) — set by SCENETIME command
    pub skin_scene: Option<i32>,
    /// Fadeout time (ms) — set by FADEOUT command
    pub skin_fadeout: Option<i32>,

    pub groovex: i32,
    pub groovey: i32,
    pub line: Option<String>,
    pub imagesetarray: Vec<Vec<TextureRegion>>,

    // Active skin objects (built by SRC, destination set by DST)
    pub image: Option<SkinImage>,
    pub num: Option<SkinNumber>,
    pub text: Option<SkinObject>,
    pub slider: Option<SkinSlider>,
    pub bar: Option<SkinGraph>,
    pub button: Option<SkinImage>,
    pub onmouse: Option<SkinImage>,
    pub gauger: Option<SkinGauge>,
    /// Collected skin objects to add to Skin after parsing
    pub collected_objects: Vec<SkinObject>,
}

mod command_parser;
mod object_builder;

pub use object_builder::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> LR2SkinCSVLoaderState {
        LR2SkinCSVLoaderState::new(
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

    #[test]
    fn test_startinput_parses_value() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT", "1000"]), None);
        assert_eq!(state.skin_input, Some(1000));
    }

    #[test]
    fn test_startinput_empty_parts_no_panic() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT"]), None);
        assert_eq!(state.skin_input, None);
    }

    #[test]
    fn test_scenetime_parses_value() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "5000"]), None);
        assert_eq!(state.skin_scene, Some(5000));
    }

    #[test]
    fn test_fadeout_parses_value() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "300"]), None);
        assert_eq!(state.skin_fadeout, Some(300));
    }

    #[test]
    fn test_fadeout_invalid_value_returns_none() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "abc"]), None);
        assert_eq!(state.skin_fadeout, None);
    }

    #[test]
    fn test_stretch_parses_value() {
        let mut state = make_state();
        assert_eq!(state.stretch, -1);
        state.process_csv_command("STRETCH", &str_vec(&["STRETCH", "2"]), None);
        assert_eq!(state.stretch, 2);
    }

    #[test]
    fn test_apply_to_skin_transfers_values() {
        let mut state = make_state();
        state.skin_input = Some(500);
        state.skin_scene = Some(60000);
        state.skin_fadeout = Some(200);
        state.base.op.insert(30, 1);
        state.base.op.insert(31, 0);

        let mut skin = crate::skin::Skin::new(crate::skin_header::SkinHeader::new());
        state.apply_to_skin(&mut skin);
        assert_eq!(skin.input(), 500);
        assert_eq!(skin.scene(), 60000);
        assert_eq!(skin.fadeout(), 200);
        assert_eq!(skin.option().get(&30), Some(&1));
        assert_eq!(skin.option().get(&31), Some(&0));
    }

    #[test]
    fn test_apply_to_skin_none_values_not_overwritten() {
        let state = make_state();
        let mut skin = crate::skin::Skin::new(crate::skin_header::SkinHeader::new());
        skin.input = 42;
        skin.scene = 99;
        skin.fadeout = 77;

        state.apply_to_skin(&mut skin);
        // None values should not overwrite existing values
        assert_eq!(skin.input(), 42);
        assert_eq!(skin.scene(), 99);
        assert_eq!(skin.fadeout(), 77);
    }

    #[test]
    fn test_unknown_command_no_panic() {
        let mut state = make_state();
        state.process_csv_command("NONEXISTENT", &str_vec(&["NONEXISTENT", "1"]), None);
        // Should not panic, no state changed
        assert_eq!(state.skin_input, None);
    }

    // --- load_skin0 file-based tests ---

    /// Helper: write content to a temp file and return the path.
    fn write_temp_csv(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_skin0_parses_directives_from_file() {
        let csv = "\
#STARTINPUT,750\n\
#SCENETIME,4000\n\
#FADEOUT,200\n\
#STRETCH,1\n";
        let path = write_temp_csv("directives.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        assert_eq!(state.skin_input, Some(750));
        assert_eq!(state.skin_scene, Some(4000));
        assert_eq!(state.skin_fadeout, Some(200));
        assert_eq!(state.stretch, 1);
    }

    #[test]
    fn test_load_skin0_empty_file() {
        let path = write_temp_csv("empty.lr2skin", "");
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        // Nothing should be set
        assert_eq!(state.skin_input, None);
        assert_eq!(state.skin_scene, None);
        assert_eq!(state.skin_fadeout, None);
        assert_eq!(state.stretch, -1);
        assert!(state.imagelist.is_empty());
    }

    #[test]
    fn test_load_skin0_nonexistent_file_returns_error() {
        let path = std::path::PathBuf::from("/nonexistent/path/skin.lr2skin");
        let mut state = make_state();
        assert!(state.load_skin0(&path, None).is_err());
    }

    #[test]
    fn test_load_skin0_lines_without_hash_are_skipped() {
        // Lines not starting with '#' are ignored by process_line_directives
        let csv = "\
This is a comment line\n\
SCENETIME,9999\n\
   indented line\n\
\n\
#SCENETIME,1234\n";
        let path = write_temp_csv("comments.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        // Only the #SCENETIME line should be processed
        assert_eq!(state.skin_scene, Some(1234));
        assert_eq!(state.skin_input, None);
    }

    #[test]
    fn test_load_skin0_blank_lines_are_harmless() {
        let csv = "\n\n\n#FADEOUT,100\n\n\n";
        let path = write_temp_csv("blanks.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_fadeout, Some(100));
    }

    #[test]
    fn test_apply_to_skin_preserves_option_gated_object_through_prepare() {
        use crate::objects::skin_image::SkinImage;
        use crate::skin::SkinObject;
        use crate::skin_object::DestinationParams;
        use rubato_core::main_state::SkinDrawable;

        let mut state = make_state();
        state.base.op.insert(30, 1);
        state.base.op.insert(31, 0);

        let mut skin = crate::skin::Skin::new(crate::skin_header::SkinHeader::new());
        skin.add(SkinObject::Image(SkinImage::new_with_image_id(111)));
        skin.set_destination(
            0,
            &DestinationParams {
                time: 0,
                x: 0.0,
                y: 0.0,
                w: 32.0,
                h: 32.0,
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
            &[30],
            &[],
        );

        state.apply_to_skin(&mut skin);
        skin.prepare_skin();

        assert_eq!(
            skin.objects().len(),
            1,
            "selected option must keep the gated object alive through Skin::prepare()"
        );
    }

    // --- #IF / #ELSE / #ENDIF conditional processing ---

    #[test]
    fn test_load_skin0_if_true_branch() {
        let csv = "\
#SETOPTION,42,1\n\
#IF,42\n\
#SCENETIME,1111\n\
#ELSE\n\
#SCENETIME,2222\n\
#ENDIF\n";
        let path = write_temp_csv("if_true.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(1111));
    }

    #[test]
    fn test_load_skin0_if_false_branch() {
        let csv = "\
#SETOPTION,42,0\n\
#IF,42\n\
#SCENETIME,1111\n\
#ELSE\n\
#SCENETIME,2222\n\
#ENDIF\n";
        let path = write_temp_csv("if_false.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(2222));
    }

    #[test]
    fn test_load_skin0_if_unset_option_skips_true_branch() {
        // When the option is not set at all, #IF evaluates to false
        let csv = "\
#IF,99\n\
#SCENETIME,1111\n\
#ELSE\n\
#SCENETIME,2222\n\
#ENDIF\n";
        let path = write_temp_csv("if_unset.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(2222));
    }

    // --- IMAGE command tests ---

    #[test]
    fn test_image_command_nonexistent_file_pushes_null() {
        let mut state = make_state();
        state.process_csv_command(
            "IMAGE",
            &str_vec(&["#IMAGE", "/nonexistent/image.png"]),
            None,
        );
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(state.imagelist[0], ImageListEntry::Null));
    }

    #[test]
    fn test_image_command_movie_extension_detection() {
        // Create a temp file with a movie extension to test classification
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let movie_path = dir.join("test.mp4");
        std::fs::write(&movie_path, b"fake movie data").unwrap();

        let mut state = make_state();
        state.process_csv_command(
            "IMAGE",
            &str_vec(&["#IMAGE", movie_path.to_str().unwrap()]),
            None,
        );
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(state.imagelist[0], ImageListEntry::Movie(_)));
    }

    #[test]
    fn test_image_command_real_png_loads_as_texture() {
        // Create a minimal 1x1 PNG
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let png_path = dir.join("test_1x1.png");
        // Minimal valid 1x1 white PNG
        let png_data: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // 8-bit RGB
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // compressed data
            0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, // ...
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&png_path, png_data).unwrap();

        let mut state = make_state();
        state.process_csv_command(
            "IMAGE",
            &str_vec(&["#IMAGE", png_path.to_str().unwrap()]),
            None,
        );
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(
            state.imagelist[0],
            ImageListEntry::TextureEntry(_)
        ));
    }

    #[test]
    fn test_multiple_images_grow_imagelist() {
        let mut state = make_state();
        // All nonexistent, but imagelist should still grow
        for i in 0..5 {
            state.process_csv_command(
                "IMAGE",
                &str_vec(&["#IMAGE", &format!("/nonexistent/img{}.png", i)]),
                None,
            );
        }
        assert_eq!(state.imagelist.len(), 5);
        assert!(
            state
                .imagelist
                .iter()
                .all(|e| matches!(e, ImageListEntry::Null))
        );
    }

    // --- LR2FONT command tests ---

    #[test]
    fn test_lr2font_nonexistent_file_pushes_none() {
        let mut state = make_state();
        state.process_csv_command(
            "LR2FONT",
            &str_vec(&["#LR2FONT", "/nonexistent/font.lr2font"]),
            None,
        );
        assert_eq!(state.fontlist.len(), 1);
        assert!(state.fontlist[0].is_none());
    }

    #[test]
    fn test_lr2font_existing_file_pushes_some() {
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let font_path = dir.join("test.lr2font");
        std::fs::write(&font_path, b"fake font data").unwrap();

        let mut state = make_state();
        state.process_csv_command(
            "LR2FONT",
            &str_vec(&["#LR2FONT", font_path.to_str().unwrap()]),
            None,
        );
        assert_eq!(state.fontlist.len(), 1);
        assert!(state.fontlist[0].is_some());
    }

    // --- parse_int tests ---

    #[test]
    fn test_parse_int_basic() {
        let parts = str_vec(&["#CMD", "10", "20", "30"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        assert_eq!(result[1], 10);
        assert_eq!(result[2], 20);
        assert_eq!(result[3], 30);
        // Rest should be 0
        assert_eq!(result[4], 0);
    }

    #[test]
    fn test_parse_int_empty_parts() {
        let parts = str_vec(&["#CMD"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        // All zeros
        assert!(result.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_parse_int_bang_as_negative() {
        // '!' is replaced with '-' in Java, so !5 becomes -5
        let parts = str_vec(&["#CMD", "!5"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        assert_eq!(result[1], -5);
    }

    #[test]
    fn test_parse_int_non_numeric_becomes_zero() {
        let parts = str_vec(&["#CMD", "abc", "42"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        assert_eq!(result[1], 0); // "abc" -> parse fails -> 0
        assert_eq!(result[2], 42);
    }

    #[test]
    fn test_parse_int_more_than_22_parts_truncated() {
        // parse_int only reads up to index 21
        let mut parts: Vec<&str> = vec!["#CMD"];
        parts.extend(std::iter::repeat_n("7", 25));
        let result = LR2SkinCSVLoaderState::parse_int(&str_vec(&parts));
        assert_eq!(result[1], 7);
        assert_eq!(result[21], 7);
        // Index 0 is always 0 (skipped)
        assert_eq!(result[0], 0);
    }

    // --- read_offset tests ---

    #[test]
    fn test_read_offset_basic() {
        let parts = str_vec(&[
            "#DST", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0",
            "0", "0", "0", "0", "0", "100", "200",
        ]);
        let offsets = LR2SkinCSVLoaderState::read_offset(&parts, 21);
        // Index 21 is "0", index 22 is "100", index 23 is "200"
        assert_eq!(offsets, vec![0, 100, 200]);
    }

    #[test]
    fn test_read_offset_no_extra_parts() {
        let parts = str_vec(&["#DST", "0"]);
        let offsets = LR2SkinCSVLoaderState::read_offset(&parts, 21);
        assert!(offsets.is_empty());
    }

    // --- source_image_from_texture tests ---

    #[test]
    fn test_get_source_image_from_texture_basic_grid() {
        let tex = Texture {
            width: 100,
            height: 100,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, 100, 100, 2, 2);
        // 2x2 grid = 4 images
        assert_eq!(images.len(), 4);
        // First cell: (0,0) 50x50
        assert_eq!(images[0].region_x, 0);
        assert_eq!(images[0].region_y, 0);
        assert_eq!(images[0].region_width, 50);
        assert_eq!(images[0].region_height, 50);
        // Second cell: (50,0)
        assert_eq!(images[1].region_x, 50);
        assert_eq!(images[1].region_y, 0);
    }

    #[test]
    fn test_get_source_image_from_texture_w_h_minus_one_uses_full_texture() {
        let tex = Texture {
            width: 200,
            height: 150,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, -1, -1, 1, 1);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].region_width, 200);
        assert_eq!(images[0].region_height, 150);
    }

    #[test]
    fn test_get_source_image_from_texture_zero_div_treated_as_one() {
        let tex = Texture {
            width: 64,
            height: 64,
            ..Default::default()
        };
        // divx=0, divy=0 should be treated as 1
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, 64, 64, 0, 0);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].region_width, 64);
        assert_eq!(images[0].region_height, 64);
    }

    #[test]
    fn test_get_source_image_from_texture_negative_div_treated_as_one() {
        let tex = Texture {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, 64, 64, -3, -2);
        assert_eq!(images.len(), 1);
    }

    // --- source_image tests ---

    #[test]
    fn test_get_source_image_out_of_bounds_index_returns_none() {
        let state = make_state();
        // imagelist is empty, gr=0 is out of bounds
        let values = [0i32; 22];
        assert!(state.source_image(&values).is_none());
    }

    #[test]
    fn test_get_source_image_null_entry_returns_none() {
        let mut state = make_state();
        state.imagelist.push(ImageListEntry::Null);
        let mut values = [0i32; 22];
        values[2] = 0; // gr index
        assert!(state.source_image(&values).is_none());
    }

    #[test]
    fn test_get_source_image_movie_entry_returns_none() {
        let mut state = make_state();
        state
            .imagelist
            .push(ImageListEntry::Movie("test.mp4".to_string()));
        let mut values = [0i32; 22];
        values[2] = 0; // gr index
        assert!(state.source_image(&values).is_none());
    }

    #[test]
    fn test_get_source_image_valid_texture_returns_regions() {
        let mut state = make_state();
        let tex = Texture {
            width: 128,
            height: 64,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0; // gr index
        values[3] = 0; // x
        values[4] = 0; // y
        values[5] = 128; // w
        values[6] = 64; // h
        values[7] = 2; // divx
        values[8] = 2; // divy
        let result = state.source_image(&values);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 4); // 2x2 grid
    }

    // --- finalize_active_objects tests ---

    #[test]
    fn test_finalize_active_objects_empty_state() {
        let mut state = make_state();
        state.finalize_active_objects();
        assert!(state.collected_objects.is_empty());
    }

    // --- STRETCH edge cases ---

    #[test]
    fn test_stretch_invalid_value_defaults_to_minus_one() {
        let mut state = make_state();
        state.process_csv_command("STRETCH", &str_vec(&["STRETCH", "abc"]), None);
        assert_eq!(state.stretch, -1);
    }

    #[test]
    fn test_stretch_empty_parts_unchanged() {
        let mut state = make_state();
        state.process_csv_command("STRETCH", &str_vec(&["STRETCH"]), None);
        assert_eq!(state.stretch, -1);
    }

    // --- Directive value edge cases ---

    #[test]
    fn test_startinput_negative_value() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT", "-100"]), None);
        assert_eq!(state.skin_input, Some(-100));
    }

    #[test]
    fn test_scenetime_zero_value() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "0"]), None);
        assert_eq!(state.skin_scene, Some(0));
    }

    #[test]
    fn test_fadeout_whitespace_trimmed() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "  500  "]), None);
        assert_eq!(state.skin_fadeout, Some(500));
    }

    #[test]
    fn test_multiple_commands_last_value_wins() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "1000"]), None);
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "2000"]), None);
        assert_eq!(state.skin_scene, Some(2000));
    }

    // --- new() constructor tests ---

    #[test]
    fn test_new_initializes_defaults() {
        let state = make_state();
        assert_eq!(state.stretch, -1);
        assert_eq!(state.skin_input, None);
        assert_eq!(state.skin_scene, None);
        assert_eq!(state.skin_fadeout, None);
        assert_eq!(state.groovex, 0);
        assert_eq!(state.groovey, 0);
        assert!(state.imagelist.is_empty());
        assert!(state.fontlist.is_empty());
        assert!(state.filemap.is_empty());
        assert!(state.collected_objects.is_empty());
        assert!(state.button.is_none());
        assert!(state.onmouse.is_none());
        assert!(state.gauger.is_none());
        assert!(state.line.is_none());
        assert!(state.imagesetarray.is_empty());
    }

    #[test]
    fn test_new_registers_csv_command_names() {
        let _state = make_state();
        // Verify key command names are registered by checking the base state
        // accepts them via process_line_directives
        let expected_commands = [
            "STARTINPUT",
            "SCENETIME",
            "FADEOUT",
            "STRETCH",
            "INCLUDE",
            "IMAGE",
            "LR2FONT",
            "SRC_IMAGE",
            "DST_IMAGE",
            "SRC_NUMBER",
            "DST_NUMBER",
            "SRC_BUTTON",
            "DST_BUTTON",
            "SRC_GROOVEGAUGE",
        ];
        // All these commands should be recognized (they won't return None for
        // skip-related reasons since skip is false initially)
        for cmd in &expected_commands {
            let mut test_state = make_state();
            let line = format!("#{},0", cmd);
            let result = test_state.base.process_line_directives(&line, None);
            assert!(result.is_some(), "Command {} should be recognized", cmd);
        }
    }

    // --- INCLUDE command tests ---

    #[test]
    fn test_include_nonexistent_file_no_panic() {
        let mut state = make_state();
        state.process_csv_command(
            "INCLUDE",
            &str_vec(&["#INCLUDE", "/nonexistent/include.lr2skin"]),
            None,
        );
        // Should silently skip
        assert_eq!(state.skin_scene, None);
    }

    #[test]
    fn test_include_processes_included_file_commands() {
        // Create an included file with directives
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let inc_path = dir.join("included.lr2skin");
        std::fs::write(&inc_path, "#SCENETIME,7777\n#FADEOUT,333\n").unwrap();

        let mut state = make_state();
        state.skinpath = dir.to_str().unwrap().to_string();
        state.process_csv_command(
            "INCLUDE",
            &str_vec(&["#INCLUDE", inc_path.to_str().unwrap()]),
            None,
        );
        assert_eq!(state.skin_scene, Some(7777));
        assert_eq!(state.skin_fadeout, Some(333));
    }

    // --- load_skin0 integration: SRC/DST pairs through full pipeline ---

    #[test]
    fn test_load_skin0_combined_directives_and_conditionals() {
        let csv = "\
#STARTINPUT,200\n\
#SETOPTION,10,1\n\
#IF,10\n\
#SCENETIME,5555\n\
#ENDIF\n\
#FADEOUT,400\n\
#STRETCH,3\n";
        let path = write_temp_csv("combined.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        assert_eq!(state.skin_input, Some(200));
        assert_eq!(state.skin_scene, Some(5555));
        assert_eq!(state.skin_fadeout, Some(400));
        assert_eq!(state.stretch, 3);
    }

    #[test]
    fn test_load_skin0_shift_jis_encoding() {
        // load_skin0 decodes Shift-JIS. Verify ASCII content works fine.
        let csv = b"#SCENETIME,9999\n";
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sjis_ascii.lr2skin");
        std::fs::write(&path, csv).unwrap();

        let mut state = make_state();
        state.skinpath = dir.to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(9999));
    }

    // --- build_gauge_image_array tests ---

    #[test]
    fn test_build_gauge_standard_4_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 80,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        // 4 divx, 1 divy -> total=4, standard mode: 4 per state = 1 state
        let mut values = [0i32; 22];
        values[2] = 0; // gr
        values[3] = 0; // x
        values[4] = 0; // y
        values[5] = 80; // w
        values[6] = 20; // h
        values[14] = 0; // anim_type != 3 -> standard

        let gauge = state.build_gauge_image_array(&values, 4, 1, 4, false);
        assert_eq!(gauge.len(), 1); // 1 state
        assert_eq!(gauge[0].len(), 36); // 36 slots per state
        // Slots 0-3 should be populated
        assert!(gauge[0][0].is_some());
        assert!(gauge[0][1].is_some());
        assert!(gauge[0][2].is_some());
        assert!(gauge[0][3].is_some());
    }

    #[test]
    fn test_build_gauge_too_few_images_returns_empty() {
        let mut state = make_state();
        let tex = Texture {
            width: 20,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[5] = 20;
        values[6] = 20;

        // total=1, but standard needs at least 4 -> states=0
        let gauge = state.build_gauge_image_array(&values, 1, 1, 1, false);
        assert!(gauge.is_empty());
    }

    #[test]
    fn test_build_gauge_null_image_returns_empty() {
        let mut state = make_state();
        state.imagelist.push(ImageListEntry::Null);
        let mut values = [0i32; 22];
        values[2] = 0;
        let gauge = state.build_gauge_image_array(&values, 1, 1, 1, false);
        assert!(gauge.is_empty());
    }

    #[test]
    fn test_build_gauge_pms_mode_6_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 120,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 120;
        values[6] = 20;
        values[14] = 3; // anim_type=3 -> PMS mode

        // 6 divx, 1 divy -> total=6, PMS mode: 6 per state = 1 state
        let gauge = state.build_gauge_image_array(&values, 6, 1, 6, false);
        assert_eq!(gauge.len(), 1);
        assert_eq!(gauge[0].len(), 36);
    }

    #[test]
    fn test_build_gauge_ex_standard_8_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 160,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 160;
        values[6] = 20;
        values[14] = 0; // not PMS

        // 8 divx, 1 divy -> total=8, EX mode: 8 per state = 1 state
        let gauge = state.build_gauge_image_array(&values, 8, 1, 8, true);
        assert_eq!(gauge.len(), 1);
        assert_eq!(gauge[0].len(), 36);
    }

    #[test]
    fn test_build_gauge_ex_pms_12_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 240,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 240;
        values[6] = 20;
        values[14] = 3; // PMS

        // 12 divx, 1 divy -> total=12, EX+PMS: 12 per state = 1 state
        let gauge = state.build_gauge_image_array(&values, 12, 1, 12, true);
        assert_eq!(gauge.len(), 1);
        assert_eq!(gauge[0].len(), 36);
    }

    // --- SRC_BUTTON group_size == 0 guard ---

    // --- IMAGE command edge cases ---

    #[test]
    fn test_image_command_missing_path_pushes_null() {
        let mut state = make_state();
        state.process_csv_command("IMAGE", &str_vec(&["#IMAGE"]), None);
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(state.imagelist[0], ImageListEntry::Null));
    }

    #[test]
    fn test_image_command_empty_path_pushes_null() {
        let mut state = make_state();
        state.process_csv_command("IMAGE", &str_vec(&["#IMAGE", ""]), None);
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(state.imagelist[0], ImageListEntry::Null));
    }

    #[test]
    fn test_image_movie_extensions_case_insensitive() {
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        for ext in &["MP4", "Mp4", "avi", "AVI", "mpg", "wmv", "m4v"] {
            let movie_path = dir.join(format!("test_case.{}", ext));
            std::fs::write(&movie_path, b"fake").unwrap();
            let mut state = make_state();
            state.process_csv_command(
                "IMAGE",
                &str_vec(&["#IMAGE", movie_path.to_str().unwrap()]),
                None,
            );
            assert!(
                matches!(state.imagelist[0], ImageListEntry::Movie(_)),
                "Extension .{} should be detected as movie",
                ext
            );
        }
    }

    // --- LR2FONT command edge cases ---

    #[test]
    fn test_lr2font_missing_path_pushes_none() {
        let mut state = make_state();
        state.process_csv_command("LR2FONT", &str_vec(&["#LR2FONT"]), None);
        assert_eq!(state.fontlist.len(), 1);
        assert!(state.fontlist[0].is_none());
    }

    // --- INCLUDE edge cases ---

    #[test]
    fn test_include_missing_path_no_panic() {
        let mut state = make_state();
        state.process_csv_command("INCLUDE", &str_vec(&["#INCLUDE"]), None);
        // Should not panic
        assert_eq!(state.skin_scene, None);
    }

    // --- source_image_from_texture edge cases ---

    #[test]
    fn test_source_image_from_texture_large_grid() {
        let tex = Texture {
            width: 100,
            height: 100,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, 100, 100, 10, 10);
        assert_eq!(images.len(), 100);
        // Each cell should be 10x10
        assert_eq!(images[0].region_width, 10);
        assert_eq!(images[0].region_height, 10);
    }

    #[test]
    fn test_source_image_from_texture_offset_xy() {
        let tex = Texture {
            width: 200,
            height: 200,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 50, 60, 100, 80, 2, 1);
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].region_x, 50);
        assert_eq!(images[0].region_y, 60);
        assert_eq!(images[0].region_width, 50);
        assert_eq!(images[0].region_height, 80);
        assert_eq!(images[1].region_x, 100);
        assert_eq!(images[1].region_y, 60);
    }

    #[test]
    fn test_source_image_from_texture_single_cell() {
        let tex = Texture {
            width: 64,
            height: 32,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, 64, 32, 1, 1);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].region_width, 64);
        assert_eq!(images[0].region_height, 32);
    }

    #[test]
    fn test_source_image_from_texture_w_h_zero_produces_zero_size_regions() {
        let tex = Texture {
            width: 64,
            height: 64,
            ..Default::default()
        };
        // w=0, h=0 (not -1) should produce zero-size regions
        let images = LR2SkinCSVLoaderState::source_image_from_texture(&tex, 0, 0, 0, 0, 1, 1);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].region_width, 0);
        assert_eq!(images[0].region_height, 0);
    }

    // --- build_gauge_image_array additional tests ---

    #[test]
    fn test_build_gauge_standard_two_states() {
        let mut state = make_state();
        let tex = Texture {
            width: 160,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 160;
        values[6] = 20;
        values[14] = 0; // standard

        // 8 divx, 1 divy -> total=8, standard: 4 per state = 2 states
        let gauge = state.build_gauge_image_array(&values, 8, 1, 8, false);
        assert_eq!(gauge.len(), 2);
        assert_eq!(gauge[0].len(), 36);
        assert_eq!(gauge[1].len(), 36);
    }

    #[test]
    fn test_build_gauge_pms_two_states() {
        let mut state = make_state();
        let tex = Texture {
            width: 240,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 240;
        values[6] = 20;
        values[14] = 3; // PMS mode

        // 12 divx, 1 divy -> total=12, PMS: 6 per state = 2 states
        let gauge = state.build_gauge_image_array(&values, 12, 1, 12, false);
        assert_eq!(gauge.len(), 2);
    }

    // --- process_csv_command: multiple commands ---

    #[test]
    fn test_startinput_large_value() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT", "999999"]), None);
        assert_eq!(state.skin_input, Some(999999));
    }

    #[test]
    fn test_scenetime_max_i32() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "2147483647"]), None);
        assert_eq!(state.skin_scene, Some(i32::MAX));
    }

    #[test]
    fn test_fadeout_overflow_string_returns_none() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "99999999999"]), None);
        // Overflow for i32 -> parse fails -> None
        assert_eq!(state.skin_fadeout, None);
    }

    // --- finalize_active_objects collects objects ---

    #[test]
    fn test_finalize_collects_button_onmouse_gauger() {
        // Just verify that finalize moves items from active slots to collected_objects
        let mut state = make_state();
        assert!(state.button.is_none());
        assert!(state.onmouse.is_none());
        assert!(state.gauger.is_none());
        state.finalize_active_objects();
        assert!(state.collected_objects.is_empty());
    }

    #[test]
    fn test_src_button_length_exceeds_image_count_no_panic() {
        // Regression: when SRC_BUTTON length > srcimg.len(), group_size becomes 0
        // and chunks(0) would panic. The guard should produce an empty images vec.
        let mut state = make_state();
        let tex = Texture {
            width: 20,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));

        // SRC_BUTTON with: gr=0, x=0, y=0, w=20, h=20, divx=1, divy=1 -> 1 srcimg
        // length=10 -> group_size = 1 / 10 = 0 -> must not panic
        let parts = str_vec(&[
            "#SRC_BUTTON",
            "0",  // str_parts[1] unused for button id
            "0",  // gr
            "0",  // x
            "0",  // y
            "20", // w
            "20", // h
            "1",  // divx
            "1",  // divy
            "0",  // timer
            "0",  // cycle
            "0",  // ref_id
            "0",  // clickevent
            "0",  // _
            "0",  // click_type
            "10", // length (exceeds srcimg count of 1)
        ]);
        // Should not panic
        state.process_csv_command("SRC_BUTTON", &parts, None);
        // With group_size == 0, images is empty -> no button created
        assert!(state.button.is_none());
    }

    /// Verify that DST_IMAGE passes values[18-20] as draw condition ops and
    /// position 21+ as offset IDs (Java parity).
    /// Bug: the code was calling set_destination_with_int_timer_ops which
    /// treated offsets as draw condition ops and ignored values[18-20] entirely.
    #[test]
    fn test_dst_image_passes_ops_and_offsets_correctly() {
        use crate::objects::skin_image::SkinImage;
        use crate::reexports::TextureRegion;

        let mut state = make_state();
        // Set src/dst to 1:1 to simplify coordinate math
        state.src = Resolution {
            width: 640.0,
            height: 480.0,
        };
        state.dst = Resolution {
            width: 640.0,
            height: 480.0,
        };

        // Directly inject a SkinImage (bypass SRC_IMAGE which needs a texture)
        let regions = vec![TextureRegion::new()];
        state.image = Some(SkinImage::new_with_int_timer(regions, 0, 0));

        // Build DST_IMAGE CSV parts:
        // #DST_IMAGE, id, time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop, timer, op1, op2, op3, offset...
        // indices:     0   1     2   3  4  5  6    7  8  9 10 11  12     13    14     15      16    17    18   19   20   21+
        let parts = str_vec(&[
            "#DST_IMAGE",
            "0",   // id (values[1])
            "0",   // time (values[2])
            "10",  // x (values[3])
            "20",  // y (values[4])
            "100", // w (values[5])
            "50",  // h (values[6])
            "0",   // acc (values[7])
            "255", // a (values[8])
            "255", // r (values[9])
            "255", // g (values[10])
            "255", // b (values[11])
            "0",   // blend (values[12])
            "0",   // filter (values[13])
            "0",   // angle (values[14])
            "0",   // center (values[15])
            "0",   // loop (values[16])
            "0",   // timer (values[17])
            "42",  // op1 (values[18]) -- draw condition op
            "43",  // op2 (values[19]) -- draw condition op
            "0",   // op3 (values[20]) -- zero = ignored
            "5",   // offset[0] at str index 21
            "10",  // offset[1] at str index 22
        ]);
        state.process_csv_command("DST_IMAGE", &parts, None);

        let image = state.image.as_ref().expect("image should still exist");
        // Verify offset IDs are set from position 21+ (values 5 and 10)
        let offsets = image.data.offset_id();
        let mut sorted_offsets = offsets.to_vec();
        sorted_offsets.sort();
        assert_eq!(
            sorted_offsets,
            vec![5, 10],
            "offset IDs should come from str positions 21+ (values 5, 10), not be empty"
        );
    }
}
