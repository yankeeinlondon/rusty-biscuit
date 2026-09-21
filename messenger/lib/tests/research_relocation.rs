//! A research fixture read from a checkout this crate was never compiled in.
//!
//! The seven migrated research suites resolve their fixture root through
//! `biscuit_test_harness::manifest_dir!()`. The harness unit-tests the
//! resolver against a stub lookup, and `archive_path_guard` proves no site
//! spells the compile-time form unguarded. Neither of those runs the macro in
//! a real package against a real fixture tree, which is the one thing a
//! `wsl2-ubuntu` cell does: it extracts a builder's archive, remaps
//! `CARGO_MANIFEST_DIR` onto its own checkout, and the producer's path names
//! nothing.
//!
//! This test is deliberately alone in its binary. It mutates
//! `CARGO_MANIFEST_DIR`, which is process-global; nextest gives every test its
//! own process, and a file with no siblings has nothing to disturb even under
//! `cargo test`'s shared harness. Keeping it out of `research_corpus.rs` is
//! what buys that.

#![cfg(feature = "research")]

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use test_toolkit::EnvGuard;

/// The migrated spelling, verbatim from the seven research suites.
fn lib_dir() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
}

/// One real fixture, read through the same `lib_dir()`-relative join the
/// suites use.
const FIXTURE: &str = "tests/fixtures/research/contract/minimal-valid.md";

#[test]
fn a_research_fixture_resolves_into_the_remapped_checkout() {
    let producer_copy = lib_dir().join(FIXTURE);
    let original = fs::read_to_string(&producer_copy)
        .unwrap_or_else(|e| panic!("read {}: {e}", producer_copy.display()));

    // A consumer checkout at an address the compiler never saw, carrying the
    // same fixture plus a marker the producer's copy does not have. Without
    // the marker a resolver that ignored the run-time value would still read
    // an identical file and the assertion would prove nothing.
    let consumer = TempDir::new().expect("consumer checkout");
    let relocated = consumer.path().join("guest/checkout/messenger/lib");
    let fixture = relocated.join(FIXTURE);
    fs::create_dir_all(fixture.parent().expect("the fixture has a parent"))
        .expect("creating the consumer's fixture directory");
    let marker = "\n# relocated-consumer-copy\n";
    fs::write(&fixture, format!("{original}{marker}")).expect("writing the consumer's fixture");

    let _remap = EnvGuard::set_safe("CARGO_MANIFEST_DIR", &relocated);

    // The macro, expanded in this crate: the run-time value wins over the
    // compile-time one, and the read lands in the consumer's tree.
    assert_eq!(lib_dir(), relocated);
    let read = fs::read_to_string(lib_dir().join(FIXTURE)).expect("reading the relocated fixture");
    assert!(
        read.ends_with(marker),
        "the fixture came from the producer's checkout, not the remapped one"
    );

    // And with the producer's path genuinely gone, which is the consumer's
    // real condition. `manifest_dir` is what the macro calls; supplying the
    // compile-time argument directly is the only way to make it name a
    // directory that does not exist.
    let absent = consumer.path().join("producer/messenger/lib");
    assert!(!absent.exists(), "the producer's checkout must be absent");
    let resolved = biscuit_test_harness::bin_exe::manifest_dir(
        absent.to_str().expect("temporary paths are UTF-8"),
    );
    assert_eq!(resolved, relocated);
    assert!(
        fs::read_to_string(resolved.join(FIXTURE))
            .expect("reading the relocated fixture")
            .ends_with(marker)
    );
}
