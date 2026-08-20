//! The guarantees, pinned as things that must not compile.
//!
//! `RUID<Rolled>` claims its value came from the rolling counter. That claim is worth
//! exactly as much as the absence of any other way to build one, and an absence is not
//! something a passing test can demonstrate. These are what hold it.

use std::fs;

/// How many cases this suite expects to find.
///
/// `trybuild` is given a glob, and a glob matching nothing is not an error, so without
/// this the suite would pass having checked no case at all.
const EXPECTED_CASES: usize = 3;

#[test]
fn the_provenance_guarantee_still_holds() {
    let found = fs::read_dir("tests/ui")
        .expect("the compile-fail case directory")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .count();
    assert_eq!(
        found, EXPECTED_CASES,
        "expected {EXPECTED_CASES} compile-fail cases in tests/ui, found {found}. \
         Adding one means raising the constant; losing one means something deleted it."
    );

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
