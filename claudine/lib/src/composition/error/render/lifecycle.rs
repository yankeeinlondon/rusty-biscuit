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
            let file_link = render_file_link(source_file);

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
        }
        | CompositionError::LifecycleWhenExpressionInvalid {
            source_path,
            property,
            message,
            ..
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
            source: _,
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
        CompositionError::LifecycleProxyWithEvaluationFailed {
            source_path,
            property,
            path,
            target,
            message,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`proxy`</cyan> to <cyan>`{}`</cyan> in {file_link} could \
                 not resolve <cyan>`{property}.{path}`</cyan>.\n\n{message}\n\n\
                 The whole <cyan>`with`</cyan> mapping resolves at the source before the handoff, \
                 so nothing was passed to the target and the source is still active.",
                escape_prose_path(target)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "`with` value could not be resolved",
                ))
                .body(body)
                .hint(
                    "`with:` values resolve against the source document — its live frontmatter, \
                     `ctx.*`/`env.*`, and the globals valid for this event.",
                )
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
        CompositionError::LifecycleTransitionUnownedAtStage {
            source_path,
            property,
            verb,
            stage,
            reason,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "Lifecycle action <cyan>`{verb}`</cyan> in <cyan>`{property}`</cyan> in \
                 {file_link} ran at <cyan>`{stage}`</cyan>, which cannot act on \
                 it.\n\n{reason}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "transition has no owner at this stage",
                ))
                .body(body)
                .hint(
                    "`retry`, `resume`, and `defer` act on a provider attempt, so they \
                     need one to have run. Use `proxy`, `skip`, or `error` for a \
                     decision made before the provider launches.",
                )
        }
        CompositionError::LifecycleProxyWithoutOwningCoordinator {
            source_path,
            property,
            target,
            command,
        } => {
            let file_link = render_file_link(source_path);
            let body = format!(
                "A lifecycle <cyan>`proxy`</cyan> in <cyan>`{property}`</cyan> in \
                 {file_link} hands off to <cyan>`{}`</cyan>, but <cyan>`{command}`</cyan> \
                 runs a provider memory file with a prompt supplied on the command \
                 line — it prepares no active document, so it owns no coordinator that \
                 can bring the target up.\n\nRunning the target from here would launch \
                 it with this invocation's own profile, argv, and MCP servers instead \
                 of the ones the target's own frontmatter selects, so the hand-off is \
                 refused rather than run against the wrong launch configuration.",
                escape_prose_path(target)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "proxy has no owning coordinator",
                ))
                .body(body)
                .hint(
                    "Run the target through a composition command, which does own a \
                     coordinator: `claudine compose <target>` (or `inline-compose` / a \
                     `sequence` step). To keep using the provider wrapper, remove the \
                     `proxy` from the memory file's lifecycle.",
                )
        }
        CompositionError::LifecycleProxyCycle {
            source_path,
            target,
            chain,
            limit,
        } => {
            let file_link = render_file_link(source_path);
            let rendered_chain = chain
                .iter()
                .map(|hop| format!("<cyan>`{}`</cyan>", escape_prose_path(hop)))
                .collect::<Vec<_>>()
                .join(" → ");
            let body = format!(
                "A lifecycle <cyan>`proxy`</cyan> in {file_link} hands off to \
                 <cyan>`{}`</cyan>, which would re-enter a document already on the \
                 active chain or exceed the hop limit of {limit}.\n\nActive chain: \
                 {rendered_chain}",
                escape_prose_path(target)
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "proxy chain forms a cycle",
                ))
                .body(body)
                .hint(
                    "A document already on the chain cannot be handed off to again — an \
                     overlay does not make it a distinct target. Give the chain a terminal \
                     document, or guard the `proxy` with a `when:` condition that stops it.",
                )
        }
        CompositionError::LifecycleResumeIncompatible {
            source_path,
            facets,
        } => {
            let file_link = render_file_link(source_path);
            let rendered_facets = facets
                .iter()
                .map(|facet| format!("<cyan>`{facet}`</cyan>"))
                .collect::<Vec<_>>()
                .join(", ");
            let body = format!(
                "A lifecycle <cyan>`resume`</cyan> in {file_link} kept a live provider \
                 session, but a canonical refresh changed launch propert{plural} the \
                 provider fixed when the session opened: {rendered_facets}.\n\nResuming \
                 would mix the live session with a different launch plan.",
                plural = if facets.len() == 1 { "y" } else { "ies" },
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "resume incompatible after refresh",
                ))
                .body(body)
                .hint(
                    "A session cannot be resumed under launch settings it was not opened \
                     with. Use `retry` to start a fresh session with the new plan, or keep \
                     the changed propert(ies) stable across the resume.",
                )
        }
        CompositionError::LifecycleProxyTargetBootstrapFailed {
            target_path,
            source_path,
            property,
            reason,
        } => {
            let source_link = render_file_link(source_path);
            let target_link = render_file_link(target_path);
            let body = format!(
                "The lifecycle <cyan>`proxy`</cyan> at <cyan>`{property}`</cyan> in \
                 {source_link} handed off to {target_link}, which could not be \
                 prepared.\n\n{reason}"
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "proxy target could not be prepared",
                ))
                .body(body)
                .hint(
                    "The hand-off was committed but the target never started — no part of \
                     it ran. Check that the target composes on its own before proxying to it.",
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
        CompositionError::InvalidFileReference { context, source } => {
            let FileReferenceContext {
                source_path,
                event,
                property,
                reference,
                hint,
            } = context.as_ref();
            let file_link = render_file_link(source_path);
            let surface = match event {
                Some(event) => format!(
                    "<cyan>`{property}`</cyan> in the <cyan>`{event}`</cyan> event of {file_link}"
                ),
                None => format!("<cyan>`{property}`</cyan> in {file_link}"),
            };
            // `escape_prose_path` escapes `"` for `<a href="…">` attributes;
            // it over-escapes body text, and this source's `Display` quotes
            // the reference. `Prose::escape_text` is the body-text escape.
            let mut body = format!(
                "Cannot resolve <cyan>`{}`</cyan>, referenced by {surface}.\n\n{}",
                escape_prose_path(reference),
                Prose::escape_text(&source.to_string())
            );
            // Enumerate the ordered plan only when the resolver tried more than
            // one candidate: the single-candidate `Display` above already names
            // its one path, so a "Tried:" list adds information solely for an
            // implicit reference that fell through repository- then
            // source-relative candidates (spec §D8).
            let candidates = source.resolution_candidates();
            if candidates.len() >= 2 {
                body.push_str("\n\nTried:");
                for (index, probed) in candidates.iter().enumerate() {
                    let path = biscuit_file::to_portable_string(probed.candidate().path());
                    body.push_str(&format!("\n  {}. {}", index + 1, Prose::escape_text(&path)));
                }
            }
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "Unresolvable file reference",
                ))
                .body(body)
                .hint(escape_prose_path(hint))
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
