check:
    cargo check --workspace

test:
    cargo nextest run --workspace -E 'not test(render_snapshot_parity)'

test-all:
    cargo nextest run --workspace

clippy:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

# Run golden master comparison tests
golden-master-test:
    cargo test -p golden-master -- --nocapture

# Update skin snapshot fixtures
golden-master-skin-snapshot:
    UPDATE_SNAPSHOTS=1 cargo test -p golden-master compare_skin -- --nocapture

# Screenshot tests (GPU required, local execution)
screenshot-test:
    cargo test -p bms-render --test screenshot_tests -- --ignored --nocapture

# Regenerate screenshot fixtures
screenshot-update:
    UPDATE_SCREENSHOTS=1 cargo test -p bms-render --test screenshot_tests -- --ignored --nocapture

# Compare Java-Rust RenderSnapshot (structural draw command comparison)
golden-master-render-snapshot-compare:
    cargo test -p golden-master --test compare_render_snapshot -- --nocapture --ignored

# Run criterion benchmarks
bench:
    cargo bench --workspace

# Run tests with coverage (text summary)
coverage:
    cargo llvm-cov --workspace

# Run tests with coverage (HTML report)
coverage-html:
    cargo llvm-cov --workspace --html
    @echo "Report: target/llvm-cov/html/index.html"

# Run E2E tests (in-process + subprocess)
e2e-test:
    cargo test -p rubato-bin e2e_ -- --nocapture

# Run E2E subprocess test (requires binary build, ~30s)
e2e-test-subprocess:
    cargo test -p rubato-bin --test e2e_subprocess -- --ignored --nocapture

# Seed fuzz corpus directories with real BMS files
fuzz-seed:
    #!/usr/bin/env bash
    set -euo pipefail
    BMS_CORPUS="crates/bms-model/fuzz/corpus/fuzz_bms"
    BMSON_CORPUS="crates/bms-model/fuzz/corpus/fuzz_bmson"
    OSU_CORPUS="crates/bms-model/fuzz/corpus/fuzz_osu"
    mkdir -p "$BMS_CORPUS" "$BMSON_CORPUS" "$OSU_CORPUS"
    bms_count=0
    for f in bms/bms-001/*.bms bms/bms-002/*.bms; do
        [ -f "$f" ] || continue
        cp "$f" "$BMS_CORPUS/real_$(basename "$f")"
        bms_count=$((bms_count + 1))
    done
    for f in test-bms/*.bms; do
        [ -f "$f" ] || continue
        cp "$f" "$BMS_CORPUS/test_$(basename "$f")"
        bms_count=$((bms_count + 1))
    done
    bmson_count=0
    for f in test-bms/*.bmson; do
        [ -f "$f" ] || continue
        cp "$f" "$BMSON_CORPUS/test_$(basename "$f")"
        bmson_count=$((bmson_count + 1))
    done
    osu_count=0
    for f in test-bms/*.osu; do
        [ -f "$f" ] || continue
        cp "$f" "$OSU_CORPUS/test_$(basename "$f")"
        osu_count=$((osu_count + 1))
    done
    echo "Seeded $bms_count BMS, $bmson_count BMSON, $osu_count OSU files"

# Run a single fuzz target
fuzz TARGET FUZZ_DURATION="60":
    cd crates/bms-model && cargo +nightly fuzz run {{TARGET}} -- -max_total_time={{FUZZ_DURATION}}

# Run all 3 fuzz targets sequentially
fuzz-all FUZZ_DURATION="60":
    just fuzz fuzz_bms {{FUZZ_DURATION}}
    just fuzz fuzz_bmson {{FUZZ_DURATION}}
    just fuzz fuzz_osu {{FUZZ_DURATION}}

# Update real BMS golden master fixtures
golden-master-real-bms-update:
    UPDATE_REAL_BMS_FIXTURES=1 cargo test -p golden-master --test real_bms_integration real_bms_golden_master_regression -- --nocapture

# Update ECFN timepoint snapshot fixtures
golden-master-ecfn-timepoint-update:
    UPDATE_ECFN_TIMEPOINT_SNAPSHOTS=1 cargo test -p golden-master --test skin_ecfn_integration skin_ecfn_timepoint_snapshot_regression -- --nocapture

all: check test clippy fmt-check
