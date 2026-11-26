//! Sequence declarations: where `$state`, `template`, and an inline
//! container's `$schema` may be written, which shapes they accept, and the
//! prepared `$state` contract that judges a list's normalized step states.
//!
//! A sequence owner is a document root carrying `sequence:`, or a task whose
//! `sequence:` is an inline list. Each owner declares its own contract and
//! nothing is inherited:
//!
//! - `$state` types the owner's immediate steps' normalized state;
//! - `template` fills defaults into the owner's immediate items;
//! - `$schema` on an inline container task is a check of the values that
//!   container sees at its turn. A document root's `$schema` is not a
//!   declaration here: it is the document schema and stays with composition.
//!
//! Placement and shape are judged by key *presence* (an explicit `null`
//! included) and before any row is read, so an empty list still rejects a
//! malformed declaration. See `claudine/docs/topics/flow-control/sequences.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationReport};
use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceLoadCause};
use super::super::json_util::json_type_name;
use super::grammar::child_property;
use super::model::SequencePlan;
use super::reserved;

/// The step-state schema key.
pub const STATE_KEY: &str = "$state";
/// The template-defaults key.
pub const TEMPLATE_KEY: &str = "template";
/// The inline container's check-only schema key.
pub const CHECK_KEY: &str = "$schema";

/// The keys an inline container task may declare beside its list.
const TASK_DECLARATION_KEYS: [&str; 3] = [CHECK_KEY, STATE_KEY, TEMPLATE_KEY];

/// The raw, shape-checked declarations of one owner.
///
/// Values borrow the authored map; preparing `state` (and, at its turn,
/// `check`) into a schema is [`PreparedSequenceSchema::prepare`]'s job.
#[derive(Debug, Default, Clone, Copy)]
pub struct OwnerDeclarations<'a> {
    /// The `$state` declaration.
    pub state: Option<&'a Value>,
    /// The `template` defaults.
    pub template: Option<&'a Map<String, Value>>,
    /// An inline container's check-only `$schema`; always `None` for a
    /// document root.
    pub check: Option<&'a Value>,
}

/// Read and judge a sequence document root's declarations.
///
/// `root` is the document's frontmatter (a YAML document's whole root).
/// `$state` and `template` require the root `sequence:` to be an inline list.
///
/// ## Errors
///
/// [`CompositionError::SequenceDeclarationMisplaced`] beside a reference,
/// expression, or shell source; [`CompositionError::SequenceDeclarationShape`]
/// or [`CompositionError::SequenceReservedTemplateKey`] for a malformed one.
pub fn root_declarations<'a>(
    root: &'a Map<String, Value>,
    source_path: &Path,
) -> Result<OwnerDeclarations<'a>, CompositionError> {
    if !matches!(root.get("sequence"), Some(Value::Array(_)))
        && let Some(key) = reserved::SEQUENCE_DECLARATION_KEYS.iter().find(|key| root.contains_key(**key))
    {
        return Err(CompositionError::SequenceDeclarationMisplaced {
            source_path: source_path.to_path_buf(),
            property: (*key).to_string(),
            hint: format!(
                "`{key}` declares the document's own inline `sequence:` list; this document's \
                 `sequence:` is not a list, so put `{key}` in the referenced whole document, \
                 beside its list"
            ),
        });
    }
    Ok(OwnerDeclarations {
        state: root
            .get(STATE_KEY)
            .map(|value| schema_shape(value, source_path, STATE_KEY).map(|()| value))
            .transpose()?,
        template: root
            .get(TEMPLATE_KEY)
            .map(|value| template_shape(value, source_path, TEMPLATE_KEY))
            .transpose()?,
        check: None,
    })
}

/// Read and judge a sequence task's declarations.
///
/// `property` is the task's authored path in `source_path` (`sequence[1]`,
/// `sequence[0].group.tasks[2]`, or empty for a task file's root). Only a
/// task whose `sequence:` is an inline list owns declarations.
///
/// ## Errors
///
/// As [`root_declarations`], located at `property`.
pub fn task_declarations<'a>(
    task: &'a Map<String, Value>,
    source_path: &Path,
    property: &str,
) -> Result<OwnerDeclarations<'a>, CompositionError> {
    let Some(key) = TASK_DECLARATION_KEYS.iter().copied().find(|key| task.contains_key(*key)) else {
        return Ok(OwnerDeclarations::default());
    };
    if !matches!(task.get("sequence"), Some(Value::Array(_))) {
        return Err(CompositionError::SequenceDeclarationMisplaced {
            source_path: source_path.to_path_buf(),
            property: child_property(property, key),
            hint: misplaced_task_hint(task, key),
        });
    }
    inline_container_declarations(task, source_path, property)
}

/// Read and judge the declarations of a task already known to hold an inline
/// `sequence:` list: `options` is the task's map, with or without its
/// executable.
///
/// ## Errors
///
/// [`CompositionError::SequenceDeclarationShape`] or
/// [`CompositionError::SequenceReservedTemplateKey`], located at `property`.
pub fn inline_container_declarations<'a>(
    task: &'a Map<String, Value>,
    source_path: &Path,
    property: &str,
) -> Result<OwnerDeclarations<'a>, CompositionError> {
    let located = |key: &str| child_property(property, key);
    Ok(OwnerDeclarations {
        state: task
            .get(STATE_KEY)
            .map(|value| schema_shape(value, source_path, &located(STATE_KEY)).map(|()| value))
            .transpose()?,
        template: task
            .get(TEMPLATE_KEY)
            .map(|value| template_shape(value, source_path, &located(TEMPLATE_KEY)))
            .transpose()?,
        check: task
            .get(CHECK_KEY)
            .map(|value| schema_shape(value, source_path, &located(CHECK_KEY)).map(|()| value))
            .transpose()?,
    })
}

/// Refuse declarations on a group object: the group grammar is closed, and an
/// inline sequence leg inside its `tasks:` owns its own declarations.
///
/// ## Errors
///
/// [`CompositionError::SequenceDeclarationMisplaced`] at `property.<key>`.
pub fn reject_group_declarations(
    group: &Map<String, Value>,
    source_path: &Path,
    property: &str,
) -> Result<(), CompositionError> {
    match TASK_DECLARATION_KEYS.iter().copied().find(|key| group.contains_key(*key)) {
        Some(key) => Err(CompositionError::SequenceDeclarationMisplaced {
            source_path: source_path.to_path_buf(),
            property: child_property(property, key),
            hint: format!(
                "a group object takes no `{key}`; put it beside an inline `sequence:` list on one \
                 of the group's `tasks:` legs"
            ),
        }),
        None => Ok(()),
    }
}

/// Refuse a value route (a sequence setter, for example) that would write a
/// declaration of the sequence document at `source_path`.
///
/// ## Errors
///
/// [`CompositionError::SequenceDeclarationMisplaced`] naming `key` and
/// `route` (`a setter`, `` `with:` ``, …).
pub fn reject_declaration_write(key: &str, source_path: &Path, route: &str) -> Result<(), CompositionError> {
    match reserved::declaration_written_by(key) {
        Some(declaration) => Err(CompositionError::SequenceDeclarationMisplaced {
            source_path: source_path.to_path_buf(),
            property: key.to_string(),
            hint: format!(
                "`{declaration}` is a sequence declaration, and {route} supplies values, not \
                 declarations; write `{declaration}` in the document, beside its inline `sequence:` list"
            ),
        }),
        None => Ok(()),
    }
}

/// Where a misplaced task declaration belongs, by the task's executable.
fn misplaced_task_hint(task: &Map<String, Value>, key: &str) -> String {
    let owns = format!("`{key}` belongs to an inline sequence container and goes beside its `sequence: [ … ]` list");
    let executable = reserved::EXECUTABLE_KEYS.iter().copied().find(|executable| task.contains_key(*executable));
    let tail = match executable {
        Some("sequence") => "this task references a sequence document; put the declaration at that document's root, beside its list",
        Some("group") => "put it on an inline sequence leg inside the group's `tasks:`",
        Some("task") => "this task references a task file; put the declaration beside an inline list in that file",
        Some(_) => "this task runs no list of its own; wrap it in an inline `sequence:` to give it one",
        None => "a step that runs the document body owns no list",
    };
    format!("{owns}; {tail}")
}

/// Judge a schema declaration's shape: an inline mapping (`{}` is an open
/// schema), a non-empty reference string, or a non-empty root union of
/// mappings and references. Parsing and resolution are left to
/// [`PreparedSequenceSchema::prepare`].
fn schema_shape(value: &Value, source_path: &Path, property: &str) -> Result<(), CompositionError> {
    let invalid = |message: String| CompositionError::SequenceDeclarationShape {
        source_path: source_path.to_path_buf(),
        property: property.to_string(),
        message,
    };
    match value {
        Value::Object(_) => Ok(()),
        Value::String(text) if text.trim().is_empty() => {
            Err(invalid("a schema reference must not be empty".into()))
        }
        Value::String(_) => Ok(()),
        Value::Array(arms) if arms.is_empty() => {
            Err(invalid("a root union needs at least one schema mapping or reference".into()))
        }
        Value::Array(arms) => {
            for (index, arm) in arms.iter().enumerate() {
                let valid = match arm {
                    Value::Object(_) => true,
                    Value::String(text) => !text.trim().is_empty(),
                    _ => false,
                };
                if !valid {
                    return Err(CompositionError::SequenceDeclarationShape {
                        source_path: source_path.to_path_buf(),
                        property: format!("{property}[{index}]"),
                        message: format!(
                            "a root union arm must be a schema mapping or a non-empty reference, got {}",
                            describe(arm)
                        ),
                    });
                }
            }
            Ok(())
        }
        Value::Null => Err(invalid("must not be null; remove the key to declare no schema".into())),
        other => Err(invalid(format!(
            "must be a schema mapping, a schema file reference, or a root union, got {}",
            json_type_name(other)
        ))),
    }
}

/// Judge `template`: a mapping (`{}` is an explicit no-op) whose keys write
/// neither generated/overlay state nor a declaration.
fn template_shape<'a>(
    value: &'a Value,
    source_path: &Path,
    property: &str,
) -> Result<&'a Map<String, Value>, CompositionError> {
    let template = match value {
        Value::Object(map) => map,
        Value::Null => {
            return Err(CompositionError::SequenceDeclarationShape {
                source_path: source_path.to_path_buf(),
                property: property.to_string(),
                message: "must not be null; remove the key to declare no defaults".into(),
            });
        }
        other => {
            return Err(CompositionError::SequenceDeclarationShape {
                source_path: source_path.to_path_buf(),
                property: property.to_string(),
                message: format!("must be an object, got {}", json_type_name(other)),
            });
        }
    };
    // A `$schema` default would land on every item as a container's check.
    if let Some(key) = template.keys().find(|key| reserved::is_reserved_state_key(key) || *key == CHECK_KEY) {
        return Err(CompositionError::SequenceReservedTemplateKey(key.clone()));
    }
    Ok(template)
}

/// A value's type name, with an empty string called out.
fn describe(value: &Value) -> String {
    match value {
        Value::String(_) => "an empty string".into(),
        other => json_type_name(other).to_string(),
    }
}

/// A resolved `$state` (or inline `$schema`) declaration, prepared once per
/// owner and reused for every row it judges.
///
/// Preparation runs in the declaring file's context with no trigger discovery
/// and no baseline: the contract is exactly what the author declared.
/// Validation is passive; the converted working copy a report is built from
/// is discarded, so no step value is coerced or defaulted.
#[derive(Clone)]
pub struct PreparedSequenceSchema {
    /// `None` for a declaration that constrains nothing (`{}`).
    schema: Option<Arc<EffectiveSchema>>,
    source_path: PathBuf,
    property: String,
}

impl std::fmt::Debug for PreparedSequenceSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedSequenceSchema")
            .field("source_path", &self.source_path)
            .field("property", &self.property)
            .field("constrains", &self.schema.is_some())
            .finish()
    }
}

impl PreparedSequenceSchema {
    /// Resolve `declaration`, authored at `property` in `source_path`.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::SequenceDeclarationLoad`] carrying the schema
    /// layer's typed cause.
    pub fn prepare(
        declaration: &Value,
        source_path: &Path,
        property: &str,
        file_resolution_context: &biscuit_file::FileResolutionContext,
    ) -> Result<Self, CompositionError> {
        let load = |source: simplified_schema::SchemaError| CompositionError::SequenceDeclarationLoad {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            source: Box::new(source),
        };
        let mut frontmatter = darkmatter::markdown::Frontmatter::new();
        // `$schema` is a key every frontmatter accepts; a refusal here would be
        // a Darkmatter invariant change, kept typed as the declaring file's.
        frontmatter.insert("$schema", declaration.clone()).map_err(|error| CompositionError::SequenceExternalLoad {
            context: biscuit_file::to_forward_slash_string(source_path),
            source: SequenceLoadCause::Frontmatter(Box::new(error)),
        })?;
        let view = darkmatter::markdown::Markdown::with_frontmatter(frontmatter, "");
        crate::note_work!(SequenceSchemaPrepare, 1);
        let schema = DarkmatterSchemas::new(file_resolution_context.for_source(source_path))
            .effective_for(&view)
            .map_err(load)?;
        Ok(Self {
            schema: schema.map(Arc::new),
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
        })
    }

    /// The file that declared this schema.
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// The declaration's authored path (`$state`, `sequence[1].$state`).
    pub fn property(&self) -> &str {
        &self.property
    }

    /// The schema files this declaration read; a change to one invalidates it.
    pub fn dependencies(&self) -> &[PathBuf] {
        self.schema.as_deref().map_or(&[], EffectiveSchema::dependencies)
    }

    /// Judge `instance` (an object) passively. `None` when the declaration
    /// constrains nothing.
    pub fn validate(&self, instance: &Value) -> Option<ValidationReport> {
        self.schema.as_ref().map(|schema| schema.validate(instance))
    }

    /// Judge every step's normalized state in `plan`, the list authored at
    /// `container` in this declaration's file.
    ///
    /// Only the state portion is judged: executables and task options
    /// (including a nested container's own declarations) were partitioned out
    /// during normalization.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::SequenceStateSchemaViolation`] for the first
    /// failing step.
    pub fn validate_states(&self, plan: &SequencePlan, container: &str) -> Result<(), CompositionError> {
        for step in &plan.steps {
            let Some(report) = self.validate(&step.state.to_value()) else {
                return Ok(());
            };
            if let Some(problem) = report.problems.first() {
                // A top-level missing property has no pointer of its own, so it
                // is named; every other problem, including a field missing
                // inside a value, keeps its instance pointer (the message names
                // the field).
                let property = match &problem.property {
                    Some(name) if problem.path.is_empty() => name.clone(),
                    _ => problem.path.clone(),
                };
                return Err(CompositionError::SequenceStateSchemaViolation {
                    index: step.index,
                    id: step.state.id.clone(),
                    property,
                    message: problem.message.clone(),
                    source_path: self.source_path.clone(),
                    container: container.to_string(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
