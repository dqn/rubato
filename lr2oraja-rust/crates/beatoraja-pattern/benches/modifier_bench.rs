use std::path::{Path, PathBuf};

use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use beatoraja_core::player_config::PlayerConfig;
use beatoraja_pattern::lane_shuffle_modifier::{
    LaneMirrorShuffleModifier, LaneRandomShuffleModifier, LaneRotateShuffleModifier,
};
use beatoraja_pattern::note_shuffle_modifier::NoteShuffleModifier;
use beatoraja_pattern::pattern_modifier::PatternModifier;
use beatoraja_pattern::random::Random;
use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

fn make_test_model(mode: &Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_all_time_line(timelines);
    model.set_mode(mode.clone());
    model
}

/// Build a model with the given number of timelines, each having notes on
/// all lanes. Simulates a dense BMS chart.
fn make_dense_model(mode: &Mode, timeline_count: usize) -> BMSModel {
    let key_count = mode.key() as usize;
    let mut timelines = Vec::with_capacity(timeline_count);
    for i in 0..timeline_count {
        let mut tl = TimeLine::new(i as f64, (i * 1000) as i64, key_count as i32);
        for lane in 0..key_count {
            let wav = (i as i32) * 100 + lane as i32;
            tl.set_note(lane as i32, Some(Note::new_normal(wav)));
        }
        timelines.push(tl);
    }
    make_test_model(mode, timelines)
}

/// Discover real .bms files under bms/bms-001/ and bms/bms-002/.
///
/// Returns an empty Vec if the bms/ directory is not found (it is gitignored
/// and may not be present in worktrees).
fn discover_real_bms_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [manifest.join("../../../bms"), manifest.join("../../bms")];

    let base = match candidates.iter().find(|p| p.is_dir()) {
        Some(p) => p.clone(),
        None => return Vec::new(),
    };

    let subdirs = ["bms-001", "bms-002"];
    let mut files = Vec::new();

    for subdir in &subdirs {
        let dir = base.join(subdir);
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir).expect("Failed to read BMS directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("bms") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

/// Decode a BMS file into a BMSModel.
fn decode_bms(path: &Path) -> BMSModel {
    let mut decoder = BMSDecoder::new();
    decoder
        .decode_path(path)
        .unwrap_or_else(|| panic!("BMSDecoder returned None for {}", path.display()))
}

// ---------------------------------------------------------------------------
// Synthetic benchmarks (existing)
// ---------------------------------------------------------------------------

fn bench_mirror(c: &mut Criterion) {
    let mode = Mode::BEAT_7K;
    c.bench_function("mirror", |b| {
        b.iter_batched(
            || {
                let model = make_dense_model(&mode, 100);
                let mut modifier = LaneMirrorShuffleModifier::new(0, false);
                modifier.set_seed(42);
                (model, modifier)
            },
            |(mut model, mut modifier)| {
                modifier.modify(black_box(&mut model));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_random(c: &mut Criterion) {
    let mode = Mode::BEAT_7K;
    c.bench_function("random", |b| {
        b.iter_batched(
            || {
                let model = make_dense_model(&mode, 100);
                let mut modifier = LaneRandomShuffleModifier::new(0, false);
                modifier.set_seed(42);
                (model, modifier)
            },
            |(mut model, mut modifier)| {
                modifier.modify(black_box(&mut model));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_rotate(c: &mut Criterion) {
    let mode = Mode::BEAT_7K;
    c.bench_function("rotate", |b| {
        b.iter_batched(
            || {
                let model = make_dense_model(&mode, 100);
                let mut modifier = LaneRotateShuffleModifier::new(0, false);
                modifier.set_seed(42);
                (model, modifier)
            },
            |(mut model, mut modifier)| {
                modifier.modify(black_box(&mut model));
            },
            BatchSize::SmallInput,
        );
    });
}

// ---------------------------------------------------------------------------
// Real BMS benchmarks
// ---------------------------------------------------------------------------

fn bench_real_bms_mirror(c: &mut Criterion) {
    let files = discover_real_bms_files();
    if files.is_empty() {
        eprintln!("Skipping real BMS mirror benchmarks: bms/ directory not found");
        return;
    }

    let mut group = c.benchmark_group("real_bms_mirror");
    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let base_model = decode_bms(path);

        group.bench_with_input(BenchmarkId::new("mirror", &filename), &base_model, |b, m| {
            b.iter_batched(
                || {
                    let model = m.clone();
                    let mut modifier = LaneMirrorShuffleModifier::new(0, false);
                    modifier.set_seed(42);
                    (model, modifier)
                },
                |(mut model, mut modifier)| {
                    modifier.modify(black_box(&mut model));
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_real_bms_random(c: &mut Criterion) {
    let files = discover_real_bms_files();
    if files.is_empty() {
        eprintln!("Skipping real BMS random benchmarks: bms/ directory not found");
        return;
    }

    let mut group = c.benchmark_group("real_bms_random");
    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let base_model = decode_bms(path);

        group.bench_with_input(
            BenchmarkId::new("random", &filename),
            &base_model,
            |b, m| {
                b.iter_batched(
                    || {
                        let model = m.clone();
                        let mut modifier = LaneRandomShuffleModifier::new(0, false);
                        modifier.set_seed(42);
                        (model, modifier)
                    },
                    |(mut model, mut modifier)| {
                        modifier.modify(black_box(&mut model));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn bench_real_bms_srandom(c: &mut Criterion) {
    let files = discover_real_bms_files();
    if files.is_empty() {
        eprintln!("Skipping real BMS S-Random benchmarks: bms/ directory not found");
        return;
    }

    let config = PlayerConfig::default();

    let mut group = c.benchmark_group("real_bms_srandom");
    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let base_model = decode_bms(path);
        let mode = base_model
            .get_mode()
            .unwrap_or_else(|| panic!("{filename}: mode should be set"))
            .clone();

        group.bench_with_input(
            BenchmarkId::new("srandom", &filename),
            &base_model,
            |b, m| {
                b.iter_batched(
                    || {
                        let model = m.clone();
                        let mut modifier =
                            NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
                        modifier.set_seed(42);
                        (model, modifier)
                    },
                    |(mut model, mut modifier)| {
                        modifier.modify(black_box(&mut model));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_mirror,
    bench_random,
    bench_rotate,
    bench_real_bms_mirror,
    bench_real_bms_random,
    bench_real_bms_srandom
);
criterion_main!(benches);
