//! The `loop:` gate's lifecycle concerns render the loop's ambient values
//! through `claudine compose`, including on the pass that ends the loop.

use std::fs;
use std::path::Path;

use crate::common;
use common::{CliProcessFixture, strip_ansi};

/// The documented loop example, taken from the lifecycle reference so the
/// page and this test cannot drift apart.
const LIFECYCLE_DOC: &str = include_str!("../../../docs/topics/flow-control/lifecycle.md");

/// Install a fake `goose` that appends one line to `$PROVIDER_MARKER` per call
/// and exits 0.
fn install_counting_goose(bin_dir: &Path) {
    #[cfg(unix)]
    common::write_executable(
        &bin_dir.join("goose"),
        "#!/bin/sh\nprintf 'call\\n' >> \"$PROVIDER_MARKER\"\ncat > /dev/null\nexit 0\n",
    );
    // A built binary, not a `.cmd` shim: Rust refuses to pass the multi-line
    // prompt argument to a batch file ("batch file arguments are invalid").
    #[cfg(windows)]
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        bin_dir.join("goose.exe"),
    )
    .unwrap();
}

/// Run `claudine compose --goose` on `document` and return (provider calls,
/// ANSI-stripped stderr).
fn compose_loop(document: &str) -> (usize, String) {
    let fixture = CliProcessFixture::named("loop-gate-ambient");
    let count_path = fixture.cwd().join("call-count.txt");
    let md_file = fixture.cwd().join("loop.md");
    fs::write(&md_file, document).unwrap();
    install_counting_goose(fixture.bin_dir());

    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &count_path)
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "compose failed:\n{stderr}");
    let calls = fs::read_to_string(&count_path)
        .unwrap_or_default()
        .lines()
        .count();
    (calls, stderr)
}

/// Lines of `stderr` that contain `needle`, trimmed of any status glyph.
fn lines_containing<'a>(stderr: &'a str, needle: &str) -> Vec<&'a str> {
    stderr
        .lines()
        .filter_map(|line| line.find(needle).map(|start| line[start..].trim_end()))
        .collect()
}

/// The reported reproduction: a `loop:` block reading `_loop_count` in its
/// own `info` field.
#[test]
fn loop_gate_info_renders_loop_count_on_every_pass() {
    let (calls, stderr) = compose_loop(
        r#"---
counter: 0
loop:
    until: "_loop_count >= 2"
    action: increment(counter)
    info: "gate after iteration {{_loop_count}}"
---
Counter is {{counter}}
"#,
    );

    assert_eq!(calls, 2, "`until: _loop_count >= 2` runs twice\n{stderr}");
    assert_eq!(
        lines_containing(&stderr, "gate after iteration"),
        ["gate after iteration 1", "gate after iteration 2"],
        "the gate renders each finished iteration, including the last\n{stderr}"
    );
}

/// The "Loop lifecycle concerns" example in the lifecycle reference runs as
/// written.
#[test]
fn lifecycle_reference_loop_example_runs_as_written() {
    let section = LIFECYCLE_DOC
        .split_once("### Loop lifecycle concerns")
        .expect("lifecycle.md keeps its loop example section")
        .1;
    let frontmatter = section
        .split_once("```yaml\n")
        .and_then(|(_, rest)| rest.split_once("```"))
        .expect("the section opens with a yaml example")
        .0;
    let document = format!("{frontmatter}Run iteration {{{{iteration}}}}\n");

    let (calls, stderr) = compose_loop(&document);

    assert_eq!(calls, 3, "the reference promises three iterations\n{stderr}");
    assert_eq!(
        lines_containing(&stderr, "finished iteration"),
        [
            "finished iteration 1 of 3",
            "finished iteration 2 of 3",
            "finished iteration 3 of 3",
        ],
        "the gate message renders each finished iteration\n{stderr}"
    );
    assert_eq!(
        lines_containing(&stderr, "loop gate").len(),
        3,
        "the gate's `stderr` fires on every pass\n{stderr}"
    );
}
