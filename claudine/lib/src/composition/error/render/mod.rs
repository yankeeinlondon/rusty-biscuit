//! Terminal rendering and diagnostics for [`CompositionError`].
//!
//! [`BlockError::status_block`] is a thin exhaustive dispatcher: it groups the
//! error variants by family and delegates the actual block construction to a
//! per-family module. Each family renderer returns the same [`StatusBlock`]
//! and reuses the shared path/link helpers below rather than copying styles or
//! prose. The [`Diagnostic`] impl (error codes and machine-facing `detail`
//! payloads) lives here alongside the shared helpers.

use super::*;

mod lifecycle;
mod provider;
mod schema;
mod selection;
mod sequence_loop;

#[cfg(test)]
pub(crate) use selection::render_agent_resolution_failed_body;

impl BlockError for CompositionError {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        match self {
            // Transparent wrappers render their inner error directly, so the
            // deepest typed block survives (see the CLI error walker).
            CompositionError::WithFrontmatter { inner, .. }
            | CompositionError::LifecycleEvaluationAlreadyEmitted { inner } => {
                inner.status_block(term)
            }

            // Lifecycle authoring / evaluation family.
            CompositionError::LifecycleInvalid { .. }
            | CompositionError::LifecycleInterpolationLeak { .. }
            | CompositionError::LifecycleUndefinedVariable { .. }
            | CompositionError::LifecycleEvaluationError { .. }
            | CompositionError::RemovedValidationKey { .. }
            | CompositionError::LifecycleStackInvalidShape { .. }
            | CompositionError::LifecycleActionInvalidShortForm { .. }
            | CompositionError::LifecycleActionInvalidLongForm { .. }
            | CompositionError::LifecycleUnknownVerb { .. }
            | CompositionError::LifecycleStackAmbiguous { .. }
            | CompositionError::LifecycleObjectDataThroughInterpolationPositional { .. }
            | CompositionError::LifecycleObjectDataThroughInterpolationParameter { .. }
            | CompositionError::LifecycleWrongArity { .. }
            | CompositionError::LifecycleShortFormRemoved { .. }
            | CompositionError::LifecycleActionPlacement { .. }
            | CompositionError::LifecycleMultipleLifecycleActions { .. }
            | CompositionError::LifecycleActionOrder { .. }
            | CompositionError::LifecycleInvalidArgs { .. }
            | CompositionError::LifecycleErrNotAvailable { .. } => lifecycle::status_block(self),

            // Schema / frontmatter validation family.
            CompositionError::SchemaLoad { .. }
            | CompositionError::SchemaParse { .. }
            | CompositionError::SchemaValidation { .. }
            | CompositionError::MissingProperties { .. }
            | CompositionError::UnsupportedInteractiveSchema { .. } => schema::status_block(self),

            // Selection / target family.
            CompositionError::AgentResolutionFailed { .. }
            | CompositionError::AutocompleteNoMatches { .. }
            | CompositionError::AutocompleteOverCap { .. }
            | CompositionError::AutocompleteNotInteractive
            | CompositionError::AutocompleteCancelled { .. } => selection::status_block(self),

            // Sequence / loop family.
            CompositionError::LoopIterationFailed { .. }
            | CompositionError::LoopRateLimited { .. }
            | CompositionError::SequenceMissingProperties { .. }
            | CompositionError::SequenceInteractiveRejected(..)
            | CompositionError::InlineComposeSequenceMismatch { .. } => {
                sequence_loop::status_block(self)
            }

            // Provider / execution / file-reference family, plus the generic
            // catch-all for every variant without a dedicated block.
            _ => provider::status_block(self, term),
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

/// Render an absolute OSC8 hyperlink to `path` showing its relative form
/// where possible (falling back to the full display).
///
/// The Prose layer downgrades `<a href>` to plain text when the terminal
/// does not support OSC8.
pub(super) fn render_file_link(path: &std::path::Path) -> String {
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

pub(super) fn escape_prose_path(input: &str) -> String {
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
