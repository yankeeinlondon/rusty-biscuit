use super::*;

pub(super) struct CategorizedProblems {
    pub(super) missing_required: Vec<MissingProperty>,
    pub(super) invalid_required: Vec<ValidationProblem>,
    pub(super) invalid_optional: Vec<ValidationProblem>,
    pub(super) pointer_paths: Vec<String>,
}

pub(super) fn categorize_problems(
    problems: &[ValidationProblem],
    effective: Option<&EffectiveSchema>,
    phase: Option<SchemaPhase>,
) -> CategorizedProblems {
    let mut missing_required = Vec::new();
    let mut invalid_required = Vec::new();
    let mut invalid_optional = Vec::new();
    let mut pointer_paths = Vec::new();

    let shape: Option<&SchemaShape> = effective.and_then(|e| match e.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => Some(s),
        Some(SimplifiedSchema::Union(_)) | None => None,
    });

    for problem in problems {
        pointer_paths.push(problem.path.clone());
        match problem.kind {
            ValidationProblemKind::Missing => {
                let name = problem
                    .property
                    .clone()
                    .unwrap_or_else(|| problem.path.clone());
                let atom = shape.and_then(|s| atom_for_property(s, &name));
                let (type_label, description, interactive_shape) = match atom {
                    Some(a) => (
                        Some(type_label_for_atom(a)),
                        a.description.clone(),
                        interactive_shape_for_atom(a),
                    ),
                    None => (None, None, None),
                };
                missing_required.push(MissingProperty {
                    name,
                    type_label,
                    description,
                    interactive_shape,
                });
            }
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                let top = top_level_pointer_segment(&problem.path);
                let hard_at_this_phase = top
                    .as_deref()
                    .map(|name| {
                        is_required(shape, name)
                            || (phase == Some(SchemaPhase::Launch) && is_eager(shape, name))
                    })
                    .unwrap_or(true);
                if hard_at_this_phase {
                    invalid_required.push(problem.clone());
                } else {
                    invalid_optional.push(problem.clone());
                }
            }
        }
    }

    CategorizedProblems {
        missing_required,
        invalid_required,
        invalid_optional,
        pointer_paths,
    }
}

pub(super) fn atom_for_property<'s>(shape: &'s SchemaShape, name: &str) -> Option<&'s PropertyAtom> {
    let def = shape.properties.get(name)?;
    match def {
        PropertyDef::Single(atom) => Some(atom),
        // Property-level unions cannot be reduced to a single atom; the
        // type label is left blank so the renderer falls back to a
        // generic "(unknown type)" hint.
        PropertyDef::Union(_) => None,
    }
}

/// Whether any arm of `name` carries `eager` (on the value or, for an array
/// property, on the array itself). A present eager value is validated at
/// launch; `required` independently controls presence.
pub(super) fn is_eager(shape: Option<&SchemaShape>, name: &str) -> bool {
    let Some(def) = shape.and_then(|shape| shape.properties.get(name)) else {
        return false;
    };
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(a) => vec![a],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .chain(atom.array_constraints.iter())
            .any(|c| matches!(c, Constraint::Eager))
    })
}

/// Reclassify a required top-level property whose effective value is `null`
/// from "invalid" to "missing".
///
/// A direct-compose document commonly authors
/// `foo: {{ spec ? parent_dir(spec) + '/bar.md' : null }}`; when the
/// expression resolves to `null` the caller must supply `foo` at launch, which
/// is the interactive-collection path, not a wrong-type failure.
///
/// `instance` is the judged effective frontmatter when the caller has it;
/// the compose-time failure path has no instance and recognizes the null
/// verdict from the validator's own `null is not of type …` message.
pub(super) fn promote_null_required_to_missing(
    categorized: &mut CategorizedProblems,
    effective: Option<&EffectiveSchema>,
    instance: Option<&serde_json::Value>,
) {
    let shape: Option<&SchemaShape> = effective.and_then(|e| match e.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => Some(s),
        Some(SimplifiedSchema::Union(_)) | None => None,
    });
    let Some(shape) = shape else {
        return;
    };
    let map = instance.and_then(serde_json::Value::as_object);
    let mut promoted: Vec<String> = Vec::new();
    categorized.invalid_required.retain(|problem| {
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            return true;
        };
        let is_null = match map {
            Some(map) => map.get(&name).is_some_and(serde_json::Value::is_null),
            None => problem.message.starts_with("null is not of type"),
        };
        if !is_null || !is_required(Some(shape), &name) {
            return true;
        }
        if !promoted.contains(&name) {
            promoted.push(name);
        }
        false
    });
    for name in promoted {
        if categorized
            .missing_required
            .iter()
            .any(|missing| missing.name == name)
        {
            continue;
        }
        let atom = atom_for_property(shape, &name);
        let (type_label, description, interactive_shape) = match atom {
            Some(a) => (
                Some(type_label_for_atom(a)),
                a.description.clone(),
                interactive_shape_for_atom(a),
            ),
            None => (None, None, None),
        };
        categorized.missing_required.push(MissingProperty {
            name,
            type_label,
            description,
            interactive_shape,
        });
    }
}

pub(super) fn is_required(shape: Option<&SchemaShape>, name: &str) -> bool {
    let Some(shape) = shape else {
        // Without typed metadata we can't distinguish optional from required.
        // Be conservative: treat the property as required so unexpected
        // invalid values surface loudly instead of being silently dropped.
        return true;
    };
    let Some(def) = shape.properties.get(name) else {
        // Property not declared in the shape — extras are neither required
        // nor optional in our vocabulary. Treat as required for the same
        // reason.
        return true;
    };
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(a) => vec![a],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .chain(atom.array_constraints.iter())
            .any(|c| matches!(c, Constraint::Required))
    })
}

/// Classify a validation problem set as a
/// [`CompositionError::UnresolvedFileReference`] when a **provided** value for
/// a `file`/`file[]` property with non-empty `match(...)` patterns failed
/// existence resolution (Darkmatter's `NoMatch` case).
///
/// This is the read-side twin of the missing-property classification: instead
/// of the value being *absent*, the user supplied a value best interpreted as a
/// **partial** — a substring to match against the property's `match(...)` glob
/// candidates. The CLI catches this variant and drives a confirmation dialog
/// (single match) or chooser (multiple), mirroring the missing-property loop.
///
/// Returns `None` when no problem qualifies, so the caller falls back to the
/// generic [`CompositionError::SchemaValidation`]. Only the first qualifying
/// property is surfaced; the interactive retry re-runs validation and picks up
/// any remaining ones one at a time.
pub(super) fn classify_unresolved_file_reference(
    source_path: &std::path::Path,
    problems: &[ValidationProblem],
    effective: Option<&EffectiveSchema>,
    instance: &serde_json::Value,
) -> Option<CompositionError> {
    let shape = match effective?.simplified.as_ref()? {
        SimplifiedSchema::Single(s) => s,
        SimplifiedSchema::Union(_) => return None,
    };
    for problem in problems {
        // Only Darkmatter's `NoMatch` ("no existing file matched reference")
        // is a resolvable partial — a parse/resolution error is a genuinely
        // bad value that a glob walk cannot rescue.
        if !problem.message.contains("no existing file matched reference") {
            continue;
        }
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            continue;
        };
        let Some(atom) = atom_for_property(shape, &name) else {
            continue;
        };
        if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
            continue;
        }
        let patterns: Vec<String> = atom
            .constraints
            .iter()
            .find_map(|c| match c {
                Constraint::Match(p) => Some(p.clone()),
                _ => None,
            })
            .unwrap_or_default();
        // A bare `file` (no glob) has nothing to walk — leave it to the generic
        // validation path.
        if patterns.is_empty() {
            continue;
        }
        let Some(provided) = provided_partial_value(instance.get(&name)) else {
            continue;
        };
        return Some(CompositionError::UnresolvedFileReference {
            source_path: source_path.to_path_buf(),
            property: name,
            provided,
            patterns,
            is_array: atom.is_array,
            reason: problem.message.clone(),
        });
    }
    None
}

/// Extract the user-provided partial from a frontmatter/override value.
///
/// Returns the string for a scalar `file` value (the substring to match against
/// the `match(...)` glob candidates). For a `file[]` value, accepts an array of
/// strings and uses the first non-empty string as the partial; a scalar string
/// is treated as single-element intent for convenience. Non-string array
/// elements or empty arrays are rejected and left to the generic
/// schema-validation path.
pub(super) fn provided_partial_value(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        serde_json::Value::Array(arr) => arr
            .iter()
            .find_map(|v| match v {
                serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
                _ => None,
            }),
        _ => None,
    }
}

/// Map a [`PropertyAtom`] to an [`InteractiveShape`] for CLI prompting.
///
/// Returns `None` when the atom describes a shape that cannot be
/// collected via a single TUI widget (e.g. `object`, `any`, or a semantic
/// definition artifact).
pub(super) fn interactive_shape_for_atom(atom: &PropertyAtom) -> Option<InteractiveShape> {
    match &atom.ty {
        TypeExpr::Primitive(SimplifiedType::Enum) => {
            let members: Vec<String> = atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Members(m) => Some(m.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if members.is_empty() {
                return None;
            }
            if atom.is_array {
                Some(InteractiveShape::EnumMany { members })
            } else {
                Some(InteractiveShape::EnumOne { members })
            }
        }
        TypeExpr::Primitive(SimplifiedType::Boolean | SimplifiedType::Boolish) => {
            Some(InteractiveShape::Boolean)
        }
        TypeExpr::Primitive(SimplifiedType::Number | SimplifiedType::NumberLike) => {
            let integer = atom
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Integer));
            let (min, max) = min_max_constraints(atom);
            Some(InteractiveShape::Number { integer, min, max })
        }
        TypeExpr::Primitive(
            SimplifiedType::String
            | SimplifiedType::Yaml
            | SimplifiedType::Json
            | SimplifiedType::Expression,
        ) => {
            let (min_len, max_len) = string_length_constraints(atom);
            Some(InteractiveShape::Text {
                format: TextFormat::Plain,
                min_len,
                max_len,
            })
        }
        TypeExpr::Primitive(SimplifiedType::Date) => Some(InteractiveShape::Text {
            format: TextFormat::Date,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::DateTime) => Some(InteractiveShape::Text {
            format: TextFormat::DateTime,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::Time) => Some(InteractiveShape::Text {
            format: TextFormat::Time,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::Url) => Some(InteractiveShape::Text {
            format: TextFormat::Url,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::Email) => Some(InteractiveShape::Text {
            format: TextFormat::Email,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::File) => {
            let patterns = atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Match(p) => Some(p.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            Some(InteractiveShape::File {
                is_array: atom.is_array,
                patterns,
            })
        }
        // A `literal(...)` is a fixed `const` with exactly one valid value, so
        // there is nothing to prompt for.
        TypeExpr::Primitive(
            SimplifiedType::Object
            | SimplifiedType::Any
            | SimplifiedType::Literal
            | SimplifiedType::TypeDefinition
            | SimplifiedType::Schema,
        )
        | TypeExpr::InlineObject(_)
        | TypeExpr::Imported { .. } => None,
    }
}

pub(super) fn min_max_constraints(atom: &PropertyAtom) -> (Option<f64>, Option<f64>) {
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for c in &atom.constraints {
        match c {
            Constraint::Min(v) => min = Some(*v),
            Constraint::Max(v) => max = Some(*v),
            _ => {}
        }
    }
    (min, max)
}

pub(super) fn string_length_constraints(atom: &PropertyAtom) -> (Option<usize>, Option<usize>) {
    let mut min_len: Option<usize> = None;
    let mut max_len: Option<usize> = None;
    for c in &atom.constraints {
        match c {
            Constraint::MinLen(v) => min_len = Some(*v),
            Constraint::MaxLen(v) => max_len = Some(*v),
            _ => {}
        }
    }
    (min_len, max_len)
}

pub(super) fn type_label_for_atom(atom: &PropertyAtom) -> String {
    let suffix = if atom.is_array { "[]" } else { "" };
    match &atom.ty {
        TypeExpr::Primitive(SimplifiedType::Enum) => {
            let members = atom.constraints.iter().find_map(|c| match c {
                Constraint::Members(m) => Some(m.join("|")),
                _ => None,
            });
            match members {
                Some(m) => format!("enum({m}){suffix}"),
                None => format!("enum(){suffix}"),
            }
        }
        TypeExpr::Primitive(ty) => format!("{base}{suffix}", base = ty.as_keyword()),
        TypeExpr::InlineObject(_) => format!("object{suffix}"),
        TypeExpr::Imported { name, reference } => format!("{name}{suffix}@{reference}"),
    }
}

// -- status report ----------------------------------------------------------

/// Structured per-property status of a schema-validated composition.
///
/// Used by the CLI to render the diagnostic status report (Phase 3 Task 1)
/// before driving interactive collection of missing required properties.
/// Tests should assert on the structured fields rather than rendered
/// terminal output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaStatusReport {
    /// The prompt file whose schema was evaluated.
    pub source_path: std::path::PathBuf,
    /// Required properties in declaration order.
    pub required: Vec<PropertyStatus>,
    /// Optional properties in declaration order.
    pub optional: Vec<PropertyStatus>,
    /// `true` when at least one optional property had an invalid value
    /// (these are dropped by the validation retry but worth surfacing).
    pub has_invalid_optional: bool,
    /// `true` when the schema is raw JSON Schema (no SimplifiedSchema
    /// metadata is available, so property listing falls back to the
    /// validator's pointer paths).
    pub raw_json_schema: bool,
}

/// Per-property status entry inside a [`SchemaStatusReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyStatus {
    /// Property name as declared in the schema.
    pub name: String,
    /// Schema type label (e.g. `string`, `enum(a|b|c)`, `number[]`).
    pub type_label: String,
    /// Schema-declared description for the property, if any.
    pub description: Option<String>,
    /// Validation outcome for the property under current frontmatter.
    pub state: PropertyState,
}

/// Validation outcome for a single property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyState {
    /// The property is present and validates against the schema.
    Valid,
    /// The property is present but violates a schema constraint.
    Invalid,
    /// The property is absent from the (effective) frontmatter.
    Missing,
    /// The property is required by completion but not at launch, and is
    /// absent: a later actor (the inline agent, or the closure for
    /// `last_updated`) is expected to supply it. Only reported when the
    /// report was judged at [`SchemaPhase::Launch`] for `inline-compose`.
    Deferred,
}

/// Build a [`SchemaStatusReport`] for `source` under the supplied
/// `set_overrides`, mirroring the validation Darkmatter would run during
/// composition.
///
/// `file_ref_fallback_dir` is captured launch metadata retained by
/// `file`-typed property diagnostics. Pass `None` only when no launch area is
/// known (for example, in unit tests).
///
/// Returns `Ok(None)` when the document has no `$schema` declaration.
///
/// ## Errors
///
/// Returns [`CompositionError::SchemaLoad`] when the document's `$schema`
/// reference cannot be resolved or compiled.
pub fn build_schema_status_report(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<Option<SchemaStatusReport>, CompositionError> {
    build_schema_status_report_for_mode(
        source,
        set_overrides,
        file_ref_fallback_dir,
        CompositionMode::ChainedDocument,
    )
}

/// [`build_schema_status_report`] judged at the launch phase of `mode`.
///
/// For `inline-compose` a required-but-not-eager property that is absent is
/// reported as [`PropertyState::Deferred`] rather than [`PropertyState::Missing`].
///
/// ## Errors
///
/// See [`build_schema_status_report`].
pub fn build_schema_status_report_for_mode(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    mode: CompositionMode,
) -> Result<Option<SchemaStatusReport>, CompositionError> {
    let phase = launch_phase_for_mode(mode);
    // Skip when the document has no `$schema`.
    if source
        .markdown
        .frontmatter()
        .as_map()
        .get("$schema")
        .is_none()
    {
        return Ok(None);
    }

    let effective = load_effective_schema(source, file_ref_fallback_dir)?;
    let Some(effective) = effective else {
        // Raw JSON Schema (no SimplifiedSchema projection): we can still
        // run validation, but without typed metadata for per-property
        // categorization. Return a minimal report flagged as raw.
        return Ok(Some(SchemaStatusReport {
            source_path: source.resolved_path.clone(),
            required: Vec::new(),
            optional: Vec::new(),
            has_invalid_optional: false,
            raw_json_schema: true,
        }));
    };

    // Build a frontmatter view with `set_overrides` applied so the
    // validation result reflects what the user has supplied on the
    // command line so far.
    let mut fm_map: serde_json::Map<String, serde_json::Value> = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if let Some(serde_json::Value::Object(overrides)) = set_overrides {
        for (k, v) in overrides {
            fm_map.insert(k.clone(), v.clone());
        }
    }

    status_report_for_instance(
        &effective,
        phase,
        &source.resolved_path,
        fm_map,
    )
    .map_err(|error| {
        schema_error_to_composition_error(
            &source.resolved_path,
            error.to_string(),
            Some(&error),
        )
    })
}

/// Build the per-property status of `instance` against an already-resolved
/// effective schema, judged at `phase` (`None` = unphased).
///
/// Shared by the pre-prepare launch report and the retained launch report on
/// a prepared composition; the completion verdict renders through the same
/// shape so both ends of a run look alike.
pub(in crate::composition) fn status_report_for_instance(
    effective: &EffectiveSchema,
    phase: Option<SchemaPhase>,
    source_path: &std::path::Path,
    fm_map: serde_json::Map<String, serde_json::Value>,
) -> Result<Option<SchemaStatusReport>, SchemaError> {
    let instance = serde_json::Value::Object(fm_map.clone());
    let report = match phase {
        Some(phase) => effective.validate_for_phase(&instance, phase)?,
        None => effective.validate(&instance),
    };
    Ok(status_report_from_validation(
        effective,
        phase,
        source_path,
        fm_map,
        &report,
    ))
}

/// Build a status report from the validation pass that judged the same
/// effective schema and instance.
pub(in crate::composition) fn status_report_from_validation(
    effective: &EffectiveSchema,
    phase: Option<SchemaPhase>,
    source_path: &std::path::Path,
    fm_map: serde_json::Map<String, serde_json::Value>,
    report: &ValidationReport,
) -> Option<SchemaStatusReport> {

    // Walk problems and build a per-property index keyed by top-level
    // segment / property name.
    //
    // Composition-tolerant: Type/Invalid verdicts against a raw value that
    // contains Darkmatter template syntax (`{{ ... }}`) are deferred — the
    // status report mirrors what the preflight + prepare pipeline will
    // ultimately see, which is the *effective* frontmatter after
    // composition. Flagging `runtime_agent: '{{ env.AGENT }}'` as Invalid
    // here would contradict the (correct) successful execution that
    // follows. See `features/2026-05-15-schemas/review-4.md`.
    let mut missing_by_name: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut invalid_by_name: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for problem in &report.problems {
        match problem.kind {
            ValidationProblemKind::Missing => {
                let name = problem
                    .property
                    .clone()
                    .unwrap_or_else(|| problem.path.clone());
                missing_by_name.insert(name);
            }
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                let Some(name) = top_level_pointer_segment(&problem.path) else {
                    continue;
                };
                if value_needs_composition(fm_map.get(&name)) {
                    continue;
                }
                invalid_by_name.insert(name);
            }
        }
    }

    let shape = match effective.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => s,
        // Root-level unions don't expose a single property table; report
        // as raw so the CLI falls back to a minimal listing.
        _ => {
            return Some(SchemaStatusReport {
                source_path: source_path.to_path_buf(),
                required: Vec::new(),
                optional: Vec::new(),
                has_invalid_optional: false,
                raw_json_schema: true,
            });
        }
    };

    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut has_invalid_optional = false;

    for (name, def) in &shape.properties {
        let is_present = fm_map.contains_key(name);
        let is_null = fm_map.get(name).is_some_and(serde_json::Value::is_null);
        let is_missing = missing_by_name.contains(name);
        let is_invalid = invalid_by_name.contains(name);
        // At launch an inline run may leave a required-but-not-eager property
        // for the agent; it is deferred, not missing.
        let deferred = phase == Some(SchemaPhase::Launch)
            && is_required(Some(shape), name)
            && !is_eager(Some(shape), name)
            && (!is_present || is_null);
        let state = if is_invalid {
            PropertyState::Invalid
        } else if deferred {
            PropertyState::Deferred
        } else if is_missing || !is_present {
            PropertyState::Missing
        } else {
            PropertyState::Valid
        };

        let (type_label, description) = match def {
            PropertyDef::Single(atom) => (type_label_for_atom(atom), atom.description.clone()),
            PropertyDef::Union(_) => (
                "<union>".to_string(),
                None,
            ),
        };

        let entry = PropertyStatus {
            name: name.clone(),
            type_label,
            description,
            state,
        };

        if is_required(Some(shape), name) {
            required.push(entry);
        } else {
            if state == PropertyState::Invalid {
                has_invalid_optional = true;
            }
            optional.push(entry);
        }
    }

    Some(SchemaStatusReport {
        source_path: source_path.to_path_buf(),
        required,
        optional,
        has_invalid_optional,
        raw_json_schema: false,
    })
}

#[cfg(test)]
mod semantic_type_tests {
    use super::*;

    #[test]
    fn semantic_schema_types_are_not_single_widget_values() {
        for ty in [SimplifiedType::TypeDefinition, SimplifiedType::Schema] {
            let scalar = PropertyAtom::bare(ty);
            let mut array = PropertyAtom::bare(ty);
            array.is_array = true;

            assert_eq!(interactive_shape_for_atom(&scalar), None);
            assert_eq!(interactive_shape_for_atom(&array), None);
        }
    }
}
