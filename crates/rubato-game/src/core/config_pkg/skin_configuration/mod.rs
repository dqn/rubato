#[cfg(test)]
use std::path::PathBuf;

use crate::core::main_state::MainStateData;
use crate::core::player_config::PlayerConfig;
use crate::core::skin_config::SkinConfig;
#[cfg(test)]
use crate::core::skin_config::{SkinOption, SkinProperty};
#[cfg(test)]
use crate::core::timer_manager::TimerManager;
use rubato_types::skin_type::SkinType;

mod applier;
mod types;

pub use types::*;

/// Skin configuration screen.
/// Translated from Java: SkinConfiguration extends MainState
pub struct SkinConfiguration {
    state_data: MainStateData,
    skin_type: Option<SkinType>,
    config: Option<SkinConfig>,
    pub all_skins: Vec<SkinHeaderInfo>,
    available_skins: Vec<SkinHeaderInfo>,
    selected_skin_index: i32,
    selected_skin_header: Option<SkinHeaderInfo>,
    custom_options: Option<Vec<CustomItem>>,
    custom_option_offset: i32,
    custom_option_offset_max: i32,
    player: PlayerConfig,
    pub custom_property_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_header(path: &str, skin_type: SkinType) -> SkinHeaderInfo {
        SkinHeaderInfo {
            path: Some(PathBuf::from(path)),
            skin_type: Some(skin_type),
            name: Some(format!("Test Skin {}", path)),
            ..SkinHeaderInfo::default()
        }
    }

    fn make_config_with_path(path: &str) -> SkinConfig {
        SkinConfig {
            path: Some(path.to_string()),
            properties: Some(SkinProperty::default()),
        }
    }

    /// Helper to create a minimal SkinConfiguration for testing (bypasses MainController).
    fn make_test_skin_config() -> SkinConfiguration {
        SkinConfiguration {
            state_data: MainStateData::new(TimerManager::new()),
            skin_type: None,
            config: Some(SkinConfig {
                path: Some("skin/test.json".to_string()),
                properties: Some(SkinProperty::default()),
            }),
            all_skins: Vec::new(),
            available_skins: Vec::new(),
            selected_skin_index: -1,
            selected_skin_header: None,
            custom_options: None,
            custom_option_offset: 0,
            custom_option_offset_max: 0,
            player: PlayerConfig::default(),
            custom_property_count: -1,
        }
    }

    #[test]
    fn test_get_selected_skin_header_none() {
        let sc = make_test_skin_config();
        assert!(sc.selected_skin_header().is_none());
    }

    #[test]
    fn test_get_selected_skin_header_some() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(make_test_header("skin/play7.json", SkinType::Play7Keys));
        let header = sc.selected_skin_header().unwrap();
        assert_eq!(header.path, Some(PathBuf::from("skin/play7.json")));
        assert_eq!(header.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_set_file_path_new() {
        let mut sc = make_test_skin_config();
        sc.set_file_path("bg_image", "background.png");

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.file.len(), 1);
        let f = props.file[0].as_ref().unwrap();
        assert_eq!(f.name.as_deref(), Some("bg_image"));
        assert_eq!(f.path.as_deref(), Some("background.png"));
    }

    #[test]
    fn test_set_file_path_update_existing() {
        let mut sc = make_test_skin_config();
        sc.set_file_path("bg_image", "old.png");
        sc.set_file_path("bg_image", "new.png");

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.file.len(), 1);
        let f = props.file[0].as_ref().unwrap();
        assert_eq!(f.path.as_deref(), Some("new.png"));
    }

    #[test]
    fn test_set_custom_option_new() {
        let mut sc = make_test_skin_config();
        sc.set_custom_option("judge_timing", 42);

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.option.len(), 1);
        let o = props.option[0].as_ref().unwrap();
        assert_eq!(o.name.as_deref(), Some("judge_timing"));
        assert_eq!(o.value, 42);
    }

    #[test]
    fn test_set_custom_option_update_existing() {
        let mut sc = make_test_skin_config();
        sc.set_custom_option("judge_timing", 42);
        sc.set_custom_option("judge_timing", 100);

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.option.len(), 1);
        let o = props.option[0].as_ref().unwrap();
        assert_eq!(o.value, 100);
    }

    #[test]
    fn test_set_custom_offset_new() {
        let mut sc = make_test_skin_config();
        sc.set_custom_offset("judge_offset", 0, 10); // x = 10

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.offset.len(), 1);
        let o = props.offset[0].as_ref().unwrap();
        assert_eq!(o.name.as_deref(), Some("judge_offset"));
        assert_eq!(o.x, 10);
        assert_eq!(o.y, 0);
    }

    #[test]
    fn test_set_custom_offset_update_existing() {
        let mut sc = make_test_skin_config();
        sc.set_custom_offset("judge_offset", 0, 10); // x = 10
        sc.set_custom_offset("judge_offset", 1, 20); // y = 20

        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.offset.len(), 1);
        let o = props.offset[0].as_ref().unwrap();
        assert_eq!(o.x, 10);
        assert_eq!(o.y, 20);
    }

    #[test]
    fn test_set_custom_offset_all_kinds() {
        let mut sc = make_test_skin_config();
        for kind in 0..6 {
            sc.set_custom_offset("test", kind, (kind as i32 + 1) * 10);
        }
        // First call creates, subsequent calls update
        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        assert_eq!(props.offset.len(), 1);
        let o = props.offset[0].as_ref().unwrap();
        assert_eq!(o.x, 10);
        assert_eq!(o.y, 20);
        assert_eq!(o.w, 30);
        assert_eq!(o.h, 40);
        assert_eq!(o.r, 50);
        assert_eq!(o.a, 60);
    }

    #[test]
    fn test_save_skin_history_new_entry() {
        let mut sc = make_test_skin_config();
        sc.config = Some(make_config_with_path("skin/play7.json"));
        assert!(sc.player.skin_history.is_empty());

        sc.save_skin_history();
        assert_eq!(sc.player.skin_history.len(), 1);
        assert_eq!(sc.player.skin_history[0].path(), Some("skin/play7.json"));
    }

    #[test]
    fn test_save_skin_history_update_existing() {
        let mut sc = make_test_skin_config();
        sc.player.skin_history.push(SkinConfig {
            path: Some("skin/play7.json".to_string()),
            properties: None,
        });
        sc.config = Some(SkinConfig {
            path: Some("skin/play7.json".to_string()),
            properties: Some(SkinProperty::default()),
        });

        sc.save_skin_history();
        assert_eq!(sc.player.skin_history.len(), 1);
        // Should have updated properties (not None anymore)
        assert!(sc.player.skin_history[0].properties.is_some());
    }

    #[test]
    fn test_save_skin_history_no_config() {
        let mut sc = make_test_skin_config();
        sc.config = None;
        sc.save_skin_history();
        assert!(sc.player.skin_history.is_empty());
    }

    #[test]
    fn test_save_skin_history_empty_path() {
        let mut sc = make_test_skin_config();
        sc.config = Some(SkinConfig {
            path: Some(String::new()),
            properties: None,
        });
        sc.save_skin_history();
        assert!(sc.player.skin_history.is_empty());
    }

    #[test]
    fn test_change_skin_type_filters_available() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/select.json", SkinType::MusicSelect),
            make_test_header("skin/play7_alt.json", SkinType::Play7Keys),
        ];

        sc.change_skin_type(Some(SkinType::Play7Keys));
        assert_eq!(sc.available_skins.len(), 2);
        assert_eq!(sc.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_change_skin_type_defaults_to_play7keys() {
        let mut sc = make_test_skin_config();
        sc.change_skin_type(None);
        assert_eq!(sc.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_change_skin_type_selects_matching_config_path() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/play7_alt.json", SkinType::Play7Keys),
        ];
        // Set up player config with a skin path for Play7Keys (id=0)
        if sc.player.skin.is_empty() {
            sc.player.skin.resize_with(19, || None);
        }
        sc.player.skin[0] = Some(make_config_with_path("skin/play7_alt.json"));

        sc.change_skin_type(Some(SkinType::Play7Keys));
        assert_eq!(sc.selected_skin_index, 1); // second entry
    }

    #[test]
    fn test_select_skin_positive_index() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/b.json"));

        sc.select_skin(1);
        assert_eq!(sc.selected_skin_index, 1);
        let header = sc.selected_skin_header.as_ref().unwrap();
        assert_eq!(header.path, Some(PathBuf::from("skin/b.json")));
        assert!(sc.custom_options.is_some());
    }

    #[test]
    fn test_select_skin_negative_clears() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(make_test_header("skin/a.json", SkinType::Play7Keys));
        sc.custom_options = Some(vec![]);

        sc.select_skin(-1);
        assert_eq!(sc.selected_skin_index, -1);
        assert!(sc.selected_skin_header.is_none());
        assert!(sc.custom_options.is_none());
    }

    #[test]
    fn test_set_other_skin_wraps_forward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
            make_test_header("skin/c.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/c.json"));
        sc.selected_skin_index = 2; // last skin

        sc.set_other_skin(1); // should wrap to 0
        assert_eq!(sc.selected_skin_index, 0);
    }

    #[test]
    fn test_set_other_skin_wraps_backward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
            make_test_header("skin/c.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/a.json"));
        sc.selected_skin_index = 0;

        sc.set_other_skin(-1); // should wrap to 2
        assert_eq!(sc.selected_skin_index, 2);
    }

    #[test]
    fn test_set_next_prev_skin() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/a.json"));
        sc.selected_skin_index = 0;

        sc.set_next_skin();
        assert_eq!(sc.selected_skin_index, 1);

        sc.set_prev_skin();
        assert_eq!(sc.selected_skin_index, 0);
    }

    #[test]
    fn test_set_other_skin_empty_available() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![];
        sc.selected_skin_index = 0;

        sc.set_other_skin(1); // should not panic
        assert_eq!(sc.selected_skin_index, 0); // unchanged
    }

    #[test]
    fn test_update_custom_options_basic() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_options: vec![CustomOptionDef {
                name: "judge_type".to_string(),
                option: vec![0, 1, 2],
                contents: vec!["Normal".to_string(), "Hard".to_string(), "Easy".to_string()],
                def: Some("Normal".to_string()),
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_options();

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].category_name(), "judge_type");
        // "Normal" + "Hard" + "Easy" + "Random" = 4 display values, max = 3
        assert_eq!(options[0].max(), 3);
        assert_eq!(options[0].value(), 0); // default selection = 0 (Normal)
    }

    #[test]
    fn test_update_custom_options_with_saved_value() {
        let mut sc = make_test_skin_config();
        // Set up saved option
        {
            let props = sc.ensure_properties();
            props.option.push(Some(SkinOption {
                name: Some("judge_type".to_string()),
                value: 2, // "Easy"
            }));
        }
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_options: vec![CustomOptionDef {
                name: "judge_type".to_string(),
                option: vec![0, 1, 2],
                contents: vec!["Normal".to_string(), "Hard".to_string(), "Easy".to_string()],
                def: None,
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_options();

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].value(), 2); // Easy is at index 2
    }

    #[test]
    fn test_update_custom_options_random_value() {
        let mut sc = make_test_skin_config();
        {
            let props = sc.ensure_properties();
            props.option.push(Some(SkinOption {
                name: Some("judge_type".to_string()),
                value: OPTION_RANDOM_VALUE,
            }));
        }
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_options: vec![CustomOptionDef {
                name: "judge_type".to_string(),
                option: vec![0, 1],
                contents: vec!["Normal".to_string(), "Hard".to_string()],
                def: None,
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_options();

        let options = sc.custom_options.as_ref().unwrap();
        // Random is at index option.len() = 2
        assert_eq!(options[0].value(), 2);
        assert_eq!(options[0].display_value(), "Random");
    }

    #[test]
    fn test_update_custom_offsets_basic() {
        let mut sc = make_test_skin_config();
        sc.selected_skin_header = Some(SkinHeaderInfo {
            custom_offsets: vec![CustomOffsetDef {
                name: "judge_pos".to_string(),
                caps: rubato_types::offset_capabilities::OffsetCapabilities {
                    x: true,
                    y: true,
                    ..Default::default()
                },
            }],
            ..SkinHeaderInfo::default()
        });
        sc.custom_options = Some(Vec::new());

        sc.update_custom_offsets();

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options.len(), 2); // x and y enabled
        assert_eq!(options[0].category_name(), "judge_pos - x");
        assert_eq!(options[1].category_name(), "judge_pos - y");
        assert_eq!(options[0].min(), -9999);
        assert_eq!(options[0].max(), 9999);
    }

    #[test]
    fn test_custom_item_option_set_value() {
        let mut item = CustomItem::Option {
            category_name: "test".to_string(),
            contents: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            options: vec![10, 20, 30],
            selection: 0,
            display_value: "A".to_string(),
        };
        assert_eq!(item.value(), 0);
        assert_eq!(item.display_value(), "A");

        // Simulate changing selection
        if let CustomItem::Option {
            ref mut selection,
            ref mut display_value,
            ref contents,
            ..
        } = item
        {
            *selection = 2;
            *display_value = contents[2].clone();
        }
        assert_eq!(item.value(), 2);
        assert_eq!(item.display_value(), "C");
    }

    #[test]
    fn test_custom_item_offset_properties() {
        let item = CustomItem::Offset {
            category_name: "pos - x".to_string(),
            offset_name: "pos".to_string(),
            kind: 0,
            min: -100,
            max: 100,
            value: 42,
        };
        assert_eq!(item.category_name(), "pos - x");
        assert_eq!(item.value(), 42);
        assert_eq!(item.min(), -100);
        assert_eq!(item.max(), 100);
        assert_eq!(item.display_value(), "42");
    }

    #[test]
    fn test_extract_file_pattern_simple() {
        assert_eq!(
            SkinConfiguration::extract_file_pattern("skin/images/*.png"),
            "*.png"
        );
    }

    #[test]
    fn test_extract_file_pattern_with_pipe() {
        // "skin/images/bg*.png|.jpg" -> "bg*.png.jpg"
        assert_eq!(
            SkinConfiguration::extract_file_pattern("skin/images/bg*.png|.jpg"),
            "bg*.png.jpg"
        );
    }

    #[test]
    fn test_extract_file_pattern_with_pipe_empty_suffix() {
        // "skin/images/bg*.png|" -> "bg*.png"
        assert_eq!(
            SkinConfiguration::extract_file_pattern("skin/images/bg*.png|"),
            "bg*.png"
        );
    }

    #[test]
    fn test_get_category_name_with_offset() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![
            CustomItem::Option {
                category_name: "first".to_string(),
                contents: vec![],
                options: vec![],
                selection: 0,
                display_value: String::new(),
            },
            CustomItem::Option {
                category_name: "second".to_string(),
                contents: vec![],
                options: vec![],
                selection: 0,
                display_value: String::new(),
            },
        ]);
        sc.custom_option_offset = 1;

        assert_eq!(sc.category_name(0), "second");
    }

    #[test]
    fn test_get_category_name_out_of_bounds() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![]);
        assert_eq!(sc.category_name(0), "");
    }

    #[test]
    fn test_ensure_properties_creates_config() {
        let mut sc = make_test_skin_config();
        sc.config = None;
        let _props = sc.ensure_properties();
        assert!(sc.config.is_some());
        assert!(sc.config.as_ref().unwrap().properties.is_some());
    }

    #[test]
    fn test_skin_select_position() {
        let mut sc = make_test_skin_config();
        sc.custom_option_offset_max = 10;

        sc.set_skin_select_position(0.5);
        assert_eq!(sc.custom_option_offset, 5);
        assert!((sc.skin_select_position() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_skin_select_position_zero_max() {
        let sc = make_test_skin_config();
        assert_eq!(sc.skin_select_position(), 0.0);
    }

    #[test]
    fn test_load_all_skins_stub_no_panic() {
        let mut sc = make_test_skin_config();
        // Should not panic even though no skin dir exists
        sc.load_all_skins(&|_path| Vec::new());
        assert!(sc.all_skins.is_empty());
    }

    #[test]
    fn test_set_all_skins() {
        let mut sc = make_test_skin_config();
        let skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::MusicSelect),
        ];
        sc.all_skins = skins;
        assert_eq!(sc.all_skins.len(), 2);
    }

    // ----------------------------------------------------------------
    // execute_event tests
    // ----------------------------------------------------------------

    #[test]
    fn test_execute_event_change_skin_forward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/a.json"));
        sc.selected_skin_index = 0;

        // BUTTON_CHANGE_SKIN = 190, arg1 >= 0 => set_next_skin
        sc.execute_event(BUTTON_CHANGE_SKIN, 1, 0);
        assert_eq!(sc.selected_skin_index, 1);
    }

    #[test]
    fn test_execute_event_change_skin_backward() {
        let mut sc = make_test_skin_config();
        sc.available_skins = vec![
            make_test_header("skin/a.json", SkinType::Play7Keys),
            make_test_header("skin/b.json", SkinType::Play7Keys),
        ];
        sc.config = Some(make_config_with_path("skin/b.json"));
        sc.selected_skin_index = 1;

        // arg1 < 0 => set_prev_skin
        sc.execute_event(BUTTON_CHANGE_SKIN, -1, 0);
        assert_eq!(sc.selected_skin_index, 0);
    }

    #[test]
    fn test_execute_event_customize_button_increment() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec![
                "Normal".to_string(),
                "Hard".to_string(),
                "Random".to_string(),
            ],
            options: vec![0, 1, OPTION_RANDOM_VALUE],
            selection: 0,
            display_value: "Normal".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // BUTTON_SKIN_CUSTOMIZE1 = 220, arg1 >= 0 => increment
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].value(), 1); // selection moved to index 1
        assert_eq!(options[0].display_value(), "Hard");
    }

    #[test]
    fn test_execute_event_customize_button_decrement() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec![
                "Normal".to_string(),
                "Hard".to_string(),
                "Random".to_string(),
            ],
            options: vec![0, 1, OPTION_RANDOM_VALUE],
            selection: 1,
            display_value: "Hard".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // arg1 < 0 => decrement
        sc.execute_event(220, -1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].value(), 0);
        assert_eq!(options[0].display_value(), "Normal");
    }

    #[test]
    fn test_execute_event_customize_button_wrap_forward() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec!["A".to_string(), "B".to_string()],
            options: vec![0, 1],
            selection: 1, // at max
            display_value: "B".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // At max, increment should wrap to min (0)
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].value(), 0);
        assert_eq!(options[0].display_value(), "A");
    }

    #[test]
    fn test_execute_event_customize_button_wrap_backward() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "judge_type".to_string(),
            contents: vec!["A".to_string(), "B".to_string()],
            options: vec![0, 1],
            selection: 0, // at min
            display_value: "A".to_string(),
        }]);
        sc.custom_option_offset = 0;

        // At min, decrement should wrap to max (1)
        sc.execute_event(220, -1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].value(), 1);
        assert_eq!(options[0].display_value(), "B");
    }

    #[test]
    fn test_execute_event_customize_button_with_offset() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![
            CustomItem::Option {
                category_name: "first".to_string(),
                contents: vec!["X".to_string(), "Y".to_string()],
                options: vec![10, 20],
                selection: 0,
                display_value: "X".to_string(),
            },
            CustomItem::Option {
                category_name: "second".to_string(),
                contents: vec!["A".to_string(), "B".to_string(), "C".to_string()],
                options: vec![100, 200, 300],
                selection: 0,
                display_value: "A".to_string(),
            },
        ]);
        sc.custom_option_offset = 1; // offset by 1, so CUSTOMIZE1 (index 0) maps to items[1]

        // BUTTON_SKIN_CUSTOMIZE1 = 220, index = 0 + offset 1 = items[1]
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        // First item should be unchanged
        assert_eq!(options[0].value(), 0);
        // Second item should have incremented
        assert_eq!(options[1].value(), 1);
        assert_eq!(options[1].display_value(), "B");
    }

    #[test]
    fn test_execute_event_customize_persists_option() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Option {
            category_name: "my_opt".to_string(),
            contents: vec!["Off".to_string(), "On".to_string()],
            options: vec![0, 42],
            selection: 0,
            display_value: "Off".to_string(),
        }]);
        sc.custom_option_offset = 0;

        sc.execute_event(220, 1, 0); // increment to selection=1, option value=42

        // Verify the option was persisted to config
        let props = sc.config.as_ref().unwrap().properties.as_ref().unwrap();
        let saved = props
            .option
            .iter()
            .flatten()
            .find(|o| o.name.as_deref() == Some("my_opt"));
        assert!(saved.is_some());
        assert_eq!(saved.unwrap().value, 42);
    }

    #[test]
    fn test_execute_event_customize_offset_item() {
        let mut sc = make_test_skin_config();
        sc.custom_options = Some(vec![CustomItem::Offset {
            category_name: "pos - x".to_string(),
            offset_name: "pos".to_string(),
            kind: 0,
            min: -9999,
            max: 9999,
            value: 50,
        }]);
        sc.custom_option_offset = 0;

        // Increment from 50 to 51
        sc.execute_event(220, 1, 0);

        let options = sc.custom_options.as_ref().unwrap();
        assert_eq!(options[0].value(), 51);
    }

    #[test]
    fn test_execute_event_skin_select_type() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/select.json", SkinType::MusicSelect),
        ];

        // BUTTON_SKINSELECT_7KEY = 170 => SkinType::Play7Keys (id 0)
        sc.execute_event(170, 0, 0);
        assert_eq!(sc.skin_type, Some(SkinType::Play7Keys));
        assert_eq!(sc.available_skins.len(), 1);
    }

    #[test]
    fn test_execute_event_skin_select_type_music_select() {
        let mut sc = make_test_skin_config();
        sc.all_skins = vec![
            make_test_header("skin/play7.json", SkinType::Play7Keys),
            make_test_header("skin/select.json", SkinType::MusicSelect),
        ];

        // BUTTON_SKINSELECT_MUSIC_SELECT = 175 (7KEY=170, offset 5 = MusicSelect)
        sc.execute_event(175, 0, 0);
        assert_eq!(sc.skin_type, Some(SkinType::MusicSelect));
        assert_eq!(sc.available_skins.len(), 1);
    }

    #[test]
    fn test_execute_event_unknown_id_no_panic() {
        let mut sc = make_test_skin_config();
        // Unknown event id — should not panic (falls through to no-op)
        sc.execute_event(9999, 0, 0);
    }

    // ----------------------------------------------------------------
    // Local SkinPropertyMapper function tests
    // ----------------------------------------------------------------

    #[test]
    fn test_is_skin_customize_button() {
        // Range: [220, 229] inclusive — all 10 slots
        assert!(is_skin_customize_button(220));
        assert!(is_skin_customize_button(224));
        assert!(is_skin_customize_button(228));
        assert!(is_skin_customize_button(229)); // slot 10
        assert!(!is_skin_customize_button(219));
        assert!(!is_skin_customize_button(230));
    }

    #[test]
    fn test_get_skin_customize_index() {
        assert_eq!(skin_customize_index(220), 0);
        assert_eq!(skin_customize_index(225), 5);
        assert_eq!(skin_customize_index(228), 8);
        assert_eq!(skin_customize_index(229), 9);
    }

    #[test]
    fn test_is_skin_select_type_id() {
        // Primary range: [170, 185]
        assert!(is_skin_select_type_id(170)); // 7KEY
        assert!(is_skin_select_type_id(185)); // COURSE_RESULT
        assert!(!is_skin_select_type_id(169));
        assert!(!is_skin_select_type_id(186));
        // 24KEY range: [386, 388]
        assert!(is_skin_select_type_id(386));
        assert!(is_skin_select_type_id(388));
        assert!(!is_skin_select_type_id(385));
        assert!(!is_skin_select_type_id(389));
    }

    #[test]
    fn test_get_skin_select_type() {
        // 170 = BUTTON_SKINSELECT_7KEY => SkinType id 0 = Play7Keys
        assert_eq!(skin_select_type(170), Some(SkinType::Play7Keys));
        // 175 = Music Select => SkinType id 5 = MusicSelect
        assert_eq!(skin_select_type(175), Some(SkinType::MusicSelect));
        // Out of range
        assert_eq!(skin_select_type(0), None);
        assert_eq!(skin_select_type(999), None);
    }
}
