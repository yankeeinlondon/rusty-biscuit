//! Auto-select state classification, unsupported-property lookup helpers, and
//! the shared interrupted-run predicate.

use super::*;
use claudine::composition::{InteractiveShape, TextFormat};
use std::path::PathBuf;

fn step(
    n: usize,
    path: &str,
    props: Vec<MissingProperty>,
) -> SequenceMissingPropertiesStep {
    SequenceMissingPropertiesStep {
        step: n,
        step_name: format!("step-{n}"),
        source_path: PathBuf::from(path),
        missing: props,
        frontmatter_description: None,
        pointer_paths: Vec::new(),
    }
}

fn prop_supported(name: &str) -> MissingProperty {
    MissingProperty {
        name: name.to_string(),
        type_label: Some("string".to_string()),
        description: None,
        interactive_shape: Some(InteractiveShape::Text {
            format: TextFormat::Plain,
            min_len: None,
            max_len: None,
        }),
    }
}

fn prop_unsupported(name: &str, type_label: Option<&str>) -> MissingProperty {
    MissingProperty {
        name: name.to_string(),
        type_label: type_label.map(str::to_string),
        description: None,
        interactive_shape: None,
    }
}

#[test]
fn auto_selectable_states_are_classified() {
    use claudine::composition::AgentResolutionState;
    use claudine::provider::Provider;

    // Only Selected and ListOneInstalled auto-select; every other state is
    // a prompting state that the no-TTY gate must abort on.
    assert!(is_auto_selectable_state(&AgentResolutionState::Selected {
        provider: Provider::Claude
    }));
    assert!(is_auto_selectable_state(
        &AgentResolutionState::ListOneInstalled {
            selected: Provider::Claude,
            not_installed: Vec::new(),
            invalid: Vec::new(),
        }
    ));

    for state in [
        AgentResolutionState::NoAgent,
        AgentResolutionState::SingleInvalid {
            hint: "nope".into(),
        },
        AgentResolutionState::SingleNotInstalled {
            provider: Provider::Gemini,
        },
        AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude, Provider::Codex],
            not_installed: Vec::new(),
            invalid: Vec::new(),
        },
        AgentResolutionState::ZeroInstalledList {
            not_installed: vec![Provider::Gemini],
            invalid: Vec::new(),
        },
    ] {
        assert!(
            !is_auto_selectable_state(&state),
            "prompting state {state:?} must not be auto-selectable"
        );
    }
}

#[test]
fn find_first_unsupported_returns_none_when_all_supported() {
    let failures = vec![step(
        1,
        "/tmp/a.md",
        vec![prop_supported("topic"), prop_supported("tier")],
    )];
    assert!(find_first_unsupported(&failures).is_none());
}

#[test]
fn find_first_unsupported_returns_first_unsupported_property() {
    let failures = vec![
        step(1, "/tmp/a.md", vec![prop_supported("topic")]),
        step(
            2,
            "/tmp/b.md",
            vec![
                prop_supported("tier"),
                prop_unsupported("config", Some("object")),
                prop_unsupported("other", None),
            ],
        ),
    ];
    let (path, name, shape) = find_first_unsupported(&failures).unwrap();
    assert_eq!(path, PathBuf::from("/tmp/b.md"));
    assert_eq!(name, "config");
    assert_eq!(shape, "object");
}

#[test]
fn find_first_unsupported_falls_back_to_unknown_label() {
    let failures = vec![step(
        1,
        "/tmp/a.md",
        vec![prop_unsupported("raw", None)],
    )];
    let (_, _, shape) = find_first_unsupported(&failures).unwrap();
    assert_eq!(shape, "(unknown)");
}

#[test]
fn exit_code_130_alone_marks_a_run_interrupted() {
    let flag = AtomicBool::new(false);
    assert!(run_was_interrupted(SEQUENCE_INTERRUPT_EXIT_CODE, &flag));
}

#[test]
fn an_ordinary_failure_without_the_flag_is_not_interrupted() {
    let flag = AtomicBool::new(false);
    for exit_code in [1, 2, 127] {
        assert!(
            !run_was_interrupted(exit_code, &flag),
            "exit {exit_code} with no interrupt observed is a plain failure"
        );
    }
}

#[test]
fn the_interrupt_flag_covers_exits_no_host_reports_as_130() {
    let flag = AtomicBool::new(true);
    // 0xC000013A: the NTSTATUS a console app terminating on CTRL_BREAK_EVENT
    // exits with. 1: TerminateJobObject's force rung. 137: the Unix second
    // press (128 + SIGKILL). 0: a provider that traps SIGINT and exits clean.
    for exit_code in [0xC000_013A_u32 as i32, 1, 137, 0] {
        assert!(
            run_was_interrupted(exit_code, &flag),
            "exit {exit_code} with the interrupt flag set is an interruption"
        );
    }
}
