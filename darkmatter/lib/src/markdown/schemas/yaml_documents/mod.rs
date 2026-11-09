//! Validation of whole YAML documents (`.yaml`, `.yml`).
//!
//! A YAML document selects its schema by its top-level `kind` when a
//! `kind: yaml-schema` definition claims it, and otherwise by a root
//! `$schema`; a document with neither is not validated ([`select`]). The
//! claims of one document context are its [`KindRegistry`]. The selected
//! schema alone validates the document: no Markdown baseline and no trigger
//! layers apply, on either route. Validation is soft, on a converted copy
//! that is never written back, and whole-value `{{ … }}` / `$(…)` strings are
//! ordinary data. Darkmatter's own kinds (`schema`, `schema-trigger`,
//! `yaml-schema`) then get the checks their definitions cannot state
//! ([`semantic`]), in the same report.
//!
//! The user-facing contract is the "YAML documents" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use simplified_schema::{SchemaError, ValidationProblem};

use super::{DarkmatterSchemas, EffectiveSchema, ValidationReport};

mod load;
mod marks;
pub mod registry;
mod select;
mod semantic;

pub(crate) use load::{first_tag_start, load_yaml_document, locating_problem, pointer_string};
pub use registry::{Claim, DefinitionSource, Incomplete, KindDefinition, KindRegistry, RegistryScope};

/// The route a validated YAML document took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Its `kind` (as authored) is claimed by a definition.
    Kind(String),
    /// No claimed `kind`; its root `$schema` was the contract.
    RootSchema,
}

/// The result of validating one YAML document.
#[derive(Debug, Clone)]
pub enum YamlOutcome {
    /// No schema was selected: the root is not a mapping, or it has neither
    /// a claimed `kind` nor a `$schema`. Nothing was checked.
    NoSchema,
    /// A schema was selected and the document judged against it.
    Validated(YamlValidation),
}

/// A YAML document judged against its selected schema.
#[derive(Clone)]
pub struct YamlValidation {
    /// How the schema was selected.
    pub route: Route,
    /// The definition's problems, then (only when there are none) the kind's
    /// own checks, located against the original text.
    pub report: ValidationReport,
    /// What validated it: the definition's label, or the `$schema` it named.
    pub definition: String,
    effective: Arc<EffectiveSchema>,
    loaded: Arc<load::LoadedYaml>,
}

impl std::fmt::Debug for YamlValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YamlValidation")
            .field("route", &self.route)
            .field("report", &self.report)
            .field("definition", &self.definition)
            .finish_non_exhaustive()
    }
}

impl YamlValidation {
    /// The byte range, in the validated text, of the token `problem` is
    /// reported at; `None` when it cannot be located.
    #[must_use]
    pub fn byte_range(&self, problem: &ValidationProblem) -> Option<Range<usize>> {
        self.loaded.byte_range(problem)
    }

    /// The selected schema the document was judged against: the definition
    /// alone, or the root `$schema` alone, with no baseline or trigger layer.
    #[must_use]
    pub fn effective(&self) -> &Arc<EffectiveSchema> {
        &self.effective
    }

    /// The whole document as JSON, as loaded (before soft conversion, and
    /// with a fallback-route `$schema` still present).
    #[must_use]
    pub fn document_json(&self) -> &serde_json::Value {
        &self.loaded.json
    }
}

impl DarkmatterSchemas {
    /// This instance selecting YAML document kinds from `registry` instead
    /// of building a registry for each call (DMLS supplies its cached one).
    /// The registry must have been built for this instance's context.
    #[must_use]
    pub fn with_kind_registry(mut self, registry: Arc<KindRegistry>) -> Self {
        self.kind_registry = Some(registry);
        self
    }

    /// Validates `text`, the whole YAML document at `path`, against the
    /// schema it selects.
    ///
    /// This instance's context must be the document's own context, as for
    /// Markdown validation: its schema roots supply the kind definitions and
    /// resolve the document's relative file values. Without
    /// [`Self::with_kind_registry`], the registry is built for the call, with
    /// [`RegistryScope::for_context`]. The baseline and the trigger registry
    /// are never consulted.
    ///
    /// ## Errors
    ///
    /// - [`SchemaError::SchemaDocument`] naming `path`, with a span, when the
    ///   text is not one admissible YAML document (malformed YAML, a second
    ///   document, a duplicate or non-string key, an inadmissible number).
    /// - [`SchemaError::KindSelection`] when the document's `kind` is claimed
    ///   ambiguously, by a failed definition, or cannot be ruled out as
    ///   unclaimed.
    /// - Every schema-resolution error of a root `$schema` (a present `null`
    ///   included), and validator-construction errors.
    pub fn validate_yaml_document(&self, text: &str, path: &Path) -> Result<YamlOutcome, SchemaError> {
        self.validate_loaded_yaml_document(&LoadedYamlDocument::load(text, path)?)
    }

    /// Validates a document [`LoadedYamlDocument::load`] already admitted;
    /// [`Self::validate_yaml_document`] is the load and this call in one.
    /// Every error is a schema or selection error, never a load error.
    ///
    /// ## Errors
    ///
    /// The selection and schema errors of [`Self::validate_yaml_document`].
    pub fn validate_loaded_yaml_document(&self, document: &LoadedYamlDocument) -> Result<YamlOutcome, SchemaError> {
        Ok(match self.prepare_yaml_contract(document)? {
            None => YamlOutcome::NoSchema,
            Some(contract) => YamlOutcome::Validated(contract.validate(document)),
        })
    }

    /// Selects and prepares the schema `document` is judged against, without
    /// judging it: the selection, the resolution of a root `$schema` (its
    /// files read), and the compiled validators. The result judges any later
    /// text of the same document whose [`LoadedYamlDocument::declaration_key`]
    /// is unchanged, in this instance's context
    /// ([`YamlContract::validate`]). [`Self::validate_loaded_yaml_document`]
    /// is this call and that one.
    ///
    /// ## Returns
    ///
    /// `None` when no schema is selected.
    ///
    /// ## Errors
    ///
    /// The selection and schema errors of [`Self::validate_yaml_document`].
    pub fn prepare_yaml_contract(&self, document: &LoadedYamlDocument) -> Result<Option<YamlContract>, SchemaError> {
        let (path, loaded) = (document.path.as_path(), &document.loaded);
        let context = &self.file_resolution_context;
        let registry = match &self.kind_registry {
            Some(registry) => Arc::clone(registry),
            None => Arc::new(KindRegistry::from_filesystem(context, RegistryScope::for_context(context))),
        };
        let base_dir = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| context.cwd().to_path_buf(), Path::to_path_buf);
        let schema_roots = registry.roots().search_paths().to_vec();
        let selection = select::select(&registry, &loaded.value)?;
        if !matches!(selection, select::Selection::NoSchema) {
            crate::note_work!(YamlContractPreparation);
        }
        let (route, definition, effective) = match selection {
            select::Selection::NoSchema => return Ok(None),
            select::Selection::Kind { kind, definition } => {
                let effective = EffectiveSchema::for_yaml_document(definition.resolved().clone(), base_dir, self)?;
                (Route::Kind(kind), definition.source().label(), effective)
            }
            select::Selection::RootSchema(value) => {
                let json = serde_json::to_value(value).map_err(|error| SchemaError::FrontmatterShape {
                    message: format!("$schema is not representable as JSON: {error}"),
                })?;
                // An inline `$schema`'s slots are positioned in this file.
                let authored = super::mentions_slot(&json).then(|| simplified_schema::resolve::AuthoredSchemaValue {
                    file: Some(path.to_path_buf()),
                    mapping: 0..document.text.len(),
                    text: document.text.as_str().into(),
                });
                // The trigger check reads the bytes the resolution just read.
                let resolved = simplified_schema::resolve::buffers::with_reads_retained(|| {
                    let resolved = super::resolve::resolve_authored_schema_with_roots(
                        &json,
                        authored.as_ref(),
                        &base_dir,
                        &schema_roots,
                        context,
                    )
                        // A template resolves through the slot session, as a
                        // Markdown document's `$schema` does.
                        .or_else(|error| self.resolve_template(error, &super::document_template_identity(&json, &base_dir)))
                        .map_err(|error| match super::triggers::assemble::refused_trigger_file(&error) {
                            Some(path) => super::triggers::assemble::trigger_schema_referenced(&path),
                            None => error,
                        })?;
                    super::triggers::assemble::check_document_schema_not_trigger(&resolved.referenced_files)?;
                    Ok::<_, SchemaError>(resolved)
                })?;
                let label = match value.as_str() {
                    Some(reference) => reference.to_string(),
                    None => "the inline `$schema`".to_string(),
                };
                (Route::RootSchema, label, EffectiveSchema::for_yaml_document(resolved, base_dir, self)?)
            }
        };
        effective.prepare_conversion();
        Ok(Some(YamlContract {
            route,
            definition,
            effective: Arc::new(effective),
            schema_roots,
            context: context.clone(),
        }))
    }
}

/// The schema a whole YAML document selected, prepared and compiled
/// ([`DarkmatterSchemas::prepare_yaml_contract`]), ready to judge the
/// document's text.
#[derive(Clone)]
pub struct YamlContract {
    route: Route,
    definition: String,
    effective: Arc<EffectiveSchema>,
    schema_roots: Vec<PathBuf>,
    context: biscuit_file::FileResolutionContext,
}

impl std::fmt::Debug for YamlContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YamlContract")
            .field("route", &self.route)
            .field("definition", &self.definition)
            .finish_non_exhaustive()
    }
}

impl YamlContract {
    /// How the schema was selected.
    #[must_use]
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// The selected schema alone, as [`YamlValidation::effective`].
    #[must_use]
    pub fn effective(&self) -> &Arc<EffectiveSchema> {
        &self.effective
    }

    /// Whether [`Self::validate`] reads files: the kind checks of a
    /// `schema` or `yaml-schema` document resolve what the document itself
    /// imports, so they depend on its whole text. Every other judgment is
    /// passive.
    #[must_use]
    pub fn checks_read_files(&self) -> bool {
        matches!(&self.route, Route::Kind(kind) if semantic::FILE_READING_KINDS.contains(&kind.as_str()))
    }

    /// Judges `document`, which must have the
    /// [`declaration_key`](LoadedYamlDocument::declaration_key) of the
    /// document this contract was prepared from, against the prepared
    /// schema, then (only when the schema reports nothing) runs its kind's
    /// checks over the copy the schema accepted. Passive unless
    /// [`Self::checks_read_files`].
    #[must_use]
    pub fn validate(&self, document: &LoadedYamlDocument) -> YamlValidation {
        let (text, path, loaded) = (document.text.as_str(), document.path.as_path(), &document.loaded);
        let mut instance = loaded.json.clone();
        if self.route == Route::RootSchema
            && let serde_json::Value::Object(map) = &mut instance
        {
            map.shift_remove("$schema");
        }
        let mut report = self.effective.validate_data(&instance, &loaded.positions);
        if report.valid
            && let Route::Kind(kind) = &self.route
            && semantic::CHECKED_KINDS.contains(&kind.as_str())
        {
            let accepted = self.effective.data_copy(&instance);
            report.problems =
                semantic::kind_checks(kind, text, path, loaded, &accepted, &self.schema_roots, &self.context);
            report.valid = report.problems.is_empty();
        }
        YamlValidation {
            route: self.route.clone(),
            report,
            definition: self.definition.clone(),
            effective: Arc::clone(&self.effective),
            loaded: Arc::clone(loaded),
        }
    }
}

/// A whole YAML document admitted by the one YAML document loader, ready for
/// [`DarkmatterSchemas::validate_loaded_yaml_document`].
///
/// Loading is separate from validation so a caller can tell a document that
/// is not admissible YAML from one whose schema failed: both are
/// [`SchemaError`]s, and a `$schema` may name a file whose own load error
/// looks exactly like the document's.
#[derive(Debug, Clone)]
pub struct LoadedYamlDocument {
    text: String,
    path: PathBuf,
    loaded: Arc<load::LoadedYaml>,
}

impl LoadedYamlDocument {
    /// Loads `text`, the whole YAML document at `path`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::SchemaDocument`] naming `path`, with a span, when the
    /// text is not one admissible YAML document (malformed YAML, a second
    /// document, a duplicate or non-string key, an inadmissible number).
    pub fn load(text: &str, path: &Path) -> Result<Self, SchemaError> {
        let loaded = Arc::new(load::load_yaml_document(text, path)?);
        Ok(Self { text: text.to_string(), path: path.to_path_buf(), loaded })
    }

    /// A hash of what [`DarkmatterSchemas::prepare_yaml_contract`] reads
    /// from this document: its top-level `kind` and root `$schema`, as
    /// authored. Two texts of one document with the same key select and
    /// prepare the same contract in the same context and registry, so a
    /// prepared [`YamlContract`] judges both.
    #[must_use]
    pub fn declaration_key(&self) -> u64 {
        let map = self.loaded.value.as_mapping();
        let declaration = (
            map.is_some(),
            map.and_then(|map| map.get("kind")),
            map.and_then(|map| map.get("$schema")),
        );
        // The loader admitted these values (string keys, admissible
        // numbers), so serializing them does not fail.
        biscuit_hash::xx_hash(&serde_yaml_ng::to_string(&declaration).unwrap_or_default())
    }

    /// Whether this document's `kind` names a kind whose checks read files
    /// (`schema`, `yaml-schema`): judging it then depends on its whole text,
    /// not only on its [`Self::declaration_key`]
    /// ([`YamlContract::checks_read_files`]).
    #[must_use]
    pub fn kind_checks_read_files(&self) -> bool {
        self.loaded
            .value
            .get("kind")
            .and_then(serde_yaml_ng::Value::as_str)
            .is_some_and(|kind| semantic::FILE_READING_KINDS.contains(&kind))
    }

    /// The whole document as JSON, exactly as loaded: no soft conversion,
    /// and a root `$schema` still present. The same value
    /// [`YamlValidation::document_json`] returns after validation.
    #[must_use]
    pub fn json(&self) -> &serde_json::Value {
        &self.loaded.json
    }
}

/// The definition paths every YAML document kind registry starts from.
#[must_use]
pub fn shipped_kind_definition_paths() -> Vec<PathBuf> {
    super::embedded::shipped_kind_definitions().iter().map(|document| PathBuf::from(document.path)).collect()
}
