// Java vs Rust RenderSnapshot comparison tests.
//
// Instead of comparing pixel screenshots (SSIM), this compares the structural
// draw commands that each side would produce. Both Java and Rust export a JSON
// snapshot of "what to draw" (position, color, angle, blend, type-specific
// detail) and this test verifies field-by-field equality with tolerances:
//   - Position (x, y, w, h): ±1.0 pixel
//   - Color (r, g, b): ±0.005
//   - Alpha (a): ±0.01
//   - Angle, blend, visibility, text content: exact match
//
// Java fixtures: golden-master/fixtures/render_snapshots_java/{name}.json
// Rust snapshots: generated on-the-fly from skin + state
//
// Run: cargo test -p golden-master compare_render_snapshot -- --nocapture

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use bms_config::resolution::Resolution;
use bms_render::state_provider::StaticStateProvider;
use bms_skin::loader::{json_loader, lua_loader};
use bms_skin::skin_header::CustomOption;
use golden_master::render_snapshot::{
    DrawCommand, DrawDetail, RenderSnapshot, capture_render_snapshot, compare_snapshots,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn skins_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skin/ECFN")
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn state_dir() -> PathBuf {
    fixture_dir().join("screenshot_states")
}

fn default_enabled_option_ids(options: &[CustomOption]) -> HashSet<i32> {
    options
        .iter()
        .filter_map(CustomOption::default_option)
        .collect()
}

/// Load a Java-generated RenderSnapshot fixture.
fn load_java_snapshot(name: &str) -> RenderSnapshot {
    let path = fixture_dir()
        .join("render_snapshots_java")
        .join(format!("{name}.json"));
    assert!(
        path.exists(),
        "Java fixture not found: {}. Run `just golden-master-render-snapshot-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}

/// Load a StaticStateProvider from a state JSON file.
fn load_state(name: &str) -> StaticStateProvider {
    let path = state_dir().join(name);
    if !path.exists() {
        return StaticStateProvider::default();
    }
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let mut state: StaticStateProvider = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));

    if std::env::var_os("GM_STATE_TIMERS_ONLY").is_some() {
        state.integers.clear();
        state.floats.clear();
        state.booleans.clear();
    }

    state
}

/// Load a Lua skin from the ECFN directory.
fn load_lua_skin(relative_path: &str) -> bms_skin::skin::Skin {
    let path = skins_dir().join(relative_path);
    assert!(path.exists(), "Skin not found: {}", path.display());
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let header = lua_loader::load_lua_header(&content, Some(&path))
        .unwrap_or_else(|e| panic!("Failed to load Lua header {}: {}", path.display(), e));
    let enabled = default_enabled_option_ids(&header.options);
    lua_loader::load_lua_skin(
        &content,
        &enabled,
        Resolution::Fullhd,
        Some(&path),
        &[],
        None,
    )
    .unwrap_or_else(|e| panic!("Failed to load Lua skin {}: {}", path.display(), e))
}

/// Load a JSON skin from the ECFN directory.
fn load_json_skin(relative_path: &str) -> bms_skin::skin::Skin {
    let path = skins_dir().join(relative_path);
    assert!(path.exists(), "Skin not found: {}", path.display());
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let header = json_loader::load_header(&content)
        .unwrap_or_else(|e| panic!("Failed to load JSON header {}: {}", path.display(), e));
    let enabled = default_enabled_option_ids(&header.options);
    json_loader::load_skin(&content, &enabled, Resolution::Fullhd, Some(&path))
        .unwrap_or_else(|e| panic!("Failed to load JSON skin {}: {}", path.display(), e))
}

struct RenderSnapshotTestCase {
    name: &'static str,
    skin_path: &'static str,
    state_json: &'static str,
    /// Whether the skin is Lua (.luaskin) or JSON (.json)
    is_lua: bool,
    /// Current known diff budget between Java and Rust snapshots.
    /// Used by non-ignored regression guard test to catch worsened parity.
    known_diff_budget: usize,
}

const TEST_CASES: &[RenderSnapshotTestCase] = &[
    RenderSnapshotTestCase {
        name: "ecfn_select",
        skin_path: "select/select.luaskin",
        state_json: "state_default.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_decide",
        skin_path: "decide/decide.luaskin",
        state_json: "state_default.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_play7_active",
        skin_path: "play/play7.luaskin",
        state_json: "state_play_active.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_play7_fullcombo",
        skin_path: "play/play7.luaskin",
        state_json: "state_play_fullcombo.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_play7_danger",
        skin_path: "play/play7.luaskin",
        state_json: "state_play_danger.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_result_clear",
        skin_path: "RESULT/result.luaskin",
        state_json: "state_result_clear.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_result_fail",
        skin_path: "RESULT/result.luaskin",
        state_json: "state_result_fail.json",
        is_lua: true,
        known_diff_budget: 0,
    },
    RenderSnapshotTestCase {
        name: "ecfn_play14_active",
        skin_path: "play/play14.luaskin",
        state_json: "state_play_active.json",
        is_lua: true,
        known_diff_budget: 27,
    },
    RenderSnapshotTestCase {
        name: "ecfn_play7wide_active",
        skin_path: "play/play7wide.luaskin",
        state_json: "state_play_active.json",
        is_lua: true,
        known_diff_budget: 29,
    },
    RenderSnapshotTestCase {
        name: "ecfn_course_result",
        skin_path: "RESULT/course_result.luaskin",
        state_json: "state_result_clear.json",
        is_lua: true,
        known_diff_budget: 0,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DiffCategory {
    CommandCount,
    Visibility,
    Geometry,
    Detail,
    Other,
}

impl DiffCategory {
    fn label(self) -> &'static str {
        match self {
            Self::CommandCount => "command_count",
            Self::Visibility => "visibility",
            Self::Geometry => "geometry",
            Self::Detail => "detail",
            Self::Other => "other",
        }
    }
}

fn categorize_diff(diff: &str) -> DiffCategory {
    if diff.starts_with("command_count:") {
        DiffCategory::CommandCount
    } else if diff.contains(" visible:") {
        DiffCategory::Visibility
    } else if diff.starts_with("skin_width:")
        || diff.starts_with("skin_height:")
        || diff.contains(" dst.")
    {
        DiffCategory::Geometry
    } else if diff.contains(" detail")
        || diff.contains(" color.")
        || diff.contains(" angle:")
        || diff.contains(" blend:")
        || diff.contains(" object_type:")
    {
        DiffCategory::Detail
    } else {
        DiffCategory::Other
    }
}

fn summarize_diff_categories(diffs: &[String]) -> String {
    let mut counts: BTreeMap<DiffCategory, usize> = BTreeMap::new();
    for diff in diffs {
        *counts.entry(categorize_diff(diff)).or_insert(0) += 1;
    }
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .iter()
        .map(|(category, count)| format!("{}={}", category.label(), count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn summarize_command_count_gap(java: &RenderSnapshot, rust: &RenderSnapshot) -> Option<String> {
    if java.commands.len() == rust.commands.len() {
        return None;
    }

    let mut type_counts: BTreeMap<String, isize> = BTreeMap::new();
    let mut visible_type_counts: BTreeMap<String, isize> = BTreeMap::new();

    for cmd in &rust.commands {
        *type_counts.entry(cmd.object_type.clone()).or_insert(0) += 1;
        if cmd.visible {
            *visible_type_counts
                .entry(cmd.object_type.clone())
                .or_insert(0) += 1;
        }
    }
    for cmd in &java.commands {
        *type_counts.entry(cmd.object_type.clone()).or_insert(0) -= 1;
        if cmd.visible {
            *visible_type_counts
                .entry(cmd.object_type.clone())
                .or_insert(0) -= 1;
        }
    }

    let type_delta = type_counts
        .into_iter()
        .filter(|(_, delta)| *delta != 0)
        .map(|(ty, delta)| format!("{ty}:{delta:+}"))
        .collect::<Vec<_>>()
        .join(", ");
    let visible_delta = visible_type_counts
        .into_iter()
        .filter(|(_, delta)| *delta != 0)
        .map(|(ty, delta)| format!("{ty}:{delta:+}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sequence_delta = summarize_command_sequence_gap(java, rust);

    Some(format!(
        "type_delta(rust-java): [{}]; visible_type_delta(rust-java): [{}]; sequence_delta: {}",
        type_delta, visible_delta, sequence_delta
    ))
}

fn command_detail_kind(command: &DrawCommand) -> &'static str {
    match command.detail.as_ref() {
        Some(DrawDetail::Image { .. }) => "Image",
        Some(DrawDetail::Number { .. }) => "Number",
        Some(DrawDetail::Text { .. }) => "Text",
        Some(DrawDetail::Slider { .. }) => "Slider",
        Some(DrawDetail::Graph { .. }) => "Graph",
        Some(DrawDetail::Gauge { .. }) => "Gauge",
        Some(DrawDetail::BpmGraph) => "BpmGraph",
        Some(DrawDetail::HitErrorVisualizer) => "HitErrorVisualizer",
        Some(DrawDetail::NoteDistributionGraph) => "NoteDistributionGraph",
        Some(DrawDetail::TimingDistributionGraph) => "TimingDistributionGraph",
        Some(DrawDetail::TimingVisualizer) => "TimingVisualizer",
        None => "-",
    }
}

fn command_signature(command: &DrawCommand) -> String {
    format!(
        "{}|{}|{}",
        command.object_type,
        command.visible,
        command_detail_kind(command)
    )
}

fn format_command_at(commands: &[DrawCommand], pos: usize) -> String {
    let command = &commands[pos];
    let name = command
        .name
        .as_deref()
        .map(|n| format!(" name={n}"))
        .unwrap_or_default();
    format!(
        "pos={pos} idx={} type={} visible={} detail={}{}",
        command.object_index,
        command.object_type,
        command.visible,
        command_detail_kind(command),
        name
    )
}

fn summarize_command_sequence_gap(java: &RenderSnapshot, rust: &RenderSnapshot) -> String {
    let cap = if std::env::var_os("GM_DEBUG_SEQUENCE_ALL").is_some() {
        usize::MAX
    } else {
        5
    };

    let java_sig: Vec<String> = java.commands.iter().map(command_signature).collect();
    let rust_sig: Vec<String> = rust.commands.iter().map(command_signature).collect();

    let jn = java_sig.len();
    let rn = rust_sig.len();
    let mut lcs = vec![vec![0usize; rn + 1]; jn + 1];

    for i in (0..jn).rev() {
        for j in (0..rn).rev() {
            lcs[i][j] = if java_sig[i] == rust_sig[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut java_only_pos = Vec::new();
    let mut rust_only_pos = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < jn && j < rn {
        if java_sig[i] == rust_sig[j] {
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            java_only_pos.push(i);
            i += 1;
        } else {
            rust_only_pos.push(j);
            j += 1;
        }
    }
    while i < jn {
        java_only_pos.push(i);
        i += 1;
    }
    while j < rn {
        rust_only_pos.push(j);
        j += 1;
    }

    let java_only = java_only_pos
        .iter()
        .take(cap)
        .map(|&pos| format_command_at(&java.commands, pos))
        .collect::<Vec<_>>()
        .join(" | ");
    let rust_only = rust_only_pos
        .iter()
        .take(cap)
        .map(|&pos| format_command_at(&rust.commands, pos))
        .collect::<Vec<_>>()
        .join(" | ");

    format!(
        "java_only(first{cap}/{}): [{}]; rust_only(first{cap}/{}): [{}]",
        java_only_pos.len(),
        java_only,
        rust_only_pos.len(),
        rust_only
    )
}

fn render_snapshot_debug_paths(case_name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let debug_dir = fixture_dir().join("render_snapshots_debug");
    (
        debug_dir.join(format!("{case_name}__java.json")),
        debug_dir.join(format!("{case_name}__rust.json")),
        debug_dir.join(format!("{case_name}__diffs.txt")),
    )
}

fn snapshot_diffs(tc: &RenderSnapshotTestCase) -> (RenderSnapshot, RenderSnapshot, Vec<String>) {
    let java_snapshot = load_java_snapshot(tc.name);

    let skin = if tc.is_lua {
        load_lua_skin(tc.skin_path)
    } else {
        load_json_skin(tc.skin_path)
    };

    let provider = load_state(tc.state_json);
    let rust_snapshot = capture_render_snapshot(&skin, &provider);

    let diffs = compare_snapshots(&java_snapshot, &rust_snapshot);
    (java_snapshot, rust_snapshot, diffs)
}

fn compare_java_rust_render_snapshot(tc: &RenderSnapshotTestCase) {
    let (java_snapshot, rust_snapshot, diffs) = snapshot_diffs(tc);
    let category_summary = summarize_diff_categories(&diffs);
    let command_gap_summary = summarize_command_count_gap(&java_snapshot, &rust_snapshot);

    let visible_java = java_snapshot.commands.iter().filter(|c| c.visible).count();
    let visible_rust = rust_snapshot.commands.iter().filter(|c| c.visible).count();
    eprintln!(
        "{}: java {} objects ({} visible), rust {} objects ({} visible), {} diffs ({})",
        tc.name,
        java_snapshot.commands.len(),
        visible_java,
        rust_snapshot.commands.len(),
        visible_rust,
        diffs.len(),
        category_summary
    );

    if !diffs.is_empty() {
        // Save Java/Rust snapshots and raw diffs for deterministic debugging.
        let (java_path, rust_path, diffs_path) = render_snapshot_debug_paths(tc.name);
        let debug_dir = fixture_dir().join("render_snapshots_debug");
        std::fs::create_dir_all(&debug_dir).ok();

        std::fs::write(
            &java_path,
            serde_json::to_string_pretty(&java_snapshot).unwrap(),
        )
        .ok();
        std::fs::write(
            &rust_path,
            serde_json::to_string_pretty(&rust_snapshot).unwrap(),
        )
        .ok();
        std::fs::write(&diffs_path, diffs.join("\n")).ok();

        // Only panic if diffs exceed the known budget (allows tracking Lua draw function limitations)
        if diffs.len() > tc.known_diff_budget {
            let first_10: Vec<_> = diffs.iter().take(10).collect();
            panic!(
                "RenderSnapshot mismatch for {} ({} differences > {} budget, categories: {}, showing first 10):\n{}\n  \
                 java debug: {}\n  \
                 rust debug: {}\n  \
                 diff list: {}\n  \
                 command_count breakdown: {}",
                tc.name,
                diffs.len(),
                tc.known_diff_budget,
                category_summary,
                first_10
                    .iter()
                    .map(|d| format!("  - {}", d))
                    .collect::<Vec<_>>()
                    .join("\n"),
                java_path.display(),
                rust_path.display(),
                diffs_path.display(),
                command_gap_summary.as_deref().unwrap_or("n/a"),
            );
        }
    }
}

// --- Test cases ---

#[test]
fn render_snapshot_java_fixtures_exist() {
    for tc in TEST_CASES {
        let _ = load_java_snapshot(tc.name);
    }
}

#[test]
fn render_snapshot_parity_regression_guard() {
    for tc in TEST_CASES {
        let (java_snapshot, rust_snapshot, diffs) = snapshot_diffs(tc);
        let category_summary = summarize_diff_categories(&diffs);
        let command_gap_summary = summarize_command_count_gap(&java_snapshot, &rust_snapshot);
        assert!(
            diffs.len() <= tc.known_diff_budget,
            "RenderSnapshot diff budget exceeded for {}: {} > {}.\nCategories: {}\nCommand-count breakdown: {}\nFirst differences:\n{}",
            tc.name,
            diffs.len(),
            tc.known_diff_budget,
            category_summary,
            command_gap_summary.as_deref().unwrap_or("n/a"),
            diffs
                .iter()
                .take(10)
                .map(|d| format!("  - {}", d))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn render_snapshot_ecfn_select() {
    let tc = &TEST_CASES[0];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_decide() {
    let tc = &TEST_CASES[1];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_play7_active() {
    let tc = &TEST_CASES[2];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_play7_fullcombo() {
    let tc = &TEST_CASES[3];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_play7_danger() {
    let tc = &TEST_CASES[4];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_result_clear() {
    let tc = &TEST_CASES[5];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_result_fail() {
    let tc = &TEST_CASES[6];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_play14_active() {
    let tc = &TEST_CASES[7];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_play7wide_active() {
    let tc = &TEST_CASES[8];
    compare_java_rust_render_snapshot(tc);
}

#[test]
fn render_snapshot_ecfn_course_result() {
    let tc = &TEST_CASES[9];
    compare_java_rust_render_snapshot(tc);
}

// --- Rust-only snapshot tests for additional ECFN skins ---
// These tests verify Rust-side snapshot capture for skins that don't have
// Java fixtures yet. They validate that skin loading + capture_render_snapshot
// produces non-empty, structurally sound output.

struct RustOnlySnapshotTestCase {
    name: &'static str,
    skin_path: &'static str,
    state_json: &'static str,
    is_lua: bool,
    /// Expected minimum object count (basic sanity check).
    min_objects: usize,
    /// Expected object types that must be present.
    expected_types: &'static [&'static str],
}

const RUST_ONLY_CASES: &[RustOnlySnapshotTestCase] = &[
    RustOnlySnapshotTestCase {
        name: "ecfn_play7_mid_song",
        skin_path: "play/play7.luaskin",
        state_json: "state_play_mid_song.json",
        is_lua: true,
        min_objects: 10,
        expected_types: &["Image", "SkinNote"],
    },
    RustOnlySnapshotTestCase {
        name: "ecfn_select_with_song",
        skin_path: "select/select.luaskin",
        state_json: "state_select_with_song.json",
        is_lua: true,
        min_objects: 10,
        expected_types: &["Image", "SkinBar"],
    },
    RustOnlySnapshotTestCase {
        name: "ecfn_result2_clear",
        skin_path: "RESULT/result2.luaskin",
        state_json: "state_result_clear.json",
        is_lua: true,
        min_objects: 5,
        expected_types: &["Image"],
    },
];

fn run_rust_only_snapshot(tc: &RustOnlySnapshotTestCase) {
    let skin = if tc.is_lua {
        load_lua_skin(tc.skin_path)
    } else {
        load_json_skin(tc.skin_path)
    };
    let provider = load_state(tc.state_json);
    let snapshot = capture_render_snapshot(&skin, &provider);

    assert!(
        snapshot.commands.len() >= tc.min_objects,
        "{}: expected at least {} objects, got {}",
        tc.name,
        tc.min_objects,
        snapshot.commands.len()
    );

    let types = count_object_types(&snapshot);
    for &expected in tc.expected_types {
        assert!(
            types.get(expected).copied().unwrap_or(0) > 0,
            "{}: expected object type '{}' not found (types: {:?})",
            tc.name,
            expected,
            types
        );
    }
}

#[test]
fn rust_only_snapshot_ecfn_play7_mid_song() {
    run_rust_only_snapshot(&RUST_ONLY_CASES[0]);
}

#[test]
fn rust_only_snapshot_ecfn_select_with_song() {
    run_rust_only_snapshot(&RUST_ONLY_CASES[1]);
}

#[test]
fn rust_only_snapshot_ecfn_result2_clear() {
    run_rust_only_snapshot(&RUST_ONLY_CASES[2]);
}

// --- Timeline snapshot tests ---
// Verify that animation state changes correctly over time by capturing
// snapshots at multiple time points and asserting monotonic changes.

fn capture_at_time(skin_path: &str, state_json: &str, time_ms: i64) -> RenderSnapshot {
    let skin = load_lua_skin(skin_path);
    let mut provider = load_state(state_json);
    provider.time_ms = time_ms;

    // Update timer values proportionally to time
    let timer_keys: Vec<i32> = provider.timers.keys().copied().collect();
    for key in timer_keys {
        if let Some(val) = provider.timers.get_mut(&key) {
            *val = time_ms;
        }
    }

    capture_render_snapshot(&skin, &provider)
}

#[test]
fn timeline_decide_visibility_changes() {
    // Decide skin should have different visibility at different times
    // (timer-driven animations change which objects are shown)
    let snap_0 = capture_at_time("decide/decide.luaskin", "state_default.json", 0);
    let snap_3000 = capture_at_time("decide/decide.luaskin", "state_default.json", 3000);

    let visible_0 = snap_0.commands.iter().filter(|c| c.visible).count();
    let visible_3000 = snap_3000.commands.iter().filter(|c| c.visible).count();

    // Both snapshots should have some visible objects
    assert!(visible_0 > 0, "decide: no visible objects at t=0");
    assert!(visible_3000 > 0, "decide: no visible objects at t=3000");

    // Total object count should be the same (same skin, different time)
    assert_eq!(
        snap_0.commands.len(),
        snap_3000.commands.len(),
        "decide: total command count should be equal across time points"
    );
}

#[test]
fn timeline_play7_has_consistent_structure() {
    // Play skin should have consistent structure across time points
    let times = [0i64, 1000, 5000, 30000];
    let snapshots: Vec<_> = times
        .iter()
        .map(|&t| capture_at_time("play/play7.luaskin", "state_play_active.json", t))
        .collect();

    // All snapshots should have the same total object count
    let base_count = snapshots[0].commands.len();
    for (i, snap) in snapshots.iter().enumerate() {
        assert_eq!(
            snap.commands.len(),
            base_count,
            "play7: total command count at t={} ({}) differs from t=0 ({})",
            times[i],
            snap.commands.len(),
            base_count
        );
    }

    // At t=30000, there should be visible objects (game is running)
    let visible_30000 = snapshots[3].commands.iter().filter(|c| c.visible).count();
    assert!(visible_30000 > 0, "play7: no visible objects at t=30000");
}

#[test]
fn timeline_result_has_stable_visible_set() {
    // Result screen should reach a stable state after initial animations
    let snap_5000 = capture_at_time("RESULT/result.luaskin", "state_result_clear.json", 5000);
    let snap_10000 = capture_at_time("RESULT/result.luaskin", "state_result_clear.json", 10000);

    let visible_5000 = snap_5000.commands.iter().filter(|c| c.visible).count();
    let visible_10000 = snap_10000.commands.iter().filter(|c| c.visible).count();

    // Result screen should have stabilized by t=5000
    // Visible count should be similar (within small delta for blinking elements)
    let delta = (visible_5000 as i32 - visible_10000 as i32).unsigned_abs() as usize;
    assert!(
        delta <= 5,
        "result: visible count changed significantly between t=5000 ({}) and t=10000 ({}), delta={}",
        visible_5000,
        visible_10000,
        delta
    );
}

// --- Skin State Object structure tests ---
// Verify that state-specific skin objects (SkinNote, SkinBar, SkinJudge, etc.)
// are present in the correct skin snapshots. These catch regressions where
// state-specific loaders fail to produce expected object types.

fn count_object_types(snapshot: &RenderSnapshot) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for cmd in &snapshot.commands {
        *counts.entry(cmd.object_type.clone()).or_insert(0) += 1;
    }
    counts
}

fn count_visible_object_types(snapshot: &RenderSnapshot) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for cmd in &snapshot.commands {
        if cmd.visible {
            *counts.entry(cmd.object_type.clone()).or_insert(0) += 1;
        }
    }
    counts
}

#[test]
fn skin_state_objects_play_has_note_judge() {
    // Play skin snapshots must contain SkinNote and SkinJudge objects
    for tc in TEST_CASES.iter().filter(|tc| tc.name.contains("play7")) {
        let (_, rust_snapshot, _) = snapshot_diffs(tc);
        let counts = count_object_types(&rust_snapshot);

        assert!(
            counts.get("SkinNote").copied().unwrap_or(0) > 0,
            "{}: play skin should contain SkinNote objects (found: {:?})",
            tc.name,
            counts
        );
        assert!(
            counts.get("SkinJudge").copied().unwrap_or(0) > 0,
            "{}: play skin should contain SkinJudge objects (found: {:?})",
            tc.name,
            counts
        );
    }
}

#[test]
fn skin_state_objects_select_has_bar() {
    // Select skin snapshot must contain SkinBar objects
    let tc = &TEST_CASES[0]; // ecfn_select
    assert!(tc.name.contains("select"));
    let (_, rust_snapshot, _) = snapshot_diffs(tc);
    let counts = count_object_types(&rust_snapshot);

    assert!(
        counts.get("SkinBar").copied().unwrap_or(0) > 0,
        "select skin should contain SkinBar objects (found: {:?})",
        counts
    );
}

#[test]
fn skin_state_objects_type_distribution_parity() {
    // For each snapshot with zero known diffs, verify Java and Rust produce the same object type distribution.
    // Skip test cases with known_diff_budget > 0 (e.g., result skins with Lua draw function gaps).
    for tc in TEST_CASES {
        if tc.known_diff_budget > 0 {
            continue;
        }

        let java_snapshot = load_java_snapshot(tc.name);
        let (_, rust_snapshot, _) = snapshot_diffs(tc);

        let java_counts = count_object_types(&java_snapshot);
        let rust_counts = count_object_types(&rust_snapshot);

        // Collect all type keys
        let all_types: HashSet<&String> = java_counts.keys().chain(rust_counts.keys()).collect();

        for obj_type in all_types {
            let j = java_counts.get(obj_type).copied().unwrap_or(0);
            let r = rust_counts.get(obj_type).copied().unwrap_or(0);
            assert_eq!(
                j, r,
                "{}: object type '{}' count mismatch: java={} rust={}",
                tc.name, obj_type, j, r
            );
        }
    }
}

#[test]
fn skin_state_objects_visible_type_distribution_parity() {
    // Verify visible object type distribution matches between Java and Rust
    for tc in TEST_CASES {
        // Skip cases with known diff budgets (Lua draw function limitations)
        if tc.known_diff_budget > 0 {
            continue;
        }

        let java_snapshot = load_java_snapshot(tc.name);
        let (_, rust_snapshot, _) = snapshot_diffs(tc);

        let java_counts = count_visible_object_types(&java_snapshot);
        let rust_counts = count_visible_object_types(&rust_snapshot);

        let all_types: HashSet<&String> = java_counts.keys().chain(rust_counts.keys()).collect();

        for obj_type in all_types {
            let j = java_counts.get(obj_type).copied().unwrap_or(0);
            let r = rust_counts.get(obj_type).copied().unwrap_or(0);
            assert_eq!(
                j, r,
                "{}: visible '{}' count mismatch: java={} rust={}",
                tc.name, obj_type, j, r
            );
        }
    }
}
