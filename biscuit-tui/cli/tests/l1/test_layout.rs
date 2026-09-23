//! Layout gate (`2026-09-21-consolidated-test-binaries`, acceptance 12): with
//! `autotests = false`, a `tests/` file that no declared `[[test]]` root
//! reaches is never compiled, so its tests would silently never run.

use test_toolkit::test_layout::{collect_test_sources, layout_violations};

#[test]
fn every_test_source_is_compiled_by_a_declared_target() {
    let crate_root = biscuit_test_harness::manifest_dir!();
    let manifest = std::fs::read_to_string(crate_root.join("Cargo.toml")).expect("read Cargo.toml");
    let files = collect_test_sources(&crate_root).expect("scan tests/");

    // Non-vacuity: the walk read the real suite (the three roots, their
    // fifteen former targets, the shared `common` helpers, and this gate).
    assert!(files.len() >= 22, "scanned only {} test source file(s): {:?}", files.len(), files.keys());
    for expected in [
        "tests/l1/main.rs",
        "tests/l1/test_layout.rs",
        "tests/l1/choose_cli.rs",
        "tests/level2/main.rs",
        "tests/level2/terminal_render.rs",
        "tests/level2/windows_captured_stdout.rs",
        "tests/level3/main.rs",
        "tests/level3/level3_chord_select.rs",
        "tests/common/mod.rs",
        "tests/common/real_terminal/mod.rs",
    ] {
        assert!(files.contains_key(expected), "{expected} was not scanned");
    }

    let violations = layout_violations(&manifest, &files);
    assert!(
        violations.is_empty(),
        "test sources outside every declared test target:\n{}",
        violations.join("\n")
    );
}
