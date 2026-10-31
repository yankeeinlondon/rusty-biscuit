//! Validation of whole YAML documents by their top-level `kind`.
//!
//! A YAML document whose top-level `kind` is a shipped kind is validated
//! against that kind's definition, a `kind: schema` document compiled into
//! the library ([`SHIPPED_KINDS`]). Those definitions are the authority for
//! each kind's structure: `darkmatter/schemas/schema.yaml` for `kind: schema`
//! and `darkmatter/schemas/schema-trigger.yaml` for `kind: schema-trigger`.
//! The Rust envelope parsers keep only the rules a definition cannot express.
//!
//! ## Contract
//!
//! - Validation is passive: no file reads, expression evaluation, shell
//!   execution, or remote fetches. The caller supplies the text.
//! - The document is loaded by the loader standalone schema recognition uses,
//!   so a duplicate key, a second YAML document, or invalid YAML fails the same
//!   way there and here.
//! - The whole mapping is validated raw: no coercion, and `$schema` is kept as
//!   data (in a kind document it is content, not a reference to follow).
//! - A document is claimed only when its `kind` is a string naming a shipped
//!   kind. Anything else (no `kind`, a null or non-string `kind`, another
//!   string, or a document that is not a mapping) is unclaimed and has no
//!   kind problems.

use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

use serde_json::Value;
use simplified_schema::{
    MappingOrders, PositionMap, SchemaError, embedded::EmbeddedDocument, resolve::ResolvedSchema,
};

use super::{DarkmatterSchemas, EffectiveSchema, ValidationReport, embedded, resolve};

/// The shipped kinds and the compiled-in definition each one validates
/// against, as `(kind, definition path)`.
pub const SHIPPED_KINDS: &[(&str, &str)] = &[
    ("schema", "darkmatter/schemas/schema.yaml"),
    ("schema-trigger", "darkmatter/schemas/schema-trigger.yaml"),
];

/// Resolved kind definitions, keyed by the `kind` value that claims each.
#[derive(Debug, Clone)]
pub struct KindDocuments {
    kinds: Vec<(String, ResolvedSchema)>,
}

static SHIPPED_REGISTRY: OnceLock<KindDocuments> = OnceLock::new();

impl KindDocuments {
    /// The shipped registry ([`SHIPPED_KINDS`]), prepared once per process.
    ///
    /// ## Panics
    ///
    /// Panics if a compiled-in definition fails to prepare. This is a library
    /// bug, not an author error.
    pub fn shipped() -> &'static KindDocuments {
        SHIPPED_REGISTRY.get_or_init(|| {
            Self::from_documents(SHIPPED_KINDS, embedded::SHIPPED)
                .unwrap_or_else(|error| panic!("a shipped kind definition must prepare: {error}"))
        })
    }

    /// A registry of `(kind, path)` entries, each path resolved in
    /// `documents` by the resolver [`Self::shipped`] uses. Meant for tests
    /// that change a definition without changing Rust.
    ///
    /// The definitions are prepared before anything is validated, so a
    /// definition never validates itself while it is being prepared.
    ///
    /// ## Errors
    ///
    /// The [`SchemaError`] of a definition that does not resolve or lower.
    pub fn from_documents(
        kinds: &[(&str, &str)],
        documents: &[EmbeddedDocument<'_>],
    ) -> Result<Self, SchemaError> {
        let kinds = kinds
            .iter()
            .map(|(kind, path)| {
                resolve::prepare_embedded_schema_in(path, documents, "")
                    .map(|(resolved, _)| ((*kind).to_owned(), resolved))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { kinds })
    }

    /// The kind `document` claims: its top-level `kind` when that is a string
    /// naming a registered kind, else `None`.
    pub fn claimed_kind<'a>(&self, document: &'a Value) -> Option<&'a str> {
        let kind = document.as_object()?.get("kind")?.as_str()?;
        self.resolved(kind).map(|_| kind)
    }

    fn resolved(&self, kind: &str) -> Option<&ResolvedSchema> {
        self.kinds.iter().find(|(name, _)| name == kind).map(|(_, resolved)| resolved)
    }
}

/// The outcome of validating a claimed kind document.
#[derive(Debug, Clone)]
pub struct KindValidation {
    /// The `kind` that claimed the document.
    pub kind: String,
    /// The raw validation report, with problems located in the document text.
    pub report: ValidationReport,
}

impl DarkmatterSchemas {
    /// This instance validating kind documents against `kinds` instead of the
    /// shipped registry.
    #[must_use]
    pub fn with_kind_documents(mut self, kinds: KindDocuments) -> Self {
        self.kind_documents = Some(Arc::new(kinds));
        self
    }

    fn kind_registry(&self) -> &KindDocuments {
        match &self.kind_documents {
            Some(kinds) => kinds,
            None => KindDocuments::shipped(),
        }
    }

    /// The effective schema `kind`'s definition validates against, or `None`
    /// when `kind` is not registered. It has no baseline and no trigger
    /// layers, and resolves through this instance's context.
    ///
    /// ## Errors
    ///
    /// The [`SchemaError`] of a validator that cannot be compiled.
    pub fn effective_for_kind(&self, kind: &str) -> Result<Option<EffectiveSchema>, SchemaError> {
        self.kind_registry()
            .resolved(kind)
            .map(|resolved| self.effective_for_resolved(resolved))
            .transpose()
    }

    /// Validates the YAML document `text` against the definition its
    /// top-level `kind` claims.
    ///
    /// `path` names the document in errors only; nothing is read from it.
    ///
    /// ## Returns
    ///
    /// `Ok(None)` for an unclaimed document (see the module docs), otherwise
    /// the claiming kind and the raw report. Problems carry the line and
    /// column of the offending key or value in `text`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::SchemaDocument`] when `text` does not load (invalid
    /// YAML, a duplicate key, more than one document, an inadmissible number)
    /// or its mapping cannot be represented as data (a non-string key); the
    /// [`SchemaError`] of a definition whose validator cannot be compiled.
    pub fn validate_kind_document(
        &self,
        text: &str,
        path: &Path,
    ) -> Result<Option<KindValidation>, SchemaError> {
        let document_error = |message: String| SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message,
            source: None,
            span: None,
        };
        let yaml = simplified_schema::numeric::load_admitted_yaml(text).map_err(|error| {
            // The wording standalone schema recognition uses for the same loader.
            if simplified_schema::numeric::is_yaml_admission_error(&error) {
                document_error(error.to_string())
            } else {
                document_error(format!("not valid YAML: {error}"))
            }
        })?;
        if !yaml.is_mapping() {
            return Ok(None);
        }
        let orders = MappingOrders::collect(&yaml);
        let document: Value = serde_yaml_ng::from_value(yaml)
            .map_err(|error| document_error(format!("cannot be read as a kind document: {error}")))?;
        let registry = self.kind_registry();
        let Some(kind) = registry.claimed_kind(&document) else {
            return Ok(None);
        };
        let effective = self
            .effective_for_kind(kind)?
            .expect("a claimed kind is registered");
        let positions = PositionMap::project(text, 0..text.len(), 1, &orders)
            .unwrap_or_else(|| PositionMap::top_level(text, 1));
        Ok(Some(KindValidation {
            kind: kind.to_owned(),
            report: effective.validate_raw_with_positions(&document, &positions),
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use simplified_schema::StandaloneSchemaEnvelope;

    use crate::markdown::schemas::triggers::envelope;

    /// Every key either envelope could plausibly name; the tie tests check
    /// each against both the Rust envelope and the YAML definition.
    const CANDIDATE_KEYS: &[&str] =
        &["kind", "$schema", "types", "description", "categories", "match", "name", "any"];

    /// The root arms of a lowered definition: its `anyOf`/`oneOf` arms, or the
    /// root itself.
    fn root_arms(json: &Value) -> Vec<&Value> {
        ["anyOf", "oneOf"]
            .iter()
            .find_map(|key| json.get(*key).and_then(Value::as_array))
            .map_or_else(|| vec![json], |arms| arms.iter().collect())
    }

    fn property_names(json: &Value) -> BTreeSet<String> {
        root_arms(json)
            .into_iter()
            .flat_map(|arm| arm.get("properties").and_then(Value::as_object).into_iter().flat_map(|props| props.keys()))
            .cloned()
            .collect()
    }

    fn required_sets(json: &Value) -> BTreeSet<BTreeSet<String>> {
        root_arms(json)
            .into_iter()
            .map(|arm| match arm.get("required") {
                // JSON Schema: an absent `required` requires nothing.
                None => BTreeSet::new(),
                Some(names) => names
                    .as_array()
                    .expect("`required` is a list")
                    .iter()
                    .map(|name| name.as_str().expect("a required name is a string").to_owned())
                    .collect(),
            })
            .collect()
    }

    fn kind_literal(json: &Value) -> BTreeSet<String> {
        root_arms(json)
            .into_iter()
            .map(|arm| arm.pointer("/properties/kind/const").and_then(Value::as_str).expect("every arm fixes `kind`"))
            .map(str::to_owned)
            .collect()
    }

    fn definition(kind: &str) -> &'static Value {
        &KindDocuments::shipped().resolved(kind).expect("shipped kind").json_schema
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn every_shipped_kind_definition_is_compiled_in() {
        for (kind, path) in SHIPPED_KINDS {
            assert!(
                embedded::SHIPPED.iter().any(|document| document.path == *path),
                "`{kind}`'s definition `{path}` is not in `SHIPPED`"
            );
        }
    }

    #[test]
    fn the_trigger_kind_is_the_envelope_kind() {
        let kinds: Vec<&str> = SHIPPED_KINDS.iter().map(|(kind, _)| *kind).collect();
        assert_eq!(kinds, ["schema", envelope::ENVELOPE_KIND]);
    }

    #[test]
    fn the_shipped_registry_claims_exactly_the_shipped_kinds() {
        let registry = KindDocuments::shipped();
        for (kind, _) in SHIPPED_KINDS {
            assert_eq!(registry.claimed_kind(&serde_json::json!({ "kind": kind })), Some(*kind));
        }
        for unclaimed in [
            serde_json::json!({}),
            serde_json::json!({ "kind": null }),
            serde_json::json!({ "kind": 5 }),
            serde_json::json!({ "kind": "trigger-schema" }),
            serde_json::json!({ "kind": "Schema" }),
            serde_json::json!(["kind", "schema"]),
        ] {
            assert_eq!(registry.claimed_kind(&unclaimed), None, "{unclaimed}");
        }
    }

    #[test]
    fn the_schema_definition_names_the_tagged_envelope_keys() {
        let names = property_names(definition("schema"));
        for key in CANDIDATE_KEYS {
            assert_eq!(
                names.contains(*key),
                StandaloneSchemaEnvelope::Tagged.admits_top_level_key(key),
                "`{key}`: schema.yaml and the tagged envelope disagree"
            );
        }
        assert_eq!(names, set(&["kind", "$schema", "types", "description", "categories"]));
    }

    #[test]
    fn the_trigger_definition_names_the_trigger_envelope_keys() {
        let names = property_names(definition(envelope::ENVELOPE_KIND));
        for key in CANDIDATE_KEYS {
            assert_eq!(
                names.contains(*key),
                envelope::KNOWN_KEYS.contains(key),
                "`{key}`: schema-trigger.yaml and the trigger envelope disagree"
            );
        }
        assert_eq!(names, envelope::KNOWN_KEYS.iter().map(|key| (*key).to_owned()).collect());
    }

    #[test]
    fn each_definition_fixes_its_kind_literal() {
        assert_eq!(kind_literal(definition("schema")), set(&["schema"]));
        assert_eq!(
            kind_literal(definition(envelope::ENVELOPE_KIND)),
            set(&[envelope::ENVELOPE_KIND])
        );
    }

    #[test]
    fn each_definition_requires_what_its_envelope_requires() {
        assert_eq!(
            required_sets(definition("schema")),
            [set(&["kind", "$schema"]), set(&["kind", "types"])].into_iter().collect()
        );
        assert_eq!(
            required_sets(definition(envelope::ENVELOPE_KIND)),
            [set(&["kind", "match"])].into_iter().collect()
        );
    }
}
