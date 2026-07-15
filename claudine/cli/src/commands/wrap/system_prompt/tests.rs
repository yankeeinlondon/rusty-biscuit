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
    assert!(
        path.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("test-prefix-")
    );
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

    assert_eq!(
        app.args,
        vec!["-c", "developer_instructions=\"Small prompt.\""]
    );
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

    assert_eq!(
        app.args,
        vec!["--append-system-prompt", "Qwen append content."]
    );
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
