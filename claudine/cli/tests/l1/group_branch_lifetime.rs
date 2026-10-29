//! A group leg's private runtime branch, observed through `claudine sequence`:
//! built only when the leg is admitted, from the group-start state, and
//! released before the next leg takes its slot.
//!
//! The counts come from the `test-fixtures` seam `CLAUDINE_TEST_GROUP_BRANCHES`,
//! which appends one JSON line per settled group attempt. Tasks are
//! `side_effect:` writes, so nothing launches a provider and the run is
//! portable.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

/// One settled group attempt's branch counts.
#[derive(Debug, PartialEq)]
struct Attempt {
    group: String,
    legs: u64,
    cap: u64,
    built: u64,
    peak: u64,
    live: u64,
}

struct Run {
    attempts: Vec<Attempt>,
    trace: Vec<String>,
}

/// Run `claudine sequence --goose --yolo seq.md` over `seq`.
fn run(seq: &str) -> Run {
    let fixture = CliProcessFixture::named("group-branch-lifetime");
    let seq_path = fixture.cwd().join("seq.md");
    fs::write(&seq_path, seq).unwrap();
    let ledger = fixture.cwd().join("branches.jsonl");
    let output = fixture
        .command()
        .env("CLAUDINE_TEST_GROUP_BRANCHES", &ledger)
        .args(["sequence", "--goose", "--yolo"])
        .arg(&seq_path)
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "stderr:\n{stderr}");
    let attempts = fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            let count = |key: &str| {
                value[key]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{key} in {line}"))
            };
            Attempt {
                group: value["group"].as_str().unwrap().to_string(),
                legs: count("legs"),
                cap: count("cap"),
                built: count("built"),
                peak: count("peak"),
                live: count("live"),
            }
        })
        .collect();
    let trace = fs::read_to_string(fixture.cwd().join("trace.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    assert!(!fixture.audio_spool().exists());
    Run { attempts, trace }
}

/// Eight legs under `max_parallel: 2` build eight branches, never more than
/// two alive, all released when the group settles; an uncapped group's peak
/// is bounded by its leg count. Every leg still runs, in declaration order
/// of admission.
#[test]
fn capped_group_branches_are_bounded_by_the_cap_and_built_once_per_leg() {
    let legs: String = (0..8)
        .map(|index| format!("        - name: leg-{index}\n          side_effect: {{ append_line: [trace.log, \"leg-{index}\"] }}\n"))
        .collect();
    let seq = format!(
        "---\nsequence:\n  - name: capped\n    group:\n      name: capped\n      max_parallel: 2\n      tasks:\n{legs}  - name: open\n    group:\n      name: open\n      tasks:\n{legs}---\n\nBody.\n"
    );
    let run = run(&seq);
    assert_eq!(
        run.attempts,
        vec![
            Attempt {
                group: "capped".into(),
                legs: 8,
                cap: 2,
                built: 8,
                peak: run.attempts[0].peak,
                live: 0
            },
            Attempt {
                group: "open".into(),
                legs: 8,
                cap: 8,
                built: 8,
                peak: run.attempts[1].peak,
                live: 0
            },
        ],
    );
    assert!(
        (1..=2).contains(&run.attempts[0].peak),
        "{:?}",
        run.attempts[0]
    );
    assert!(
        (1..=8).contains(&run.attempts[1].peak),
        "{:?}",
        run.attempts[1]
    );
    let mut trace = run.trace.clone();
    trace.sort();
    let mut expected: Vec<String> = (0..8)
        .flat_map(|index| [format!("leg-{index}"), format!("leg-{index}")])
        .collect();
    expected.sort();
    assert_eq!(trace, expected, "every leg of both groups ran once");
}

/// A leg admitted after its sibling settled reads the group-start value, not
/// the sibling's write; the write reaches the next step once the group folds.
/// A nested group's legs build their own branches inside the outer leg's.
#[test]
fn a_delayed_leg_reads_the_group_start_state_and_nested_legs_build_their_own() {
    let run = run(r#"---
marker: initial
sequence:
  - name: legs
    group:
      name: outer
      max_parallel: 1
      tasks:
        - name: writer
          side_effect: { set: { marker: written } }
        - name: reader
          sequence:
            - name: read
              side_effect: { append_line: [trace.log, "reader={{ marker }}"] }
        - name: nested
          group:
            name: inner
            max_parallel: 1
            tasks:
              - name: one
                side_effect: { append_line: [trace.log, "inner={{ marker }}"] }
              - name: two
                side_effect: { append_line: [trace.log, "inner={{ marker }}"] }
  - name: after
    side_effect: { append_line: [trace.log, "after={{ marker }}"] }
---

Body.
"#);
    assert_eq!(
        run.trace,
        vec![
            "reader=initial",
            "inner=initial",
            "inner=initial",
            "after=written"
        ]
    );
    assert_eq!(
        run.attempts,
        vec![
            Attempt {
                group: "inner".into(),
                legs: 2,
                cap: 1,
                built: 2,
                peak: 1,
                live: 0
            },
            Attempt {
                group: "outer".into(),
                legs: 3,
                cap: 1,
                built: 3,
                peak: 1,
                live: 0
            },
        ],
        "per-group counts: the inner group settles inside the outer group's third leg",
    );
}
