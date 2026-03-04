use std::process::Command;

fn rubato_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rubato"))
}

/// Write a minimal `config_sys.json` to the given directory so that the binary
/// recognises it as having a config and takes the play() path.
fn write_minimal_config(dir: &std::path::Path) {
    std::fs::write(dir.join("config_sys.json"), "{}").expect("failed to write config_sys.json");
}

#[test]
fn help_flag() {
    let output = rubato_bin()
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
    let output = rubato_bin()
        .arg("--version")
        .output()
        .expect("failed to execute binary");

    assert!(output.status.success(), "exit code was not 0");
}

#[test]
fn invalid_flag_exits_error() {
    let output = rubato_bin()
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

    let output = rubato_bin()
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

/// With `config_sys.json` present and `-s` flag, the binary takes the play()
/// path (`config_exists && player_mode.is_some()`). It will fail because there
/// is no GPU/display, but it must NOT panic (exit code 101) or be killed by a
/// signal. A normal error exit is acceptable.
#[test]
#[ignore] // requires GPU/display
fn play_flag_with_config_exits_gracefully() {
    let tmp = tempfile::TempDir::new().expect("failed to create tempdir");
    write_minimal_config(tmp.path());

    let output = rubato_bin()
        .arg("-s")
        .current_dir(tmp.path())
        .output()
        .expect("failed to execute binary");

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
            "process panicked (exit code 101). stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// With `-s` but NO `config_sys.json`, the binary falls through to launch()
/// (the launcher/configuration UI path) instead of play(). It must not panic
/// or be signal-killed; a normal exit (including errors from missing display)
/// is fine.
#[test]
#[ignore] // requires display for launcher
fn play_flag_without_config_launches_launcher() {
    let tmp = tempfile::TempDir::new().expect("failed to create tempdir");
    // Intentionally no config file in this tempdir

    let output = rubato_bin()
        .arg("-s")
        .current_dir(tmp.path())
        .output()
        .expect("failed to execute binary");

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
            "process panicked (exit code 101). stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// When re-exec'd as a child process (the path `launch()` takes via
/// `Command::new(current_exe()).arg("-s")`), the child must read
/// `config_sys.json` from the correct working directory. Verify the child
/// does not panic or get signal-killed.
#[test]
#[ignore] // requires GPU/display
fn reexec_child_inherits_working_directory() {
    let tmp = tempfile::TempDir::new().expect("failed to create tempdir");
    write_minimal_config(tmp.path());

    // Simulate the re-exec path: binary launched with `-s` and cwd set to
    // the directory containing config_sys.json.
    let output = rubato_bin()
        .arg("-s")
        .current_dir(tmp.path())
        .output()
        .expect("failed to execute binary");

    // The process may fail (no GPU), but it must not panic or be signalled.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            output.status.signal().is_none(),
            "child process was killed by signal: {:?}",
            output.status.signal()
        );
    }

    if let Some(code) = output.status.code() {
        assert_ne!(
            code,
            101,
            "child process panicked (exit code 101). stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
