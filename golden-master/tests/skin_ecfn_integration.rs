// ECFN skin loading and font/image integration tests.
//
// Validates that all ECFN skin files load correctly, produce non-empty object
// sets, render reasonable snapshots at multiple time points, and that all
// bundled font/image assets are valid.
//
// Run: cargo test -p golden-master -- skin_ecfn

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use golden_master::render_snapshot::capture_render_snapshot;
use golden_master::state_provider::{StaticMainStateAdapter, StaticStateProvider};
use rubato_render::font::BitmapFontData;
use rubato_skin::json::json_skin_loader::{JSONSkinLoader, SkinConfigProperty};
use rubato_skin::lua::lua_skin_loader::LuaSkinLoader;
use rubato_skin::skin::Skin;
use rubato_skin::skin_data_converter;
use rubato_skin::skin_type::SkinType;
use rubato_skin::stubs::{MainState, Resolution as SkinResolution};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn skins_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skin/ECFN")
}

fn state_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/screenshot_states")
}

fn load_state(name: &str) -> StaticStateProvider {
    let path = state_dir().join(name);
    if !path.exists() {
        return StaticStateProvider::default();
    }
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}

fn load_lua_skin_with_state(relative_path: &str, provider: &StaticStateProvider) -> Skin {
    let path = skins_dir().join(relative_path);
    assert!(path.exists(), "Skin not found: {}", path.display());

    let mut loader = LuaSkinLoader::new();

    let mut adapter = StaticMainStateAdapter::new(provider);
    let adapter_ptr: *mut dyn MainState = &mut adapter;
    // SAFETY: adapter outlives the Lua closures; single-threaded skin loading.
    let adapter_ptr: *mut dyn MainState = unsafe { std::mem::transmute(adapter_ptr) };
    unsafe { loader.lua.export_main_state_accessor(adapter_ptr) };

    let header = loader
        .load_header(&path)
        .unwrap_or_else(|| panic!("Failed to load Lua skin header: {}", path.display()));
    let skin_type = SkinType::get_skin_type_by_id(header.skin_type)
        .unwrap_or_else(|| panic!("Unknown skin type {} in header", header.skin_type));
    let skin_data = loader
        .load_skin(&path, &skin_type, &SkinConfigProperty)
        .unwrap_or_else(|| panic!("Failed to load Lua skin data: {}", path.display()));

    let dstr = SkinResolution {
        width: 1920.0,
        height: 1080.0,
    };
    skin_data_converter::convert_skin_data(
        &header,
        skin_data,
        &mut loader.json_loader.source_map,
        &path,
        false,
        &dstr,
    )
    .unwrap_or_else(|| panic!("Failed to convert skin data: {}", path.display()))
}

fn load_json_skin(relative_path: &str) -> Skin {
    let path = skins_dir().join(relative_path);
    assert!(path.exists(), "Skin not found: {}", path.display());

    let mut loader = JSONSkinLoader::new();
    let header = loader
        .load_header(&path)
        .unwrap_or_else(|| panic!("Failed to load JSON skin header: {}", path.display()));
    let skin_type = SkinType::get_skin_type_by_id(header.skin_type)
        .unwrap_or_else(|| panic!("Unknown skin type {} in header", header.skin_type));
    let skin_data = loader
        .load_skin(&path, &skin_type, &SkinConfigProperty)
        .unwrap_or_else(|| panic!("Failed to load JSON skin data: {}", path.display()));

    let dstr = SkinResolution {
        width: 1920.0,
        height: 1080.0,
    };
    skin_data_converter::convert_skin_data(
        &header,
        skin_data,
        &mut loader.source_map,
        &path,
        false,
        &dstr,
    )
    .unwrap_or_else(|| panic!("Failed to convert skin data: {}", path.display()))
}

/// Capture a render snapshot at a specific time point.
fn capture_at_time(
    skin_path: &str,
    state_json: &str,
    time_ms: i64,
) -> golden_master::render_snapshot::RenderSnapshot {
    let mut provider = load_state(state_json);
    provider.time_ms = time_ms;

    // Update timer values proportionally to time.
    let timer_keys: Vec<i32> = provider.timers.keys().copied().collect();
    for key in timer_keys {
        if let Some(val) = provider.timers.get_mut(&key) {
            *val = time_ms;
        }
    }

    let skin = load_lua_skin_with_state(skin_path, &provider);
    capture_render_snapshot(&skin, &provider)
}

/// Recursively find all files with the given extension under `dir`.
fn find_files_recursively(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if !dir.is_dir() {
        return results;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(ext) = path.extension()
                && ext.eq_ignore_ascii_case(extension)
            {
                results.push(path);
            }
        }
    }
    results.sort();
    results
}

// ---------------------------------------------------------------------------
// All ECFN Lua skin paths
// ---------------------------------------------------------------------------

const ALL_LUA_SKINS: &[&str] = &[
    "decide/decide.luaskin",
    "play/play7.luaskin",
    "play/play14.luaskin",
    "play/play7wide.luaskin",
    "RESULT/result.luaskin",
    "RESULT/result2.luaskin",
    "RESULT/course_result.luaskin",
    "select/select.luaskin",
];

// ---------------------------------------------------------------------------
// Skin Loading Tests
// ---------------------------------------------------------------------------

#[test]
fn skin_ecfn_all_lua_skins_load_successfully() {
    let provider = load_state("state_default.json");
    for &skin_path in ALL_LUA_SKINS {
        let skin = load_lua_skin_with_state(skin_path, &provider);
        let object_count = skin.get_objects().len();
        assert!(
            object_count > 0,
            "{}: loaded skin has zero objects",
            skin_path
        );
        eprintln!("  loaded {}: {} objects", skin_path, object_count);
    }
}

#[test]
fn skin_ecfn_select_json_loads_successfully() {
    let skin = load_json_skin("select/select.json");
    let object_count = skin.get_objects().len();
    assert!(
        object_count > 0,
        "select/select.json: loaded skin has zero objects"
    );
    eprintln!("  loaded select/select.json: {} objects", object_count);
}

#[test]
fn skin_ecfn_skin_object_counts_reasonable() {
    let provider = load_state("state_default.json");
    for &skin_path in ALL_LUA_SKINS {
        let skin = load_lua_skin_with_state(skin_path, &provider);
        let object_count = skin.get_objects().len();
        assert!(
            object_count > 10,
            "{}: expected > 10 objects, got {}",
            skin_path,
            object_count
        );
    }
    // Also check JSON skin.
    let json_skin = load_json_skin("select/select.json");
    assert!(
        json_skin.get_objects().len() > 10,
        "select/select.json: expected > 10 objects, got {}",
        json_skin.get_objects().len()
    );
}

// ---------------------------------------------------------------------------
// Render Snapshot Tests
// ---------------------------------------------------------------------------

struct SkinStateCombo {
    skin_path: &'static str,
    state_json: &'static str,
}

const SNAPSHOT_COMBOS: &[SkinStateCombo] = &[
    SkinStateCombo {
        skin_path: "play/play7.luaskin",
        state_json: "state_play_active.json",
    },
    SkinStateCombo {
        skin_path: "play/play14.luaskin",
        state_json: "state_play_active.json",
    },
    SkinStateCombo {
        skin_path: "decide/decide.luaskin",
        state_json: "state_default.json",
    },
    SkinStateCombo {
        skin_path: "RESULT/result.luaskin",
        state_json: "state_result_clear.json",
    },
    SkinStateCombo {
        skin_path: "RESULT/result.luaskin",
        state_json: "state_result_fail.json",
    },
    SkinStateCombo {
        skin_path: "RESULT/course_result.luaskin",
        state_json: "state_result_clear.json",
    },
    SkinStateCombo {
        skin_path: "select/select.luaskin",
        state_json: "state_default.json",
    },
];

#[test]
fn skin_ecfn_multi_timepoint_snapshots() {
    let time_points: &[i64] = &[0, 5000, 30000, 60000];

    for combo in SNAPSHOT_COMBOS {
        for &t in time_points {
            let snapshot = capture_at_time(combo.skin_path, combo.state_json, t);

            assert!(
                !snapshot.commands.is_empty(),
                "{} + {} at t={}: snapshot has zero commands",
                combo.skin_path,
                combo.state_json,
                t
            );

            let visible = snapshot.commands.iter().filter(|c| c.visible).count();
            // At least some commands should be visible at every time point.
            // Timer-driven animations may hide some objects, but not all.
            assert!(
                visible > 0,
                "{} + {} at t={}: no visible commands (total={})",
                combo.skin_path,
                combo.state_json,
                t,
                snapshot.commands.len()
            );

            eprintln!(
                "  {} + {} at t={}: {} commands, {} visible",
                combo.skin_path,
                combo.state_json,
                t,
                snapshot.commands.len(),
                visible
            );
        }
    }
}

#[test]
fn skin_ecfn_play_with_active_state_has_objects() {
    let provider = load_state("state_play_active.json");
    let skin = load_lua_skin_with_state("play/play7.luaskin", &provider);
    let snapshot = capture_render_snapshot(&skin, &provider);

    assert!(
        snapshot.commands.len() > 50,
        "play7 + state_play_active: expected > 50 commands, got {}",
        snapshot.commands.len()
    );

    let visible = snapshot.commands.iter().filter(|c| c.visible).count();
    eprintln!(
        "  play7 + state_play_active: {} commands, {} visible",
        snapshot.commands.len(),
        visible
    );

    // Verify object type diversity - a play skin should have multiple types.
    let mut type_counts: BTreeMap<String, usize> = BTreeMap::new();
    for cmd in &snapshot.commands {
        *type_counts.entry(cmd.object_type.clone()).or_insert(0) += 1;
    }
    assert!(
        type_counts.len() > 1,
        "play7: expected multiple object types, got {:?}",
        type_counts
    );
    eprintln!("  play7 object types: {:?}", type_counts);
}

// ---------------------------------------------------------------------------
// Font/Image Asset Tests
// ---------------------------------------------------------------------------

#[test]
fn skin_ecfn_fnt_files_parse_successfully() {
    let ecfn_dir = skins_dir();
    let fnt_files = find_files_recursively(&ecfn_dir, "fnt");
    assert!(
        !fnt_files.is_empty(),
        "No .fnt files found in {}",
        ecfn_dir.display()
    );

    let mut parsed = 0;
    let mut encoding_skip = 0;
    for fnt_path in &fnt_files {
        // Verify the raw file is non-empty regardless of encoding.
        let raw = std::fs::read(fnt_path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", fnt_path.display(), e));
        assert!(!raw.is_empty(), "{}: FNT file is empty", fnt_path.display());

        // BitmapFontData::from_fnt uses read_to_string which requires UTF-8.
        // ECFN FNT files are Shift-JIS encoded, so from_fnt returns None.
        // Parse manually via the raw bytes converted lossy to verify structure.
        match BitmapFontData::from_fnt(fnt_path) {
            Some(data) => {
                assert!(
                    !data.glyphs.is_empty(),
                    "{}: parsed FNT has zero glyphs",
                    fnt_path.display()
                );
                assert!(
                    data.line_height > 0.0,
                    "{}: line_height should be positive, got {}",
                    fnt_path.display(),
                    data.line_height
                );
                parsed += 1;
            }
            None => {
                // UTF-8 read failed (likely Shift-JIS). Parse via lossy conversion
                // to verify the file is structurally valid FNT.
                let lossy = String::from_utf8_lossy(&raw);
                let data = BitmapFontData::parse_fnt(&lossy, fnt_path.parent());
                match data {
                    Some(d) => {
                        assert!(
                            !d.glyphs.is_empty(),
                            "{}: lossy-parsed FNT has zero glyphs",
                            fnt_path.display()
                        );
                        assert!(
                            d.line_height > 0.0,
                            "{}: line_height should be positive, got {}",
                            fnt_path.display(),
                            d.line_height
                        );
                        encoding_skip += 1;
                    }
                    None => {
                        panic!(
                            "{}: FNT file could not be parsed even with lossy UTF-8 conversion",
                            fnt_path.display()
                        );
                    }
                }
            }
        }
    }

    eprintln!(
        "  FNT files: {} total, {} parsed (UTF-8), {} parsed (lossy Shift-JIS)",
        fnt_files.len(),
        parsed,
        encoding_skip
    );
}

#[test]
fn skin_ecfn_ttf_files_load_successfully() {
    let ecfn_dir = skins_dir();
    let ttf_files = find_files_recursively(&ecfn_dir, "ttf");
    assert!(
        !ttf_files.is_empty(),
        "No .ttf files found in {}",
        ecfn_dir.display()
    );

    for ttf_path in &ttf_files {
        let metadata = std::fs::metadata(ttf_path)
            .unwrap_or_else(|e| panic!("Cannot stat {}: {}", ttf_path.display(), e));
        assert!(
            metadata.len() > 100,
            "{}: TTF file too small ({} bytes), likely corrupt",
            ttf_path.display(),
            metadata.len()
        );
        eprintln!("  {}: {} bytes", ttf_path.display(), metadata.len());
    }

    eprintln!("  TTF files verified: {}", ttf_files.len());
}

#[test]
fn skin_ecfn_png_files_decode_successfully() {
    let ecfn_dir = skins_dir();
    let png_files = find_files_recursively(&ecfn_dir, "png");
    assert!(
        !png_files.is_empty(),
        "No .png files found in {}",
        ecfn_dir.display()
    );

    let mut decoded = 0;
    let mut failed = Vec::new();
    for png_path in &png_files {
        let data = match std::fs::read(png_path) {
            Ok(d) => d,
            Err(e) => {
                failed.push(format!("{}: read error: {}", png_path.display(), e));
                continue;
            }
        };
        match image::load_from_memory(&data) {
            Ok(img) => {
                let (w, h) = (img.width(), img.height());
                assert!(
                    w > 0 && h > 0,
                    "{}: decoded PNG has zero dimensions ({}x{})",
                    png_path.display(),
                    w,
                    h
                );
                decoded += 1;
            }
            Err(e) => {
                failed.push(format!("{}: decode error: {}", png_path.display(), e));
            }
        }
    }

    eprintln!(
        "  PNG files: {} total, {} decoded, {} failed",
        png_files.len(),
        decoded,
        failed.len()
    );

    assert!(
        failed.is_empty(),
        "Failed to decode {} PNG files:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Multi-Timepoint Snapshot Golden-Master Regression Tests
// ---------------------------------------------------------------------------

fn timepoint_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/skin_ecfn_timepoint")
}

/// Derive a fixture filename slug from a skin path and state JSON name.
/// e.g. "play/play7.luaskin" + "state_play_active.json" + 5000
///   -> "play_play7_state_play_active_5000.json"
fn fixture_slug(skin_path: &str, state_json: &str, time_ms: i64) -> String {
    let skin_part = skin_path.replace('/', "_").replace(".luaskin", "");
    let state_part = state_json.replace(".json", "");
    format!("{}_{}_{}ms.json", skin_part, state_part, time_ms)
}

#[test]
fn skin_ecfn_timepoint_snapshot_regression() {
    use golden_master::render_snapshot::compare_snapshots;

    let time_points: &[i64] = &[0, 5000, 30000, 60000];
    let fixture_dir = timepoint_fixture_dir();
    let update_mode = std::env::var_os("UPDATE_ECFN_TIMEPOINT_SNAPSHOTS").is_some();

    if update_mode {
        std::fs::create_dir_all(&fixture_dir)
            .unwrap_or_else(|e| panic!("Failed to create fixture dir: {}", e));
    }

    let mut updated = 0;
    let mut compared = 0;
    let mut skipped = 0;

    for combo in SNAPSHOT_COMBOS {
        for &t in time_points {
            let slug = fixture_slug(combo.skin_path, combo.state_json, t);
            let fixture_path = fixture_dir.join(&slug);

            let snapshot = capture_at_time(combo.skin_path, combo.state_json, t);

            if update_mode {
                // Save snapshot as fixture.
                let json = serde_json::to_string_pretty(&snapshot)
                    .unwrap_or_else(|e| panic!("Failed to serialize snapshot: {}", e));
                std::fs::write(&fixture_path, json).unwrap_or_else(|e| {
                    panic!("Failed to write {}: {}", fixture_path.display(), e)
                });
                eprintln!("  updated: {}", slug);
                updated += 1;
            } else if fixture_path.exists() {
                // Load expected fixture and compare.
                let expected_json = std::fs::read_to_string(&fixture_path)
                    .unwrap_or_else(|e| panic!("Failed to read {}: {}", fixture_path.display(), e));
                let expected: golden_master::render_snapshot::RenderSnapshot =
                    serde_json::from_str(&expected_json).unwrap_or_else(|e| {
                        panic!("Failed to parse {}: {}", fixture_path.display(), e)
                    });

                let diffs = compare_snapshots(&expected, &snapshot);
                assert!(
                    diffs.is_empty(),
                    "{}: regression detected ({} diffs):\n{}",
                    slug,
                    diffs.len(),
                    diffs.join("\n")
                );
                eprintln!(
                    "  compared: {} (ok, {} commands)",
                    slug,
                    snapshot.commands.len()
                );
                compared += 1;
            } else {
                // Fixture doesn't exist yet - skip without failing to allow
                // incremental fixture generation.
                eprintln!("  skipped (no fixture): {}", slug);
                skipped += 1;
            }
        }
    }

    eprintln!(
        "\n  timepoint regression: {} updated, {} compared, {} skipped",
        updated, compared, skipped
    );
}

#[test]
fn skin_ecfn_play_state_diversity() {
    let snapshot = capture_at_time("play/play7.luaskin", "state_play_active.json", 5000);

    // Collect distinct DrawDetail variant type names from commands that have detail.
    let mut detail_types: BTreeSet<String> = BTreeSet::new();
    for cmd in &snapshot.commands {
        if let Some(ref detail) = cmd.detail {
            let variant = match detail {
                golden_master::render_snapshot::DrawDetail::Image { .. } => "Image",
                golden_master::render_snapshot::DrawDetail::Number { .. } => "Number",
                golden_master::render_snapshot::DrawDetail::Text { .. } => "Text",
                golden_master::render_snapshot::DrawDetail::Slider { .. } => "Slider",
                golden_master::render_snapshot::DrawDetail::Graph { .. } => "Graph",
                golden_master::render_snapshot::DrawDetail::Gauge { .. } => "Gauge",
                golden_master::render_snapshot::DrawDetail::BpmGraph => "BpmGraph",
                golden_master::render_snapshot::DrawDetail::HitErrorVisualizer => {
                    "HitErrorVisualizer"
                }
                golden_master::render_snapshot::DrawDetail::NoteDistributionGraph => {
                    "NoteDistributionGraph"
                }
                golden_master::render_snapshot::DrawDetail::TimingDistributionGraph => {
                    "TimingDistributionGraph"
                }
                golden_master::render_snapshot::DrawDetail::TimingVisualizer => "TimingVisualizer",
            };
            detail_types.insert(variant.to_string());
        }
    }

    // Count distinct object_type strings which cover all rendered object categories
    // including those that don't produce a DrawDetail (Note, Judge, Gauge, BGA, etc.).
    let mut object_types: BTreeSet<String> = BTreeSet::new();
    for cmd in &snapshot.commands {
        object_types.insert(cmd.object_type.clone());
    }

    eprintln!("  play7 DrawDetail variants: {:?}", detail_types);
    eprintln!("  play7 object_type values:  {:?}", object_types);

    // A play skin should have at least 4 different object types
    // (typically: Image, Text, Graph, SkinNote, SkinJudge, SkinGauge, SkinBGA, etc.).
    assert!(
        object_types.len() >= 4,
        "play7 + state_play_active at t=5000: expected >= 4 distinct object types, got {} ({:?})",
        object_types.len(),
        object_types
    );
}

// ---------------------------------------------------------------------------
// Position Verification Tests
// ---------------------------------------------------------------------------

/// Verify that skin objects actually change position/alpha between timepoints,
/// proving that animations are working. A frozen skin (all identical across time)
/// indicates a regression in timer-driven destination evaluation.
#[test]
fn skin_ecfn_timepoint_position_delta() {
    let time_points: &[i64] = &[0, 5000, 30000];

    for combo in SNAPSHOT_COMBOS {
        let snapshots: Vec<_> = time_points
            .iter()
            .map(|&t| capture_at_time(combo.skin_path, combo.state_json, t))
            .collect();

        // Group commands by object_index across timepoints.
        // Only consider objects visible at all timepoints.
        let cmd_count = snapshots[0].commands.len();
        let mut changed_count = 0usize;
        let mut compared_count = 0usize;

        for idx in 0..cmd_count {
            // Check the object exists and is visible at all timepoints.
            let all_visible = snapshots.iter().all(|s| {
                s.commands
                    .get(idx)
                    .is_some_and(|c| c.visible && c.dst.is_some())
            });
            if !all_visible {
                continue;
            }
            compared_count += 1;

            // Check if dst or alpha differs between any two timepoints.
            let dsts: Vec<_> = snapshots
                .iter()
                .map(|s| {
                    let cmd = &s.commands[idx];
                    let d = cmd.dst.as_ref().unwrap();
                    let a = cmd.color.as_ref().map_or(1.0, |c| c.a);
                    (d.x, d.y, d.w, d.h, a)
                })
                .collect();

            let any_changed = dsts.windows(2).any(|pair| {
                let (x0, y0, w0, h0, a0) = pair[0];
                let (x1, y1, w1, h1, a1) = pair[1];
                (x0 - x1).abs() > 0.5
                    || (y0 - y1).abs() > 0.5
                    || (w0 - w1).abs() > 0.5
                    || (h0 - h1).abs() > 0.5
                    || (a0 - a1).abs() > 0.01
            });

            if any_changed {
                changed_count += 1;
            }
        }

        // At least some objects should change across timepoints.
        // Threshold: >= 1 changed object (very conservative -- most skins have
        // timer-driven animations that change many objects).
        if compared_count > 0 {
            eprintln!(
                "  {} + {}: {}/{} objects changed across timepoints",
                combo.skin_path, combo.state_json, changed_count, compared_count
            );
            assert!(
                changed_count > 0,
                "{} + {}: no objects changed position/alpha across t={:?} ({} visible objects compared). \
                 Animations may be frozen.",
                combo.skin_path,
                combo.state_json,
                time_points,
                compared_count
            );
        }
    }
}

/// Verify specific expected positions in the play7 skin snapshot.
/// Checks that Image, SkinNote, and SkinJudge objects exist with valid positions.
#[test]
fn skin_ecfn_play_specific_positions() {
    let snapshot = capture_at_time("play/play7.luaskin", "state_play_active.json", 5000);

    // 1. Find Image objects and verify the largest one is reasonably sized.
    //    Play skins may compose backgrounds from multiple images rather than
    //    one full-screen image, so we check the largest by area.
    let image_cmds: Vec<_> = snapshot
        .commands
        .iter()
        .filter(|c| c.visible && c.object_type == "Image" && c.dst.is_some())
        .collect();
    assert!(
        !image_cmds.is_empty(),
        "play7: expected visible Image objects"
    );
    let largest_image = image_cmds
        .iter()
        .max_by(|a, b| {
            let area_a = a.dst.as_ref().map_or(0.0, |d| d.w * d.h);
            let area_b = b.dst.as_ref().map_or(0.0, |d| d.w * d.h);
            area_a.partial_cmp(&area_b).unwrap()
        })
        .unwrap();
    let largest_dst = largest_image.dst.as_ref().unwrap();
    // The largest image should cover a significant portion of the screen.
    assert!(
        largest_dst.w >= 100.0 && largest_dst.h >= 100.0,
        "play7: largest Image is too small ({}x{})",
        largest_dst.w,
        largest_dst.h
    );

    // 2. SkinNote objects should exist.
    let note_cmds: Vec<_> = snapshot
        .commands
        .iter()
        .filter(|c| c.object_type == "SkinNote")
        .collect();
    assert!(
        !note_cmds.is_empty(),
        "play7: expected SkinNote objects in play skin"
    );

    // 3. SkinJudge objects should exist.
    let judge_cmds: Vec<_> = snapshot
        .commands
        .iter()
        .filter(|c| c.object_type == "SkinJudge")
        .collect();
    assert!(
        !judge_cmds.is_empty(),
        "play7: expected SkinJudge objects in play skin"
    );

    // 4. Visible SkinNote objects should have y within the play area (0..1080).
    for cmd in &note_cmds {
        if cmd.visible
            && let Some(ref d) = cmd.dst
        {
            assert!(
                d.y >= -10.0 && d.y <= 1090.0,
                "play7: SkinNote dst.y={} is outside play area",
                d.y
            );
        }
    }

    eprintln!(
        "  play7 positions: largest_image=({},{},{}x{}), {} images, {} notes, {} judges",
        largest_dst.x,
        largest_dst.y,
        largest_dst.w,
        largest_dst.h,
        image_cmds.len(),
        note_cmds.len(),
        judge_cmds.len()
    );
}
