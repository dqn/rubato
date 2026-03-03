use criterion::{Criterion, black_box, criterion_group, criterion_main};

use beatoraja_skin::json::json_skin;

/// A realistic JSON skin string with multiple object types,
/// destinations, and nested animations. Exercises the full
/// serde deserialization path including custom defaults and
/// lenient field handling.
fn sample_skin_json() -> String {
    r#"{
        "type": 0,
        "name": "Bench Skin",
        "w": 1920,
        "h": 1080,
        "fadeout": 500,
        "input": 500,
        "scene": 3000,
        "close": 1500,
        "loadend": 1000,
        "playstart": 500,
        "source": [
            {"id": "bg", "path": "bg.png"},
            {"id": "notes", "path": "notes.png"},
            {"id": "judge", "path": "judge.png"},
            {"id": "gauge", "path": "gauge.png"},
            {"id": 5, "path": "extra.png"}
        ],
        "image": [
            {"id": "bg_img", "src": "bg", "x": 0, "y": 0, "w": 1920, "h": 1080, "divx": 1, "divy": 1},
            {"id": "note_img", "src": "notes", "x": 0, "y": 0, "w": 256, "h": 32, "divx": 8, "divy": 1},
            {"id": "judge_img", "src": "judge", "x": 0, "y": 0, "w": 512, "h": 256, "divx": 4, "divy": 4},
            {"id": 42, "src": 5, "x": 0, "y": 0, "w": 100, "h": 100, "divx": 2, "divy": 2}
        ],
        "font": [
            {"id": "main_font", "path": "font.ttf", "type": 0},
            {"id": 2, "path": "alt_font.fnt", "type": 1}
        ],
        "value": [
            {"id": "score", "src": "notes", "x": 0, "y": 32, "w": 200, "h": 24, "divx": 10, "divy": 1, "ref": 100, "digit": 7, "align": 2, "zeropadding": 1},
            {"id": "combo", "src": "notes", "x": 0, "y": 56, "w": 160, "h": 24, "divx": 10, "divy": 1, "ref": 101, "digit": 4, "align": 1}
        ],
        "text": [
            {"id": "title", "font": "main_font", "size": 24, "ref": 10, "align": 0, "wrapping": false, "overflow": 0, "outlineColor": "000000ff", "outlineWidth": 1.5}
        ],
        "gaugegraph": [
            {
                "id": "main_gauge",
                "assistClearBGColor": "440044",
                "assistAndEasyFailBGColor": "004444",
                "grooveFailBGColor": "004400",
                "grooveClearAndHardBGColor": "440000",
                "exHardBGColor": "444400",
                "hazardBGColor": "444444",
                "assistClearLineColor": "ff00ff",
                "assistAndEasyFailLineColor": "00ffff",
                "grooveFailLineColor": "00ff00",
                "grooveClearAndHardLineColor": "ff0000",
                "exHardLineColor": "ffff00",
                "hazardLineColor": "cccccc",
                "borderlineColor": "ff0000",
                "borderColor": "440000"
            }
        ],
        "hiterrorvisualizer": [
            {"id": "hev", "width": 301, "judgeWidthMillis": 150, "lineWidth": 1}
        ],
        "destination": [
            {"id": "bg_img", "timer": 1, "loop": -1, "dst": [
                {"time": 0, "x": 0, "y": 0, "w": 1920, "h": 1080, "a": 255, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "note_img", "timer": 40, "loop": 0, "dst": [
                {"time": 0, "x": 100, "y": 200, "w": 32, "h": 32, "a": 255, "r": 255, "g": 255, "b": 255},
                {"time": 500, "x": 100, "y": 800, "w": 32, "h": 32, "a": 255, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "judge_img", "blend": 2, "timer": 46, "dst": [
                {"time": 0, "x": 500, "y": 400, "w": 128, "h": 64, "a": 255, "r": 255, "g": 255, "b": 255},
                {"time": 200, "x": 500, "y": 400, "w": 128, "h": 64, "a": 0, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "score", "timer": 1, "dst": [
                {"time": 0, "x": 1600, "y": 50, "w": 200, "h": 24, "a": 255, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "combo", "timer": 40, "dst": [
                {"time": 0, "x": 900, "y": 500, "w": 160, "h": 24, "a": 255, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "title", "timer": 1, "dst": [
                {"time": 0, "x": 50, "y": 1050, "w": 600, "h": 24, "a": 255, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "main_gauge", "timer": 1, "dst": [
                {"time": 0, "x": 50, "y": 900, "w": 500, "h": 30, "a": 255, "r": 255, "g": 255, "b": 255}
            ]},
            {"id": "hev", "timer": 1, "dst": [
                {"time": 0, "x": 50, "y": 950, "w": 301, "h": 50, "a": 255, "r": 255, "g": 255, "b": 255}
            ]}
        ]
    }"#
    .to_string()
}

/// Benchmark deserializing a JSON skin string directly into `json_skin::Skin`.
fn bench_skin_json_parse(c: &mut Criterion) {
    let json = sample_skin_json();

    c.bench_function("skin_json_parse", |b| {
        b.iter(|| {
            let skin: json_skin::Skin = serde_json::from_str(black_box(&json)).unwrap();
            black_box(skin);
        });
    });
}

/// Benchmark deserializing from a pre-parsed `serde_json::Value` into `json_skin::Skin`.
/// This isolates the schema-level deserialization cost from JSON tokenization.
fn bench_skin_json_from_value(c: &mut Criterion) {
    let json = sample_skin_json();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    c.bench_function("skin_json_from_value", |b| {
        b.iter(|| {
            let skin: json_skin::Skin = serde_json::from_value(black_box(value.clone())).unwrap();
            black_box(skin);
        });
    });
}

/// Benchmark the Destination struct deserialization with multiple animation keyframes.
/// Destinations are the most numerous elements in real skins.
fn bench_destination_parse(c: &mut Criterion) {
    // Build a destination with 20 animation keyframes (realistic for complex animations)
    let mut dst_json = String::from(
        r#"{"id": "anim", "timer": 40, "loop": -1, "blend": 2, "filter": 1, "op": [1, 2, 3], "dst": ["#,
    );
    for i in 0..20 {
        if i > 0 {
            dst_json.push(',');
        }
        dst_json.push_str(&format!(
            r#"{{"time": {}, "x": {}, "y": {}, "w": 64, "h": 64, "a": {}, "r": 255, "g": 255, "b": 255, "angle": {}}}"#,
            i * 100,
            100 + i * 10,
            200 + i * 5,
            255 - i * 12,
            i * 18,
        ));
    }
    dst_json.push_str("]}");

    c.bench_function("destination_parse_20_keyframes", |b| {
        b.iter(|| {
            let dst: json_skin::Destination = serde_json::from_str(black_box(&dst_json)).unwrap();
            black_box(dst);
        });
    });
}

/// Benchmark FloatFormatter digit calculation (pure CPU, no I/O).
fn bench_float_formatter(c: &mut Criterion) {
    use beatoraja_skin::float_formatter::FloatFormatter;

    c.bench_function("float_formatter_calculate", |b| {
        let mut formatter = FloatFormatter::new(5, 2, true, 1);
        b.iter(|| {
            let digits = formatter.calculate_and_get_digits(black_box(12345.67));
            black_box(digits);
        });
    });
}

/// Benchmark the lenient i32 deserializer (used for Lua-coerced fields).
/// Tests the custom `deserialize_i32_lenient` via CustomEvent which uses it.
fn bench_lenient_i32_deserialize(c: &mut Criterion) {
    // CustomEvent uses deserialize_i32_lenient for its `id` field
    let from_number = r#"{"id": 42, "action": 1}"#;
    let from_string = r#"{"id": "42", "action": 1}"#;
    let from_float = r#"{"id": 42.0, "action": 1}"#;

    c.bench_function("lenient_i32_from_number", |b| {
        b.iter(|| {
            let evt: json_skin::CustomEvent = serde_json::from_str(black_box(from_number)).unwrap();
            black_box(evt);
        });
    });

    c.bench_function("lenient_i32_from_string", |b| {
        b.iter(|| {
            let evt: json_skin::CustomEvent = serde_json::from_str(black_box(from_string)).unwrap();
            black_box(evt);
        });
    });

    c.bench_function("lenient_i32_from_float", |b| {
        b.iter(|| {
            let evt: json_skin::CustomEvent = serde_json::from_str(black_box(from_float)).unwrap();
            black_box(evt);
        });
    });
}

criterion_group!(
    benches,
    bench_skin_json_parse,
    bench_skin_json_from_value,
    bench_destination_parse,
    bench_float_formatter,
    bench_lenient_i32_deserialize,
);
criterion_main!(benches);
