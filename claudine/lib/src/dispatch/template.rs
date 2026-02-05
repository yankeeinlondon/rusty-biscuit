use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

use crate::events::EventMeta;

/// Compiled regex matching `{placeholder}` patterns.
///
/// Captures the inner name which must be lowercase ASCII letters,
/// underscores, and dots (for `env.*` placeholders).
static PLACEHOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([a-z_.]+)\}").expect("placeholder regex is valid"));

/// Interpolate `{placeholder}` patterns in a template string using event metadata.
///
/// Supported placeholders: `{provider}`, `{event}`, `{session_id}`, `{tool_name}`,
/// `{error}`, `{prompt}`, `{agent_type}`, `{notification_type}`, `{cwd}`, `{timestamp}`,
/// and `{env.*}` variants for environment context fields.
///
/// Unknown placeholders are left as-is. `None` optional fields render as empty string.
///
/// ## Examples
///
/// ```
/// # use claudine::dispatch::template::interpolate;
/// # use claudine::events::*;
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
/// let result = interpolate("Provider is {provider}", &meta);
/// assert_eq!(result, "Provider is claude");
/// ```
pub fn interpolate(template: &str, meta: &EventMeta) -> String {
    PLACEHOLDER_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            resolve_placeholder(key, meta)
        })
        .into_owned()
}

/// Resolve a single placeholder key to its replacement string.
///
/// Returns a `Cow::Owned` for known keys (value or empty string for `None`),
/// or the original `{key}` text for unknown keys.
fn resolve_placeholder<'a>(key: &str, meta: &'a EventMeta) -> Cow<'a, str> {
    match key {
        // Top-level fields
        "provider" => Cow::Owned(meta.provider.to_string()),
        "event" => Cow::Owned(meta.event.to_string()),
        "timestamp" => Cow::Owned(meta.timestamp.to_rfc3339()),
        "session_id" => opt_to_cow(&meta.session_id),
        "tool_name" => opt_to_cow(&meta.tool_name),
        "error" => opt_to_cow(&meta.error),
        "prompt" => opt_to_cow(&meta.prompt),
        "agent_type" => opt_to_cow(&meta.agent_type),
        "notification_type" => opt_to_cow(&meta.notification_type),
        "cwd" => opt_to_cow(&meta.cwd),

        // Environment: OS
        "env.os" => Cow::Owned(meta.env.os.name.clone()),
        "env.os_type" => Cow::Owned(meta.env.os.os_type.clone()),
        "env.os_version" => Cow::Owned(meta.env.os.version.clone()),
        "env.hostname" => Cow::Owned(meta.env.os.hostname.clone()),

        // Environment: Hardware
        "env.arch" => Cow::Owned(meta.env.hardware.arch.clone()),
        "env.cpu" => Cow::Owned(meta.env.hardware.cpu.clone()),
        "env.cores" => Cow::Owned(meta.env.hardware.cores.to_string()),

        // Environment: Git
        "env.branch" => opt_nested_to_cow(
            meta.env.git.as_ref().and_then(|g| g.branch.as_deref()),
        ),
        "env.is_dirty" => Cow::Owned(
            meta.env
                .git
                .as_ref()
                .map(|g| g.is_dirty.to_string())
                .unwrap_or_default(),
        ),
        "env.head_sha" => opt_nested_to_cow(
            meta.env.git.as_ref().and_then(|g| g.head_sha.as_deref()),
        ),
        "env.head_message" => opt_nested_to_cow(
            meta.env
                .git
                .as_ref()
                .and_then(|g| g.head_message.as_deref()),
        ),
        "env.remote" => opt_nested_to_cow(
            meta.env
                .git
                .as_ref()
                .and_then(|g| g.remote_name.as_deref()),
        ),
        "env.hosting" => opt_nested_to_cow(
            meta.env
                .git
                .as_ref()
                .and_then(|g| g.hosting_provider.as_deref()),
        ),

        // Environment: Repo
        "env.is_monorepo" => Cow::Owned(
            meta.env
                .repo
                .as_ref()
                .map(|r| r.is_monorepo.to_string())
                .unwrap_or_default(),
        ),
        "env.monorepo_tool" => opt_nested_to_cow(
            meta.env
                .repo
                .as_ref()
                .and_then(|r| r.monorepo_tool.as_deref()),
        ),

        // Environment: Language
        "env.language" => opt_nested_to_cow(meta.env.primary_language.as_deref()),

        // Unknown placeholder: leave as-is
        _ => Cow::Owned(format!("{{{key}}}")),
    }
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
                }),
                repo: Some(RepoContext {
                    is_monorepo: true,
                    monorepo_tool: Some("cargoworkspace".to_string()),
                    root: std::path::PathBuf::from("/tmp/project"),
                    packages: vec!["lib".to_string(), "cli".to_string()],
                }),
                primary_language: Some("Rust".to_string()),
            },
        }
    }

    #[test]
    fn provider_placeholder() {
        let meta = sample_meta();
        assert_eq!(interpolate("Hello {provider}", &meta), "Hello claude");
    }

    #[test]
    fn tool_name_and_env_branch() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("{tool_name} on {env.branch}", &meta),
            "Bash on main"
        );
    }

    #[test]
    fn unknown_placeholder_left_as_is() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("{unknown_field}", &meta),
            "{unknown_field}"
        );
    }

    #[test]
    fn none_optional_renders_empty() {
        let mut meta = sample_meta();
        meta.tool_name = None;
        assert_eq!(interpolate("Tool: {tool_name}", &meta), "Tool: ");
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
        assert_eq!(interpolate("{event}", &meta), "before_tool");
    }

    #[test]
    fn env_cores() {
        let meta = sample_meta();
        assert_eq!(interpolate("{env.cores}", &meta), "16");
    }

    #[test]
    fn multiple_env_placeholders() {
        let meta = sample_meta();
        assert_eq!(
            interpolate("{env.os_type} {env.arch}", &meta),
            "macos aarch64"
        );
    }
}
