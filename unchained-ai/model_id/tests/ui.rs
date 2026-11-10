#[test]
fn ui() {
    // trybuild drives nested `cargo` builds, so a host without Cargo cannot
    // run this suite. Fail rather than pass without compiling anything: the
    // package declares `requires-toolchain`, and CI records an environment
    // without one as a governed gap instead of scheduling this test there.
    let cargo = std::process::Command::new("cargo").arg("--version").output();
    assert!(
        cargo.as_ref().is_ok_and(|output| output.status.success()),
        "the trybuild UI tests need `cargo` on PATH: {cargo:?}"
    );

    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
    t.compile_fail("tests/fail/*.rs");
}
