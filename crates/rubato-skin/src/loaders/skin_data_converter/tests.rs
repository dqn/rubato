use super::*;
use crate::json::json_skin_loader::{
    CustomEventData, CustomOffsetData, CustomOptionData, CustomTimerData, DestinationData,
    RectData, SkinData, SkinHeaderData, SkinObjectData as DataSkinObjectData,
};
use crate::stubs::Resolution;

fn make_test_header_data() -> SkinHeaderData {
    SkinHeaderData {
        skin_type: 1, // Play7Keys
        name: "Test Skin".to_string(),
        author: "Test Author".to_string(),
        path: std::path::PathBuf::from("/test/skin.json"),
        header_type: 0,
        custom_options: vec![],
        custom_files: vec![],
        custom_offsets: vec![],
        custom_categories: vec![],
        source_resolution: Some(Resolution {
            width: 1920.0,
            height: 1080.0,
        }),
        destination_resolution: None,
    }
}

fn make_test_dst() -> Resolution {
    Resolution {
        width: 1920.0,
        height: 1080.0,
    }
}

// -- Test: header conversion --

#[test]
fn test_convert_header_data_basic() {
    let header_data = make_test_header_data();
    let src = Resolution {
        width: 1920.0,
        height: 1080.0,
    };
    let dst = make_test_dst();

    let header = convert_header_data(&header_data, &src, &dst);

    assert_eq!(header.name(), Some("Test Skin"));
    assert_eq!(header.author(), Some("Test Author"));
    assert_eq!(header.source_resolution().width, 1920.0);
    assert_eq!(header.source_resolution().height, 1080.0);
    assert_eq!(header.destination_resolution().width, 1920.0);
    assert_eq!(header.destination_resolution().height, 1080.0);
}

#[test]
fn test_convert_header_with_options() {
    let mut header_data = make_test_header_data();
    header_data.custom_options = vec![CustomOptionData {
        name: "Option1".to_string(),
        option: vec![100, 101, 102],
        names: vec!["A".to_string(), "B".to_string(), "C".to_string()],
        def: None,
        selected_option: 101,
    }];

    let src = Resolution {
        width: 1920.0,
        height: 1080.0,
    };
    let dst = make_test_dst();
    let header = convert_header_data(&header_data, &src, &dst);

    assert_eq!(header.custom_options().len(), 1);
    assert_eq!(header.custom_options()[0].name, "Option1");
    assert_eq!(header.custom_options()[0].option, vec![100, 101, 102]);
    assert_eq!(header.custom_options()[0].selected_index, 1);
}

#[test]
fn test_convert_header_with_offsets() {
    let mut header_data = make_test_header_data();
    header_data.custom_offsets = vec![CustomOffsetData {
        name: "Offset1".to_string(),
        id: 900,
        x: true,
        y: true,
        w: false,
        h: false,
        r: false,
        a: false,
    }];

    let src = Resolution {
        width: 1920.0,
        height: 1080.0,
    };
    let dst = make_test_dst();
    let header = convert_header_data(&header_data, &src, &dst);

    assert_eq!(header.custom_offsets().len(), 1);
    assert_eq!(header.custom_offsets()[0].name, "Offset1");
    assert_eq!(header.custom_offsets()[0].id, 900);
}

// -- Test: empty SkinData -> Skin --

#[test]
fn test_convert_empty_skin_data() {
    let header_data = make_test_header_data();
    let data = SkinData::new();
    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    assert!(skin.is_some());
    let skin = skin.unwrap();
    assert_eq!(skin.all_skin_objects_count(), 0);
    assert_eq!(skin.custom_events_count(), 0);
    assert_eq!(skin.custom_timers_count(), 0);
}

// -- Test: skin with ImageById object --

#[test]
fn test_convert_skin_data_with_image_by_id() {
    let header_data = make_test_header_data();
    let mut data = SkinData::new();
    data.objects.push(DataSkinObjectData {
        name: Some("-1".to_string()),
        object_type: SkinObjectType::ImageById(1),
        destinations: vec![DestinationData {
            time: 0,
            x: 100,
            y: 200,
            w: 300,
            h: 400,
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
            timer: None,
            op: vec![],
            draw: None,
        }],
        offset_ids: vec![],
        stretch: -1,
        mouse_rect: None,
    });

    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    assert!(skin.is_some());
    let skin = skin.unwrap();
    assert_eq!(skin.all_skin_objects_count(), 1);
    assert_eq!(skin.objects()[0].type_name(), "Image");
}

// -- Test: option wiring --

#[test]
fn test_option_wiring() {
    let mut header_data = make_test_header_data();
    header_data.custom_options = vec![CustomOptionData {
        name: "TestOpt".to_string(),
        option: vec![200, 201],
        names: vec!["Off".to_string(), "On".to_string()],
        def: None,
        selected_option: 201,
    }];

    let data = SkinData::new();
    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    let skin = skin.unwrap();
    let option = skin.option();
    // 200 is not selected => 0, 201 is selected => 1
    assert_eq!(option.get(&200), Some(&0));
    assert_eq!(option.get(&201), Some(&1));
}

// -- Test: offset wiring --

#[test]
fn test_offset_wiring() {
    let mut header_data = make_test_header_data();
    header_data.custom_offsets = vec![CustomOffsetData {
        name: "TestOffset".to_string(),
        id: 42,
        x: true,
        y: true,
        w: false,
        h: false,
        r: false,
        a: false,
    }];

    let data = SkinData::new();
    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    let skin = skin.unwrap();
    let offset = skin.offset();
    assert!(offset.contains_key(&42));
    assert_eq!(offset.get(&42).unwrap().name, "TestOffset");
}

// -- Test: fadeout/input/scene wiring --

#[test]
fn test_fadeout_input_scene() {
    let header_data = make_test_header_data();
    let mut data = SkinData::new();
    data.fadeout = 500;
    data.input = 100;
    data.scene = 60000;

    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    let skin = skin.unwrap();
    assert_eq!(skin.fadeout(), 500);
    assert_eq!(skin.input(), 100);
    assert_eq!(skin.scene(), 60000);
}

// -- Test: custom event/timer registration --

#[test]
fn test_custom_timer_registration() {
    let header_data = make_test_header_data();
    let mut data = SkinData::new();
    data.custom_timers.push(CustomTimerData {
        id: 10,
        timer: None,
    });
    data.custom_timers.push(CustomTimerData {
        id: 20,
        timer: Some(42),
    });

    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    let skin = skin.unwrap();
    assert_eq!(skin.custom_timers_count(), 2);
}

// -- Test: conversion with destinations --

#[test]
fn test_convert_with_destinations() {
    let header_data = make_test_header_data();
    let mut data = SkinData::new();
    data.objects.push(DataSkinObjectData {
        name: Some("-5".to_string()),
        object_type: SkinObjectType::ImageById(5),
        destinations: vec![
            DestinationData {
                time: 0,
                x: 0,
                y: 0,
                w: 100,
                h: 100,
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
                timer: None,
                op: vec![],
                draw: None,
            },
            DestinationData {
                time: 1000,
                x: 100,
                y: 100,
                w: 200,
                h: 200,
                acc: 0,
                a: 128,
                r: 255,
                g: 255,
                b: 255,
                blend: 0,
                filter: 0,
                angle: 0,
                center: 0,
                loop_val: 0,
                timer: None,
                op: vec![],
                draw: None,
            },
        ],
        offset_ids: vec![],
        stretch: -1,
        mouse_rect: None,
    });

    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    let skin = skin.unwrap();
    assert_eq!(skin.all_skin_objects_count(), 1);
    // The object should have 2 destinations set via set_destination
    // We can verify the object data has destinations
    let obj = &skin.objects()[0];
    assert_eq!(obj.data().dst.len(), 2);
}

// -- Test: mouse rect --

#[test]
fn test_convert_with_mouse_rect() {
    let header_data = make_test_header_data();
    let mut data = SkinData::new();
    data.objects.push(DataSkinObjectData {
        name: Some("-1".to_string()),
        object_type: SkinObjectType::ImageById(1),
        destinations: vec![DestinationData {
            time: 0,
            x: 0,
            y: 0,
            w: 100,
            h: 100,
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
            timer: None,
            op: vec![],
            draw: None,
        }],
        offset_ids: vec![],
        stretch: -1,
        mouse_rect: Some(RectData {
            x: 10,
            y: 20,
            w: 30,
            h: 40,
        }),
    });

    let mut source_map = HashMap::new();
    let dst = make_test_dst();

    let skin = convert_skin_data(
        &header_data,
        data,
        &mut source_map,
        Path::new("/test/skin.json"),
        false,
        &dst,
    );

    let skin = skin.unwrap();
    assert_eq!(skin.all_skin_objects_count(), 1);
    // Mouse rect is set — verify via the object's mouse_rect field
    let obj = &skin.objects()[0];
    assert!(obj.data().mouse_rect.is_some());
}

// -- Test: stub types return None --

#[test]
fn test_bga_returns_some() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let bga = convert_skin_object(
        &SkinObjectType::Bga { bga_expand: 0 },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(bga.is_some());
    assert_eq!(bga.unwrap().type_name(), "SkinBGA");
}

#[test]
fn test_gauge_graph_from_color_strings() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let gg = convert_skin_object(
        &SkinObjectType::GaugeGraph {
            color: None,
            assist_clear_bg_color: "ff0000".to_string(),
            assist_and_easy_fail_bg_color: "00ff00".to_string(),
            groove_fail_bg_color: "0000ff".to_string(),
            groove_clear_and_hard_bg_color: "ffff00".to_string(),
            ex_hard_bg_color: "ff00ff".to_string(),
            hazard_bg_color: "00ffff".to_string(),
            assist_clear_line_color: "880000".to_string(),
            assist_and_easy_fail_line_color: "008800".to_string(),
            groove_fail_line_color: "000088".to_string(),
            groove_clear_and_hard_line_color: "888800".to_string(),
            ex_hard_line_color: "880088".to_string(),
            hazard_line_color: "008888".to_string(),
            borderline_color: "ffffff".to_string(),
            border_color: "444444".to_string(),
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(gg.is_some());
    assert_eq!(gg.unwrap().type_name(), "SkinGaugeGraph");
}

#[test]
fn test_gauge_graph_from_color_array() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let colors: Vec<String> = (0..24)
        .map(|i| format!("{:02x}{:02x}{:02x}", i * 10, 0, 0))
        .collect();
    let gg = convert_skin_object(
        &SkinObjectType::GaugeGraph {
            color: Some(colors),
            assist_clear_bg_color: String::new(),
            assist_and_easy_fail_bg_color: String::new(),
            groove_fail_bg_color: String::new(),
            groove_clear_and_hard_bg_color: String::new(),
            ex_hard_bg_color: String::new(),
            hazard_bg_color: String::new(),
            assist_clear_line_color: String::new(),
            assist_and_easy_fail_line_color: String::new(),
            groove_fail_line_color: String::new(),
            groove_clear_and_hard_line_color: String::new(),
            ex_hard_line_color: String::new(),
            hazard_line_color: String::new(),
            borderline_color: String::new(),
            border_color: String::new(),
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(gg.is_some());
    assert_eq!(gg.unwrap().type_name(), "SkinGaugeGraph");
}

#[test]
fn test_gauge_returns_some() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let gauge = convert_skin_object(
        &SkinObjectType::Gauge {
            nodes: vec!["n1".to_string(), "n2".to_string()],
            parts: 50,
            gauge_type: 0,
            range: 3,
            cycle: 33,
            starttime: 0,
            endtime: 500,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(gauge.is_some());
    assert_eq!(gauge.unwrap().type_name(), "SkinGauge");
}

#[test]
fn test_hidden_cover_no_texture_returns_none() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let hidden = convert_skin_object(
        &SkinObjectType::HiddenCover {
            src: Some("nonexistent".to_string()),
            x: 0,
            y: 0,
            w: 100,
            h: 200,
            divx: 1,
            divy: 1,
            timer: None,
            cycle: 0,
            disapear_line: 300,
            is_disapear_line_link_lift: true,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(hidden.is_none());
}

#[test]
fn test_lift_cover_no_texture_returns_none() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let lift = convert_skin_object(
        &SkinObjectType::LiftCover {
            src: Some("nonexistent".to_string()),
            x: 0,
            y: 0,
            w: 100,
            h: 200,
            divx: 1,
            divy: 1,
            timer: None,
            cycle: 0,
            disapear_line: 300,
            is_disapear_line_link_lift: false,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(lift.is_none());
}

#[test]
fn test_pmchara_returns_some() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let pm = convert_skin_object(
        &SkinObjectType::PmChara {
            src: Some("chara.png".to_string()),
            color: 1,
            chara_type: 0,
            side: 1,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(pm.is_some());
    // PmChara returns a placeholder SkinImage
    assert_eq!(pm.unwrap().type_name(), "Image");
}

#[test]
fn test_search_text_region_returns_none() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let sr = convert_skin_object(
        &SkinObjectType::SearchTextRegion {
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 30.0,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    // SearchTextRegion is a skin property, not a SkinObject
    assert!(sr.is_none());
}

#[test]
fn test_imageset_empty_returns_none() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let is = convert_skin_object(
        &SkinObjectType::ImageSet {
            images: vec![],
            ref_id: 0,
            value: None,
            act: None,
            click: 0,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(is.is_none());
}

#[test]
fn test_imageset_nonempty_returns_placeholder() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let is = convert_skin_object(
        &SkinObjectType::ImageSet {
            images: vec!["img1".to_string(), "img2".to_string()],
            ref_id: 42,
            value: None,
            act: None,
            click: 0,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(is.is_some());
    assert_eq!(is.unwrap().type_name(), "Image");
}

#[test]
fn test_note_judge_songlist_return_some() {
    let mut source_map = HashMap::new();
    let path = Path::new("/test/skin.json");

    let note = convert_skin_object(&SkinObjectType::Note, &mut source_map, path, false, 1.0);
    assert!(note.is_some());
    assert_eq!(note.unwrap().type_name(), "SkinNote");

    let judge = convert_skin_object(
        &SkinObjectType::Judge {
            index: 0,
            shift: false,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(judge.is_some());
    assert_eq!(judge.unwrap().type_name(), "SkinJudge");

    let bar = convert_skin_object(
        &SkinObjectType::SongList {
            center: 5,
            clickable: vec![],
            bar_data: None,
        },
        &mut source_map,
        path,
        false,
        1.0,
    );
    assert!(bar.is_some());
    assert_eq!(bar.unwrap().type_name(), "SkinBar");
}
