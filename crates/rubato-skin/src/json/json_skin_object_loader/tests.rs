use super::*;
use crate::json::json_skin;
use crate::json::json_skin_loader::SkinObjectType;

fn make_loader() -> JSONSkinLoader {
    JSONSkinLoader::new()
}

fn make_skin() -> SkinData {
    SkinData::new()
}

fn make_sk() -> json_skin::Skin {
    json_skin::Skin {
        w: 1920,
        h: 1080,
        ..Default::default()
    }
}

fn make_dst(id: &str) -> json_skin::Destination {
    json_skin::Destination {
        id: Some(id.to_string()),
        ..Default::default()
    }
}

#[test]
fn test_load_image_no_source() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.image.push(json_skin::Image {
        id: Some("img1".to_string()),
        src: Some("src1".to_string()),
        ..Default::default()
    });
    let dst = make_dst("img1");
    let p = std::path::Path::new("/fake/skin.json");

    // No source data loaded, so get_source returns None
    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_none());
}

#[test]
fn test_load_imageset() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    // Add image entries so resolution can find them
    sk.image.push(json_skin::Image {
        id: Some("a".to_string()),
        src: Some("src_a".to_string()),
        x: 0,
        y: 0,
        w: 32,
        h: 32,
        ..Default::default()
    });
    sk.image.push(json_skin::Image {
        id: Some("b".to_string()),
        src: Some("src_b".to_string()),
        x: 10,
        y: 20,
        w: 64,
        h: 64,
        ..Default::default()
    });
    sk.imageset.push(json_skin::ImageSet {
        id: Some("imgset1".to_string()),
        ref_id: 42,
        value: Some(100),
        images: vec!["a".to_string(), "b".to_string()],
        act: Some(10),
        click: 1,
    });
    let dst = make_dst("imgset1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    let obj = result.unwrap();
    assert_eq!(obj.name, Some("imgset1".to_string()));
    match &obj.object_type {
        SkinObjectType::ResolvedImageSet {
            images,
            ref_id,
            act,
            click,
        } => {
            assert_eq!(images.len(), 2);
            assert_eq!(images[0].src, Some("src_a".to_string()));
            assert_eq!(images[1].src, Some("src_b".to_string()));
            // value takes precedence over ref_id
            assert_eq!(*ref_id, 100);
            assert_eq!(*act, Some(10));
            assert_eq!(*click, 1);
        }
        _ => panic!("Expected ResolvedImageSet"),
    }
}

#[test]
fn test_load_value_number() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.value.push(json_skin::Value {
        id: Some("num1".to_string()),
        src: Some("src1".to_string()),
        digit: 5,
        padding: 1,
        zeropadding: 1,
        space: 2,
        ref_id: 10,
        value: Some(200),
        align: 1,
        divx: 10,
        divy: 1,
        ..Default::default()
    });
    let dst = make_dst("num1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    let obj = result.unwrap();
    match &obj.object_type {
        SkinObjectType::Number {
            digit,
            padding,
            ref_id,
            value,
            align,
            ..
        } => {
            assert_eq!(*digit, 5);
            assert_eq!(*padding, 1);
            assert_eq!(*ref_id, 10);
            assert_eq!(*value, Some(200));
            assert_eq!(*align, 1);
        }
        _ => panic!("Expected Number"),
    }
}

#[test]
fn test_load_float_value() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.floatvalue.push(json_skin::FloatValue {
        id: Some("fv1".to_string()),
        iketa: 3,
        fketa: 2,
        gain: 1.5,
        is_signvisible: true,
        ..Default::default()
    });
    let dst = make_dst("fv1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::Float {
            iketa,
            fketa,
            gain,
            is_signvisible,
            ..
        } => {
            assert_eq!(*iketa, 3);
            assert_eq!(*fketa, 2);
            assert!((gain - 1.5).abs() < f32::EPSILON);
            assert!(*is_signvisible);
        }
        _ => panic!("Expected Float"),
    }
}

#[test]
fn test_load_text() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.font.push(json_skin::Font {
        id: Some("font1".to_string()),
        path: Some("VL-Gothic-Regular.ttf".to_string()),
        ..Default::default()
    });
    sk.text.push(json_skin::Text {
        id: Some("txt1".to_string()),
        font: Some("font1".to_string()),
        size: 24,
        align: 2,
        ref_id: 5,
        constant_text: Some("Hello".to_string()),
        wrapping: true,
        ..Default::default()
    });
    let dst = make_dst("txt1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::Text {
            font,
            size,
            align,
            constant_text,
            wrapping,
            ..
        } => {
            assert_eq!(*font, Some("/fake/VL-Gothic-Regular.ttf".to_string()));
            assert_eq!(*size, 24);
            assert_eq!(*align, 2);
            assert_eq!(*constant_text, Some("Hello".to_string()));
            assert!(*wrapping);
        }
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_load_slider() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.slider.push(json_skin::Slider {
        id: Some("sl1".to_string()),
        angle: 1,
        range: 100,
        slider_type: 2,
        changeable: false,
        value: Some(50),
        ..Default::default()
    });
    let dst = make_dst("sl1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::Slider {
            angle,
            range,
            slider_type,
            changeable,
            value,
            ..
        } => {
            assert_eq!(*angle, 1);
            assert_eq!(*range, 100);
            assert_eq!(*slider_type, 2);
            assert!(!changeable);
            assert_eq!(*value, Some(50));
        }
        _ => panic!("Expected Slider"),
    }
}

#[test]
fn test_load_graph_positive_type() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.graph.push(json_skin::Graph {
        id: Some("gr1".to_string()),
        graph_type: 0,
        angle: 1,
        value: Some(300),
        ..Default::default()
    });
    let dst = make_dst("gr1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::Graph {
            graph_type,
            angle,
            value,
            ..
        } => {
            assert_eq!(*graph_type, 0);
            assert_eq!(*angle, 1);
            assert_eq!(*value, Some(300));
        }
        _ => panic!("Expected Graph"),
    }
}

#[test]
fn test_load_graph_negative_type_distribution() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.graph.push(json_skin::Graph {
        id: Some("dgr1".to_string()),
        graph_type: -1,
        ..Default::default()
    });
    let dst = make_dst("dgr1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::DistributionGraph { graph_type, .. } => {
            assert_eq!(*graph_type, -1);
        }
        _ => panic!("Expected DistributionGraph"),
    }
}

#[test]
fn test_load_gauge_graph() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.gaugegraph.push(json_skin::GaugeGraph {
        id: Some("gg1".to_string()),
        color: Some(vec!["ff0000".to_string(); 24]),
        ..Default::default()
    });
    let dst = make_dst("gg1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::GaugeGraph { color, .. } => {
            assert!(color.is_some());
            assert_eq!(color.as_ref().unwrap().len(), 24);
        }
        _ => panic!("Expected GaugeGraph"),
    }
}

#[test]
fn test_load_judge_graph() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.judgegraph.push(json_skin::JudgeGraph {
        id: Some("jg1".to_string()),
        graph_type: 1,
        delay: 500,
        ..Default::default()
    });
    let dst = make_dst("jg1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::JudgeGraph {
            graph_type, delay, ..
        } => {
            assert_eq!(*graph_type, 1);
            assert_eq!(*delay, 500);
        }
        _ => panic!("Expected JudgeGraph"),
    }
}

#[test]
fn test_load_bpm_graph() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.bpmgraph.push(json_skin::BPMGraph {
        id: Some("bg1".to_string()),
        delay: 100,
        line_width: 3,
        ..Default::default()
    });
    let dst = make_dst("bg1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::BpmGraph {
            delay, line_width, ..
        } => {
            assert_eq!(*delay, 100);
            assert_eq!(*line_width, 3);
        }
        _ => panic!("Expected BpmGraph"),
    }
}

#[test]
fn test_load_hit_error_visualizer() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.hiterrorvisualizer.push(json_skin::HitErrorVisualizer {
        id: Some("hev1".to_string()),
        ..Default::default()
    });
    let dst = make_dst("hev1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    matches!(
        result.unwrap().object_type,
        SkinObjectType::HitErrorVisualizer { .. }
    );
}

#[test]
fn test_load_timing_visualizer() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.timingvisualizer.push(json_skin::TimingVisualizer {
        id: Some("tv1".to_string()),
        ..Default::default()
    });
    let dst = make_dst("tv1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    matches!(
        result.unwrap().object_type,
        SkinObjectType::TimingVisualizer { .. }
    );
}

#[test]
fn test_load_timing_distribution_graph() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.timingdistributiongraph
        .push(json_skin::TimingDistributionGraph {
            id: Some("td1".to_string()),
            ..Default::default()
        });
    let dst = make_dst("td1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    matches!(
        result.unwrap().object_type,
        SkinObjectType::TimingDistributionGraph { .. }
    );
}

#[test]
fn test_load_gauge() {
    let mut loader = make_loader();
    let skin = make_skin();
    let mut sk = make_sk();
    sk.gauge = Some(json_skin::Gauge {
        id: Some("gauge1".to_string()),
        nodes: vec!["n1".to_string(), "n2".to_string()],
        parts: 50,
        gauge_type: 0,
        range: 3,
        cycle: 33,
        starttime: 0,
        endtime: 500,
    });
    let dst = make_dst("gauge1");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_some());
    match &result.unwrap().object_type {
        SkinObjectType::Gauge {
            nodes,
            parts,
            gauge_type,
            range,
            cycle,
            starttime,
            endtime,
        } => {
            assert_eq!(nodes.len(), 2);
            assert_eq!(*parts, 50);
            assert_eq!(*gauge_type, 0);
            assert_eq!(*range, 3);
            assert_eq!(*cycle, 33);
            assert_eq!(*starttime, 0);
            assert_eq!(*endtime, 500);
        }
        _ => panic!("Expected Gauge"),
    }
}

#[test]
fn test_load_no_match_returns_none() {
    let mut loader = make_loader();
    let skin = make_skin();
    let sk = make_sk();
    let dst = make_dst("nonexistent");
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_none());
}

#[test]
fn test_load_no_id_returns_none() {
    let mut loader = make_loader();
    let skin = make_skin();
    let sk = make_sk();
    let dst = json_skin::Destination::default(); // id is None
    let p = std::path::Path::new("/fake/skin.json");

    let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
    assert!(result.is_none());
}
