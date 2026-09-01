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
        if final_response.trim().is_empty() {
            log::message(&crate::output::fm_check_fail(
                &format!(
                    "<b>User interrupted the agent with CTRL+C; the body of \
                     <blue-500>{display_path}</blue-500> is empty so it appears no work was accomplished.</b>"
                ),
                term,
            ));
        } else {
            log::message(&crate::output::fm_check_fail(
                &format!(
                    "<b>User interrupted the agent with CTRL+C; the body of \
                     <blue-500>{display_path}</blue-500> has been at least partially filled:</b>"
                ),
                term,
            ));
            eprintln!();
            for line in final_response.lines() {
                eprintln!("  {line}");
            }
        }
    }
}

/// Attempt inline closure validation and application.
///
/// Returns `Ok(())` on success (file rewritten), or a list of human-readable
/// failure messages if the closure could not be applied.
pub(crate) fn try_inline_closure(
    closure_plan: &claudine::composition::InlineClosurePlan,
    final_response: &str,
    source_path: &Path,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) -> Result<(), Vec<String>> {
    let display_path = biscuit_file::to_portable_string(
        source_path.strip_prefix(child_cwd).unwrap_or(source_path),
    );

    let replacement = match claudine::composition::closure::extract_replacement_parts(
        final_response,
    ) {
        Ok(body) => body,
        Err(error) => {
            let message = format!(
                "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
            );
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            return Err(vec![message]);
        }
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match claudine::composition::closure::apply_inline_closure(
        closure_plan,
        &replacement,
        source_path,
        &today,
    ) {
        Ok(result) => {
            if show_checks {
                use biscuit_terminal::components::status::{Status, StatusState};
                use biscuit_terminal::prelude::TerminalRenderable;

                log::message(&crate::output::fm_check_ok(
                    "Applied the captured replacement body to the target document",
                    term,
                ));
                log::message(&crate::output::fm_check_ok(
                    "Preserved original frontmatter and updated <bold>last_updated</bold>",
                    term,
                ));

                for key in &result.inserted_properties {
                    log::message(&crate::output::fm_check_ok(
                        &format!("Inserted response frontmatter property <bold>\"{key}\"</bold>"),
                        term,
                    ));
                }

                for key in &result.refreshed_properties {
                    log::message(&crate::output::fm_check_ok(
                        &format!("Refreshed response frontmatter property <bold>\"{key}\"</bold>"),
                        term,
                    ));
                }

                for notice in &result.ignored_properties {
                    let status = Status::from_prose(format!(
                        "Response frontmatter property <b>\"{}\"</b> at line {} was not authorized — ignored",
                        notice.key, notice.line
                    ))
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }

                for key in &result.missing_properties {
                    let status = Status::from_prose(format!(
                        "Allowed response frontmatter property <b>\"{key}\"</b> was not returned"
                    ))
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }

                for key in &result.restored_frontmatter_properties {
                    let status = Status::from_prose(format!(
                        "Frontmatter property <b>\"{key}\"</b> changed on disk during the run — restored the authored value"
                    ))
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }

                if result.unclassified_frontmatter_drift_restored {
                    let status = Status::from_prose(
                        "The document frontmatter changed on disk during the run and could not be compared property by property — restored the authored frontmatter",
                    )
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }

                if result.body_drift_restored {
                    let status = Status::from_prose(
                        "The document body changed on disk during the run — applied the captured replacement body",
                    )
                    .state(StatusState::Info);
                    log::message(&status.render(term));
                }
            }

            if result.body_cleaned && show_checks {
                log::message(&crate::output::fm_check_ok(
                    "Cleaned up generated markdown formatting",
                    term,
                ));
            }

            Ok(())
        }
        Err(error) => {
            let message = format!("failed to rewrite {display_path}: {error}");
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            Err(vec![message])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;
    use claudine::composition::InlineClosurePlan;
    use claudine::mcp::session::lex_tags;
    use tempfile::TempDir;

    #[test]
    fn extracts_tags_from_codex_prompt_position() {
        let prompt = "fix #calendar bugs";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), lex_tags);

        assert_eq!(tags, vec!["calendar"]);
        assert_eq!(cleaned.as_deref(), Some("fix bugs"));
    }

    #[test]
    fn extracts_tags_from_gemini_prompt_flag() {
        let prompt = "debug #slack auth";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), lex_tags);

        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned.as_deref(), Some("debug auth"));
    }

    #[test]
    fn try_inline_closure_writes_cleaned_body_to_disk() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("doc.md");

        let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
        std::fs::write(&file, original).unwrap();

        let original_md: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_hash: original_md.compute_hash(
                darkmatter::markdown::hash::MdHashKind::Simple,
                &claudine::composition::closure::inline_hash_options(),
            ),
            response_frontmatter: Vec::new(),
        };

        let term = Terminal::new_optimistic(120);
        // "Dirty" replacement: no blank line between header and paragraph
        let dirty_response = "# Generated Title\nParagraph without blank line\n";

        try_inline_closure(
            &plan,
            dirty_response,
            &file,
            file.parent().unwrap(),
            false,
            &term,
        )
        .expect("closure should succeed");

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(
            on_disk.contains("# Generated Title\n\nParagraph without blank line"),
            "body must be cleaned (blank line between header and paragraph); got:\n{on_disk}"
        );
        assert!(
            on_disk.contains("last_updated:"),
            "frontmatter must include last_updated"
        );
        assert!(
            on_disk.contains("prompt: test"),
            "original frontmatter must be preserved"
        );
    }

    #[test]
    fn try_inline_closure_cleans_table_alignment() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("doc.md");

        let original = "---\nprompt: test\n---\nOld body\n";
        std::fs::write(&file, original).unwrap();

        let original_md: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_hash: original_md.compute_hash(
                darkmatter::markdown::hash::MdHashKind::Simple,
                &claudine::composition::closure::inline_hash_options(),
            ),
            response_frontmatter: Vec::new(),
        };

        let term = Terminal::new_optimistic(120);
        // Unaligned table
        let dirty_response =
            "|A|B|\n|---|---|\n|short|much longer column|\n";

        try_inline_closure(
            &plan,
            dirty_response,
            &file,
            file.parent().unwrap(),
            false,
            &term,
        )
        .expect("closure should succeed");

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(
            on_disk.contains("| A "),
            "table cells should be padded with leading space; got:\n{on_disk}"
        );
        assert!(
            on_disk.contains("| B "),
            "table cells should be padded with leading space; got:\n{on_disk}"
        );
    }
}
