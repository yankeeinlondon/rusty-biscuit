//! Caller-owned eager file inputs that need completion before lifecycle work.

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::markdown::compose::CallerInputRecords;
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, EffectiveSchema, PropertyDef, SchemaArm, SchemaShape,
    SimplifiedSchema, SimplifiedType, TypeExpr,
};

use super::{
    CompositionError, ResolvedCompositionSource, build_effective_instance,
    is_composition_independent, load_effective_schema_in_context, value_needs_composition,
};

/// An unresolved supplied file, retaining its slot when the caller supplied an array.
#[derive(Debug)]
pub struct UnresolvedSuppliedFile {
    /// The existing typed completion signal and actionable failure reason.
    pub error: CompositionError,
    /// The original array slot; `None` also covers scalar shorthand for `file[]`.
    pub array_index: Option<usize>,
}

/// Inspect only caller-owned eager file inputs, without judging the schema.
///
/// Missing values, lazy output files, and unsupported schema shapes are left
/// for canonical preparation. An unavailable schema is likewise deferred so
/// `initialize` can create or repair it. Literal references use each caller
/// record's frozen origin; the document context owns only schema discovery.
pub fn unresolved_supplied_files(
    source: &ResolvedCompositionSource,
    records: &CallerInputRecords,
    document_context: &FileResolutionContext,
) -> Vec<UnresolvedSuppliedFile> {
    if records.is_empty() {
        return Vec::new();
    }
    let Ok(Some(effective)) =
        load_effective_schema_in_context(source, None, Some(document_context))
    else {
        return Vec::new();
    };
    let Some(shape) = supplied_file_shape(source, records, document_context, &effective) else {
        return Vec::new();
    };
    let mut pending = Vec::new();
    for (name, record) in records {
        let Some(PropertyDef::Single(atom)) = shape.properties.get(name) else {
            continue;
        };
        if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File))
            || !atom
                .constraints
                .iter()
                .chain(&atom.array_constraints)
                .any(|constraint| matches!(constraint, Constraint::Eager))
        {
            continue;
        }
        let Some(patterns) = atom.constraints.iter().find_map(|constraint| {
            match constraint {
                Constraint::Match(patterns) if !patterns.is_empty() => Some(patterns),
                _ => None,
            }
        }) else {
            continue;
        };
        let values: Vec<_> = match record.raw() {
            serde_json::Value::String(value) => vec![(None, value.as_str())],
            serde_json::Value::Array(values)
                if atom.is_array && values.iter().all(|v| v.is_string()) =>
            {
                values
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        value.as_str().map(|value| (Some(index), value))
                    })
                    .collect()
            }
            _ => continue,
        };
        for (array_index, provided) in values {
            if provided.trim().is_empty()
                || value_needs_composition(Some(&serde_json::Value::String(provided.to_string())))
            {
                continue;
            }
            if !matches!(
                FileReference::new(provided)
                    .and_then(|reference| reference.resolve_in_context(record.origin())),
                Ok(None)
            ) {
                continue;
            }
            pending.push(UnresolvedSuppliedFile {
                error: CompositionError::UnresolvedFileReference {
                    source_path: source.resolved_path.clone(),
                    property: name.clone(),
                    provided: provided.to_string(),
                    patterns: patterns.clone(),
                    is_array: atom.is_array,
                    reason: format!(
                        "no existing file matched reference `{provided}` while resolving from `{}`",
                        record.origin().base_dir().display(),
                    ),
                },
                array_index,
            });
        }
    }
    pending
}

/// A root union needs a unique applicable arm before its file metadata has meaning.
///
/// Two relaxations apply when judging each arm. Existence is relaxed only for
/// caller-owned files, because initialization still owns that verdict. A
/// problem on a value that still needs composition (`{{…}}`/`$(…)`) is ignored,
/// by the same rule the pre-validator uses, because composition may yet make
/// it valid. Nothing else is relaxed: ordinary string alternatives, literal
/// invalid siblings, and conflicting discriminants never select an arm, so they
/// cannot trigger an unsolicited file chooser.
fn supplied_file_shape<'a>(
    source: &ResolvedCompositionSource,
    records: &CallerInputRecords,
    document_context: &FileResolutionContext,
    effective: &'a EffectiveSchema,
) -> Option<&'a SchemaShape> {
    let arms = match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => return Some(shape),
        SimplifiedSchema::Union(arms) => arms,
    };
    let overrides = serde_json::Value::Object(
        records
            .iter()
            .map(|(name, record)| (name.clone(), record.raw().clone()))
            .collect(),
    );
    let instance = build_effective_instance(source, Some(&overrides));
    let mut schema_source = source.markdown.clone();
    schema_source.frontmatter_mut().as_map_mut().shift_remove("$schema");
    let mut selected = None;
    for arm in arms {
        let SchemaArm::Inline(shape) = arm else {
            return None;
        };
        let mut relaxed = shape.clone();
        let mut candidate = instance.clone();
        for (name, definition) in &mut relaxed.properties {
            if !records.contains_key(name) {
                continue;
            }
            let PropertyDef::Single(atom) = definition else {
                continue;
            };
            if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
                continue;
            }
            atom.constraints.retain(|constraint| !matches!(constraint, Constraint::Eager));
            atom.array_constraints.retain(|constraint| !matches!(constraint, Constraint::Eager));
            if atom.is_array && candidate.get(name).is_some_and(serde_json::Value::is_string) {
                candidate[name] = serde_json::Value::Array(vec![candidate[name].take()]);
            }
        }
        let schemas = DarkmatterSchemas::new()
            .with_file_resolution_context(document_context.clone())
            .with_baseline(SimplifiedSchema::Single(relaxed))
            .ok()?;
        let projected = schemas.effective_for(&schema_source).ok()??;
        let report = projected.validate(&candidate);
        if report
            .problems
            .iter()
            .all(|problem| !is_composition_independent(problem, &candidate))
        {
            if selected.is_some() {
                return None;
            }
            selected = Some(shape);
        }
    }
    selected
}

#[cfg(test)]
mod tests {
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
        std::fs::write(&path, include_str!("../../../../../prompts/review.md")).unwrap();
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
            "[{spec: 'file(required;eager;match(**/*.md))'}, {spec: 'file(required;eager;match(**/*spec*.md))'}]",
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
            include_str!("../../../../../prompts/clarify.md"),
            json!({"spec": "fix"}),
        );
        assert_eq!(pending.len(), 1, "{pending:?}");
        assert!(matches!(
            &pending[0].error,
            CompositionError::UnresolvedFileReference { property, provided, patterns, .. }
                if property == "spec" && provided == "fix" && patterns == &vec!["**/*spec*.md".to_string()]
        ));
    }
}
