//! The hover of a `file`-typed frontmatter value: where it resolves, or why
//! it resolves nowhere.
//!
//! Resolution probes the filesystem through the document's context, like
//! navigation; the wording is
//! [`reference_hover`](crate::providers::reference_hover)'s.

use biscuit_file::{FileReference, ResolutionFailure};
use lsp_types::Hover;
use simplified_schema::pending::holds_pending_syntax;

use super::markup_hover;
use crate::overlay::analysis::ScalarTyping;
use crate::overlay::references::decoded_string;
use crate::overlay::{FmEntryRole, FrontmatterAst};
use crate::providers::DocumentContext;
use crate::providers::reference_hover::{self as text, Eagerness};

/// Whether a `file` value is a fixed reference rather than one composition
/// computes. A Darkmatter `{{ expr }}` or `$(cmd)` value is not, while a
/// biscuit-file environment reference such as `{{DIR}}/x.md` parses as a
/// file reference and is.
pub(super) fn file_value_is_static(value: &str) -> bool {
    !(holds_pending_syntax(value) && (value.contains("$(") || FileReference::new(value).is_err()))
}

/// The hover of the scalar entry `index` when its definition admits `file`
/// and its value is a static YAML string. `None` otherwise, so the value
/// keeps the hover it had.
pub(super) fn file_value_hover(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    index: usize,
    typing: &ScalarTyping,
) -> Option<Hover> {
    if !typing.file {
        return None;
    }
    let entry = ast.entries().get(index)?;
    let value = decoded_string(entry, ctx.text).filter(|value| file_value_is_static(value))?;
    let markdown = match ctx.file_context() {
        Err(_) => text::no_resolution_context(&value),
        Ok(context) => match FileReference::new(&value) {
            Err(error) => text::invalid_file_reference(&value, &error.to_string()),
            Ok(reference) => match reference.resolve_in_context(context) {
                Ok(Some(path)) => text::found_file(&path, context.repository_root()),
                Ok(None) => text::missing_file(&property_of(ast, index), eagerness(typing)),
                Err(error) => match error.resolution_failure() {
                    ResolutionFailure::InvalidReference => text::invalid_file_reference(&value, &error.to_string()),
                    ResolutionFailure::NoMatch => text::missing_file(&property_of(ast, index), eagerness(typing)),
                    failure => text::unchecked_file(failure)?.to_string(),
                },
            },
        },
    };
    markup_hover(ctx, entry.value_span.clone(), markdown)
}

fn eagerness(typing: &ScalarTyping) -> Eagerness {
    match (typing.eager, typing.mixed_arms) {
        (false, _) => Eagerness::Lazy,
        (true, false) => Eagerness::Eager,
        (true, true) => Eagerness::EagerFileArm,
    }
}

/// The dotted path of the property entry `index` belongs to: the entry
/// itself, or for a sequence item the nearest enclosing mapping property.
fn property_of(ast: &FrontmatterAst, index: usize) -> String {
    let entries = ast.entries();
    let mut current = index;
    while let FmEntryRole::SequenceItem { .. } = entries[current].role {
        match entries[current].parent {
            Some(parent) => current = parent,
            None => break,
        }
    }
    entries[current].dotted.clone()
}
