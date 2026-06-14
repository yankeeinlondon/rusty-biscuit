//! Schema validation stage for the compose pipeline.
//!
//! This module implements the always-on Schema Validation stage that runs
//! after `--set` / `--state` overrides are applied to frontmatter AND after
//! frontmatter interpolation has resolved `{{ }}` expressions, but BEFORE
//! frontmatter shell expansion runs. Validating after interpolation lets
//! schema-constrained fields derive from templates
//! (e.g. `runtime_agent: '{{ env.AGENT }}'`); validating before shell
//! expansion preserves fail-fast behavior so an invalid schema does not
//! trigger expensive or side-effectful shell commands. When frontmatter
//! shell expansion is enabled, values that depend on shell-expanded inputs
//! are re-validated downstream by the caller (e.g. claudine's
//! `prepare_*_with_schema`) against the post-shell effective frontmatter,
//! so their problems are deferred here. When shell expansion is disabled,
//! no later stage re-resolves those values, so every problem is reported
//! here rather than deferred.
//!
//! When the effective frontmatter violates the resolved schema, compose aborts
//! with a styled [`BlockError`] that names the offending property.
//!
//! This stage also MUTATES the stored frontmatter: before reporting, it coerces
//! schema-recognized scalar values to their declared types (e.g. the string
//! `"true"` against a `boolean` field becomes a real JSON bool) via
//! [`coerce_frontmatter_with_pending`] and writes the coerced top-level properties back into
//! `markdown.frontmatter_mut()`, so the real types flow to every later stage and
//! into the composed output. Values still holding a `$(...)` shell expression are
//! left untouched here — their real type is resolved at post-shell re-validation.

use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::types::{ComposeOperation, ComposeSource};
use crate::markdown::schemas::DarkmatterSchemas;
use crate::markdown::schemas::coerce::coerce_frontmatter_with_pending;
use crate::markdown::types::{MarkdownError, MarkdownResult};

/// Runs schema validation against the document's effective frontmatter,
/// coercing schema-recognized scalar values to their declared types and
/// writing the coerced values back into the document's frontmatter.
///
/// 1. Checks whether either document `$schema` or `ComposeOptions::baseline_schema`
///    is present; if neither exists, returns `Ok(())` without constructing a validator.
/// 2. Builds `DarkmatterSchemas::new()` plus `.with_baseline(...)` when
///    `ComposeOptions::baseline_schema` is set.
/// 3. Resolves the effective schema, coerces the frontmatter against it via
///    [`coerce_frontmatter_with_pending`], and writes coerced top-level properties back into
///    `markdown.frontmatter_mut()` (skipping any value still holding a `$(...)`
///    shell expression). The pending-key set is passed through so a root-union
///    arm can be committed when its only residual problems are shell-pending
///    keys, letting non-shell siblings still coerce and write back.
/// 4. Calls `.validate(&markdown)` on the now-coerced frontmatter.
/// 5. Converts schema-preparation `SchemaError` into
///    `MarkdownError::SchemaValidationFailed`, preserving the original error
///    on the variant's `source` field for `Error::source()` recovery.
/// 6. Converts `ValidationReport { valid: false, problems }` into the same
///    error variant. On success, returns `Ok(())`.
pub(crate) fn run(markdown: &mut Markdown, options: &ComposeOptions) -> MarkdownResult<()> {
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
                    source: Some(Box::new(err)),
                }
            })?;
        }
        builder
    };

    // Coerce schema-recognized scalars to their declared types and write the
    // coerced top-level properties back, so real types flow to every later
    // stage and into the composed output. `effective_for` returns an owned
    // schema, so no borrow of `markdown` persists into the mutation below.
    if let Some(effective) =
        schemas
            .effective_for(markdown)
            .map_err(|err| MarkdownError::SchemaValidationFailed {
                path: path.clone(),
                problems: Vec::new(),
                summary: format!("schema could not be prepared: {err}"),
                description: description.clone(),
                source: Some(Box::new(err)),
            })?
    {
        // Build the validation instance and the shell-pending key set while
        // holding only an immutable borrow; both are dropped before the write.
        let (instance, shell_pending) = {
            let fm_map = markdown.frontmatter().as_map();
            let mut object = serde_json::Map::with_capacity(fm_map.len());
            let mut shell_pending = std::collections::HashSet::new();
            for (key, value) in fm_map {
                // `$schema` is a Darkmatter control key, not document data. This
                // exclusion must match `schemas::frontmatter_as_json`, which
                // builds the instance the library validate path coerces against;
                // if the two diverge, compose and validate would coerce against
                // different instances.
                if key == "$schema" {
                    continue;
                }
                if value_needs_shell_expansion(Some(value)) {
                    shell_pending.insert(key.clone());
                }
                object.insert(key.clone(), value.clone());
            }
            (serde_json::Value::Object(object), shell_pending)
        };

        let outcome =
            coerce_frontmatter_with_pending(&effective.json_schema, &instance, &shell_pending);
        if outcome.changed
            && let serde_json::Value::Object(coerced) = outcome.value
        {
            let fm_map = markdown.frontmatter_mut().as_map_mut();
            for (key, value) in coerced {
                // A value still holding `$(...)` is resolved (and coerced)
                // later at post-shell re-validation; writing back here would
                // clobber the literal form shell expansion must consume.
                if shell_pending.contains(&key) {
                    continue;
                }
                fm_map.insert(key, value);
            }
        }
    }

    let report =
        schemas
            .validate(markdown)
            .map_err(|err| MarkdownError::SchemaValidationFailed {
                path: path.clone(),
                problems: Vec::new(),
                summary: format!("schema could not be prepared: {err}"),
                description: description.clone(),
                source: Some(Box::new(err)),
            })?;

    // Defer problems whose value will be re-resolved by frontmatter shell
    // expansion. The compose-time validator runs BEFORE shell expansion so
    // values that depend on `$(...)` expressions still hold their literal
    // form here. The downstream consumer (e.g. claudine's
    // `prepare_*_with_schema`) re-validates the post-shell effective
    // frontmatter and reports any residual problems. See the rationale at
    // `compose::run::compose` where this stage is invoked.
    //
    // Deferral is only sound when shell expansion will actually run. When
    // `FrontmatterShellExpansion` is disabled, no later stage expands or
    // re-validates `$(...)` values, so deferring would silently accept a
    // schema violation. In that case every problem is final and must be
    // reported here.
    let shell_expansion_enabled = options.is_enabled(ComposeOperation::FrontmatterShellExpansion);
    let fm_map = markdown.frontmatter().as_map();
    let composition_independent: Vec<_> = report
        .problems
        .iter()
        .filter(|p| {
            if !shell_expansion_enabled {
                return true;
            }
            let Some(name) = top_level_pointer_segment(&p.path) else {
                return true;
            };
            !value_needs_shell_expansion(fm_map.get(&name))
        })
        .cloned()
        .collect();

    if !report.valid && !composition_independent.is_empty() {
        return Err(MarkdownError::SchemaValidationFailed {
            path,
            problems: composition_independent,
            summary: "frontmatter did not satisfy the schema".to_string(),
            description,
            // Schema was prepared successfully; the failure detail lives in
            // `problems`, so there is no upstream `SchemaError` cause.
            source: None,
        });
    }

    Ok(())
}

/// Returns `true` when `value` contains a frontmatter shell expression
/// (`$(...)`) somewhere in any string descendant. Compose-time schema
/// validation must not fail values that will be transformed by frontmatter
/// shell expansion — the consumer re-validates the post-shell effective
/// frontmatter.
fn value_needs_shell_expansion(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else { return false };
    match value {
        serde_json::Value::String(s) => s.contains("$("),
        serde_json::Value::Array(items) => {
            items.iter().any(|v| value_needs_shell_expansion(Some(v)))
        }
        serde_json::Value::Object(map) => {
            map.values().any(|v| value_needs_shell_expansion(Some(v)))
        }
        _ => false,
    }
}

fn top_level_pointer_segment(pointer: &str) -> Option<String> {
    let stripped = pointer.strip_prefix('/')?;
    let first = stripped.split('/').next()?;
    if first.is_empty() {
        return None;
    }
    Some(first.replace("~1", "/").replace("~0", "~"))
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
        TypeExpr,
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
                ty: TypeExpr::Primitive(SimplifiedType::String),
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
        let mut md = md_with_schema("name: alice\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
    }

    #[test]
    fn no_schema_no_baseline_downstream_stages_untouched() {
        let mut md = md_with_schema("title: Hello\n");
        let options = ComposeOptions::new();
        let result = run(&mut md, &options);
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
        let mut md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: Hello\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
    }

    #[test]
    fn document_schema_missing_required_fails() {
        let mut md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed {
                problems, summary, ..
            } => {
                assert!(!problems.is_empty());
                assert_eq!(summary, "frontmatter did not satisfy the schema");
                assert!(
                    problems
                        .iter()
                        .any(|p| p.property.as_deref() == Some("title"))
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn document_schema_number_coerces_to_string_and_writes_back() {
        // `42` against a `string` field is coercible (`42` -> `"42"`), so under
        // coercion this now validates AND the stored value is the string "42".
        let mut md = md_with_schema("$schema:\n  spec: 'string(required)'\nspec: 42\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("42"))
        );
    }

    #[test]
    fn document_schema_uncoercible_type_still_fails() {
        // An array against a `string` field is outside the coercion matrix, so
        // coercion never masks it — the type problem must still be reported.
        let mut md = md_with_schema("$schema:\n  spec: 'string(required)'\nspec:\n  - a\n  - b\n");
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
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
        // Write-back must not have partially mutated the uncoercible value.
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!(["a", "b"]))
        );
    }

    // ── Coercion write-back ───────────────────────────────────────────

    #[test]
    fn write_back_produces_real_boolean() {
        // Post-interpolation literal form: the resolved string "true" against a
        // `boolean` field is coerced and written back as a real JSON bool.
        let mut md = md_with_schema("$schema:\n  has_spec: boolean\nhas_spec: \"true\"\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("has_spec"),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn write_back_produces_real_number() {
        let mut md = md_with_schema("$schema:\n  n: number\nn: \"42\"\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("n"),
            Some(&serde_json::json!(42))
        );
    }

    #[test]
    fn write_back_handles_root_union() {
        // Mirrors prompts/implement.md: a 3-arm root union where every arm types
        // `has_*` as boolean. Frontmatter holds them as strings; after run they
        // must be stored as real booleans.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 - review: string(required)\n\
             \x20   has_spec: boolean\n\
             \x20   has_plan: boolean\n\
             \x20   has_review: boolean\n\
             \x20 - spec: string(required)\n\
             \x20   has_spec: boolean\n\
             \x20   has_plan: boolean\n\
             \x20   has_review: boolean\n\
             \x20 - plan: string(required)\n\
             \x20   has_spec: boolean\n\
             \x20   has_plan: boolean\n\
             \x20   has_review: boolean\n\
             spec: design.md\n\
             has_spec: \"true\"\n\
             has_plan: \"false\"\n\
             has_review: \"false\"\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let fm = md.frontmatter();
        let map = fm.as_map();
        assert_eq!(map.get("has_spec"), Some(&serde_json::json!(true)));
        assert_eq!(map.get("has_plan"), Some(&serde_json::json!(false)));
        assert_eq!(map.get("has_review"), Some(&serde_json::json!(false)));
    }

    #[test]
    fn write_back_skips_shell_pending_value() {
        // A `$(...)`-bearing typed value is deferred (run returns Ok) AND left
        // untouched so its literal form survives into shell expansion.
        let mut md = md_with_schema("$schema:\n  n: number\nn: \"$(echo 1)\"\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("n"),
            Some(&serde_json::json!("$(echo 1)"))
        );
    }

    #[test]
    fn write_back_root_union_defers_pending_and_coerces_sibling() {
        // A root-union arm declares both a shell-pending typed field (`n:
        // number`) and a non-shell boolean (`flag`). The pending `n` must be
        // deferred (left as its literal `$(...)` form, run returns Ok) while the
        // non-shell `flag` still coerces and is written back as a real bool —
        // even though `n` keeps the raw arm candidate from validating.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 - kind: string(required)\n\
             \x20   n: number\n\
             \x20   flag: boolean\n\
             \x20 - other: string(required)\n\
             kind: implement\n\
             n: \"$(echo 1)\"\n\
             flag: \"false\"\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let fm = md.frontmatter();
        let map = fm.as_map();
        assert_eq!(map.get("flag"), Some(&serde_json::json!(false)));
        assert_eq!(map.get("n"), Some(&serde_json::json!("$(echo 1)")));
    }

    // ── Baseline merging ──────────────────────────────────────────────

    #[test]
    fn baseline_applies_when_document_has_no_schema() {
        let mut md = md_with_schema("title: hi\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline_required_string("owner"));
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems
                        .iter()
                        .any(|p| p.property.as_deref() == Some("owner"))
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn baseline_applies_when_document_has_no_schema_and_valid() {
        let mut md = md_with_schema("owner: alice\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline_required_string("owner"));
        assert!(run(&mut md, &options).is_ok());
    }

    #[test]
    fn baseline_merges_with_document_schema() {
        let mut md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline_required_string("owner"));
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems
                        .iter()
                        .any(|p| p.property.as_deref() == Some("owner"))
                );
                // Document schema property should not be reported as missing
                assert!(
                    !problems
                        .iter()
                        .any(|p| p.property.as_deref() == Some("title"))
                );
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
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        let baseline = SimplifiedSchema::Single(SchemaShape {
            properties: baseline_props,
        });

        let mut md = md_with_schema("$schema:\n  spec: 'number(required)'\nspec: 42\n");
        let options = ComposeOptions::new().with_baseline_schema(baseline);
        assert!(run(&mut md, &options).is_ok());
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
        assert!(run(&mut md, &options).is_ok());
    }

    #[test]
    fn set_override_can_introduce_validation_failure() {
        // Set overrides are applied by the pipeline before schema_validation::run
        // is called. To unit-test the interaction, we manually apply the override
        // to the frontmatter and then call run.
        let mut md =
            md_with_schema("$schema:\n  spec: 'string(min(1);required)'\nspec: 'design.md'\n");
        md.frontmatter_mut()
            .as_map_mut()
            .insert("spec".into(), serde_json::json!(""));
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
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
        let mut md = md_with_schema_and_source(
            "$schema:\n  title: 'string(required)'\nother: stuff\n",
            &path,
        );
        let options = ComposeOptions::new().with_source_file(&path);
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { path: err_path, .. } => {
                assert_eq!(err_path, path);
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn error_includes_description_when_present() {
        let mut md = md_with_schema(
            "$schema:\n  title: 'string(required)'\ndescription: My doc\nother: stuff\n",
        );
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
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

    // ── Shell-dependent deferral gating ───────────────────────────────

    #[test]
    fn shell_dependent_problem_deferred_when_shell_expansion_enabled() {
        // `spec` holds a `$(...)` value and shell expansion is enabled (the
        // default), so the type violation is deferred to post-shell
        // re-validation and run returns Ok.
        let mut md = md_with_schema("$schema:\n  spec: 'number(required)'\nspec: \"$(echo 1)\"\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
    }

    #[test]
    fn shell_dependent_problem_not_deferred_when_shell_expansion_disabled() {
        // Same document, but FrontmatterShellExpansion is disabled. Nothing
        // downstream will expand or re-validate `spec`, so the schema
        // violation must be reported rather than silently deferred.
        let mut md = md_with_schema("$schema:\n  spec: 'number(required)'\nspec: \"$(echo 1)\"\n");
        let options = ComposeOptions::new().disable(ComposeOperation::FrontmatterShellExpansion);
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| p.path == "/spec"),
                    "expected problem on /spec, got {problems:?}"
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn schema_preparation_error_carries_message() {
        // An invalid baseline schema triggers a preparation error.
        // We use with_baseline_schema which already validates the schema,
        // so we construct an invalid one indirectly by passing something
        // that will fail at validation time. A simpler approach: use a
        // malformed inline schema in the document.
        let mut md = md_with_schema("$schema: 42\n");
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed {
                problems, summary, ..
            } => {
                assert!(problems.is_empty());
                assert!(
                    summary.contains("schema could not be prepared"),
                    "expected preparation summary, got: {summary}"
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    #[test]
    fn schema_preparation_error_preserves_source_chain() {
        use std::error::Error;

        // A malformed inline `$schema` (an integer, not a mapping/sequence/
        // string) fails during schema preparation. The compose error must
        // expose the underlying `SchemaError` via `Error::source()` so
        // programmatic callers can recover and inspect the original cause
        // rather than only reading the formatted summary.
        let mut md = md_with_schema("$schema: 42\n");
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();

        let source = err
            .source()
            .expect("preparation failure should expose a source error");
        let schema_err = source
            .downcast_ref::<crate::markdown::schemas::SchemaError>()
            .expect("source should downcast to SchemaError");
        assert!(
            matches!(
                schema_err,
                crate::markdown::schemas::SchemaError::FrontmatterShape { .. }
            ),
            "expected FrontmatterShape, got {schema_err:?}"
        );
    }

    #[test]
    fn validation_failure_has_no_source_chain() {
        use std::error::Error;

        // A successfully prepared schema that the frontmatter simply does not
        // satisfy carries its detail in `problems`, not an upstream
        // `SchemaError`. `Error::source()` must be `None` so callers can
        // distinguish "schema could not be prepared" from "frontmatter did
        // not satisfy the prepared schema".
        let mut md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
        assert!(
            err.source().is_none(),
            "validation failure should have no source"
        );
    }

    // ── Inline object coercion (Phase 2) ─────────────────────────────────

    #[test]
    fn inline_object_nested_scalar_coercion() {
        // `enabled: "true"` against `boolean` and `retries: "3"` against
        // `number` are coerced through the inline object fragment.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 config: \"{ enabled: boolean, retries: number }\"\n\
             config:\n\
             \x20 enabled: \"true\"\n\
             \x20 retries: \"3\"\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let map = md.frontmatter().as_map();
        let config = map.get("config").expect("config written back");
        assert_eq!(config["enabled"], serde_json::json!(true));
        assert_eq!(config["retries"], serde_json::json!(3));
    }

    #[test]
    fn inline_object_mixed_opaque_sibling_still_coerces_recognized_field() {
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 config: \"{ enabled: boolean, metadata: object }\"\n\
             config:\n\
             \x20 enabled: \"true\"\n\
             \x20 metadata:\n\
             \x20   source: user\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let config = &md.frontmatter().as_map()["config"];
        assert_eq!(config["enabled"], serde_json::json!(true));
        assert_eq!(config["metadata"], serde_json::json!({ "source": "user" }));
    }

    #[test]
    fn inline_object_array_nested_coercion() {
        // Each array item's nested scalar is coerced independently.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 authors: \"{ active: boolish, score: numberlike }[]\"\n\
             authors:\n\
             \x20 - active: \"true\"\n\
             \x20   score: \"4.5\"\n\
             \x20 - active: \"False\"\n\
             \x20   score: \"-2\"\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let authors = &md.frontmatter().as_map()["authors"];
        assert_eq!(authors[0]["active"], serde_json::json!(true));
        assert_eq!(authors[0]["score"], serde_json::json!(4.5));
        assert_eq!(authors[1]["active"], serde_json::json!(false));
        assert_eq!(authors[1]["score"], serde_json::json!(-2));
    }

    #[test]
    fn inline_object_array_mixed_opaque_sibling_still_coerces_recognized_field() {
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 authors: \"{ active: boolean, metadata: object }[]\"\n\
             authors:\n\
             \x20 - active: \"true\"\n\
             \x20   metadata:\n\
             \x20     role: admin\n\
             \x20 - active: \"false\"\n\
             \x20   metadata:\n\
             \x20     role: reader\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let authors = &md.frontmatter().as_map()["authors"];
        assert_eq!(authors[0]["active"], serde_json::json!(true));
        assert_eq!(authors[1]["active"], serde_json::json!(false));
        assert_eq!(
            authors[0]["metadata"],
            serde_json::json!({ "role": "admin" })
        );
    }

    #[test]
    fn inline_object_uncoercible_value_left_alone() {
        // An array nested inside an inline object property that declares
        // `string` is outside the matrix; coercion must not touch it and
        // validation still fails.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 config: \"{ name: string(required) }\"\n\
             config:\n\
             \x20 name:\n\
             \x20   - Ada\n\
             \x20   - Lovelace\n",
        );
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| p.path.contains("/name")),
                    "expected a problem under /config/name, got {problems:?}"
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
        // The value is not coerced (array is outside the matrix), so it
        // survives unchanged for the validator to reject.
        let config = &md.frontmatter().as_map()["config"];
        assert_eq!(config["name"], serde_json::json!(["Ada", "Lovelace"]));
    }

    #[test]
    fn inline_object_unambiguous_union_coercion() {
        // `metadata: [inline-object, string]` with a value that satisfies
        // exactly one arm after coercion. The inline object arm wins and the
        // nested string coerces to a number; the original value is replaced.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 metadata:\n\
             \x20   - \"{ key: string(required), count: number }\"\n\
             \x20   - string\n\
             metadata:\n\
             \x20   key: visits\n\
             \x20   count: \"42\"\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let metadata = &md.frontmatter().as_map()["metadata"];
        assert_eq!(metadata["key"], serde_json::json!("visits"));
        assert_eq!(metadata["count"], serde_json::json!(42));
    }

    #[test]
    fn inline_object_union_with_opaque_sibling_coerces_recognized_field() {
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 metadata:\n\
             \x20   - \"{ kind: string(required), enabled: boolean(required), details: object(required) }\"\n\
             \x20   - string\n\
             metadata:\n\
             \x20 kind: config\n\
             \x20 enabled: \"true\"\n\
             \x20 details:\n\
             \x20   source: user\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let metadata = &md.frontmatter().as_map()["metadata"];
        assert_eq!(metadata["enabled"], serde_json::json!(true));
        assert_eq!(metadata["kind"], serde_json::json!("config"));
        assert_eq!(metadata["details"], serde_json::json!({ "source": "user" }));
    }

    #[test]
    fn inline_object_zero_match_union_coercion() {
        // An object value that no arm can accept — the inline object arm
        // needs `{key, count}` and the string arm needs a string. An array
        // value satisfies neither, so per-arm coercion yields no candidates
        // and the original value is left untouched; normal validation then
        // reports the failure.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 metadata:\n\
             \x20   - \"{ key: string(required), count: number }\"\n\
             \x20   - string\n\
             metadata:\n\
             \x20 - a\n\
             \x20 - b\n",
        );
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(!problems.is_empty(), "expected at least one problem");
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
        // The original array survives untouched — no coercion happened.
        assert_eq!(
            md.frontmatter().as_map()["metadata"],
            serde_json::json!(["a", "b"])
        );
    }

    #[test]
    fn inline_object_ambiguous_union_coercion_leaves_value_alone() {
        // `42` (a number) coerces to `"42"` (string) and validates against
        // both arms after coercion — the number arm accepts `42` directly
        // and the string arm accepts `"42"`. Per-arm coercion sees two
        // matching arms, leaves the value untouched (no guessing), and
        // normal validation runs against the unchanged `42`.
        let mut md = md_with_schema(
            "$schema:\n\
             \x20 count:\n\
             \x20   - number\n\
             \x20   - string\n\
             count: \"42\"\n",
        );
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        // `"42"` was unambiguous against the string arm AND ambiguous with
        // the number arm (`"42"` coerces to 42, which matches number), so
        // both arms validate; coercion bails out and the value is left
        // alone. The string arm still validates the original `"42"`, so
        // the document is valid.
        assert_eq!(md.frontmatter().as_map()["count"], serde_json::json!("42"));
    }
}
