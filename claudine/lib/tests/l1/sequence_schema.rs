//! The sequence document schema: `claudine/registration/sequence-document.yaml` (a trigger)
//! and its type library `sequence.yaml`, judged through the same
//! trigger discovery, effective schema, and validation DMLS and
//! `md schema validate` use.
//!
//! Every fixture under `tests/fixtures/sequence-schema/` is named by exactly
//! one test (the `containers-depth4-*` controls by
//! `container_schema_recursion.rs`); `sequence_schema_fixtures_are_all_named`
//! keeps that true. The
//! embedded copy the `claudine` launch verdict judges is covered in
//! `claudine_schemas_embedded.rs`.

use std::collections::BTreeSet;

use claudine::composition::{CompositionError, resolve_composition_source_in_context, resolve_sequence_plan};

use crate::claudine_schemas::{Discovered, SchemaRepo, problems, shipped_bundle};

/// What the schema must decide for one fixture.
enum Expect {
    Valid,
    /// Invalid, with a problem at each `(instance path, text in its message)`.
    Invalid(&'static [(&'static str, &'static str)]),
}

use Expect::{Invalid, Valid};

/// Judges each `(fixture name, text, expectation)` row: the sequence trigger
/// selects it, and the verdict is the expected one.
fn assert_sequence_rows(rows: &[(&str, &str, Expect)]) {
    let repo = SchemaRepo::new(&shipped_bundle!());
    // The review-loop example's own `$schema` requires an eager `spec` file.
    repo.write_document("review-spec.md", "---\ntitle: spec\n---\n");
    let discovered = repo.discover().expect("the shipped bundle loads");
    for (name, text, expect) in rows {
        assert_row(&repo, &discovered, name, text, expect);
    }
}

fn assert_row(repo: &SchemaRepo, discovered: &Discovered, name: &str, text: &str, expect: &Expect) {
    let path = repo.write_document(&format!("{name}.md"), text);
    assert!(
        discovered.selects(&path, include_str!("../../../registration/sequence-document.yaml")),
        "{name}: selects sequence-document.yaml"
    );
    let report = discovered
        .judge(
            &path,
            include_str!("../../../registration/composition.yaml"),
            include_str!("../../../registration/inline-composition.yaml"),
        )
        .expect("judged")
        .report;
    let found = problems(&report);
    match expect {
        Valid => assert!(report.valid, "{name}: {found:?}"),
        Invalid(expected) => {
            assert!(!report.valid, "{name} is invalid");
            for (pointer, needle) in *expected {
                assert!(
                    found.iter().any(|(path, message)| path == pointer && message.contains(needle)),
                    "{name}: expected a problem at `{pointer}` naming `{needle}`, got {found:?}"
                );
            }
            assert_eq!(found.len(), expected.len(), "{name}: no other problem: {found:?}");
        }
    }
}

#[test]
fn every_sequence_example_of_the_container_and_loop_specs_is_valid() {
    assert_sequence_rows(&[
        ("example-containers-kinds", include_str!("../fixtures/sequence-schema/example-containers-kinds.md"), Valid),
        (
            "example-containers-tolerant-group",
            include_str!("../fixtures/sequence-schema/example-containers-tolerant-group.md"),
            Valid,
        ),
        (
            "example-containers-sibling-cancellation",
            include_str!("../fixtures/sequence-schema/example-containers-sibling-cancellation.md"),
            Valid,
        ),
        (
            "example-containers-teardown-migration",
            include_str!("../fixtures/sequence-schema/example-containers-teardown-migration.md"),
            Valid,
        ),
        (
            "example-containers-published-outputs",
            include_str!("../fixtures/sequence-schema/example-containers-published-outputs.md"),
            Valid,
        ),
        (
            "example-loops-looping-prompt-step",
            include_str!("../fixtures/sequence-schema/example-loops-looping-prompt-step.md"),
            Valid,
        ),
        ("example-loops-task-guard", include_str!("../fixtures/sequence-schema/example-loops-task-guard.md"), Valid),
        ("example-loops-review-loop", include_str!("../fixtures/sequence-schema/example-loops-review-loop.md"), Valid),
    ]);
}

/// Every container level accepts a well-formed task and group and rejects a
/// malformed one at its own path: the container types recurse, so the fourth
/// and fifth levels are judged like the first.
#[test]
fn every_container_level_is_typed() {
    assert_sequence_rows(&[
        ("level1-valid", include_str!("../fixtures/sequence-schema/level1-valid.md"), Valid),
        (
            "level1-invalid",
            include_str!("../fixtures/sequence-schema/level1-invalid.md"),
            Invalid(&[("/sequence/0/with", "not of type \"object\"")]),
        ),
        ("level2-valid", include_str!("../fixtures/sequence-schema/level2-valid.md"), Valid),
        (
            "level2-invalid",
            include_str!("../fixtures/sequence-schema/level2-invalid.md"),
            Invalid(&[("/sequence/0/sequence/0/group", "key `execution` has been removed")]),
        ),
        ("level3-valid", include_str!("../fixtures/sequence-schema/level3-valid.md"), Valid),
        (
            "level3-invalid",
            include_str!("../fixtures/sequence-schema/level3-invalid.md"),
            Invalid(&[
                ("/sequence/0/sequence/0/sequence/0/with", "not of type \"object\""),
                ("/sequence/0/sequence/0/sequence/1/group", "key `execution` has been removed"),
            ]),
        ),
        (
            "level4-invalid",
            include_str!("../fixtures/sequence-schema/level4-invalid.md"),
            Invalid(&[
                ("/sequence/0/sequence/0/sequence/0/sequence/0/with", "not of type \"object\""),
                ("/sequence/0/sequence/0/sequence/0/sequence/0/sequence/0/group", "key `execution` has been removed"),
            ]),
        ),
    ]);
}

/// A group's `execution` and the root's `fail_fast` are declared
/// `removed(…)`: each is reported at its owner (the group, the root) with the
/// key named, whatever its value, an explicit `null` included, and the
/// sequence parser refuses both too. A misspelled group key is an unknown
/// key at the group (a group is a union of an object and a file reference,
/// so the problem is the union's).
#[test]
fn removed_and_misspelled_keys_are_reported_by_name() {
    const ROOT_FAIL_FAST_NULL: &str = "---\nfail_fast: null\nsequence:\n    - name: one\n      shell: just one\n---\nBody.\n";
    assert_sequence_rows(&[
        (
            "group-execution",
            include_str!("../fixtures/sequence-schema/group-execution.md"),
            Invalid(&[("/sequence/0/group", "key `execution` has been removed")]),
        ),
        (
            "group-misspelled-max-parallel",
            include_str!("../fixtures/sequence-schema/group-misspelled-max-parallel.md"),
            Invalid(&[("/sequence/0/group", "'max_paralel' was unexpected")]),
        ),
        (
            "root-fail-fast",
            include_str!("../fixtures/sequence-schema/root-fail-fast.md"),
            Invalid(&[("", "key `fail_fast` has been removed")]),
        ),
        ("root-fail-fast-null", ROOT_FAIL_FAST_NULL, Invalid(&[("", "key `fail_fast` has been removed")])),
    ]);

    let repo = SchemaRepo::new(&[]);
    let path = repo.write_document("root-fail-fast.md", include_str!("../fixtures/sequence-schema/root-fail-fast.md"));
    let error = sequence_rejection(&repo, &path).expect("the parser rejects root fail_fast");
    assert!(error.to_string().contains("fail_fast"), "{error}");
    let path = repo.write_document("root-fail-fast-null.md", ROOT_FAIL_FAST_NULL);
    let error = sequence_rejection(&repo, &path).expect("the parser rejects root `fail_fast: null`");
    assert!(error.to_string().contains("fail_fast"), "{error}");
}

/// A task object is open: `params` and any other undeclared key are authored
/// step data, never a schema error.
#[test]
fn undeclared_step_keys_are_step_data() {
    assert_sequence_rows(&[("step-data", include_str!("../fixtures/sequence-schema/step-data.md"), Valid)]);
}

/// Every non-null `sequence` selects the trigger; a mapping then fails the
/// payload. A native number or boolean is selected too and fails it as well:
/// the reference arm is `string(no-coerce)`, so neither becomes reference
/// text, and the sequence parser refuses both on its own. Written text, a
/// quoted `"42"` included, is a reference. A null or absent `sequence`
/// selects nothing.
#[test]
fn sequence_trigger_activation_follows_the_value_shape() {
    assert_sequence_rows(&[
        (
            "sequence-mapping",
            include_str!("../fixtures/sequence-schema/sequence-mapping.md"),
            Invalid(&[("/sequence", "\"tasks\"")]),
        ),
        ("sequence-number", include_str!("../fixtures/sequence-schema/sequence-number.md"), Invalid(&[("/sequence", "42 is not valid")])),
        ("sequence-boolean", "---\nsequence: true\n---\n", Invalid(&[("/sequence", "true is not valid")])),
        ("sequence-reference", "---\nsequence: ./steps.yaml\n---\n", Valid),
        ("sequence-quoted-number", "---\nsequence: \"42\"\n---\n", Valid),
        // An empty list activates and is valid; a list is judged per item,
        // so one malformed item is reported at its own index.
        ("sequence-empty", "---\nsequence: []\n---\n", Valid),
        ("sequence-one-bad-item", "---\nsequence: [a, [b]]\n---\n", Invalid(&[("/sequence/1", "[\"b\"]")])),
        ("sequence-every-bad-item", "---\nsequence: [[a], [b]]\n---\n", Invalid(&[("/sequence/0", "[\"a\"]"), ("/sequence/1", "[\"b\"]")])),
    ]);

    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let number = repo.write_document("sequence-number.md", include_str!("../fixtures/sequence-schema/sequence-number.md"));
    let error = sequence_rejection(&repo, &number).expect("the parser rejects `sequence: 42`");
    assert!(matches!(error, CompositionError::SequenceInvalid(_)), "{error}");
    let boolean = repo.write_document("sequence-boolean.md", "---\nsequence: true\n---\n");
    let error = sequence_rejection(&repo, &boolean).expect("the parser rejects `sequence: true`");
    assert!(matches!(error, CompositionError::SequenceInvalid(_)), "{error}");

    for (name, text) in [
        ("sequence-null", include_str!("../fixtures/sequence-schema/sequence-null.md")),
        ("sequence-absent", include_str!("../fixtures/sequence-schema/sequence-absent.md")),
    ] {
        let path = repo.write_document(&format!("{name}.md"), text);
        assert!(!discovered.selects(&path, include_str!("../../../registration/sequence-document.yaml")), "{name}: not selected");
        let judged = discovered
            .judge(
                &path,
                include_str!("../../../registration/composition.yaml"),
                include_str!("../../../registration/inline-composition.yaml"),
            )
            .expect("judged");
        assert!(!judged.properties.contains("sequence"), "{name}: no sequence declaration applies");
        assert!(judged.report.valid, "{name}: {:?}", problems(&judged.report));
    }
}

/// A task's `prompt`, `group`, and nested `sequence` references refuse a
/// native number or boolean, as the task parser does; the written reference
/// beside each is accepted.
#[test]
fn task_references_refuse_a_scalar_and_accept_reference_text() {
    assert_sequence_rows(&[
        ("task-prompt-reference", "---\nsequence:\n    - name: a\n      prompt: ./p.md\n---\n", Valid),
        ("task-prompt-number", "---\nsequence:\n    - name: a\n      prompt: 42\n---\n", Invalid(&[("/sequence/0/prompt", "42")])),
        ("task-group-reference", "---\nsequence:\n    - name: a\n      group: ./g.yaml\n---\n", Valid),
        ("task-group-boolean", "---\nsequence:\n    - name: a\n      group: true\n---\n", Invalid(&[("/sequence/0/group", "true")])),
        ("task-sequence-reference", "---\nsequence:\n    - name: a\n      sequence: ./s.yaml\n---\n", Valid),
        ("task-sequence-number", "---\nsequence:\n    - name: a\n      sequence: 7\n---\n", Invalid(&[("/sequence/0/sequence", "7")])),
    ]);
}

/// A document selecting both `composition.yaml` and `sequence-document.yaml` still
/// rejects a malformed task `with`: no payload weakens the task type.
#[test]
fn malformed_task_with_fails_when_both_claudine_triggers_apply() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document(
        "both-triggers-malformed-with.md",
        include_str!("../fixtures/sequence-schema/both-triggers-malformed-with.md"),
    );
    assert!(discovered.selects(&path, include_str!("../../../registration/sequence-document.yaml")));
    let judged = discovered
        .judge(
            &path,
            include_str!("../../../registration/composition.yaml"),
            include_str!("../../../registration/inline-composition.yaml"),
        )
        .expect("judged");
    assert!(judged.claudine, "composition.yaml applies too");
    assert_eq!(
        problems(&judged.report).into_iter().map(|(path, _)| path).collect::<BTreeSet<_>>(),
        BTreeSet::from(["/sequence/0/with".to_string()])
    );
}

#[test]
fn sequence_schema_fixtures_are_all_named() {
    let named: BTreeSet<&str> = [
        "example-containers-kinds",
        "example-containers-tolerant-group",
        "example-containers-sibling-cancellation",
        "example-containers-teardown-migration",
        "example-containers-published-outputs",
        "example-loops-looping-prompt-step",
        "example-loops-task-guard",
        "example-loops-review-loop",
        "level1-valid",
        "level1-invalid",
        "level2-valid",
        "level2-invalid",
        "level3-valid",
        "level3-invalid",
        "level4-invalid",
        "containers-depth4-seq-seq",
        "containers-depth4-seq-group",
        "containers-depth4-group-seq",
        "containers-depth4-group-group",
        "group-execution",
        "group-misspelled-max-parallel",
        "root-fail-fast",
        "step-data",
        "sequence-mapping",
        "sequence-number",
        "sequence-null",
        "sequence-absent",
        "both-triggers-malformed-with",
    ]
    .into_iter()
    .collect();
    let fixtures: BTreeSet<String> =
        std::fs::read_dir(biscuit_test_harness::manifest_dir!().join("tests/fixtures/sequence-schema"))
            .expect("fixture directory")
            .map(|entry| entry.expect("entry").path().file_stem().expect("stem").to_string_lossy().into_owned())
            .collect();
    assert_eq!(fixtures.iter().map(String::as_str).collect::<BTreeSet<_>>(), named);
}

/// The sequence parser's rejection of the document at `path`, or `None` when
/// it accepts the document.
fn sequence_rejection(repo: &SchemaRepo, path: &std::path::Path) -> Option<CompositionError> {
    let context = repo.context();
    let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context)
        .expect("the document resolves");
    resolve_sequence_plan(&source, &context).err()
}

/// The declarations a sequence document and an inline container own:
/// `$state` and an inline task's `$schema` are typed with the shared
/// `schema` meta-type, `template` with `object`. The installed registration
/// and the copy compiled into the binary give every row the same verdict.
#[test]
fn sequence_declarations_are_typed_alike_on_disk_and_embedded() {
    const VALID: &str = "---\n$schema:\n    spec: string\n$state:\n    color: string(required)\ntemplate:\n    desc: \"{{ color }}\"\nsequence:\n    - name: outer\n      $schema:\n          spec: string\n      $state: ./state.schema.yaml\n      template:\n          rank: 3\n      sequence:\n          - name: inner\n            shell: just inner\n---\nBody.\n";
    const ROOT_STATE_NUMBER: &str = "---\n$state: 42\nsequence:\n    - one\n---\nBody.\n";
    const TASK_TEMPLATE_NUMBER: &str = "---\nsequence:\n    - name: outer\n      template: 3\n      sequence:\n          - one\n---\nBody.\n";
    let rows: [(&str, &str, Expect); 3] = [
        ("declarations-valid", VALID, Valid),
        ("root-state-number", ROOT_STATE_NUMBER, Invalid(&[("/$state", "")])),
        ("task-template-number", TASK_TEMPLATE_NUMBER, Invalid(&[("/sequence/0/template", "object")])),
    ];

    let shipped = shipped_bundle!();
    let disk = SchemaRepo::new(&shipped);
    let embedded = SchemaRepo::new(&shipped);
    let verdicts = |repo: &SchemaRepo, discovered: &Discovered| -> Vec<(bool, Vec<(String, String)>)> {
        repo.write_document("state.schema.yaml", "kind: schema\n$schema:\n    rank: number\n");
        rows.iter()
            .map(|(name, text, expect)| {
                let path = repo.write_document(&format!("{name}.md"), text);
                let report = discovered
                    .judge(
                        &path,
                        include_str!("../../../registration/composition.yaml"),
                        include_str!("../../../registration/inline-composition.yaml"),
                    )
                    .expect("judged")
                    .report;
                let found = problems(&report);
                match expect {
                    Valid => assert!(report.valid, "{name}: {found:?}"),
                    Invalid(expected) => {
                        assert!(!report.valid, "{name} is invalid");
                        for (pointer, needle) in *expected {
                            assert!(
                                found.iter().any(|(at, message)| at == pointer && message.contains(needle)),
                                "{name}: expected a problem at `{pointer}` naming `{needle}`, got {found:?}"
                            );
                        }
                    }
                }
                (report.valid, found)
            })
            .collect()
    };
    let on_disk = verdicts(&disk, &disk.discover_installed().expect("the installed bundle loads"));
    let in_binary = verdicts(&embedded, &embedded.discover().expect("the compiled bundle loads"));
    assert_eq!(on_disk, in_binary, "installed and compiled verdicts agree");
}
