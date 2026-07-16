use super::*;

impl BlockError for CompositionError {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        match self {
            CompositionError::WithFrontmatter { inner, .. } => inner.status_block(term),
            CompositionError::LifecycleEvaluationAlreadyEmitted { inner } => {
                inner.status_block(term)
            }
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
            CompositionError::LoopIterationFailed {
                iteration,
                exit_code,
                reason,
                exit_reason,
                ..
            } => {
                // Surface the actionable cause (`step_timeout`,
                // `wall-clock timeout`, signal, …) in the header instead of
                // the generic `composition failed` line. The cause comes
                // from the iteration's session_end JSONL row's
                // `extra.exit_reason` — not from `LoopInvalid` (which is
                // reserved for frontmatter parse errors).
                let title = exit_reason
                    .clone()
                    .unwrap_or_else(|| "iteration failed".to_string());
                let body =
                    format!("Iteration {iteration} exited with code {exit_code}.\n\n{reason}");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", &title))
                    .body(body)
            }
            CompositionError::LoopRateLimited { .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "rate limited"))
                .body(self.to_string())
                .hint(
                    "Re-run after the listed reset time, or use \
                         `--on-rate-limit pause` to wait automatically.",
                ),
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
            CompositionError::SequenceMissingProperties { failures, .. } => {
                render_sequence_missing_properties_block(failures)
            }
            CompositionError::SequenceInteractiveRejected(source_path) => {
                let file_link = render_file_link(source_path);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "interactive rejected for sequence",
                    ))
                    .body(format!(
                        "The document {file_link} sets <cyan>`interactive: true`</cyan> in its \
                         frontmatter, but a <cyan>`sequence`</cyan> is serial automation and does \
                         not support interactive sessions.\n\n\
                         Use <cyan>`claudine compose`</cyan> or <cyan>`claudine inline-compose`</cyan> \
                         for dialog-shaped prompts. To run an individual sequence step \
                         interactively, use the <cyan>`--interactive`</cyan> CLI flag — this remains \
                         the only explicit override."
                    ))
            }
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
            CompositionError::InlineComposeSequenceMismatch { source_path } => {
                render_inline_sequence_mismatch_block(source_path)
            }
            CompositionError::AgentResolutionFailed {
                source_path,
                state,
                installed,
            } => {
                let file_link = render_file_link(source_path);
                let body = render_agent_resolution_failed_body(state, installed, &file_link);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "agent resolution failed"))
                    .body(body)
                    .hint(
                        "Specify an installed provider with --claude, --codex, etc., run in an \
                         interactive terminal, or correct the `agent` frontmatter property."
                    )
            }
            CompositionError::ComposedBodyEmpty {
                source_path,
                mode,
                provided_overrides,
            } => {
                let file_link = render_file_link(source_path);
                let mode_label = match mode {
                    super::super::types::CompositionMode::ChainedDocument => "chained (compose)",
                    super::super::types::CompositionMode::InlineFrontmatterPrompt => {
                        "inline (inline-compose)"
                    }
                };
                let mut body = format!(
                    "Composition produced an <b>empty prompt body</b> for {file_link}.\n\n\
                     Mode: <i>{mode_label}</i>"
                );
                if provided_overrides.is_empty() {
                    body.push_str("\n\nNo `key=value` overrides were provided.");
                } else {
                    body.push_str("\n\n<b>Provided overrides:</b>");
                    for key in provided_overrides {
                        body.push_str(&format!("\n- <cyan>`{key}`</cyan>"));
                    }
                }
                body.push_str(
                    "\n\nThe document composed without error, but every block in the body \
                     was stripped by its `when` condition (or the body was empty to begin with). \
                     The provider CLI would otherwise reject this as \"Input must be provided …\" \
                     without naming the real cause.",
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "composed prompt is empty",
                    ))
                    .body(body)
                    .hint(
                        "Check that the variables you passed match the `::block when=…` \
                         conditions in the prompt, or verify there is body content outside any \
                         conditional block.",
                    )
            }
            CompositionError::AutocompleteNoMatches { query } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "no autocomplete matches"))
                .body(format!(
                    "No files matched autocomplete query <cyan>`{}`</cyan>.",
                    escape_prose_path(query)
                ))
                .hint("Check the query token or run without a query to see all candidates."),
            CompositionError::AutocompleteOverCap { query, cap } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "too many matches"))
                .body(format!(
                    "More than <cyan>{cap}</cyan> files matched autocomplete query \
                     <cyan>`{}`</cyan>.",
                    escape_prose_path(query)
                ))
                .hint("Type more characters to narrow the query."),
            CompositionError::AutocompleteNotInteractive => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "autocomplete not available",
                ))
                .body("Autocomplete requires an interactive terminal.".to_string())
                .hint("Run in a terminal, or supply an explicit file path or reference."),
            CompositionError::AutocompleteCancelled { query } => StatusBlock::new(StatusState::Warning)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "autocomplete cancelled",
                ))
                .body(format!(
                    "Autocomplete for query <cyan>`{}`</cyan> was cancelled.",
                    escape_prose_path(query)
                ))
                .hint("Supply an explicit file path or reference, or run the command again."),
            CompositionError::ShellExpansionFailed { error, .. } => {
                // Delegate to the structured shell-expansion block so the
                // linked source path, source excerpt, composed frontmatter
                // block, and captured stderr/stdout all survive the claudine
                // boundary instead of being flattened by the catch-all arm.
                error.status_block(term)
            }
            _ => {
                let msg = self.to_string();
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "composition failed"))
                    .body(msg)
            }
        }
    }

    /// Strips escape bytes from the rendered status block when the terminal has
    /// no color depth.
    ///
    /// `StatusBlock`'s bespoke path (entered whenever the error header carries
    /// `<b>` markup, i.e. every variant here) emits SGR styling and OSC 8 links
    /// even at [`ColorDepth::None`]. On a piped / `NO_COLOR` / JSON terminal
    /// those bytes must be removed so pipeable output stays plain text per the
    /// error-formatting contract. Frontmatter YAML blocks are appended
    /// separately by the CLI error walker (see `output::error_walker`).
    ///
    /// [`ColorDepth::None`]: biscuit_terminal::discovery::detection::ColorDepth::None
    fn report_block_error(&self, term: &Terminal) -> String {
        let out = self.status_block(term).render(term);
        if matches!(
            term.color_depth,
            biscuit_terminal::discovery::detection::ColorDepth::None
        ) {
            biscuit_terminal::utils::escape_codes::strip_escape_codes(&out)
        } else {
            out
        }
    }
}

/// Convert a JSON pointer (`/success/message`) or a bare property name into the
/// dotted form (`success.message`) that
/// [`locate_property_line`](super::frontmatter_excerpt::locate_property_line)
/// expects. A leading `/` and any pointer escaping (`~1` → `/`, `~0` → `~`) are
/// normalized; a value without a leading `/` is treated as already-dotted.
pub(super) fn pointer_to_dotted(pointer: &str) -> String {
    let trimmed = pointer.trim_start_matches('/');
    if pointer.starts_with('/') {
        trimmed
            .split('/')
            .map(|seg| seg.replace("~1", "/").replace("~0", "~"))
            .collect::<Vec<_>>()
            .join(".")
    } else {
        trimmed.to_string()
    }
}

/// Human-readable label for the late-binding surface that raised a
/// [`CompositionError::LifecycleEvaluationError`].
///
/// `surface` is the raised
/// [`LifecycleErrorInfo::variant`](super::lifecycle_context::LifecycleErrorInfo::variant):
/// `when` for a guard, `interpolation` for a communication/action string, or an
/// action verb (`shell`, `set_frontmatter`, …) for a side-effect argument.
fn lifecycle_evaluation_surface_label(surface: &str) -> String {
    match surface {
        "when" => "the `when:` guard".to_string(),
        "interpolation" => "an interpolated string".to_string(),
        verb => format!("the `{verb}` action value"),
    }
}

/// Render an absolute OSC8 hyperlink to `path` showing its relative form
/// where possible (falling back to the full display).
///
/// The Prose layer downgrades `<a href>` to plain text when the terminal
/// does not support OSC8.
fn render_file_link(path: &std::path::Path) -> String {
    let abs = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let abs_display = abs.display().to_string();
    let label = path.display().to_string();
    format!(
        "<a href=\"{}\">{}</a>",
        escape_prose_path(&abs_display),
        escape_prose_path(&label)
    )
}

/// Render the diagnostic block for
/// [`CompositionError::InlineComposeSequenceMismatch`].
///
/// Builds the blank-line-separated paragraph sequence: the opening statement;
/// the explanation (document link, both property names, what `sequence` does,
/// and the `claudine sequence` directive); and the upcoming-`sections` note.
/// The authored frontmatter YAML block is appended after this diagnostic by the
/// CLI error walker when the error is enriched with a [`FrontmatterExcerpt`].
fn render_inline_sequence_mismatch_block(source_path: &std::path::Path) -> StatusBlock {
    let file_link = render_file_link(source_path);

    let opening =
        Prose::new("You tried to run an inline-compose operation on a document configured as a sequence.");

    let explanation = Prose::new(format!(
        "The document {file_link} defines both <cyan>`prompt`</cyan> and <cyan>`sequence`</cyan>. \
         A <cyan>`sequence`</cyan> makes each state invoke an inline-compose operation using \
         <cyan>`prompt`</cyan>, so run it with <cyan>`claudine sequence`</cyan> instead."
    ));

    let sections_note = Prose::new(
        "Note: the upcoming <cyan>`sections`</cyan> feature may be a better fit when each \
         operation should update a particular section of the document. It may not suit every \
         sequence workflow and is not available yet.",
    );

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "CompositionError",
            "inline-compose on a sequence",
        ))
        .body(vec![opening, explanation, sections_note])
        .hint("Run the document with `claudine sequence <file>`.")
}

/// Render the human-facing body for [`CompositionError::AgentResolutionFailed`].
///
/// The no-TTY abort body must be the **same** styled message the TTY path
/// would show for the state, so it shares one source of truth with the
/// dry-run table cell and the live TTY pre-prompt — see
/// [`super::agent_message`]. The only state with a distinct (imperative)
/// live message is [`AgentResolutionState::SingleInvalid`], which is built
/// here from [`super::agent_message::invalid_agent_message`] plus the
/// installed-agent list the TTY picker would offer.
pub(super) fn render_agent_resolution_failed_body(
    state: &super::super::types::AgentResolutionState,
    installed: &[Provider],
    file_link: &str,
) -> String {
    use super::super::agent_message::{agent_state_breakdown, invalid_agent_message};
    use super::super::types::AgentResolutionState;

    match state {
        AgentResolutionState::SingleInvalid { hint } => {
            let mut body = invalid_agent_message(hint, file_link);
            if installed.is_empty() {
                body.push_str("\n\n<i><dim>(no agents are installed)</dim></i>");
            } else {
                for provider in installed {
                    body.push_str(&format!("\n- {provider}"));
                }
            }
            body
        }
        // Auto-selecting states never abort; keep a diagnostic if they
        // somehow reach this path.
        AgentResolutionState::ListOneInstalled { .. } => format!(
            "Agent resolution unexpectedly aborted for {file_link} despite an auto-selectable suggestion."
        ),
        AgentResolutionState::Selected { provider } => format!(
            "Agent resolution unexpectedly aborted for {file_link} when <b>{provider}</b> was already selected."
        ),
        // Every other prompting state aborts with the same breakdown the
        // dry-run table predicts and the TTY path shows.
        other => agent_state_breakdown(other),
    }
}

fn render_sequence_missing_properties_block(
    failures: &[SequenceMissingPropertiesStep],
) -> StatusBlock {
    let plural = if failures.len() == 1 { "step" } else { "steps" };
    let mut body = format!(
        "Missing required schema properties in {} {plural} of the sequence.",
        failures.len()
    );

    for failure in failures {
        let file_link = render_file_link(&failure.source_path);
        body.push_str(&format!(
            "\n\n<b>Step {}: <cyan>{}</cyan></b> ({file_link})",
            failure.step,
            escape_prose_path(&failure.step_name),
        ));
        if let Some(desc) = failure
            .frontmatter_description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
        {
            body.push_str(&format!("\n  <i><dim>{}</dim></i>", escape_prose_path(desc)));
        }
        if !failure.missing.is_empty() {
            for prop in &failure.missing {
                let type_label = prop
                    .type_label
                    .as_deref()
                    .filter(|t| !t.is_empty())
                    .unwrap_or("(unknown type)");
                let mut line = format!("\n  - <cyan>`{}`</cyan>: {}", prop.name, type_label);
                if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
                    line.push_str(&format!(" <i><dim>— {}</dim></i>", escape_prose_path(desc)));
                }
                body.push_str(&line);
            }
        } else if !failure.pointer_paths.is_empty() {
            for pointer in &failure.pointer_paths {
                body.push_str(&format!("\n  - <cyan>`{pointer}`</cyan>"));
            }
        }
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "CompositionError",
            "sequence missing properties",
        ))
        .body(body)
        .hint(
            "Fix the missing values in the sequence document (or pass them via --set) and re-run; \
             every step is validated before the first provider session starts.",
        )
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

fn escape_prose_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

/// Map a `ComposeFailed`'s inner [`MarkdownError`] to a composition code,
/// delegating an interpolation failure to its deepest typed cause (design §9:
/// the code follows the same deepest-meaningful-cause walk as rendering).
fn compose_failed_code(md: &MarkdownError) -> &'static str {
    match md {
        MarkdownError::Interpolation { cause, .. } => match cause.as_ref() {
            ExpressionError::FileReference(_) => "composition.invalid_file_reference",
            ExpressionError::UnknownFunction { .. } => "composition.unknown_function",
            _ => "composition.expression_invalid",
        },
        MarkdownError::FrontmatterParse { .. } | MarkdownError::FrontmatterFenceMismatch { .. } => {
            "composition.frontmatter_parse"
        }
        MarkdownError::ShellExpansion(_) => "composition.shell_expansion",
        MarkdownError::SchemaValidationFailed { .. } => "composition.schema_validation",
        _ => "composition.failed",
    }
}

/// Build the `composition.invalid_file_reference` `detail` payload from a
/// [`FileReferenceDiagnostic`].
///
/// Emits exactly the field set the registry declares (`reference`, `kind`,
/// `base_dir`, `suggestions`, `fallback_dir`). `kind` is the catalog snake_case
/// slug, never the `Debug` form. `suggestions` reuses the **same** render-time
/// did-you-mean computation as the interpolation block (a missing reference,
/// `base_dir`-joined, ranked against its siblings) so `err.detail.suggestions`
/// is byte-for-byte what the human report shows. `fallback_dir` is omitted (it
/// projects to `null`) when the resolution context carried none.
fn file_reference_detail(diagnostic: &FileReferenceDiagnostic) -> Value {
    // Mirror the render gate (errors/blocks.rs): suggestions are computed only
    // for a *missing* reference — a malformed/remote reference has no sibling
    // hint, so the array stays empty rather than fabricating one.
    let suggestions = if matches!(diagnostic.kind, FileRefFailure::NotFound) {
        let expected = diagnostic.base_dir.join(&diagnostic.reference);
        suggest_sibling_files(&expected, DEFAULT_MAX_SUGGESTIONS)
    } else {
        Vec::new()
    };
    json!({
        "reference": diagnostic.reference,
        "kind": diagnostic.kind.as_str(),
        "base_dir": diagnostic.base_dir.to_string_lossy(),
        "suggestions": suggestions,
        "fallback_dir": diagnostic
            .fallback_dir
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
    })
}

impl Diagnostic for CompositionError {
    fn code(&self) -> &'static str {
        match self {
            // Transparent wrapper: classify by the cause it carries (§6).
            CompositionError::WithFrontmatter { inner, .. } => inner.code(),
            CompositionError::ComposeFailed(md) => compose_failed_code(md),
            CompositionError::InvalidReference { .. } | CompositionError::FileNotFound { .. } => {
                "composition.invalid_file_reference"
            }
            CompositionError::SchemaLoad { .. } => "composition.schema_load",
            CompositionError::SchemaParse { .. } => "composition.schema_parse",
            CompositionError::SchemaValidation { .. }
            | CompositionError::UnresolvedFileReference { .. } => "composition.schema_validation",
            CompositionError::MissingProperties { .. }
            | CompositionError::SequenceMissingProperties { .. } => "composition.missing_properties",
            CompositionError::FrontmatterParse(_) => "composition.frontmatter_parse",
            CompositionError::ShellExpansionFailed { .. } => "composition.shell_expansion",
            CompositionError::AtomicWriteFailed { .. } => "io.write_failed",
            // The lifecycle-stack family shares one authoring-error code; the
            // `variant` facet still distinguishes them for finer handlers.
            CompositionError::LifecycleInvalid { .. }
            | CompositionError::LifecycleSayConflict(_)
            | CompositionError::LifecycleUnknownEffect(..)
            | CompositionError::LifecycleInterpolationLeak { .. }
            | CompositionError::LifecycleUndefinedVariable { .. }
            | CompositionError::LifecycleStackInvalidShape { .. }
            | CompositionError::LifecycleActionInvalidShortForm { .. }
            | CompositionError::LifecycleActionInvalidLongForm { .. }
            | CompositionError::LifecycleUnknownVerb { .. }
            | CompositionError::LifecycleStackAmbiguous { .. }
            | CompositionError::LifecycleWrongArity { .. }
            | CompositionError::LifecycleActionPlacement { .. }
            | CompositionError::LifecycleActionOrder { .. }
            | CompositionError::LifecycleMultipleLifecycleActions { .. }
            | CompositionError::LifecycleInvalidArgs { .. }
            | CompositionError::LifecycleErrNotAvailable { .. }
            | CompositionError::LifecycleEvaluationError { .. } => "composition.lifecycle_invalid",
            // Everything else is a composition failure without a finer code yet.
            _ => "composition.failed",
        }
    }

    fn category(&self) -> Category {
        code_spec(self.code())
            .map(|spec| spec.category)
            .unwrap_or(Category::Composition)
    }

    fn disposition(&self) -> Disposition {
        code_spec(self.code())
            .map(|spec| spec.disposition)
            .unwrap_or(Disposition::Correctable)
    }

    fn origin(&self) -> Origin {
        code_spec(self.code())
            .map(|spec| spec.origin)
            .unwrap_or(Origin::Author)
    }

    fn detail(&self) -> Value {
        // Seed every key the mapped code declares to `null` so an unavailable
        // optional still projects its key (error-catalog §2.5). Explicit arms
        // overwrite the keys they can populate; variants with no extractable
        // specifics keep the all-`null` base.
        let mut base = null_detail_for(self.code());

        match self {
            CompositionError::WithFrontmatter { inner, .. } => return inner.detail(),
            CompositionError::ComposeFailed(MarkdownError::Interpolation {
                cause,
                expression,
                ..
            }) => match cause.as_ref() {
                ExpressionError::FileReference(diagnostic) => {
                    return file_reference_detail(diagnostic);
                }
                // `composition.unknown_function` declares `name`, `suggestions`.
                // The interpolation cause carries no sibling-name ranking, so
                // `suggestions` stays the seeded empty array.
                ExpressionError::UnknownFunction { name } => {
                    base["name"] = json!(name);
                    base["suggestions"] = json!([]);
                }
                other => {
                    base["expression"] = json!(expression);
                    base["message"] = json!(other.to_string());
                }
            },
            // The remaining `composition.invalid_file_reference` constructors
            // (the typed-source and resolved-not-found variants) carry no
            // `FileReferenceDiagnostic`, so only `reference` is recoverable.
            CompositionError::InvalidReference { reference, .. } => {
                base["reference"] = json!(reference);
            }
            CompositionError::FileNotFound(reference) => {
                base["reference"] = json!(reference);
            }
            CompositionError::SchemaParse {
                source_path,
                property,
                message,
                ..
            } => {
                base["source_path"] = json!(source_path.to_string_lossy());
                base["property"] = json!(property);
                base["message"] = json!(message);
            }
            CompositionError::SchemaLoad {
                source_path,
                message,
            } => {
                base["source_path"] = json!(source_path.to_string_lossy());
                base["message"] = json!(message);
            }
            // `composition.schema_validation` declares `source_path`,
            // `problems`, `pointer_paths`.
            CompositionError::SchemaValidation {
                source_path,
                problems,
                ..
            } => {
                base["source_path"] = json!(source_path.to_string_lossy());
                base["problems"] = json!(problems);
                base["pointer_paths"] = json!([]);
            }
            // Shares the `composition.schema_validation` code; project the
            // property pointer as its single problem.
            CompositionError::UnresolvedFileReference {
                source_path,
                property,
                ..
            } => {
                base["source_path"] = json!(source_path.to_string_lossy());
                base["problems"] = json!([format!("/{property}")]);
                base["pointer_paths"] = json!([]);
            }
            // `composition.missing_properties` declares `missing`,
            // `pointer_paths`.
            CompositionError::MissingProperties {
                missing,
                pointer_paths,
                ..
            } => {
                base["missing"] =
                    json!(missing.iter().map(|p| p.name.clone()).collect::<Vec<_>>());
                base["pointer_paths"] = json!(pointer_paths);
            }
            // `composition.shell_expansion` declares `command`: the failed
            // authored shell command, NOT the Markdown source path. Most
            // `ShellExpansionError` variants carry it; the command-less variants
            // (`ParseDirective`, `PolicyIo`, `Preflight`) leave the seeded
            // `null` in place rather than substitute the file path.
            CompositionError::ShellExpansionFailed { error, .. } => {
                if let Some(command) = error.command() {
                    base["command"] = json!(command);
                }
            }
            // `io.write_failed` declares `path`.
            CompositionError::AtomicWriteFailed { path, .. } => {
                base["path"] = json!(path.to_string_lossy());
            }
            // `composition.lifecycle_invalid` declares `property`, `message`.
            // The lifecycle family threads a `property` (and usually a
            // `message`); project both where the variant carries them.
            CompositionError::LifecycleInvalid {
                property, message, ..
            } => {
                base["property"] = json!(property);
                base["message"] = json!(message);
            }
            CompositionError::LifecycleStackInvalidShape {
                property, message, ..
            }
            | CompositionError::LifecycleStackAmbiguous {
                property, message, ..
            } => {
                base["property"] = json!(property);
                base["message"] = json!(message);
            }
            CompositionError::LifecycleActionInvalidShortForm {
                property, message, ..
            }
            | CompositionError::LifecycleActionInvalidLongForm {
                property, message, ..
            }
            | CompositionError::LifecycleWrongArity {
                property, message, ..
            }
            | CompositionError::LifecycleInvalidArgs {
                property, message, ..
            } => {
                base["property"] = json!(property);
                base["message"] = json!(message);
            }
            CompositionError::LifecycleUnknownVerb {
                property, verb, ..
            } => {
                base["property"] = json!(property);
                base["message"] = json!(format!("unknown lifecycle action `{verb}`"));
            }
            CompositionError::LifecycleActionPlacement {
                property, action, ..
            } => {
                base["property"] = json!(property);
                base["message"] = json!(format!("action `{action}` is not valid here"));
            }
            CompositionError::LifecycleErrNotAvailable {
                property, event, ..
            } => {
                base["property"] = json!(property);
                base["message"] = json!(format!("`err` is not available in the `{event}` event"));
            }
            CompositionError::LifecycleEvaluationError {
                event, message, ..
            } => {
                base["property"] = json!(event);
                base["message"] = json!(message);
            }
            CompositionError::LifecycleSayConflict(property)
            | CompositionError::LifecycleUnknownEffect(property, _)
            | CompositionError::LifecycleInterpolationLeak { property, .. }
            | CompositionError::LifecycleUndefinedVariable { property, .. }
            // These two cardinality errors carry no dedicated `message` field,
            // so synthesize one from the `#[error]` rendering like the other
            // message-less lifecycle variants above.
            | CompositionError::LifecycleActionOrder { property, .. }
            | CompositionError::LifecycleMultipleLifecycleActions { property, .. } => {
                base["property"] = json!(property);
                base["message"] = json!(self.to_string());
            }
            // Every other variant maps to a code (`composition.failed`,
            // `composition.frontmatter_parse`, …) whose declared keys it cannot
            // populate from extractable instance data; the all-`null` base
            // already satisfies the registry key set.
            _ => {}
        }

        base
    }
}
