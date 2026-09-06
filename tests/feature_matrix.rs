//! Each feature configuration either builds, or refuses in words that name the
//! way out.
//!
//! `trybuild` pins a refusal that comes from source, and cannot vary features:
//! it compiles one crate configuration and hands it files. These refusals *are*
//! the configuration, so they are checked by building the crate several ways
//! and reading what came back.
//!
//! Every case here was first run by hand while the features were being written.
//! That is what this file is: the hand checks, kept, so the next person does
//! not repeat them and so a change that quietly removes a refusal is a failing
//! test rather than a discovery.

use std::process::Command;

/// Runs `cargo check` for one configuration and gives back whether it built and
/// its stderr.
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
    // The crate's own code is `core` only at every width but one, so this is the
    // attribute and nothing else.
    let (ok, err) = check(&["--no-default-features", "--features", "u32_index,no_std"]);
    assert!(ok, "no_std builds at u32:\n{err}");
}

#[test]
fn no_std_and_const_build_together() {
    // `const` keeps the value in a cell that fills on first read. Under `std` that
    // is a `OnceLock`; without it, a state byte and a slot, spun on for the
    // length of one atomic add. The two features used to refuse each other, and
    // the refusal named `OnceLock` as the reason, so this is the test that
    // stopped being a refusal.
    let (ok, err) =
        check(&["--no-default-features", "--features", "u32_index,ruid_type,no_std,const"]);
    assert!(ok, "no_std with const builds:\n{err}");
}

#[test]
fn no_std_and_a_128_bit_index_build_together() {
    // That width keeps a lock, because there is no atomic to hold it. Under `std` a
    // `Mutex`, without it a spin lock, and either way the width builds.
    let (ok, err) = check(&["--no-default-features", "--features", "u128_index,no_std"]);
    assert!(ok, "no_std with a 128-bit index builds:\n{err}");
}

/// The suite itself runs under `no_std`, not only the library.
///
/// `check` builds the library, and a library that builds under `no_std` while
/// its own unit tests do not is what this crate had: `cargo test --features
/// no_alloc` failed on `Vec` and `format!` in the test module, and nothing said
/// so because nothing ran it.
#[test]
fn the_unit_tests_run_under_no_std() {
    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "--quiet",
            "--lib",
            "--no-default-features",
            "--features",
            "u32_index,ruid_type,const,no_std,allow_arithmetics",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/feature-matrix"),
        )
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "the unit tests pass under no_std:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn no_alloc_builds_and_brings_the_lending_contract_with_it() {
    // `no_alloc` does not remove allocation here, because nothing in this crate
    // allocates under any configuration. It declares that, and brings in
    // notko's contract for storage handed the other way, which is what
    // `fill_rolling_idx` is written against.
    let (ok, err) = check(&["--no-default-features", "--features", "u32_index,no_alloc"]);
    assert!(ok, "no_alloc builds:\n{err}");
}

#[test]
fn every_index_width_builds_on_its_own() {
    // The width features are mutually exclusive and each defines the same items, so
    // the manifest's guard is the only thing standing between a wrong pair and
    // a wall of duplicate-definition errors naming internals. Each one alone
    // has to work.
    for width in ["u8_index", "u16_index", "u32_index", "u64_index", "u128_index", "usize_index"] {
        let (ok, err) = check(&["--no-default-features", "--features", width]);
        assert!(ok, "{width} builds on its own:\n{err}");
    }
}

#[test]
fn two_index_widths_at_once_are_refused_by_name() {
    // Without the guard this is a wall of E0428 naming internals, with nothing
    // pointing at the feature flags that caused it.
    let (ok, err) = check(&["--no-default-features", "--features", "u8_index,u32_index"]);
    assert!(!ok, "two widths cannot build");
    assert!(
        err.contains("more than one index-width feature"),
        "the refusal names the actual problem:\n{err}"
    );
}

/// Every index width builds, and builds its examples and benches too.
///
/// `check(&[])` above builds the library and nothing else, which is how
/// `examples/taking_ids.rs` came to call `u128::from` on the index type: that
/// compiles at five of the six widths and not at `usize_index`, where `usize`
/// has no `From` into `u128`. Nothing in the suite compiled the example, so
/// nothing said so. `--all-targets` is what says so.
#[test]
fn every_index_width_builds_its_examples_and_benches_too() {
    for width in ["u8_index", "u16_index", "u32_index", "u64_index", "u128_index", "usize_index"] {
        let (ok, err) = check(&["--all-targets", "--no-default-features", "--features", width]);
        assert!(ok, "{width} builds every target:\n{err}");
    }
}

/// The declared minimum builds the default selection.
///
/// `rust-version` is a claim about consumers' compilers, and the repository's
/// own pin is the workspace nightly, so nothing else here ever runs the
/// minimum. Ignored because it needs a toolchain most machines do not have;
/// `cargo test -- --ignored` runs it where `rustup toolchain install 1.70.0`
/// has been done.
///
/// Built as a crate of its own rather than in place: the manifest names notko
/// as an optional git dependency, notko's manifest is edition 2024, and 1.70's
/// cargo refuses to resolve that whichever features are selected. That is a
/// fact about the lock rather than about whether this crate's source compiles
/// at 1.70, and the default selection has no dependencies at all, so a copy of
/// the sources under a manifest without the dependency is the whole of it.
#[test]
#[ignore = "catalogue: needs the 1.70.0 toolchain; run with --ignored"]
fn the_declared_minimum_toolchain_builds_the_default_selection() {
    const MSRV: &str = "1.70.0";

    let installed = Command::new("rustup")
        .args(["toolchain", "list"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains(MSRV))
        .unwrap_or(false);
    assert!(
        installed,
        "the {MSRV} toolchain is not installed, so the `rust-version` claim cannot be checked \
         here. `rustup toolchain install {MSRV}` and run again."
    );

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("target/msrv-crate");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("the msrv crate directory");

    // The manifest, with the dependency section and everything after the features
    // cut away: the features themselves are what the default selection is
    // defined by.
    let manifest = std::fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("the manifest");
    let features = manifest
        .split("\n[features]\n")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a features section");
    let features: String = features
        .lines()
        .filter(|line| !line.contains("dep:notko"))
        .map(|line| format!("{line}\n"))
        .collect();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"msrv_check\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
             rust-version = \"{MSRV}\"\n\n[features]\n{features}\n[workspace]\n"
        ),
    )
    .expect("the msrv manifest");

    for entry in std::fs::read_dir(manifest_dir.join("src")).expect("src") {
        let path = entry.expect("an entry").path();
        std::fs::copy(
            &path,
            root.join("src").join(path.file_name().expect("a name")),
        )
        .expect("copying a module");
    }
    // The crate root includes the readme by path, and the copy has none.
    std::fs::copy(manifest_dir.join("README.md"), root.join("README.md")).expect("the readme");

    let output = Command::new("cargo")
        .args([format!("+{MSRV}"), "check".into()])
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "the default selection does not build under its own declared minimum, {MSRV}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
