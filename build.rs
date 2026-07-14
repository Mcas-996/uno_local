use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=UNO_GIT_COMMIT");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    track_git_head();

    let commit = env::var("UNO_GIT_COMMIT")
        .ok()
        .and_then(|value| normalize_commit(&value))
        .or_else(|| {
            env::var("GITHUB_SHA")
                .ok()
                .and_then(|value| normalize_commit(&value))
        })
        .or_else(git_commit)
        .unwrap_or_else(|| "unknown".to_owned());

    println!("cargo:rustc-env=UNO_GIT_COMMIT={commit}");
}

fn git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
        .and_then(|value| normalize_commit(&value))
}

fn normalize_commit(value: &str) -> Option<String> {
    let value = value.trim();
    (value.len() >= 12 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value[..12].to_ascii_lowercase())
}

fn track_git_head() {
    let Ok(output) = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }

    let git_dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );

    if let Ok(contents) = fs::read_to_string(head)
        && let Some(reference) = contents.trim().strip_prefix("ref: ")
    {
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join(reference).display()
        );
    }
}
