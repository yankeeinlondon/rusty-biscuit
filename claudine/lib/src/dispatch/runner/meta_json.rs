use serde_json::{Map, Value};
use tracing::warn;

use crate::events::EventMeta;

/// Serialize an [`EventMeta`] to a JSON value for `when` evaluation.
///
/// The serialized [`EventMeta`] is augmented with flattened top-level
/// alias keys — `os`, `hardware`, `git`, and `project` — that mirror the
/// paths exposed by
/// [`EventMetaExpressionLookup`](crate::dispatch::expression::EventMetaExpressionLookup).
/// Darkmatter's [`evaluate_condition_against`] uses a
/// [`ShortcutLookup`](darkmatter::markdown::compose::expression::ShortcutLookup)
/// that performs flat JSON-path resolution, so without these aliases an
/// expression such as `git.branch == 'main'` would fail to resolve
/// (the underlying serialized value nests these fields under `env.git.*`,
/// `env.hardware.*`, etc.). Mirroring the alias surface here keeps hook
/// `when` evaluation, template interpolation, and matcher evaluation in
/// agreement on a single set of paths.
///
/// Falls back to [`Value::Null`] on the (effectively unreachable)
/// serialization failure so a transient encoding issue cannot abort the
/// dispatch loop.
///
/// ## Notes
///
/// Keep the alias surface in sync with
/// [`EventMetaExpressionLookup::resolve_env_path`](crate::dispatch::expression)
/// when adding or renaming event metadata fields.
pub(super) fn event_meta_to_json(meta: &EventMeta) -> Value {
    let mut value = serde_json::to_value(meta).unwrap_or_else(|err| {
        warn!(%err, "serializing EventMeta for `when` evaluation failed; using null payload");
        Value::Null
    });

    if let Value::Object(map) = &mut value {
        for (key, alias) in flatten_event_meta_aliases(meta) {
            map.insert(key, alias);
        }
    }

    value
}

/// Build the flattened top-level alias entries for `when` evaluation.
///
/// The returned map mirrors
/// [`EventMetaExpressionLookup::resolve_env_path`](crate::dispatch::expression)
/// exactly:
///
/// - `os` — `{ name, type, version, hostname }` (note `type`, NOT
///   `os_type`, matching the path the expression lookup exposes).
/// - `hardware` — `{ arch, cpu, cores }` with `cores` preserved as a
///   JSON `Number` so numeric comparisons such as `hardware.cores > 8`
///   work without coercion.
/// - `git` — present only when `meta.env.git.is_some()`. Keys: `branch`,
///   `is_dirty` (JSON `Bool`), `head_sha`, `head_message`, `remote`
///   (from `remote_name`), `hosting` (from `hosting_provider`),
///   `repo_name`, `repo_org`. When `git` is `None` the alias is omitted
///   so `git.branch` resolves to `Null` instead of an empty object.
/// - `project` — present when either `primary_language` or `repo` is
///   set. Keys: `language`, `is_monorepo` (JSON `Bool`), `monorepo_tool`.
///
/// ## Notes
///
/// This helper is unit-tested as a contract: the same paths that
/// [`EventMetaExpressionLookup`] exposes must appear here with the
/// same value types.
pub(super) fn flatten_event_meta_aliases(meta: &EventMeta) -> Map<String, Value> {
    let mut aliases = Map::new();

    let mut os_obj = Map::new();
    os_obj.insert("name".to_string(), Value::String(meta.env.os.name.clone()));
    os_obj.insert(
        "type".to_string(),
        Value::String(meta.env.os.os_type.clone()),
    );
    os_obj.insert(
        "version".to_string(),
        Value::String(meta.env.os.version.clone()),
    );
    os_obj.insert(
        "hostname".to_string(),
        Value::String(meta.env.os.hostname.clone()),
    );
    aliases.insert("os".to_string(), Value::Object(os_obj));

    let mut hw_obj = Map::new();
    hw_obj.insert(
        "arch".to_string(),
        Value::String(meta.env.hardware.arch.clone()),
    );
    hw_obj.insert(
        "cpu".to_string(),
        Value::String(meta.env.hardware.cpu.clone()),
    );
    hw_obj.insert(
        "cores".to_string(),
        Value::Number(meta.env.hardware.cores.into()),
    );
    aliases.insert("hardware".to_string(), Value::Object(hw_obj));

    if let Some(git) = meta.env.git.as_ref() {
        let mut git_obj = Map::new();
        if let Some(branch) = git.branch.as_ref() {
            git_obj.insert("branch".to_string(), Value::String(branch.clone()));
        }
        git_obj.insert("is_dirty".to_string(), Value::Bool(git.is_dirty));
        if let Some(sha) = git.head_sha.as_ref() {
            git_obj.insert("head_sha".to_string(), Value::String(sha.clone()));
        }
        if let Some(message) = git.head_message.as_ref() {
            git_obj.insert("head_message".to_string(), Value::String(message.clone()));
        }
        if let Some(remote) = git.remote_name.as_ref() {
            git_obj.insert("remote".to_string(), Value::String(remote.clone()));
        }
        if let Some(hosting) = git.hosting_provider.as_ref() {
            git_obj.insert("hosting".to_string(), Value::String(hosting.clone()));
        }
        if let Some(repo_name) = git.repo_name.as_ref() {
            git_obj.insert("repo_name".to_string(), Value::String(repo_name.clone()));
        }
        if let Some(repo_org) = git.repo_org.as_ref() {
            git_obj.insert("repo_org".to_string(), Value::String(repo_org.clone()));
        }
        aliases.insert("git".to_string(), Value::Object(git_obj));
    }

    if meta.env.primary_language.is_some() || meta.env.repo.is_some() {
        let mut project_obj = Map::new();
        if let Some(language) = meta.env.primary_language.as_ref() {
            project_obj.insert("language".to_string(), Value::String(language.clone()));
        }
        if let Some(repo) = meta.env.repo.as_ref() {
            project_obj.insert("is_monorepo".to_string(), Value::Bool(repo.is_monorepo));
            if let Some(tool) = repo.monorepo_tool.as_ref() {
                project_obj.insert("monorepo_tool".to_string(), Value::String(tool.clone()));
            }
        }
        aliases.insert("project".to_string(), Value::Object(project_obj));
    }

    aliases
}

pub(super) fn strip_nulls(value: &mut Value) {
    strip_nulls_recursive(value, 0);
}

const STRIP_NULLS_MAX_DEPTH: u32 = 64;

fn strip_nulls_recursive(value: &mut Value, depth: u32) {
    if depth >= STRIP_NULLS_MAX_DEPTH {
        return;
    }
    match value {
        Value::Object(map) => {
            let mut to_remove = Vec::new();
            for (key, nested) in map.iter_mut() {
                strip_nulls_recursive(nested, depth + 1);
                if nested.is_null() {
                    to_remove.push(key.clone());
                }
            }
            for key in to_remove {
                map.remove(&key);
            }
        }
        Value::Array(items) => {
            for nested in items.iter_mut() {
                strip_nulls_recursive(nested, depth + 1);
            }
            items.retain(|item| !item.is_null());
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;
    use crate::events::{
        AgenticEvent, EnvironmentContext, EventMeta, GitContext, HardwareContext, OsContext,
        RepoContext,
    };
    use crate::provider::Provider;

    fn meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: Utc::now(),
            session_id: Some("test-session".to_string()),
            cwd: Some("/tmp".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn make_meta_for_when_tests() -> EventMeta {
        let mut m = meta();
        m.tool_name = Some("Bash".to_string());
        m
    }

    fn meta_with_full_env() -> EventMeta {
        let mut m = make_meta_for_when_tests();
        m.env.os = OsContext {
            os_type: "macos".to_string(),
            name: "macOS".to_string(),
            version: "15.3".to_string(),
            kernel: "Darwin 25.3.0".to_string(),
            hostname: "test-host".to_string(),
            linux_family: None,
            package_managers: vec!["brew".to_string()],
        };
        m.env.hardware = HardwareContext {
            arch: "aarch64".to_string(),
            cpu: "Apple M4 Max".to_string(),
            cores: 16,
            memory_bytes: 68_719_476_736,
            memory_available_bytes: 34_359_738_368,
        };
        m.env.git = Some(GitContext {
            repo_root: PathBuf::from("/tmp/project"),
            branch: Some("main".to_string()),
            is_dirty: true,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            head_sha: Some("abc123def".to_string()),
            head_message: Some("feat: add feature".to_string()),
            user_name: None,
            user_email: None,
            remote_name: Some("origin".to_string()),
            remote_url: None,
            hosting_provider: Some("github".to_string()),
            repo_name: Some("rusty-biscuit".to_string()),
            repo_org: Some("anthropics".to_string()),
        });
        m.env.repo = Some(RepoContext {
            is_monorepo: true,
            monorepo_tool: Some("cargo_workspace".to_string()),
            root: PathBuf::from("/tmp/project"),
            packages: vec!["lib".to_string(), "cli".to_string()],
        });
        m.env.primary_language = Some("Rust".to_string());
        m
    }

    #[test]
    fn flatten_event_meta_aliases_mirrors_expression_lookup() {
        // Contract test: the alias surface produced by the runner must
        // match the paths the `EventMetaExpressionLookup` exposes,
        // including value types (booleans stay bools, numbers stay
        // numbers).
        let meta = meta_with_full_env();
        let aliases = flatten_event_meta_aliases(&meta);

        let git = aliases
            .get("git")
            .and_then(|v| v.as_object())
            .expect("git alias should be present when env.git is Some");
        assert_eq!(git.get("branch"), Some(&serde_json::json!("main")));
        assert_eq!(git.get("is_dirty"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(
            git.get("repo_name"),
            Some(&serde_json::json!("rusty-biscuit"))
        );
        assert_eq!(git.get("remote"), Some(&serde_json::json!("origin")));
        assert_eq!(git.get("hosting"), Some(&serde_json::json!("github")));

        let hardware = aliases
            .get("hardware")
            .and_then(|v| v.as_object())
            .expect("hardware alias should be present");
        let cores = hardware
            .get("cores")
            .expect("hardware.cores must be present");
        assert!(cores.is_number(), "hardware.cores must be a JSON number");
        assert_eq!(cores.as_u64(), Some(16));

        let project = aliases
            .get("project")
            .and_then(|v| v.as_object())
            .expect("project alias should be present");
        assert_eq!(
            project.get("is_monorepo"),
            Some(&serde_json::Value::Bool(true)),
            "project.is_monorepo must be a JSON bool"
        );
        assert_eq!(project.get("language"), Some(&serde_json::json!("Rust")));

        let os = aliases
            .get("os")
            .and_then(|v| v.as_object())
            .expect("os alias should be present");
        assert_eq!(
            os.get("type"),
            Some(&serde_json::json!("macos")),
            "os.type must be exposed (NOT os.os_type)"
        );
    }
}
