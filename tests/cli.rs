use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn empty_working_directory() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("uno-cli-test-{}-{unique}", std::process::id()));
    fs::create_dir(&path).expect("failed to create temporary working directory");
    path
}

#[test]
fn version_flags_work_without_cargo_or_git_files() {
    let working_directory = empty_working_directory();
    let expected = format!(
        "uno {} (commit {})",
        env!("CARGO_PKG_VERSION"),
        env!("UNO_GIT_COMMIT")
    );

    for argument in ["-v", "--version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_uno"))
            .arg(argument)
            .current_dir(&working_directory)
            .output()
            .expect("failed to run uno");

        assert!(output.status.success(), "{argument} should succeed");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), expected);
        assert!(
            output.stderr.is_empty(),
            "{argument} should not write stderr"
        );
    }

    fs::remove_dir(working_directory).expect("failed to remove temporary working directory");
}

#[test]
fn version_flag_rejects_additional_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_uno"))
        .args(["--version", "extra"])
        .output()
        .expect("failed to run uno");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("does not accept positional arguments")
    );
}
