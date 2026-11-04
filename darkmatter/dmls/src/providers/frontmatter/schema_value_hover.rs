//! The hover of a `$schema` value: the file the reference selects and what
//! it holds, or why it selects nothing.
//!
//! Reads only the overlay's [`SchemaReferenceFacts`](crate::overlay::SchemaReferenceFacts),
//! built with the schema on a cache miss, so a hover reads no file; the
//! wording is [`reference_hover`](crate::providers::reference_hover)'s.

use biscuit_file::FileReference;
use lsp_types::Hover;
use simplified_schema::SchemaError;
use simplified_schema::resolve::{SelectionFailure, SelectionOutcome, is_bare_name};

use super::{markup_hover, overlay_ast};
use crate::overlay::references::references;
use crate::overlay::{SchemaReferenceFact, SchemaTargetVerdict};
use crate::providers::DocumentContext;
use crate::providers::reference_hover::{self as text, MissingReference, ValidTarget};

/// The hover of the `$schema` string (the scalar, or a root-union item) under
/// the cursor. `None` elsewhere, including on the key and on inline schema
/// mappings and arms, which keep their schema-authoring hovers.
pub(super) fn schema_value_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let ast = overlay_ast(ctx)?;
    let Ok(context) = ctx.file_context() else {
        let (pointer, reference) = references(ast, ctx.text)
            .find(|(pointer, _)| ast.entry_by_pointer(pointer).is_some_and(|entry| entry.value_span.contains(&offset)))?;
        let span = ast.entry_by_pointer(&pointer)?.value_span.clone();
        return markup_hover(ctx, span, text::no_resolution_context(&reference));
    };
    let (entry, fact) = ctx.overlay?.schema_references.entries.iter().find_map(|fact| {
        let entry = ast.entry_by_pointer(&fact.pointer)?;
        let current = entry.value_span.contains(&offset) && entry.scalar.as_deref() == Some(fact.reference.as_str());
        current.then_some((entry, fact))
    })?;
    markup_hover(ctx, entry.value_span.clone(), markdown(fact, context.repository_root())?)
}

/// The hover text of one fact.
fn markdown(fact: &SchemaReferenceFact, repository_root: Option<&std::path::Path>) -> Option<String> {
    let selection = match &fact.selection {
        Ok(selection) => selection,
        Err(error) => return Some(text::unreadable_schema_roots(&fact.reference, &error.to_string())),
    };
    Some(match &selection.outcome {
        SelectionOutcome::Found(path) => {
            let target = fact.target.as_ref()?;
            match &target.verdict {
                SchemaTargetVerdict::Valid => text::valid_schema_target(
                    &ValidTarget {
                        path,
                        format: target.format,
                        description: target.description.as_deref(),
                        preview: &target.preview,
                        total_lines: target.total_lines,
                    },
                    repository_root,
                ),
                SchemaTargetVerdict::Rejected(error) => match &**error {
                    SchemaError::NotASchemaDocument { path: rejected, .. } if rejected == path => {
                        text::not_a_schema_target(path, repository_root)
                    }
                    SchemaError::UnsupportedJsonDialect { path: rejected, declared } if rejected == path => {
                        text::unsupported_dialect_target(path, repository_root, declared)
                    }
                    other => text::unusable_schema_target(path, repository_root, &other.to_string()),
                },
            }
        }
        SelectionOutcome::Missing { bare_name, creation } => text::missing_schema_reference(
            &MissingReference {
                reference: &selection.reference,
                sibling_suggestion: None,
                // A bare name skips the schema roots only when none exists.
                without_roots: !bare_name
                    && FileReference::new(&selection.reference).is_ok_and(|reference| is_bare_name(&reference)),
                attempted: &selection.attempted,
                creation: creation.as_deref(),
            },
            repository_root,
        ),
        SelectionOutcome::SiblingOnly { suggestion, creation, .. } => text::missing_schema_reference(
            &MissingReference {
                reference: &selection.reference,
                sibling_suggestion: Some(suggestion),
                without_roots: false,
                attempted: &selection.attempted,
                creation: Some(creation),
            },
            repository_root,
        ),
        SelectionOutcome::Failed(SelectionFailure::Remote) => text::remote_schema_reference(&selection.reference),
        SelectionOutcome::Failed(SelectionFailure::Resolution(error)) => {
            text::unresolvable_schema_reference(&selection.reference, &error.to_string())
        }
    })
}
