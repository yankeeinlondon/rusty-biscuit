use biscuit_terminal::prelude::{Prose, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use claudine::provider::Provider;
use color_eyre::eyre::Result;
use std::path::Path;

use crate::log;

pub(crate) fn strip_prompt_tags_for_provider(provider: Provider, prompt: &str) -> String {
    if matches!(
        provider,
        Provider::Codex | Provider::Gemini | Provider::OpenCode
    ) {
        claudine::mcp::session::lex_tags(prompt).0
    } else {
        prompt.to_string()
    }
}

pub(crate) fn extract_tags_from_prompt(
    prompt: Option<&str>,
    extract_tags: fn(&str) -> (String, Vec<String>),
) -> (Option<String>, Vec<String>) {
    let Some(prompt) = prompt else {
        return (None, Vec::new());
    };
    let (cleaned, tags) = extract_tags(prompt);
    if tags.is_empty() {
        (None, Vec::new())
    } else {
        (Some(cleaned), tags)
    }
}

/// Report how the inline agent's own process ended, before the loop decides
/// what to do about it.
///
/// The interrupt notice is worded prospectively ("will restore"): this runs
/// inside the attempt, and the baseline is not put back until the loop reaches
/// its rollback seam. A past-tense claim here would contradict the typed
/// rollback-failure diagnostic in the one case where accuracy matters.
#[allow(clippy::too_many_arguments)]
pub(crate) fn report_inline_agent_status(
    provider: Provider,
    source_path: &Path,
    final_response: &str,
    agent_exit: i32,
    termination: claudine::harness::ProcessTermination,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) {
    let provider_name = crate::output::capitalize_provider(provider);
    let display_path = biscuit_file::to_portable_string(
        source_path.strip_prefix(child_cwd).unwrap_or(source_path),
    );
    let was_interrupted = matches!(
        termination,
        claudine::harness::ProcessTermination::Interrupted
    ) || agent_exit == 130
        || agent_exit == 143;

    if show_checks {
        if was_interrupted {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent was interrupted by the user (code {agent_exit})"),
                term,
            ));
        } else if agent_exit == 0 {
            log::message(&crate::output::fm_check_ok(
                &format!("{provider_name} agent completed successfully"),
                term,
            ));
        } else {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent exited with error (code {agent_exit})"),
                term,
            ));
        }
    }

    if was_interrupted && show_checks {
        log::message(&crate::output::fm_check_fail(
            &format!(
                "<b>User interrupted the agent with CTRL+C; Claudine will restore \
                 <blue-500>{display_path}</blue-500> to its pre-run state.</b>"
            ),
            term,
        ));
        if !final_response.trim().is_empty() {
            log::message(
                &Prose::new(format!("<b>Agent summary:</b>\n\n{final_response}")).render(term),
            );
        }
    }
}

/// Report what the closure did to the agent's on-disk artifact.
///
/// Display only: the artifact has already been reconciled, stamped, and written
/// by [`complete_active_document`], which is the single seam that judges a run.
/// A restored owned property is a warning, never an error — the caller's value
/// is authoritative and the agent's edit was discarded, and the author is told
/// exactly which property that happened to.
///
/// [`complete_active_document`]: claudine::composition::complete_active_document
pub(crate) fn report_inline_artifact(
    artifact: &claudine::composition::InlineArtifact,
    source_path: &Path,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) {
    if !show_checks {
        return;
    }
    use biscuit_terminal::components::status::{Status, StatusState};
    use biscuit_terminal::prelude::TerminalRenderable;

    let display_path = biscuit_file::to_portable_string(
        source_path.strip_prefix(child_cwd).unwrap_or(source_path),
    );
    log::message(&crate::output::fm_check_ok(
        &format!("The agent updated <blue-500>{display_path}</blue-500>"),
        term,
    ));
    log::message(&crate::output::fm_check_ok(
        "Stamped <bold>hash</bold> and <bold>last_updated</bold>",
        term,
    ));

    for key in &artifact.restored_properties {
        let status = Status::from_prose(format!(
            "The agent changed the caller-owned property <b>\"{key}\"</b> — restored the authored value"
        ))
        .state(StatusState::Warning);
        log::message(&status.render(term));
    }

    if artifact.body_cleaned {
        log::message(&crate::output::fm_check_ok(
            "Cleaned up generated markdown formatting",
            term,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;
    use claudine::composition::{
        CompletionContext, CompositionMode, InlineClosurePlan, complete_active_document,
    };
    use tempfile::TempDir;

    #[test]
    fn extracts_tags_from_codex_prompt_position() {
        let prompt = "fix #calendar bugs";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), claudine::mcp::session::lex_tags);

        assert_eq!(tags, vec!["calendar"]);
        assert_eq!(cleaned.as_deref(), Some("fix bugs"));
    }

    #[test]
    fn extracts_tags_from_gemini_prompt_flag() {
        let prompt = "debug #slack auth";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), claudine::mcp::session::lex_tags);

        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned.as_deref(), Some("debug auth"));
    }

    /// Seed the guard from `original`, then let the "agent" leave
    /// `agent_wrote` on disk.
    fn agent_run(
        dir: &TempDir,
        original: &str,
        agent_wrote: &str,
    ) -> (std::path::PathBuf, InlineClosurePlan) {
        let file = dir.path().join("doc.md");
        let original_md: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            document_path: file.clone(),
            original_document_text: original.to_string(),
            original_hash: original_md.compute_hash(
                darkmatter::markdown::hash::MdHashKind::Simple,
                &claudine::composition::closure::inline_hash_options(),
            ),
        };
        std::fs::write(&file, agent_wrote).unwrap();
        (file, plan)
    }

    /// Drive the same completion seam the harness loop drives, and report the
    /// artifact exactly as the loop does.
    fn complete_inline(
        file: &std::path::Path,
        plan: &InlineClosurePlan,
        prior_body_change: bool,
    ) -> claudine::composition::CompletionOutcome {
        let live = serde_json::json!({});
        let outcome = complete_active_document(CompletionContext {
            mode: CompositionMode::InlineFrontmatterPrompt,
            active_path: file,
            launch_schema: None,
            live_frontmatter: &live,
            inline_guard: Some(plan),
            prior_body_change,
            today: "2026-09-06",
        })
        .expect("reconciliation should not fail");
        if let Some(artifact) = outcome.artifact.as_deref() {
            report_inline_artifact(
                artifact,
                file,
                file.parent().unwrap(),
                false,
                &Terminal::new_optimistic(120),
            );
        }
        outcome
    }

    #[test]
    fn inline_completion_cleans_the_agents_body_on_disk() {
        let dir = TempDir::new().unwrap();
        let (file, plan) = agent_run(
            &dir,
            "---\nprompt: test\nlast_updated: '2026-01-01'\n---\nOld body\n",
            // The agent wrote a heading with no blank line before the paragraph.
            "---\nprompt: test\nlast_updated: '2026-01-01'\n---\n# Generated Title\nParagraph without blank line\n",
        );

        let outcome = complete_inline(&file, &plan, false);
        assert!(outcome.verdict.is_satisfied());

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(
            on_disk.contains("# Generated Title\n\nParagraph without blank line"),
            "body must be cleaned (blank line between header and paragraph); got:\n{on_disk}"
        );
        assert!(on_disk.contains("last_updated: '2026-09-06'"), "got:\n{on_disk}");
        assert!(
            on_disk.contains("prompt: test"),
            "original frontmatter must be preserved"
        );
    }

    #[test]
    fn inline_completion_cleans_table_alignment() {
        let dir = TempDir::new().unwrap();
        let (file, plan) = agent_run(
            &dir,
            "---\nprompt: test\n---\nOld body\n",
            "---\nprompt: test\n---\n|A|B|\n|---|---|\n|short|much longer column|\n",
        );

        let outcome = complete_inline(&file, &plan, false);
        assert!(outcome.verdict.is_satisfied());

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(
            on_disk.contains("| A ") && on_disk.contains("| B "),
            "table cells should be padded with leading space; got:\n{on_disk}"
        );
    }

    #[test]
    fn inline_completion_reports_a_document_the_agent_never_updated() {
        let dir = TempDir::new().unwrap();
        let original = "---\nprompt: test\n---\nOld body\n";
        let (file, plan) = agent_run(&dir, original, original);

        let outcome = complete_inline(&file, &plan, false);

        let error = outcome
            .verdict
            .into_error()
            .expect("an untouched document must not complete");
        assert!(
            matches!(
                error,
                claudine::composition::CompositionError::CompletionBodyUnchanged {
                    reason: claudine::composition::BodyRejection::Unchanged,
                    ..
                }
            ),
            "unexpected error: {error}"
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    }

    /// AC18: a metadata-only recovery is not refused as unchanged once the
    /// operation already produced a body.
    #[test]
    fn inline_completion_accepts_a_metadata_only_recovery_after_a_prior_body_change() {
        let dir = TempDir::new().unwrap();
        let original = "---\nprompt: test\n---\nOld body\n";
        let (file, plan) = agent_run(
            &dir,
            original,
            "---\nprompt: test\nresearched_by: gpt-5\n---\nOld body\n",
        );

        let outcome = complete_inline(&file, &plan, true);

        assert!(
            outcome.verdict.is_satisfied(),
            "carried body-change evidence must satisfy the body half: {:?}",
            outcome.verdict
        );
        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(on_disk.contains("researched_by: gpt-5"), "got:\n{on_disk}");
        // The document carried no authored `last_updated`, so the stamp is
        // added in the writer's own plain scalar style.
        assert!(on_disk.contains("last_updated: 2026-09-06"), "got:\n{on_disk}");
        assert!(on_disk.contains("hash:"), "an accepted recovery stamps; got:\n{on_disk}");
    }

    /// The evidence never rescues an empty body: a blank document is not a
    /// deliverable no matter what an earlier attempt produced.
    #[test]
    fn inline_completion_still_refuses_an_empty_body_with_prior_evidence() {
        let dir = TempDir::new().unwrap();
        let original = "---\nprompt: test\n---\nOld body\n";
        let (file, plan) = agent_run(&dir, original, "---\nprompt: test\n---\n\n");

        let outcome = complete_inline(&file, &plan, true);

        assert!(matches!(
            outcome.verdict.body,
            claudine::composition::BodyEvidence::Rejected(
                claudine::composition::BodyRejection::Empty
            )
        ));
    }
}
