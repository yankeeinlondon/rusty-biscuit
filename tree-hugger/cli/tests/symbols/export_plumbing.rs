use crate::Scratch;

#[test]
fn export_dispatch_settles_file_errors_and_empty_selections() {
    let scratch = Scratch::new(&[("a.rs", "pub fn first() {}\n")]);
    crate::reports::add_bad_file(&scratch, "bad.rs");
    let clean = Scratch::new(&[("a.rs", "pub fn first() {}\n")]);
    for format in ["tsv", "markdown", "prose", "ctags"] {
        let output = scratch.hug(&["symbols", "--format", format, "--verbose"]);
        assert_eq!(output.status.code(), Some(1), "{format}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("bad.rs"), "{format}");

        let empty = clean.hug(&["symbols", "--format", format, "--name", "no_such_symbol"]);
        assert_eq!(empty.status.code(), Some(0), "{format}");
    }

    let missing = clean.hug(&["symbols", "--format", "tsv", "missing/**"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).to_lowercase().contains("source"), "{}", String::from_utf8_lossy(&missing.stderr));
}
