use std::borrow::Cow;
use std::sync::LazyLock;

use darkmatter::markdown::compose::expression::{
    Expr, ExpressionFinder, ParseMode, Parser, evaluate, scalar_string,
};
use regex::Regex;
use tracing::warn;

use crate::dispatch::expression::EventMetaExpressionLookup;
use crate::events::EventMeta;

/// Legacy single-brace placeholder key validation.
static LEGACY_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_.]*$").expect("legacy key regex is valid"));

/// Category of template variables for display grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableCategory {
    /// Core event fields (provider, event, timestamp, etc.)
    Event,
    /// Context: Operating system (detected at runtime)
    ContextOs,
    /// Context: Hardware (detected at runtime)
    ContextHardware,
    /// Context: Git repository state
    ContextGit,
    /// Context: Project/repo structure
    ContextRepo,
}

impl VariableCategory {
    /// Human-readable label for the category.
    pub fn label(&self) -> &'static str {
        match self {
            VariableCategory::Event => "Event Fields",
            VariableCategory::ContextOs => "OS",
            VariableCategory::ContextHardware => "Hardware",
            VariableCategory::ContextGit => "Git",
            VariableCategory::ContextRepo => "Project",
        }
    }
}

/// A template variable that can be interpolated in speak/report templates.
///
/// This enum is the **single source of truth** for all supported template variables.
/// Add new variables here and they will automatically appear in:
/// - Template interpolation (`interpolate()`)
/// - CLI documentation (`claudine hooks --variables`)
/// - Skill documentation (via iteration)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateVariable {
    // Event fields
    Provider,
    Event,
    Timestamp,
    SessionId,
    Cwd,
    ToolName,
    Error,
    Prompt,
    AgentType,
    NotificationType,

    // Environment: OS
    EnvOs,
    EnvOsType,
    EnvOsVersion,
    EnvHostname,

    // Environment: Hardware
    EnvArch,
    EnvCpu,
    EnvCores,

    // Environment: Git
    EnvBranch,
    EnvIsDirty,
    EnvHeadSha,
    EnvHeadMessage,
    EnvRemote,
    EnvHosting,
    EnvRepoName,
    EnvRepoOrg,

    // Environment: Repo
    EnvLanguage,
    EnvIsMonorepo,
    EnvMonorepoTool,
}

impl TemplateVariable {
    /// The placeholder key used in templates (e.g., "provider", "git.branch").
    pub fn key(&self) -> &'static str {
        match self {
            // Event fields
            TemplateVariable::Provider => "provider",
            TemplateVariable::Event => "event",
            TemplateVariable::Timestamp => "timestamp",
            TemplateVariable::SessionId => "session_id",
            TemplateVariable::Cwd => "cwd",
            TemplateVariable::ToolName => "tool_name",
            TemplateVariable::Error => "error",
            TemplateVariable::Prompt => "prompt",
            TemplateVariable::AgentType => "agent_type",
            TemplateVariable::NotificationType => "notification_type",

            // OS
            TemplateVariable::EnvOs => "os.name",
            TemplateVariable::EnvOsType => "os.type",
            TemplateVariable::EnvOsVersion => "os.version",
            TemplateVariable::EnvHostname => "os.hostname",

            // Hardware
            TemplateVariable::EnvArch => "hardware.arch",
            TemplateVariable::EnvCpu => "hardware.cpu",
            TemplateVariable::EnvCores => "hardware.cores",

            // Git
            TemplateVariable::EnvBranch => "git.branch",
            TemplateVariable::EnvIsDirty => "git.is_dirty",
            TemplateVariable::EnvHeadSha => "git.head_sha",
            TemplateVariable::EnvHeadMessage => "git.head_message",
            TemplateVariable::EnvRemote => "git.remote",
            TemplateVariable::EnvHosting => "git.hosting",
            TemplateVariable::EnvRepoName => "git.repo_name",
            TemplateVariable::EnvRepoOrg => "git.repo_org",

            // Project
            TemplateVariable::EnvLanguage => "project.language",
            TemplateVariable::EnvIsMonorepo => "project.is_monorepo",
            TemplateVariable::EnvMonorepoTool => "project.monorepo_tool",
        }
    }

    /// Human-readable description for documentation.
    pub fn description(&self) -> &'static str {
        match self {
            // Event fields
            TemplateVariable::Provider => "Provider slug (claude, codex, gemini, etc.)",
            TemplateVariable::Event => "Event name (tool_error, turn_complete, etc.)",
            TemplateVariable::Timestamp => "RFC3339 timestamp",
            TemplateVariable::SessionId => "Session ID",
            TemplateVariable::Cwd => "Current working directory",
            TemplateVariable::ToolName => "Tool name (Bash, Read, etc.)",
            TemplateVariable::Error => "Error message",
            TemplateVariable::Prompt => "User prompt text",
            TemplateVariable::AgentType => "Agent/subagent type",
            TemplateVariable::NotificationType => "Notification type",

            // Environment: OS
            TemplateVariable::EnvOs => "OS name (macOS, Ubuntu, Windows)",
            TemplateVariable::EnvOsType => "OS type slug (macos, linux, windows)",
            TemplateVariable::EnvOsVersion => "OS version string",
            TemplateVariable::EnvHostname => "Machine hostname",

            // Environment: Hardware
            TemplateVariable::EnvArch => "CPU architecture (aarch64, x86_64)",
            TemplateVariable::EnvCpu => "CPU model name",
            TemplateVariable::EnvCores => "Logical CPU core count",

            // Environment: Git
            TemplateVariable::EnvBranch => "Current git branch",
            TemplateVariable::EnvIsDirty => "Git dirty state (true/false)",
            TemplateVariable::EnvHeadSha => "HEAD commit SHA",
            TemplateVariable::EnvHeadMessage => "HEAD commit message",
            TemplateVariable::EnvRemote => "Git remote name (usually origin)",
            TemplateVariable::EnvHosting => "Git hosting provider (github, gitlab, bitbucket)",
            TemplateVariable::EnvRepoName => "Repository name (e.g., cargo)",
            TemplateVariable::EnvRepoOrg => "Organization/owner name (e.g., rust-lang)",

            // Environment: Repo
            TemplateVariable::EnvLanguage => "Primary project language",
            TemplateVariable::EnvIsMonorepo => "Monorepo detection (true/false)",
            TemplateVariable::EnvMonorepoTool => "Monorepo tool (cargo_workspace, pnpm, nx)",
        }
    }

    /// Which events this variable is available for.
    pub fn availability(&self) -> &'static str {
        match self {
            // Event fields with limited availability
            TemplateVariable::ToolName => "tool events",
            TemplateVariable::Error => "error events",
            TemplateVariable::Prompt => "before_prompt",
            TemplateVariable::AgentType => "subagent events",
            TemplateVariable::NotificationType => "notification",

            // Everything else is available for all events
            _ => "all events",
        }
    }

    /// Category for display grouping.
    pub fn category(&self) -> VariableCategory {
        match self {
            TemplateVariable::Provider
            | TemplateVariable::Event
            | TemplateVariable::Timestamp
            | TemplateVariable::SessionId
            | TemplateVariable::Cwd
            | TemplateVariable::ToolName
            | TemplateVariable::Error
            | TemplateVariable::Prompt
            | TemplateVariable::AgentType
            | TemplateVariable::NotificationType => VariableCategory::Event,

            TemplateVariable::EnvOs
            | TemplateVariable::EnvOsType
            | TemplateVariable::EnvOsVersion
            | TemplateVariable::EnvHostname => VariableCategory::ContextOs,

            TemplateVariable::EnvArch | TemplateVariable::EnvCpu | TemplateVariable::EnvCores => {
                VariableCategory::ContextHardware
            }

            TemplateVariable::EnvBranch
            | TemplateVariable::EnvIsDirty
            | TemplateVariable::EnvHeadSha
            | TemplateVariable::EnvHeadMessage
            | TemplateVariable::EnvRemote
            | TemplateVariable::EnvHosting
            | TemplateVariable::EnvRepoName
            | TemplateVariable::EnvRepoOrg => VariableCategory::ContextGit,

            TemplateVariable::EnvLanguage
            | TemplateVariable::EnvIsMonorepo
            | TemplateVariable::EnvMonorepoTool => VariableCategory::ContextRepo,
        }
    }

    /// Returns the placeholder with Handlebars braces (e.g., "{{provider}}").
    pub fn placeholder(&self) -> String {
        format!("{{{{{}}}}}", self.key())
    }

    /// All template variables in definition order.
    pub fn all() -> &'static [TemplateVariable] {
        &[
            // Event fields
            TemplateVariable::Provider,
            TemplateVariable::Event,
            TemplateVariable::Timestamp,
            TemplateVariable::SessionId,
            TemplateVariable::Cwd,
            TemplateVariable::ToolName,
            TemplateVariable::Error,
            TemplateVariable::Prompt,
            TemplateVariable::AgentType,
            TemplateVariable::NotificationType,
            // Environment: OS
            TemplateVariable::EnvOs,
            TemplateVariable::EnvOsType,
            TemplateVariable::EnvOsVersion,
            TemplateVariable::EnvHostname,
            // Environment: Hardware
            TemplateVariable::EnvArch,
            TemplateVariable::EnvCpu,
            TemplateVariable::EnvCores,
            // Environment: Git
            TemplateVariable::EnvBranch,
            TemplateVariable::EnvIsDirty,
            TemplateVariable::EnvHeadSha,
            TemplateVariable::EnvHeadMessage,
            TemplateVariable::EnvRemote,
            TemplateVariable::EnvHosting,
            TemplateVariable::EnvRepoName,
            TemplateVariable::EnvRepoOrg,
            // Environment: Repo
            TemplateVariable::EnvLanguage,
            TemplateVariable::EnvIsMonorepo,
            TemplateVariable::EnvMonorepoTool,
        ]
    }

    /// Event variables only (non-environment).
    pub fn event_variables() -> impl Iterator<Item = &'static TemplateVariable> {
        Self::all()
            .iter()
            .filter(|v| v.category() == VariableCategory::Event)
    }

    /// Context variables only (`os.*`, `hardware.*`, `git.*`, `project.*`).
    ///
    /// These are auto-detected context values about the OS, hardware, git
    /// state, and project structure. Shell environment variables are resolved
    /// separately via `{{env.VAR_NAME}}`.
    pub fn context_variables() -> impl Iterator<Item = &'static TemplateVariable> {
        Self::all()
            .iter()
            .filter(|v| v.category() != VariableCategory::Event)
    }

    /// Resolve this variable's value from event metadata.
    pub fn resolve<'a>(&self, meta: &'a EventMeta) -> Cow<'a, str> {
        match self {
            // Event fields
            TemplateVariable::Provider => Cow::Borrowed(meta.provider.as_slug()),
            TemplateVariable::Event => Cow::Owned(meta.event.to_string()),
            TemplateVariable::Timestamp => Cow::Owned(meta.timestamp.to_rfc3339()),
            TemplateVariable::SessionId => opt_to_cow(&meta.session_id),
            TemplateVariable::Cwd => opt_to_cow(&meta.cwd),
            TemplateVariable::ToolName => opt_to_cow(&meta.tool_name),
            TemplateVariable::Error => opt_to_cow(&meta.error),
            TemplateVariable::Prompt => opt_to_cow(&meta.prompt),
            TemplateVariable::AgentType => opt_to_cow(&meta.agent_type),
            TemplateVariable::NotificationType => opt_to_cow(&meta.notification_type),

            // Environment: OS
            TemplateVariable::EnvOs => Cow::Owned(meta.env.os.name.clone()),
            TemplateVariable::EnvOsType => Cow::Owned(meta.env.os.os_type.clone()),
            TemplateVariable::EnvOsVersion => Cow::Owned(meta.env.os.version.clone()),
            TemplateVariable::EnvHostname => Cow::Owned(meta.env.os.hostname.clone()),

            // Environment: Hardware
            TemplateVariable::EnvArch => Cow::Owned(meta.env.hardware.arch.clone()),
            TemplateVariable::EnvCpu => Cow::Owned(meta.env.hardware.cpu.clone()),
            TemplateVariable::EnvCores => Cow::Owned(meta.env.hardware.cores.to_string()),

            // Environment: Git
            TemplateVariable::EnvBranch => {
                opt_nested_to_cow(meta.env.git.as_ref().and_then(|g| g.branch.as_deref()))
            }
            TemplateVariable::EnvIsDirty => Cow::Owned(
                meta.env
                    .git
                    .as_ref()
                    .map(|g| g.is_dirty.to_string())
                    .unwrap_or_default(),
            ),
            TemplateVariable::EnvHeadSha => {
                opt_nested_to_cow(meta.env.git.as_ref().and_then(|g| g.head_sha.as_deref()))
            }
            TemplateVariable::EnvHeadMessage => opt_nested_to_cow(
                meta.env
                    .git
                    .as_ref()
                    .and_then(|g| g.head_message.as_deref()),
            ),
            TemplateVariable::EnvRemote => {
                opt_nested_to_cow(meta.env.git.as_ref().and_then(|g| g.remote_name.as_deref()))
            }
            TemplateVariable::EnvHosting => opt_nested_to_cow(
                meta.env
                    .git
                    .as_ref()
                    .and_then(|g| g.hosting_provider.as_deref()),
            ),
            TemplateVariable::EnvRepoName => {
                opt_nested_to_cow(meta.env.git.as_ref().and_then(|g| g.repo_name.as_deref()))
            }
            TemplateVariable::EnvRepoOrg => {
                opt_nested_to_cow(meta.env.git.as_ref().and_then(|g| g.repo_org.as_deref()))
            }

            // Environment: Repo
            TemplateVariable::EnvLanguage => {
                opt_nested_to_cow(meta.env.primary_language.as_deref())
            }
            TemplateVariable::EnvIsMonorepo => Cow::Owned(
                meta.env
                    .repo
                    .as_ref()
                    .map(|r| r.is_monorepo.to_string())
                    .unwrap_or_default(),
            ),
            TemplateVariable::EnvMonorepoTool => opt_nested_to_cow(
                meta.env
                    .repo
                    .as_ref()
                    .and_then(|r| r.monorepo_tool.as_deref()),
            ),
        }
    }

    /// Look up a variable by its key string.
    pub fn from_key(key: &str) -> Option<TemplateVariable> {
        Self::all().iter().find(|v| v.key() == key).copied()
    }
}

/// Interpolate `{{placeholder}}` patterns in a template string using event metadata.
///
/// Supported placeholders are defined in [`TemplateVariable`]. Use
/// [`TemplateVariable::all()`] to enumerate all available variables.
///
/// Unknown placeholders are left as-is. `None` optional fields render as empty string.
///
/// Legacy single-brace placeholders (for example `{tool_name}`) are rewritten
/// to `{{tool_name}}` for transitional compatibility and emit a warning.
///
/// ## Examples
///
/// ```
/// # use claudine::dispatch::template::interpolate;
/// # use claudine::events::*;
/// # use claudine::provider::Provider;
/// # use std::collections::HashMap;
/// # use chrono::Utc;
/// # let meta = EventMeta {
/// #     provider: Provider::Claude,
/// #     event: AgenticEvent::BeforeTool,
/// #     timestamp: Utc::now(),
/// #     session_id: None, cwd: None, tool_name: None, tool_input: None,
/// #     tool_response: None, error: None, prompt: None, agent_type: None,
/// #     notification_type: None, notification_message: None,
/// #     extra: HashMap::new(), env: EnvironmentContext::default(),
/// # };
/// let result = interpolate("Provider is {{provider}}", &meta);
/// assert_eq!(result, "Provider is claude");
/// ```
pub fn interpolate(template: &str, meta: &EventMeta) -> String {
    let rewritten = rewrite_legacy_single_brace_placeholders(template);
    let source = rewritten.as_ref();

    let locations = ExpressionFinder::find_all_plain(source);
    if locations.is_empty() {
        return rewritten.into_owned();
    }

    let lookup = EventMetaExpressionLookup::new(meta);
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for location in &locations {
        output.push_str(&source[cursor..location.start]);
        let original = &source[location.start..location.end];
        match render_expression(&location.expression, &lookup) {
            Some(rendered) => output.push_str(&rendered),
            None => output.push_str(original),
        }
        cursor = location.end;
    }
    output.push_str(&source[cursor..]);
    output
}

/// Render a single `{{...}}` expression body, or `None` to preserve the
/// original token (matching legacy `interpolate()` behavior for unknown or
/// malformed expressions).
fn render_expression(expression: &str, lookup: &EventMetaExpressionLookup<'_>) -> Option<String> {
    let trimmed = expression.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = Parser::with_mode(trimmed, ParseMode::Interpolation)
        .ok()?
        .parse()
        .ok()?;

    // Preserve unknown bare-variable references unchanged so e.g.
    // `{{unknown_field}}` does not silently render as the empty string.
    // Composite expressions (fallbacks, ternaries, comparisons, function
    // calls) always evaluate, since their authors intentionally opted into
    // expression semantics.
    if let Expr::Variable(path) = &parsed
        && !is_known_path(path)
    {
        return None;
    }

    evaluate(&parsed, lookup)
        .ok()
        .map(|value| scalar_string(&value))
}

/// Returns whether a dotted path is a recognized template variable handled
/// by [`EventMetaExpressionLookup`].
fn is_known_path(path: &str) -> bool {
    if path.starts_with("env.")
        || path.starts_with("extra.")
        || path.starts_with("tool_input.")
        || path.starts_with("tool_response.")
        || path.starts_with("ctx.")
        || path == "ctx"
    {
        return true;
    }
    matches!(
        path,
        "provider"
            | "event"
            | "timestamp"
            | "session_id"
            | "cwd"
            | "tool_name"
            | "tool_input"
            | "tool_response"
            | "error"
            | "prompt"
            | "agent_type"
            | "notification_type"
            | "notification_message"
            | "extra"
            | "os.name"
            | "os.type"
            | "os.version"
            | "os.hostname"
            | "hardware.arch"
            | "hardware.cpu"
            | "hardware.cores"
            | "git.branch"
            | "git.is_dirty"
            | "git.head_sha"
            | "git.head_message"
            | "git.remote"
            | "git.hosting"
            | "git.repo_name"
            | "git.repo_org"
            | "project.language"
            | "project.is_monorepo"
            | "project.monorepo_tool"
    )
}

fn rewrite_legacy_single_brace_placeholders(template: &str) -> Cow<'_, str> {
    let mut output = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut rewrote = false;

    while let Some(ch) = chars.next() {
        if ch != '{' {
            output.push(ch);
            continue;
        }

        if matches!(chars.peek(), Some('{')) {
            output.push('{');
            output.push('{');
            chars.next();
            continue;
        }

        let mut inner = String::new();
        let mut found_closing = false;

        for next in chars.by_ref() {
            if next == '}' {
                found_closing = true;
                break;
            }
            inner.push(next);
        }

        if found_closing {
            let key = inner.trim();
            if is_legacy_placeholder_key(key) {
                output.push_str("{{");
                output.push_str(key);
                output.push_str("}}");
                rewrote = true;
            } else {
                output.push('{');
                output.push_str(&inner);
                output.push('}');
            }
        } else {
            output.push('{');
            output.push_str(&inner);
            break;
        }
    }

    if rewrote {
        warn!(
            template = %template,
            "legacy single-brace template placeholders are deprecated; use {{...}}"
        );
        Cow::Owned(output)
    } else {
        Cow::Borrowed(template)
    }
}

fn is_legacy_placeholder_key(key: &str) -> bool {
    LEGACY_KEY_RE.is_match(key)
}

/// Convert an `Option<String>` to a `Cow`, rendering `None` as empty string.
fn opt_to_cow(opt: &Option<String>) -> Cow<'_, str> {
    match opt {
        Some(s) => Cow::Borrowed(s.as_str()),
        None => Cow::Borrowed(""),
    }
}

/// Convert an `Option<&str>` (from nested optional access) to a `Cow`,
/// rendering `None` as empty string.
fn opt_nested_to_cow(opt: Option<&str>) -> Cow<'static, str> {
    match opt {
        Some(s) => Cow::Owned(s.to_string()),
        None => Cow::Borrowed(""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use crate::provider::Provider;
    use chrono::Utc;
    use std::collections::HashMap;

    fn sample_meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: Utc::now(),
            session_id: Some("abc123".to_string()),
            cwd: Some("/tmp/project".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext {
                os: OsContext {
                    os_type: "macos".to_string(),
                    name: "macOS".to_string(),
                    version: "15.3".to_string(),
                    kernel: "Darwin 25.3.0".to_string(),
                    hostname: "test-host".to_string(),
                    linux_family: None,
                    package_managers: vec!["brew".to_string()],
                },
                hardware: HardwareContext {
                    arch: "aarch64".to_string(),
                    cpu: "Apple M4 Max".to_string(),
                    cores: 16,
                    memory_bytes: 68719476736,
                    memory_available_bytes: 34359738368,
                },
                git: Some(GitContext {
                    repo_root: std::path::PathBuf::from("/tmp/project"),
                    branch: Some("main".to_string()),
                    is_dirty: true,
                    staged_count: 2,
                    unstaged_count: 1,
                    untracked_count: 0,
                    head_sha: Some("abc123def".to_string()),
                    head_message: Some("feat: add feature".to_string()),
                    user_name: Some("Test User".to_string()),
                    user_email: None,
                    remote_name: Some("origin".to_string()),
                    remote_url: None,
                    hosting_provider: Some("github".to_string()),
                    repo_name: Some("rusty-biscuit".to_string()),
                    repo_org: Some("anthropics".to_string()),
                }),
                repo: Some(RepoContext {
                    is_monorepo: true,
                    monorepo_tool: Some("cargoworkspace".to_string()),
                    root: std::path::PathBuf::from("/tmp/project"),
                    packages: vec!["lib".to_string(), "cli".to_string()],
                }),
                primary_language: Some("Rust".to_string()),
                package_area: Some("claudine".to_string()),
                package: Some("claudine-cli".to_string()),
            },
        }
    }

    #[test]
    fn provider_placeholder() {
        let meta = sample_meta();
        assert_eq!(interpolate("Hello {{provider}}", &meta), "Hello claude");
    }

    #[test]
    fn tool_name_and_git_branch() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("{{tool_name}} on {{git.branch}}", &meta),
            "Bash on main"
        );
    }

    #[test]
    fn unknown_placeholder_left_as_is() {
        let meta = sample_meta();
        assert_eq!(interpolate("{{unknown_field}}", &meta), "{{unknown_field}}");
    }

    #[test]
    fn none_optional_renders_empty() {
        let mut meta = sample_meta();
        meta.tool_name = None;
        assert_eq!(interpolate("Tool: {{tool_name}}", &meta), "Tool: ");
    }

    #[test]
    fn no_placeholders_returns_original() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("no placeholders here", &meta),
            "no placeholders here"
        );
    }

    #[test]
    fn event_display() {
        let meta = sample_meta();
        assert_eq!(interpolate("{{event}}", &meta), "before_tool");
    }

    #[test]
    fn hardware_cores() {
        let meta = sample_meta();
        assert_eq!(interpolate("{{hardware.cores}}", &meta), "16");
    }

    #[test]
    fn multiple_context_placeholders() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("{{os.type}} {{hardware.arch}}", &meta),
            "macos aarch64"
        );
    }

    #[test]
    fn git_repo_name() {
        let meta = sample_meta();
        assert_eq!(interpolate("{{git.repo_name}}", &meta), "rusty-biscuit");
    }

    #[test]
    fn git_repo_org() {
        let meta = sample_meta();
        assert_eq!(interpolate("{{git.repo_org}}", &meta), "anthropics");
    }

    #[test]
    fn git_repo_name_and_org_combined() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("{{git.repo_org}}/{{git.repo_name}}", &meta),
            "anthropics/rusty-biscuit"
        );
    }

    #[test]
    fn env_variable_resolution() {
        let meta = sample_meta();
        let expected = std::env::var("HOME").unwrap_or_default();
        let rendered = interpolate("{{env.HOME}}", &meta);
        assert_eq!(rendered, expected);
    }

    #[test]
    fn env_variable_default_is_used_when_not_present() {
        let meta = sample_meta();
        let rendered = interpolate(
            "{{env.CLAUDINE_TEMPLATE_TEST_MISSING || \"fallback\"}}",
            &meta,
        );
        assert_eq!(rendered, "fallback");
    }

    #[test]
    fn ternary_on_git_is_dirty() {
        let meta = sample_meta();
        let rendered = interpolate("Status: {{git.is_dirty ? \"dirty\" : \"clean\"}}", &meta);
        assert_eq!(rendered, "Status: dirty");
    }

    #[test]
    fn numeric_comparison_on_hardware_cores() {
        let meta = sample_meta();
        let rendered = interpolate("Speed: {{hardware.cores > 8 ? \"fast\" : \"slow\"}}", &meta);
        assert_eq!(rendered, "Speed: fast");
    }

    #[test]
    fn fallback_on_missing_env_variable() {
        let meta = sample_meta();
        // SAFETY: tests run sequentially in a single process; we remove the
        // var on the way out.
        unsafe {
            std::env::remove_var("CLAUDINE_TEMPLATE_TEST_FALLBACK_X");
        }
        let rendered = interpolate(
            "Mode: {{env.CLAUDINE_TEMPLATE_TEST_FALLBACK_X || \"local\"}}",
            &meta,
        );
        assert_eq!(rendered, "Mode: local");
    }

    #[test]
    fn length_helper_on_branch_name() {
        let meta = sample_meta();
        let rendered = interpolate(
            "{{length(git.branch) > 30 ? \"long-branch\" : git.branch}}",
            &meta,
        );
        // sample_meta branch is "main" (4 chars), so we get the branch name.
        assert_eq!(rendered, "main");
    }

    #[test]
    fn boolean_value_renders_as_text() {
        let meta = sample_meta();
        assert_eq!(interpolate("{{git.is_dirty}}", &meta), "true");
    }

    #[test]
    fn malformed_expression_is_preserved() {
        let meta = sample_meta();
        // `+` is not a supported operator; parser fails -> token preserved.
        assert_eq!(interpolate("{{a + b}}", &meta), "{{a + b}}");
    }

    #[test]
    fn legacy_single_pipe_fallback_is_preserved_verbatim() {
        // Single-pipe `|` is no longer recognised as a fallback operator;
        // Darkmatter's interpolation lexer only accepts `||`. The token
        // must therefore round-trip unchanged so operators can spot stale
        // configs in their output.
        let meta = sample_meta();
        let raw = "{{env.CLAUDINE_TEMPLATE_TEST_LEGACY_PIPE | \"fallback\"}}";
        // SAFETY: tests run sequentially in this module.
        unsafe {
            std::env::remove_var("CLAUDINE_TEMPLATE_TEST_LEGACY_PIPE");
        }
        assert_eq!(interpolate(raw, &meta), raw);
    }

    #[test]
    fn legacy_single_brace_template_is_rewritten() {
        let meta = sample_meta();
        assert_eq!(interpolate("Tool {tool_name}", &meta), "Tool Bash");
    }

    // Tests for TemplateVariable enum
    #[test]
    fn all_variables_have_unique_keys() {
        let keys: Vec<_> = TemplateVariable::all().iter().map(|v| v.key()).collect();
        let mut unique_keys = keys.clone();
        unique_keys.sort();
        unique_keys.dedup();
        assert_eq!(
            keys.len(),
            unique_keys.len(),
            "Duplicate keys found in TemplateVariable::all()"
        );
    }

    #[test]
    fn from_key_finds_all_variables() {
        for var in TemplateVariable::all() {
            let found = TemplateVariable::from_key(var.key());
            assert_eq!(
                found,
                Some(*var),
                "from_key({}) should find {:?}",
                var.key(),
                var
            );
        }
    }

    #[test]
    fn from_key_returns_none_for_unknown() {
        assert_eq!(TemplateVariable::from_key("unknown"), None);
        assert_eq!(TemplateVariable::from_key("git.unknown"), None);
    }

    #[test]
    fn placeholder_format() {
        assert_eq!(TemplateVariable::Provider.placeholder(), "{{provider}}");
        assert_eq!(TemplateVariable::EnvBranch.placeholder(), "{{git.branch}}");
    }

    #[test]
    fn event_variables_filter() {
        let event_vars: Vec<_> = TemplateVariable::event_variables().collect();
        assert!(event_vars.contains(&&TemplateVariable::Provider));
        assert!(event_vars.contains(&&TemplateVariable::ToolName));
        assert!(!event_vars.contains(&&TemplateVariable::EnvBranch));
    }

    #[test]
    fn context_variables_filter() {
        let ctx_vars: Vec<_> = TemplateVariable::context_variables().collect();
        assert!(ctx_vars.contains(&&TemplateVariable::EnvBranch));
        assert!(ctx_vars.contains(&&TemplateVariable::EnvRepoName));
        assert!(!ctx_vars.contains(&&TemplateVariable::Provider));
    }

    #[test]
    fn all_variables_have_descriptions() {
        for var in TemplateVariable::all() {
            assert!(
                !var.description().is_empty(),
                "{:?} should have a description",
                var
            );
        }
    }

    #[test]
    fn all_variables_have_availability() {
        for var in TemplateVariable::all() {
            assert!(
                !var.availability().is_empty(),
                "{:?} should have availability",
                var
            );
        }
    }
}
