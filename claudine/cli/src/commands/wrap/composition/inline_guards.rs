use std::collections::HashMap;
use std::path::Path;

use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};

use super::CompositionStreamResult;
use super::summary::{emit_composition_summary, emit_minimal_composition_summary};
use crate::commands::wrap::profile::WrapperProfile;

/// Apply inline closure post-processing after a composition run.
///
/// Validates disk state, applies the closure plan, runs cleanup, and emits
/// the deferred summary. Returns the final exit code.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_inline_closure(
    agent_exit: i32,
    final_response: String,
    deferred_summary: Option<CompositionStreamResult>,
    closure_plan: &claudine::composition::InlineClosurePlan,
    resolved_path: &Path,
    _session_interactive: bool,
    show_checks: bool,
    provider: Provider,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
    child_cwd: &Path,
) -> Result<i32> {
    let mut final_exit = agent_exit;
    let provider_name = crate::output::capitalize_provider(provider);
    let should_separate_checks = deferred_summary
        .as_ref()
        .is_some_and(|result| !result.summary.assistant_text.trim().is_empty());

    if show_checks && should_separate_checks {
        eprintln!();
        eprintln!();
    }

    let was_interrupted = agent_exit == 130 || agent_exit == 143;

    if was_interrupted && show_checks {
        crate::log::message(&crate::output::fm_check_fail(
            &format!("{provider_name} agent was interrupted by the user (code {agent_exit})"),
            term,
        ));
    } else if agent_exit == 0 && show_checks {
        crate::log::message(&crate::output::fm_check_ok(
            &format!("{provider_name} agent completed successfully"),
            term,
        ));
    } else if agent_exit != 0 && show_checks {
        crate::log::message(&crate::output::fm_check_fail(
            &format!("{provider_name} agent exited with error (code {agent_exit})"),
            term,
        ));
    }

    let display_path = resolved_path
        .strip_prefix(child_cwd)
        .unwrap_or(resolved_path)
        .display();

    if was_interrupted {
        report_interruption(&display_path, final_response.trim(), term);
        return Ok(1);
    }

    if agent_exit == 0 {
        let replacement_body = match claudine::composition::closure::extract_replacement_body(
            &final_response,
        ) {
            Ok(body) => body,
            Err(error) => {
                if show_checks {
                    crate::log::message(&crate::output::fm_check_fail(
                        &format!(
                            "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
                        ),
                        term,
                    ));
                }
                final_exit = 1;
                String::new()
            }
        };

        if final_exit == 0 {
            let post_run_fm = std::fs::read_to_string(resolved_path).ok().map(|text| {
                let md: darkmatter::markdown::Markdown = text.into();
                md.frontmatter().as_map().clone()
            });

            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            match claudine::composition::closure::apply_inline_closure(
                closure_plan,
                &replacement_body,
                resolved_path,
                &today,
                post_run_fm.as_ref(),
            ) {
                Ok(result) => {
                    if show_checks {
                        crate::log::message(&crate::output::fm_check_ok(
                            "Applied the captured replacement body to the target document",
                            term,
                        ));
                        crate::log::message(&crate::output::fm_check_ok(
                            "Preserved original frontmatter and updated <bold>last_updated</bold>",
                            term,
                        ));

                        for key in &result.new_properties {
                            crate::log::message(&crate::output::fm_check_ok(
                                &format!("Merged new frontmatter property <bold>\"{key}\"</bold>"),
                                term,
                            ));
                        }

                        for key in &result.reverted_properties {
                            let status = Status::from_prose(format!(
                                "Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value"
                            ))
                            .state(StatusState::Warning);
                            crate::log::message(&status.render(term));
                        }
                    }

                    match cleanup_inline_output(resolved_path) {
                        Ok(true) => {
                            if show_checks {
                                crate::log::message(&crate::output::fm_check_ok(
                                    "Cleaned up generated markdown formatting",
                                    term,
                                ));
                            }
                        }
                        Ok(false) => {}
                        Err(error) => {
                            if show_checks {
                                crate::log::message(&crate::output::fm_check_fail(
                                    &format!("markdown cleanup failed: {error}"),
                                    term,
                                ));
                            }
                        }
                    }
                }
                Err(error) => {
                    if show_checks {
                        crate::log::message(&crate::output::fm_check_fail(
                            &format!("failed to rewrite {display_path}: {error}"),
                            term,
                        ));
                    }
                    final_exit = 1;
                }
            }
        }
    }

    if let Some(result) = deferred_summary {
        if stream_verbosity != Verbosity::Silent {
            eprintln!();
        }
        emit_composition_summary(
            &result.summary,
            &result.details,
            profile,
            env_context,
            stream_verbosity,
            detail_requested,
            dispatch_context,
            None,
            true,
        );
    } else {
        emit_minimal_composition_summary(
            provider,
            final_exit,
            profile,
            env_context,
            dispatch_context,
        );
    }

    Ok(final_exit)
}

fn report_interruption(
    display_path: &std::path::Display<'_>,
    captured_body: &str,
    term: &Terminal,
) {
    if captured_body.is_empty() {
        crate::log::message(&crate::output::fm_check_fail(
            &format!(
                "<b>User interrupted the agent with CTRL+C; the body of \
                 <blue-500>{display_path}</blue-500> is empty so it appears \
                 no work was accomplished.</b>"
            ),
            term,
        ));
    } else {
        crate::log::message(&crate::output::fm_check_fail(
            &format!(
                "<b>User interrupted the agent with CTRL+C; the body of \
                 <blue-500>{display_path}</blue-500> has been at least \
                 partially filled:</b>"
            ),
            term,
        ));
        eprintln!();
        for line in captured_body.lines() {
            eprintln!("  {line}");
        }
    }
}

// -- Post-processing: Darkmatter cleanup ----------------------------------

/// Run Darkmatter's cleanup pass over a written inline composition file.
///
/// Reads the file, applies `cleanup_content` to the body (preserving
/// frontmatter), and writes back only if the content changed.
///
/// Returns `Ok(true)` when the file was updated, `Ok(false)` when no
/// changes were needed.
pub(crate) fn cleanup_inline_output(path: &Path) -> Result<bool> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| eyre!("failed to read {}: {e}", path.display()))?;

    // Split frontmatter from body so cleanup operates only on the body,
    // preserving frontmatter (including YAML block scalars) byte-for-byte.
    let (frontmatter_prefix, body) = split_frontmatter_and_body(&text);

    let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(body);

    if cleaned_body == body {
        return Ok(false);
    }

    let mut output = String::with_capacity(frontmatter_prefix.len() + cleaned_body.len());
    output.push_str(frontmatter_prefix);
    output.push_str(&cleaned_body);

    std::fs::write(path, output.as_bytes())
        .map_err(|e| eyre!("failed to write cleaned output to {}: {e}", path.display()))?;

    Ok(true)
}

/// Split text into a frontmatter prefix (including closing delimiter) and the body.
///
/// If the text starts with `---\n`, scans for the closing `---\n` and returns
/// everything up to and including that line as the prefix. Otherwise returns
/// an empty prefix and the full text as body.
pub(crate) fn split_frontmatter_and_body(text: &str) -> (&str, &str) {
    let mut lines = text.split_inclusive('\n');
    let first = match lines.next() {
        Some(l) => l,
        None => return ("", text),
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return ("", text);
    }

    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return (&text[..offset], &text[offset..]);
        }
    }

    // No closing delimiter — treat entire text as body
    ("", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_basic() {
        let text = "---\ntitle: Test\n---\n# Body\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "---\ntitle: Test\n---\n");
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn split_frontmatter_block_scalar() {
        let text = concat!(
            "---\n",
            "prompt: |-\n",
            "    First line\n",
            "\n",
            "    - bullet\n",
            "last_updated: 2026-03-18\n",
            "---\n",
            "# Body\n",
        );
        let (prefix, body) = split_frontmatter_and_body(text);
        assert!(prefix.ends_with("---\n"));
        assert!(prefix.contains("prompt: |-"));
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn split_frontmatter_no_frontmatter() {
        let text = "# Just a heading\n\nContent\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "");
        assert_eq!(body, text);
    }

    #[test]
    fn split_frontmatter_unclosed() {
        let text = "---\ntitle: Test\nNo closing\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "");
        assert_eq!(body, text);
    }

    #[test]
    fn cleanup_preserves_frontmatter_block_scalar() {
        // Reproduces the bug: cleanup_content on full text corrupts YAML
        // block scalar indentation. The fix splits frontmatter from body
        // so cleanup only operates on the body.
        let frontmatter = concat!(
            "---\n",
            "prompt: |-\n",
            "    First line of prompt\n",
            "\n",
            "    - bullet one\n",
            "    - bullet two\n",
            "\n",
            "    Final paragraph\n",
            "last_updated: 2026-03-18\n",
            "---\n",
        );
        let body = "# Body\n\nSome content\n";
        let text = format!("{frontmatter}{body}");

        let (prefix, body_part) = split_frontmatter_and_body(&text);

        // Frontmatter must be preserved byte-for-byte
        assert_eq!(prefix, frontmatter);

        // Cleaning only the body should not corrupt frontmatter
        let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(body_part);
        let result = format!("{prefix}{cleaned_body}");

        // The frontmatter portion must remain unchanged
        assert!(result.starts_with(frontmatter));
    }
}
