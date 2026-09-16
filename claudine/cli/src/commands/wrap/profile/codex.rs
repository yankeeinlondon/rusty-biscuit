use claudine::error::ClaudineError;
use claudine::invocation_context::{EnvBaseline, HomeBaseline};
use claudine::provider::{OverlayResourceClass, Provider};
use claudine::provider_overlay::{
    OverlayMaterialization, OverlayPlan, OverlayPlanner, OverlayReason, OverlayReasons,
};
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail, eyre};
use std::path::{Path, PathBuf};

use super::{PromptDelivery, WrapperProfile, has_flag};
use std::io::Write;

pub(crate) struct CodexWrapper;

/// The `CODEX_SQLITE_HOME` a Codex overlay launch of this invocation receives.
///
/// Read off a Codex plan rather than re-derived, so the SQLite-state contract
/// tests pin exactly the value the overlay pins.
#[cfg(test)]
pub(crate) fn launch_sqlite_home(home: &HomeBaseline, env: &EnvBaseline) -> Result<PathBuf> {
    let mut plan = OverlayPlanner::new(home, env)
        .plan(
            Provider::Codex,
            OverlayReasons::single(OverlayReason::RepoPrompt),
        )
        .map_err(ClaudineError::from)?;
    CodexWrapper.overlay_strategy(&mut plan, env)?;
    plan.env_patch()
        .into_iter()
        .find(|(name, _)| name == "CODEX_SQLITE_HOME")
        .map(|(_, value)| PathBuf::from(value))
        .ok_or_else(|| eyre!("the Codex overlay plan pinned no SQLite home"))
}

/// The directory Codex keeps its SQLite state in, resolved from the launch
/// baseline.
///
/// Codex's configured `sqlite_home` still wins when it reads its own
/// configuration; this supplies the environment fallback. `source_root` is the
/// plan's pre-overlay Codex directory, which already folds an explicit ambient
/// `CODEX_HOME` and the `~/.codex` default, so this only has to prefer an
/// explicit `CODEX_SQLITE_HOME` over it.
fn codex_sqlite_home(env: &EnvBaseline, source_root: &Path) -> Result<PathBuf> {
    let path = env
        .get("CODEX_SQLITE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| source_root.to_path_buf());

    if !path.is_absolute() {
        bail!(
            "Codex SQLite home '{}' must be an absolute path",
            biscuit_file::to_portable_string(&path)
        );
    }
    Ok(path)
}

impl WrapperProfile for CodexWrapper {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn apply_entrypoint(&self, args: &mut Vec<String>, non_interactive: bool) {
        // The typed entrypoint catalog records `exec` but not Codex's native
        // `e` alias. Keep this override so callers that already supplied
        // `e` are not rewritten to `exec e ...`.
        if !non_interactive {
            return;
        }
        let entrypoint = "exec";
        let aliases: &[&str] = &["e"];
        if !args
            .first()
            .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
        {
            args.insert(0, entrypoint.to_string());
        }
    }

    fn apply_system_prompt(
        &self,
        prompt: &PreparedSystemPrompt,
        interactive: bool,
        _cwd: &Path,
        scoped_tmp: &Path,
    ) -> Result<crate::commands::wrap::system_prompt::SystemPromptApplication> {
        crate::commands::wrap::system_prompt::apply_system_prompt_via_spec(
            self.system_prompt_spec(),
            prompt.mode,
            interactive,
            &prompt.composed_markdown,
            None,
            scoped_tmp,
        )
    }

    fn overlay_strategy(&self, plan: &mut OverlayPlan, env: &EnvBaseline) -> Result<()> {
        let (Some(source_root), Some(visible_root)) =
            (plan.source_root(), plan.provider_visible_root())
        else {
            return Ok(());
        };
        let (source_root, visible_root) = (source_root.to_path_buf(), visible_root.to_path_buf());

        // Codex reads custom prompts from `$CODEX_HOME/prompts`, and the
        // repository's own prompts are merged over the user's. That makes the
        // directory real content rather than a link to the user's tree.
        plan.materialize(
            OverlayMaterialization::directory(
                source_root.join("prompts"),
                visible_root.join("prompts"),
                OverlayResourceClass::Config,
            )
            .repo_scoped(),
        );

        // Live SQLite state stays where it was: Codex's own state selector
        // points at the pre-overlay directory, so no database, journal, or
        // sidecar is mirrored into the overlay.
        plan.pin_external_state("CODEX_SQLITE_HOME", codex_sqlite_home(env, &source_root)?);

        Ok(())
    }

    fn apply_sandbox(&self, args: &mut Vec<String>) -> Option<String> {
        if !has_flag(args, "--sandbox") {
            args.push("--sandbox".to_string());
        }
        None
    }

    fn prompt_delivery(
        &self,
        args: &[String],
        prompt: &str,
        non_interactive: bool,
    ) -> Result<PromptDelivery> {
        if non_interactive {
            // In non-interactive mode, deliver via stdin to avoid ENAMETOOLONG
            // errors when prompt-file content exceeds OS argument length limits.
            // Codex exec reads from stdin when no positional prompt is provided.
            Ok(PromptDelivery::Stdin(prompt.to_string()))
        } else if prompt.starts_with('-') {
            // A prompt beginning with `-` (e.g. a Markdown bullet) is otherwise
            // parsed by Codex's clap as an option ("unexpected argument '- '").
            // Append it after a `--` end-of-options marker, past every flag, so
            // it is taken as the positional PROMPT.
            Ok(PromptDelivery::AppendArgs(vec![
                "--".to_string(),
                prompt.to_string(),
            ]))
        } else {
            // Interactive: insert as positional after "exec"
            let insert_at = if args.first().is_some_and(|f| f == "exec" || f == "e") {
                1
            } else {
                0
            };
            Ok(PromptDelivery::InsertArgs {
                index: insert_at,
                args: vec![prompt.to_string()],
            })
        }
    }

    fn build_resume_args(&self, session_id: &str) -> Result<Vec<String>> {
        Ok(vec![
            "codex".to_string(),
            "exec".to_string(),
            "resume".to_string(),
            session_id.to_string(),
        ])
    }
}
