use super::*;

pub(super) fn schema_error_to_composition_error(
    source_path: &std::path::Path,
    fallback_message: String,
    schema_error: Option<&SchemaError>,
) -> CompositionError {
    // Grammar/Convert attach a synthetic name (`<root>`, `<arm[N]>`) for purely
    // structural failures; only a real, user-addressable property is worth
    // surfacing as scope.
    fn real_property(name: &str) -> Option<String> {
        (!name.starts_with('<')).then(|| name.to_string())
    }

    match schema_error {
        Some(SchemaError::Grammar {
            property,
            message,
            span,
        }) => CompositionError::SchemaParse {
            source_path: source_path.to_path_buf(),
            property: real_property(property),
            message: message.clone(),
            span: Some(span.clone()),
        },
        Some(SchemaError::Convert { property, message }) => CompositionError::SchemaParse {
            source_path: source_path.to_path_buf(),
            property: real_property(property),
            message: message.clone(),
            span: None,
        },
        Some(SchemaError::FrontmatterShape { message }) => CompositionError::SchemaParse {
            source_path: source_path.to_path_buf(),
            property: None,
            message: message.clone(),
            span: None,
        },
        _ => CompositionError::SchemaLoad {
            source_path: source_path.to_path_buf(),
            message: fallback_message,
        },
    }
}

pub(super) fn handle_compose_error(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: &PrepareMode,
    err: CompositionError,
    dropped: &mut Vec<DroppedOptional>,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(markdown_err) = err else {
        return Err(err);
    };

    match markdown_err {
        MarkdownError::SchemaValidationFailed {
            problems,
            summary,
            source: schema_source,
            ..
        } => translate_schema_failure(
            source,
            options,
            mode,
            problems,
            summary,
            schema_source,
            dropped,
        ),
        other => Err(CompositionError::ComposeFailed(other)),
    }
}

pub(super) fn translate_schema_failure(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: &PrepareMode,
    problems: Vec<ValidationProblem>,
    summary: String,
    schema_source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    dropped: &mut Vec<DroppedOptional>,
) -> Result<PreparedComposition, CompositionError> {
    // Empty problems list signals a preparation failure: either a *parse* error
    // in the schema body, or a *reference-resolution* failure. The typed
    // `SchemaError` carried on the source distinguishes them — a grammar/convert/
    // shape error is a body-syntax problem (`SchemaParse`, with constraint-grammar
    // remediation), whereas a missing file / remote URL / ambiguous reference is a
    // resolution failure (`SchemaLoad`, with path remediation). Without a typed
    // cause, fall back to `SchemaLoad`.
    if problems.is_empty() {
        let typed = schema_source
            .as_deref()
            .and_then(|err| err.downcast_ref::<SchemaError>());
        return Err(schema_error_to_composition_error(
            &source.resolved_path,
            summary,
            typed,
        ));
    }

    let effective = load_effective_schema(source, options.file_ref_fallback_dir.as_deref())?;
    let categorized = categorize_problems(&problems, effective.as_ref());

    if !categorized.invalid_required.is_empty() {
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }

    if !categorized.invalid_optional.is_empty() {
        let droppable =
            filter_droppable_invalid_optionals(&categorized.invalid_optional, effective.as_ref());
        if droppable.len() != categorized.invalid_optional.len() {
            let hard: Vec<_> = categorized
                .invalid_optional
                .iter()
                .filter(|problem| {
                    !droppable.iter().any(|droppable_problem| {
                        droppable_problem.path == problem.path
                            && droppable_problem.message == problem.message
                    })
                })
                .cloned()
                .collect();
            return Err(build_schema_validation_error(&source.resolved_path, &hard));
        }
        // Drop invalid optionals from a clone of the source AND from the
        // run's `set_overrides` map. Source-only removal is not enough:
        // overrides land on top of frontmatter during compose, so a bad
        // `key=value` or `--set` value would otherwise re-fail validation.
        // Retry the prepare step exactly once. If composition still fails,
        // fall through to surface the residual problem (likely a missing
        // required property).
        let (retry_source, source_drops) = source_with_dropped_optionals(source, &droppable);
        let (retry_options, override_drops) =
            options_with_dropped_optionals(options.clone(), &droppable);
        dropped.extend(source_drops);
        dropped.extend(override_drops);
        return match run_prepare(&retry_source, retry_options, mode) {
            Ok(prepared) => Ok(prepared),
            Err(retry_err) => {
                handle_retry_error(source, retry_err, options.file_ref_fallback_dir.as_deref())
            }
        };
    }

    if !categorized.missing_required.is_empty() {
        return Err(build_missing_properties_error(
            source,
            &categorized.missing_required,
            &categorized.pointer_paths,
        ));
    }

    // Fall-through: problems list contained only categories we couldn't
    // act on (e.g. arm-index union problems with no `kind` mapping). Surface
    // a generic SchemaValidation so the user still sees the failure.
    Err(CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message: summary,
        problems: problems.iter().map(|p| p.path.clone()).collect(),
    })
}

pub(super) fn handle_retry_error(
    source: &ResolvedCompositionSource,
    err: CompositionError,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(MarkdownError::SchemaValidationFailed {
        problems,
        summary,
        ..
    }) = err
    else {
        return Err(err);
    };

    let effective = load_effective_schema(source, file_ref_fallback_dir)?;
    let categorized = categorize_problems(&problems, effective.as_ref());

    if !categorized.invalid_required.is_empty() {
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }

    if !categorized.missing_required.is_empty() {
        return Err(build_missing_properties_error(
            source,
            &categorized.missing_required,
            &categorized.pointer_paths,
        ));
    }

    Err(CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message: summary,
        problems: problems.iter().map(|p| p.path.clone()).collect(),
    })
}

/// Validate `prepared.effective_frontmatter` after Darkmatter composition
/// (including frontmatter shell expansion) has finished, and apply the
/// same typed error / drop-and-retry rules as the prepare-time validator.
///
/// Darkmatter's compose pipeline runs schema validation AFTER frontmatter
/// interpolation but BEFORE frontmatter shell expansion (see
/// `darkmatter::markdown::compose::schema_validation`). Values produced by
/// `$(...)` expressions therefore never satisfy or violate the schema
/// during compose itself. This step closes that gap: it validates the
/// final effective frontmatter against the resolved schema and:
///
/// - Returns [`CompositionError::SchemaValidation`] if a required value
///   is now invalid (e.g. shell command produced bad output).
/// - Returns [`CompositionError::MissingProperties`] if a required value
///   is now missing (e.g. shell command emitted empty string and the
///   schema required a non-empty value).
/// - Drops invalid optionals from `effective_frontmatter` in place,
///   tracks each as a [`DroppedOptional`] tagged
///   [`DroppedOptionalStage::PostShellExpansion`].
pub(super) fn build_schema_validation_error(
    source_path: &std::path::Path,
    invalid: &[ValidationProblem],
) -> CompositionError {
    let message = invalid
        .iter()
        .map(|p| {
            if p.path.is_empty() {
                p.message.clone()
            } else {
                format!("{}: {}", p.path, p.message)
            }
        })
        .collect::<Vec<_>>()
        .join("; ");
    let problems = invalid.iter().map(|p| p.path.clone()).collect();
    CompositionError::SchemaValidation {
        source_path: source_path.to_path_buf(),
        message,
        problems,
    }
}

pub(super) fn build_missing_properties_error(
    source: &ResolvedCompositionSource,
    missing: &[MissingProperty],
    pointer_paths: &[String],
) -> CompositionError {
    let frontmatter_description = source
        .markdown
        .frontmatter()
        .as_map()
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    CompositionError::MissingProperties {
        source_path: source.resolved_path.clone(),
        missing: missing.to_vec(),
        frontmatter_description,
        pointer_paths: pointer_paths.to_vec(),
    }
}

/// Build a clone of `source` with each invalid-optional key removed from
/// the markdown's frontmatter. Returns the modified source plus a record
/// of every property that was actually elided.
pub(super) fn source_with_dropped_optionals(
    source: &ResolvedCompositionSource,
    invalid_optional: &[ValidationProblem],
) -> (ResolvedCompositionSource, Vec<DroppedOptional>) {
    let mut clone = source.clone();
    let map = clone.markdown.frontmatter_mut().as_map_mut();
    let mut drops = Vec::new();
    for problem in invalid_optional {
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            continue;
        };
        if map.shift_remove(&name).is_some() {
            tracing::warn!(
                property = %name,
                pointer = %problem.path,
                message = %problem.message,
                "dropping optional schema property with invalid value",
            );
            drops.push(DroppedOptional {
                property: name,
                source: DroppedOptionalSource::Frontmatter,
                stage: DroppedOptionalStage::Composition,
                reason: problem.message.clone(),
            });
        }
    }
    (clone, drops)
}

/// Build a clone of `options` with each invalid-optional key removed from
/// the run's `set_overrides` JSON object. Returns the modified options
/// plus a record of every override that was elided.
///
/// CLI-supplied values (`key=value` setters and `--set` JSON) land in
/// `set_overrides` and are applied on top of frontmatter by Darkmatter's
/// compose stage. Dropping them on retry mirrors the source-level drop so
/// run-scoped invalid optionals follow the same rule as file-authored
/// invalid optionals: warn, elide, re-compose, re-validate.
pub(super) fn options_with_dropped_optionals(
    mut options: PrepareOptions,
    invalid_optional: &[ValidationProblem],
) -> (PrepareOptions, Vec<DroppedOptional>) {
    let mut drops = Vec::new();
    let Some(serde_json::Value::Object(ref mut map)) = options.set_overrides else {
        return (options, drops);
    };
    for problem in invalid_optional {
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            continue;
        };
        if map.remove(&name).is_some() {
            tracing::warn!(
                property = %name,
                pointer = %problem.path,
                message = %problem.message,
                "dropping optional schema override with invalid value",
            );
            drops.push(DroppedOptional {
                property: name,
                source: DroppedOptionalSource::Override,
                stage: DroppedOptionalStage::Composition,
                reason: problem.message.clone(),
            });
        }
    }
    (options, drops)
}

pub(super) fn filter_droppable_invalid_optionals(
    invalid_optional: &[ValidationProblem],
    effective: Option<&EffectiveSchema>,
) -> Vec<ValidationProblem> {
    let shape: Option<&SchemaShape> = effective.and_then(|e| match e.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => Some(s),
        Some(SimplifiedSchema::Union(_)) | None => None,
    });
    invalid_optional
        .iter()
        .filter(|problem| !is_eager_file_problem(shape, problem))
        .cloned()
        .collect()
}

// -- categorization ---------------------------------------------------------
