//! Schema-definition diagnostics for named-type resolution failures.
//!
//! An unknown name, contradictory constraints on a reused type, an operation a
//! union-typed reference cannot take, a type cycle that never descends into a
//! child value, and an over-deep reference chain are defects of the
//! schema, not of a document value. Each is ranged on
//! the consuming reference's name when this document wrote it, and lists the
//! declarations that took part as `relatedInformation`, wherever they live.
//!
//! The resolver addresses every location structurally
//! ([`SchemaReferenceSite`], [`SchemaConstraintOrigin`]); this module turns
//! those addresses into ranges with the same source map the library projects,
//! never by searching the text.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::schemas::{
    SchemaConstraintOrigin, SchemaError, SchemaReferenceSite, SchemaSourceId,
    SchemaSourcePath, SchemaSourcePathSegment, SchemaSpanKind, StandaloneSchemaDocument,
    parse_standalone_schema_document,
};
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Range as LspRange, Uri};

use crate::diagnostics::codes::{code, source};
use crate::overlay::FrontmatterAst;
use crate::overlay::named_types::{AuthoredDeclaration, NamedReference};
use crate::providers::DocumentContext;
use crate::source_map::SourceMap;
use crate::workspace::file_path_to_uri;

/// Whether `error` is a named-type resolution failure this module projects.
pub(crate) fn is_named_type_error(error: &SchemaError) -> bool {
    matches!(
        error,
        SchemaError::UnknownNamedType { .. }
            | SchemaError::ConstraintConflict { .. }
            | SchemaError::UnionReference { .. }
            | SchemaError::NonDescendingCycle { .. }
            | SchemaError::ImportCycle { .. }
    )
}

/// The document whose schema failed, as the resolver identifies it.
pub(crate) struct NamedTypeScope<'a> {
    ctx: &'a DocumentContext<'a>,
    /// The resolver's identity for this document's own declaration.
    this: SchemaSourceId,
    /// This document's declaration with spans in document offsets, when it
    /// can be projected.
    declaration: Option<AuthoredDeclaration>,
    /// The frontmatter tree, for a property-entry fallback when projection
    /// fails.
    ast: Option<&'a FrontmatterAst>,
    /// The range used when no reference in this document can be named.
    fallback: Option<Range<usize>>,
    /// Imported files' text and parse, read on first use.
    files: std::cell::RefCell<HashMap<PathBuf, Option<Arc<ExternalFile>>>>,
}

struct ExternalFile {
    text: String,
    document: Option<StandaloneSchemaDocument>,
    /// Why the file no longer parses as a schema document; its locations
    /// cannot be projected, so the reason is reported instead.
    error: Option<SchemaError>,
}

impl<'a> NamedTypeScope<'a> {
    /// A Markdown document whose inline `$schema` the resolver knows as
    /// [`SchemaSourceId::InlineDocument`].
    pub(crate) fn frontmatter(
        ctx: &'a DocumentContext<'a>,
        ast: Option<&'a FrontmatterAst>,
        fallback: Option<Range<usize>>,
    ) -> Self {
        let fresh = ctx.overlay.is_some_and(|overlay| !overlay.stale);
        let declaration = ast
            .filter(|_| fresh)
            .and_then(|ast| AuthoredDeclaration::frontmatter(ctx.text, ast));
        Self {
            ctx,
            this: SchemaSourceId::InlineDocument,
            declaration,
            ast,
            fallback,
            files: Default::default(),
        }
    }

    /// A standalone schema document, which the resolver knows by its
    /// canonical path.
    pub(crate) fn standalone(ctx: &'a DocumentContext<'a>, model: &StandaloneSchemaDocument) -> Self {
        let path = biscuit_file::canonicalize_simplified(ctx.path).unwrap_or_else(|_| ctx.path.to_path_buf());
        Self {
            ctx,
            this: SchemaSourceId::File(path),
            declaration: Some(AuthoredDeclaration::standalone(model)),
            ast: None,
            fallback: model
                .source_map
                .spans(&SchemaSourcePath::root(), SchemaSpanKind::Declaration)
                .first()
                .cloned(),
            files: Default::default(),
        }
    }

    /// The diagnostic for one named-type failure. `referenced` names the
    /// referenced schema file the failure was reported through, if any.
    pub(crate) fn diagnostic(&self, error: &SchemaError, referenced: Option<&Path>) -> Diagnostic {
        let mut related = Vec::new();
        if let Some(path) = referenced
            && let Some(uri) = file_path_to_uri(path)
        {
            related.push(related_at(uri, LspRange::default(), "schema definition is authored in this file"));
        }
        let span = match error {
            SchemaError::UnknownNamedType { name, namespace, site } => {
                if site.source != self.this {
                    related.extend(self.site_location(site).map(|location| DiagnosticRelatedInformation {
                        location,
                        message: format!("`{name}` is referenced here"),
                    }));
                }
                if let SchemaSourceId::File(path) = namespace
                    && *namespace != site.source
                    && let Some(uri) = file_path_to_uri(path)
                {
                    related.push(related_at(uri, LspRange::default(), &format!("this schema does not define `{name}`")));
                }
                self.site_span(site)
            }
            SchemaError::ConstraintConflict { site, constraints, .. } => {
                related.extend(constraints.iter().filter_map(|origin| {
                    Some(DiagnosticRelatedInformation {
                        location: self.origin_location(origin)?,
                        message: format!("`{}` is declared here", origin.keyword),
                    })
                }));
                self.site_span(site)
            }
            SchemaError::UnionReference { name, site, definition, .. } => {
                if site.source != self.this {
                    related.extend(self.site_location(site).map(|location| DiagnosticRelatedInformation {
                        location,
                        message: format!("`{name}` is referenced here"),
                    }));
                }
                related.extend(self.definition_location(definition).map(|location| {
                    DiagnosticRelatedInformation {
                        location,
                        message: format!("`{name}` is defined here"),
                    }
                }));
                self.site_span(site)
            }
            // Ranged on the first reference of the cycle this document wrote;
            // every other reference of the cycle is related information.
            SchemaError::NonDescendingCycle { cycles } => {
                let references: Vec<_> = cycles.iter().flatten().collect();
                let primary = references.iter().position(|reference| reference.site.source == self.this);
                for (index, reference) in references.iter().enumerate() {
                    if Some(index) == primary {
                        continue;
                    }
                    related.extend(self.site_location(&reference.site).map(|location| {
                        DiagnosticRelatedInformation {
                            location,
                            message: format!("`{}` is referenced here", reference.target),
                        }
                    }));
                }
                match primary {
                    Some(index) => self.site_span(&references[index].site),
                    None => references.first().and_then(|reference| self.site_span(&reference.site)),
                }
                .or_else(|| self.fallback.clone())
            }
            other => self.target_span(other).or_else(|| self.fallback.clone()),
        };
        for (path, file) in self.files.borrow().iter() {
            if let Some(file) = file
                && let Some(error) = &file.error
                && let Some(uri) = file_path_to_uri(path)
            {
                related.push(related_at(
                    uri,
                    LspRange::default(),
                    &format!("this schema file could not be read again: {error}"),
                ));
            }
        }
        let code_value = if is_named_type_error(error) {
            code::SCHEMA_INVALID_TYPE_DEFINITION
        } else {
            code::SCHEMA_PREPARE
        };
        let range = span
            .and_then(|span| self.ctx.source_map.byte_range_to_lsp(span))
            .unwrap_or_default();
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(lsp_types::NumberOrString::String(code_value.to_string())),
            source: Some(source::SCHEMA.to_string()),
            message: error.to_string(),
            related_information: (!related.is_empty()).then_some(related),
            ..Default::default()
        }
    }

    /// The primary range: the reference name when this document wrote it,
    /// else this document's reference into the file that did, else the
    /// fallback.
    fn site_span(&self, site: &SchemaReferenceSite) -> Option<Range<usize>> {
        if site.source == self.this {
            return self
                .declaration
                .as_ref()
                .and_then(|declaration| {
                    declaration
                        .source_map
                        .spans(&site.path, SchemaSpanKind::ImportName)
                        .first()
                        .cloned()
                })
                .or_else(|| self.property_entry_span(site))
                .or_else(|| self.fallback.clone());
        }
        self.reference_into(&site.source).or_else(|| self.fallback.clone())
    }

    /// The authored target of a reference whose file could not be selected
    /// or read, matched by its spelling.
    fn target_span(&self, error: &SchemaError) -> Option<Range<usize>> {
        let reference = match error {
            SchemaError::NoMatch { reference, .. }
            | SchemaError::Unresolved { reference, .. }
            | SchemaError::BareNameSiblingExists { reference, .. }
            | SchemaError::RemoteUnsupported { reference } => reference.trim(),
            _ => return None,
        };
        let declaration = self.declaration.as_ref()?;
        declaration
            .source_map
            .entries()
            .filter(|(_, kind, _)| *kind == SchemaSpanKind::ImportReference)
            .flat_map(|(_, _, spans)| spans.iter())
            .filter(|span| self.ctx.text.get((*span).clone()).map(str::trim) == Some(reference))
            .min_by_key(|span| span.start)
            .cloned()
    }

    /// The value span of the frontmatter entry holding `site`, for a
    /// declaration the source projector cannot read (a block scalar).
    fn property_entry_span(&self, site: &SchemaReferenceSite) -> Option<Range<usize>> {
        let mut keys = vec!["$schema"];
        for segment in site.path.segments() {
            match segment {
                SchemaSourcePathSegment::Property(name) => keys.push(name),
                SchemaSourcePathSegment::UnionArm(_) => break,
            }
        }
        (keys.len() > 1)
            .then(|| self.ast?.entry_by_key_path(&keys).map(|entry| entry.value_span.clone()))
            .flatten()
    }

    /// The first explicit reference in this document whose target is the
    /// file `source`.
    fn reference_into(&self, source: &SchemaSourceId) -> Option<Range<usize>> {
        let SchemaSourceId::File(path) = source else {
            return None;
        };
        let catalog = &self.ctx.overlay?.named_types;
        let declaration = self.declaration.as_ref()?;
        let mut spans: Vec<Range<usize>> = declaration
            .source_map
            .entries()
            .filter(|(_, kind, _)| *kind == SchemaSpanKind::ImportName)
            .flat_map(|(_, _, spans)| spans.iter().cloned())
            .collect();
        spans.sort_by_key(|span| span.start);
        spans.into_iter().find(|span| {
            declaration
                .reference_at(self.ctx.text, span.start)
                .and_then(|reference: NamedReference| reference.target)
                .and_then(|target| catalog.get(&target))
                .is_some_and(|external| external.path == *path)
        })
    }

    /// The reference name of `site` in the file that wrote it.
    fn site_location(&self, site: &SchemaReferenceSite) -> Option<Location> {
        let SchemaSourceId::File(path) = &site.source else {
            return None;
        };
        let file = self.file(path)?;
        let span = file
            .document
            .as_ref()?
            .source_map
            .spans(&site.path, SchemaSpanKind::ImportName)
            .first()?
            .clone();
        self.file_location(path, &file.text, span)
    }

    /// The definition a named-type error names, in whichever source holds it.
    fn definition_location(&self, definition: &SchemaReferenceSite) -> Option<Location> {
        if definition.source == self.this {
            let span = self
                .declaration
                .as_ref()?
                .source_map
                .spans(&definition.path, SchemaSpanKind::Definition)
                .first()?
                .clone();
            let range = self.ctx.source_map.byte_range_to_lsp(span)?;
            return Some(Location::new(self.ctx.uri.clone(), range));
        }
        let SchemaSourceId::File(path) = &definition.source else {
            return None;
        };
        let file = self.file(path)?;
        let span = file
            .document
            .as_ref()?
            .source_map
            .spans(&definition.path, SchemaSpanKind::Definition)
            .first()?
            .clone();
        self.file_location(path, &file.text, span)
    }

    /// Where one conflicting constraint was authored.
    fn origin_location(&self, origin: &SchemaConstraintOrigin) -> Option<Location> {
        if origin.source == self.this {
            // This document's buffer, not the resolver's span: a standalone
            // document's origins were located against the file on disk.
            let declaration = self.declaration.as_ref()?;
            let span = declaration.source_map.constraint_span(
                self.ctx.text,
                &origin.path,
                &origin.keyword,
                origin.occurrence,
            )?;
            let range = self.ctx.source_map.byte_range_to_lsp(span)?;
            return Some(Location::new(self.ctx.uri.clone(), range));
        }
        let SchemaSourceId::File(path) = &origin.source else {
            return None;
        };
        let file = self.file(path)?;
        let span = origin.span.clone().or_else(|| {
            file.document.as_ref()?.source_map.constraint_span(
                &file.text,
                &origin.path,
                &origin.keyword,
                origin.occurrence,
            )
        })?;
        self.file_location(path, &file.text, span)
    }

    fn file_location(&self, path: &Path, text: &str, span: Range<usize>) -> Option<Location> {
        let uri: Uri = file_path_to_uri(path)?;
        let map = SourceMap::new(uri.clone(), 0, self.ctx.source_map.encoding(), Arc::from(text));
        Some(Location::new(uri, map.byte_range_to_lsp(span)?))
    }

    /// An imported file's text, read once per diagnostic pass. Only the
    /// failure path reads, and only files the resolver already read.
    fn file(&self, path: &Path) -> Option<Arc<ExternalFile>> {
        self.files
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let text = crate::work_counters::read_to_string(path).ok()?;
                let (document, error) = match parse_standalone_schema_document(&text, path) {
                    Ok(document) => (document, None),
                    Err(error) => (None, Some(error)),
                };
                Some(Arc::new(ExternalFile { text, document, error }))
            })
            .clone()
    }
}

fn related_at(uri: Uri, range: LspRange, message: &str) -> DiagnosticRelatedInformation {
    DiagnosticRelatedInformation {
        location: Location::new(uri, range),
        message: message.to_string(),
    }
}

/// Every diagnostic for a standalone schema document whose own references do
/// not resolve, unwrapping aggregated and referenced-schema failures.
pub(crate) fn standalone_diagnostics(
    scope: &NamedTypeScope<'_>,
    error: &SchemaError,
    referenced: Option<&Path>,
    out: &mut Vec<Diagnostic>,
) {
    match error {
        SchemaError::Aggregate { errors } => {
            for error in errors {
                standalone_diagnostics(scope, error, referenced, out);
            }
        }
        SchemaError::ReferencedSchema { path, source } => {
            standalone_diagnostics(scope, source, Some(path), out);
        }
        // Anything else is an imported file that could not be selected or
        // read, ranged on its authored target.
        error => out.push(scope.diagnostic(error, referenced)),
    }
}
