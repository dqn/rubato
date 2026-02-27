use std::process::Command;

fn beatoraja_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_beatoraja"))
}

#[test]
fn help_flag() {
    let output = beatoraja_bin()
        .arg("--help")
        .output()
        .expect("failed to execute binary");

    assert!(output.status.success(), "exit code was not 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("BMS player"),
        "stdout did not contain 'BMS player': {stdout}"
    );
}

#[test]
fn version_flag() {
    let output = beatoraja_bin()
        .arg("--version")
        .output()
        .expect("failed to execute binary");

    assert!(output.status.success(), "exit code was not 0");
}

#[test]
fn invalid_flag_exits_error() {
    let output = beatoraja_bin()
        .arg("--invalid-nonexistent-flag")
        .output()
        .expect("failed to execute binary");

    assert!(
        !output.status.success(),
        "expected non-zero exit code for invalid flag"
    );
}

/// Run the binary with no arguments in a clean tempdir (no config file).
///
/// The binary will attempt to launch the configuration UI (eframe/egui),
/// which requires a display server. In headless CI environments this will
/// fail, so the test is marked `#[ignore]`. The key assertion is that the
/// process does not crash with a panic / signal — it should exit with an
/// ordinary error code.
#[test]
#[ignore]
fn no_config_runs_without_crash() {
    let tmp = tempfile::TempDir::new().expect("failed to create tempdir");

    let output = beatoraja_bin()
        .current_dir(tmp.path())
        .output()
        .expect("failed to execute binary");

    // On Unix, a signal-killed process has no exit code.
    // If the process panicked, the exit code is typically 101.
    // We allow any "normal" exit (including non-zero for missing display),
    // but reject signal termination and Rust panic code 101.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            output.status.signal().is_none(),
            "process was killed by signal: {:?}",
            output.status.signal()
        );
    }

    if let Some(code) = output.status.code() {
        assert_ne!(
            code,
            101,
            "process exited with code 101 (Rust panic). stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
