//! Schema validation stage for the compose pipeline.
//!
//! This module implements the always-on Schema Validation stage that runs
//! after `--set` / `--state` overrides are applied to frontmatter, but before
//! any interpolation or shell expansion.
//!
//! When the effective frontmatter violates the resolved schema, compose aborts
//! with a styled [`BlockError`] that names the offending property.

use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::types::ComposeSource;
use crate::markdown::schemas::DarkmatterSchemas;
use crate::markdown::types::{MarkdownError, MarkdownResult};

/// Runs schema validation against the document's effective frontmatter.
///
/// 1. Checks whether either document `$schema` or `ComposeOptions::baseline_schema`
///    is present; if neither exists, returns `Ok(())` without constructing a validator.
/// 2. Builds `DarkmatterSchemas::new()` plus `.with_baseline(...)` when
///    `ComposeOptions::baseline_schema` is set.
/// 3. Calls `.validate(&markdown)`.
/// 4. Converts schema-preparation `SchemaError` into
///    `MarkdownError::SchemaValidationFailed`.
/// 5. Converts `ValidationReport { valid: false, problems }` into the same
///    error variant. On success, returns `Ok(())`.
pub(crate) fn run(markdown: &Markdown, options: &ComposeOptions) -> MarkdownResult<()> {
    let has_document_schema = markdown.frontmatter().as_map().contains_key("$schema");
    let has_baseline = options.baseline_schema.is_some();

    if !has_document_schema && !has_baseline {
        return Ok(());
    }

    let path = source_path(markdown, options);
    let description = markdown
        .frontmatter()
        .as_map()
        .get("description")
        .and_then(|v| v.as_str().map(String::from));

    let schemas = {
        let mut builder = DarkmatterSchemas::new();
        if let Some(baseline) = options.baseline_schema.clone() {
            builder = builder.with_baseline(baseline).map_err(|err| {
                MarkdownError::SchemaValidationFailed {
                    path: path.clone(),
                    problems: Vec::new(),
                    summary: format!("schema could not be prepared: {err}"),
                    description: description.clone(),
                }
            })?;
        }
        builder
    };

    let report = schemas.validate(markdown).map_err(|err| {
        MarkdownError::SchemaValidationFailed {
            path: path.clone(),
            problems: Vec::new(),
            summary: format!("schema could not be prepared: {err}"),
            description: description.clone(),
        }
    })?;

    if !report.valid {
        return Err(MarkdownError::SchemaValidationFailed {
            path,
            problems: report.problems,
            summary: "frontmatter did not satisfy the schema".to_string(),
            description,
        });
    }

    Ok(())
}

/// Determines the source path to report in validation errors.
///
/// Prefers the document's own source, falling back to the compose options
/// source. Returns `<stdin>` when no source is known.
///
/// ## Notes
///
/// The error variant carries a [`PathBuf`] because file sources dominate the
/// real-world usage. For non-file sources (`Url`, `Unknown`) the returned
/// `PathBuf` is constructed from the source's [`ComposeSource::display()`]
/// form and is semantically a *display carrier*, not a filesystem path —
/// renderers use [`Path::to_string_lossy`] to surface it, which is correct
/// for both file paths and the URL/`<stdin>` strings.
fn source_path(markdown: &Markdown, options: &ComposeOptions) -> PathBuf {
    fn carrier(source: &ComposeSource) -> Option<PathBuf> {
        match source {
            ComposeSource::Unknown => None,
            ComposeSource::File(p) => Some(p.clone()),
            // URL is wrapped in a PathBuf as a display carrier — the
            // rendered block reads it back via `to_string_lossy()`.
            ComposeSource::Url(_) => Some(PathBuf::from(source.display().as_ref())),
        }
    }

    markdown
        .source()
        .as_ref()
        .and_then(carrier)
        .or_else(|| carrier(&options.source))
        .unwrap_or_else(|| PathBuf::from(ComposeSource::Unknown.display().as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::{
        Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType,
    };
    use indexmap::IndexMap;

    fn md_with_schema(yaml_body: &str) -> Markdown {
        let content = format!("---\n{yaml_body}---\nbody\n");
        content.as_str().into()
    }

    fn md_with_schema_and_source(yaml_body: &str, path: &std::path::Path) -> Markdown {
        let content = format!("---\n{yaml_body}---\nbody\n");
        let md: Markdown = content.as_str().into();
        md.with_source(ComposeSource::File(path.into()))
    }

    fn baseline_required_string(property: &str) -> SimplifiedSchema {
        let mut properties = IndexMap::new();
        properties.insert(
            property.into(),
            PropertyDef::Single(PropertyAtom {
                ty: SimplifiedType::String,
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        SimplifiedSchema::Single(SchemaShape { properties })
    }

    // ── No-op cases ───────────────────────────────────────────────────

    #[test]
    fn no_schema_no_baseline_is_no_op() {
        let md = md_with_schema("name: alice\n");
        let options = ComposeOptions::new();
        assert!(run(&md, &options).is_ok());
    }

    #[test]
    fn no_schema_no_baseline_downstream_stages_untouched() {
        let md = md_with_schema("title: Hello\n");
        let options = ComposeOptions::new();
        let result = run(&md, &options);
        assert!(result.is_ok());
        // Frontmatter should remain intact
        assert_eq!(
            md.frontmatter().as_map().get("title"),
            Some(&serde_json::json!("Hello"))
        );
    }

    // ── Document $schema honored ──────────────────────────────────────

    #[test]
    fn document_schema_valid() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: Hello\n");
        let options = ComposeOptions::new();
        assert!(run(&md, &options).is_ok());
    }

    #[test]
    fn document_schema_missing_required_fails() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let options = ComposeOptions::new();
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, summary, .. } => {
                assert!(!problems.is_empty());
                assert_eq!(summary, "frontmatter did not satisfy the schema");
                assert!(problems.iter().any(|p| p.property.as_deref() == Some("title")));
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn document_schema_wrong_type_fails() {
        let md = md_with_schema("$schema:\n  spec: 'string(required)'\nspec: 42\n");
        let options = ComposeOptions::new();
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(!problems.is_empty());
                assert!(
                    problems.iter().any(|p| p.path == "/spec"),
                    "expected problem on /spec, got {problems:?}"
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    // ── Baseline merging ──────────────────────────────────────────────

    #[test]
    fn baseline_applies_when_document_has_no_schema() {
        let md = md_with_schema("title: hi\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline_required_string("owner"));
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(problems.iter().any(|p| p.property.as_deref() == Some("owner")));
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn baseline_applies_when_document_has_no_schema_and_valid() {
        let md = md_with_schema("owner: alice\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline_required_string("owner"));
        assert!(run(&md, &options).is_ok());
    }

    #[test]
    fn baseline_merges_with_document_schema() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline_required_string("owner"));
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(problems.iter().any(|p| p.property.as_deref() == Some("owner")));
                // Document schema property should not be reported as missing
                assert!(!problems.iter().any(|p| p.property.as_deref() == Some("title")));
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn document_wins_when_both_declare_same_property() {
        // Baseline says `spec` is required string; document says `spec` is number.
        // Document wins, so `spec: 42` should validate successfully.
        let mut baseline_props = IndexMap::new();
        baseline_props.insert(
            "spec".into(),
            PropertyDef::Single(PropertyAtom {
                ty: SimplifiedType::String,
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        let baseline = SimplifiedSchema::Single(SchemaShape {
            properties: baseline_props,
        });

        let md = md_with_schema("$schema:\n  spec: 'number(required)'\nspec: 42\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline);
        assert!(run(&md, &options).is_ok());
    }

    // ── Override interaction ──────────────────────────────────────────

    #[test]
    fn set_override_can_fix_validation_failure() {
        // Set overrides are applied by the pipeline before schema_validation::run
        // is called. To unit-test the interaction, we manually apply the override
        // to the frontmatter and then call run.
        let mut md = md_with_schema("$schema:\n  spec: 'string(min(1);required)'\nspec: \"\"\n");
        md.frontmatter_mut()
            .as_map_mut()
            .insert("spec".into(), serde_json::json!("design.md"));
        let options = ComposeOptions::new();
        assert!(run(&md, &options).is_ok());
    }

    #[test]
    fn set_override_can_introduce_validation_failure() {
        // Set overrides are applied by the pipeline before schema_validation::run
        // is called. To unit-test the interaction, we manually apply the override
        // to the frontmatter and then call run.
        let mut md = md_with_schema("$schema:\n  spec: 'string(min(1);required)'\nspec: 'design.md'\n");
        md.frontmatter_mut()
            .as_map_mut()
            .insert("spec".into(), serde_json::json!(""));
        let options = ComposeOptions::new();
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(problems.iter().any(|p| p.path == "/spec"));
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    // ── Error path and description ────────────────────────────────────

    #[test]
    fn error_includes_document_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        let md = md_with_schema_and_source(
            "$schema:\n  title: 'string(required)'\nother: stuff\n",
            &path,
        );
        let options = ComposeOptions::new().with_source_file(&path);
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { path: err_path, .. } => {
                assert_eq!(err_path, path);
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn error_includes_description_when_present() {
        let md = md_with_schema(
            "$schema:\n  title: 'string(required)'\ndescription: My doc\nother: stuff\n",
        );
        let options = ComposeOptions::new();
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed {
                description: Some(desc),
                ..
            } => {
                assert_eq!(desc, "My doc");
            }
            other => panic!("expected SchemaValidationFailed with description, got {other:?}"),
        }
    }

    #[test]
    fn schema_preparation_error_carries_message() {
        // An invalid baseline schema triggers a preparation error.
        // We use with_baseline_schema which already validates the schema,
        // so we construct an invalid one indirectly by passing something
        // that will fail at validation time. A simpler approach: use a
        // malformed inline schema in the document.
        let md = md_with_schema("$schema: 42\n");
        let options = ComposeOptions::new();
        let err = run(&md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, summary, .. } => {
                assert!(problems.is_empty());
                assert!(
                    summary.contains("schema could not be prepared"),
                    "expected preparation summary, got: {summary}"
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }
}
