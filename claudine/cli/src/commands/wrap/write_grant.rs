//! The narrowest launch posture that lets an inline agent write its document.
//!
//! An `inline-compose` agent works on the file itself, so the provider must be
//! launched in a shape where it may edit that one document without an
//! operator approving the write. Every provider spells that differently: an
//! edit-accepting approval mode (Claude, Gemini, Qwen, Antigravity), a
//! writable sandbox (Codex), an approval environment (Goose), an inline
//! permission overlay (OpenCode), or nothing at all (Pi, Kimi's wire mode).
//! When the document lies outside the provider's workspace, the provider's
//! own additional-root mechanism is added; a provider with no such mechanism
//! refuses before spawn.
//!
//! Three rules hold for every provider:
//!
//! - **Never widen to bypass.** The grant is the edit-accepting posture, not
//!   YOLO. When `--yolo` already applied, nothing is added except a workspace
//!   scope the provider's file tools still enforce under bypass.
//! - **Explicit denies win.** A caller who pinned a plan/read-only mode, a
//!   deny rule, or a restrictive approval environment gets a typed refusal
//!   that names the denial, never a silently widened launch.
//! - **Paths are native.** The document and its parent directory travel as
//!   the native strings the provider's tools accept (see
//!   [`claudine::composition::native_document_path`]); nothing is escaped.
//!
//! The planner is a pure function of its inputs and is table-tested across
//! macOS, Linux, and Windows path shapes without launching a provider.
//!
//! ## Provider notes
//!
//! - Kimi's non-interactive `--wire` session applies the runtime AFK
//!   auto-approval its print mode documents, so only the workspace scope is
//!   added; a pinned `--plan` is a denial.
//! - Kilo has no session-scoped `external_directory` grant, so a document
//!   outside its worktree is unsupported unless bypass applied.
//! - Goose has no path scoping; `GOOSE_MODE=auto` is its only non-interactive
//!   posture that executes a file edit, and it is the provider's own default.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use claudine::composition::native_document_path;
use claudine::provider::Provider;

/// What the planner needs to know about one launch.
pub(crate) struct WriteGrantRequest<'a> {
    pub(crate) provider: Provider,
    /// Native absolute path of the active document.
    pub(crate) document: &'a Path,
    /// The child working directory the provider treats as its workspace.
    pub(crate) workspace: &'a Path,
    pub(crate) non_interactive: bool,
    /// Whether the provider's bypass mechanism actually applied.
    pub(crate) yolo_applied: bool,
    /// The provider argv assembled so far (forwarded tail plus Claudine's
    /// earlier injections), read for pinned modes and deny rules.
    pub(crate) args: &'a [String],
    /// The child environment as it stands, read for approval-mode variables.
    pub(crate) env: &'a HashMap<OsString, OsString>,
}

/// The posture one launch grants: argv, environment, and (for OpenCode) the
/// permission overlay folded into `OPENCODE_CONFIG_CONTENT`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WriteGrant {
    /// Stable facet label recorded in the session-compatibility key, so a
    /// retry or resume whose grant moved is refused rather than mixed.
    pub(crate) posture: String,
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
    /// OpenCode `permission` overlay, when the document is outside the
    /// workspace.
    pub(crate) opencode_permission: Option<serde_json::Value>,
}

/// Why no writable launch shape exists for this provider and document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WriteGrantError {
    /// The caller explicitly pinned a posture that cannot write.
    Denied {
        provider: Provider,
        document: PathBuf,
        /// The flag, value, or environment assignment that denies the write.
        denial: String,
    },
    /// The provider has no safe launch shape for this document.
    Unsupported {
        provider: Provider,
        document: PathBuf,
        /// The missing capability, in the provider's own vocabulary.
        capability: String,
    },
}

impl std::fmt::Display for WriteGrantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied {
                provider,
                document,
                denial,
            } => write!(
                f,
                "{provider} cannot write the inline document {} because the launch \
                 explicitly denies file edits ({denial}); remove that setting or run the \
                 document with `claudine compose` instead",
                native_document_path(document)
            ),
            Self::Unsupported {
                provider,
                document,
                capability,
            } => write!(
                f,
                "{provider} has no launch shape that can write the inline document {}: \
                 it lacks {capability}; move the document inside the workspace, choose \
                 another provider, or run with --yolo",
                native_document_path(document)
            ),
        }
    }
}

impl std::error::Error for WriteGrantError {}

/// Plan the minimum writable posture for one launch.
///
/// ## Errors
///
/// [`WriteGrantError::Denied`] when the launch already pins a posture that
/// cannot write the document; [`WriteGrantError::Unsupported`] when the
/// provider has no shape that reaches the document safely.
pub(crate) fn plan_write_grant(
    request: &WriteGrantRequest<'_>,
) -> Result<WriteGrant, WriteGrantError> {
    let provider = request.provider;
    let document = native_document_path(request.document);
    let workspace = native_document_path(request.workspace);
    let inside = document_within_workspace(&document, &workspace);
    let root = native_parent(&document);
    let denied = |denial: String| WriteGrantError::Denied {
        provider,
        document: request.document.to_path_buf(),
        denial,
    };
    let unsupported = |capability: &str| WriteGrantError::Unsupported {
        provider,
        document: request.document.to_path_buf(),
        capability: capability.to_string(),
    };
    // Approval-accepting modes are only needed where no operator can answer a
    // prompt; an interactive session keeps the provider's own asking posture.
    let accept_edits = request.non_interactive && !request.yolo_applied;
    let mut grant = WriteGrant::default();
    let mut posture: Vec<String> = Vec::new();
    if request.yolo_applied {
        posture.push("bypass".to_string());
    }

    match provider {
        Provider::Claude => {
            if let Some(mode) = option_value(request.args, "--permission-mode") {
                if mode == "plan" {
                    return Err(denied(format!("--permission-mode {mode}")));
                }
                posture.push(format!("permission-mode={mode}"));
            } else if accept_edits {
                grant.args.extend(["--permission-mode".into(), "acceptEdits".into()]);
                posture.push("accept-edits".to_string());
            }
            for flag in ["--disallowedTools", "--disallowed-tools"] {
                if let Some(tools) = option_value(request.args, flag)
                    && names_edit_tool(&tools)
                {
                    return Err(denied(format!("{flag} {tools}")));
                }
            }
            if !inside {
                grant.args.extend(["--add-dir".into(), root.clone()]);
                posture.push(format!("add-dir={root}"));
            }
        }
        Provider::Codex => {
            let pinned = option_value(request.args, "--sandbox")
                .or_else(|| option_value(request.args, "-s"));
            match pinned.as_deref() {
                Some("read-only") => return Err(denied("--sandbox read-only".to_string())),
                Some(mode) => posture.push(format!("sandbox={mode}")),
                None if request.yolo_applied => {}
                None => {
                    grant
                        .args
                        .extend(["--sandbox".into(), "workspace-write".into()]);
                    posture.push("workspace-write".to_string());
                }
            }
            if !inside && !request.yolo_applied {
                grant.args.extend(["--add-dir".into(), root.clone()]);
                posture.push(format!("add-dir={root}"));
            }
        }
        Provider::Gemini => approval_mode_grant(
            request,
            "auto_edit",
            accept_edits,
            inside,
            &root,
            &mut grant,
            &mut posture,
            &denied,
        )?,
        Provider::QwenCode => approval_mode_grant(
            request,
            "auto-edit",
            accept_edits,
            inside,
            &root,
            &mut grant,
            &mut posture,
            &denied,
        )?,
        Provider::Goose => {
            let mode = request
                .env
                .get(OsString::from("GOOSE_MODE").as_os_str())
                .map(|value| value.to_string_lossy().into_owned());
            match mode.as_deref() {
                Some(pinned @ ("chat" | "approve" | "smart_approve")) if accept_edits => {
                    return Err(denied(format!("GOOSE_MODE={pinned}")));
                }
                Some(pinned) => posture.push(format!("goose-mode={pinned}")),
                None if accept_edits => {
                    grant.env.push(("GOOSE_MODE".into(), "auto".into()));
                    posture.push("goose-mode=auto".to_string());
                }
                None => {}
            }
        }
        Provider::KimiCode => {
            if has_flag(request.args, "--plan") {
                return Err(denied("--plan".to_string()));
            }
            if request.non_interactive && !request.yolo_applied {
                posture.push("wire-afk".to_string());
            }
            if !inside {
                grant.args.extend(["--add-dir".into(), root.clone()]);
                posture.push(format!("add-dir={root}"));
            }
        }
        Provider::OpenCode => {
            if let Some(denial) = opencode_permission_denies_edit(request.env) {
                return Err(denied(denial));
            }
            if inside {
                posture.push("default".to_string());
            } else if accept_edits {
                grant.opencode_permission = Some(serde_json::json!({
                    "permission": {
                        "external_directory": {
                            root.clone(): "allow",
                            format!("{root}{}*", separator_for(&root)): "allow",
                        }
                    }
                }));
                posture.push(format!("external-directory={root}"));
            } else if request.yolo_applied {
                // Bypass auto-approves the external-directory ask.
            } else {
                posture.push("external-directory=ask".to_string());
            }
        }
        Provider::Kilo => {
            if inside {
                posture.push("default".to_string());
            } else if !request.yolo_applied {
                return Err(unsupported(
                    "a session-scoped `external_directory` grant for a document outside its worktree",
                ));
            }
        }
        Provider::Pi => posture.push("unrestricted".to_string()),
        Provider::Antigravity => {
            if let Some(mode) = option_value(request.args, "--mode") {
                if mode == "plan" {
                    return Err(denied(format!("--mode {mode}")));
                }
                posture.push(format!("mode={mode}"));
            } else if accept_edits {
                grant
                    .args
                    .extend(["--mode".into(), "accept-edits".into()]);
                posture.push("accept-edits".to_string());
            }
            if !inside {
                grant.args.extend(["--add-dir".into(), root.clone()]);
                posture.push(format!("add-dir={root}"));
            }
        }
        // `Provider` is `#[non_exhaustive]`: a variant this table does not know
        // has no proven write posture, which is a refusal, never a guess.
        _ => return Err(unsupported("a known inline write posture")),
    }

    if posture.is_empty() {
        posture.push(if request.non_interactive {
            "default".to_string()
        } else {
            "interactive".to_string()
        });
    }
    grant.posture = format!("{}:{}", provider.as_slug(), posture.join("+"));
    Ok(grant)
}

/// The Gemini/Qwen shape: an `--approval-mode` edit-accepting mode plus
/// `--include-directories` for a document outside the workspace, with a pinned
/// `plan` (or headless `default`) mode and an edit-tool exclusion as denials.
#[allow(clippy::too_many_arguments)]
fn approval_mode_grant(
    request: &WriteGrantRequest<'_>,
    auto_edit: &str,
    accept_edits: bool,
    inside: bool,
    root: &str,
    grant: &mut WriteGrant,
    posture: &mut Vec<String>,
    denied: &dyn Fn(String) -> WriteGrantError,
) -> Result<(), WriteGrantError> {
    if let Some(mode) = option_value(request.args, "--approval-mode") {
        let lowered = mode.to_ascii_lowercase();
        if lowered == "plan" || (lowered == "default" && request.non_interactive) {
            return Err(denied(format!("--approval-mode {mode}")));
        }
        posture.push(format!("approval-mode={lowered}"));
    } else if accept_edits {
        grant
            .args
            .extend(["--approval-mode".into(), auto_edit.into()]);
        posture.push(auto_edit.to_string());
    }
    if let Some(tools) = option_value(request.args, "--exclude-tools")
        && names_edit_tool(&tools)
    {
        return Err(denied(format!("--exclude-tools {tools}")));
    }
    if !inside {
        grant
            .args
            .extend(["--include-directories".into(), root.to_string()]);
        posture.push(format!("include-directories={root}"));
    }
    Ok(())
}

/// Whether `document` lies under `workspace`, judged on the native spelling
/// of both.
///
/// Windows-shaped paths (a drive letter or a UNC root) compare
/// case-insensitively with either separator and without a verbatim prefix;
/// POSIX paths compare exactly. Both inputs must share a shape — a POSIX
/// document is never inside a Windows workspace.
pub(crate) fn document_within_workspace(document: &str, workspace: &str) -> bool {
    let (document_shape, document_parts) = path_components(document);
    let (workspace_shape, workspace_parts) = path_components(workspace);
    document_shape == workspace_shape
        && document_parts.len() > workspace_parts.len()
        && document_parts.starts_with(&workspace_parts)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathShape {
    Posix,
    Windows,
}

fn path_components(path: &str) -> (PathShape, Vec<String>) {
    let windows_shaped = is_windows_shaped(path);
    if windows_shaped {
        let text = path.strip_prefix(r"\\?\UNC\").map_or_else(
            || path.strip_prefix(r"\\?\").unwrap_or(path).to_string(),
            |rest| format!(r"\\{rest}"),
        );
        let parts = text
            .split(['\\', '/'])
            .filter(|part| !part.is_empty())
            .map(str::to_ascii_lowercase)
            .collect();
        (PathShape::Windows, parts)
    } else {
        let parts = path
            .split('/')
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        (PathShape::Posix, parts)
    }
}

/// The parent directory of a native path, judged on its own shape so a
/// Windows document is scoped correctly on every host.
fn native_parent(path: &str) -> String {
    let split = if is_windows_shaped(path) {
        path.rfind(['\\', '/'])
    } else {
        path.rfind('/')
    };
    match split {
        Some(0) => path[..1].to_string(),
        Some(index) => path[..index].to_string(),
        None => path.to_string(),
    }
}

fn is_windows_shaped(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with(r"\\")
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

fn separator_for(native_root: &str) -> char {
    if is_windows_shaped(native_root) {
        '\\'
    } else {
        '/'
    }
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

/// The value of `--flag value` or `--flag=value`, when present.
fn option_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix(flag).and_then(|rest| rest.strip_prefix('=')) {
            return Some(value.to_string());
        }
    }
    None
}

/// Whether a comma/space separated tool list names a file-editing tool.
fn names_edit_tool(tools: &str) -> bool {
    tools
        .split([',', ' '])
        .map(str::trim)
        .any(|tool| {
            let lowered = tool.to_ascii_lowercase();
            let name = lowered.split('(').next().unwrap_or(&lowered);
            matches!(
                name,
                "edit" | "write" | "writefile" | "write_file" | "replace" | "notebookedit"
            )
        })
}

/// The `OPENCODE_PERMISSION` overlay in the environment denies `edit` when it
/// says so directly or through a `*` catch-all with no `edit` allowance.
fn opencode_permission_denies_edit(env: &HashMap<OsString, OsString>) -> Option<String> {
    let raw = env
        .get(OsString::from("OPENCODE_PERMISSION").as_os_str())?
        .to_string_lossy()
        .into_owned();
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let map = value.as_object()?;
    let edit = map.get("edit").and_then(serde_json::Value::as_str);
    let catch_all = map.get("*").and_then(serde_json::Value::as_str);
    match (edit, catch_all) {
        (Some("deny"), _) => Some(r#"OPENCODE_PERMISSION {"edit":"deny"}"#.to_string()),
        (None, Some("deny")) => Some(r#"OPENCODE_PERMISSION {"*":"deny"}"#.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
