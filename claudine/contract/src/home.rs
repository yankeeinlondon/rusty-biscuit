//! Shadow-home isolation: an ephemeral `HOME` tree holding only the minimum
//! provider auth material plus an adapter-owned restrictive policy file.
//!
//! For an enabled provider, the real `HOME` is also where the agentic CLI's
//! config, memory, MCP config, and session state live. Forwarding it would let
//! a CLI running untrusted prompt text read all of that. Instead the session's
//! `HOME` is pointed at a throwaway tree that contains nothing but the
//! credential file(s) needed to authenticate and a deny-all policy file — so
//! the user's normal provider home is never mounted into the session.
//!
//! Auth material is curated per provider as an explicit, auditable allowlist
//! rather than derived from the catalog: this is a security boundary, and only
//! credential files are copied — never settings, memory, MCP config, or
//! session state.

use std::io;
use std::path::Path;

use claudine::provider_id::Provider;
use tempfile::TempDir;

/// Relative paths under `HOME` whose contents are copied into the shadow home.
///
/// Strictly credential files. Anything not listed here is absent from the
/// shadow home, which is the point: the provider sees auth and nothing else.
pub(crate) fn auth_files(provider: Provider) -> &'static [&'static str] {
    match provider {
        // macOS Claude Code keeps credentials in the Keychain (reachable
        // regardless of HOME); the file path is the Linux/secondary location.
        Provider::Claude => &[".claude/.credentials.json"],
        Provider::Codex => &[".codex/auth.json"],
        _ => &[],
    }
}

/// A restrictive policy file written into the shadow home to deny tools before
/// the model turn starts, for providers that read one. Returns the relative
/// path under `HOME` and the file content.
///
/// Claude Code reads `~/.claude/settings.json`; a catch-all `permissions.deny`
/// blocks every tool for the run. Codex exposes no deny-all policy file (its
/// execution-rules can only forbid named command prefixes escaping the sandbox),
/// so its pre-turn constraint is the `--sandbox read-only` argv flag instead —
/// see [`crate::session::tool_denial_args`].
pub(crate) fn policy_file(provider: Provider) -> Option<(&'static str, String)> {
    match provider {
        Provider::Claude => {
            let settings = r#"{"permissions":{"deny":["*"]}}"#;
            Some((".claude/settings.json", settings.to_string()))
        }
        _ => None,
    }
}

/// Build an isolated shadow `HOME` for a provider session.
///
/// Copies each present auth file from `real_home` (the caller's `HOME`, when
/// known) into the new tree, then writes the provider's deny-all policy file.
/// The returned [`TempDir`] must be held until the session completes; dropping
/// it removes the tree.
///
/// ## Errors
///
/// Returns the underlying [`io::Error`] if the tree, a parent directory, a
/// copy, or the policy write fails.
pub(crate) fn build_shadow_home(
    provider: Provider,
    real_home: Option<&Path>,
) -> io::Result<TempDir> {
    let dir = tempfile::Builder::new()
        .prefix("claudine-contract-home-")
        .tempdir()?;

    if let Some(home) = real_home {
        for rel in auth_files(provider) {
            let src = home.join(rel);
            if src.is_file() {
                let dst = dir.path().join(rel);
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src, &dst)?;
            }
        }
    }

    if let Some((rel, content)) = policy_file(provider) {
        let dst = dir.path().join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dst, content)?;
    }

    Ok(dir)
}
