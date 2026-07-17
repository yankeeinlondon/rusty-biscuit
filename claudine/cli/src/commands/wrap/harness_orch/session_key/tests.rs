//! Extraction tests for [`session_compat_key`]: each launch input the harness
//! resolves must project into the facet the resume comparison reads, so a
//! canonical refresh that changes it is named.

use super::*;

use std::path::PathBuf;

use crate::commands::wrap::profile::profile_for_provider;

fn claude() -> &'static dyn WrapperProfile {
    profile_for_provider(Provider::Claude).expect("claude profile exists")
}

fn materialized(env_overrides: Vec<(String, String)>) -> MaterializedHarnessPrompt {
    MaterializedHarnessPrompt {
        live_frontmatter: MaterializedHarnessPrompt::live_cell_from(&serde_json::Value::Null),
        frontmatter: serde_json::Value::Null,
        prompt: "body".to_string(),
        env_overrides,
        selection_hints: claudine::composition::EffectiveSelectionHints::default(),
        inline_closure_plan: None,
        lifecycle: None,
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
    base_args: Vec<String>,
    base_env: HashMap<OsString, OsString>,
    env_overrides: Vec<(String, String)>,
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
            base_args: Vec::new(),
            base_env: HashMap::new(),
            env_overrides: Vec::new(),
        }
    }
}

fn key_of(inputs: &Inputs) -> SessionCompatibilityKey {
    session_compat_key(
        inputs.provider,
        claude(),
        Path::new("/usr/bin/claude"),
        &inputs.child_cwd,
        inputs.yolo,
        inputs.non_interactive,
        inputs.use_structured,
        inputs.structured_codex,
        &inputs.base_args,
        &inputs.base_env,
        &materialized(inputs.env_overrides.clone()),
    )
}

#[test]
fn identical_inputs_produce_compatible_keys() {
    let base = key_of(&Inputs::default());
    let same = key_of(&Inputs::default());
    assert!(base.is_compatible(&same));
}

#[test]
fn a_model_env_overlay_projects_the_model_facet() {
    let base = key_of(&Inputs::default());
    let with_model = key_of(&Inputs {
        env_overrides: vec![("MODEL".to_string(), "claude-sonnet".to_string())],
        ..Inputs::default()
    });
    assert_eq!(base.incompatibilities(&with_model), vec!["model".to_string()]);
}

#[test]
fn the_env_overlay_wins_over_the_base_model() {
    let mut base_env = HashMap::new();
    base_env.insert(OsString::from("MODEL"), OsString::from("base-model"));
    let overlaid = key_of(&Inputs {
        base_env: base_env.clone(),
        env_overrides: vec![("MODEL".to_string(), "override-model".to_string())],
        ..Inputs::default()
    });
    assert_eq!(overlaid.model.as_deref(), Some("override-model"));
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
        &HashMap::new(),
        &materialized(Vec::new()),
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
        base_args: vec![
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
        base_args: vec![
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

#[test]
fn a_changed_mcp_env_projects_the_mcp_facet() {
    let base = key_of(&Inputs::default());
    let mut mcp_env = HashMap::new();
    mcp_env.insert(
        OsString::from("OPENCODE_CONFIG_CONTENT"),
        OsString::from("{\"mcp\":{\"fs\":{}}}"),
    );
    let with_mcp = key_of(&Inputs {
        base_env: mcp_env,
        ..Inputs::default()
    });
    assert_eq!(
        base.incompatibilities(&with_mcp),
        vec!["MCP server set".to_string()]
    );
}
