use super::*;
use crate::composition::{CallerInputLayers, resolve_composition_source};
use serde_json::json;

#[test]
fn supplied_files_separate_resolution_from_schema_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.md");
    std::fs::write(
        &path,
        "---\n\
         $schema:\n\
         \x20 spec: file(required;eager;match(**/*.md))\n\
         \x20 extra: file(eager;match(**/*.md))\n\
         \x20 files: file(eager;match(**/*.md))[]\n\
         \x20 absent: string(required)\n\
         \x20 count: number(required)\n\
         \x20 output: file(required;match(**/*.md))\n\
         \x20 bare: file(eager)\n\
         \x20 ordinary: string\n\
         ---\nbody\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("existing.md"), "existing").unwrap();
    let context = FileResolutionContext::new(dir.path());
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let records = CallerInputLayers::from_caller_overrides(
        Some(json!({
            "spec": "first-partial", "extra": "second-partial",
            "files": ["existing.md", "third-partial", "fourth-partial"],
            "count": "initialize repairs this", "output": "future.md",
            "bare": "missing.md", "ordinary": "path-like/partial"
        })),
        context.clone(),
    )
    .caller_input_records;
    let pending = unresolved_supplied_files(&source, &records, &context.for_source(&path));
    assert_eq!(pending.len(), 4);
    let slots: Vec<_> = pending.iter().filter_map(|pending| pending.array_index).collect();
    assert_eq!(slots, vec![1, 2]);
    assert!(pending.iter().all(|pending| matches!(
        &pending.error, CompositionError::UnresolvedFileReference { property, .. }
        if matches!(property.as_str(), "spec" | "extra" | "files")
    )));
}

#[test]
fn supplied_files_use_caller_origin_and_defer_unavailable_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let caller = dir.path().join("caller");
    let document = dir.path().join("document");
    std::fs::create_dir(&caller).unwrap();
    std::fs::create_dir(&document).unwrap();
    std::fs::write(caller.join("input.md"), "input").unwrap();
    let path = document.join("prompt.md");
    let context = FileResolutionContext::new(&caller);
    let records = CallerInputLayers::from_caller_overrides(
        Some(json!({"spec": "./input.md"})),
        context.clone(),
    )
    .caller_input_records;
    for schema in [
        "{spec: 'file(required;eager;match(**/*.md))'}",
        "./not-created-yet.yaml",
        "{spec: 'unknown-type'}",
    ] {
        std::fs::write(&path, format!("---\n$schema: {schema}\n---\nbody\n")).unwrap();
        let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
        assert!(
            unresolved_supplied_files(&source, &records, &context.for_source(&path)).is_empty()
        );
    }
}

#[test]
fn supplied_file_failure_matches_existing_partial_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.md");
    std::fs::write(
        &path,
        "---\n$schema:\n  spec: file(required;eager;match(**/*.md))\n---\nbody\n",
    )
    .unwrap();
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let context = FileResolutionContext::new(dir.path());
    let overrides = Some(json!({"spec": "no-matching-file"}));
    let records = CallerInputLayers::from_caller_overrides(
        overrides.clone(),
        context.clone(),
    )
    .caller_input_records;
    let pending = unresolved_supplied_files(&source, &records, &context.for_source(&path));
    let previous =
        super::super::pre_validate_schema(&source, overrides.as_ref(), Some(dir.path()), &crate::test_support::context())
            .unwrap_err();
    let CompositionError::UnresolvedFileReference { reason: old_reason, .. } = previous else {
        panic!("expected the existing partial-file classification");
    };
    let CompositionError::UnresolvedFileReference { ref reason, .. } = pending[0].error else {
        panic!("expected the supplied partial-file classification");
    };
    assert_eq!(reason, &old_reason);
}

#[test]
fn supplied_files_select_shipped_review_router_union_arm() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.md");
    // A frozen copy, intentionally not kept in step with `prompts/review.md`.
    std::fs::write(
        &path,
        include_str!("../../../../tests/fixtures/frozen_prompts/prompts/review.md"),
    )
    .unwrap();
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let context = FileResolutionContext::new(dir.path());
    let records = CallerInputLayers::from_caller_overrides(
        Some(json!({"spec": "fixes/2026-09-10-local"})),
        context.clone(),
    )
    .caller_input_records;
    let pending = unresolved_supplied_files(&source, &records, &context.for_source(&path));
    assert_eq!(pending.len(), 1);
    assert!(matches!(
        &pending[0].error,
        CompositionError::UnresolvedFileReference { property, .. } if property == "spec"
    ));
}

#[test]
fn supplied_files_leave_ambiguous_and_mismatched_union_arms_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.md");
    let context = FileResolutionContext::new(dir.path());
    let records = CallerInputLayers::from_caller_overrides(
        Some(json!({"spec": "partial", "kind": "text", "count": "invalid"})),
        context.clone(),
    )
    .caller_input_records;
    for schema in [
        "[{spec: 'file(required;eager;match(**/*.md))'}, {spec: 'string(required)'}]",
        "[{kind: 'literal(file)', spec: 'file(required;eager;match(**/*.md))'}, {kind: 'literal(text)', spec: 'string(required)'}]",
        // Initialization may repair count, but no root arm applies yet.
        "[{spec: 'file(required;eager;match(**/*.md))', count: 'number(required)'}, {plan: 'file(required;eager;match(**/*.md))'}]",
    ] {
        std::fs::write(&path, format!("---\n$schema: {schema}\n---\nbody\n")).unwrap();
        let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
        assert!(
            unresolved_supplied_files(&source, &records, &context.for_source(&path)).is_empty()
        );
    }
}
/// The two-arm shape from `prompts/clarify.md`, with `doc` templated over
/// both alternatives. `sibling` replaces the `doc` value.
fn union_with_templated_sibling(sibling: &str) -> String {
    format!(
        "---\n\
         $schema:\n\
         \x20 - spec: 'file(required;match(**/*spec*.md);eager)'\n\
         \x20   doc: file\n\
         \x20 - design: 'file(required;match(**/*design*.md))'\n\
         \x20   doc: file\n\
         doc: {sibling}\n\
         initialize:\n\
         \x20 - append_line: [\"events.log\", \"initialize\"]\n\
         ---\nSpec document: {{{{spec}}}}\n"
    )
}

fn pending_for(document: &str, overrides: serde_json::Value) -> Vec<UnresolvedSuppliedFile> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plan.md");
    std::fs::write(&path, document).unwrap();
    std::fs::write(dir.path().join("unrelated.md"), "unrelated").unwrap();
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let context = FileResolutionContext::new(dir.path());
    let records = CallerInputLayers::from_caller_overrides(Some(overrides), context.clone())
        .caller_input_records;
    unresolved_supplied_files(&source, &records, &context.for_source(&path))
}

fn assert_single_spec_partial(pending: &[UnresolvedSuppliedFile], expected: &str) {
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference {
        property,
        provided,
        alternatives,
        is_array,
        ..
    } = &pending[0].error
    else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(property, "spec");
    assert_eq!(provided, expected);
    assert_eq!(alternatives, &alternatives_of(&[&[&["**/*spec*.md"]]]));
    assert!(!is_array);
    assert_eq!(pending[0].array_index, None);
}

#[test]
fn templated_file_sibling_does_not_rule_out_the_applicable_union_arm() {
    for sibling in ["\"{{spec || design}}\"", "'{{ spec }}'", "\"$(echo spec.md)\""] {
        let pending =
            pending_for(&union_with_templated_sibling(sibling), json!({"spec": "everywhere"}));
        assert_single_spec_partial(&pending, "everywhere");
    }
}

#[test]
fn literal_invalid_sibling_still_rules_out_the_union_arm() {
    let document = |count: &str| {
        format!(
            "---\n\
             $schema:\n\
             \x20 - spec: 'file(required;match(**/*spec*.md);eager)'\n\
             \x20   doc: file\n\
             \x20   count: number\n\
             \x20 - design: 'file(required;match(**/*design*.md))'\n\
             \x20   doc: file\n\
             doc: \"{{{{spec || design}}}}\"\n\
             count: {count}\n\
             ---\nbody\n"
        )
    };
    // Control: a valid literal leaves the arm applicable.
    assert_single_spec_partial(
        &pending_for(&document("3"), json!({"spec": "everywhere"})),
        "everywhere",
    );
    // A literal that no composition can repair rules the arm out, even
    // though its `doc` sibling is templated.
    let pending = pending_for(&document("many"), json!({"spec": "everywhere"}));
    assert!(pending.is_empty(), "{pending:?}");
}

#[test]
fn templated_sibling_does_not_select_conflicting_or_string_only_arms() {
    for schema in [
        // Conflicting discriminants: `kind` is literal and matches neither arm.
        "[{kind: 'literal(file)', spec: 'file(required;eager;match(**/*.md))', doc: file}, \
         {kind: 'literal(text)', spec: 'string(required)', doc: file}]",
        // String-only alternative: both arms accept `spec`, so neither is unique.
        "[{spec: 'file(required;eager;match(**/*.md))', doc: file}, \
         {spec: 'string(required)', doc: file}]",
    ] {
        let document = format!(
            "---\n$schema: {schema}\ndoc: \"{{{{spec}}}}\"\n---\nbody\n"
        );
        let pending =
            pending_for(&document, json!({"spec": "partial", "kind": "other"}));
        assert!(pending.is_empty(), "{schema}: {pending:?}");
    }
}

#[test]
fn supplied_files_select_shipped_clarify_spec_arm() {
    let pending = pending_for(
        // A frozen copy, intentionally not kept in step with `prompts/clarify.md`.
        include_str!("../../../../tests/fixtures/frozen_prompts/prompts/clarify.md"),
        json!({"spec": "fix"}),
    );
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert!(matches!(
        &pending[0].error,
        CompositionError::UnresolvedFileReference { property, provided, alternatives, .. }
            if property == "spec" && provided == "fix" && alternatives == &alternatives_of(&[&[&["**/*spec*.md"]]])
    ));
}

/// The D1 shape: two arms discriminated by an optional `kind`, each
/// declaring `spec` as an eager `file(match)` over its own tree. `spec_a`
/// and `spec_b` replace the arms' `spec` declarations.
fn two_tree_union(spec_a: &str, spec_b: &str) -> String {
    format!(
        "---\n\
         $schema:\n\
         \x20 - kind: 'literal(feature)'\n\
         \x20   spec: '{spec_a}'\n\
         \x20 - kind: 'literal(fix)'\n\
         \x20   spec: '{spec_b}'\n\
         ---\nbody\n"
    )
}

const FEATURES_SPEC: &str = "file(required;eager;match(**/features/**/spec.md))";
const FIXES_SPEC: &str = "file(required;eager;match(**/fixes/**/spec.md))";

#[test]
fn undecided_union_offers_every_arms_eager_file_groups_as_alternatives() {
    let pending = pending_for(&two_tree_union(FEATURES_SPEC, FIXES_SPEC), json!({"spec": "cli"}));
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference {
        property,
        provided,
        alternatives,
        is_array,
        reason,
        ..
    } = &pending[0].error
    else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(property, "spec");
    assert_eq!(provided, "cli");
    assert_eq!(
        alternatives,
        &alternatives_of(&[&[&["**/features/**/spec.md"]], &[&["**/fixes/**/spec.md"]]])
    );
    assert!(!is_array);
    assert!(reason.starts_with("no existing file matched reference `cli`"), "{reason}");
}

#[test]
fn undecided_union_keeps_each_arm_as_one_alternative_in_arm_order() {
    let pending = pending_for(
        &two_tree_union(
            "file(required;eager;match(**/fixes/**/spec.md, **/*.md))",
            "file(required;eager;match(**/*.md, **/features/**/spec.md))",
        ),
        json!({"spec": "cli"}),
    );
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference { alternatives, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    // Never one concatenated list: each arm's globs stay together.
    assert_eq!(
        alternatives,
        &alternatives_of(&[
            &[&["**/fixes/**/spec.md", "**/*.md"]],
            &[&["**/*.md", "**/features/**/spec.md"]],
        ])
    );
    // The arms of a previously ambiguous pair now share one existence check.
    let pending = pending_for(
        "---\n$schema: [{spec: 'file(required;eager;match(**/*.md))'}, \
         {spec: 'file(required;eager;match(**/*spec*.md))'}]\n---\nbody\n",
        json!({"spec": "partial"}),
    );
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference { alternatives, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(alternatives, &alternatives_of(&[&[&["**/*.md"]], &[&["**/*spec*.md"]]]));
    // Arms with identical groups are one alternative.
    let pending = pending_for(
        &two_tree_union(FIXES_SPEC, FIXES_SPEC),
        json!({"spec": "partial"}),
    );
    let CompositionError::UnresolvedFileReference { alternatives, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(alternatives, &alternatives_of(&[&[&["**/fixes/**/spec.md"]]]));
}

/// `alternatives` spelled as nested string-slice literals.
fn alternatives_of(alternatives: &[&[&[&str]]]) -> Vec<Vec<Vec<String>>> {
    alternatives
        .iter()
        .map(|groups| {
            groups
                .iter()
                .map(|group| group.iter().map(|pattern| (*pattern).to_string()).collect())
                .collect()
        })
        .collect()
}

#[test]
fn undecided_union_without_a_shared_eager_file_shape_has_no_fallback() {
    for (label, spec_b) in [
        ("string arm", "string(required)"),
        ("non-eager arm", "file(required;match(**/fixes/**/spec.md))"),
        ("bare file arm", "file(required;eager)"),
        ("array arm", "file(required;eager;match(**/fixes/**/spec.md))[]"),
    ] {
        let pending =
            pending_for(&two_tree_union(FEATURES_SPEC, spec_b), json!({"spec": "cli"}));
        assert!(pending.is_empty(), "{label}: {pending:?}");
    }
    // Both arms apply, but the second does not declare `spec` at all, so
    // the verdict would depend on which arm wins.
    let pending = pending_for(
        "---\n$schema: [{spec: 'file(required;eager;match(**/features/**/spec.md))'}, \
         {plan: 'file(eager;match(**/fixes/**/spec.md))'}]\n---\nbody\n",
        json!({"spec": "cli"}),
    );
    assert!(pending.is_empty(), "{pending:?}");
}

#[test]
fn undecided_union_leaves_a_resolvable_literal_alone() {
    // Control: `cli` names no file, so the merged check is live here.
    assert_eq!(
        pending_for(&two_tree_union(FEATURES_SPEC, FIXES_SPEC), json!({"spec": "cli"})).len(),
        1
    );
    // `unrelated.md` exists beside the document (see `pending_for`).
    let pending = pending_for(
        &two_tree_union(FEATURES_SPEC, FIXES_SPEC),
        json!({"spec": "unrelated.md"}),
    );
    assert!(pending.is_empty(), "{pending:?}");
}

/// An existing caller file selects the arm whose contested `spec` glob admits
/// it, and that arm alone drives completion of a sibling partial.
#[test]
fn an_existing_file_selects_the_arm_whose_contested_glob_admits_it() {
    let dir = tempfile::tempdir().unwrap();
    for tree in ["features", "fixes"] {
        std::fs::create_dir_all(dir.path().join(tree).join("x")).unwrap();
        std::fs::write(dir.path().join(tree).join("x/spec.md"), "# Spec\n").unwrap();
    }
    let path = dir.path().join("prompt.md");
    std::fs::write(
        &path,
        "---\n\
         $schema:\n\
         \x20 - spec: 'file(required;eager;match(**/features/**/spec.md))'\n\
         \x20   plan: 'file(required;eager;match(**/features/**/plan.md))'\n\
         \x20 - spec: 'file(required;eager;match(**/fixes/**/spec.md))'\n\
         \x20   plan: 'file(required;eager;match(**/fixes/**/plan.md))'\n\
         ---\nbody\n",
    )
    .unwrap();
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let context = FileResolutionContext::new(dir.path());
    for (tree, expected) in [
        ("fixes", "**/fixes/**/plan.md"),
        ("features", "**/features/**/plan.md"),
    ] {
        let records = CallerInputLayers::from_caller_overrides(
            Some(json!({"spec": format!("{tree}/x/spec.md"), "plan": "partial"})),
            context.clone(),
        )
        .caller_input_records;
        let pending = unresolved_supplied_files(&source, &records, &context.for_source(&path));
        assert_eq!(pending.len(), 1, "{pending:?}");
        let CompositionError::UnresolvedFileReference { property, alternatives, .. } =
            &pending[0].error
        else {
            panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
        };
        assert_eq!(property, "plan");
        assert_eq!(alternatives, &alternatives_of(&[&[&[expected]]]), "spec in {tree}");
    }
}

/// A caller file selects the arm whose reused `doc` type it satisfies only
/// when it satisfies every `match` group, judged from the caller's origin.
#[test]
fn an_existing_caller_file_selects_an_arm_only_when_every_match_group_admits_it() {
    let dir = tempfile::tempdir().unwrap();
    let caller = dir.path().join("caller");
    let document = dir.path().join("document");
    std::fs::create_dir(&caller).unwrap();
    std::fs::create_dir(&document).unwrap();
    // The files exist only beside the caller: judged from the document they
    // would name nothing and rule no arm out.
    for file in ["guide-intro.md", "notes.md", "guide.txt"] {
        std::fs::write(caller.join(file), "x\n").unwrap();
    }
    std::fs::write(
        document.join("types.yaml"),
        "kind: schema\ntypes:\n  doc: \"file(match('*.md'))\"\n",
    )
    .unwrap();
    let path = document.join("prompt.md");
    std::fs::write(
        &path,
        "---\n\
         $schema:\n\
         \x20 - doc: \"doc(match('guide*'))@./types.yaml\"\n\
         \x20   plan: 'file(required;eager;match(**/one/*.md))'\n\
         \x20 - doc: file\n\
         \x20   plan: 'file(required;eager;match(**/two/*.md))'\n\
         ---\nbody\n",
    )
    .unwrap();
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let origin = FileResolutionContext::new(&caller);
    for (doc, expected) in [
        // Both groups admit it, so both arms still contend (ruling D1).
        ("guide-intro.md", alternatives_of(&[&[&["**/one/*.md"]], &[&["**/two/*.md"]]])),
        // Only the inherited group admits it: the first arm is ruled out.
        ("notes.md", alternatives_of(&[&[&["**/two/*.md"]]])),
        // Only the use-site group admits it.
        ("guide.txt", alternatives_of(&[&[&["**/two/*.md"]]])),
    ] {
        let records = CallerInputLayers::from_caller_overrides(
            Some(json!({"doc": doc, "plan": "partial"})),
            origin.clone(),
        )
        .caller_input_records;
        let pending = unresolved_supplied_files(
            &source,
            &records,
            &FileResolutionContext::new(&document).for_source(&path),
        );
        assert_eq!(pending.len(), 1, "{doc}: {pending:?}");
        let CompositionError::UnresolvedFileReference { property, alternatives, .. } =
            &pending[0].error
        else {
            panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
        };
        assert_eq!(property, "plan");
        assert_eq!(alternatives, &expected, "doc = {doc}");
    }
}

/// The globs `plan` is completed against when the caller supplies an existing
/// `spec` file and a `plan` partial, for a union whose first arm declares
/// `spec` (and its named types) as `first_arm_spec`.
fn plan_alternatives_for_caller_spec(first_arm_spec: &str, spec_file: &str) -> Vec<Vec<Vec<String>>> {
    let dir = tempfile::tempdir().unwrap();
    let caller = dir.path().join("caller");
    let document = dir.path().join("document");
    for folder in ["notes", "other"] {
        std::fs::create_dir_all(caller.join(folder)).unwrap();
        std::fs::write(caller.join(folder).join("x.md"), "x\n").unwrap();
    }
    std::fs::create_dir(&document).unwrap();
    let path = document.join("prompt.md");
    std::fs::write(
        &path,
        format!(
            "---\n\
             $schema:\n\
             {first_arm_spec}\
             \x20   plan: 'file(required;eager;match(**/one/*.md))'\n\
             \x20 - spec: file\n\
             \x20   plan: 'file(required;eager;match(**/two/*.md))'\n\
             ---\nbody\n"
        ),
    )
    .unwrap();
    let source =
        resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let records = CallerInputLayers::from_caller_overrides(
        Some(json!({"spec": spec_file, "plan": "partial"})),
        FileResolutionContext::new(&caller),
    )
    .caller_input_records;
    let pending =
        unresolved_supplied_files(&source, &records, &FileResolutionContext::new(&document).for_source(&path));
    assert_eq!(pending.len(), 1, "{first_arm_spec} / {spec_file}: {pending:?}");
    let CompositionError::UnresolvedFileReference { property, alternatives, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(property, "plan");
    alternatives.clone()
}

/// An existing caller file outside the glob a root-union arm enforces rules
/// that arm out, whether the glob is reached through a recursive reference,
/// an acyclic named file type, or directly. Judged from the caller's origin,
/// a file inside the glob leaves both arms contending (ruling D1).
#[test]
fn an_existing_caller_file_outside_a_glob_reached_through_a_reference_rules_its_arm_out() {
    for (shape, first_arm_spec) in [
        (
            "recursive reference",
            "\x20 - node: \"file(match('./notes/*.md')) | { children: node[] }\"\n\x20   spec: node\n",
        ),
        (
            "acyclic named type",
            "\x20 - note: \"file(match('./notes/*.md'))\"\n\x20   spec: note\n",
        ),
        ("direct file property", "\x20 - spec: \"file(match('./notes/*.md'))\"\n"),
    ] {
        assert_eq!(
            plan_alternatives_for_caller_spec(first_arm_spec, "./notes/x.md"),
            alternatives_of(&[&[&["**/one/*.md"]], &[&["**/two/*.md"]]]),
            "{shape}: a file inside the glob keeps the first arm"
        );
        assert_eq!(
            plan_alternatives_for_caller_spec(first_arm_spec, "./other/x.md"),
            alternatives_of(&[&[&["**/two/*.md"]]]),
            "{shape}: a file outside the glob rules the first arm out"
        );
        assert_eq!(
            plan_alternatives_for_caller_spec(first_arm_spec, "./notes/partial"),
            alternatives_of(&[&[&["**/one/*.md"]], &[&["**/two/*.md"]]]),
            "{shape}: a value naming no file rules no arm out"
        );
    }
}

/// A file alternative written in the arm itself (`file(match(...)) | number`)
/// has an acyclic, suggestion-only glob: an existing caller file outside it
/// rules no arm out, so both arms' candidates remain.
#[test]
fn a_caller_file_outside_a_property_union_alternative_glob_rules_no_arm_out() {
    let first_arm_spec = "\x20 - spec: \"file(match('./notes/*.md')) | number\"\n";
    for spec_file in ["./notes/x.md", "./other/x.md"] {
        assert_eq!(
            plan_alternatives_for_caller_spec(first_arm_spec, spec_file),
            alternatives_of(&[&[&["**/one/*.md"]], &[&["**/two/*.md"]]]),
            "{spec_file}"
        );
    }
}

/// A partial names no file yet, so it rules no arm out: a settled arm keeps
/// its own glob, and an undecided union still merges both (ruling D1).
#[test]
fn a_partial_never_rules_a_contested_arm_out() {
    let settled = format!(
        "{}kind: fix\n---\nbody\n",
        two_tree_union(FEATURES_SPEC, FIXES_SPEC).trim_end_matches("---\nbody\n")
    );
    let pending = pending_for(&settled, json!({"spec": "cli"}));
    let CompositionError::UnresolvedFileReference { alternatives, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(alternatives, &alternatives_of(&[&[&["**/fixes/**/spec.md"]]]));
}

/// The late verdict offers only the settled arm's candidates: a file from the
/// other arm's tree would fail the settled arm's contested glob.
#[test]
fn late_verdict_on_a_settled_union_completes_against_that_arm_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.md");
    let settled = format!(
        "{}kind: fix\n---\nbody\n",
        two_tree_union(FEATURES_SPEC, FIXES_SPEC).trim_end_matches("---\nbody\n")
    );
    std::fs::write(&path, settled).unwrap();
    let source = resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let error = super::super::pre_validate_schema(&source, Some(&json!({"spec": "cli"})), Some(dir.path()), &crate::test_support::context())
    .unwrap_err();
    let CompositionError::UnresolvedFileReference { alternatives, .. } = error else {
        panic!("expected UnresolvedFileReference, got {error:?}");
    };
    assert_eq!(alternatives, alternatives_of(&[&[&["**/fixes/**/spec.md"]]]));
}

/// The early pass reads a caller file's timing and globs through references:
/// a recursive type and a list of a recursive type each complete a partial
/// against their eager file's globs, from the caller's origin, like the
/// direct controls. A lazy file reached the same way is left for canonical
/// preparation, and so is an acyclic union with a file alternative, which is
/// not read as a file (as before recursive types existed).
#[test]
fn caller_files_through_references_are_classified_and_completed() {
    let eager = "file(eager;match(**/*spec*.md))";
    let lazy = "file(match(**/*spec*.md))";
    for (shape, schema, value, pending_slots) in [
        ("direct eager control", format!("\x20 spec: '{eager}'\n"), json!("cli"), Some(vec![None])),
        (
            "recursive eager",
            format!("\x20 node: \"{eager} | {{ children: node[] }}\"\n\x20 spec: node\n"),
            json!("cli"),
            Some(vec![None]),
        ),
        (
            "acyclic named union with an eager file",
            format!("\x20 doc: \"{eager} | {{ title: string }}\"\n\x20 spec: doc\n"),
            json!("cli"),
            None,
        ),
        ("direct eager list control", format!("\x20 spec: '{eager}[]'\n"), json!(["cli"]), Some(vec![Some(0)])),
        (
            "list of recursive eager",
            format!("\x20 node: \"{eager} | {{ children: node[] }}\"\n\x20 spec: node[]\n"),
            json!(["cli"]),
            Some(vec![Some(0)]),
        ),
        ("direct lazy control", format!("\x20 spec: '{lazy}'\n"), json!("cli"), None),
        (
            "recursive lazy",
            format!("\x20 node: \"{lazy} | {{ children: node[] }}\"\n\x20 spec: node\n"),
            json!("cli"),
            None,
        ),
        (
            "recursive eager beside a plain string",
            format!("\x20 node: \"{eager} | string | {{ children: node[] }}\"\n\x20 spec: node\n"),
            json!("cli"),
            None,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let caller = dir.path().join("caller");
        let document = dir.path().join("document");
        std::fs::create_dir_all(&caller).unwrap();
        std::fs::create_dir_all(&document).unwrap();
        let path = document.join("prompt.md");
        std::fs::write(&path, format!("---\n$schema:\n{schema}---\nbody\n")).unwrap();
        let source =
            resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
        let origin = FileResolutionContext::new(&caller);
        let records = CallerInputLayers::from_caller_overrides(Some(json!({ "spec": value })), origin)
            .caller_input_records;
        let pending =
            unresolved_supplied_files(&source, &records, &FileResolutionContext::new(&document).for_source(&path));
        let Some(slots) = pending_slots else {
            assert!(pending.is_empty(), "{shape}: {pending:?}");
            continue;
        };
        assert_eq!(pending.iter().map(|p| p.array_index).collect::<Vec<_>>(), slots, "{shape}");
        let CompositionError::UnresolvedFileReference { alternatives, is_array, reason, .. } = &pending[0].error else {
            panic!("{shape}: expected UnresolvedFileReference, got {:?}", pending[0].error);
        };
        assert_eq!(alternatives, &alternatives_of(&[&[&["**/*spec*.md"]]]), "{shape}");
        assert_eq!(*is_array, value.is_array(), "{shape}");
        assert!(reason.contains(&caller.display().to_string()), "{shape}: resolved from the caller: {reason}");
    }
}

/// A root-union arm whose eager file is reached through a reference still
/// relaxes existence for a caller's partial, so the arm applies alone and the
/// partial completes against its glob; the other arm needs `design`. An
/// acyclic union with an eager file alternative is not read as a file, so it
/// offers no completion, as before recursive types existed.
#[test]
fn a_partial_through_a_reference_keeps_its_root_union_arm_applicable() {
    for spec in [
        "\x20   spec: 'file(required;eager;match(**/*spec*.md))'\n",
        "\x20   node: \"file(eager;match(**/*spec*.md)) | { children: node[] }\"\n\x20   spec: node(required)\n",
    ] {
        let document = format!(
            "---\n\
             $schema:\n\
             \x20 - doc_ref: file\n\
             {spec}\
             \x20 - design: 'file(required;match(**/*design*.md))'\n\
             ---\nbody\n"
        );
        let pending = pending_for(&document, json!({"spec": "everywhere"}));
        assert_single_spec_partial(&pending, "everywhere");
    }
    let acyclic = "---\n\
         $schema:\n\
         \x20 - doc_ref: file\n\
         \x20   doc: \"file(eager;match(**/*spec*.md)) | { title: string }\"\n\
         \x20   spec: doc(required)\n\
         \x20 - design: 'file(required;match(**/*design*.md))'\n\
         ---\nbody\n";
    assert!(pending_for(acyclic, json!({"spec": "everywhere"})).is_empty());
}

/// The late verdict on a settled union completes against that arm's glob when
/// the arm reaches its file through a recursive reference.
#[test]
fn late_verdict_reads_a_settled_arms_file_through_a_reference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.md");
    std::fs::write(
        &path,
        "---\n\
         $schema:\n\
         \x20 - kind: 'literal(feature)'\n\
         \x20   node: \"file(required;eager;match(**/features/**/spec.md)) | { children: node[] }\"\n\
         \x20   spec: node(required)\n\
         \x20 - kind: 'literal(fix)'\n\
         \x20   node: \"file(required;eager;match(**/fixes/**/spec.md)) | { children: node[] }\"\n\
         \x20   spec: node(required)\n\
         kind: fix\n\
         ---\nbody\n",
    )
    .unwrap();
    let source =
        resolve_composition_source(path.to_str().unwrap(), &crate::test_support::snapshot_at(dir.path())).unwrap();
    let error = super::super::pre_validate_schema(
        &source,
        Some(&json!({"spec": "cli"})),
        Some(dir.path()),
        &crate::test_support::context(),
    )
    .unwrap_err();
    let CompositionError::UnresolvedFileReference { alternatives, .. } = error else {
        panic!("expected UnresolvedFileReference, got {error:?}");
    };
    assert_eq!(alternatives, alternatives_of(&[&[&["**/fixes/**/spec.md"]]]));
}
