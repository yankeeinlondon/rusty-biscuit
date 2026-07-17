//! R8 — the session-compatibility key computed from a resolved launch plan.
//!
//! A `resume` reuses the provider session an earlier attempt opened. Before the
//! resume runs, the active document is refreshed through canonical preparation;
//! if that refresh changes any property the provider fixed when it opened the
//! session, the old session and the new plan disagree. This module reads the
//! *same* resolved launch state the harness is about to execute — provider,
//! effective child environment, resolved argv, working directory, and the
//! invocation's mode flags — into a [`SessionCompatibilityKey`] the loop stores
//! with the live session and re-derives on the resume attempt to compare.

use std::collections::HashMap;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::Path;

use claudine::composition::SessionCompatibilityKey;
use claudine::provider::Provider;

use super::super::profile::WrapperProfile;
use super::MaterializedHarnessPrompt;

/// System-prompt delivery flags a provider profile pushes onto argv. The value
/// following each is either the prompt content inline or a file carrying it;
/// both are folded into the key's content digest.
const SYSTEM_PROMPT_FLAGS: &[&str] = &[
    "--append-system-prompt",
    "--append-system-prompt-file",
    "--replace-system-prompt",
    "--replace-system-prompt-file",
    "--system-prompt",
    "--system-prompt-file",
];

/// Compute the compatibility key for the launch state an attempt is about to
/// run with.
///
/// `base_args` and `base_env` are the invocation-wide resolved argv/environment;
/// `materialized` carries the per-attempt env overlay (the R6 target launch
/// identity for a proxied target). The effective environment is `base_env`
/// overlaid with `materialized.env_overrides`, matching what
/// [`super::build_harness_launch`] hands the child.
#[allow(clippy::too_many_arguments)]
pub(crate) fn session_compat_key(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    yolo: bool,
    effective_non_interactive: bool,
    use_structured: bool,
    structured_codex: bool,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    materialized: &MaterializedHarnessPrompt,
) -> SessionCompatibilityKey {
    SessionCompatibilityKey {
        provider: provider.as_slug().to_string(),
        model: effective_env_value(base_env, &materialized.env_overrides, "MODEL"),
        binary: format!("{}@{}", profile.binary(), binary_path.display()),
        // A provider that cannot resume never opens a resumable session, so the
        // capability is part of the session's identity alongside its slug.
        resume_protocol: format!("{}:{}", provider.as_slug(), profile.supports_resume()),
        workspace_cwd: child_cwd.display().to_string(),
        permission_mode: if yolo { "bypass" } else { "prompt" }.to_string(),
        interactivity: if effective_non_interactive {
            "non-interactive"
        } else {
            "interactive"
        }
        .to_string(),
        structured_output: format!("{use_structured}:{structured_codex}"),
        system_prompt: system_prompt_digest(base_args),
        mcp_servers: mcp_signal_set(base_args, base_env, &materialized.env_overrides),
        extra: std::collections::BTreeMap::new(),
    }
}

/// The value of `key` in the child environment: the per-attempt override wins
/// over the invocation-wide base, mirroring the overlay `build_harness_launch`
/// applies.
fn effective_env_value(
    base_env: &HashMap<OsString, OsString>,
    overrides: &[(String, String)],
    key: &str,
) -> Option<String> {
    if let Some((_, value)) = overrides.iter().rev().find(|(k, _)| k == key) {
        return Some(value.clone());
    }
    base_env
        .get(OsString::from(key).as_os_str())
        .map(|v| v.to_string_lossy().into_owned())
}

/// Fold every system-prompt delivery flag and the content it delivers into one
/// digest. A file-backed value contributes its file *content* (not its unstable
/// temp path); an inline value contributes the value itself.
fn system_prompt_digest(base_args: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut iter = base_args.iter().peekable();
    while let Some(arg) = iter.next() {
        if !SYSTEM_PROMPT_FLAGS.contains(&arg.as_str()) {
            continue;
        }
        let content = iter.peek().map(|value| {
            let path = Path::new(value.as_str());
            match std::fs::read_to_string(path) {
                Ok(body) => body,
                Err(_) => (*value).clone(),
            }
        });
        match content {
            Some(body) => parts.push(format!("{arg}={:x}", hash_str(&body))),
            None => parts.push(arg.clone()),
        }
    }
    if parts.is_empty() {
        return "none".to_string();
    }
    parts.join("|")
}

/// Best-effort MCP identity: the config-bearing signals visible at this layer —
/// MCP argv flags and their values, plus MCP-carrying environment (the shadow
/// config content OpenCode injects and any `*MCP*` variable). Sorted so the set,
/// not its order, defines equality. Provider adapters that can enumerate a
/// precise server set contribute it through the key's `extra` map instead.
fn mcp_signal_set(
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    overrides: &[(String, String)],
) -> Vec<String> {
    let mut signals: Vec<String> = Vec::new();
    let mut iter = base_args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg.to_ascii_lowercase().contains("mcp") {
            match iter.peek() {
                Some(value) => signals.push(format!("{arg} {value}")),
                None => signals.push(arg.clone()),
            }
        }
    }
    let mut push_env = |key: &str, value: &str| {
        let upper = key.to_ascii_uppercase();
        if upper.contains("MCP") || upper == "OPENCODE_CONFIG_CONTENT" {
            signals.push(format!("{key}={:x}", hash_str(value)));
        }
    };
    for (key, value) in base_env {
        push_env(&key.to_string_lossy(), &value.to_string_lossy());
    }
    for (key, value) in overrides {
        push_env(key, value);
    }
    signals.sort();
    signals.dedup();
    signals
}

fn hash_str(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
