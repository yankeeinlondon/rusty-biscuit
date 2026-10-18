//! The sequence document schema: `claudine/schemas/sequence.yaml` (a trigger)
//! and its type library `sequence-types.yaml`, judged through the same
//! trigger discovery, effective schema, and validation DMLS and
//! `md schema validate` use.
//!
//! Every fixture under `tests/fixtures/sequence-schema/` is named by exactly
//! one test; `sequence_schema_fixtures_are_all_named` keeps that true. The
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
        discovered.selects(&path, include_str!("../../../schemas/sequence.yaml")),
        "{name}: selects sequence.yaml"
    );
    let report = discovered
        .judge(
            &path,
            include_str!("../../../schemas/claudine.yaml"),
            include_str!("../../../schemas/inline-compose.yaml"),
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

/// Each of the three typed container levels accepts a well-formed task and
/// group and rejects a malformed one at its own path; below the third level
/// the schema accepts any list, and the task parser judges it.
#[test]
fn each_typed_container_level_validates_and_deeper_levels_are_untyped() {
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
            Invalid(&[("/sequence/0/sequence/0/group", "\"execution\"")]),
        ),
        ("level3-valid", include_str!("../fixtures/sequence-schema/level3-valid.md"), Valid),
        (
            "level3-invalid",
            include_str!("../fixtures/sequence-schema/level3-invalid.md"),
            Invalid(&[
                ("/sequence/0/sequence/0/sequence/0/with", "not of type \"object\""),
                ("/sequence/0/sequence/0/sequence/1/group", "\"execution\""),
            ]),
        ),
        ("deeper-than-typed-levels", include_str!("../fixtures/sequence-schema/deeper-than-typed-levels.md"), Valid),
    ]);
}

/// A group object is closed, so its removed `execution` key and a misspelled
/// key are reported at the group, with the key named in the message (a
/// group is a union of an object and a file reference, so the problem is the
/// union's). Root `fail_fast` is declared `null`, so a value is reported at
/// `/fail_fast`; the sequence parser rejects it too.
#[test]
fn removed_and_misspelled_keys_are_reported_by_name() {
    assert_sequence_rows(&[
        (
            "group-execution",
            include_str!("../fixtures/sequence-schema/group-execution.md"),
            Invalid(&[("/sequence/0/group", "\"execution\"")]),
        ),
        (
            "group-misspelled-max-parallel",
            include_str!("../fixtures/sequence-schema/group-misspelled-max-parallel.md"),
            Invalid(&[("/sequence/0/group", "\"max_paralel\"")]),
        ),
        (
            "root-fail-fast",
            include_str!("../fixtures/sequence-schema/root-fail-fast.md"),
            Invalid(&[("/fail_fast", "not of type \"null\"")]),
        ),
    ]);

    let repo = SchemaRepo::new(&[]);
    let path = repo.write_document("root-fail-fast.md", include_str!("../fixtures/sequence-schema/root-fail-fast.md"));
    let error = sequence_rejection(&repo, &path).expect("the parser rejects root fail_fast");
    assert!(error.to_string().contains("fail_fast"), "{error}");
}

/// A task object is open: `params` and any other undeclared key are authored
/// step data, never a schema error.
#[test]
fn undeclared_step_keys_are_step_data() {
    assert_sequence_rows(&[("step-data", include_str!("../fixtures/sequence-schema/step-data.md"), Valid)]);
}

/// Every non-null `sequence` selects the trigger; a mapping then fails the
/// payload. A native number is selected too, but Darkmatter coerces a
/// top-level scalar to the file-reference string arm, so the schema accepts
/// it and the sequence parser is what rejects it. A null or absent
/// `sequence` selects nothing.
#[test]
fn sequence_trigger_activation_follows_the_value_shape() {
    assert_sequence_rows(&[
        (
            "sequence-mapping",
            include_str!("../fixtures/sequence-schema/sequence-mapping.md"),
            Invalid(&[("/sequence", "\"tasks\"")]),
        ),
        ("sequence-number", include_str!("../fixtures/sequence-schema/sequence-number.md"), Valid),
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

    for (name, text) in [
        ("sequence-null", include_str!("../fixtures/sequence-schema/sequence-null.md")),
        ("sequence-absent", include_str!("../fixtures/sequence-schema/sequence-absent.md")),
    ] {
        let path = repo.write_document(&format!("{name}.md"), text);
        assert!(!discovered.selects(&path, include_str!("../../../schemas/sequence.yaml")), "{name}: not selected");
        let judged = discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged");
        assert!(!judged.properties.contains("sequence"), "{name}: no sequence declaration applies");
        assert!(judged.report.valid, "{name}: {:?}", problems(&judged.report));
    }
}

/// A document selecting both `claudine.yaml` and `sequence.yaml` still
/// rejects a malformed task `with`: no payload weakens the task type.
#[test]
fn malformed_task_with_fails_when_both_claudine_triggers_apply() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document(
        "both-triggers-malformed-with.md",
        include_str!("../fixtures/sequence-schema/both-triggers-malformed-with.md"),
    );
    assert!(discovered.selects(&path, include_str!("../../../schemas/sequence.yaml")));
    let judged = discovered
        .judge(
            &path,
            include_str!("../../../schemas/claudine.yaml"),
            include_str!("../../../schemas/inline-compose.yaml"),
        )
        .expect("judged");
    assert!(judged.claudine, "claudine.yaml applies too");
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
        "deeper-than-typed-levels",
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
