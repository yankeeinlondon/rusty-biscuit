//! The capability rules of the pager, checked against the committed
//! generated clients rather than hand-written endpoint types (R31): each
//! `tests/fixtures/compile_fail/*.rs` case asks a real generated endpoint for
//! a capability its definition does not author and is compared with the
//! compiler diagnostic in its `.stderr` file; the `pass` control uses every
//! authored capability of the same endpoints.
//!
//! `slow_`: trybuild runs Cargo and builds this crate once more in its own
//! target directory.
//!
//! The `.stderr` files are portable on macOS and Linux: trybuild rewrites this
//! crate's path to `$DIR` and each path dependency's to `$<NAME>`, and the
//! compiler wrapper is turned off below. On Windows trybuild cannot rewrite a
//! path dependency (it canonicalizes the dependency to the verbatim
//! `\\?\C:\…` form, which never matches the path rustc prints), so there the
//! cases whose expected output names `$SCHEMATIC_RUNTIME` are left out.
//! Refresh the files with `TRYBUILD=overwrite` on macOS or Linux.

mod common;

use common::EnvGuard;
use serial_test::serial;

#[test]
#[serial]
fn slow_generated_pager_surface_reports_the_intended_diagnostics() {
    // A caching compiler wrapper rewrites source paths in diagnostics, which
    // would make the expected output depend on the host.
    let wrapper = EnvGuard::clear(&["RUSTC_WRAPPER"]);
    wrapper.set("RUSTC_WRAPPER", "");
    let cases = trybuild::TestCases::new();
    cases.pass(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/pass/*.rs"));
    let compile_fail = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/compile_fail");
    if cfg!(windows) {
        let mut portable = 0;
        for entry in std::fs::read_dir(compile_fail).expect("the compile-fail fixtures are readable") {
            let case = entry.expect("a fixture entry").path();
            if case.extension().is_some_and(|extension| extension == "rs") {
                let expected = std::fs::read_to_string(case.with_extension("stderr"))
                    .expect("every compile-fail case has its expected output");
                if !expected.contains("$SCHEMATIC_RUNTIME") {
                    cases.compile_fail(case);
                    portable += 1;
                }
            }
        }
        assert!(portable > 0, "no compile-fail case is comparable on Windows");
    } else {
        cases.compile_fail(format!("{compile_fail}/*.rs"));
    }
}
