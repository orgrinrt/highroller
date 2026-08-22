//! Each feature configuration either builds, or refuses in words that name the way out.
//!
//! `trybuild` pins a refusal that comes from source, and cannot vary features: it compiles
//! one crate configuration and hands it files. These refusals *are* the configuration, so
//! they are checked by building the crate several ways and reading what came back.
//!
//! Every case here was first run by hand while the features were being written. That is
//! what this file is: the hand checks, kept, so the next person does not repeat them and so
//! a change that quietly removes a refusal is a failing test rather than a discovery.

use std::process::Command;

/// Runs `cargo check` for one configuration and gives back whether it built and its stderr.
fn check(args: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--quiet")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        // A target directory of its own, so these do not fight the outer `cargo test` for
        // the build lock and do not invalidate its artifacts by rebuilding under other
        // features.
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/feature-matrix"),
        )
        .output()
        .expect("cargo runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn the_default_configuration_builds() {
    let (ok, err) = check(&[]);
    assert!(ok, "the default features build:\n{err}");
}

#[test]
fn no_std_builds_at_a_width_that_has_an_atomic() {
    // The crate's own code is `core` only at every width but one, so this is the attribute
    // and nothing else.
    let (ok, err) = check(&["--no-default-features", "--features", "u32_index,no_std"]);
    assert!(ok, "no_std builds at u32:\n{err}");
}

#[test]
fn no_std_and_const_refuse_together_and_say_why() {
    // `const` keeps the value in a `std::sync::OnceLock` so `RUID::new()` can be a
    // `const fn` and take its index on first read. `core` has no shareable equivalent.
    let (ok, err) = check(&["--no-default-features", "--features", "u32_index,no_std,const"]);
    assert!(!ok, "no_std with const cannot build");
    assert!(
        err.contains("OnceLock") && err.contains("default-features = false"),
        "the refusal names what it conflicts with and what to do about it:\n{err}"
    );
}

#[test]
fn no_std_and_a_128_bit_index_refuse_together_and_say_why() {
    // That width alone keeps a `std::sync::Mutex`, because there is no atomic to hold it.
    let (ok, err) = check(&["--no-default-features", "--features", "u128_index,no_std"]);
    assert!(!ok, "no_std with a 128-bit index cannot build");
    assert!(
        err.contains("no atomic") && err.contains("narrower"),
        "the refusal names the reason and the way out:\n{err}"
    );
}

#[test]
fn no_alloc_builds_and_brings_the_lending_contract_with_it() {
    // `no_alloc` does not remove allocation here, because nothing in this crate allocates
    // under any configuration. It declares that, and brings in notko's contract for storage
    // handed the other way, which is what `fill_rolling_idx` is written against.
    let (ok, err) = check(&["--no-default-features", "--features", "u32_index,no_alloc"]);
    assert!(ok, "no_alloc builds:\n{err}");
}

#[test]
fn every_index_width_builds_on_its_own() {
    // The width features are mutually exclusive and each defines the same items, so the
    // manifest's guard is the only thing standing between a wrong pair and a wall of
    // duplicate-definition errors naming internals. Each one alone has to work.
    for width in [
        "u8_index",
        "u16_index",
        "u32_index",
        "u64_index",
        "u128_index",
        "usize_index",
    ] {
        let (ok, err) = check(&["--no-default-features", "--features", width]);
        assert!(ok, "{width} builds on its own:\n{err}");
    }
}

#[test]
fn two_index_widths_at_once_are_refused_by_name() {
    // Without the guard this is a wall of E0428 naming internals, with nothing pointing at
    // the feature flags that caused it.
    let (ok, err) = check(&["--no-default-features", "--features", "u8_index,u32_index"]);
    assert!(!ok, "two widths cannot build");
    assert!(
        err.contains("more than one index-width feature"),
        "the refusal names the actual problem:\n{err}"
    );
}
