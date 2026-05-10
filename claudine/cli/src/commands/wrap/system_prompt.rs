use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use claudine::composition::LaunchWorkspaceContext;
use claudine::provider::{SystemPromptDelivery, SystemPromptSpec};
use claudine::system_prompt::{
    EffectiveSystemPrompt, StandardPromptScope, SystemPromptMode, SystemPromptSource,
};
use color_eyre::eyre::Result;
use tempfile::{NamedTempFile, TempDir};

/// Artifacts that must remain alive until the child process exits.
///
/// The inner values are never read directly — they exist purely for RAII
/// cleanup (the `Drop` impl deletes the temp resource).
#[allow(dead_code)]
pub(crate) enum SystemPromptArtifact {
    TempFile(NamedTempFile),
    TempDir(TempDir),
}

impl std::fmt::Debug for SystemPromptArtifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemPromptArtifact::TempFile(tmp) => f
                .debug_tuple("TempFile")
                .field(&tmp.path())
                .finish(),
            SystemPromptArtifact::TempDir(tmp) => f
                .debug_tuple("TempDir")
                .field(&tmp.path())
                .finish(),
        }
    }
}

/// The result of applying a system prompt to a provider launch plan.
#[derive(Debug)]
pub(crate) struct SystemPromptApplication {
    /// Additional CLI args to append.
    pub args: Vec<String>,
    /// Additional or overridden env vars.
    pub env: Vec<(OsString, OsString)>,
    /// Temp resources that must outlive the child process.
    pub artifacts: Vec<SystemPromptArtifact>,
    /// Non-fatal warnings to display.
    pub warnings: Vec<String>,
}

impl SystemPromptApplication {
    pub fn empty() -> Self {
        Self {
            args: vec![],
            env: vec![],
            artifacts: vec![],
            warnings: vec![],
        }
    }
}

/// Human-readable description of where a system prompt came from.
pub(crate) fn describe_source(source: &SystemPromptSource) -> String {
    match source {
        SystemPromptSource::StandardDiscovered { path, scope } => {
            let scope_label = match scope {
                StandardPromptScope::Package => "package",
                StandardPromptScope::PackageArea => "package-area",
                StandardPromptScope::Repo => "repo",
                StandardPromptScope::User => "user",
                StandardPromptScope::CurrentDirectory => "cwd",
            };
            format!("{} ({})", path.display(), scope_label)
        }
        SystemPromptSource::ExplicitFile { path, mode } => {
            let mode_label = match mode {
                SystemPromptMode::Append => "append",
                SystemPromptMode::Replace => "replace",
            };
            format!("{} (explicit {})", path.display(), mode_label)
        }
        SystemPromptSource::NonInteractiveFile { path, scope } => {
            let scope_label = match scope {
                StandardPromptScope::Package => "package",
                StandardPromptScope::PackageArea => "package-area",
                StandardPromptScope::Repo => "repo",
                StandardPromptScope::User => "user",
                StandardPromptScope::CurrentDirectory => "cwd",
            };
            format!("{} (non-interactive {})", path.display(), scope_label)
        }
        SystemPromptSource::BuiltInNonInteractive => {
            "built-in non-interactive fallback".to_string()
        }
    }
}

/// Format the effective system prompt for dry-run output.
pub(crate) fn describe_effective(effective: &EffectiveSystemPrompt) -> Option<Vec<String>> {
    match effective {
        EffectiveSystemPrompt::None => None,
        EffectiveSystemPrompt::Disabled { source } => Some(vec![
            format!("source: {}", describe_source(source)),
            "status: disabled (empty body)".to_string(),
        ]),
        EffectiveSystemPrompt::Ready(prepared) => {
            let mode_label = match prepared.mode {
                SystemPromptMode::Append => "append",
                SystemPromptMode::Replace => "replace",
            };
            Some(vec![
                format!("source: {}", describe_source(&prepared.source)),
                format!("mode: {}", mode_label),
                prepared
                    .non_interactive_appendix
                    .as_ref()
                    .map(|appendix| {
                        format!(
                            "non-interactive appendix: {}",
                            describe_source(&appendix.source)
                        )
                    })
                    .unwrap_or_else(|| "non-interactive appendix: none".to_string()),
                format!(
                    "composed length: {} chars",
                    prepared.composed_markdown.len()
                ),
            ])
        }
    }
}

/// Resolve the directory where claudine should drop transient
/// per-invocation overlay files (system prompts, instruction files,
/// etc.). Always inside the user's trust boundary.
pub(crate) fn scoped_tmp_dir(launch_workspace: &LaunchWorkspaceContext) -> PathBuf {
    let base = launch_workspace
        .repo_root
        .clone()
        .unwrap_or_else(|| launch_workspace.launch_cwd.clone());

    let scoped = if launch_workspace.repo_root.is_some() {
        base.join(".claudine").join("tmp")
    } else {
        base.join(".claudine-tmp")
    };
    let _ = std::fs::create_dir_all(&scoped);
    scoped
}

/// Create a named temp file inside the given base directory.
pub(crate) fn scoped_tempfile(base: &Path, prefix: &str) -> std::io::Result<NamedTempFile> {
    tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".md")
        .tempfile_in(base)
}

/// Best-effort `.gitignore` augmentation.
///
/// When `repo_root.join(".gitignore")` exists and doesn't already contain
/// `.claudine/tmp/`, append the line.  Skip silently on failure (this is a
/// courtesy, not a contract).
pub(crate) fn maybe_gitignore_claudine_tmp(repo_root: &Path) {
    let gitignore = repo_root.join(".gitignore");
    if !gitignore.is_file() {
        return;
    }

    let Ok(contents) = std::fs::read_to_string(&gitignore) else {
        return;
    };

    let needle = ".claudine/tmp/";
    if contents.lines().any(|l| l.trim() == needle) {
        return;
    }

    let mut with_trailing = contents;
    if !with_trailing.ends_with('\n') {
        with_trailing.push('\n');
    }
    with_trailing.push_str(needle);
    with_trailing.push('\n');

    let _ = std::fs::write(&gitignore, with_trailing);
}

/// Hard cap for Codex append inline argv size (64 KiB).
const CODEX_INLINE_LIMIT_BYTES: usize = 64 * 1024;

/// Spec-driven system-prompt dispatcher.
///
/// Reads [`SystemPromptSpec`] and applies the correct delivery mechanism
/// per provider.  No HOME redirect for Gemini, Codex, or Qwen — those
/// providers have native non-HOME mechanisms.
pub(crate) fn apply_system_prompt_via_spec(
    spec: &SystemPromptSpec,
    mode: SystemPromptMode,
    interactive: bool,
    content: &str,
    real_provider_dir: Option<&Path>,
    scoped_tmp: &Path,
) -> Result<SystemPromptApplication> {
    use color_eyre::eyre::bail;

    let delivery_by_mode = match mode {
        SystemPromptMode::Append => &spec.append,
        SystemPromptMode::Replace => &spec.replace,
    };
    let delivery = if interactive {
        delivery_by_mode.interactive
    } else {
        delivery_by_mode.non_interactive
    };

    let mut app = SystemPromptApplication::empty();

    match delivery {
        SystemPromptDelivery::InlineFlag { flag } => {
            app.args.push(flag.to_string());
            app.args.push(content.to_string());
        }
        SystemPromptDelivery::FileFlag { flag } => {
            let mut tmp = scoped_tempfile(scoped_tmp, "system-prompt-")?;
            tmp.write_all(content.as_bytes())?;
            app.args.push(flag.to_string());
            app.args.push(tmp.path().display().to_string());
            app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
        }
        SystemPromptDelivery::EnvVarFile { env_var } => {
            let merged = if mode == SystemPromptMode::Append {
                if let Some(provider_dir) = real_provider_dir {
                    let existing = provider_dir.join("GEMINI.md");
                    if existing.is_file() {
                        let existing_content = std::fs::read_to_string(&existing)?;
                        format!("{}\n\n{}", existing_content.trim_end(), content)
                    } else {
                        content.to_string()
                    }
                } else {
                    content.to_string()
                }
            } else {
                content.to_string()
            };

            let mut tmp = scoped_tempfile(scoped_tmp, "system-prompt-")?;
            tmp.write_all(merged.as_bytes())?;
            app.env.push((
                OsString::from(env_var),
                OsString::from(tmp.path().display().to_string()),
            ));
            app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
        }
        SystemPromptDelivery::ConfigKeyInline { flag, key } => {
            if content.len() > CODEX_INLINE_LIMIT_BYTES {
                bail!(
                    "codex append-system-prompt exceeds {}KB argv limit ({}B); \
                     shrink the system prompt or use --replace-system-prompt instead",
                    CODEX_INLINE_LIMIT_BYTES / 1024,
                    content.len()
                );
            }
            let escaped = content.replace('"', "\\\"");
            app.args.push(flag.to_string());
            app.args.push(format!("{}=\"{}\"", key, escaped));
        }
        SystemPromptDelivery::ConfigKeyFile { flag, key } => {
            let mut tmp = scoped_tempfile(scoped_tmp, "system-prompt-")?;
            tmp.write_all(content.as_bytes())?;
            app.args.push(flag.to_string());
            app.args.push(format!("{}={}", key, tmp.path().display()));
            app.artifacts.push(SystemPromptArtifact::TempFile(tmp));
        }
        SystemPromptDelivery::ShadowHomeFile { .. } => {
            app.warnings.push(
                "ShadowHomeFile delivery is no longer implemented; \
                 use a native provider mechanism instead"
                    .to_string(),
            );
        }
        SystemPromptDelivery::Custom(_tag) => {
            app.warnings.push(
                "Custom system-prompt delivery is not yet implemented for this provider"
                    .to_string(),
            );
        }
        SystemPromptDelivery::Unsupported => {
            app.warnings.push(format!(
                "{} system prompt is not supported for this provider",
                match mode {
                    SystemPromptMode::Append => "append",
                    SystemPromptMode::Replace => "replace",
                }
            ));
        }
    }

    Ok(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_application_empty() {
        let app = SystemPromptApplication::empty();
        assert!(app.args.is_empty());
        assert!(app.env.is_empty());
        assert!(app.artifacts.is_empty());
        assert!(app.warnings.is_empty());
    }

    #[test]
    fn scoped_tmp_dir_inside_repo() {
        let repo = tempfile::tempdir().unwrap();
        let ctx = LaunchWorkspaceContext {
            launch_cwd: repo.path().join("sub"),
            repo_root: Some(repo.path().to_path_buf()),
            child_cwd: repo.path().join("sub"),
            package_context: None,
            warnings: vec![],
        };
        let dir = scoped_tmp_dir(&ctx);
        assert_eq!(dir, repo.path().join(".claudine").join("tmp"));
        assert!(dir.exists());
    }

    #[test]
    fn scoped_tmp_dir_outside_repo() {
        let cwd = tempfile::tempdir().unwrap();
        let ctx = LaunchWorkspaceContext {
            launch_cwd: cwd.path().to_path_buf(),
            repo_root: None,
            child_cwd: cwd.path().to_path_buf(),
            package_context: None,
            warnings: vec![],
        };
        let dir = scoped_tmp_dir(&ctx);
        assert_eq!(dir, cwd.path().join(".claudine-tmp"));
        assert!(dir.exists());
    }

    #[test]
    fn scoped_tempfile_creates_file_in_base() {
        let base = tempfile::tempdir().unwrap();
        let file = scoped_tempfile(base.path(), "test-prefix-").unwrap();
        let path = file.path();
        assert!(path.starts_with(base.path()));
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("test-prefix-"));
        assert!(path.extension().unwrap() == "md");
        assert!(path.exists());
    }

    #[test]
    fn maybe_gitignore_appends_once() {
        let repo = tempfile::tempdir().unwrap();
        let gitignore = repo.path().join(".gitignore");
        std::fs::write(&gitignore, "target/\n").unwrap();

        maybe_gitignore_claudine_tmp(repo.path());
        let contents = std::fs::read_to_string(&gitignore).unwrap();
        assert!(contents.contains(".claudine/tmp/"));
        assert_eq!(contents.matches(".claudine/tmp/").count(), 1);

        // Idempotent: second call must not append a duplicate.
        maybe_gitignore_claudine_tmp(repo.path());
        let contents2 = std::fs::read_to_string(&gitignore).unwrap();
        assert_eq!(contents2.matches(".claudine/tmp/").count(), 1);
    }

    #[test]
    fn maybe_gitignore_noop_when_missing() {
        let repo = tempfile::tempdir().unwrap();
        // No .gitignore present — should not panic or create one.
        maybe_gitignore_claudine_tmp(repo.path());
        assert!(!repo.path().join(".gitignore").exists());
    }

    #[test]
    fn maybe_gitignore_noop_when_already_present() {
        let repo = tempfile::tempdir().unwrap();
        let gitignore = repo.path().join(".gitignore");
        std::fs::write(&gitignore, ".claudine/tmp/\n").unwrap();

        maybe_gitignore_claudine_tmp(repo.path());
        let contents = std::fs::read_to_string(&gitignore).unwrap();
        assert_eq!(contents, ".claudine/tmp/\n");
    }

    // -----------------------------------------------------------------------
    // Phase 4 — spec-driven dispatcher tests
    // -----------------------------------------------------------------------

    #[test]
    fn gemini_append_composes_existing_gemini_md() {
        let scoped_tmp = tempfile::tempdir().unwrap();
        let fake_home = tempfile::tempdir().unwrap();
        let gemini_dir = fake_home.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        std::fs::write(gemini_dir.join("GEMINI.md"), "Existing Gemini content.").unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            "Overlay content.",
            Some(&gemini_dir),
            scoped_tmp.path(),
        )
        .unwrap();

        assert_eq!(app.args.len(), 0);
        assert_eq!(app.env.len(), 1);
        assert_eq!(app.env[0].0, "GEMINI_SYSTEM_MD");
        assert_eq!(app.artifacts.len(), 1);

        // Verify the merged content
        let tmp_path = PathBuf::from(app.env[0].1.to_str().unwrap());
        let merged = std::fs::read_to_string(&tmp_path).unwrap();
        assert_eq!(merged, "Existing Gemini content.\n\nOverlay content.");
    }

    #[test]
    fn gemini_replace_writes_only_overlay() {
        let scoped_tmp = tempfile::tempdir().unwrap();
        let fake_home = tempfile::tempdir().unwrap();
        let gemini_dir = fake_home.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        std::fs::write(gemini_dir.join("GEMINI.md"), "Existing Gemini content.").unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Replace,
            false,
            "Replacement content.",
            Some(&gemini_dir),
            scoped_tmp.path(),
        )
        .unwrap();

        assert_eq!(app.args.len(), 0);
        assert_eq!(app.env.len(), 1);
        assert_eq!(app.env[0].0, "GEMINI_SYSTEM_MD");

        let tmp_path = PathBuf::from(app.env[0].1.to_str().unwrap());
        let content = std::fs::read_to_string(&tmp_path).unwrap();
        assert_eq!(content, "Replacement content.");
    }

    #[test]
    fn codex_append_exceeds_64kb_limit() {
        let scoped_tmp = tempfile::tempdir().unwrap();
        let huge_content = "x".repeat(CODEX_INLINE_LIMIT_BYTES + 1);

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
            },
            memory_files: &[],
        };

        let result = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            &huge_content,
            None,
            scoped_tmp.path(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("64KB argv limit"));
    }

    #[test]
    fn codex_append_small_content_pushes_config_key_inline() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            "Small prompt.",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert_eq!(app.args, vec!["-c", "developer_instructions=\"Small prompt.\""]);
        assert!(app.env.is_empty());
        assert!(app.artifacts.is_empty());
    }

    #[test]
    fn codex_replace_writes_scoped_file_and_pushes_config_key_file() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Replace,
            false,
            "Replacement prompt.",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert_eq!(app.args.len(), 2);
        assert_eq!(app.args[0], "-c");
        assert!(app.args[1].starts_with("model_instructions_file="));
        assert!(app.env.is_empty());
        assert_eq!(app.artifacts.len(), 1);
    }

    #[test]
    fn qwen_append_pushes_inline_flag() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--append-system-prompt",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--append-system-prompt",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--system-prompt",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--system-prompt",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            "Qwen append content.",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert_eq!(app.args, vec!["--append-system-prompt", "Qwen append content."]);
        assert!(app.env.is_empty());
        assert!(app.artifacts.is_empty());
    }

    #[test]
    fn qwen_replace_pushes_inline_flag() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--append-system-prompt",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--append-system-prompt",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--system-prompt",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--system-prompt",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Replace,
            false,
            "Qwen replace content.",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert_eq!(app.args, vec!["--system-prompt", "Qwen replace content."]);
        assert!(app.env.is_empty());
        assert!(app.artifacts.is_empty());
    }

    #[test]
    fn gemini_no_home_env_set() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::EnvVarFile {
                    env_var: "GEMINI_SYSTEM_MD",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            "test",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert!(
            !app.env.iter().any(|(k, _)| k == "HOME"),
            "Gemini append must not set HOME"
        );
    }

    #[test]
    fn codex_no_home_env_set() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyInline {
                    flag: "-c",
                    key: "developer_instructions",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::ConfigKeyFile {
                    flag: "-c",
                    key: "model_instructions_file",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            "test",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert!(
            !app.env.iter().any(|(k, _)| k == "HOME"),
            "Codex append must not set HOME"
        );
    }

    #[test]
    fn qwen_no_home_env_set() {
        let scoped_tmp = tempfile::tempdir().unwrap();

        let spec = SystemPromptSpec {
            append: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--append-system-prompt",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--append-system-prompt",
                },
            },
            replace: claudine::provider::SystemPromptDeliveryByMode {
                interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--system-prompt",
                },
                non_interactive: claudine::provider::SystemPromptDelivery::InlineFlag {
                    flag: "--system-prompt",
                },
            },
            memory_files: &[],
        };

        let app = apply_system_prompt_via_spec(
            &spec,
            SystemPromptMode::Append,
            false,
            "test",
            None,
            scoped_tmp.path(),
        )
        .unwrap();

        assert!(
            !app.env.iter().any(|(k, _)| k == "HOME"),
            "Qwen append must not set HOME"
        );
    }
}
