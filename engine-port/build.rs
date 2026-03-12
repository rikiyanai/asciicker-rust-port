use std::process::Command;

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../.git/HEAD");

    let git_hash = git_output(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "nogit".into());
    let iteration = git_output(&["rev-list", "--count", "HEAD"]).unwrap_or_else(|| "0".into());

    println!("cargo:rustc-env=ASCIICKER_GIT_HASH={git_hash}");
    println!("cargo:rustc-env=ASCIICKER_BUILD_ITERATION={iteration}");
}
