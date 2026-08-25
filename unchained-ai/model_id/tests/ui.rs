#[test]
fn ui() {
    if !std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("skipping trybuild UI tests because cargo is unavailable");
        return;
    }

    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
    t.compile_fail("tests/fail/*.rs");
}
