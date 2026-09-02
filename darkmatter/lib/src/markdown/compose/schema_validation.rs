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
//! This stage also MUTATES the stored frontmatter. It first materializes each
//! absent, optional, no-default top-level property owned by the document's
//! inline or referenced SimplifiedSchema as JSON `null`. It then coerces
//! schema-recognized scalar values to their declared types (e.g. the string
//! `"true"` against a `boolean` field becomes a real JSON bool) via
//! [`coerce_frontmatter_with_pending`] and writes the coerced top-level
//! properties back into `markdown.frontmatter_mut()`, so the effective bindings
//! and real types flow to every later stage and into the composed output. Values
//! still composition-pending — holding a `$(...)` shell expression or an
//! unresolved `{{ ... }}` template — are left untouched here; their real type
//! is resolved at post-shell re-validation.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[cfg(test)]
pub(crate) static TRIGGER_DISCOVERY_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::{ComposeOperation, ComposeSource};
use crate::markdown::schemas::{
    CallerFileReferenceProvenance, DarkmatterSchemas, EffectiveSchema, FileReferenceDiagnostic,
    JsonPointer, SchemaOriginKind, SimplifiedSchema, ValidationProblem, ValidationProblemCode,
    ValidationProblemKind,
};
use crate::markdown::schemas::coerce::coerce_frontmatter_with_pending;
use crate::markdown::types::{MarkdownError, MarkdownResult};

/// Single-pass convenience wrapper used by this module's tests.
#[cfg(test)]
pub(crate) fn run(markdown: &mut Markdown, options: &ComposeOptions) -> MarkdownResult<()> {
    let mut trigger_registry = None;
    let prepared = prepare_schemas(markdown, options, &mut trigger_registry)?;
    let projection = prepare_caller_projection(markdown, options, &prepared)?;
    projection.install(markdown);
    run_with_registry(markdown, options, &prepared, &projection)
}

/// Request-owned schema state assembled once before frontmatter interpolation.
pub(crate) struct PreparedSchemas {
    schemas: Option<DarkmatterSchemas>,
}

/// Caller-file values projected from the captured launch context.
#[derive(Default)]
pub(crate) struct CallerProjection {
    native: serde_json::Map<String, serde_json::Value>,
    presentation: HashMap<String, serde_json::Value>,
    modes: HashMap<String, CallerFileMode>,
    classified: HashSet<String>,
    provenance: HashMap<String, super::expression::resolve_ctx::CallerFileProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallerFileMode {
    Eager,
    Lazy,
}

impl CallerProjection {
    pub(crate) fn install(&self, markdown: &mut Markdown) {
        let frontmatter = markdown.frontmatter_mut().as_map_mut();
        for (key, value) in &self.native {
            frontmatter.insert(key.clone(), value.clone());
        }
    }

    pub(crate) fn presentation_values(&self) -> HashMap<String, serde_json::Value> {
        self.presentation.clone()
    }

    pub(crate) fn install_provenance(&self, options: &mut ComposeOptions) {
        options.caller_file_provenance = self.provenance.clone();
    }
}

fn trigger_discovery_boundary(options: &ComposeOptions, document_path: &Path) -> Option<PathBuf> {
    match options.file_resolution_context.as_ref() {
        Some(context) => context.repository_root().map(Path::to_path_buf),
        None => crate::markdown::compose::find_git_root_from(document_path),
    }
}

pub(crate) fn prepare_schemas(
    markdown: &Markdown,
    options: &ComposeOptions,
    trigger_registry: &mut Option<crate::markdown::schemas::triggers::TriggerRegistry>,
) -> MarkdownResult<PreparedSchemas> {
    let has_document_schema = markdown.frontmatter().as_map().contains_key("$schema");
    if !has_document_schema && options.baseline_schema.is_none() && !options.trigger_schemas {
        return Ok(PreparedSchemas { schemas: None });
    }

    let path = source_path(markdown, options);
    let description = markdown
        .frontmatter()
        .as_map()
        .get("description")
        .and_then(|value| value.as_str().map(String::from));
    let prepare_error = |summary: String, source| MarkdownError::SchemaValidationFailed {
        path: path.clone(),
        problems: Vec::new(),
        summary,
        description: description.clone(),
        source: Some(Box::new(source)),
    };

    let mut schemas = DarkmatterSchemas::new();
    if let Some(fallback) = options.file_ref_fallback_dir.clone() {
        schemas = schemas.with_file_ref_fallback_dir(fallback);
    }
    if let Some(context) = options.file_resolution_context.clone() {
        schemas = schemas.with_file_resolution_context(context);
    }
    if let Some(baseline) = options.baseline_schema.clone() {
        schemas = schemas.with_baseline(baseline).map_err(|err| {
            prepare_error(format!("schema could not be prepared: {err}"), err)
        })?;
    }
    if options.trigger_schemas
        && let Some(ComposeSource::File(document_path)) = markdown.source()
        && let Some(boundary) = trigger_discovery_boundary(options, document_path)
    {
        schemas = if let Some(registry) = trigger_registry.take() {
            schemas.with_trigger_registry(registry)
        } else {
            #[cfg(test)]
            TRIGGER_DISCOVERY_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            schemas
                .with_trigger_discovery(document_path, boundary)
                .map_err(|err| {
                    prepare_error(format!("trigger schemas could not be prepared: {err}"), err)
                })?
        };
        if let Some(registry) = schemas.trigger_registry() {
            *trigger_registry = Some(registry.clone());
        }
    }

    Ok(PreparedSchemas {
        schemas: Some(schemas),
    })
}

/// Runs schema validation against the document's effective frontmatter,
/// materializing eligible optional bindings, coercing schema-recognized scalar
/// values to their declared types, and writing the effective values back into
/// the document's frontmatter.
///
/// 1. Checks whether document `$schema`, `ComposeOptions::baseline_schema`, or
///    trigger schemas are in play; if none, returns `Ok(())` without
///    constructing a validator.
/// 2. Builds `DarkmatterSchemas::new()` plus the baseline: the process-cached
///    compiled JSON Schema when the baseline is the Darkmatter default (F9),
///    else `.with_baseline(...)` for a caller-supplied `SimplifiedSchema`.
/// 3. Resolves the effective schema once and materializes absent, optional,
///    no-default top-level bindings owned by a document or referenced-file
///    SimplifiedSchema as JSON `null`.
/// 4. Coerces the frontmatter against that schema via
///    [`coerce_frontmatter_with_pending`] and writes coerced top-level properties
///    back into `markdown.frontmatter_mut()` (skipping any value still
///    composition-pending — a `$(...)` shell expression or an unresolved
///    `{{ ... }}` template). The pending-key set is passed through so a
///    root-union arm can be committed when its only residual problems are
///    composition-pending keys, letting resolved siblings still coerce and
///    write back.
/// 5. Validates the coerced frontmatter through the held [`EffectiveSchema`]
///    (F5); with active triggers, re-resolves via `schemas.validate` because
///    trigger matching depends on the coerced values.
/// 6. Converts schema-preparation `SchemaError` into
///    `MarkdownError::SchemaValidationFailed`, preserving the original error
///    on the variant's `source` field for `Error::source()` recovery.
/// 7. Converts `ValidationReport { valid: false, problems }` into the same
///    error variant. On success, returns `Ok(())`.
///
/// `prepared` owns the schema assembly and trigger registry discovered before
/// frontmatter interpolation. `projection` owns launch-resolved caller values;
/// this stage verifies their selected file mode is still stable before it
/// mutates frontmatter for validation.
pub(crate) fn run_with_registry(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    prepared: &PreparedSchemas,
    projection: &CallerProjection,
) -> MarkdownResult<()> {
    let Some(schemas) = prepared.schemas.as_ref() else {
        return Ok(());
    };

    let path = source_path(markdown, options);
    let description = markdown
        .frontmatter()
        .as_map()
        .get("description")
        .and_then(|v| v.as_str().map(String::from));

    // Build the effective schema once; the same instance is reused for the
    // eager-`file` rewrite pass after validation accepts the instance.
    // `effective_for` returns an owned schema (no borrow of `markdown`), so it
    // can outlive the frontmatter mutations below.
    let effective = match schemas.effective_for(markdown) {
        Ok(effective) => effective,
        // A schema that cannot even be prepared is still not this pass's verdict
        // to report when the caller deferred it; coercion simply has nothing to
        // work from.
        Err(_) if options.defer_schema_verdict => return Ok(()),
        Err(err) => {
            return Err(MarkdownError::SchemaValidationFailed {
                path: path.clone(),
                problems: Vec::new(),
                summary: format!("schema could not be prepared: {err}"),
                description: description.clone(),
                source: Some(Box::new(err)),
            });
        }
    };

    ensure_projection_stable(effective.as_ref(), markdown, options, projection)?;

    if let Some(effective) = effective.as_ref() {
        materialize_optional_document_bindings(markdown, effective);
    }

    if let Some(effective) = effective.as_ref() {
        // Coerce schema-recognized scalars to their declared types and write the
        // coerced top-level properties back, so real types flow to every later
        // stage and into the composed output.
        let (instance, composition_pending) = build_validation_instance(markdown, options);
        let outcome =
            coerce_frontmatter_with_pending(&effective.json_schema, &instance, &composition_pending);
        if outcome.changed
            && let serde_json::Value::Object(coerced) = outcome.value
        {
            let fm_map = markdown.frontmatter_mut().as_map_mut();
            for (key, value) in coerced {
                // A value still holding `$(...)` or an unresolved `{{ ... }}`
                // template is resolved (and coerced) later at post-shell
                // re-validation; writing back here would clobber the literal
                // form shell expansion and template re-evaluation must consume.
                if composition_pending.contains(&key) {
                    continue;
                }
                fm_map.insert(key, value);
            }
        }
    }

    // Coercion above is unconditional; the verdict is not. A pass that does not
    // own the verdict stops here, before a validator is even built: a schema
    // that cannot be *prepared* is likewise not this pass's failure to report.
    if options.defer_schema_verdict {
        return Ok(());
    }

    let report = schemas.validate(markdown).map_err(|err| {
        MarkdownError::SchemaValidationFailed {
            path: path.clone(),
            problems: Vec::new(),
            summary: format!("schema could not be prepared: {err}"),
            description: description.clone(),
            source: Some(Box::new(err)),
        }
    })?;

    // Defer problems whose value will be re-resolved later in composition. The
    // compose-time validator runs after template interpolation but BEFORE shell
    // expansion, so values that depend on `$(...)` expressions still hold their
    // literal form here, and values whose `{{ ... }}` template could not resolve
    // yet (because it transitively references such a `$(...)` value) still hold
    // the unresolved template. The downstream consumer (e.g. claudine's
    // `prepare_*_with_schema`) re-validates the post-shell effective frontmatter
    // and reports any residual problems. See the rationale at
    // `compose::run::compose` where this stage is invoked.
    //
    // Deferral is only sound when shell expansion will actually run. When
    // `FrontmatterShellExpansion` is disabled, no later stage expands or
    // re-validates these pending values, so deferring would silently accept a
    // schema violation. In that case every problem is final and must be
    // reported here.
    // Deferral is sound when a later stage will expand and re-validate the
    // `$(...)` value: either this pass runs frontmatter shell expansion, or the
    // caller is a discovery pass that strips it but promises a terminal compose
    // pass will re-validate (`defer_shell_pending_schema_problems`).
    let defer_shell_pending = options.is_enabled(ComposeOperation::FrontmatterShellExpansion)
        || options.defer_shell_pending_schema_problems;
    let fm_map = markdown.frontmatter().as_map();
    let composition_independent: Vec<_> = report
        .problems
        .iter()
        .filter(|p| {
            // DM1b: problems referencing a deferred (excluded) key are not
            // user-schema violations — the caller owns those keys' validation.
            if let Some(name) = top_level_pointer_segment(&p.path)
                && options.exclude_keys.contains(&name)
            {
                return false;
            }
            // Caller-originated files were materialized against their
            // invocation-fixed launch context above. A target document may
            // live across a repository boundary, where rechecking that
            // absolute value as document-authored can only produce a
            // contradictory provenance failure.
            if matches!(
                p.code,
                crate::markdown::schemas::ValidationProblemCode::InvalidFileReference
            ) && top_level_pointer_segment(&p.path)
                .is_some_and(|name| projection.native.contains_key(&name))
            {
                return false;
            }
            if !defer_shell_pending {
                return true;
            }
            let Some(name) = top_level_pointer_segment(&p.path) else {
                return true;
            };
            !value_pending_composition(fm_map.get(&name))
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

    // Eager-`file` normalization (Decision #4: "a document with final
    // validation problems is never rewritten"). We only reach here when the
    // composition-independent gate passed, so it is safe to rewrite present
    // document-authored eager-`file` values to resolved repo-relative paths.
    // Caller-originated overrides are restored as absolute native paths so
    // later consumers retain their launch-area provenance. The rewrite shares
    // the same instance-construction loop as coercion, so `$schema` and
    // `options.exclude_keys` stay outside the write-back; pending keys are
    // skipped the same way.
    if let Some(effective) = effective.as_ref() {
        let (instance, composition_pending) = build_validation_instance(markdown, options);
        let outcome = effective.normalize_frontmatter(&instance, &composition_pending);
        if outcome.changed
            && let serde_json::Value::Object(rewritten) = outcome.value
        {
            let fm_map = markdown.frontmatter_mut().as_map_mut();
            for (key, value) in rewritten {
                if composition_pending.contains(&key) {
                    continue;
                }
                let value = projection
                    .native
                    .get(&key)
                    .cloned()
                    .unwrap_or(value);
                fm_map.insert(key, value);
            }
        }
    }

    Ok(())
}

/// Rechecks only caller-file classification against the current effective
/// schema, without taking ownership of validation or frontmatter mutation.
pub(crate) fn verify_projection_stability(
    markdown: &Markdown,
    options: &ComposeOptions,
    prepared: &PreparedSchemas,
    projection: &CallerProjection,
) -> MarkdownResult<()> {
    let Some(schemas) = prepared.schemas.as_ref() else {
        return Ok(());
    };
    let effective = match schemas.effective_for(markdown) {
        Ok(effective) => effective,
        Err(_) if options.defer_schema_verdict => return Ok(()),
        Err(err) => {
            return Err(MarkdownError::SchemaValidationFailed {
                path: source_path(markdown, options),
                problems: Vec::new(),
                summary: format!("schema could not be prepared: {err}"),
                description: markdown
                    .frontmatter()
                    .as_map()
                    .get("description")
                    .and_then(|value| value.as_str().map(String::from)),
                source: Some(Box::new(err)),
            });
        }
    };
    ensure_projection_stable(effective.as_ref(), markdown, options, projection)
}

/// Adds composition bindings for eligible document-owned schema properties.
///
/// Eligibility comes from the resolved SimplifiedSchema AST and origin map,
/// not from nullable JSON Schema shape. This keeps raw JSON Schema, root
/// unions, nested properties, baseline properties, and trigger payloads out of
/// the mutation boundary. Presence wins over synthesis, including explicit
/// `null` and other falsey values; repeated schema passes are therefore
/// idempotent.
fn materialize_optional_document_bindings(markdown: &mut Markdown, effective: &EffectiveSchema) {
    let Some(SimplifiedSchema::Single(shape)) = effective.simplified.as_ref() else {
        return;
    };

    let eligible = shape.properties.iter().filter_map(|(name, definition)| {
        let origin = effective.origins.get(name)?;
        let document_owned = matches!(
            origin.kind,
            SchemaOriginKind::Document | SchemaOriginKind::ReferencedFile
        );
        (document_owned && !definition.is_required() && !definition.has_default())
            .then(|| name.clone())
    });
    let fm_map = markdown.frontmatter_mut().as_map_mut();
    for name in eligible {
        fm_map.entry(name).or_insert(serde_json::Value::Null);
    }
}

pub(crate) fn prepare_caller_projection(
    markdown: &Markdown,
    options: &ComposeOptions,
    prepared: &PreparedSchemas,
) -> MarkdownResult<CallerProjection> {
    let Some(schemas) = prepared.schemas.as_ref() else {
        return Ok(CallerProjection::default());
    };
    let effective = match schemas.effective_for(markdown) {
        Ok(effective) => effective,
        Err(_) if options.defer_schema_verdict => None,
        Err(err) => {
            return Err(MarkdownError::SchemaValidationFailed {
                path: source_path(markdown, options),
                problems: Vec::new(),
                summary: format!("schema could not be prepared: {err}"),
                description: markdown
                    .frontmatter()
                    .as_map()
                    .get("description")
                    .and_then(|value| value.as_str().map(String::from)),
                source: Some(Box::new(err)),
            });
        }
    };
    let (instance, composition_pending) = caller_classification_instance(markdown, options);
    resolve_caller_file_overrides(
        effective.as_ref(),
        options,
        &instance,
        &composition_pending,
    )
    .map_err(|failure| {
        MarkdownError::SchemaValidationFailed {
            path: source_path(markdown, options),
            problems: vec![*failure.problem],
            summary: "frontmatter did not satisfy the schema".to_string(),
            description: markdown
                .frontmatter()
                .as_map()
                .get("description")
                .and_then(|value| value.as_str().map(String::from)),
            source: None,
        }
    })
}

fn ensure_projection_stable(
    effective: Option<&EffectiveSchema>,
    markdown: &Markdown,
    options: &ComposeOptions,
    projection: &CallerProjection,
) -> MarkdownResult<()> {
    let (instance, composition_pending) = caller_classification_instance(markdown, options);
    let current = classify_caller_overrides(effective, options, &instance, &composition_pending);
    for key in &projection.classified {
        if projection.modes.get(key) != current.get(key) {
            return Err(MarkdownError::CallerFileClassificationChanged {
                property: key.clone(),
            });
        }
    }
    Ok(())
}

fn caller_classification_instance(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> (serde_json::Value, HashSet<String>) {
    let (mut instance, mut composition_pending) = build_validation_instance(markdown, options);
    let Some(object) = instance.as_object_mut() else {
        return (instance, composition_pending);
    };
    for (key, record) in caller_input_records(options) {
        if !options.exclude_keys.contains(&key) {
            if value_pending_composition(Some(record.raw())) {
                composition_pending.insert(key.clone());
            } else {
                composition_pending.remove(&key);
            }
            object.insert(key, record.raw().clone());
        }
    }
    (instance, composition_pending)
}

/// Resolve schema-selected file values from immutable caller records before
/// frontmatter interpolation.
///
/// A value authored in the document resolves from the document. A value passed
/// by a caller is a different provenance class: it resolves from the caller's
/// captured base and remains absolute in the effective frontmatter so later
/// consumers do not reinterpret it as document-authored. The schema tells us
/// which override strings are file references; ordinary string overrides are
/// never probed.
fn resolve_caller_file_overrides(
    effective: Option<&crate::markdown::schemas::EffectiveSchema>,
    options: &ComposeOptions,
    instance: &serde_json::Value,
    composition_pending: &HashSet<String>,
) -> Result<CallerProjection, CallerProjectionFailure> {
    let records = caller_input_records(options);
    let mut resolved = CallerProjection::default();
    resolved.classified.extend(
        records
            .keys()
            .filter(|key| !options.exclude_keys.contains(*key))
            .cloned(),
    );
    let document_context = options.local_expression_resolution_context();
    let Some(fragments_by_property) = effective.and_then(|effective| {
        collect_applicable_root_schema_fragments(
            &effective.json_schema,
            instance,
            &document_context,
            &records,
            composition_pending,
        )
    }) else {
        return Ok(resolved);
    };
    for (key, record) in &records {
        if options.exclude_keys.contains(key) {
            continue;
        }
        let Some(fragments) = fragments_by_property.get(key) else {
            continue;
        };
        let selections: Vec<_> = fragments
            .iter()
            .filter_map(|fragment| select_file_mode(record.raw(), fragment, record.origin()))
            .collect();
        if selections.len() != 1 {
            continue;
        }
        let mode = selections[0];
        resolved.modes.insert(key.clone(), mode);
        for fragment in fragments {
            if select_file_mode(record.raw(), fragment, record.origin()) != Some(mode) {
                continue;
            }
            if let Some(value) = resolve_caller_file_value(
                record.raw(),
                fragment,
                record.origin(),
                key,
                &caller_occurrence(key),
                mode,
            )? {
                resolved.native.insert(key.clone(), value.native);
                resolved.presentation.insert(key.clone(), value.presentation);
                for provenance in value.provenance {
                    resolved.provenance.insert(provenance.occurrence, provenance.value);
                }
                break;
            }
        }
    }
    Ok(resolved)
}

fn classify_caller_overrides(
    effective: Option<&EffectiveSchema>,
    options: &ComposeOptions,
    instance: &serde_json::Value,
    composition_pending: &HashSet<String>,
) -> HashMap<String, CallerFileMode> {
    let Some(effective) = effective else {
        return HashMap::new();
    };
    let records = caller_input_records(options);
    let document_context = options.local_expression_resolution_context();
    let Some(fragments_by_property) = collect_applicable_root_schema_fragments(
        &effective.json_schema,
        instance,
        &document_context,
        &records,
        composition_pending,
    ) else {
        return HashMap::new();
    };
    records
        .iter()
        .filter_map(|(key, record)| {
            if options.exclude_keys.contains(key) {
                return None;
            }
            let fragments = fragments_by_property.get(key)?;
            let modes: Vec<_> = fragments
                .iter()
                .filter_map(|fragment| select_file_mode(record.raw(), fragment, record.origin()))
                .collect();
            (modes.len() == 1).then(|| (key.clone(), modes[0]))
        })
        .collect()
}

fn caller_input_records(options: &ComposeOptions) -> crate::markdown::compose::CallerInputRecords {
    if !options.caller_input_records().is_empty() {
        return options.caller_input_records().clone();
    }
    let Some(overrides) = options.set_overrides.as_ref().and_then(serde_json::Value::as_object)
    else {
        return Default::default();
    };
    let origin = match (
        options.file_resolution_context.as_ref(),
        options.file_ref_fallback_dir.as_ref(),
    ) {
        (Some(context), Some(fallback)) => context.for_trusted_external_base(fallback),
        (Some(context), None) => {
            context.for_trusted_external_base(context.request_base_dir())
        }
        (None, Some(fallback)) => biscuit_file::FileResolutionContext::new(fallback),
        (None, None) => return Default::default(),
    };
    overrides
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                crate::markdown::compose::CallerInputRecord::new(value.clone(), origin.clone()),
            )
        })
        .collect()
}

fn select_file_mode(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    context: &biscuit_file::FileResolutionContext,
) -> Option<CallerFileMode> {
    let mode = match schema.get("format").and_then(serde_json::Value::as_str) {
        Some(crate::markdown::schemas::format::DARKMATTER_FILE_FORMAT) => {
            Some(CallerFileMode::Eager)
        }
        Some(crate::markdown::schemas::format::DARKMATTER_FILE_REFERENCE_FORMAT) => {
            Some(CallerFileMode::Lazy)
        }
        _ => None,
    };
    if mode.is_some() {
        return value.is_string().then_some(mode).flatten();
    }
    if let Some(items) = schema.get("items") && let Some(values) = value.as_array() {
        let modes: Vec<_> = values
            .iter()
            .filter_map(|item| select_file_mode(item, items, context))
            .collect();
        if !values.is_empty() && modes.len() == values.len() && modes.iter().all(|m| *m == modes[0])
        {
            return Some(modes[0]);
        }
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(arms) = schema.get(combinator).and_then(serde_json::Value::as_array) {
            let applicable: Vec<_> = arms
                .iter()
                .filter(|arm| {
                    crate::markdown::schemas::validate::build_validator_in_context(
                        arm,
                        Some(context.base_dir()),
                        None,
                        Some(context),
                    )
                    .is_ok_and(|validator| validator.is_valid(value))
                })
                .collect();
            if applicable.len() != 1 {
                return None;
            }
            return select_file_mode(value, applicable[0], context);
        }
    }
    None
}

fn collect_applicable_root_schema_fragments<'a>(
    schema: &'a serde_json::Value,
    instance: &serde_json::Value,
    document_context: &super::expression::ResolutionContext,
    records: &crate::markdown::compose::CallerInputRecords,
    composition_pending: &HashSet<String>,
) -> Option<HashMap<String, Vec<&'a serde_json::Value>>> {
    let mut selected: HashMap<String, Vec<&serde_json::Value>> = schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .map(|(key, fragment)| (key.clone(), vec![fragment]))
                .collect()
        })
        .unwrap_or_default();

    if let Some(arms) = schema.get("allOf").and_then(serde_json::Value::as_array) {
        for arm in arms {
            let arm_fragments = collect_applicable_root_schema_fragments(
                arm,
                instance,
                document_context,
                records,
                composition_pending,
            )?;
            if root_schema_arm_applies(
                arm,
                instance,
                document_context,
                records,
                composition_pending,
                &arm_fragments,
            ) == RootArmApplicability::None
            {
                return None;
            }
            for (key, fragments) in arm_fragments {
                selected.entry(key).or_default().extend(fragments);
            }
        }
    }

    for combinator in ["anyOf", "oneOf"] {
        let Some(arms) = schema.get(combinator).and_then(serde_json::Value::as_array) else {
            continue;
        };
        let candidates: Vec<_> = arms
            .iter()
            .filter_map(|arm| {
                let fragments = collect_applicable_root_schema_fragments(
                    arm,
                    instance,
                    document_context,
                    records,
                    composition_pending,
                )?;
                let applicability = root_schema_arm_applies(
                    arm,
                    instance,
                    document_context,
                    records,
                    composition_pending,
                    &fragments,
                );
                (applicability != RootArmApplicability::None)
                    .then_some((applicability, fragments))
            })
            .collect();
        let exact: Vec<_> = candidates
            .iter()
            .filter(|(applicability, _)| *applicability == RootArmApplicability::Exact)
            .collect();
        let selected_arm = match exact.as_slice() {
            [(_, fragments)] => fragments,
            [] => {
                let pending: Vec<_> = candidates
                    .iter()
                    .filter(|(applicability, _)| {
                        *applicability == RootArmApplicability::Pending
                    })
                    .collect();
                let [(_, fragments)] = pending.as_slice() else {
                    return None;
                };
                fragments
            }
            _ => return None,
        };
        for (key, fragments) in selected_arm.iter() {
            selected
                .entry(key.clone())
                .or_default()
                .extend(fragments.iter().copied());
        }
    }

    Some(selected)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootArmApplicability {
    Exact,
    Pending,
    None,
}

fn root_schema_arm_applies(
    arm: &serde_json::Value,
    instance: &serde_json::Value,
    document_context: &super::expression::ResolutionContext,
    records: &crate::markdown::compose::CallerInputRecords,
    composition_pending: &HashSet<String>,
    fragments_by_property: &HashMap<String, Vec<&serde_json::Value>>,
) -> RootArmApplicability {
    let mut candidate = coerce_frontmatter_with_pending(arm, instance, composition_pending).value;
    if !project_root_arm_caller_values(&mut candidate, records, fragments_by_property) {
        return RootArmApplicability::None;
    }
    let wrapped = crate::markdown::schemas::validate::wrap_arm_as_root_schema(arm);
    let Ok(validator) = crate::markdown::schemas::validate::build_validator_in_context(
        &wrapped,
        Some(&document_context.base_dir),
        None,
        document_context.file_resolution_context.as_ref(),
    ) else {
        return RootArmApplicability::None;
    };
    if validator.is_valid(&candidate) {
        return RootArmApplicability::Exact;
    }
    if composition_pending.is_empty() {
        return RootArmApplicability::None;
    }
    if validator.iter_errors(&candidate).all(|error| {
        error
            .instance_path()
            .iter()
            .next()
            .is_some_and(|segment| match segment {
                jsonschema::paths::LocationSegment::Property(property) => {
                    composition_pending.contains(property.as_ref())
                }
                jsonschema::paths::LocationSegment::Index(_) => false,
            })
    }) {
        RootArmApplicability::Pending
    } else {
        RootArmApplicability::None
    }
}

fn project_root_arm_caller_values(
    candidate: &mut serde_json::Value,
    records: &crate::markdown::compose::CallerInputRecords,
    fragments_by_property: &HashMap<String, Vec<&serde_json::Value>>,
) -> bool {
    let Some(object) = candidate.as_object_mut() else {
        return false;
    };
    for (key, record) in records {
        let Some(fragments) = fragments_by_property.get(key) else {
            continue;
        };
        let modes: Vec<_> = fragments
            .iter()
            .filter_map(|fragment| select_file_mode(record.raw(), fragment, record.origin()))
            .collect();
        if modes.len() != 1 {
            continue;
        }
        let mode = modes[0];
        let projected = fragments.iter().find_map(|fragment| {
            if select_file_mode(record.raw(), fragment, record.origin()) != Some(mode) {
                return None;
            }
            match resolve_caller_file_value(
                record.raw(),
                fragment,
                record.origin(),
                key,
                &caller_occurrence(key),
                mode,
            ) {
                Ok(Some(projected)) => Some(Ok(projected)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            }
        });
        match projected {
            Some(Ok(projected)) => {
                object.insert(key.clone(), projected.native);
            }
            Some(Err(_)) => return false,
            None => {}
        }
    }
    true
}

struct CallerProjectionFailure {
    problem: Box<ValidationProblem>,
}

struct ResolvedCallerFileValue {
    native: serde_json::Value,
    presentation: serde_json::Value,
    provenance: Vec<CallerFileOccurrenceProvenance>,
}

struct CallerFileOccurrenceProvenance {
    occurrence: String,
    value: super::expression::resolve_ctx::CallerFileProvenance,
}

fn caller_occurrence(property: &str) -> String {
    format!("/{}", property.replace('~', "~0").replace('/', "~1"))
}

fn resolve_caller_file_value(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    context: &biscuit_file::FileResolutionContext,
    key: &str,
    occurrence: &str,
    mode: CallerFileMode,
) -> Result<Option<ResolvedCallerFileValue>, CallerProjectionFailure> {
    let failure = |message, diagnostic, candidate| {
        caller_projection_failure(key, message, diagnostic, context, candidate)
    };
    if select_file_mode(value, schema, context) == Some(mode)
        && schema.get("format").and_then(serde_json::Value::as_str).is_some()
    {
        let Some(raw) = value.as_str() else {
            return Ok(None);
        };
        let reference = biscuit_file::FileReference::new(raw).map_err(|err| {
            failure(
                format!("`{raw}` is not a valid file reference: {err}"),
                FileReferenceDiagnostic::InvalidSyntax {
                    raw: raw.to_string(),
                },
                None,
            )
        })?;
        if mode == CallerFileMode::Lazy {
            if reference.class().recursive {
                return Err(failure(
                    format!(
                        "recursive caller file reference `{raw}` has no single lazy identity; declare the parameter as `file(eager)`"
                    ),
                    FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                    None,
                ));
            }
            if reference.class().kind == biscuit_file::FileReferenceKind::Url {
                let target = reference.resolve_target().map_err(|err| {
                    failure(
                        format!("could not classify remote file reference `{raw}`: {err}"),
                        FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                        None,
                    )
                })?;
                let Some(biscuit_file::Resolved::Remote(url)) = target else {
                    return Err(failure(
                        format!("remote file reference `{raw}` did not produce a remote target"),
                        FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                        None,
                    ));
                };
                let value = serde_json::Value::String(url.to_string());
                return Ok(Some(ResolvedCallerFileValue {
                    native: value.clone(),
                    presentation: value,
                    provenance: Vec::new(),
                }));
            }
            let candidates = reference
                .candidate_plan_with_order(
                    context,
                    biscuit_file::CandidatePlanOrder::AuthoringBaseFirst,
                )
                .map_err(|err| {
                    failure(
                        format!(
                            "could not bind file reference `{raw}` from `{}`: {err}",
                            context.base_dir().display()
                        ),
                        FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                        None,
                    )
                })?;
            let candidate = candidates.first().ok_or_else(|| {
                failure(
                    format!(
                        "file reference `{raw}` produced no candidate from `{}`",
                        context.base_dir().display()
                    ),
                    FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                    None,
                )
            })?;
            let path = candidate.path().to_path_buf();
            if !path.is_absolute() {
                return Err(failure(
                    format!(
                        "file reference `{raw}` produced a non-absolute candidate from `{}`",
                        context.base_dir().display(),
                    ),
                    FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                    Some(candidate.clone()),
                ));
            }
            let native = path.to_string_lossy().into_owned();
            let presentation = biscuit_file::to_portable_string(&path);
            return Ok(Some(ResolvedCallerFileValue {
                native: serde_json::Value::String(native),
                presentation: serde_json::Value::String(presentation),
                provenance: vec![CallerFileOccurrenceProvenance {
                    occurrence: occurrence.to_string(),
                    value: super::expression::resolve_ctx::CallerFileProvenance {
                        property: key.to_string(),
                        reference: raw.to_string(),
                        origin: context.clone(),
                        candidate: path,
                        candidate_provenance: candidate.provenance(),
                    },
                }],
            }));
        }
        let detailed = reference.resolve_detailed(context);
        let resolved = detailed.matched_path().map(Path::to_path_buf).ok_or_else(|| {
            if let Some(err) = detailed.error() {
                return failure(
                    format!(
                        "could not resolve file reference `{raw}` from `{}`: {err}",
                        context.base_dir().display()
                    ),
                    FileReferenceDiagnostic::ResolutionFailed { raw: raw.to_string() },
                    detailed
                        .candidates()
                        .first()
                        .map(|candidate| candidate.candidate().clone()),
                );
            }
            failure(
                format!(
                    "file reference `{raw}` did not match a file from `{}`",
                    context.base_dir().display()
                ),
                FileReferenceDiagnostic::NoMatch {
                    raw: raw.to_string(),
                    resolved_from: Some(context.base_dir().to_path_buf()),
                },
                detailed
                    .candidates()
                    .first()
                    .map(|candidate| candidate.candidate().clone()),
            )
        })?;
        let candidate_provenance = detailed
            .candidates()
            .iter()
            .find(|candidate| candidate.candidate().path() == resolved)
            .map(|candidate| candidate.candidate().provenance())
            .unwrap_or(biscuit_file::RootProvenance::Absolute);
        let native = resolved.to_string_lossy().into_owned();
        let presentation = biscuit_file::to_portable_string(&resolved);
        return Ok(Some(ResolvedCallerFileValue {
            native: serde_json::Value::String(native),
            presentation: serde_json::Value::String(presentation),
            provenance: vec![CallerFileOccurrenceProvenance {
                occurrence: occurrence.to_string(),
                value: super::expression::resolve_ctx::CallerFileProvenance {
                    property: key.to_string(),
                    reference: raw.to_string(),
                    origin: context.clone(),
                    candidate: resolved,
                    candidate_provenance,
                },
            }],
        }));
    }
    if let Some(items) = schema.get("items")
        && let Some(values) = value.as_array()
    {
        let mut changed = false;
        let mut native = Vec::with_capacity(values.len());
        let mut presentation = Vec::with_capacity(values.len());
        let mut provenance = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let item_occurrence = format!("{occurrence}/{index}");
            if let Some(resolved) = resolve_caller_file_value(
                value,
                items,
                context,
                key,
                &item_occurrence,
                mode,
            )? {
                changed = true;
                native.push(resolved.native);
                presentation.push(resolved.presentation);
                provenance.extend(resolved.provenance);
            } else {
                native.push(value.clone());
                presentation.push(value.clone());
            }
        }
        if changed {
            return Ok(Some(ResolvedCallerFileValue {
                native: serde_json::Value::Array(native),
                presentation: serde_json::Value::Array(presentation),
                provenance,
            }));
        }
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(arms) = schema.get(combinator).and_then(serde_json::Value::as_array) {
            for arm in arms {
                if let Some(resolved) =
                    resolve_caller_file_value(value, arm, context, key, occurrence, mode)?
                {
                    return Ok(Some(resolved));
                }
            }
        }
    }
    Ok(None)
}

fn caller_projection_failure(
    key: &str,
    message: String,
    diagnostic: FileReferenceDiagnostic,
    origin: &biscuit_file::FileResolutionContext,
    candidate: Option<biscuit_file::ResolutionCandidate>,
) -> CallerProjectionFailure {
    let pointer = format!("/{}", key.replace('~', "~0").replace('/', "~1"));
    CallerProjectionFailure {
        problem: Box::new(ValidationProblem {
            path: pointer.clone(),
            message,
            kind: ValidationProblemKind::Invalid,
            property: None,
            line: None,
            column: None,
            arm_index: None,
            description: None,
            code: ValidationProblemCode::InvalidFileReference,
            instance_path: JsonPointer::parse(&pointer),
            schema_path: None,
            offending_property: None,
            file_reference: Some(diagnostic),
            caller_file: Some(CallerFileReferenceProvenance {
                origin: origin.clone(),
                candidate,
            }),
        }),
    }
}

/// Returns `true` when `value` is still composition-pending — it holds a
/// frontmatter shell expression (`$(...)`) or an unresolved Darkmatter
/// template (`{{ ... }}`) somewhere in any string descendant.
///
/// This stage runs after template interpolation but before shell expansion.
/// A value that survives interpolation still holding `{{ ... }}` could not be
/// evaluated yet — typically because it transitively depends on a `$(...)`
/// value, which interpolation leaves in literal form (e.g. `possible_spec:
/// "{{dir}}/spec.md"` where `dir` is `$(...)`). Such templated values, like
/// their `$(...)` counterparts, must not be failed here: the consumer
/// re-validates the post-shell effective frontmatter once every expression
/// has resolved.
fn value_pending_composition(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else { return false };
    match value {
        serde_json::Value::String(s) => s.contains("$(") || s.contains("{{"),
        serde_json::Value::Array(items) => items.iter().any(|v| value_pending_composition(Some(v))),
        serde_json::Value::Object(map) => map.values().any(|v| value_pending_composition(Some(v))),
        _ => false,
    }
}

/// Builds the validation instance and the composition-pending key set from
/// the document's current frontmatter.
///
/// Excludes `$schema` (a Darkmatter control key, not document data — this
/// must match `schemas::frontmatter_as_json`) and every key in
/// `options.exclude_keys` (DM1b: caller-owned control keys). Both the
/// coercion pass and the eager-`file` rewrite pass consume this same
/// instance construction so the two never diverge on which keys are in
/// scope.
fn build_validation_instance(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> (serde_json::Value, std::collections::HashSet<String>) {
    let fm_map = markdown.frontmatter().as_map();
    let mut object = serde_json::Map::with_capacity(fm_map.len());
    let mut composition_pending = std::collections::HashSet::new();
    for (key, value) in fm_map {
        if key == "$schema" {
            continue;
        }
        if options.exclude_keys.contains(key) {
            continue;
        }
        if value_pending_composition(Some(value)) {
            composition_pending.insert(key.clone());
        }
        object.insert(key.clone(), value.clone());
    }
    (serde_json::Value::Object(object), composition_pending)
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
        Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr,
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
        SimplifiedSchema::Single(SchemaShape {
            properties,
            ..Default::default()
        })
    }

    // ── No-op cases ───────────────────────────────────────────────────

    #[test]
    fn no_schema_no_baseline_is_no_op() {
        let mut md = md_with_schema("name: alice\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
    }

    #[test]
    fn trigger_discovery_reuses_request_repository_boundary() {
        let request_repo = tempfile::tempdir().unwrap();
        let nested_repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(request_repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(nested_repo.path().join(".git")).unwrap();
        let document_path = nested_repo.path().join("prompt.md");
        let snapshot = biscuit_file::FileResolutionContext::new(request_repo.path())
            .with_repository_root(request_repo.path());
        let options = ComposeOptions::new().with_file_resolution_context(snapshot);

        assert_eq!(
            trigger_discovery_boundary(&options, &document_path).as_deref(),
            Some(request_repo.path()),
        );
        assert_eq!(
            trigger_discovery_boundary(&ComposeOptions::new(), &document_path).as_deref(),
            Some(nested_repo.path()),
        );
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

    // ── Deferred verdict ──────────────────────────────────────────────

    #[test]
    fn deferred_verdict_reports_no_problem() {
        let mut md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let options = ComposeOptions::new().with_deferred_schema_verdict(true);
        assert!(
            run(&mut md, &options).is_ok(),
            "a pass that does not own the verdict reports no violation"
        );
    }

    #[test]
    fn deferred_verdict_still_coerces() {
        let mut md = md_with_schema("$schema:\n  enabled: boolean\nenabled: \"true\"\n");
        let options = ComposeOptions::new().with_deferred_schema_verdict(true);
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("enabled"),
            Some(&serde_json::json!(true)),
            "coercion is unconditional; only the verdict is withheld"
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
    fn document_optional_bindings_materialize_before_coercion_and_are_idempotent() {
        let mut md = md_with_schema("$schema:\n  first: string\n  count: number\n");
        let options = ComposeOptions::new();
        let effective = DarkmatterSchemas::new()
            .effective_for(&md)
            .unwrap()
            .unwrap();
        let expected_order = match effective.simplified.as_ref().unwrap() {
            SimplifiedSchema::Single(shape) => {
                shape.properties.keys().map(String::as_str).collect::<Vec<_>>()
            }
            SimplifiedSchema::Union(_) => panic!("expected a single schema shape"),
        };

        assert!(run(&mut md, &options).is_ok());
        let first_pass = md.frontmatter().as_map().clone();
        assert_eq!(first_pass.get("first"), Some(&serde_json::Value::Null));
        assert_eq!(first_pass.get("count"), Some(&serde_json::Value::Null));
        assert_eq!(
            first_pass
                .keys()
                .filter(|key| matches!(key.as_str(), "first" | "count"))
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected_order,
            "bindings follow resolved SimplifiedSchema declaration order"
        );

        assert!(run(&mut md, &options).is_ok());
        assert_eq!(md.frontmatter().as_map(), &first_pass);
    }

    #[test]
    fn referenced_document_optional_binding_materializes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("schema.yaml"), "$schema:\n  optional: string\n")
            .unwrap();
        let doc_path = dir.path().join("doc.md");
        let mut md = md_with_schema_and_source("$schema: ./schema.yaml\n", &doc_path);
        let options = ComposeOptions::new().with_source_file(&doc_path);

        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("optional"),
            Some(&serde_json::Value::Null)
        );
    }

    #[test]
    fn present_optional_values_are_preserved() {
        let cases = [
            ("null", serde_json::Value::Null),
            ("''", serde_json::json!("")),
            ("false", serde_json::json!(false)),
            ("0", serde_json::json!(0)),
            ("[]", serde_json::json!([])),
            ("{}", serde_json::json!({})),
        ];

        for (yaml, expected) in cases {
            let mut md = md_with_schema(&format!("$schema:\n  value: any\nvalue: {yaml}\n"));
            assert!(run(&mut md, &ComposeOptions::new()).is_ok(), "value: {yaml}");
            assert_eq!(md.frontmatter().as_map().get("value"), Some(&expected));
        }
    }

    #[test]
    fn non_document_owned_and_nested_properties_do_not_materialize() {
        let mut baseline_only = md_with_schema("title: hi\n");
        let baseline = SimplifiedSchema::Single({
            let mut shape = SchemaShape::new();
            shape.properties.insert(
                "baseline_optional".into(),
                PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String)),
            );
            shape
        });
        let options = ComposeOptions::new().with_baseline_schema(baseline);
        assert!(run(&mut baseline_only, &options).is_ok());
        assert!(
            !baseline_only
                .frontmatter()
                .as_map()
                .contains_key("baseline_optional")
        );

        let mut root_union = md_with_schema(
            "$schema:\n  - first: string\n  - second: string\nkind: existing\n",
        );
        assert!(run(&mut root_union, &ComposeOptions::new()).is_ok());
        assert!(!root_union.frontmatter().as_map().contains_key("first"));
        assert!(!root_union.frontmatter().as_map().contains_key("second"));

        let mut nested = md_with_schema("$schema:\n  config:\n    child: string\nconfig: {}\n");
        assert!(run(&mut nested, &ComposeOptions::new()).is_ok());
        assert_eq!(
            nested.frontmatter().as_map().get("config"),
            Some(&serde_json::json!({}))
        );
    }

    #[test]
    fn defaulted_document_property_does_not_materialize() {
        let mut md = md_with_schema(
            "$schema:\n  mode:\n    - 'string(default(auto))'\n    - 'literal(auto; default(auto))'\n",
        );

        assert!(run(&mut md, &ComposeOptions::new()).is_ok());
        assert!(!md.frontmatter().as_map().contains_key("mode"));
    }

    #[test]
    fn raw_json_schema_property_does_not_materialize() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("schema.json"),
            r#"{"type":"object","properties":{"optional":{"type":["string","null"]}}}"#,
        )
        .unwrap();
        let doc_path = dir.path().join("doc.md");
        let mut md = md_with_schema_and_source("$schema: ./schema.json\n", &doc_path);
        let options = ComposeOptions::new().with_source_file(&doc_path);

        assert!(run(&mut md, &options).is_ok());
        assert!(!md.frontmatter().as_map().contains_key("optional"));
    }

    #[test]
    fn trigger_payload_property_does_not_materialize() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(repo.path().join("schemas")).unwrap();
        std::fs::write(
            repo.path().join("schemas/test.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: payload.yaml\n",
        )
        .unwrap();
        std::fs::write(
            repo.path().join("schemas/payload.yaml"),
            "$schema:\n  trigger_optional: string\n",
        )
        .unwrap();
        let doc_path = repo.path().join("doc.md");
        let mut md = md_with_schema_and_source("prompt: hello\n", &doc_path);
        let options = ComposeOptions::new()
            .with_source_file(&doc_path)
            .with_trigger_schemas(true);

        assert!(run(&mut md, &options).is_ok());
        assert!(!md.frontmatter().as_map().contains_key("trigger_optional"));
    }

    #[test]
    fn normal_darkmatter_baseline_does_not_flood_frontmatter() {
        let mut md = md_with_schema("$schema:\n  optional: string\ntitle: hi\n");
        let options = ComposeOptions::new().with_darkmatter_baseline_schema();

        assert!(run(&mut md, &options).is_ok());
        let map = md.frontmatter().as_map();
        assert_eq!(map.get("optional"), Some(&serde_json::Value::Null));
        assert_eq!(
            map.len(),
            3,
            "only $schema, title, and the document binding remain"
        );
    }

    #[test]
    fn document_schema_missing_required_fails() {
        let mut md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let options = ComposeOptions::new();
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, summary, .. } => {
                assert!(!problems.is_empty());
                assert_eq!(summary, "frontmatter did not satisfy the schema");
                assert!(problems.iter().any(|p| p.property.as_deref() == Some("title")));
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
        assert!(!md.frontmatter().as_map().contains_key("title"));
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
    fn write_back_serializes_native_value_for_yaml_content_format() {
        // A native mapping under a `yaml` content-format field is serialized to
        // its YAML string and written back (the write-back is the only surface
        // that exposes the serialized form; validation-only stays non-mutating).
        let mut md =
            md_with_schema("$schema:\n  frontmatter: yaml\nfrontmatter:\n  title: Foo\n  n: 1\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        let stored = md.frontmatter().as_map().get("frontmatter").cloned();
        let yaml = match stored {
            Some(serde_json::Value::String(yaml)) => yaml,
            other => panic!("expected a serialized YAML string, got {other:?}"),
        };
        // The written-back string parses back to the original native mapping.
        let round_trip: serde_json::Value = biscuit_file::Yaml::from_str(&yaml)
            .expect("valid yaml")
            .as_json()
            .expect("as_json");
        assert_eq!(round_trip, serde_json::json!({ "title": "Foo", "n": 1 }));
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
    fn write_back_skips_template_pending_value() {
        // A typed value still holding an unresolved `{{ ... }}` template is
        // deferred (run returns Ok) AND left untouched, so the template survives
        // to be resolved and re-validated after shell expansion. This mirrors a
        // frontmatter expression that could not evaluate during the pre-shell
        // interpolation pass because it transitively references a `$(...)` value
        // — e.g. `spec: "{{ file_exists(possible_spec) ? possible_spec : '' }}"`
        // where `possible_spec` resolves from a `$(dirname ...)` field.
        let mut md = md_with_schema("$schema:\n  n: number\nn: \"{{ some_expr }}\"\n");
        let options = ComposeOptions::new();
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("n"),
            Some(&serde_json::json!("{{ some_expr }}"))
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
                assert!(problems.iter().any(|p| p.property.as_deref() == Some("owner")));
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
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        let baseline = SimplifiedSchema::Single(SchemaShape {
            properties: baseline_props,
            ..Default::default()
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
        let mut md = md_with_schema("$schema:\n  spec: 'string(min(1);required)'\nspec: 'design.md'\n");
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
        assert!(err.source().is_none(), "validation failure should have no source");
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
        assert_eq!(authors[0]["metadata"], serde_json::json!({ "role": "admin" }));
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
                    problems
                        .iter()
                        .any(|p| p.path == "/config" || p.path.contains("/name")),
                    "expected a problem under /config or /config/name, got {problems:?}"
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
        assert_eq!(
            md.frontmatter().as_map()["count"],
            serde_json::json!("42")
        );
    }

    // ── Motivating case: lazy `file` output + eager `file` input ──────

    #[test]
    fn motivating_lazy_plan_composes_while_eager_review_present() {
        // Mirrors `sniff/prompts/plan-review-implementation.md` post-migration:
        // `review` is an eager INPUT that must exist; `plan` is a lazy OUTPUT
        // path this run is about to create. With the review file present and
        // `plan` naming a not-yet-existing path, schema validation must pass —
        // lazy `file` never checks existence, while eager `review` resolves
        // explicitly source-relative to the present review file.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("design-review.md"), b"# review\n").unwrap();
        let doc_path = dir.path().join("prompt.md");
        let mut md = md_with_schema_and_source(
            "$schema:\n\
             \x20 review: 'file(eager; required; match(**/*review*.md))'\n\
             \x20 plan: 'file'\n\
             \x20 iteration: 'number'\n\
             review: design-review.md\n\
             plan: plan-1.md\n\
             iteration: 1\n",
            &doc_path,
        );
        let options = ComposeOptions::new().with_source_file(&doc_path);
        assert!(
            run(&mut md, &options).is_ok(),
            "lazy `plan` output path must compose even though it does not exist \
             yet, while eager `review` resolves to the present review file",
        );
    }

    #[test]
    fn motivating_missing_eager_review_still_fails() {
        // The companion: with the eager `review` input absent from disk, schema
        // validation still fails on `/review` — `eager` restores the existence
        // check the default lazy `file` no longer performs. `plan` remains a
        // lazy output and is not the cause of the failure.
        let dir = tempfile::tempdir().unwrap();
        let doc_path = dir.path().join("prompt.md");
        let mut md = md_with_schema_and_source(
            "$schema:\n\
             \x20 review: 'file(eager; required; match(**/*review*.md))'\n\
             \x20 plan: 'file'\n\
             \x20 iteration: 'number'\n\
             review: missing-review.md\n\
             plan: plan-1.md\n\
             iteration: 1\n",
            &doc_path,
        );
        let options = ComposeOptions::new().with_source_file(&doc_path);
        let err = run(&mut md, &options).unwrap_err();
        match err {
            MarkdownError::SchemaValidationFailed { problems, .. } => {
                assert!(
                    problems.iter().any(|p| p.path == "/review"),
                    "expected an existence problem on /review, got {problems:?}",
                );
                assert!(
                    !problems.iter().any(|p| p.path == "/plan"),
                    "lazy `plan` must not contribute a problem, got {problems:?}",
                );
            }
            other => panic!("expected SchemaValidationFailed, got {other:?}"),
        }
    }

    // ── DM1b: deferred keys excluded from user schema validation ─────

    fn baseline_two_string_properties(
        required: &str,
        optional: &str,
    ) -> SimplifiedSchema {
        let mut properties = IndexMap::new();
        properties.insert(
            required.into(),
            PropertyDef::Single(PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        properties.insert(
            optional.into(),
            PropertyDef::Single(PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![],
                array_constraints: vec![],
                description: None,
            }),
        );
        SimplifiedSchema::Single(SchemaShape {
            properties,
            ..Default::default()
        })
    }

    #[test]
    fn deferred_key_with_lifecycle_syntax_not_rejected_by_user_schema() {
        // A deferred lifecycle key containing `{{err.msg}}` would fail a string
        // schema check if validated, but DM1b skips it.
        let mut md = md_with_schema(
            "name: my-prompt\n\
             failure:\n\
             \x20 message: \"{{err.msg}}\"\n",
        );
        let options = ComposeOptions::new()
            .with_baseline_schema(baseline_two_string_properties("name", "failure"))
            .with_exclude_keys(["failure"]);
        let result = run(&mut md, &options);
        assert!(
            result.is_ok(),
            "deferred key should not be schema-validated: {:?}",
            result.err()
        );
        // The ordinary `name` key still validates fine.
    }

    #[test]
    fn ordinary_schema_validation_runs_alongside_deferred_key() {
        // `name` is required by the baseline schema but missing from the
        // document; `failure` is deferred. The missing-required violation on
        // the non-deferred key must still be reported.
        let mut md = md_with_schema(
            "failure:\n\
             \x20 message: \"{{err.msg}}\"\n",
        );
        let options = ComposeOptions::new()
            .with_baseline_schema(baseline_two_string_properties("name", "failure"))
            .with_exclude_keys(["failure"]);
        let result = run(&mut md, &options);
        assert!(
            result.is_err(),
            "missing required 'name' must still be reported alongside a deferred key"
        );
    }

    #[test]
    fn deferred_key_not_coerced_by_schema() {
        // A deferred key's value must not be coerced — it stays raw.
        let mut md = md_with_schema(
            "failure:\n\
             \x20 flag: \"true\"\n",
        );
        let options = ComposeOptions::new()
            .with_baseline_schema(baseline_required_string("failure"))
            .with_exclude_keys(["failure"]);
        let _ = run(&mut md, &options);
        // The deferred key's nested "flag" stays as the string "true",
        // not coerced to a boolean.
        let failure = md.frontmatter().as_map().get("failure").expect("present");
        assert_eq!(failure.get("flag"), Some(&serde_json::json!("true")));
    }

    // ── Phase 3: eager-`file` value normalization write-back ──────────

    /// A repo fixture with a `.git` marker so the rewrite projects
    /// git-root-relative. The prompt lives in `area/` and references a file
    /// beside it; the rewrite must store the git-root-relative form.
    #[test]
    fn eager_file_value_rewritten_to_repo_relative_after_run() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(repo.path().join("area")).unwrap();
        std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
        let doc_path = repo.path().join("area/prompt.md");
        let mut md = md_with_schema_and_source(
            "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
            &doc_path,
        );
        let options = ComposeOptions::new().with_source_file(&doc_path);
        assert!(run(&mut md, &options).is_ok());
        // The raw `./spec.md` is rewritten to the git-root-relative
        // `area/spec.md`, matching `relative(spec)`/`dirname(spec)`.
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("area/spec.md")),
        );
    }

    /// Idempotence at the compose level (Decision #6): running the stage on
    /// an already-rewritten value is a fixpoint — the stored value does not
    /// drift across runs. The request-scoped repository root lets the
    /// git-root-relative rewritten value re-resolve on the second pass.
    #[test]
    fn eager_file_rewrite_is_idempotent_across_runs() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(repo.path().join("area")).unwrap();
        std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
        let doc_path = repo.path().join("area/prompt.md");
        let mut md = md_with_schema_and_source(
            "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
            &doc_path,
        );
        let options = ComposeOptions::new()
            .with_source_file(&doc_path)
            .with_file_ref_fallback_dir(repo.path().to_path_buf());
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("area/spec.md")),
        );
        // Re-run on the persisted (rewritten) value — it must not change.
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("area/spec.md")),
            "re-running the stage on a rewritten value must be a fixpoint",
        );
    }

    /// A `$(...)`-pending eager-`file` value is left verbatim by the rewrite
    /// (Decision #4) — its literal form survives into shell expansion, where
    /// the post-shell re-validation will handle it.
    #[test]
    fn eager_file_rewrite_skips_pending_value() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let doc_path = repo.path().join("prompt.md");
        let mut md = md_with_schema_and_source(
            "$schema:\n  spec: 'file(eager; required)'\nspec: \"$(echo spec.md)\"\n",
            &doc_path,
        );
        let options = ComposeOptions::new().with_source_file(&doc_path);
        assert!(run(&mut md, &options).is_ok(), "pending value is deferred");
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("$(echo spec.md)")),
            "pending eager-file value must be left verbatim",
        );
    }

    /// A lazy (bare) `file` property is never rewritten even when its value
    /// happens to resolve to an existing file (Decision #1: only the eager
    /// marker triggers).
    #[test]
    fn lazy_file_value_not_rewritten_by_compose() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::write(repo.path().join("spec.md"), "# Spec\n").unwrap();
        let doc_path = repo.path().join("prompt.md");
        let mut md = md_with_schema_and_source(
            "$schema:\n  spec: 'file'\nspec: ./spec.md\n",
            &doc_path,
        );
        let options = ComposeOptions::new().with_source_file(&doc_path);
        assert!(run(&mut md, &options).is_ok());
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("./spec.md")),
            "lazy `file` value must be left verbatim",
        );
    }

    #[test]
    fn eager_file_set_override_resolves_from_the_callers_launch_area() {
        let dir = tempfile::tempdir().unwrap();
        let document_dir = dir.path().join("document");
        let launch_dir = dir.path().join("launch");
        std::fs::create_dir_all(&document_dir).unwrap();
        std::fs::create_dir_all(&launch_dir).unwrap();
        let resolved = launch_dir.join("spec.md");
        std::fs::write(&resolved, "# Spec\n").unwrap();
        let doc_path = document_dir.join("prompt.md");
        let content = "---\n$schema:\n  spec: 'file(eager; required)'\nspec: authored.md\n---\nspec says: {{ spec }}\n";
        let md: Markdown = content.into();
        let md = md.with_source(ComposeSource::File(doc_path.clone()));
        let options = ComposeOptions::new()
            .with_source_file(&doc_path)
            .with_file_ref_fallback_dir(&launch_dir)
            .with_set_overrides(serde_json::json!({ "spec": "spec.md" }));

        let (composed, _) = md.compose_with(options).unwrap();
        assert_eq!(
            composed.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!(resolved.to_string_lossy())),
        );
        assert_eq!(
            composed.content(),
            format!(
                "spec says: {}\n",
                resolved.to_string_lossy().replace('\\', "/")
            ),
            "Markdown presentation uses forward slashes without changing effective frontmatter",
        );
    }

    #[test]
    fn eager_file_set_override_keeps_launch_area_across_target_repository_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let launch_repo = dir.path().join("launch-repo");
        let target_repo = dir.path().join("target-repo");
        std::fs::create_dir_all(launch_repo.join(".git")).unwrap();
        std::fs::create_dir_all(target_repo.join(".git")).unwrap();
        let resolved = launch_repo.join("spec.md");
        std::fs::write(&resolved, "# Spec\n").unwrap();
        let doc_path = target_repo.join("prompt.md");

        for schema in ["file(required;eager)", "'file(required;eager)'"] {
            let md = md_with_schema_and_source(
                &format!("$schema:\n  spec: {schema}\nspec: authored.md\n"),
                &doc_path,
            );
            let target_context = biscuit_file::FileResolutionContext::new(&target_repo)
                .with_repository_root(&target_repo)
                .with_source_path(&doc_path);
            let options = ComposeOptions::new()
                .with_source_file(&doc_path)
                .with_file_resolution_context(target_context)
                .with_file_ref_fallback_dir(&launch_repo)
                .with_set_overrides(serde_json::json!({ "spec": "spec.md" }));

            let (composed, _) = md.compose_with(options).unwrap();
            assert_eq!(
                composed.frontmatter().as_map().get("spec"),
                Some(&serde_json::json!(resolved.to_string_lossy())),
                "caller-originated `spec.md` must stay anchored to the launch area for {schema}",
            );
        }
    }

    #[test]
    fn eager_file_array_set_override_resolves_each_launch_area_value() {
        let dir = tempfile::tempdir().unwrap();
        let document_dir = dir.path().join("document");
        let launch_dir = dir.path().join("launch");
        std::fs::create_dir_all(&document_dir).unwrap();
        std::fs::create_dir_all(&launch_dir).unwrap();
        let first = launch_dir.join("first.md");
        let second = launch_dir.join("second.md");
        std::fs::write(&first, "# First\n").unwrap();
        std::fs::write(&second, "# Second\n").unwrap();
        let doc_path = document_dir.join("prompt.md");
        let content = "---\n$schema:\n  specs: 'file(eager)[]'\nspecs: []\n---\nfirst: {{ specs[0] }}\n";
        let md: Markdown = content.into();
        let md = md.with_source(ComposeSource::File(doc_path.clone()));
        let options = ComposeOptions::new()
            .with_source_file(&doc_path)
            .with_file_ref_fallback_dir(&launch_dir)
            .with_set_overrides(serde_json::json!({ "specs": ["first.md", "second.md"] }));

        let (composed, _) = md.compose_with(options).unwrap();
        assert_eq!(
            composed.frontmatter().as_map().get("specs"),
            Some(&serde_json::json!([
                first.to_string_lossy(),
                second.to_string_lossy(),
            ])),
        );
        assert_eq!(
            composed.content(),
            format!(
                "first: {}\n",
                first.to_string_lossy().replace('\\', "/")
            ),
            "indexed eager-file presentation uses forward slashes without changing the native array",
        );
    }

    #[test]
    fn ordinary_string_set_override_is_not_treated_as_a_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        let launch_dir = dir.path().join("launch");
        std::fs::create_dir_all(&launch_dir).unwrap();
        std::fs::write(launch_dir.join("spec.md"), "# Spec\n").unwrap();
        let doc_path = dir.path().join("prompt.md");
        let md = md_with_schema_and_source(
            "$schema:\n  label: 'string(required)'\nlabel: authored\n",
            &doc_path,
        );
        let options = ComposeOptions::new()
            .with_source_file(&doc_path)
            .with_file_ref_fallback_dir(&launch_dir)
            .with_set_overrides(serde_json::json!({ "label": "spec.md" }));

        let (composed, _) = md.compose_with(options).unwrap();
        assert_eq!(
            composed.frontmatter().as_map().get("label"),
            Some(&serde_json::json!("spec.md")),
        );
    }

    // The rewrite consumes the `ResolutionContext` carried by `EffectiveSchema`
    // and must not introduce an ambient-CWD anchor. With process CWD mutated to
    // an unrelated directory, compose still produces the repository-relative
    // value.

    /// RAII guard that restores the process CWD on drop, even on panic.
    struct CwdGuard {
        prior: std::path::PathBuf,
    }

    impl CwdGuard {
        fn enter(dir: &std::path::Path) -> Self {
            let prior = std::env::current_dir().expect("read CWD");
            std::env::set_current_dir(dir).expect("set CWD");
            Self { prior }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prior);
        }
    }

    /// With the process CWD mutated to an unrelated directory, compose's
    /// eager-`file` rewrite still produces the git-root-relative value. The
    /// rewrite resolves through the document base directory (repository-first
    /// then source), never the ambient CWD.
    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn eager_file_rewrite_is_independent_of_process_cwd() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(repo.path().join("area")).unwrap();
        std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
        let doc_path = repo.path().join("area/prompt.md");

        let unrelated = tempfile::tempdir().unwrap();

        let mut md = md_with_schema_and_source(
            "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
            &doc_path,
        );
        // The rewritten value (`area/spec.md`) is an implicit reference that
        // re-resolves repository-first on a second pass, proving the rewrite
        // does not depend on the ambient CWD. The launch-area anchor is set but
        // inert for resolution (D2).
        let options = ComposeOptions::new()
            .with_source_file(&doc_path)
            .with_file_ref_fallback_dir(repo.path().to_path_buf());

        let _cwd = CwdGuard::enter(unrelated.path());
        assert!(run(&mut md, &options).is_ok(), "compose must succeed from an unrelated CWD");
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("area/spec.md")),
            "rewrite must produce the git-root-relative value regardless of process CWD",
        );
    }
}
