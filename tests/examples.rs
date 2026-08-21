//! Every example runs, and exits zero.
//!
//! An example that no longer compiles is documentation that lies, and nothing else in a
//! normal `cargo test` looks at one: `cargo test` builds examples but does not run them, so
//! an example that compiles and then panics is invisible. These run them.
//!
//! Each carries the feature set it needs, which is the same set `required-features` declares
//! in the manifest, so a mismatch between the two shows up here.

use std::process::Command;

/// Runs one example under a feature set and gives back whether it exited zero.
fn run(example: &str, features: &[&str]) -> (bool, String) {
    let mut command = Command::new(env!("CARGO"));
    command
        .args(["run", "--quiet", "--example", example])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/examples-check"),
        );
    if !features.is_empty() {
        command.args(["--no-default-features", "--features", &features.join(",")]);
    }
    let output = command.output().expect("cargo runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn taking_ids_runs() {
    let (ok, err) = run("taking_ids", &[]);
    assert!(ok, "the example runs:\n{err}");
}

#[test]
fn two_counters_runs() {
    let (ok, err) = run("two_counters", &[]);
    assert!(ok, "the example runs:\n{err}");
}

#[test]
fn provenance_runs() {
    let (ok, err) = run("provenance", &["u16_index", "ruid_type", "allow_arithmetics"]);
    assert!(ok, "the example runs:\n{err}");
}

#[test]
fn id_pool_runs() {
    let (ok, err) = run("id_pool", &["u32_index", "ruid_type", "no_alloc"]);
    assert!(ok, "the example runs:\n{err}");
}
