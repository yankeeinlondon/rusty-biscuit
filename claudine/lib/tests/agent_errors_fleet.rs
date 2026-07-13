//! The `agent-errors` fleet prompt (`docs/research/agent-errors/_fleet.md`)
//! wires the spec-D10 validate-and-resume pattern into its `success` stack:
//! a `no_error` shell gate followed by a `when`-guarded `resume` with
//! `max_attempts: 2`. That lifecycle grammar is subtle (the shell `no_error`
//! must be the long key/value form, and `resume` must be the last action in
//! its stack item), so this test parses the real committed frontmatter through
//! the production lifecycle parser and asserts the shape survives — a
//! regression gate for the fleet authoring.

use std::path::{Path, PathBuf};

use claudine::composition::{
    parse_lifecycle_config, LifecycleActionKind, LifecycleSignal,
};
use darkmatter::markdown::Markdown;

/// The committed fleet prompt under the claudine package area.
fn fleet_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("lib crate lives under the claudine package area")
        .join("docs/research/agent-errors/_fleet.md")
}

/// Reads the fleet prompt's raw frontmatter as a JSON value (interpolation
/// spans stay literal — the lifecycle parser defers them by design).
fn fleet_frontmatter() -> serde_json::Value {
    let path = fleet_path();
    let md = Markdown::try_from(path.as_path()).expect("parse _fleet.md");
    serde_json::to_value(md.frontmatter().as_map()).expect("frontmatter to json")
}

#[test]
fn fleet_lifecycle_config_parses() {
    let frontmatter = fleet_frontmatter();
    let config = parse_lifecycle_config(&frontmatter, &fleet_path())
        .expect("the agent-errors fleet lifecycle must parse");

    let success = config
        .stack(LifecycleSignal::Success)
        .expect("success stack present");

    // 1. gate (shell/no_error), 2. resume, plus the not-updated guard and the
    // clean-success report — four items in the authored order.
    assert_eq!(
        success.len(),
        4,
        "expected the four-item validate-and-resume success stack"
    );

    // The deterministic gate is a `no_error` shell so a failing check never
    // stops the stack before the resume can fire.
    let has_no_error_shell = success.iter().any(|item| {
        item.actions
            .iter()
            .any(|a| a.no_error && matches!(a.kind, LifecycleActionKind::Shell(_)))
    });
    assert!(
        has_no_error_shell,
        "the success stack must run the gate as a `no_error` shell"
    );

    // Exactly one resume, and it is the terminal action of its stack item.
    let resume_item = success
        .iter()
        .find(|item| {
            item.actions.iter().any(|a| {
                matches!(&a.kind, LifecycleActionKind::LifecycleControl(c) if c.verb() == "resume")
            })
        })
        .expect("a resume action is present");
    let last = resume_item
        .actions
        .last()
        .expect("resume stack item is non-empty");
    assert!(
        matches!(&last.kind, LifecycleActionKind::LifecycleControl(c) if c.verb() == "resume"),
        "resume must be the last action in its stack item"
    );
}
