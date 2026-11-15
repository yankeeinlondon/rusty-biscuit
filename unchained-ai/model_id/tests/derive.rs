//! Compile-pass cases: every `tests/pass/*.rs` file is a module of this test
//! binary, so building the binary compiles each derive through the real
//! compiler and running it checks the generated API's behavior.

macro_rules! compile_pass_cases {
    ($($case:ident),+ $(,)?) => {
        mod pass {
            $(mod $case;)+
        }

        const COMPILE_PASS_CASES: &[&str] = &[$(stringify!($case)),+];
    };
}

compile_pass_cases!(
    aggregator_provider,
    no_bespoke,
    primary_provider,
    with_override
);

#[test]
fn every_compile_pass_fixture_is_compiled() {
    let dir = biscuit_test_harness::manifest_dir!().join("tests/pass");
    let mut cases: Vec<String> = std::fs::read_dir(&dir)
        .expect("tests/pass is readable")
        .map(|entry| entry.expect("a tests/pass entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            path.file_stem()
                .expect("a fixture file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    cases.sort();
    assert_eq!(
        cases, COMPILE_PASS_CASES,
        "every tests/pass fixture must be listed in `compile_pass_cases!`"
    );
}
