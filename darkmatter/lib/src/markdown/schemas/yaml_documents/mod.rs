//! Validation of whole YAML documents (`.yaml`, `.yml`).
//!
//! Loading, kind selection, the [`KindRegistry`], and the checks of the
//! language's own kinds are the library's
//! ([`simplified_schema::yaml_documents`]). This module adapts them to
//! Darkmatter: it supplies Darkmatter's shipped sources and prepared shipped
//! definitions ([`kind_host`]), prepares the selected schema as an
//! [`EffectiveSchema`], and reports through a [`ValidationReport`].
//!
//! A YAML document selects its schema by its top-level `kind` when a
//! `kind: yaml-schema` definition claims it, and otherwise by a root
//! `$schema`; a document with neither is not validated. The attached
//! registered hosts' definitions
//! ([`DarkmatterSchemas::with_registered_hosts`]) claim only a kind the
//! shipped and local definitions establish is unclaimed. The selected schema
//! alone validates the document: no Markdown baseline and no trigger layers
//! apply, on either route. Validation is soft, on a converted copy that is
//! never written back, and whole-value `{{ … }}` / `$(…)` strings are
//! ordinary data. The language's own kinds (`schema`, `schema-trigger`,
//! `yaml-schema`) then get the checks their definitions cannot state, in the
//! same report.
//!
//! A configuration file has no mandatory `kind`, so a caller that knows a
//! document is one (DMLS, by its exact location) prepares the configuration
//! schema it chose with [`DarkmatterSchemas::prepare_configuration_contract`]
//! instead, and that document's `kind` and `$schema` select nothing.
//!
//! The user-facing contract is the "YAML documents" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use simplified_schema::yaml_documents::{
    DefinitionSource, FILE_READING_KINDS, KindHost, KindRegistry, LoadedYaml, LoadedYamlDocument, RegisteredSource,
    RegistryScope, Selection, select_with_registered,
};
use simplified_schema::triggers::assemble;
use simplified_schema::{SchemaError, ValidationProblem};

use super::embedded::SHIPPED_SOURCES;
use super::{DarkmatterSchemas, EffectiveSchema, ValidationReport};

/// The diagnostic code of a [`SchemaError::CallableCatalog`] failure, as
/// `md schema validate` and the editor report it.
pub const CALLABLE_CATALOG_CODE: &str = "dm.schema.callable_catalog";

/// Darkmatter's [`KindHost`]: its shipped sources and the shipped
/// definitions its accepted artifact recorded (or, when the artifact or its
/// section was refused, the definitions prepared from text). Every registry
/// Darkmatter, its CLI, and DMLS build passes it.
#[must_use]
pub fn kind_host() -> KindHost<'static> {
    KindHost { sources: SHIPPED_SOURCES, shipped: super::shipped::registry().kind_definitions() }
}

/// The registry of the shipped definitions alone, built once per process
/// from [`kind_host`]'s definitions.
#[must_use]
pub fn shipped_kind_registry() -> Arc<KindRegistry> {
    super::shipped::registry().shipped_kind_registry()
}

/// The route a validated YAML document took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Its `kind` (as authored) is claimed by a definition.
    Kind(String),
    /// No claimed `kind`; its root `$schema` was the contract.
    RootSchema,
    /// The document is a configuration file, selected by its path before
    /// any `kind` or `$schema` was consulted
    /// ([`DarkmatterSchemas::prepare_configuration_contract`]). Its `kind`
    /// and `$schema`, if authored, are data the contract judges.
    Configuration,
}

/// The result of validating one YAML document.
// Returned once per document and matched at once; boxing the validation
// would only add an allocation to the common case.
#[allow(clippy::large_enum_variant)]
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
    loaded: Arc<LoadedYaml>,
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
        let (path, loaded) = (document.path(), document.loaded());
        let context = &self.file_resolution_context;
        let registry = match &self.kind_registry {
            Some(registry) => Arc::clone(registry),
            None => Arc::new(KindRegistry::from_filesystem(context, RegistryScope::for_context(context), kind_host())),
        };
        let base_dir = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| context.cwd().to_path_buf(), Path::to_path_buf);
        let schema_roots = registry.roots().search_paths().to_vec();
        // Registered hosts' definitions are the lowest tier: consulted only
        // for a kind the shipped and local definitions leave unclaimed.
        let registered_kinds = self.registered_hosts.as_deref().map(crate::config::registration::RegisteredHosts::yaml_kinds);
        let selection = select_with_registered(&registry, registered_kinds, &loaded.value)?;
        if !matches!(selection, Selection::NoSchema) {
            crate::note_work!(YamlContractPreparation);
        }
        let mut registered = None;
        let (route, definition, effective) = match selection {
            Selection::NoSchema => return Ok(None),
            Selection::Kind { kind, definition } => {
                // A callable catalog's semantics are judged by the catalog
                // reader before its kind contract. The shared reader's
                // located conflicts use this route as well. Shape and syntax
                // refusals without a competing declaration stay with the
                // kind contract.
                if let Err(error @ SchemaError::CallableCatalog { .. }) = self.validate_callable_catalog(document) {
                    return Err(error);
                }
                let effective = EffectiveSchema::for_yaml_document(definition.resolved().clone(), base_dir, self)?;
                if let DefinitionSource::Registered(source) = definition.source() {
                    registered = Some(source.clone());
                }
                (Route::Kind(kind), definition.source().label(), effective)
            }
            Selection::RootSchema(value) => {
                let json = serde_json::to_value(value).map_err(|error| SchemaError::FrontmatterShape {
                    message: format!("$schema is not representable as JSON: {error}"),
                })?;
                // An inline `$schema`'s slots are positioned in this file.
                let authored = super::mentions_slot(&json).then(|| simplified_schema::resolve::AuthoredSchemaValue {
                    file: Some(path.to_path_buf()),
                    mapping: 0..document.text().len(),
                    text: document.text().into(),
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
                        .map_err(|error| match assemble::refused_trigger_file(&error) {
                            Some(refused) => assemble::trigger_schema_referenced(&refused.path, refused.site),
                            None => error,
                        })?;
                    assemble::check_document_schema_not_trigger(&resolved.referenced_files)?;
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
            registered,
            effective: Arc::new(effective),
            schema_roots,
            context: context.clone(),
        }))
    }
}

impl DarkmatterSchemas {
    /// Judges `document` as a callable catalog when its `kind` is
    /// `expression-schema` or `mutation-schema`, with the one catalog reader
    /// registration uses; `Ok` for every other document. Passive: nothing is
    /// resolved, read, or evaluated.
    ///
    /// The built-in catalogs (`darkmatter/schemas/partials/functions.yaml`
    /// and `mutations.yaml`, matched by path identity: that relative path, or
    /// the same path under this instance's captured repository root) admit
    /// their own names and read their compiled-in file-form examples. Every
    /// other catalog is judged as a host would author it: no file-form
    /// examples, and an expression callable may not reuse a built-in
    /// function's canonical name or alias.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::CallableCatalog`] for a failed callable check, located
    /// in `document`'s text, with both declarations when they conflict.
    /// Shape and other syntax refusals retain the shared reader's error.
    pub fn validate_callable_catalog(&self, document: &LoadedYamlDocument) -> Result<(), SchemaError> {
        use crate::markdown::compose::expression::catalog::{self as catalog, CatalogError};
        use simplified_schema::simplified::StandaloneSchemaEnvelope;

        let envelope = match document.loaded().value.get("kind").and_then(serde_yaml_ng::Value::as_str) {
            Some("expression-schema") => StandaloneSchemaEnvelope::ExpressionSchema,
            Some("mutation-schema") => StandaloneSchemaEnvelope::MutationSchema,
            _ => return Ok(()),
        };
        let (path, source) = (document.path(), document.text());
        let result = match (self.builtin_catalog(path), envelope) {
            (Some(BuiltInCatalog::Functions), StandaloneSchemaEnvelope::ExpressionSchema) => {
                catalog::load_catalog(source, path, envelope, catalog::EXAMPLE_FILES)
            }
            (Some(BuiltInCatalog::Mutations), StandaloneSchemaEnvelope::MutationSchema) => {
                catalog::load_catalog(source, path, envelope, &[])
            }
            _ => catalog::load_authored_catalog(source, path, envelope),
        };
        result.map(drop).map_err(CatalogError::into_schema_error)
    }

    /// Which built-in catalog `path` is, by path identity: the catalog's
    /// repository-relative path itself, or that path under the captured
    /// repository root. A file name alone never matches.
    fn builtin_catalog(&self, path: &Path) -> Option<BuiltInCatalog> {
        use biscuit_file::{PathIdentity, canonicalize_simplified};
        let context = &self.file_resolution_context;
        let identity = |path: &Path| PathIdentity::new(&canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf()));
        let absolute = if path.is_absolute() { path.to_path_buf() } else { context.cwd().join(path) };
        let target = identity(&absolute);
        [
            (crate::markdown::compose::expression::catalog::CATALOG_PATH, BuiltInCatalog::Functions),
            (crate::effects::types::MUTATIONS_PATH, BuiltInCatalog::Mutations),
        ]
        .into_iter()
        .find(|(relative, _)| {
            PathIdentity::new(path) == PathIdentity::new(Path::new(relative))
                || context.repository_root().is_some_and(|root| identity(&root.join(relative)) == target)
        })
        .map(|(_, catalog)| catalog)
    }

    /// Prepares `resolved`, the configuration schema the caller selected for
    /// the YAML document at `path` by its location, as that document's
    /// contract ([`Route::Configuration`]). `definition` names the schema in
    /// reports. No kind registry, `kind`, or `$schema` is consulted, and an
    /// empty document is judged as an empty mapping.
    ///
    /// ## Errors
    ///
    /// Validator-construction errors.
    pub fn prepare_configuration_contract(
        &self,
        resolved: simplified_schema::resolve::ResolvedSchema,
        definition: impl Into<String>,
        path: &Path,
    ) -> Result<YamlContract, SchemaError> {
        let context = &self.file_resolution_context;
        let base_dir = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| context.cwd().to_path_buf(), Path::to_path_buf);
        crate::note_work!(YamlContractPreparation);
        let effective = EffectiveSchema::for_yaml_document(resolved, base_dir, self)?;
        effective.prepare_conversion();
        Ok(YamlContract {
            route: Route::Configuration,
            definition: definition.into(),
            registered: None,
            effective: Arc::new(effective),
            schema_roots: Vec::new(),
            context: context.clone(),
        })
    }
}

/// A built-in callable catalog, by source identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltInCatalog {
    Functions,
    Mutations,
}

/// The schema a whole YAML document selected, prepared and compiled
/// ([`DarkmatterSchemas::prepare_yaml_contract`]), ready to judge the
/// document's text.
#[derive(Clone)]
pub struct YamlContract {
    route: Route,
    definition: String,
    /// The registered host definition that was selected, when the kind came
    /// from the registered tier.
    registered: Option<RegisteredSource>,
    effective: Arc<EffectiveSchema>,
    schema_roots: Vec<PathBuf>,
    context: biscuit_file::FileResolutionContext,
}

impl std::fmt::Debug for YamlContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YamlContract")
            .field("route", &self.route)
            .field("definition", &self.definition)
            .field("registered", &self.registered)
            .finish_non_exhaustive()
    }
}

impl YamlContract {
    /// How the schema was selected.
    #[must_use]
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// The registered host and definition that judge the document, when its
    /// `kind` was left unclaimed by the shipped and local definitions and a
    /// registered host's `kind: yaml-schema` claims it.
    #[must_use]
    pub fn registered(&self) -> Option<&RegisteredSource> {
        self.registered.as_ref()
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
        matches!(&self.route, Route::Kind(kind) if FILE_READING_KINDS.contains(&kind.as_str()))
    }

    /// Judges `document`, which must have the
    /// [`declaration_key`](LoadedYamlDocument::declaration_key) of the
    /// document this contract was prepared from, against the prepared
    /// schema, then (only when the schema reports nothing) runs its kind's
    /// checks over the copy the schema accepted; when it reports a problem
    /// at a kind's `$schema` or `types` carrier whose declaration does not
    /// parse, the problem is narrowed to the failing declaration
    /// ([`LoadedYamlDocument::locate_declaration_problems`]). Passive unless
    /// [`Self::checks_read_files`].
    #[must_use]
    pub fn validate(&self, document: &LoadedYamlDocument) -> YamlValidation {
        let loaded = document.loaded();
        let mut instance = loaded.json.clone();
        // A newly created, still empty configuration file keeps every default.
        if self.route == Route::Configuration && instance.is_null() {
            instance = serde_json::Value::Object(serde_json::Map::new());
        }
        if self.route == Route::RootSchema
            && let serde_json::Value::Object(map) = &mut instance
        {
            map.shift_remove("$schema");
        }
        let mut report = self.effective.validate_data(&instance, &loaded.positions);
        if let Route::Kind(kind) = &self.route
            && simplified_schema::yaml_documents::CHECKED_KINDS.contains(&kind.as_str())
        {
            if report.valid {
                let accepted = self.effective.data_copy(&instance);
                report.problems = document.kind_checks(kind, &accepted, &self.schema_roots, &self.context, SHIPPED_SOURCES);
                report.valid = report.problems.is_empty();
            } else {
                document.locate_declaration_problems(kind, &mut report.problems);
            }
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
