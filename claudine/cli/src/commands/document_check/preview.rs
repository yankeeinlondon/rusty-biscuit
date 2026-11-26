//! The passive preview of one prompt task.
//!
//! The prompt is prepared through the canonical preparer
//! ([`composition::prepare_direct`] or [`composition::prepare_inline`]) with a
//! passive session, so the preview is what the real command would compose,
//! except that work needing an effect is left as written. Each thing left
//! as written is its own `not checked` row, and the human preview marks it.

use claudine::composition::CallerInputLayers;
use darkmatter::markdown::compose::passive::{Deferral, DeferredConstruct, PassiveSession};

use super::*;

/// Appended to a construct the preview shows as written.
const DEFERRED_MARK: &str = "[not evaluated in a dry run]";

impl Check {
    /// Prepares the task's prompt passively and keeps its preview, or records
    /// why it could not be previewed.
    pub(super) fn preview(&mut self, task: &PromptTask<'_>) {
        let session = PassiveSession::new();
        let Some(options) = self.passive_prepare_options(task, session.clone()) else { return };
        // A sequence step composes a document with a string `prompt:`
        // inline, as the real run decides for its own document and its
        // prompt files.
        let inline = match self.mode {
            Mode::InlineCompose => true,
            Mode::Compose => false,
            Mode::Sequence => {
                matches!(task.prompt.markdown.frontmatter().as_map().get("prompt"), Some(Value::String(_)))
            }
        };
        let prepared = if inline {
            composition::prepare_inline(task.prompt, options)
        } else {
            composition::prepare_direct(task.prompt, options)
        };
        let deferrals = session.deferrals();
        for deferral in &deferrals {
            self.sink.not_checked(task.subject, check_name(deferral), &deferral.reason.describe(), None);
        }
        match prepared {
            Ok(prepared) => self.previews.push(marked(&prepared.prompt, &deferrals)),
            Err(CompositionError::ComposedBodyEmpty { cause, .. }) => {
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
            }
            Err(error) => {
                self.push_error(task.subject, &error, task.trail.clone());
            }
        }
    }

    /// The canonical preparation options of `task`'s prompt as a dry run
    /// recording into `session`: the task's evaluated `with:` values as data
    /// beneath the caller's setters, as the task's turn layers them. `None`
    /// once an error is recorded.
    pub(super) fn passive_prepare_options(
        &mut self,
        task: &PromptTask<'_>,
        session: PassiveSession,
    ) -> Option<composition::PrepareOptions> {
        let invocation = self.invocation.clone()?;
        let context = match invocation.derive_composition_source(task.prompt) {
            Ok(source) => source.file_resolution_context().clone(),
            Err(error) => {
                self.push_error(task.subject, &error, task.trail.clone());
                return None;
            }
        };
        let mut overrides = task.with_evaluated.clone();
        let data_keys: std::collections::BTreeSet<String> =
            overrides.keys().filter(|key| !self.setters.contains_key(*key)).cloned().collect();
        overrides.extend(self.setters.clone());
        let layers = CallerInputLayers::from_caller_overrides(
            Some(Value::Object(overrides)),
            invocation.launch_file_resolution_context().clone(),
        );
        let epoch = invocation.begin_document_epoch();
        // The source repository anchors the document's own references, as
        // `compose/prep.rs` sets it for a real run.
        let source_repo_root = context.repository_root().map(std::path::Path::to_path_buf);
        // `apply_to` installs the layers' (launch) context; the document's
        // own references resolve from its source context, as a real run's
        // preparation resolves them, while the caller's setters keep their
        // launch-anchored records.
        let mut options = layers.apply_to(composition::PrepareOptions::new(context.clone()));
        options.file_resolution_context = context;
        options.source_repo_root = source_repo_root;
        options.file_ref_fallback_dir = Some(invocation.launch_cwd().to_path_buf());
        options.data_override_keys.extend(data_keys);
        options.sequence_binding = task.seq.clone();
        options.prepared_context =
            Some(epoch.capture_launch_context(&ContextRequirements::for_document(&task.prompt.markdown)));
        options.invocation_context = Some(invocation);
        options.document_epoch = Some(epoch);
        options.passive = Some(session);
        Some(options)
    }
}

/// The `not checked` row's check for what `deferral` left as written.
fn check_name(deferral: &Deferral) -> &'static str {
    match deferral.construct {
        DeferredConstruct::FrontmatterValue => "frontmatter value",
        DeferredConstruct::ShellDirective => "`::shell` directive",
        DeferredConstruct::ShellBlock => "`::shell-block`",
        DeferredConstruct::BlockCondition => "`::block` condition",
        DeferredConstruct::SchemaTemplate => "schema",
        _ => "interpolation",
    }
}

/// `prompt` with each construct a deferral left as written marked: a
/// `{{ … }}` span after the span, a directive after its line.
fn marked(prompt: &str, deferrals: &[Deferral]) -> String {
    let mut text = prompt.to_string();
    let mut spans: Vec<&str> = deferrals
        .iter()
        .filter(|deferral| deferral.construct == DeferredConstruct::Interpolation)
        .map(|deferral| deferral.authored.as_str())
        .filter(|span| span.starts_with("{{"))
        .collect();
    // Longest first, so a span that contains another is marked once.
    spans.sort_by_key(|span| std::cmp::Reverse(span.len()));
    spans.dedup();
    for span in spans {
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(at) = rest.find(span) {
            let end = at + span.len();
            out.push_str(&rest[..end]);
            if !rest[end..].starts_with(&format!(" {DEFERRED_MARK}")) {
                out.push(' ');
                out.push_str(DEFERRED_MARK);
            }
            rest = &rest[end..];
        }
        out.push_str(rest);
        text = out;
    }
    let directive_lines: Vec<&str> = deferrals
        .iter()
        .filter_map(|deferral| match deferral.construct {
            DeferredConstruct::ShellDirective | DeferredConstruct::ShellBlock => {
                deferral.authored.lines().next().map(str::trim)
            }
            _ => None,
        })
        .collect();
    let conditions: Vec<&str> = deferrals
        .iter()
        .filter(|deferral| deferral.construct == DeferredConstruct::BlockCondition)
        .map(|deferral| deferral.authored.as_str())
        .collect();
    if directive_lines.is_empty() && conditions.is_empty() {
        return text;
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (body, ending) = line.strip_suffix('\n').map_or((line, ""), |body| (body, "\n"));
        let trimmed = body.trim();
        let deferred = directive_lines.contains(&trimmed)
            || (trimmed.starts_with("::block") && conditions.iter().any(|condition| trimmed.contains(condition)));
        out.push_str(body);
        if deferred {
            out.push(' ');
            out.push_str(DEFERRED_MARK);
        }
        out.push_str(ending);
    }
    out
}

#[cfg(test)]
mod tests {
    use darkmatter::markdown::compose::passive::DeferralReason;

    use super::*;

    fn deferral(construct: DeferredConstruct, authored: &str) -> Deferral {
        Deferral {
            construct,
            reason: DeferralReason::Command { command: "git branch".into() },
            authored: authored.into(),
            key: None,
            line: None,
        }
    }

    #[test]
    fn each_construct_left_as_written_is_marked_and_nothing_else_is() {
        let prompt = "On {{ branch }} for main.\n::shell git log\n::block when=\"branch == 'x'\"\nbody\n::end-block\nplain line\n";
        let deferrals = [
            deferral(DeferredConstruct::Interpolation, "{{ branch }}"),
            deferral(DeferredConstruct::ShellDirective, "::shell git log"),
            deferral(DeferredConstruct::BlockCondition, "branch == 'x'"),
            deferral(DeferredConstruct::FrontmatterValue, "$(git branch)"),
        ];

        assert_eq!(
            marked(prompt, &deferrals),
            format!(
                "On {{{{ branch }}}} {DEFERRED_MARK} for main.\n::shell git log {DEFERRED_MARK}\n::block when=\"branch == 'x'\" {DEFERRED_MARK}\nbody\n::end-block\nplain line\n"
            )
        );
        assert_eq!(marked(prompt, &[]), prompt);
    }
}
