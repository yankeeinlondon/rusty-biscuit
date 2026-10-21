//! Terminal rendering for the schema / frontmatter-validation error family.
//!
//! Covers `$schema` load/parse/validation failures, missing required
//! properties, and interactive-schema shape rejections. The dispatcher in
//! [`super`] routes this family here.

use super::super::*;
use super::{escape_prose_path, render_file_link, rows};
use renderable::markdown::code_span;

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
                // The loader's message can carry its own `hint:` row.
                .body(vec![
                    Prose::new(format!(
                        "Could not load the `$schema` referenced by {file_link}."
                    )),
                    rows(message.as_str()),
                ])
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
                    format!(" for property <cyan>{}</cyan>", code_span(prop)),
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
                .body(vec![
                    Prose::new(format!(
                        "The `$schema` declared in {file_link} is not a valid schema{scope}."
                    )),
                    rows(message.as_str()),
                ])
                .hint(hint)
        }
        CompositionError::SchemaValidation {
            source_path,
            message,
            problems,
            failures,
        } => {
            let file_link = render_file_link(source_path);
            let mut body = vec![
                Prose::new(format!("Schema validation failed for {file_link}.")),
                rows(message.as_str()),
            ];
            if !problems.is_empty() {
                let mut listed = String::from("<b>Problems:</b>");
                for problem in problems {
                    listed.push_str(&format!("\n- <cyan>`{problem}`</cyan>"));
                }
                body.push(rows(listed));
            }
            for failure in failures {
                body.push(Prose::new(format!(
                    "<dim>failure:</dim> {}",
                    darkmatter::markdown::errors::resolution_failure_name(*failure)
                )));
            }
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "schema validation"))
                .body(body)
        }
        CompositionError::CompletionBodyUnchanged {
            source_path,
            reason,
        } => {
            let file_link = render_file_link(source_path);
            let (what, hint) = match reason {
                crate::composition::closure::BodyRejection::Empty => (
                    "left the body empty",
                    "The agent is told the document's absolute path in its prompt. Check that \
                     it can write there, and that the prompt asks for body content.",
                ),
                crate::composition::closure::BodyRejection::Unchanged => (
                    "did not change the body",
                    "The agent's summary is shown above. Check whether it read the wrong file, \
                     refused the task, or reported the work without performing it.",
                ),
            };
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "document not updated"))
                .body(format!(
                    "The agent {what} of {file_link}, so nothing was stamped or written."
                ))
                .hint(hint)
        }
        CompositionError::InlineAgentFrontmatterRejected { path, rejection } => {
            let file_link = render_file_link(path);
            let attribution = if rejection.agent_edit {
                "The agent wrote this line during the run."
            } else {
                "The line could not be matched to the pre-run document."
            };
            let location = match (rejection.line, &rejection.property) {
                (Some(line), Some(property)) => format!("line {line}, <cyan>`{property}`</cyan>"),
                (Some(line), None) => format!("line {line}"),
                (None, Some(property)) => format!("<cyan>`{property}`</cyan>"),
                (None, None) => "the frontmatter".to_string(),
            };
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "agent frontmatter rejected"))
                .body(format!(
                    "The agent's frontmatter in {file_link} cannot be saved ({location}): {}. \
                     {attribution} Nothing was stamped or written.",
                    rejection.reason
                ))
                .hint(
                    "Quote a text value that contains `: ` or ` #`, keep each key once, and \
                     write plain scalars or indented block mappings and sequences.",
                )
        }
        CompositionError::InlineBaselineUnstampable { path, source } => {
            let file_link = render_file_link(path);
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "inline baseline not editable"))
                .body(vec![
                    Prose::new(format!(
                        "Inline composition of {file_link} did not start: the inline closure \
                         could not edit its frontmatter, so it could not record the run's \
                         result. The prompt was not composed and the file was not changed."
                    )),
                    rows(escape_prose_path(&source.to_string())),
                ])
                .hint("Fix the frontmatter as described above, then run inline-compose again.")
        }
        CompositionError::CompletionSchemaFailed {
            source_path,
            problems,
            status,
        } => {
            let file_link = render_file_link(source_path);
            // A document with no authored `$schema` was judged by trigger
            // schemas alone, whose status is the raw report.
            let schema = if status.raw_json_schema { "schema" } else { "`$schema`" };
            let mut body = vec![Prose::new(format!(
                "{file_link} does not satisfy its {schema} now that the run has completed."
            ))];
            // The status table lists the authored `$schema`'s properties; a
            // problem on a property only a trigger schema (Claudine's
            // payload) declares appears in no table row, so it is listed here.
            let tabled: std::collections::BTreeSet<&str> = if status.raw_json_schema {
                std::collections::BTreeSet::new()
            } else {
                body.push(crate::composition::schema::schema_status_report_prose(status));
                status
                    .required
                    .iter()
                    .chain(&status.optional)
                    .map(|property| property.name.as_str())
                    .collect()
            };
            let untabled: Vec<_> = problems
                .iter()
                .filter(|problem| {
                    let root = problem.property.split(['.', '[']).next().unwrap_or(&problem.property);
                    !tabled.contains(root)
                })
                .collect();
            if !untabled.is_empty() {
                let mut listed = String::from("<b>Problems:</b>");
                for problem in untabled {
                    listed.push_str(&format!(
                        "\n- <cyan>`{}`</cyan>: {}",
                        crate::composition::schema::escape_schema_prose(&problem.property),
                        crate::composition::schema::escape_schema_prose(&problem.message),
                    ));
                }
                body.push(Prose::new(listed).with_line_breaks(biscuit_terminal::components::prose::LineBreaks::Hard));
            }
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "completion schema"))
                .body(body)
                .hint(
                    "A property declared `required` must be present, and every present \
                     property valid, when the run finishes. Either the producing actor did \
                     not set it correctly, or the schema declares an obligation nothing \
                     fulfills.",
                )
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

    let mut body = vec![Prose::new(format!("Required {plural} missing in {file_link}.",
        plural = if missing.len() == 1 { "property is" } else { "properties are" }))];

    if let Some(desc) = frontmatter_description.filter(|d| !d.trim().is_empty()) {
        body.push(Prose::new(format!("<i><dim>{}</dim></i>", escape_prose_path(desc))));
    }

    if !missing.is_empty() {
        let mut listed = String::from("<b>Missing:</b>");
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
            listed.push_str(&line);
        }
        body.push(rows(listed));
    } else if !pointer_paths.is_empty() {
        let mut listed = String::from("<b>Validation problems:</b>");
        for pointer in pointer_paths {
            listed.push_str(&format!("\n- <cyan>`{pointer}`</cyan>"));
        }
        body.push(rows(listed));
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("CompositionError", "missing properties"))
        .body(body)
        .hint(
            "Pass key=value, use --set, or set prompt_for_missing to true in an interactive \
             terminal.",
        )
}
