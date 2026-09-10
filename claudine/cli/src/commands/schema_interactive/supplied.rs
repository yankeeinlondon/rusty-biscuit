use biscuit_file::{FileReference, FileResolutionContext};
use claudine::composition::schema::supplied::unresolved_supplied_files;
use darkmatter::markdown::compose::{CallerInputRecord, CallerInputRecords};

use super::{
    CompositionError, InteractiveSchemaOptions, ResolvedCompositionSource, ScopeContext,
    choose_provided_file_reference, downgrade_to_schema_validation,
};

/// Resolve supplied eager file partials without collecting gaps or judging the schema.
///
/// Both maps are installed together only after every selection succeeds. Each
/// record retains its caller origin, including across proxy handoffs.
pub(crate) fn resolve_supplied_file_inputs(
    source: &ResolvedCompositionSource,
    set_overrides: &mut Option<serde_json::Value>,
    records: &mut CallerInputRecords,
    document_context: &FileResolutionContext,
    interactive: InteractiveSchemaOptions,
) -> Result<(), CompositionError> {
    resolve_supplied_file_inputs_with(
        source,
        set_overrides,
        records,
        document_context,
        interactive,
        |property, provided, patterns, origin| {
            let scope = ScopeContext {
                cwd: origin.base_dir().to_path_buf(),
                home: origin.home_dir().map(std::path::Path::to_path_buf),
                repo_info: None,
                git_root: origin.repository_root().map(std::path::Path::to_path_buf),
            };
            let path = choose_provided_file_reference(property, provided, patterns, &scope)
                .ok()
                .flatten()?;
            let resolved = FileReference::new(path.to_str()?)
                .ok()?
                .resolve_in_context(origin)
                .ok()??;
            Some(serde_json::Value::String(resolved.display().to_string()))
        },
    )
}

fn resolve_supplied_file_inputs_with(
    source: &ResolvedCompositionSource,
    set_overrides: &mut Option<serde_json::Value>,
    records: &mut CallerInputRecords,
    document_context: &FileResolutionContext,
    interactive: InteractiveSchemaOptions,
    mut select: impl FnMut(&str, &str, &[String], &FileResolutionContext) -> Option<serde_json::Value>,
) -> Result<(), CompositionError> {
    let pending = unresolved_supplied_files(source, records, document_context);
    if pending.is_empty() {
        return Ok(());
    }
    let mut updated_records = records.clone();
    let mut updated_overrides = set_overrides
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    for pending in pending {
        let CompositionError::UnresolvedFileReference {
            ref property,
            ref provided,
            ref patterns,
            is_array,
            ..
        } = pending.error
        else {
            return Err(pending.error);
        };
        if !interactive.allowed() {
            return Err(downgrade_to_schema_validation(pending.error));
        }
        let Some(record) = updated_records.get(property) else {
            return Err(downgrade_to_schema_validation(pending.error));
        };
        let Some(selected) = select(property, provided, patterns, record.origin()) else {
            return Err(downgrade_to_schema_validation(pending.error));
        };
        let value = if let Some(index) = pending.array_index {
            let mut value = record.raw().clone();
            value[index] = selected;
            value
        } else if is_array {
            serde_json::Value::Array(vec![selected])
        } else {
            selected
        };
        let replacement = CallerInputRecord::new(value.clone(), record.origin().clone());
        updated_records.insert(property.clone(), replacement);
        updated_overrides.insert(property.clone(), value);
    }
    *records = updated_records;
    *set_overrides = Some(serde_json::Value::Object(updated_overrides));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::{CallerInputLayers, resolve_composition_source};
    use serde_json::json;

    fn fixture() -> (tempfile::TempDir, ResolvedCompositionSource, FileResolutionContext) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prompt.md");
        std::fs::write(
            &path,
            "---\n\
             $schema:\n\
             \x20 spec: file(required;eager;match(**/*.md))\n\
             \x20 files: file(eager;match(**/*.md))[]\n\
             \x20 shorthand: file(eager;match(**/*.md))[]\n\
             \x20 absent: string(required)\n\
             ---\nbody\n",
        )
        .unwrap();
        let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
        let context = FileResolutionContext::new(dir.path());
        (dir, source, context)
    }

    fn interactive() -> InteractiveSchemaOptions {
        InteractiveSchemaOptions {
            prompt_for_missing: true,
            stdin_is_tty: true,
            stderr_is_tty: true,
            silent: false,
        }
    }

    #[test]
    fn supplied_selections_preserve_array_slots_and_caller_provenance() {
        let (dir, source, context) = fixture();
        let existing = dir.path().join("existing.md");
        std::fs::write(&existing, "existing").unwrap();
        let mut overrides = Some(json!({
            "spec": "spec-partial", "files": [existing, "first-partial", "second-partial"],
            "shorthand": "shorthand-partial", "unrelated": 42,
        }));
        let mut records = CallerInputLayers::from_caller_overrides(
            overrides.clone(),
            context.clone(),
        )
        .caller_input_records;
        let mut calls = Vec::new();
        resolve_supplied_file_inputs_with(
            &source,
            &mut overrides,
            &mut records,
            &context,
            interactive(),
            |_, provided, _, origin| {
                assert_eq!(origin, &context);
                calls.push(provided.to_string());
                let selected = dir.path().join(format!("{provided}-selected.md"));
                std::fs::write(&selected, "selected").unwrap();
                Some(json!(selected))
            },
        )
        .unwrap();
        assert_eq!(calls.len(), 4);
        let result = overrides.as_ref().unwrap();
        assert_eq!(result["files"][0], json!(existing));
        assert_eq!(result["files"].as_array().unwrap().len(), 3);
        assert!(result["files"][1].as_str().unwrap().ends_with("first-partial-selected.md"));
        assert!(result["files"][2].as_str().unwrap().ends_with("second-partial-selected.md"));
        assert_eq!(result["shorthand"].as_array().unwrap().len(), 1);
        assert_eq!(result["unrelated"], json!(42));
        for (name, record) in &records {
            assert_eq!(record.raw(), &result[name]);
            assert_eq!(record.origin(), &context);
        }
        resolve_supplied_file_inputs_with(
            &source,
            &mut overrides,
            &mut records,
            &context,
            interactive(),
            |_, _, _, _| panic!("selected identities must not prompt twice"),
        )
        .unwrap();
    }

    #[test]
    fn supplied_resolution_gates_and_cancellation_leave_both_maps_unchanged() {
        let (_dir, source, context) = fixture();
        let original = Some(json!({"spec": "partial", "shorthand": "other"}));
        let original_records = CallerInputLayers::from_caller_overrides(
            original.clone(),
            context.clone(),
        )
        .caller_input_records;
        let enabled = interactive();
        let gates = [
            InteractiveSchemaOptions { prompt_for_missing: false, ..enabled },
            InteractiveSchemaOptions { stdin_is_tty: false, ..enabled },
            InteractiveSchemaOptions { stderr_is_tty: false, ..enabled },
            InteractiveSchemaOptions { silent: true, ..enabled },
            enabled,
        ];
        for gate in gates {
            let mut overrides = original.clone();
            let mut records = original_records.clone();
            let mut calls = 0;
            let result = resolve_supplied_file_inputs_with(
                &source,
                &mut overrides,
                &mut records,
                &context,
                gate,
                |_, _, _, _| {
                    assert!(gate.allowed());
                    calls += 1;
                    if calls == 1 {
                        Some(json!("selected.md"))
                    } else {
                        None
                    }
                },
            );
            assert!(matches!(result, Err(CompositionError::SchemaValidation { .. })));
            assert_eq!(overrides, original);
            assert_eq!(records, original_records);
        }
    }
}
