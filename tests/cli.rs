use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

struct ManagedInstall {
    root: PathBuf,
    executable: PathBuf,
    updater: PathBuf,
    shared_binary: PathBuf,
    receipt: PathBuf,
    config_home: PathBuf,
}

impl ManagedInstall {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("uno cli & spaces-{}-{unique}", std::process::id()));
        let prefix = root.join("install prefix");
        let bin = prefix.join("bin");
        let config_home = root.join("config home");
        let receipt_dir = config_home.join("uno");
        fs::create_dir_all(&bin).expect("failed to create test bin directory");
        fs::create_dir_all(&receipt_dir).expect("failed to create test receipt directory");

        let executable = bin.join(if cfg!(windows) { "uno.exe" } else { "uno" });
        let updater = bin.join(if cfg!(windows) {
            "uno-update.exe"
        } else {
            "uno-update"
        });
        fs::copy(env!("CARGO_BIN_EXE_uno"), &executable)
            .expect("failed to copy test UNO executable");
        fs::write(&updater, b"test updater").expect("failed to create test updater");
        let shared_binary = bin.join(if cfg!(windows) {
            "other-tool.exe"
        } else {
            "other-tool"
        });
        fs::write(&shared_binary, b"shared tool").expect("failed to create shared binary");

        let receipt = receipt_dir.join("uno-receipt.json");
        write_receipt(&receipt, &prefix);
        Self {
            root,
            executable,
            updater,
            shared_binary,
            receipt,
            config_home,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("LOCALAPPDATA", self.root.join("unused local app data"))
            .env("HOME", self.root.join("unused home"));
        command
    }

    fn assert_present(&self) {
        assert!(self.executable.is_file(), "UNO executable should remain");
        assert!(self.updater.is_file(), "UNO updater should remain");
        assert!(self.receipt.is_file(), "UNO receipt should remain");
        assert!(self.shared_binary.is_file(), "shared binary should remain");
    }

    fn wait_until_removed(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let receipt_dir = self.receipt.parent().expect("receipt should have a parent");
        while Instant::now() < deadline
            && (self.executable.exists()
                || self.updater.exists()
                || self.receipt.exists()
                || receipt_dir.exists())
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!self.executable.exists(), "UNO executable was not removed");
        assert!(!self.updater.exists(), "UNO updater was not removed");
        assert!(!self.receipt.exists(), "UNO receipt was not removed");
        assert!(self.shared_binary.is_file(), "shared binary was removed");
        assert!(
            !receipt_dir.exists(),
            "empty receipt directory was not removed"
        );
    }
}

impl Drop for ManagedInstall {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_receipt(path: &Path, install_prefix: &Path) {
    let receipt = serde_json::json!({
        "install_prefix": install_prefix,
        "binaries": ["uno"],
        "source": {
            "app_name": "uno",
            "name": "uno",
            "owner": "Mcas-996",
            "release_type": "github"
        },
        "provider": { "source": "cargo-dist", "version": "0.32.0" },
        "version": env!("CARGO_PKG_VERSION")
    });
    fs::write(path, receipt.to_string()).expect("failed to write test receipt");
}

#[test]
fn interactive_uninstall_cancels_without_deleting_files() {
    let install = ManagedInstall::new();
    let mut child = install
        .command()
        .arg("--uninstall")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run copied UNO");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"n\n")
        .expect("failed to answer confirmation");
    let output = child.wait_with_output().expect("failed to wait for UNO");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("This will remove:"));
    assert!(stdout.contains("Uninstall cancelled."));
    assert!(output.stderr.is_empty());
    install.assert_present();
}

#[test]
fn interactive_uninstall_treats_end_of_input_as_cancellation() {
    let install = ManagedInstall::new();
    let output = install
        .command()
        .arg("--uninstall")
        .output()
        .expect("failed to run copied UNO");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Uninstall cancelled."));
    assert!(output.stderr.is_empty());
    install.assert_present();
}

#[test]
fn interactive_uninstall_accepts_y_and_removes_managed_files() {
    let install = ManagedInstall::new();
    let mut child = install
        .command()
        .arg("--uninstall")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run copied UNO");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"y\n")
        .expect("failed to answer confirmation");
    let output = child.wait_with_output().expect("failed to wait for UNO");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    install.wait_until_removed();
}

#[test]
fn uninstall_force_flags_skip_confirmation() {
    for confirmation in ["-y", "--yes"] {
        let install = ManagedInstall::new();
        let output = install
            .command()
            .args(["--uninstall", confirmation])
            .output()
            .expect("failed to run copied UNO");

        assert!(output.status.success(), "{confirmation} should succeed");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("Continue?"));
        assert!(output.stderr.is_empty());
        install.wait_until_removed();
    }
}

#[test]
fn uninstall_succeeds_when_updater_is_already_missing() {
    let install = ManagedInstall::new();
    fs::remove_file(&install.updater).expect("failed to remove updater fixture");

    let output = install
        .command()
        .args(["--uninstall", "--yes"])
        .output()
        .expect("failed to run copied UNO");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    install.wait_until_removed();
}

#[test]
fn uninstall_rejects_a_receipt_for_another_location() {
    let install = ManagedInstall::new();
    let other_prefix = install.root.join("other prefix");
    fs::create_dir_all(other_prefix.join("bin")).expect("failed to create other prefix");
    write_receipt(&install.receipt, &other_prefix);

    let output = install
        .command()
        .args(["--uninstall", "--yes"])
        .output()
        .expect("failed to run copied UNO");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a matching cargo-dist"));
    install.assert_present();
}

#[test]
fn help_documents_uninstall_and_extra_force_arguments_are_rejected() {
    let help = Command::new(env!("CARGO_BIN_EXE_uno"))
        .arg("--help")
        .output()
        .expect("failed to run UNO help");
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help.status.success());
    assert!(stdout.contains("--uninstall"));
    assert!(stdout.contains("-y, --yes"));

    let invalid = Command::new(env!("CARGO_BIN_EXE_uno"))
        .args(["--uninstall", "--yes", "extra"])
        .output()
        .expect("failed to run UNO with invalid uninstall arguments");
    assert!(!invalid.status.success());
    assert!(
        String::from_utf8_lossy(&invalid.stderr).contains("does not accept positional arguments")
    );
}
