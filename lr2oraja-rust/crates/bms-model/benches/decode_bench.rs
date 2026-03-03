use std::path::{Path, PathBuf};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

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

/// Discover real .bms files under bms/bms-001/ and bms/bms-002/.
///
/// These are larger production BMS charts. Returns an empty Vec if the bms/
/// directory is not found (it is gitignored and may not be present in worktrees).
fn discover_real_bms_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Try relative path from CARGO_MANIFEST_DIR (crates/bms-model -> repo root -> bms/)
    let candidates = [
        manifest.join("../../../bms"),
        manifest.join("../../bms"),
    ];

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

fn bench_real_bms_decode_path(c: &mut Criterion) {
    let files = discover_real_bms_files();
    if files.is_empty() {
        eprintln!("Skipping real BMS decode_path benchmarks: bms/ directory not found");
        return;
    }

    let mut group = c.benchmark_group("real_bms_decode_path");
    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        group.bench_with_input(BenchmarkId::new("decode_path", &filename), path, |b, p| {
            b.iter(|| {
                let mut decoder = BMSDecoder::new();
                decoder.decode_path(p).expect("decode_path returned None");
            });
        });
    }
    group.finish();
}

fn bench_real_bms_decode_bytes(c: &mut Criterion) {
    let files = discover_real_bms_files();
    if files.is_empty() {
        eprintln!("Skipping real BMS decode_bytes benchmarks: bms/ directory not found");
        return;
    }

    let mut group = c.benchmark_group("real_bms_decode_bytes");
    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let bytes = std::fs::read(path).expect("Failed to read BMS file");
        group.bench_with_input(
            BenchmarkId::new("decode_bytes", &filename),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let mut decoder = BMSDecoder::new();
                    decoder
                        .decode_bytes(data, false, None)
                        .expect("decode_bytes returned None");
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_decode_bms,
    bench_decode_bytes_only,
    bench_real_bms_decode_path,
    bench_real_bms_decode_bytes
);
criterion_main!(benches);
