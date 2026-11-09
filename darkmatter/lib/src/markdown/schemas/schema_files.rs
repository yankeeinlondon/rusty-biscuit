//! Resolving a schema file on its own (`md schema validate <schema file>`):
//! every `{{ … }}` slot the file holds, in its exported `$schema` and in its
//! named `types`, is resolved through the instance's slot session, exactly
//! as a document naming the file would resolve it. A slot cannot read the
//! document, so the file alone is the context it needs.
//!
//! The file's text is the one given (it is the file's open buffer for the
//! length of the call); what it imports is read through the ordinary
//! resolver. Nothing here prompts by itself: the session's grants and
//! handler decide every read, as in document mode.

use std::path::Path;
use std::sync::Arc;

use serde_json::{Map, Value};
use simplified_schema::resolve::ResolvedSchema;
use simplified_schema::resolve::buffers::{self, SourceBuffers};
use simplified_schema::simplified::{SchemaDeclaration, SchemaRoot, SchemaShape};
use simplified_schema::{NotSchemaReason, SchemaError, StandaloneSchemaDocument, StandaloneSchemaEnvelope};

use super::{DarkmatterSchemas, PendingTemplate};

impl DarkmatterSchemas {
    /// Resolves the schema file at `path`, whose text is `text`: its
    /// exported `$schema` and every named type that holds a slot, through
    /// the instance's slot session in its file-resolution context. Returns
    /// the resolved export, or for a `types`-only file the resolution of
    /// its slot-bearing types (every type when none holds a slot).
    ///
    /// ## Errors
    ///
    /// - [`SchemaError::NotASchemaDocument`] when `text` is not a schema
    ///   document;
    /// - [`SchemaError::SlotResolution`] when a slot was refused or failed,
    ///   or its value (a template type's returned definition included) is
    ///   not usable, with the slot's typed cause;
    /// - [`SchemaError::UnresolvedSlots`] when the instance has no slot
    ///   session (or a passive one), or a reference-check session met a read
    ///   nobody granted;
    /// - every resolver error of the file and its imports.
    pub fn resolve_schema_file(&self, path: &Path, text: &str) -> Result<ResolvedSchema, SchemaError> {
        let document = parse(path, text)?;
        with_text(path, text, || {
            let template = biscuit_file::to_portable_string(path);
            let roots = super::roots::SchemaRoots::for_document(&self.file_resolution_context)?;
            let export = if document.exports_schema {
                Some(match self.prepare_export(&document, path, text, roots.search_paths()) {
                    Err(unresolved @ SchemaError::UnresolvedSlots(_)) => self.resolve_template(unresolved, &template)?,
                    other => other?,
                })
            } else {
                None
            };
            let types = match types_value(&document, path) {
                Some(value) => Some(self.resolve_types(&value, path, &template, roots.search_paths())?),
                None => None,
            };
            match (export, types) {
                (Some(export), _) => Ok(export),
                (None, Some(types)) => Ok(types),
                (None, None) => Err(SchemaError::NotASchemaDocument {
                    path: path.to_path_buf(),
                    reason: NotSchemaReason::NoSchemaProperty,
                }),
            }
        })
    }

    /// The templates [`Self::resolve_schema_file`] resolves for the schema
    /// file at `path` (whose text is `text`), for the session's batched
    /// approval ([`Self::approve_template_reads`]). Passive: nothing a slot
    /// names is read. Empty for a concrete schema file or one that does not
    /// parse (resolving it reports why).
    pub fn schema_file_templates(&self, path: &Path, text: &str) -> Vec<PendingTemplate> {
        let Ok(document) = parse(path, text) else {
            return Vec::new();
        };
        let Ok(roots) = super::roots::SchemaRoots::for_document(&self.file_resolution_context) else {
            return Vec::new();
        };
        let mut templates = Vec::new();
        let mut push = |prepared: Result<ResolvedSchema, SchemaError>| {
            if let Err(SchemaError::UnresolvedSlots(unresolved)) = prepared
                && let Some(pending) = unresolved.pending
            {
                templates.push(PendingTemplate { slots: pending.slots().to_vec(), pending });
            }
        };
        with_text(path, text, || {
            if document.exports_schema {
                push(self.prepare_export(&document, path, text, roots.search_paths()));
            }
            if let Some(value) = types_value(&document, path)
                && let Some(base) = path.parent()
            {
                push(super::resolve::resolve_authored_schema_with_roots(
                    &value,
                    None,
                    base,
                    roots.search_paths(),
                    &self.file_resolution_context,
                ));
            }
        });
        templates
    }

    /// The collect-and-approve step for `templates`: the session's batched
    /// prompt ([`super::slots::SlotSession::approve_reads`]) about the reads
    /// their slots name before evaluation. Returns how many reads were
    /// asked about; `0` without a session or a handler.
    ///
    /// ## Errors
    ///
    /// The approval handler's own failure.
    pub fn approve_template_reads(
        &self,
        templates: &[PendingTemplate],
    ) -> Result<usize, crate::markdown::compose::ShellExpansionError> {
        let Some(session) = &self.slot_session else {
            return Ok(0);
        };
        session.approve_reads(templates, &self.file_resolution_context)
    }

    /// Prepares the file's exported `$schema`. A `kind: yaml-schema` export
    /// is a document kind's contract, which is never a `$schema` target, so
    /// it is prepared on its own; any other export is resolved as a document
    /// naming the file resolves it.
    fn prepare_export(
        &self,
        document: &StandaloneSchemaDocument,
        path: &Path,
        text: &str,
        roots: &[std::path::PathBuf],
    ) -> Result<ResolvedSchema, SchemaError> {
        if document.envelope == StandaloneSchemaEnvelope::YamlSchema {
            return simplified_schema::resolve::prepare_yaml_schema_text(
                path,
                text,
                roots,
                &self.file_resolution_context,
                super::embedded::SHIPPED_SOURCES,
            );
        }
        let (Some(base), Some(file_name)) = (path.parent(), path.file_name()) else {
            return Err(SchemaError::NotASchemaDocument { path: path.to_path_buf(), reason: NotSchemaReason::NoSchemaProperty });
        };
        let reference = Value::String(format!("./{}", file_name.to_string_lossy()));
        super::resolve::resolve_authored_schema_with_roots(&reference, None, base, roots, &self.file_resolution_context)
    }

    /// Resolves `value`, a schema whose properties import the file's
    /// slot-bearing named types, so each of their slots resolves in the
    /// file it was written in.
    fn resolve_types(
        &self,
        value: &Value,
        path: &Path,
        template: &str,
        roots: &[std::path::PathBuf],
    ) -> Result<ResolvedSchema, SchemaError> {
        let base = path.parent().unwrap_or(Path::new("."));
        match super::resolve::resolve_authored_schema_with_roots(value, None, base, roots, &self.file_resolution_context) {
            Err(unresolved @ SchemaError::UnresolvedSlots(_)) => {
                self.resolve_template(unresolved, &format!("{template}#types"))
            }
            other => other,
        }
    }
}

fn parse(path: &Path, text: &str) -> Result<StandaloneSchemaDocument, SchemaError> {
    simplified_schema::parse_standalone_schema_document(text, path)?
        .ok_or_else(|| SchemaError::NotASchemaDocument { path: path.to_path_buf(), reason: NotSchemaReason::NoSchemaProperty })
}

/// The named types of `document`: those beside its export, or the payload of
/// a `types`-only file.
fn named_types(document: &StandaloneSchemaDocument) -> Option<&SchemaShape> {
    if document.exports_schema {
        return document.local_types.as_ref();
    }
    match &document.declaration {
        SchemaDeclaration::Schema(SchemaRoot::Single(shape)) => Some(shape),
        _ => None,
    }
}

/// Runs `resolve` with `text` as the open buffer at `path`, so every read
/// of the file (the export's reference, the types' imports, a pending
/// template's re-preparation) sees the text given, not the disk. A
/// snapshot already in force that holds the same text is kept; otherwise
/// it is replaced for the call, since its other buffers cannot be read
/// back out to merge.
fn with_text<T>(path: &Path, text: &str, resolve: impl FnOnce() -> T) -> T {
    if buffers::text(path).is_some_and(|open| *open == *text) {
        return resolve();
    }
    let mut open = SourceBuffers::default();
    open.insert(path, Arc::from(text));
    buffers::with(&open, resolve)
}

/// An inline schema importing each named type of `document` that holds a
/// slot (`Name: "Name@./file.yaml"`), or each type of a `types`-only file
/// with none (so the file is still judged); `None` when the file exports a
/// `$schema` and no type holds a slot, or it has no types.
fn types_value(document: &StandaloneSchemaDocument, path: &Path) -> Option<Value> {
    let types = named_types(document)?;
    let file_name = path.file_name()?.to_string_lossy();
    let holds_slot = |def: &simplified_schema::simplified::PropertyDef| {
        !simplified_schema::slots::definition_slots(def).is_empty()
    };
    let chosen: Vec<&String> = if types.properties.values().any(holds_slot) {
        types.properties.iter().filter(|(_, def)| holds_slot(def)).map(|(name, _)| name).collect()
    } else if document.exports_schema {
        return None;
    } else {
        types.properties.keys().collect()
    };
    if chosen.is_empty() {
        return None;
    }
    let mut properties = Map::new();
    for name in chosen {
        properties.insert(name.clone(), Value::String(format!("{name}@./{file_name}")));
    }
    Some(Value::Object(properties))
}
