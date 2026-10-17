//! Input robustness matrix for the `break` and `prep` lifecycle directives
//! and the strict `loop.gate` shapes, read from YAML frontmatter through
//! `claudine compose`.
//!
//! Every cell is one textual edit of the control document, whose `start`
//! stack holds a harmless action, so the control row proves each edit is
//! load-bearing. Cells assert the public result only: the exit status, how
//! many prompts the provider received, and what stderr says. A valid
//! directive is accepted by the configuration and then refused when it runs,
//! because loop control and the target runner are not built yet; those cells
//! prove the directive is never silently ignored or read as `stop`.

use std::fs;

use crate::common::{CliProcessFixture, launch_recorder, strip_ansi};

const CONTROL: &str = "---\nstart:\n  - info: \"starting\"\n---\nHello\n";

/// The `start` item every directive cell replaces.
const START_ITEM: &str = "  - info: \"starting\"\n";

/// The observable result of running one cell.
#[derive(Debug, PartialEq)]
enum Expect {
    /// Exit 0 after this many launches.
    Completes(usize),
    /// A configuration error before any launch, naming the document.
    Rejects,
    /// A non-zero exit after this many launches whose stderr reports this
    /// directive as not yet supported at run time.
    Refuses(&'static str, usize),
}

fn edit(from: &str, to: &str) -> String {
    assert!(CONTROL.contains(from), "edit target {from:?} is not in the control fixture");
    CONTROL.replacen(from, to, 1)
}

/// Replace the control's `start` item with `items` (each line already
/// indented as a list item).
fn start(items: &str) -> String {
    edit(START_ITEM, items)
}

fn check(document: &str, expected: Expect) {
    let fixture = CliProcessFixture::named("directive-matrix");
    let source = fixture.cwd().join("directive.md");
    fs::write(&source, document).unwrap();
    fs::write(fixture.cwd().join("helper.md"), "---\nstart:\n  - info: helper\n---\nHelper\n").unwrap();
    let recordings = launch_recorder::install(&fixture, "claude", "printf 'ok\\n'");
    let output = fixture
        .command()
        .args(["compose", "--claude", "--yolo", "--silent"])
        .arg(&source)
        .output()
        .unwrap();
    let launches = launch_recorder::prompts(&recordings);
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    // The status block wraps its prose, so phrases are matched on one line.
    let prose = stderr.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let refused = |directive: &str| {
        prose.contains(&format!("lifecycle `{directive}` at")) && prose.contains("is not yet supported at run time")
    };
    let observed = match (output.status.success(), launches.len()) {
        (true, count) => Expect::Completes(count),
        (false, count) => match ["break", "prep"].into_iter().find(|directive| refused(directive)) {
            Some(directive) => Expect::Refuses(directive, count),
            None if count == 0 => Expect::Rejects,
            None => panic!("failed after {count} launches without a refusal; stderr:\n{stderr}"),
        },
    };
    assert_eq!(observed, expected, "status {:?}; stderr:\n{stderr}", output.status);
    match expected {
        Expect::Rejects | Expect::Refuses(..) => {
            assert!(stderr.contains("directive.md"), "the error names its document:\n{stderr}");
        }
        Expect::Completes(_) => {
            let received: Vec<&str> = launches.iter().map(|prompt| prompt.trim()).collect();
            assert!(received.iter().all(|prompt| *prompt == "Hello"), "{received:?}");
        }
    }
    assert!(!fixture.audio_spool().exists());
}

macro_rules! matrix {
    ($($name:ident: $document:expr => $expected:expr;)*) => {
        $(
            #[test]
            fn $name() {
                check(AsRef::<str>::as_ref(&$document), $expected);
            }
        )*
    };
}

matrix! {
    control_start_action_launches_once: CONTROL => Expect::Completes(1);

    // `break`: valid shapes reach the run-time refusal.
    break_bare_is_refused_at_run_time: start("  - break\n") => Expect::Refuses("break", 0);
    break_null_is_refused_at_run_time: start("  - break: null\n") => Expect::Refuses("break", 0);
    break_empty_list_is_refused_at_run_time: start("  - break: []\n") => Expect::Refuses("break", 0);
    break_reason_is_refused_at_run_time: start("  - break: \"recurring finding\"\n") => Expect::Refuses("break", 0);
    break_empty_reason_is_refused_at_run_time: start("  - break: \"\"\n") => Expect::Refuses("break", 0);
    break_long_form_is_refused_at_run_time: start("  - break: { reason: done, code: 255 }\n") => Expect::Refuses("break", 0);
    break_no_error_is_still_refused: start("  - break: { code: 0, no_error: true }\n") => Expect::Refuses("break", 0);
    break_in_a_branch_is_refused_at_run_time: start("  - when: \"true\"\n    then: [break]\n  - info: after\n") => Expect::Refuses("break", 0);

    // `break`: invalid shapes are configuration errors.
    break_code_256_is_rejected: start("  - break: { code: 256 }\n") => Expect::Rejects;
    break_code_negative_is_rejected: start("  - break: { code: -1 }\n") => Expect::Rejects;
    break_code_fraction_is_rejected: start("  - break: { code: 1.5 }\n") => Expect::Rejects;
    break_code_string_is_rejected: start("  - break: { code: \"2\" }\n") => Expect::Rejects;
    break_code_null_is_rejected: start("  - break: { code: null }\n") => Expect::Rejects;
    break_reason_null_is_rejected: start("  - break: { reason: null }\n") => Expect::Rejects;
    break_reason_list_is_rejected: start("  - break: { reason: [a] }\n") => Expect::Rejects;
    break_reason_map_is_rejected: start("  - break: { reason: { a: 1 } }\n") => Expect::Rejects;
    break_reason_number_is_rejected: start("  - break: 5\n") => Expect::Rejects;
    break_positional_list_is_rejected: start("  - break: [done]\n") => Expect::Rejects;
    break_unknown_argument_is_rejected: start("  - break: { reason: done, exit: 2 }\n") => Expect::Rejects;
    break_duplicate_key_is_rejected: start("  - break:\n      code: 1\n      code: 2\n") => Expect::Rejects;
    break_not_last_is_rejected: start("  - break\n  - info: after\n") => Expect::Rejects;
    break_under_cancelled_is_rejected: edit("start:\n", "cancelled:\n  - break\nstart:\n") => Expect::Rejects;

    // `prep`: valid shapes reach the run-time refusal; items may follow it.
    prep_positional_is_refused_at_run_time: start("  - prep: \"@helper.md\"\n  - info: after\n") => Expect::Refuses("prep", 0);
    prep_long_form_is_refused_at_run_time: start("  - prep: { target: \"@helper.md\", with: { stage: all } }\n") => Expect::Refuses("prep", 0);
    prep_empty_overlay_is_refused_at_run_time: start("  - prep: { target: \"@helper.md\", with: {} }\n") => Expect::Refuses("prep", 0);
    prep_in_initialize_is_refused_at_run_time: edit("start:\n", "initialize:\n  - prep: \"@helper.md\"\nstart:\n") => Expect::Refuses("prep", 0);

    // `prep`: invalid shapes are configuration errors.
    prep_bare_is_rejected: start("  - prep\n") => Expect::Rejects;
    prep_null_is_rejected: start("  - prep: null\n") => Expect::Rejects;
    prep_empty_target_is_rejected: start("  - prep: \"\"\n") => Expect::Rejects;
    prep_number_target_is_rejected: start("  - prep: 5\n") => Expect::Rejects;
    prep_list_target_is_rejected: start("  - prep: [\"@helper.md\"]\n") => Expect::Rejects;
    prep_missing_target_is_rejected: start("  - prep: { with: { stage: all } }\n") => Expect::Rejects;
    prep_empty_long_target_is_rejected: start("  - prep: { target: \"\" }\n") => Expect::Rejects;
    prep_with_null_is_rejected: start("  - prep: { target: \"@helper.md\", with: null }\n") => Expect::Rejects;
    prep_with_string_is_rejected: start("  - prep: { target: \"@helper.md\", with: stage }\n") => Expect::Rejects;
    prep_with_list_is_rejected: start("  - prep: { target: \"@helper.md\", with: [stage] }\n") => Expect::Rejects;
    prep_unknown_argument_is_rejected: start("  - prep: { target: \"@helper.md\", args: 1 }\n") => Expect::Rejects;
    prep_duplicate_key_is_rejected: start("  - prep:\n      target: \"@helper.md\"\n      target: \"@other.md\"\n") => Expect::Rejects;
    prep_under_cancelled_is_rejected: edit("start:\n", "cancelled:\n  - prep: \"@helper.md\"\nstart:\n") => Expect::Rejects;
    prep_and_another_directive_in_one_bundle_is_rejected: start("  - prep: \"@helper.md\"\n    stop: null\n") => Expect::Rejects;

    // The loop gate: `[]` is a no-op, `null` and `{}` are errors, and a loose
    // directive under `loop:` reaches the gate's run-time refusal.
    gate_empty_list_runs_the_loop: edit("start:\n", "loop:\n  until: \"true\"\n  gate: []\nstart:\n") => Expect::Completes(1);
    gate_null_is_rejected: edit("start:\n", "loop:\n  until: \"true\"\n  gate: null\nstart:\n") => Expect::Rejects;
    gate_empty_map_is_rejected: edit("start:\n", "loop:\n  until: \"true\"\n  gate: {}\nstart:\n") => Expect::Rejects;
    loose_loop_break_is_refused_at_run_time: edit("start:\n", "loop:\n  until: \"true\"\n  break: done\nstart:\n") => Expect::Refuses("break", 1);
    gate_prep_is_refused_at_run_time: edit("start:\n", "loop:\n  until: \"true\"\n  gate: [{ prep: \"@helper.md\" }]\nstart:\n") => Expect::Refuses("prep", 1);
}
