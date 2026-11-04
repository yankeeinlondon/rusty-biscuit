//! Filesystem canonicalization in `simplified-schema` goes through
//! `biscuit_file::canonicalize_simplified`, and its home directory comes from
//! `biscuit_file::home_dir`, as in Darkmatter before the schema code moved
//! here: a verbatim Windows spelling (`\\?\C:\…`) would otherwise leak into
//! schema file identities and dependency edges. The guard engine and its
//! self-tests live in `darkmatter/cli/tests/common/path_lookup_guard.rs`.

#[path = "../../../../darkmatter/cli/tests/common/path_lookup_guard.rs"]
mod engine;

use engine::Rule;

#[test]
fn production_source_canonicalizes_and_reads_home_only_through_the_shared_helpers() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/path_lookup_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    engine::assert_guarded(
        "simplified-schema/lib",
        &biscuit_test_harness::manifest_dir!(),
        &[Rule::Canonicalize, Rule::HomeLookup],
        &[],
    );
}
