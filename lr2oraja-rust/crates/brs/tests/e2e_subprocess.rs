//! Subprocess E2E test — builds and runs the brs binary in headless mode.
//!
//! Verifies that the full application lifecycle works end-to-end:
//! BMS load → Autoplay → Result → exit with code 0.

use std::path::Path;
use std::process::Command;

/// Run the brs binary in headless autoplay mode and verify it exits cleanly.
///
/// This test builds the binary via `cargo build` and then executes it with
/// `--headless --exit-after-result --autoplay --no-launcher --bms <path>`.
/// It asserts exit code 0 and checks stderr logs for state transition markers.
#[test]
#[ignore] // Requires building the binary; run with: cargo test -p brs --test e2e_subprocess -- --ignored
fn e2e_headless_autoplay_subprocess() {
    let bms_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-bms/minimal_7k.bms");
    assert!(
        bms_path.exists(),
        "Test BMS file not found: {}",
        bms_path.display()
    );

    // Build the binary first
    let build_status = Command::new("cargo")
        .args(["build", "-p", "brs"])
        .status()
        .expect("Failed to run cargo build");
    assert!(build_status.success(), "cargo build -p brs failed");

    // Find the built binary
    let binary = env!("CARGO_BIN_EXE_brs");

    let output = Command::new(binary)
        .args([
            "--bms",
            bms_path.to_str().unwrap(),
            "--autoplay",
            "--no-launcher",
            "--headless",
            "--exit-after-result",
        ])
        .env("RUST_LOG", "info")
        .output()
        .expect("Failed to run brs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // tracing_subscriber::fmt() writes to stdout by default
    let logs = format!("{stdout}{stderr}");

    assert!(
        output.status.success(),
        "brs exited with error (code {:?}):\n{logs}",
        output.status.code(),
    );

    // Verify key state transitions appeared in logs
    assert!(
        logs.contains("MusicSelect"),
        "Logs should mention MusicSelect:\n{logs}"
    );
    assert!(
        logs.contains("MusicDecide"),
        "Logs should mention MusicDecide:\n{logs}"
    );
    assert!(logs.contains("Play:"), "Logs should mention Play:\n{logs}");
    assert!(
        logs.contains("Result:"),
        "Logs should mention Result:\n{logs}"
    );
    assert!(
        logs.contains("exit after result"),
        "Logs should mention exit after result:\n{logs}"
    );
}
