//! Terminal rendering for the lifecycle authoring/evaluation error family.
//!
//! Covers every `CompositionError::Lifecycle*` variant plus the retired
//! validation/handler-key diagnostic. The dispatcher in [`super`] routes this
//! family here; each arm reuses the shared path/link helpers rather than
//! copying styles.

use super::super::*;
use super::{escape_prose_path, render_file_link};

/// Render the [`StatusBlock`] for a lifecycle-family [`CompositionError`].
pub(super) fn status_block(err: &CompositionError) -> StatusBlock {
    match err {
        CompositionError::LifecycleInvalid {
            property,
            message,
            source_file,
            unknown_field,
            expected_fields,
        } => {
            let file_display = source_file.display().to_string();
            let escaped = escape_prose_path(&file_display);
            let file_link = format!(
                "<a href=\"{escaped}\">{}</a>",
                escape_prose_path(&source_file.file_name().map_or_else(
                    || file_display.to_string(),
                    |n| n.to_string_lossy().to_string()
                ))
            );

            // An unknown-field error carries a field catalog; render the
            // "Unknown property / Expected one of" form. Any other serde
            // error (e.g. `invalid type: map, expected a sequence` when
            // `stack:` is a map instead of a list) renders its raw message
            // verbatim — fabricating an "Unknown property" diagnostic with
            // the comm-field list would be actively misleading.
            let is_unknown_field = unknown_field.is_some() || !expected_fields.is_empty();

            let (body, hint) = if is_unknown_field {
                let dotted_property = match unknown_field {
                    Some(field) => format!("{property}.{field}"),
                    None => property.clone(),
                };
                let mut body = format!(
                    "Unknown property <cyan>`{dotted_property}`</cyan> in {file_link}"
                );
                if !expected_fields.is_empty() {
                    body.push_str("\n\n<b>Expected one of:</b>");
                    for field in expected_fields {
                        body.push_str(&format!("\n- <cyan>`{field}`</cyan>"));
                    }
                }
                (
                    body,
                    "Check the lifecycle frontmatter section in your prompt file."
                        .to_string(),
                )
            } else {
                let body = format!(
                    "Invalid value for lifecycle property <cyan>`{property}`</cyan> in \
                     {file_link}\n\n{}",
                    escape_prose_path(message)
                );
                // The only sequence-typed field on a lifecycle event block
                // is `stack`, so a "expected a sequence" mismatch almost
                // always means `stack:` was authored as a map.
                let hint = if message.contains("expected a sequence") {
                    "The `stack:` property must be a YAML list of stack items \
                     (each item begins with `-`)."
                        .to_string()
                } else {
                    "Check the lifecycle frontmatter section in your prompt file."
                        .to_string()
                };
                (body, hint)
            };

            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "invalid lifecycle property",
                ))
                .body(body)
                .hint(hint)
        }
        CompositionError::LifecycleInterpolationLeak {
            source_path,
            property,
            expression,
            reason,
        } => {
            let file_link = render_file_link(source_path);
            let mut body = format!(
                "Interpolation span leaked in lifecycle property \
                 <cyan>`{property}`</cyan> in {file_link}.\n\n\
                 <b>Expression:</b> <cyan>`{}`</cyan>",
                escape_prose_path(expression)
            );
            if !reason.is_empty() {
                body.push_str("\n\n<b>Reason:</b> ");
                body.push_str(&escape_prose_path(reason));
            }

            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "lifecycle interpolation leaked",
                ))
                .body(body)
                .hint(
                    "Fix the expression grammar or define the referenced variable in the \
                     lifecycle frontmatter section of your prompt file.",
                )
        }
        CompositionError::LifecycleUndefinedVariable {
            source_path,
            property,
            variable,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle property <cyan>`{property}`</cyan> in {file_link} references \
                 undefined variable <cyan>`{}`</cyan>, which composition resolves to an \
                 empty string.",
                escape_prose_path(variable)
            );

            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "undefined lifecycle variable",
                ))
                .body(body)
                .hint(
                    "Define the variable in frontmatter, prefix a runtime value with \
                     `ctx.`/`env.`, or supply a fallback (`{{ var || 'default' }}`).",
                )
        }
        CompositionError::LifecycleEvaluationError {
            source_path,
            event,
            surface,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let surface_label = lifecycle_evaluation_surface_label(surface);
            let body = format!(
                "A late-binding expression raised while the <cyan>`{event}`</cyan> \
                 lifecycle event was firing, in {surface_label} ({file_link}).\n\n\
                 <b>Reason:</b> {}",
                escape_prose_path(message)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "lifecycle evaluation error",
                ))
                .body(body)
                .hint(
                    "This is a crashed expression, not a clean `false` guard: the run \
                     halts and exits non-zero. Fix the expression (resolve the missing \
                     path or variable, correct the function call) or guard it with a \
                     fallback so it evaluates instead of raising.",
                )
        }
        CompositionError::RemovedValidationKey {
            source_path,
            key,
            replacement,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "The validation/handler key <cyan>`{}`</cyan> in {file_link} has been \
                 removed. Use the lifecycle stack model instead.\n\n\
                 <b>Replacement:</b> {}",
                escape_prose_path(key),
                escape_prose_path(replacement)
            );

            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "removed validation/handler key",
                ))
                .body(body)
                .hint(
                    "See the lifecycle documentation for `initialize`, `start`, `success`, \
                     `blocked`, `failure`, `finalize`, and `loop` stacks.",
                )
        }
        CompositionError::LifecycleStackInvalidShape {
            source_path,
            property,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle stack item in <cyan>`{property}`</cyan> in {file_link} has an \
                 invalid shape.\n\n{message}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "invalid lifecycle stack item",
                ))
                .body(body)
                .hint(
                    "A stack item is an object with an optional `when:` condition string \
                     and an `action:` (scalar or array). Remove any extra keys.",
                )
        }
        CompositionError::LifecycleActionInvalidShortForm {
            source_path,
            property,
            raw,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Short-form lifecycle action <cyan>`{}`</cyan> in <cyan>`{property}`</cyan> \
                 in {file_link} could not be parsed.\n\n{message}",
                escape_prose_path(raw)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "invalid lifecycle action",
                ))
                .body(body)
                .hint(
                    "Short-form actions use `verb(args)` grammar where args are Darkmatter \
                     expressions. Multi-word literals must be quoted: \
                     `say('using codex')`, not `say(using codex)`.",
                )
        }
        CompositionError::LifecycleActionInvalidLongForm {
            source_path,
            property,
            action,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Long-form lifecycle action <cyan>`{action}`</cyan> in <cyan>`{property}`</cyan> \
                 in {file_link} could not be parsed.\n\n{message}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "invalid lifecycle action",
                ))
                .body(body)
        }
        CompositionError::LifecycleUnknownVerb {
            source_path,
            property,
            verb,
            rewrite,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Unknown lifecycle action <cyan>`{verb}`</cyan> in <cyan>`{property}`</cyan> \
                 in {file_link}.\n\n{rewrite}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "unknown lifecycle action",
                ))
                .body(body)
                .hint(
                    "Lifecycle actions must use positional form (`verb: value`) or key/value \
                     form (`{ action: verb, ... }`).",
                )
        }
        CompositionError::LifecycleStackAmbiguous {
            source_path,
            property,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Ambiguous lifecycle stack item in <cyan>`{property}`</cyan> in {file_link}.\
                 \n\n{message}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "ambiguous lifecycle stack item",
                ))
                .body(body)
                .hint(
                    "Use positional form (`verb: value`) with exactly one key, or key/value \
                     form (`{ action: verb, ... }`).",
                )
        }
        CompositionError::LifecycleObjectDataThroughInterpolationPositional {
            source_path,
            property,
            verb,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{verb}`</cyan> in <cyan>`{property}`</cyan> in \
                 {file_link} received an object value where a scalar or array was expected."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "object value not allowed here",
                ))
                .body(body)
                .hint(
                    "Pass object data through a whole-value `{{ ... }}` interpolation, or use \
                     key/value form with a scalar parameter.",
                )
        }
        CompositionError::LifecycleObjectDataThroughInterpolationParameter {
            source_path,
            property,
            verb,
            param,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{verb}`</cyan> parameter <cyan>`{param}`</cyan> in \
                 <cyan>`{property}`</cyan> in {file_link} received an object value where a \
                 scalar was expected."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "object value not allowed here",
                ))
                .body(body)
                .hint(
                    "Pass object data through a whole-value `{{ ... }}` interpolation.",
                )
        }
        CompositionError::LifecycleProxyWithNotMapping {
            source_path,
            property,
            path,
            actual,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`proxy`</cyan> field <cyan>`{property}.{path}`</cyan> in \
                 {file_link} is a {actual}; <cyan>`with`</cyan> must be a mapping of target \
                 frontmatter properties."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "`with` must be a mapping"))
                .body(body)
                .hint(
                    "Author `with:` as a mapping of static keys, or omit it. `with: {}` is \
                     equivalent to omitting it.",
                )
        }
        CompositionError::LifecycleProxyWithWholeMapping {
            source_path,
            property,
            path,
            raw,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`proxy`</cyan> field <cyan>`{property}.{path}`</cyan> in \
                 {file_link} was supplied as the whole-mapping interpolation <cyan>`{}`</cyan>.\
                 \n\nSupplying the entire mapping from one expression is not supported in this \
                 version and is a named follow-up. Author the keys explicitly; each value may \
                 still be a whole-value interpolation carrying an object or array.",
                escape_prose_path(raw)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "whole-mapping `with` is not supported",
                ))
                .body(body)
                .hint(
                    "Write `with:` with explicit keys, e.g. `with: { spec: \"{{ spec }}\" }`.",
                )
        }
        CompositionError::LifecycleProxyWithDynamicKey {
            source_path,
            property,
            path,
            key,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`proxy`</cyan> field <cyan>`{property}.{path}`</cyan> in \
                 {file_link} has the dynamic key <cyan>`{}`</cyan>.\n\n\
                 <cyan>`with`</cyan> keys name top-level frontmatter properties of the target \
                 and are never interpolated. Only values resolve.",
                escape_prose_path(key)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "`with` keys must be static",
                ))
                .body(body)
                .hint("Use a literal key and move the expression into its value.")
        }
        CompositionError::LifecycleProxyOnlyParameter {
            source_path,
            property,
            verb,
            param,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{verb}`</cyan> in <cyan>`{property}`</cyan> in \
                 {file_link} received a <cyan>`{param}`</cyan> parameter, which only \
                 <cyan>`proxy`</cyan> accepts."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "parameter is proxy-only",
                ))
                .body(body)
                .hint(
                    "Remove the parameter, or use `{ action: proxy, target: ..., with: { ... } }`.",
                )
        }
        CompositionError::LifecycleWrongArity {
            source_path,
            property,
            verb,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{verb}`</cyan> in <cyan>`{property}`</cyan> in \
                 {file_link} has the wrong number of arguments.\n\n{message}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "wrong action arity",
                ))
                .body(body)
        }
        CompositionError::LifecycleShortFormRemoved {
            source_path,
            property,
            raw,
            rewrite,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Short-form lifecycle action <cyan>`{}`</cyan> in <cyan>`{property}`</cyan> \
                 in {file_link} has been removed.\n\n\
                 <b>Rewrite to positional form:</b> <cyan>`{}`</cyan>",
                escape_prose_path(raw),
                escape_prose_path(rewrite)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "short-form action removed",
                ))
                .body(body)
                .hint(
                    "Use positional form (`verb: value`) or key/value form \
                     (`{ action: verb, ... }`). `verb(args)` is no longer accepted.",
                )
        }
        CompositionError::LifecycleActionPlacement {
            source_path,
            property: _,
            action,
            event,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{action}`</cyan> is not valid in the \
                 <cyan>`{event}`</cyan> event in {file_link}."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "lifecycle action not valid here",
                ))
                .body(body)
                .hint(
                    "Check the \"Where valid\" matrix in the lifecycle spec: only certain \
                     control actions are allowed in each event.",
                )
        }
        CompositionError::LifecycleMultipleLifecycleActions {
            source_path, property, ..
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Stack item in <cyan>`{property}`</cyan> in {file_link} contains more than \
                 one lifecycle control action."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "multiple lifecycle actions",
                ))
                .body(body)
                .hint(
                    "Split the actions across separate stack items, or remove the extra \
                     lifecycle action. At most one lifecycle control action is allowed per \
                     `when/action` block.",
                )
        }
        CompositionError::LifecycleActionOrder {
            source_path, property, ..
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action in <cyan>`{property}`</cyan> in {file_link} must be the \
                 last action in its stack item."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "lifecycle action must be last",
                ))
                .body(body)
                .hint(
                    "A lifecycle control action terminates stack processing, so any actions \
                     after it would never run. Move it to the end of the `action:` array.",
                )
        }
        CompositionError::LifecycleInvalidArgs {
            source_path,
            property,
            action,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{action}`</cyan> in <cyan>`{property}`</cyan> in \
                 {file_link} has invalid arguments.\n\n{message}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "invalid lifecycle action arguments",
                ))
                .body(body)
        }
        CompositionError::LifecycleErrNotAvailable {
            source_path,
            property,
            event,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle property <cyan>`{property}`</cyan> in {file_link} references the \
                 <cyan>`err`</cyan> global in the <cyan>`{event}`</cyan> event, which never \
                 carries an error."
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "`err` not available in this event",
                ))
                .body(body)
                .hint(
                    "Use `err` only in `blocked`, `failure`, or `finalize` (where it is \
                     optional). To reference a frontmatter property named `err`, write \
                     `doc.err` explicitly.",
                )
        }
        // The dispatcher only routes lifecycle-family variants here.
        _ => unreachable!("non-lifecycle CompositionError routed to lifecycle renderer"),
    }
}

/// Human-readable label for the late-binding surface that raised a
/// [`CompositionError::LifecycleEvaluationError`].
///
/// `surface` is the raised
/// [`LifecycleErrorInfo::variant`](crate::composition::lifecycle_context::LifecycleErrorInfo::variant):
/// `when` for a guard, `interpolation` for a communication/action string, or an
/// action verb (`shell`, `set_frontmatter`, …) for a side-effect argument.
fn lifecycle_evaluation_surface_label(surface: &str) -> String {
    match surface {
        "when" => "the `when:` guard".to_string(),
        "interpolation" => "an interpolated string".to_string(),
        verb => format!("the `{verb}` action value"),
    }
}
