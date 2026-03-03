use std::path::{Path, PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};

use bms_model::bms_decoder::BMSDecoder;

/// Discover .bms files under the test-bms directory.
fn discover_test_bms_files() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms");
    if !base.is_dir() {
        panic!("test-bms directory not found: {}", base.display());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&base).expect("Failed to read test-bms directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("bms") {
            files.push(path);
        }
    }

    files.sort();
    files
}

fn bench_decode_bms(c: &mut Criterion) {
    let files = discover_test_bms_files();
    assert!(!files.is_empty(), "No .bms files found in test-bms/");
    let first = &files[0];
    let filename = first.file_name().unwrap().to_string_lossy();

    c.bench_function(&format!("decode_bms/{filename}"), |b| {
        b.iter(|| {
            let mut decoder = BMSDecoder::new();
            decoder
                .decode_path(first)
                .expect("decode_path returned None");
        });
    });
}

fn bench_decode_bytes_only(c: &mut Criterion) {
    let files = discover_test_bms_files();
    assert!(!files.is_empty(), "No .bms files found in test-bms/");
    let first = &files[0];
    let filename = first.file_name().unwrap().to_string_lossy();
    let bytes = std::fs::read(first).expect("Failed to read BMS file");

    c.bench_function(&format!("decode_bytes/{filename}"), |b| {
        b.iter(|| {
            let mut decoder = BMSDecoder::new();
            decoder
                .decode_bytes(&bytes, false, None)
                .expect("decode_bytes returned None");
        });
    });
}

criterion_group!(benches, bench_decode_bms, bench_decode_bytes_only);
criterion_main!(benches);
