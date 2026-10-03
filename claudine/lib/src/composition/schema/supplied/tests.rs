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
    let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
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
        let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
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
    let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
    let context = FileResolutionContext::new(dir.path());
    let overrides = Some(json!({"spec": "no-matching-file"}));
    let records = CallerInputLayers::from_caller_overrides(
        overrides.clone(),
        context.clone(),
    )
    .caller_input_records;
    let pending = unresolved_supplied_files(&source, &records, &context.for_source(&path));
    let previous =
        super::super::pre_validate_schema(&source, overrides.as_ref(), Some(dir.path()))
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
    std::fs::write(&path, include_str!("../../../../../../prompts/review.md")).unwrap();
    let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
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
        let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
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
         \x20 stack:\n\
         \x20   - action: {{append_line: [\"events.log\", \"initialize\"]}}\n\
         ---\nSpec document: {{{{spec}}}}\n"
    )
}

fn pending_for(document: &str, overrides: serde_json::Value) -> Vec<UnresolvedSuppliedFile> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plan.md");
    std::fs::write(&path, document).unwrap();
    std::fs::write(dir.path().join("unrelated.md"), "unrelated").unwrap();
    let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
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
        patterns,
        is_array,
        ..
    } = &pending[0].error
    else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(property, "spec");
    assert_eq!(provided, expected);
    assert_eq!(patterns, &vec!["**/*spec*.md".to_string()]);
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
        include_str!("../../../../../../prompts/clarify.md"),
        json!({"spec": "fix"}),
    );
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert!(matches!(
        &pending[0].error,
        CompositionError::UnresolvedFileReference { property, provided, patterns, .. }
            if property == "spec" && provided == "fix" && patterns == &vec!["**/*spec*.md".to_string()]
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
fn undecided_union_merges_every_arms_eager_file_patterns() {
    let pending = pending_for(&two_tree_union(FEATURES_SPEC, FIXES_SPEC), json!({"spec": "cli"}));
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference {
        property,
        provided,
        patterns,
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
        patterns,
        &vec!["**/features/**/spec.md".to_string(), "**/fixes/**/spec.md".to_string()]
    );
    assert!(!is_array);
    assert!(reason.starts_with("no existing file matched reference `cli`"), "{reason}");
}

#[test]
fn undecided_union_deduplicates_patterns_in_arm_order() {
    let pending = pending_for(
        &two_tree_union(
            "file(required;eager;match(**/fixes/**/spec.md, **/*.md))",
            "file(required;eager;match(**/*.md, **/features/**/spec.md))",
        ),
        json!({"spec": "cli"}),
    );
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference { patterns, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(
        patterns,
        &vec![
            "**/fixes/**/spec.md".to_string(),
            "**/*.md".to_string(),
            "**/features/**/spec.md".to_string(),
        ]
    );
    // The arms of a previously ambiguous pair now share one existence check.
    let pending = pending_for(
        "---\n$schema: [{spec: 'file(required;eager;match(**/*.md))'}, \
         {spec: 'file(required;eager;match(**/*spec*.md))'}]\n---\nbody\n",
        json!({"spec": "partial"}),
    );
    assert_eq!(pending.len(), 1, "{pending:?}");
    let CompositionError::UnresolvedFileReference { patterns, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(patterns, &vec!["**/*.md".to_string(), "**/*spec*.md".to_string()]);
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
    let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
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
        let CompositionError::UnresolvedFileReference { property, patterns, .. } =
            &pending[0].error
        else {
            panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
        };
        assert_eq!(property, "plan");
        assert_eq!(patterns, &vec![expected.to_string()], "spec in {tree}");
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
    let CompositionError::UnresolvedFileReference { patterns, .. } = &pending[0].error else {
        panic!("expected UnresolvedFileReference, got {:?}", pending[0].error);
    };
    assert_eq!(patterns, &vec!["**/fixes/**/spec.md".to_string()]);
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
    let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
    let error = super::super::pre_validate_schema(
        &source,
        Some(&json!({"spec": "cli"})),
        Some(dir.path()),
    )
    .unwrap_err();
    let CompositionError::UnresolvedFileReference { patterns, .. } = error else {
        panic!("expected UnresolvedFileReference, got {error:?}");
    };
    assert_eq!(patterns, vec!["**/fixes/**/spec.md".to_string()]);
}
