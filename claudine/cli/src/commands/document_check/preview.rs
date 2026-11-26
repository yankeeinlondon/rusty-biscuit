//! The passive preview of one prompt task.

use super::*;

impl Check {
    /// Composes the task's prompt with the dry-run options and keeps its
    /// preview, or records why it could not be previewed.
    pub(super) fn preview(&mut self, task: &PromptTask<'_>) {
        let Some(invocation) = self.invocation.clone() else { return };
        let mut overrides = self.setters.clone();
        for (key, value) in &task.with {
            overrides.insert(key.clone(), value.clone());
        }
        match compose_passively(&invocation, task.prompt, overrides) {
            Ok(composed) => {
                let authored = task.prompt.markdown.content();
                let prompt_text = match self.mode {
                    Mode::InlineCompose => composed
                        .frontmatter()
                        .as_map()
                        .get("prompt")
                        .and_then(Value::as_str)
                        .map_or_else(|| composed.content().to_string(), str::to_string),
                    _ => composed.content().to_string(),
                };
                if let Some(cause) = EmptyBodyCause::classify(authored, &prompt_text) {
                    self.sink.push(
                        task.subject,
                        Diagnostic::actionable(
                            EMPTY_BODY_CODE,
                            Severity::Error,
                            format!("the prompt for this task is empty: {}", cause.phrase()),
                            cause.hint(),
                        )
                        .with_source(DiagnosticSource::file_only(Some(task.prompt.resolved_path.clone())))
                        .with_trail(task.trail.clone()),
                    );
                    return;
                }
                self.previews.push(prompt_text);
            }
            Err(PreviewError::Markdown(error)) if skipped_command(&error) => {
                self.sink.not_checked(task.subject, "preview", "it depends on a command a dry run skips", None);
            }
            Err(PreviewError::Markdown(error)) => self.push_error(task.subject, &error, task.trail.clone()),
            Err(PreviewError::Context(error)) => self.push_error(task.subject, &error, task.trail.clone()),
            Err(PreviewError::Invocation(error)) => self.push_error(task.subject, &error, task.trail.clone()),
        }
    }
}

enum PreviewError {
    Markdown(MarkdownError),
    Context(darkmatter::markdown::compose::ContextBuildError),
    Invocation(claudine::invocation_context::InvocationContextError),
}

/// `prompt` composed as a dry run: no command runs, no cache is read or
/// written, and no approval is asked for.
fn compose_passively(
    invocation: &InvocationContext,
    prompt: &ResolvedCompositionSource,
    overrides: Map<String, Value>,
) -> Result<Markdown, PreviewError> {
    let source = invocation.derive_composition_source(prompt).map_err(PreviewError::Invocation)?;
    let file_resolution_context = source.file_resolution_context().clone();
    let epoch = invocation.begin_document_epoch();
    let context = epoch.capture_launch_context(&ContextRequirements::for_document(&prompt.markdown));
    let options = composition::compose_options_with_context(context)
        .with_source_file(prompt.resolved_path.clone())
        .with_cache_access_mode(CacheAccessMode::Off)
        .with_dry_run();
    // Caller values are authored templates, like the document's own
    // frontmatter; a key the document does not declare is added.
    let mut markdown = prompt.markdown.clone();
    for (key, value) in overrides {
        markdown.fm_insert(&key, value).map_err(PreviewError::Markdown)?;
    }
    let options = composition::with_invocation_config(options, Some(invocation), &file_resolution_context)
        .map_err(PreviewError::Context)?;
    let request = composition::compose_request(options, file_resolution_context).map_err(PreviewError::Context)?;
    markdown.compose_with(&request).map(|(composed, _)| composed).map_err(PreviewError::Markdown)
}

/// Whether `error` is, or was caused by, a command the dry run skipped.
fn skipped_command(error: &MarkdownError) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(cause) = current {
        if let Some(MarkdownError::ShellExpansion(inner)) = cause.downcast_ref::<MarkdownError>()
            && matches!(**inner, ShellExpansionError::SkippedInDryRun { .. })
        {
            return true;
        }
        if matches!(cause.downcast_ref::<ShellExpansionError>(), Some(ShellExpansionError::SkippedInDryRun { .. })) {
            return true;
        }
        current = cause.source();
    }
    false
}
