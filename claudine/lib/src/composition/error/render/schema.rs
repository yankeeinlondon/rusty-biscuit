//! Terminal rendering for the schema / frontmatter-validation error family.
//!
//! Covers `$schema` load/parse/validation failures, missing required
//! properties, and interactive-schema shape rejections. The dispatcher in
//! [`super`] routes this family here.

use super::super::*;
use super::{escape_prose_path, render_file_link};

/// Render the [`StatusBlock`] for a schema/frontmatter-family
/// [`CompositionError`].
pub(super) fn status_block(err: &CompositionError) -> StatusBlock {
    match err {
        CompositionError::SchemaLoad {
            source_path,
            message,
        } => {
            let file_link = render_file_link(source_path);
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "schema load failed"))
                .body(format!(
                    "Could not load the `$schema` referenced by {file_link}.\n\n{message}"
                ))
                .hint(
                    "Verify the `$schema` path is correct, relative to the prompt's parent \
                     directory. Remote `http://` / `https://` references are not supported.",
                )
        }
        CompositionError::SchemaParse {
            source_path,
            property,
            message,
            // The span drives the appended frontmatter excerpt's highlight
            // line (see `frontmatter_block_spec` → `SchemaSpan`), not the
            // block body; the body names the property and the typed message
            // and OSC8-links the prompt file via `render_file_link`.
            span: _,
        } => {
            let file_link = render_file_link(source_path);
            // A property-scoped failure is a type-and-constraint syntax error
            // (Grammar/Convert); a property-less one is a wrong-shape `$schema`
            // value. Each gets the remediation that actually applies.
            let (scope, hint) = match property {
                Some(prop) => (
                    format!(" for property <cyan>`{}`</cyan>", Prose::escape_text(prop)),
                    "Check the SimplifiedSchema type-and-constraint syntax. Constraints are \
                     separated by `;` and a constraint's arguments by `,` — e.g. \
                     `file(required; match(**/*.md))`.",
                ),
                None => (
                    String::new(),
                    "The `$schema` value must be a file reference, an inline SimplifiedSchema \
                     mapping, or a JSON Schema object.",
                ),
            };
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "invalid schema"))
                .body(format!(
                    "The `$schema` declared in {file_link} is not a valid schema{scope}.\n\n\
                     {message}"
                ))
                .hint(hint)
        }
        CompositionError::SchemaValidation {
            source_path,
            message,
            problems,
        } => {
            let file_link = render_file_link(source_path);
            let mut body = format!("Schema validation failed for {file_link}.\n\n{message}");
            if !problems.is_empty() {
                body.push_str("\n\n<b>Problems:</b>");
                for problem in problems {
                    body.push_str(&format!("\n- <cyan>`{problem}`</cyan>"));
                }
            }
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "schema validation"))
                .body(body)
        }
        CompositionError::MissingProperties {
            source_path,
            missing,
            frontmatter_description,
            pointer_paths,
        } => render_missing_properties_block(
            source_path,
            missing,
            frontmatter_description.as_deref(),
            pointer_paths,
        ),
        CompositionError::UnsupportedInteractiveSchema {
            source_path,
            property,
            shape,
        } => {
            let file_link = render_file_link(source_path);
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "unsupported interactive schema",
                ))
                .body(format!(
                    "Required property <cyan>`{property}`</cyan> in {file_link} has shape \
                     <i>{shape}</i>, which cannot be collected interactively."
                ))
                .hint(
                    "Pass the value with key=value or --set, or provide it in the prompt's \
                     frontmatter.",
                )
        }
        // The dispatcher only routes schema-family variants here.
        _ => unreachable!("non-schema CompositionError routed to schema renderer"),
    }
}

fn render_missing_properties_block(
    source_path: &std::path::Path,
    missing: &[MissingProperty],
    frontmatter_description: Option<&str>,
    pointer_paths: &[String],
) -> StatusBlock {
    let file_link = render_file_link(source_path);

    let mut body = format!("Required {plural} missing in {file_link}.",
        plural = if missing.len() == 1 { "property is" } else { "properties are" });

    if let Some(desc) = frontmatter_description.filter(|d| !d.trim().is_empty()) {
        body.push_str(&format!("\n\n<i><dim>{}</dim></i>", escape_prose_path(desc)));
    }

    if !missing.is_empty() {
        body.push_str("\n\n<b>Missing:</b>");
        for prop in missing {
            let type_label = prop
                .type_label
                .as_deref()
                .filter(|t| !t.is_empty())
                .unwrap_or("(unknown type)");
            let mut line = format!("\n- <cyan>`{}`</cyan>: {}", prop.name, type_label);
            if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
                line.push_str(&format!(" <i><dim>— {}</dim></i>", escape_prose_path(desc)));
            }
            body.push_str(&line);
        }
    } else if !pointer_paths.is_empty() {
        body.push_str("\n\n<b>Validation problems:</b>");
        for pointer in pointer_paths {
            body.push_str(&format!("\n- <cyan>`{pointer}`</cyan>"));
        }
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("CompositionError", "missing properties"))
        .body(body)
        .hint(
            "Pass key=value, use --set, or set prompt_for_missing to true in an interactive \
             terminal.",
        )
}
