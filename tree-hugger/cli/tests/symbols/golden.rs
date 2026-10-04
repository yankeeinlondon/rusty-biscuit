//! `hug symbols` output goldens over the 16-language `multi/` fixture tree.
//!
//! The plain golden was captured from the binary before the record-native
//! renderer, so it pins today's visible layout. Regenerate it only for a
//! deliberate output change (set `HUG_UPDATE_GOLDEN=1` and rerun), and record
//! each difference in the change that makes it.

use std::path::Path;

use biscuit_test_harness::manifest_dir;
use tree_hugger::corpus::{redact_snapshot_text, strip_ansi_codes};

use crate::Scratch;

/// Every `multi/` file, keyed by its path inside the tree.
const MULTI_FIXTURES: &[(&str, &str)] = &[
    ("bash.sh", include_str!("../../../lib/tests/fixtures/symbols/multi/bash.sh")),
    ("c.c", include_str!("../../../lib/tests/fixtures/symbols/multi/c.c")),
    ("c_sharp.cs", include_str!("../../../lib/tests/fixtures/symbols/multi/c_sharp.cs")),
    ("cpp.cpp", include_str!("../../../lib/tests/fixtures/symbols/multi/cpp.cpp")),
    ("java.java", include_str!("../../../lib/tests/fixtures/symbols/multi/java.java")),
    ("javascript.js", include_str!("../../../lib/tests/fixtures/symbols/multi/javascript.js")),
    ("lua.lua", include_str!("../../../lib/tests/fixtures/symbols/multi/lua.lua")),
    ("nested/go.go", include_str!("../../../lib/tests/fixtures/symbols/multi/nested/go.go")),
    ("nested/python.py", include_str!("../../../lib/tests/fixtures/symbols/multi/nested/python.py")),
    ("perl.pl", include_str!("../../../lib/tests/fixtures/symbols/multi/perl.pl")),
    ("php.php", include_str!("../../../lib/tests/fixtures/symbols/multi/php.php")),
    ("rust.rs", include_str!("../../../lib/tests/fixtures/symbols/multi/rust.rs")),
    ("scala.scala", include_str!("../../../lib/tests/fixtures/symbols/multi/scala.scala")),
    ("swift.swift", include_str!("../../../lib/tests/fixtures/symbols/multi/swift.swift")),
    ("typescript.ts", include_str!("../../../lib/tests/fixtures/symbols/multi/typescript.ts")),
    ("zsh.zsh", include_str!("../../../lib/tests/fixtures/symbols/multi/zsh.zsh")),
];

const SYMBOLS_PLAIN_GOLDEN: &str = include_str!("../fixtures/golden/symbols.plain");

/// Strips ANSI escapes, redacts scratch and temp paths, and normalizes line
/// endings and in-line whitespace, so the comparison holds on every OS.
fn normalized(text: &str, root: &Path) -> String {
    redact_snapshot_text(&strip_ansi_codes(text), root)
}

#[test]
fn plain_symbols_over_multi_match_the_golden() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let actual = scratch.stdout(&["--plain", "symbols", "--no-cache"]);

    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden/symbols.plain"), &actual)
            .expect("write symbols.plain golden");
        return;
    }

    let root = scratch.repo.path();
    let actual = normalized(&actual, root);
    let expected = normalized(SYMBOLS_PLAIN_GOLDEN, root);
    if actual != expected {
        let first_difference = actual
            .lines()
            .zip(expected.lines())
            .position(|(actual_line, expected_line)| actual_line != expected_line)
            .unwrap_or_else(|| actual.lines().count().min(expected.lines().count()));
        panic!(
            "`hug --plain symbols` over multi/ differs from tests/fixtures/golden/symbols.plain \
             at line {}:\n  expected: {:?}\n  actual:   {:?}\n\nfull output:\n{actual}",
            first_difference + 1,
            expected.lines().nth(first_difference),
            actual.lines().nth(first_difference),
        );
    }
}
