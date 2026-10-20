//! Runtime parity for the `shell` core type (acceptance criteria 7 and 12 of
//! `2026-10-16-core-schema-types`): a closed lifecycle-shaped schema judged
//! through a document's composition agrees with passive validation; union
//! arms beside `shell` follow the ordinary candidate-coercion rule; and a
//! `shell` value is pending only when its command shape was authored, while
//! the same text arriving as data is validated as plain text and never runs.

use std::path::Path;
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ShellExpansionError};
use darkmatter::markdown::schemas::coerce::coerce_frontmatter;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, PendingPolicy, PendingValueReason, PositionMap, ValidationOptions,
    parse_yaml_schema, to_json_schema,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// Records every shell approval request and answers with a fixed decision.
struct Approvals {
    decision: ShellApprovalDecision,
    commands: Mutex<Vec<String>>,
}

impl Approvals {
    fn allowing() -> Arc<Self> {
        Arc::new(Self { decision: ShellApprovalDecision::AllowOnce, commands: Mutex::default() })
    }

    fn denying() -> Arc<Self> {
        Arc::new(Self { decision: ShellApprovalDecision::Deny, commands: Mutex::default() })
    }

    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for Approvals {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.commands.lock().unwrap().push(request.normalized_exact);
        Ok(self.decision.clone())
    }
}

fn options(dir: &Path, approvals: Arc<Approvals>) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(dir.join("doc.md"))
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(approvals)
}

/// Writes `doc.md` into `dir` and composes it under `options`.
fn compose(dir: &Path, content: &str, options: ComposeOptions) -> Result<Markdown, String> {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).expect("write document");
    Markdown::try_from(path.as_path())
        .expect("read document")
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed)
        .map_err(|error| error.to_string())
}

fn value(markdown: &Markdown, key: &str) -> Value {
    markdown.frontmatter().as_map().get(key).cloned().unwrap_or(Value::Null)
}

fn effective_inline(schema: &str) -> EffectiveSchema {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema")
}

const SCHEMA_FAILURE: &str = "did not satisfy the schema";

// ── Lifecycle-shaped schema through a document's composition ────────────

/// Closed inline action objects: `initialize` takes a union without the
/// `shell` action, `start` the same union plus `{ shell: shell(required) }`.
const ACTION_TYPES: &str = "kind: schema\ntypes:\n  \
    say_action: '{ say: string(required) }'\n  \
    set_action: '{ set: object(required) }'\n  \
    shell_action: '{ shell: shell(required) }'\n  \
    init_action: [say_action, set_action]\n  \
    start_action: [say_action, set_action, shell_action]\n  \
    init_actions: init_action[]\n  \
    start_actions: start_action[]\n";

/// Each row is judged twice — by passive validation of the document and by
/// composing it — and the two must agree. The CLI test of the same shape
/// (`md schema validate`) covers the reporting route; this one covers the
/// frontmatter path composition takes, including coercion inside the arm.
#[test]
fn a_lifecycle_shaped_schema_agrees_between_validation_and_composition() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("actions.yaml"), ACTION_TYPES).expect("write action types");
    let header = "---\n$schema:\n  initialize: init_actions@./actions.yaml\n  start: start_actions@./actions.yaml\n";
    let rows: [(&str, &str, bool); 9] = [
        // Control row: allowed actions under `initialize`.
        ("init allowed", "initialize:\n  - say: hi\n  - set: { a: 1 }\n", true),
        ("init shell only", "initialize:\n  - shell: echo hi\n", false),
        ("init mixed", "initialize:\n  - say: hi\n    shell: echo hi\n", false),
        ("init empty action", "initialize:\n  - {}\n", false),
        ("start shell", "start:\n  - shell: echo ok\n", true),
        ("start shell beside say", "start:\n  - say: hi\n  - shell: ls | wc -l\n", true),
        ("start mixed", "start:\n  - say: hi\n    shell: echo hi\n", false),
        ("start empty action", "start:\n  - {}\n", false),
        ("start blank shell", "start:\n  - shell: '  '\n", false),
    ];
    let schemas = DarkmatterSchemas::new(crate::request_support::context_at(dir.path()));
    for (label, frontmatter, expected) in rows {
        let document = format!("{header}{frontmatter}---\nbody\n");
        let path = dir.path().join("doc.md");
        std::fs::write(&path, &document).expect("write document");
        let report = schemas
            .validate(&Markdown::try_from(path.as_path()).expect("read document"))
            .unwrap_or_else(|error| panic!("{label}: schema resolves: {error}"));
        assert_eq!(report.valid, expected, "{label}: validation {:?}", report.problems);

        let composed = compose(dir.path(), &document, options(dir.path(), Approvals::denying()));
        match composed {
            Ok(_) => assert!(expected, "{label}: composition accepted an invalid document"),
            Err(error) => {
                assert!(!expected, "{label}: composition failed: {error}");
                assert!(error.contains(SCHEMA_FAILURE), "{label}: {error}");
            }
        }
    }

    // A boolean command coerces inside the one arm that can accept it.
    let composed = compose(
        dir.path(),
        &format!("{header}start:\n  - shell: true\n---\nbody\n"),
        options(dir.path(), Approvals::denying()),
    )
    .expect("a boolean shell action composes");
    assert_eq!(value(&composed, "start"), json!([{ "shell": "true" }]));
}

// ── Union candidate coercion beside a `shell` arm ────────────────────────

/// A property union commits a coerced candidate only when exactly one arm
/// accepts it. `alphanumeric` never coerces, so the `shell` arm alone turns
/// `3` and `true` into text; a native `number`/`boolean` arm that already
/// accepts the value makes the choice ambiguous and the value stays native.
#[test]
fn union_arms_beside_shell_follow_candidate_coercion() {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        "num_or_cmd: [number, shell]\nid_or_cmd: [alphanumeric, shell]\nflag_or_cmd: [boolean, shell]\n",
    )
    .expect("schema YAML");
    let schema = to_json_schema(&parse_yaml_schema(&yaml).expect("schema parses")).expect("schema lowers");
    let rows: [(&str, Value, Value); 9] = [
        // Control rows: values every arm leaves alone.
        ("num_or_cmd", json!("ls -l"), json!("ls -l")),
        ("id_or_cmd", json!("abc"), json!("abc")),
        ("num_or_cmd", json!(3), json!(3)),
        ("num_or_cmd", json!(true), json!("true")),
        ("num_or_cmd", json!("3"), json!("3")),
        ("id_or_cmd", json!(3), json!("3")),
        ("id_or_cmd", json!(true), json!("true")),
        ("flag_or_cmd", json!(true), json!(true)),
        ("flag_or_cmd", json!(2), json!("2")),
    ];
    for (key, input, expected) in rows {
        let coerced = coerce_frontmatter(&schema, &json!({ key: input.clone() })).value;
        assert_eq!(coerced[key], expected, "{key}: {input}");
    }

    // Composition follows the same rule and still rejects a blank command.
    let dir = TempDir::new().expect("tempdir");
    let header = "---\n$schema:\n  id_or_cmd: [alphanumeric, shell]\n  num_or_cmd: [number, shell]\n";
    let composed = compose(
        dir.path(),
        &format!("{header}id_or_cmd: 3\nnum_or_cmd: 3\n---\nbody\n"),
        options(dir.path(), Approvals::denying()),
    )
    .expect("composes");
    assert_eq!(value(&composed, "id_or_cmd"), json!("3"));
    assert_eq!(value(&composed, "num_or_cmd"), json!(3));
    let error = compose(
        dir.path(),
        &format!("{header}id_or_cmd: '  '\n---\nbody\n"),
        options(dir.path(), Approvals::denying()),
    )
    .expect_err("a blank value fits neither arm");
    assert!(error.contains(SCHEMA_FAILURE), "{error}");
}

// ── Origin-aware pending values ──────────────────────────────────────────

/// Passive validation lists an authored whole-value `$( … )` as pending for
/// `shell` exactly as for `expression`, and runs nothing.
#[test]
fn an_authored_command_shape_is_pending_for_shell_as_for_expression() {
    for declaration in ["shell(required)", "expression(required)"] {
        let schema = effective_inline(&format!("  cmd: '{declaration}'\n"));
        for policy in [PendingPolicy::Defer, PendingPolicy::Report] {
            let report = schema.validate_with_options(
                &json!({ "cmd": "$(echo hi)" }),
                &PositionMap::new(),
                &ValidationOptions { pending_policy: policy, ..ValidationOptions::default() },
            );
            assert!(report.valid, "{declaration} {policy:?}: {:?}", report.problems);
            let pending: Vec<_> = report.pending.iter().map(|p| (p.key.as_str(), p.reason)).collect();
            assert_eq!(pending, [("cmd", PendingValueReason::ShellExpression)], "{declaration} {policy:?}");
        }
        // Control row: a value valid for both types is not pending.
        let report = schema.validate_with_options(
            &json!({ "cmd": "ls" }),
            &PositionMap::new(),
            &ValidationOptions::default(),
        );
        assert!(report.valid && report.pending.is_empty(), "{declaration}: {:?}", report.pending);
    }
}

/// Composition defers an authored `$( … )` on a `shell` property to shell
/// expansion (it asks for approval instead of failing the schema stage),
/// while the same shapes supplied as data — directly or interpolated into an
/// authored `shell` value — are plain text: no approval, exact text, and still
/// judged by the `shell` format.
#[test]
fn only_an_authored_shell_value_runs_and_data_is_validated_as_text() {
    let dir = TempDir::new().expect("tempdir");
    let schema = "---\n$schema:\n  cmd: 'shell(required)'\n  copy: shell\n";

    // Authored: approval is requested; a denial fails the shell stage, not the schema.
    let approvals = Approvals::denying();
    let error = compose(dir.path(), &format!("{schema}cmd: \"$(echo hi)\"\n---\nbody\n"), options(dir.path(), approvals.clone()))
        .expect_err("a denied authored command fails");
    assert!(!error.contains(SCHEMA_FAILURE), "the pending value was validated before it ran: {error}");
    assert_eq!(approvals.commands(), ["echo hi"]);

    // Authored and approved: the command's output replaces the value.
    let approvals = Approvals::allowing();
    let composed = compose(dir.path(), &format!("{schema}cmd: \"$(echo hi)\"\n---\nbody\n"), options(dir.path(), approvals.clone()))
        .expect("an approved command composes");
    assert_eq!(value(&composed, "cmd"), json!("hi"));
    // Data: each shape is exact text, interpolation included, and nothing asks to run.
    for payload in ["$(echo X)", "{{ area }}", "pre $(echo X) {{ area }}"] {
        let approvals = Approvals::denying();
        let composed = compose(
            dir.path(),
            &format!("{schema}area: claudine\ncopy: \"{{{{ cmd }}}}\"\n---\nbody\n"),
            options(dir.path(), approvals.clone()).with_data_overrides(json!({ "cmd": payload })),
        )
        .unwrap_or_else(|error| panic!("{payload}: {error}"));
        assert_eq!(value(&composed, "cmd"), json!(payload), "{payload}");
        assert_eq!(value(&composed, "copy"), json!(payload), "{payload}");
        assert!(approvals.commands().is_empty(), "{payload}: {:?}", approvals.commands());
    }

    // Data is still validated: a blank data command fails the schema.
    let approvals = Approvals::denying();
    let error = compose(
        dir.path(),
        &format!("{schema}---\nbody\n"),
        options(dir.path(), approvals.clone()).with_data_overrides(json!({ "cmd": "  " })),
    )
    .expect_err("blank data fails `shell(required)`");
    assert!(error.contains(SCHEMA_FAILURE), "{error}");
    assert!(approvals.commands().is_empty());
}
