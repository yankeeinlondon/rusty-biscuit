//! Extraction tests for [`session_compat_key`]: each launch input the harness
//! resolves must project into the facet the resume comparison reads, so a
//! canonical refresh that changes it is named — and inputs that only differ
//! because of the resume path (stripped argv, a new follow-up prompt) must not.

use super::*;

use std::path::PathBuf;

use crate::commands::wrap::profile::profile_for_provider;

fn claude() -> &'static dyn WrapperProfile {
    profile_for_provider(Provider::Claude).expect("claude profile exists")
}

/// A minimal launch bundle carrying `args`/`env`; every other field is inert for
/// the key computation, which reads only [`AttemptLaunch::env`].
fn launch_with(args: Vec<String>, env: HashMap<OsString, OsString>) -> AttemptLaunch {
    AttemptLaunch {
        args,
        env,
        stdin_seed: None,
        wire_prompt: None,
        timeout_config: Default::default(),
        step_timeout_user_configured: false,
        stall_timeout: None,
        stall_timeout_user_configured: false,
    }
}

/// A representative launch state and the knobs a test flips.
struct Inputs {
    provider: Provider,
    yolo: bool,
    non_interactive: bool,
    use_structured: bool,
    structured_codex: bool,
    child_cwd: PathBuf,
    /// Canonical (pre-resume-normalization) argv.
    canonical_args: Vec<String>,
    /// The effective child environment the provider is spawned with (already
    /// carrying any per-attempt overlay).
    child_env: HashMap<OsString, OsString>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            provider: Provider::Claude,
            yolo: false,
            non_interactive: true,
            use_structured: true,
            structured_codex: false,
            child_cwd: PathBuf::from("/repo"),
            canonical_args: Vec::new(),
            child_env: HashMap::new(),
        }
    }
}

fn key_of(inputs: &Inputs) -> SessionCompatibilityKey {
    let launch = launch_with(inputs.canonical_args.clone(), inputs.child_env.clone());
    session_compat_key(
        inputs.provider,
        claude(),
        Path::new("/usr/bin/claude"),
        &inputs.child_cwd,
        inputs.yolo,
        inputs.non_interactive,
        inputs.use_structured,
        inputs.structured_codex,
        &inputs.canonical_args,
        &launch,
    )
}

fn env_with(pairs: &[(&str, &str)]) -> HashMap<OsString, OsString> {
    pairs
        .iter()
        .map(|(k, v)| (OsString::from(k), OsString::from(v)))
        .collect()
}

#[test]
fn identical_inputs_produce_compatible_keys() {
    let base = key_of(&Inputs::default());
    let same = key_of(&Inputs::default());
    assert!(base.is_compatible(&same));
}

#[test]
fn the_model_facet_reads_the_effective_child_env() {
    let base = key_of(&Inputs::default());
    let with_model = key_of(&Inputs {
        child_env: env_with(&[("MODEL", "claude-sonnet")]),
        ..Inputs::default()
    });
    assert_eq!(with_model.model.as_deref(), Some("claude-sonnet"));
    assert_eq!(base.incompatibilities(&with_model), vec!["model".to_string()]);
}

#[test]
fn changing_the_working_directory_projects_the_cwd_facet() {
    let base = key_of(&Inputs::default());
    let moved = key_of(&Inputs {
        child_cwd: PathBuf::from("/elsewhere"),
        ..Inputs::default()
    });
    assert_eq!(base.incompatibilities(&moved), vec!["workspace CWD".to_string()]);
}

#[test]
fn toggling_yolo_projects_the_permission_facet() {
    let base = key_of(&Inputs::default());
    let bypass = key_of(&Inputs {
        yolo: true,
        ..Inputs::default()
    });
    assert_eq!(base.incompatibilities(&bypass), vec!["permission mode".to_string()]);
}

#[test]
fn toggling_interactivity_projects_the_interactivity_facet() {
    let base = key_of(&Inputs::default());
    let interactive = key_of(&Inputs {
        non_interactive: false,
        ..Inputs::default()
    });
    assert_eq!(base.incompatibilities(&interactive), vec!["interactivity".to_string()]);
}

#[test]
fn toggling_structured_output_projects_the_structured_facet() {
    let base = key_of(&Inputs::default());
    let text = key_of(&Inputs {
        use_structured: false,
        ..Inputs::default()
    });
    assert_eq!(
        base.incompatibilities(&text),
        vec!["structured-output mode".to_string()]
    );
}

#[test]
fn swapping_the_provider_changes_provider_binary_and_resume_protocol() {
    let base = key_of(&Inputs::default());
    let launch = launch_with(Vec::new(), HashMap::new());
    let codex = session_compat_key(
        Provider::Codex,
        profile_for_provider(Provider::Codex).expect("codex profile"),
        Path::new("/usr/bin/codex"),
        Path::new("/repo"),
        false,
        true,
        true,
        false,
        &[],
        &launch,
    );
    let named = base.incompatibilities(&codex);
    assert!(named.contains(&"provider".to_string()));
    assert!(named.contains(&"profile/binary".to_string()));
    assert!(named.contains(&"resume protocol".to_string()));
}

#[test]
fn a_changed_system_prompt_flag_projects_the_system_prompt_facet() {
    let base = key_of(&Inputs::default());
    let with_prompt = key_of(&Inputs {
        canonical_args: vec![
            "--append-system-prompt".to_string(),
            "be terse".to_string(),
        ],
        ..Inputs::default()
    });
    assert_eq!(
        base.incompatibilities(&with_prompt),
        vec!["system prompt".to_string()]
    );

    // A different inline system-prompt content flips the facet again.
    let other = key_of(&Inputs {
        canonical_args: vec![
            "--append-system-prompt".to_string(),
            "be verbose".to_string(),
        ],
        ..Inputs::default()
    });
    assert_eq!(
        with_prompt.incompatibilities(&other),
        vec!["system prompt".to_string()]
    );
}

/// An inline `--append-system-prompt` value that happens to name a real file
/// must be hashed as the literal string delivered to the provider, NOT as that
/// file's contents — only the `*-file` variant reads the file.
#[test]
fn an_inline_system_prompt_naming_a_file_is_hashed_as_the_literal() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("prompt.txt");
    let file_content = "CONTENT-FROM-FILE";
    std::fs::write(&file, file_content).unwrap();
    let path_literal = file.display().to_string();

    // Inline flag, value = the file's path.
    let inline_names_file = key_of(&Inputs {
        canonical_args: vec!["--append-system-prompt".to_string(), path_literal.clone()],
        ..Inputs::default()
    });
    // Inline flag, value = the file's *content* as a literal string.
    let inline_names_content = key_of(&Inputs {
        canonical_args: vec![
            "--append-system-prompt".to_string(),
            file_content.to_string(),
        ],
        ..Inputs::default()
    });
    // File flag, value = the file's path — reads the content.
    let file_flag = key_of(&Inputs {
        canonical_args: vec![
            "--append-system-prompt-file".to_string(),
            path_literal.clone(),
        ],
        ..Inputs::default()
    });

    // Compare the content digest alone (the substring after `=`); the flag name
    // itself legitimately differs between the inline and `-file` variants.
    let digest = |sp: &str| sp.split_once('=').map(|(_, h)| h.to_string()).unwrap();

    // The `*-file` variant reads the file, so its digest must match an inline
    // delivery of that same content...
    assert_eq!(
        digest(&file_flag.system_prompt),
        digest(&inline_names_content.system_prompt),
        "the -file variant must hash the file's content",
    );
    // ...while the inline variant that merely *names* the file must NOT — it
    // hashes the path literal the provider actually receives.
    assert_ne!(
        digest(&inline_names_file.system_prompt),
        digest(&inline_names_content.system_prompt),
        "an inline value naming a file must be hashed as the literal, not the file",
    );
}

#[test]
fn a_changed_mcp_env_projects_the_mcp_facet() {
    let base = key_of(&Inputs::default());
    let with_mcp = key_of(&Inputs {
        child_env: env_with(&[("OPENCODE_CONFIG_CONTENT", "{\"mcp\":{\"fs\":{}}}")]),
        ..Inputs::default()
    });
    assert_eq!(
        base.incompatibilities(&with_mcp),
        vec!["MCP server set".to_string()]
    );
}

/// Inputs that differ only because of the resume path — a follow-up prompt
/// substituted onto argv, or resume-normalized argv the key never reads — must
/// not perturb any facet. The key is built from the canonical argv and the
/// effective env, so a compatible resume stays compatible.
#[test]
fn resume_only_differences_do_not_change_the_key() {
    let system_prompt_args = vec![
        "--append-system-prompt".to_string(),
        "stay terse".to_string(),
    ];
    let base = key_of(&Inputs {
        canonical_args: system_prompt_args.clone(),
        child_env: env_with(&[("MODEL", "claude-sonnet")]),
        ..Inputs::default()
    });

    // The resume attempt's *bundle* argv differs (resume entrypoint, dropped
    // system-prompt flag) and its stdin follow-up differs, but the canonical
    // argv and effective env are identical — so the key is unchanged.
    let resume_launch = launch_with(
        vec!["-r".to_string(), "sess-123".to_string()],
        env_with(&[("MODEL", "claude-sonnet")]),
    );
    let resume_key = session_compat_key(
        Provider::Claude,
        claude(),
        Path::new("/usr/bin/claude"),
        Path::new("/repo"),
        false,
        true,
        true,
        false,
        &system_prompt_args,
        &resume_launch,
    );
    assert!(
        base.is_compatible(&resume_key),
        "a resume that only re-points argv and swaps the prompt must stay compatible; \
         differed on {:?}",
        base.incompatibilities(&resume_key),
    );
}
