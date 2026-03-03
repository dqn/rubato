use std::path::{Path, PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};

use beatoraja_core::player_config::PlayerConfig;
use beatoraja_pattern::lane_shuffle_modifier::LaneMirrorShuffleModifier;
use beatoraja_pattern::note_shuffle_modifier::NoteShuffleModifier;
use beatoraja_pattern::pattern_modifier::PatternModifier;
use beatoraja_pattern::random::Random;
use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::BMSModel;

/// Discover .bms files under the test-bms directory and return the first one decoded.
fn load_test_model() -> (String, BMSModel) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms");
    if !base.is_dir() {
        panic!("test-bms directory not found: {}", base.display());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(&base)
        .expect("Failed to read test-bms directory")
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("bms") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "No .bms files found in test-bms/");

    let first = &files[0];
    let filename = first.file_name().unwrap().to_string_lossy().to_string();
    let mut decoder = BMSDecoder::new();
    let model = decoder
        .decode_path(first)
        .expect("decode_path returned None");
    (filename, model)
}

fn bench_mirror_modify(c: &mut Criterion) {
    let (filename, model) = load_test_model();

    c.bench_function(&format!("mirror_modify/{filename}"), |b| {
        b.iter(|| {
            let mut m = model.clone();
            let mut modifier = LaneMirrorShuffleModifier::new(0, false);
            modifier.modify(&mut m);
        });
    });
}

fn bench_srandom_modify(c: &mut Criterion) {
    let (filename, model) = load_test_model();
    let mode = model.get_mode().expect("mode should be set");
    let config = PlayerConfig::default();

    c.bench_function(&format!("srandom_modify/{filename}"), |b| {
        b.iter(|| {
            let mut m = model.clone();
            let mut modifier = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
            modifier.set_seed(42);
            modifier.modify(&mut m);
        });
    });
}

criterion_group!(benches, bench_mirror_modify, bench_srandom_modify);
criterion_main!(benches);
