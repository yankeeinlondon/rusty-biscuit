//! The `loop:` gate's lifecycle concerns render the `loop` global through
//! `claudine compose`, including on the pass that ends the loop, and a read of
//! a retired `_loop_*` name stops the run before the provider launches.

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

/// Run `claudine compose --goose` on `document` and return (success,
/// provider calls, ANSI-stripped stderr).
fn run_compose(document: &str) -> (bool, usize, String) {
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
    let calls = fs::read_to_string(&count_path)
        .unwrap_or_default()
        .lines()
        .count();
    (output.status.success(), calls, stderr)
}

/// [`run_compose`] for a run that must succeed: (provider calls, stderr).
fn compose_loop(document: &str) -> (usize, String) {
    let (success, calls, stderr) = run_compose(document);
    assert!(success, "compose failed:\n{stderr}");
    (calls, stderr)
}

/// Lines of `stderr` that contain `needle`, trimmed of any status glyph.
fn lines_containing<'a>(stderr: &'a str, needle: &str) -> Vec<&'a str> {
    stderr
        .lines()
        .filter_map(|line| line.find(needle).map(|start| line[start..].trim_end()))
        .collect()
}

/// The reported reproduction: a `loop:` block reading `loop.count` in its
/// own `info` field renders the iteration that just finished on every pass.
#[test]
fn loop_gate_info_renders_loop_count_on_every_pass() {
    let (calls, stderr) = compose_loop(
        r#"---
counter: 0
loop:
    until: "loop.count >= 2"
    action: increment(counter)
    info: "gate after iteration {{loop.count}}"
---
Counter is {{counter}}
"#,
    );

    assert_eq!(calls, 2, "`until: loop.count >= 2` runs twice\n{stderr}");
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

/// A read of a retired `_loop_*` name — in the loop condition, a gate
/// concern, or a body span — is removed grammar: the run stops before the
/// provider launches and names the authored place and the replacement.
#[test]
fn a_retired_loop_read_stops_compose_before_launch() {
    for (document, place, removed, replacement) in [
        (
            "---\ncounter: 0\nloop:\n    until: \"_loop_count >= 2\"\n    action: increment(counter)\n---\nCounter\n",
            "loop.until",
            "_loop_count",
            "loop.count",
        ),
        (
            "---\ncounter: 0\nloop:\n    until: \"counter >= 1\"\n    action: increment(counter)\n    gate:\n        - info: \"pass {{ fm['_loop_is_last'] }}\"\n---\nCounter\n",
            "loop.gate[0].info.text",
            "_loop_is_last",
            "loop.is_last",
        ),
        (
            "---\ncounter: 0\nloop:\n    until: \"counter >= 1\"\n    action: increment(counter)\n---\nCounter {{ counter }}\n\nLast: {{ _loop_last_output }}\n",
            "body:9",
            "_loop_last_output",
            "loop.last_output",
        ),
    ] {
        let (success, calls, stderr) = run_compose(document);
        assert!(!success, "a retired read must fail the run:\n{stderr}");
        assert_eq!(calls, 0, "the provider never launches\n{stderr}");
        for needle in [place, removed, replacement] {
            assert!(stderr.contains(needle), "stderr names `{needle}`:\n{stderr}");
        }
    }
}

/// Text that only names a retired spelling is data and runs.
#[test]
fn naming_a_retired_loop_spelling_in_text_is_data() {
    let (calls, stderr) = compose_loop(
        "---\n_loop_notes: kept\ncounter: 0\nloop:\n    until: \"counter >= 1\"\n    action: increment(counter)\n    info: \"_loop_count is now loop.count\"\n---\n`_loop_count` became {{ '_loop_count' }}; notes {{ _loop_notes }}\n",
    );
    assert_eq!(calls, 2, "post-checked `until: counter >= 1` runs twice\n{stderr}");
    assert!(
        stderr.contains("_loop_count is now loop.count"),
        "the gate's literal text renders as written\n{stderr}"
    );
}

/// Every lifecycle event of an iteration reads that iteration's `loop`
/// values, in a guard, through bracket access, and in each branch.
#[test]
fn an_iterations_lifecycle_events_read_its_loop_values() {
    let (calls, stderr) = compose_loop(
        r#"---
counter: 0
loop:
    until: "loop.count >= 2"
    max: 2
    action: increment(counter)
start:
    - when: "loop.is_first"
      then:
          - info: "start first pass {{ loop.count }}"
      else:
          - info: "start pass {{ loop['count'] }} after exit {{ loop.last_exit_code }}"
success:
    - info: "success pass {{ loop.count }} last={{ loop.is_last }}"
finalize:
    - info: "finalize pass {{ loop.count }}"
---
Counter is {{counter}}
"#,
    );

    assert_eq!(calls, 2, "`until: loop.count >= 2` runs twice\n{stderr}");
    let events: Vec<&str> = stderr
        .lines()
        .filter_map(|line| {
            ["start ", "success pass", "finalize pass"]
                .iter()
                .find_map(|needle| line.find(needle).map(|start| line[start..].trim_end()))
        })
        .collect();
    assert_eq!(
        events,
        [
            "start first pass 1",
            "success pass 1 last=false",
            "finalize pass 1",
            "start pass 2 after exit 0",
            "success pass 2 last=true",
            "finalize pass 2",
        ],
        "each event renders its own iteration\n{stderr}"
    );
}

/// A loop iteration's body and compose-time frontmatter values read the
/// `loop` global (bare, dotted, and bracket), while `fm.loop` still reads the
/// `loop:` configuration; the gate after each iteration reads the output that
/// iteration produced.
#[cfg(unix)]
#[test]
fn an_iterations_body_frontmatter_and_gate_read_its_loop_values() {
    let fixture = CliProcessFixture::named("loop-values-compose");
    let launches = common::launch_recorder::install(&fixture, "goose", "printf 'answer %s\\n' \"$n\"");
    let md_file = fixture.cwd().join("loop.md");
    fs::write(
        &md_file,
        r#"---
pass_icon: "{{ loop.is_last ? 'last' : 'more' }}"
pass_label: "pass {{ loop['count'] }}"
loop:
    max: 2
    until: "loop.count >= 2"
    gate:
        - info: "gate {{ loop.count }} saw [{{ loop.last_output }}]"
---
count={{ loop.count }} first={{ loop['is_first'] }} last={{ loop.is_last }} exit={{ loop.last_exit_code }} prev=[{{ loop.last_output }}]
icon={{ pass_icon }} label={{ pass_label }} max={{ fm.loop.max }} object={{ is_object(loop) }}
"#,
    )
    .unwrap();

    let output = fixture
        .command()
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "compose failed:\n{stderr}");

    // Goose receives the composed prompt as an argument.
    let prompts: Vec<String> = common::launch_recorder::launches(&launches)
        .into_iter()
        .map(|argv| argv.join(" "))
        .collect();
    assert_eq!(prompts.len(), 2, "two iterations\n{stderr}");
    assert!(
        prompts[0].contains(
            "count=1 first=true last=false exit=0 prev=[]\nicon=more label=pass 1 max=2 object=true"
        ),
        "iteration 1 prompt:\n{}",
        prompts[0]
    );
    assert!(
        prompts[1].contains(
            "count=2 first=false last=true exit=0 prev=[answer 0]\nicon=last label=pass 2 max=2 object=true"
        ),
        "iteration 2 prompt:\n{}",
        prompts[1]
    );
    assert_eq!(
        lines_containing(&stderr, "gate "),
        ["gate 1 saw [answer 0]", "gate 2 saw [answer 1]"],
        "the gate reads the iteration that just finished\n{stderr}"
    );
}
