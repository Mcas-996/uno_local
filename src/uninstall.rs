use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;

const APP_NAME: &str = "uno";
const RECEIPT_NAME: &str = "uno-receipt.json";

#[derive(Debug, Deserialize)]
struct InstallReceipt {
    install_prefix: PathBuf,
    binaries: Vec<String>,
    source: ReceiptSource,
    provider: ReceiptProvider,
}

#[derive(Debug, Deserialize)]
struct ReceiptSource {
    app_name: String,
}

#[derive(Debug, Deserialize)]
struct ReceiptProvider {
    source: String,
}

#[derive(Debug, PartialEq, Eq)]
struct UninstallPlan {
    executable: PathBuf,
    updater: PathBuf,
    receipt: PathBuf,
}

pub fn run(force: bool) -> Result<(), String> {
    let plan = UninstallPlan::discover()?;

    if !force {
        print_targets(&plan);
        if !confirm()? {
            println!("Uninstall cancelled.");
            return Ok(());
        }
    }

    execute(&plan)?;
    #[cfg(windows)]
    println!("UNO uninstall is scheduled and will finish after this process exits.");
    #[cfg(not(windows))]
    println!("UNO was uninstalled successfully.");
    Ok(())
}

impl UninstallPlan {
    fn discover() -> Result<Self, String> {
        let executable = env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|error| format!("cannot resolve the running UNO executable: {error}"))?;
        let receipt = find_receipt()?;
        Self::from_paths(executable, receipt)
    }

    fn from_paths(executable: PathBuf, receipt_path: PathBuf) -> Result<Self, String> {
        let executable = fs::canonicalize(&executable)
            .map_err(|error| format!("cannot resolve the running UNO executable: {error}"))?;
        if !is_uno_executable(&executable) {
            return Err(not_managed_message());
        }

        let contents = fs::read_to_string(&receipt_path)
            .map_err(|error| format!("cannot read {}: {error}", receipt_path.display()))?;
        let receipt: InstallReceipt = serde_json::from_str(&contents)
            .map_err(|error| format!("cannot parse {}: {error}", receipt_path.display()))?;

        if receipt.provider.source != "cargo-dist"
            || receipt.source.app_name != APP_NAME
            || !receipt.binaries.iter().any(|binary| is_uno_name(binary))
        {
            return Err(not_managed_message());
        }

        let executable_dir = executable.parent().ok_or_else(not_managed_message)?;
        let prefix =
            fs::canonicalize(&receipt.install_prefix).map_err(|_| not_managed_message())?;
        let prefix_matches = prefix == executable_dir
            || fs::canonicalize(prefix.join("bin")).is_ok_and(|bin_dir| bin_dir == executable_dir);
        if !prefix_matches {
            return Err(not_managed_message());
        }

        let updater = executable.with_file_name(updater_file_name());
        Ok(Self {
            executable,
            updater,
            receipt: receipt_path,
        })
    }
}

fn find_receipt() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(config_home) = non_empty_env("XDG_CONFIG_HOME") {
        candidates.push(config_home.join(APP_NAME).join(RECEIPT_NAME));
    }

    #[cfg(windows)]
    if let Some(local_app_data) = non_empty_env("LOCALAPPDATA") {
        candidates.push(local_app_data.join(APP_NAME).join(RECEIPT_NAME));
    }

    #[cfg(not(windows))]
    if let Some(home) = non_empty_env("HOME") {
        candidates.push(home.join(".config").join(APP_NAME).join(RECEIPT_NAME));
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(not_managed_message)
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn is_uno_executable(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(is_uno_name)
}

fn is_uno_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(APP_NAME) || name.eq_ignore_ascii_case("uno.exe")
}

fn updater_file_name() -> &'static str {
    if cfg!(windows) {
        "uno-update.exe"
    } else {
        "uno-update"
    }
}

fn not_managed_message() -> String {
    "this copy of UNO is not a matching cargo-dist installation; use the package manager that installed it or remove it manually".to_owned()
}

fn print_targets(plan: &UninstallPlan) {
    println!("This will remove:");
    println!("  {}", plan.executable.display());
    println!("  {}", plan.updater.display());
    println!("  {}", plan.receipt.display());
}

fn confirm() -> Result<bool, String> {
    print!("Continue? [y/N] ");
    io::stdout()
        .flush()
        .map_err(|error| format!("cannot display uninstall confirmation: {error}"))?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| format!("cannot read uninstall confirmation: {error}"))?;
    Ok(is_confirmation(&answer))
}

fn is_confirmation(answer: &str) -> bool {
    let answer = answer.trim();
    answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
}

#[cfg(not(windows))]
fn execute(plan: &UninstallPlan) -> Result<(), String> {
    remove_optional_file(&plan.updater)?;
    fs::remove_file(&plan.executable)
        .map_err(|error| format!("cannot remove {}: {error}", plan.executable.display()))?;
    fs::remove_file(&plan.receipt)
        .map_err(|error| format!("cannot remove {}: {error}", plan.receipt.display()))?;
    remove_empty_receipt_dir(&plan.receipt)
}

#[cfg(not(windows))]
fn remove_optional_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot remove {}: {error}", path.display())),
    }
}

#[cfg(not(windows))]
fn remove_empty_receipt_dir(receipt: &Path) -> Result<(), String> {
    let Some(directory) = receipt.parent() else {
        return Ok(());
    };
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot inspect {}: {error}", directory.display()))?;
    if entries.next().is_none() {
        fs::remove_dir(directory)
            .map_err(|error| format!("cannot remove {}: {error}", directory.display()))?;
    }
    Ok(())
}

#[cfg(windows)]
fn execute(plan: &UninstallPlan) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CLEANUP_SCRIPT: &str = r#"
$parentId = [int]$env:UNO_UNINSTALL_PARENT_PID
Wait-Process -Id $parentId -ErrorAction SilentlyContinue
$binaryFailed = $false
foreach ($path in @($env:UNO_UNINSTALL_EXE, $env:UNO_UNINSTALL_UPDATER)) {
    for ($attempt = 0; $attempt -lt 50 -and (Test-Path -LiteralPath $path); $attempt++) {
        Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
        if (Test-Path -LiteralPath $path) { Start-Sleep -Milliseconds 100 }
    }
    if (Test-Path -LiteralPath $path) { $binaryFailed = $true }
}
if (-not $binaryFailed) {
    $receipt = $env:UNO_UNINSTALL_RECEIPT
    Remove-Item -LiteralPath $receipt -Force -ErrorAction SilentlyContinue
    if (-not (Test-Path -LiteralPath $receipt)) {
        $directory = Split-Path -Parent $receipt
        if ((Test-Path -LiteralPath $directory) -and
            @((Get-ChildItem -LiteralPath $directory -Force -ErrorAction SilentlyContinue)).Count -eq 0) {
            Remove-Item -LiteralPath $directory -Force -ErrorAction SilentlyContinue
        }
    }
}
"#;

    Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            CLEANUP_SCRIPT,
        ])
        .env("UNO_UNINSTALL_PARENT_PID", std::process::id().to_string())
        .env("UNO_UNINSTALL_EXE", &plan.executable)
        .env("UNO_UNINSTALL_UPDATER", &plan.updater)
        .env("UNO_UNINSTALL_RECEIPT", &plan.receipt)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot start the Windows uninstall helper: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        executable: PathBuf,
        receipt: PathBuf,
    }

    impl Fixture {
        fn new(prefix_is_bin: bool) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after the Unix epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!(
                "uno-uninstall-unit-{}-{unique}",
                std::process::id()
            ));
            let prefix = root.join("prefix");
            let bin = prefix.join("bin");
            let config = root.join("config").join("uno");
            fs::create_dir_all(&bin).expect("failed to create bin directory");
            fs::create_dir_all(&config).expect("failed to create config directory");
            let executable = bin.join(if cfg!(windows) { "uno.exe" } else { "uno" });
            fs::write(&executable, b"test").expect("failed to create executable");
            let receipt = config.join(RECEIPT_NAME);
            let install_prefix = if prefix_is_bin { &bin } else { &prefix };
            let json = serde_json::json!({
                "install_prefix": install_prefix,
                "binaries": ["uno"],
                "source": { "app_name": "uno" },
                "provider": { "source": "cargo-dist", "version": "0.32.0" },
                "version": "1.0.0"
            });
            fs::write(&receipt, json.to_string()).expect("failed to create receipt");
            Self {
                root,
                executable,
                receipt,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn confirmation_accepts_only_y_and_yes() {
        for answer in ["y", "Y\n", "yes", " YeS \r\n"] {
            assert!(is_confirmation(answer), "{answer:?} should confirm");
        }
        for answer in ["", "\n", "n", "true", "yeah"] {
            assert!(!is_confirmation(answer), "{answer:?} should cancel");
        }
    }

    #[test]
    fn receipt_accepts_root_and_legacy_bin_prefixes() {
        for prefix_is_bin in [false, true] {
            let fixture = Fixture::new(prefix_is_bin);
            let plan =
                UninstallPlan::from_paths(fixture.executable.clone(), fixture.receipt.clone())
                    .expect("valid receipt should create an uninstall plan");
            assert_eq!(plan.receipt, fixture.receipt);
            assert_eq!(
                plan.updater.file_name(),
                Some(OsStr::new(updater_file_name()))
            );
        }
    }

    #[test]
    fn receipt_rejects_other_provider_and_location() {
        let fixture = Fixture::new(false);
        let contents = fs::read_to_string(&fixture.receipt).expect("failed to read receipt");
        fs::write(
            &fixture.receipt,
            contents.replace("cargo-dist", "other-installer"),
        )
        .expect("failed to rewrite receipt");
        assert!(
            UninstallPlan::from_paths(fixture.executable.clone(), fixture.receipt.clone()).is_err()
        );

        let other = Fixture::new(false);
        assert!(
            UninstallPlan::from_paths(fixture.executable.clone(), other.receipt.clone()).is_err()
        );
    }
}
