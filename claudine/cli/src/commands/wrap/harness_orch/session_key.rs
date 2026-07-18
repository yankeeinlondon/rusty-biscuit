//! R8 — the session-compatibility key computed from a resolved launch bundle.
//!
//! A `resume` reuses the provider session an earlier attempt opened. Before the
//! resume runs, the active document is refreshed through canonical preparation;
//! if that refresh changes any property the provider fixed when it opened the
//! session, the old session and the new plan disagree. This module reads the
//! resolved launch state the harness is about to execute — the effective child
//! environment ([`AttemptLaunch::env`], already carrying the R6 target-launch
//! `env_overrides` overlay) plus the canonical argv and the invocation's mode
//! flags — into a [`SessionCompatibilityKey`] the loop stores with the live
//! session and re-derives on the resume attempt to compare.
//!
//! ## Which facets are authoritative
//!
//! - **Environment-derived** (`model`, and the environment MCP signals): read
//!   from [`AttemptLaunch::env`], the exact child environment the provider is
//!   spawned with. This is the effective value *after* the target-launch rebuild
//!   overlays its `AGENT`/`MODEL`/`YOLO` env, so a proxied target's rebuilt model
//!   enters the comparison rather than the router's frozen value.
//! - **Argv-derived** (`system_prompt`, the argv MCP flags): read from the
//!   invocation's *canonical* argv — the pre-resume-normalization argv, the same
//!   for the session-opening attempt and the resume attempt. It is deliberately
//!   NOT [`AttemptLaunch::args`]: the resume path re-points argv at the provider's
//!   resume entrypoint and carries through only a small whitelist of flags
//!   (`resume::append_resume_passthrough_args`), intentionally dropping the
//!   system-prompt and MCP flags because the live session already holds them.
//!   Comparing the resume's stripped argv against the opener's full argv would
//!   flag every such resume as incompatible — a false refusal. The canonical argv
//!   keeps the comparison apples-to-apples. (Per-attempt argv rebuild is deferred
//!   work — see `loop_control/target_launch.rs` — so the canonical argv is also
//!   the freshest argv identity available.)
//! - **Provider-contributed** ([`SessionCompatibilityKey::extra`]): not yet
//!   populated. No provider adapter currently contributes a precise resume
//!   identity, so the map is left empty rather than filled with a heuristic. The
//!   generic facets above are the authoritative set today.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;

use claudine::composition::SessionCompatibilityKey;
use claudine::provider::Provider;

use super::super::profile::WrapperProfile;
use super::AttemptLaunch;

/// Inline system-prompt delivery flags: the value following each is the prompt
/// content itself, delivered to the provider verbatim, and is hashed as the
/// literal argv value.
const SYSTEM_PROMPT_INLINE_FLAGS: &[&str] =
    &["--append-system-prompt", "--replace-system-prompt", "--system-prompt"];

/// File-backed system-prompt delivery flags: the value following each is a path
/// whose *content* is delivered to the provider, so the digest reads the file's
/// bytes (not the unstable temp path). A missing file falls back to the literal.
const SYSTEM_PROMPT_FILE_FLAGS: &[&str] = &[
    "--append-system-prompt-file",
    "--replace-system-prompt-file",
    "--system-prompt-file",
];

/// Compute the compatibility key for the launch state an attempt is about to
/// run with.
///
/// `canonical_args` is the invocation's pre-resume-normalization argv (the same
/// for the session-opening and resume attempts); `launch` is the resolved bundle
/// [`super::build_harness_launch`] produced, whose [`AttemptLaunch::env`] is the
/// exact effective child environment. See the module docs for why the argv
/// facets read `canonical_args` rather than [`AttemptLaunch::args`].
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
    canonical_args: &[String],
    launch: &AttemptLaunch,
) -> SessionCompatibilityKey {
    let child_env = &launch.env;
    SessionCompatibilityKey {
        provider: provider.as_slug().to_string(),
        model: child_env_value(child_env, "MODEL"),
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
        system_prompt: system_prompt_digest(canonical_args),
        mcp_servers: mcp_signal_set(canonical_args, child_env),
        extra: std::collections::BTreeMap::new(),
    }
}

/// The value of `key` in the effective child environment.
fn child_env_value(child_env: &HashMap<OsString, OsString>, key: &str) -> Option<String> {
    child_env
        .get(OsString::from(key).as_os_str())
        .map(|v| v.to_string_lossy().into_owned())
}

/// Fold every system-prompt delivery flag and the content it delivers into one
/// digest.
///
/// An inline flag contributes the literal argv value it is delivered with; a
/// `*-file` flag contributes the referenced file's *content* (its unstable temp
/// path is not part of the identity), falling back to the literal when the file
/// cannot be read. Splitting the two is what keeps an inline value that happens
/// to name an existing file hashed as the string the provider receives, not as
/// that file's bytes.
fn system_prompt_digest(args: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        let is_inline = SYSTEM_PROMPT_INLINE_FLAGS.contains(&arg.as_str());
        let is_file = SYSTEM_PROMPT_FILE_FLAGS.contains(&arg.as_str());
        if !is_inline && !is_file {
            continue;
        }
        let Some(value) = iter.peek() else {
            parts.push(arg.clone());
            continue;
        };
        let content = if is_file {
            std::fs::read_to_string(Path::new(value.as_str())).unwrap_or_else(|_| (*value).clone())
        } else {
            (*value).clone()
        };
        parts.push(format!("{arg}={:016x}", hash_str(&content)));
    }
    if parts.is_empty() {
        return "none".to_string();
    }
    parts.join("|")
}

/// MCP identity visible at this layer: the MCP argv flags and their values, plus
/// the MCP-carrying environment (the shadow config content OpenCode injects and
/// any `*MCP*` variable). Sorted so the set, not its order, defines equality.
///
/// This is a signal set, not an enumerated server list: it fingerprints the MCP
/// configuration as it is actually delivered on argv/env, which is sufficient to
/// detect a change across a refresh. A provider adapter that can enumerate a
/// precise server set would contribute it through [`SessionCompatibilityKey::extra`].
fn mcp_signal_set(args: &[String], child_env: &HashMap<OsString, OsString>) -> Vec<String> {
    let mut signals: Vec<String> = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg.to_ascii_lowercase().contains("mcp") {
            match iter.peek() {
                Some(value) => signals.push(format!("{arg} {value}")),
                None => signals.push(arg.clone()),
            }
        }
    }
    for (key, value) in child_env {
        let key = key.to_string_lossy();
        let upper = key.to_ascii_uppercase();
        if upper.contains("MCP") || upper == "OPENCODE_CONFIG_CONTENT" {
            signals.push(format!("{key}={:016x}", hash_str(&value.to_string_lossy())));
        }
    }
    signals.sort();
    signals.dedup();
    signals
}

/// Content digest via the repository's non-crypto hashing authority.
fn hash_str(value: &str) -> u64 {
    biscuit_hash::xx_hash(value)
}

#[cfg(test)]
mod tests;
