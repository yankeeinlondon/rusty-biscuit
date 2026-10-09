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

/// Neovim decodes no escape in a tags field, so a PHP namespace separator is
/// written once: `Outer\Inner` is 11 bytes and its owner 19, as recorded.
#[test]
fn ctags_writes_php_qualified_names_and_owners_with_single_backslashes() {
    let scratch = Scratch::new(&[(
        "bank.php",
        "<?php\nnamespace Outer\\Inner;\nclass Account {\n    public function deposit() {}\n}\n",
    )]);
    let output = scratch.hug(&["symbols", "--format", "ctags"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\nOuter\\Inner\tbank.php\t2;\"\tkind:namespace\n"), "{text}");
    assert!(
        text.contains("\ndeposit\tbank.php\t4;\"\tkind:method\tclass:Outer\\Inner\\Account\n"),
        "{text}"
    );
    assert!(!text.contains("\\\\"), "a backslash was doubled:\n{text}");
}

/// On Unix `\` is an ordinary file-name character; the tag names that file,
/// never an invented `back/slash.rs`.
#[cfg(unix)]
#[test]
fn ctags_writes_a_literal_backslash_file_name_as_written() {
    let scratch = Scratch::new(&[("back\\slash.rs", "fn first() {}\nfn target() {}\n")]);
    let output = scratch.hug(&["functions", "--format", "ctags"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\ntarget\tback\\slash.rs\t2;\"\tkind:function\n"), "{text}");
    assert!(!text.contains("back/slash.rs"), "{text}");
}

/// Names a reader takes for pseudo-tags and file names Neovim expands
/// (environment, shell, home or bracket globs) would lose a declaration or
/// open another file, so each is skipped with a warning and exit 1 while
/// the rest of the file list is still written.
#[test]
fn ctags_skips_pseudo_tag_names_and_expanded_file_names_with_warning_and_failure_status() {
    let scratch = Scratch::new(&[
        ("a$x.rs", "fn dollar_name() {}\n"),
        ("a[b].rs", "fn bracket() {}\n"),
        ("tick`x`.rs", "fn backtick() {}\n"),
        ("~home.rs", "fn tilde() {}\n"),
        ("cost$.rs", "fn kept_dollar() {}\n"),
        ("pseudo.sh", "!_TAG_X() { :; }\nkept_shell() { :; }\n"),
    ]);
    let output = scratch.hug(&["functions", "--format", "ctags"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for omitted in ["dollar_name", "bracket", "backtick", "tilde", "!_TAG_X"] {
        assert!(!stdout.lines().any(|tag| tag.starts_with(&format!("{omitted}\t"))), "{omitted}:\n{stdout}");
    }
    assert!(stdout.contains("\nkept_dollar\tcost$.rs\t1;\"\tkind:function\n"), "{stdout}");
    assert!(stdout.contains("\nkept_shell\tpseudo.sh\t2;\"\tkind:function\n"), "{stdout}");
    let mut warnings: Vec<&str> = stderr.lines().collect();
    warnings.sort_unstable();
    assert_eq!(
        warnings,
        [
            "warning: ctags skipped file name Neovim would expand in a$x.rs at line 1",
            "warning: ctags skipped file name Neovim would expand in a[b].rs at line 1",
            "warning: ctags skipped file name Neovim would expand in tick`x`.rs at line 1",
            "warning: ctags skipped file name Neovim would expand in ~home.rs at line 1",
            "warning: ctags skipped pseudo-tag name starting with !_ in pseudo.sh at line 1",
        ],
        "{stderr}"
    );
}
