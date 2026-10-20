//! `retry` inside a looping container replaces only the failed attempt of
//! the current pass: earlier passes' outputs stay, this pass's outputs and
//! runtime writes return to the pass's entry, and `initialize`, the count,
//! and the cap are untouched. External writes repeat.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi, write_executable};

/// `flaky-step NAME PASS FAILURES` prints `NAME` and `PASS`, after failing
/// pass 2 `FAILURES` times (counted in `NAME.failures`), and records every
/// attempt in `external.log`, which no retry rolls back.
const STEP: &str = "#!/bin/sh\nn=$(cat \"$1.failures\" 2>/dev/null || echo 0)\n\
echo \"$1$2\" >> external.log\n\
if [ \"$2\" = 2 ] && [ \"$n\" -lt \"$3\" ]; then echo $((n + 1)) > \"$1.failures\"; exit 3; fi\n\
echo \"$1$2\"\n";

/// Run `claudine sequence` on `doc`, returning the event log lines, the
/// external log lines, and the process output.
fn run(doc: &str) -> (Vec<String>, Vec<String>, std::process::Output) {
    let fixture = CliProcessFixture::named("loop-retry");
    fs::write(fixture.cwd().join("seq.md"), doc).unwrap();
    write_executable(&fixture.bin_dir().join("flaky-step"), STEP);
    let output = fixture
        .command()
        .args(["sequence", "--claude", "--yolo", "--silent"])
        .arg(fixture.cwd().join("seq.md"))
        .output()
        .unwrap();
    let read = |name: &str| {
        fs::read_to_string(fixture.cwd().join(name))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    (read("events.log"), read("external.log"), output)
}

fn stderr_of(output: &std::process::Output) -> String {
    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

/// A step that fails the first `times` attempts of pass 2.
fn flaky(step: &str, times: usize) -> String {
    format!("flaky-step {step} {{{{ loop.count }}}} {times}")
}

#[test]
fn a_looping_sequence_retry_restores_the_pass_entry_and_keeps_earlier_passes() {
    let doc = format!(
        r#"---
note: none
seen: none
loop:
  until: "loop.count >= 2"
  gate:
    - append_line: ["events.log", "gate {{{{ loop.count }}}} {{{{ seq.outputs }}}}"]
initialize:
  - append_line: ["events.log", "initialize"]
start:
  - append_line: ["events.log", "start {{{{ loop.count }}}} seen={{{{ seen }}}} note={{{{ note }}}} outputs={{{{ seq.outputs }}}}"]
failure:
  - append_line: ["events.log", "failure {{{{ loop.count }}}}"]
  - retry:
      max_attempts: 2
      with: {{ note: retried }}
finalize:
  - append_line: ["events.log", "finalize {{{{ loop.count }}}} {{{{ seq.outputs }}}}"]
sequence:
  - name: a
    shell: "{steady}"
  - name: mark
    side_effect: {{ set: {{ seen: "a{{{{ loop.count }}}}" }} }}
  - name: b
    shell: '{flaky}'
---
"#,
        flaky = flaky("b", 2),
        steady = flaky("a", 0),
    );
    let (events, external, output) = run(&doc);
    assert!(output.status.success(), "stderr:\n{}", stderr_of(&output));
    // `mark` publishes the value its `set` replaced, so the retried pass 2
    // reading `a1` (not the failed attempt's `a2`) shows its writes rolled back.
    let pass1 = r#""a1","{\"seen\":\"none\"}","b1""#;
    let pass2 = r#""a2","{\"seen\":\"a1\"}","b2""#;
    assert_eq!(
        events,
        [
            "initialize".to_string(),
            "start 1 seen=none note=none outputs=[]".to_string(),
            format!("finalize 1 [{pass1}]"),
            format!("gate 1 [{pass1}]"),
            format!("start 2 seen=a1 note=none outputs=[{pass1}]"),
            "failure 2".to_string(),
            format!("start 2 seen=a1 note=retried outputs=[{pass1}]"),
            "failure 2".to_string(),
            format!("start 2 seen=a1 note=retried outputs=[{pass1}]"),
            format!("finalize 2 [{pass1},{pass2}]"),
            format!("gate 2 [{pass1},{pass2}]"),
        ],
        "stderr:\n{}",
        stderr_of(&output)
    );
    // External writes are not rolled back: each attempt of pass 2 repeats it.
    assert_eq!(external, ["a1", "b1", "a2", "b2", "a2", "b2", "a2", "b2"]);
}

#[test]
fn a_looping_group_retry_rebuilds_the_current_pass_leg_results() {
    let doc = format!(
        r#"---
sequence:
  - name: batch
    group:
      loop:
        until: "loop.count >= 2"
        gate:
          - append_line: ["events.log", "gate {{{{ loop.count }}}} {{{{ loop.last_output.left }}}} {{{{ loop.last_output.right }}}}"]
      initialize:
        - append_line: ["events.log", "initialize"]
      start:
        - append_line: ["events.log", "start {{{{ loop.count }}}} {{{{ length(loop.last_output) }}}}"]
      failure:
        - append_line: ["events.log", "failure {{{{ loop.count }}}} {{{{ group.failed }}}}"]
        - retry: 1
      finalize:
        - append_line: ["events.log", "finalize {{{{ loop.count }}}} {{{{ group.succeeded }}}}"]
      tasks:
        - name: left
          shell: "echo L{{{{ loop.count }}}}"
        - name: right
          shell: '{flaky}'
---
"#,
        flaky = flaky("R", 1)
    );
    let (events, _, output) = run(&doc);
    assert!(output.status.success(), "stderr:\n{}", stderr_of(&output));
    assert_eq!(
        events,
        [
            "initialize",
            "start 1 0",
            r#"finalize 1 ["left","right"]"#,
            "gate 1 L1 R1",
            "start 2 2",
            r#"failure 2 ["right"]"#,
            "start 2 2",
            r#"finalize 2 ["left","right"]"#,
            "gate 2 L2 R2",
        ],
        "stderr:\n{}",
        stderr_of(&output)
    );
}

#[test]
fn a_nested_looping_sequence_retry_keeps_its_earlier_passes() {
    let doc = format!(
        r#"---
sequence:
  - name: inner
    loop:
      until: "loop.count >= 2"
    start:
      - append_line: ["events.log", "start {{{{ loop.count }}}} {{{{ seq.outputs }}}}"]
    failure:
      - retry: 1
    finalize:
      - append_line: ["events.log", "finalize {{{{ loop.count }}}} {{{{ seq.outputs }}}}"]
    sequence:
      - name: a
        shell: "echo a{{{{ loop.count }}}}"
      - name: b
        shell: '{flaky}'
finalize:
  - append_line: ["events.log", "root {{{{ seq.outputs }}}}"]
---
"#,
        flaky = flaky("b", 1)
    );
    let (events, _, output) = run(&doc);
    assert!(output.status.success(), "stderr:\n{}", stderr_of(&output));
    assert_eq!(
        events,
        [
            "start 1 []",
            r#"finalize 1 ["a1","b1"]"#,
            r#"start 2 ["a1","b1"]"#,
            r#"start 2 ["a1","b1"]"#,
            r#"finalize 2 ["a1","b1","a2","b2"]"#,
            r#"root [["a1","b1","a2","b2"]]"#,
        ],
        "stderr:\n{}",
        stderr_of(&output)
    );
}
