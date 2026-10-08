use crate::Scratch;

#[test]
fn ctags_emits_numeric_unsorted_navigation_records_and_preserves_duplicates() {
    let scratch = Scratch::new(&[
        ("dir with space/one.rs", "pub struct Same;\nimpl Same { pub fn same() {} }\npub fn duplicate() {}\n"),
        ("two.rs", "pub fn duplicate() {}\npub fn λambda() {}\n"),
    ]);
    let output = scratch.hug(&["symbols", "--format", "ctags", "--sort", "name"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("!_TAG_FILE_FORMAT\t2\t/extended format/\n!_TAG_FILE_SORTED\t0\t/0=unsorted/\n"));
    assert!(text.contains("duplicate\tdir with space/one.rs\t3;\"\tkind:function"), "{text}");
    assert!(text.contains("duplicate\ttwo.rs\t1;\"\tkind:function"));
    assert!(text.contains("λambda\ttwo.rs\t2;\"\tkind:function"));
    assert!(text.contains("same\tdir with space/one.rs\t2;\"\tkind:method\tclass:Same"));
    assert_eq!(text.lines().filter(|line| line.starts_with("duplicate\t")).count(), 2);
}

#[test]
fn ctags_skips_control_names_with_warning_and_failure_status() {
    let scratch = Scratch::new(&[("src/unsafe\u{9b}file.rs", "pub fn unsafe_name() {}\n"), ("ok.rs", "pub fn safe() {}\n")]);
    let output = scratch.hug(&["functions", "--format", "ctags"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("unsafe_name"));
    assert!(stdout.contains("safe\tok.rs\t1;\"\tkind:function"), "{stdout}");
    assert!(stderr.contains("src/unsafe") && stderr.contains("line 1"), "{stderr:?}");
}

#[test]
fn ctags_headers_survive_empty_selection_and_plain_is_identical() {
    let scratch = Scratch::new(&[("src/lib.rs", "pub fn alpha() {}\n")]);
    let args = ["functions", "--format", "ctags"];
    let normal = scratch.hug(&args);
    let empty = scratch.hug(&["functions", "--format", "ctags", "--name", "missing"]);
    let plain = scratch.hug(&["--plain", "functions", "--format", "ctags"]);
    assert_eq!(normal.stdout, plain.stdout);
    assert!(String::from_utf8(empty.stdout).unwrap().ends_with("!_TAG_FILE_SORTED\t0\t/0=unsorted/\n"));
    let cases: [&[&str]; 2] = [&["--signatures", "short"], &["--comments"]];
    for flag in cases {
        let mut call = vec!["functions", "--format", "ctags"];
        call.extend(flag);
        assert_eq!(scratch.hug(&call).status.code(), Some(2));
    }
    assert_eq!(scratch.hug(&["functions", "--format", "ctags", "--tree"]).status.code(), Some(2));
}
