//! Table-driven coverage of the write-grant planner: every supported provider,
//! each path shape (POSIX, Windows drive, Windows verbatim, UNC), inside and
//! outside the workspace, interactive and non-interactive, bypass, pinned
//! denies, and the OpenCode overlay — all without launching a provider.

use super::*;
use claudine::provider::PROVIDERS_DISPLAY_ORDER;

/// One expectation row: provider, expected argv, expected env, and either
/// the expected posture or the expected denial text.
type Row = (
    Provider,
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
    &'static str,
);

struct Case {
    provider: Provider,
    document: &'static str,
    workspace: &'static str,
    non_interactive: bool,
    yolo_applied: bool,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

impl Case {
    fn new(provider: Provider, document: &'static str, workspace: &'static str) -> Self {
        Self {
            provider,
            document,
            workspace,
            non_interactive: true,
            yolo_applied: false,
            args: &[],
            env: &[],
        }
    }

    fn interactive(mut self) -> Self {
        self.non_interactive = false;
        self
    }

    fn bypass(mut self) -> Self {
        self.yolo_applied = true;
        self
    }

    fn args(mut self, args: &'static [&'static str]) -> Self {
        self.args = args;
        self
    }

    fn env(mut self, env: &'static [(&'static str, &'static str)]) -> Self {
        self.env = env;
        self
    }

    fn plan(&self) -> Result<WriteGrant, WriteGrantError> {
        let args: Vec<String> = self.args.iter().map(|arg| (*arg).to_string()).collect();
        let env: HashMap<OsString, OsString> = self
            .env
            .iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect();
        plan_write_grant(&WriteGrantRequest {
            provider: self.provider,
            document: Path::new(self.document),
            workspace: Path::new(self.workspace),
            non_interactive: self.non_interactive,
            yolo_applied: self.yolo_applied,
            args: &args,
            env: &env,
        })
    }
}

fn args_of(grant: &WriteGrant) -> Vec<&str> {
    grant.args.iter().map(String::as_str).collect()
}

const POSIX_WS: &str = "/Users/ken/repo";
const POSIX_INSIDE: &str = "/Users/ken/repo/docs/my voip.md";
const POSIX_OUTSIDE: &str = "/Users/ken/other docs/voip.md";
const WIN_WS: &str = r"C:\Users\Ken\repo";
const WIN_INSIDE: &str = r"C:\Users\Ken\repo\docs\my voip.md";
const WIN_OUTSIDE: &str = r"D:\Other Docs\voip.md";
const WIN_VERBATIM_INSIDE: &str = r"\\?\C:\Users\Ken\repo\docs\voip.md";
const UNC_WS: &str = r"\\nas\share\repo";
const UNC_OUTSIDE: &str = r"\\nas\share\elsewhere\voip.md";

#[test]
fn containment_follows_the_native_path_shape() {
    assert!(document_within_workspace(POSIX_INSIDE, POSIX_WS));
    assert!(!document_within_workspace(POSIX_OUTSIDE, POSIX_WS));
    assert!(!document_within_workspace("/Users/ken/repo2/x.md", POSIX_WS), "prefix is not containment");
    assert!(!document_within_workspace(POSIX_WS, POSIX_WS), "the workspace itself is not a document");
    assert!(document_within_workspace(WIN_INSIDE, WIN_WS));
    assert!(document_within_workspace(WIN_VERBATIM_INSIDE, WIN_WS), "verbatim prefix ignored");
    assert!(document_within_workspace(r"c:/users/ken/REPO/docs/x.md", WIN_WS), "case and separator insensitive");
    assert!(!document_within_workspace(WIN_OUTSIDE, WIN_WS));
    assert!(document_within_workspace(r"\\nas\share\repo\a.md", UNC_WS));
    assert!(!document_within_workspace(UNC_OUTSIDE, UNC_WS));
    assert!(!document_within_workspace(POSIX_INSIDE, WIN_WS), "shapes never mix");
    assert!(!document_within_workspace("/users/ken/repo/x.md", POSIX_WS), "POSIX is case-sensitive");
}

/// Every supported provider has a row for both path families, and the grant
/// carries the native root verbatim (spaces, backslashes, drive letters).
#[test]
fn every_provider_plans_inside_and_outside_on_every_path_shape() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        for (inside, outside, workspace, root) in [
            (POSIX_INSIDE, POSIX_OUTSIDE, POSIX_WS, "/Users/ken/other docs"),
            (WIN_INSIDE, WIN_OUTSIDE, WIN_WS, r"D:\Other Docs"),
        ] {
            let inside_grant = Case::new(provider, inside, workspace)
                .plan()
                .unwrap_or_else(|e| panic!("{provider} inside {inside}: {e}"));
            assert!(
                !inside_grant.args.iter().any(|arg| arg == root),
                "{provider}: no scope needed inside the workspace: {:?}",
                inside_grant.args
            );
            assert!(
                inside_grant.posture.starts_with(&format!("{}:", provider.as_slug())),
                "{provider}: posture names the provider: {}",
                inside_grant.posture
            );

            match Case::new(provider, outside, workspace).plan() {
                Ok(grant) => {
                    let scope_flag = match provider {
                        Provider::Claude
                        | Provider::Codex
                        | Provider::KimiCode
                        | Provider::Antigravity => Some("--add-dir"),
                        Provider::Gemini | Provider::QwenCode => Some("--include-directories"),
                        _ => None,
                    };
                    match scope_flag {
                        Some(flag) => {
                            let position = grant
                                .args
                                .iter()
                                .position(|arg| arg == flag)
                                .unwrap_or_else(|| panic!("{provider}: {flag} for {outside}: {:?}", grant.args));
                            assert_eq!(grant.args[position + 1], root, "{provider}: native root verbatim");
                            assert!(
                                !grant.args[position + 1].contains("\\\\"),
                                "{provider}: backslashes must not be doubled"
                            );
                            assert!(grant.posture.contains(root), "{provider}: {}", grant.posture);
                        }
                        None if provider == Provider::OpenCode => {
                            let overlay = grant.opencode_permission.expect("OpenCode external-directory overlay");
                            let external = &overlay["permission"]["external_directory"];
                            assert_eq!(external[root], "allow", "{overlay}");
                            let glob = if root.contains('\\') { format!("{root}\\*") } else { format!("{root}/*") };
                            assert_eq!(external[glob.as_str()], "allow", "{overlay}");
                        }
                        None => assert!(grant.args.is_empty(), "{provider}: {:?}", grant.args),
                    }
                }
                Err(WriteGrantError::Unsupported { provider: p, capability, .. }) => {
                    assert_eq!(p, Provider::Kilo, "only Kilo lacks an outside-root grant: {capability}");
                    assert!(capability.contains("external_directory"));
                }
                Err(other) => panic!("{provider}: unexpected {other}"),
            }
        }
    }
}

#[test]
fn non_interactive_edit_accepting_postures_are_provider_native() {
    let rows: &[Row] = &[
        (Provider::Claude, &["--permission-mode", "acceptEdits"], &[], "claude:accept-edits"),
        (Provider::Codex, &["--sandbox", "workspace-write"], &[], "codex:workspace-write"),
        (Provider::Gemini, &["--approval-mode", "auto_edit"], &[], "gemini:auto_edit"),
        (Provider::QwenCode, &["--approval-mode", "auto-edit"], &[], "qwen:auto-edit"),
        (Provider::Goose, &[], &[("GOOSE_MODE", "auto")], "goose:goose-mode=auto"),
        (Provider::KimiCode, &[], &[], "kimi:wire-afk"),
        (Provider::OpenCode, &[], &[], "opencode:default"),
        (Provider::Kilo, &[], &[], "kilo:default"),
        (Provider::Pi, &[], &[], "pi:unrestricted"),
        (Provider::Antigravity, &["--mode", "accept-edits"], &[], "antigravity:accept-edits"),
    ];
    assert_eq!(rows.len(), PROVIDERS_DISPLAY_ORDER.len(), "one row per provider");
    for (provider, args, env, posture) in rows {
        let grant = Case::new(*provider, POSIX_INSIDE, POSIX_WS).plan().unwrap();
        assert_eq!(args_of(&grant), *args, "{provider}");
        let env_pairs: Vec<(&str, &str)> = grant.env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(env_pairs, *env, "{provider}");
        assert_eq!(grant.posture, *posture, "{provider}");
        assert!(grant.opencode_permission.is_none(), "{provider}");
    }
}

#[test]
fn bypass_adds_no_approval_posture_but_keeps_workspace_scope() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let grant = Case::new(provider, POSIX_INSIDE, POSIX_WS).bypass().plan().unwrap();
        assert!(grant.args.is_empty() && grant.env.is_empty(), "{provider}: {grant:?}");
        assert!(grant.posture.contains("bypass"), "{provider}: {}", grant.posture);
    }
    // Gemini's file tools enforce the workspace even under yolo, so the scope
    // still travels; Codex's full-access bypass needs none.
    let gemini = Case::new(Provider::Gemini, POSIX_OUTSIDE, POSIX_WS).bypass().plan().unwrap();
    assert_eq!(args_of(&gemini), ["--include-directories", "/Users/ken/other docs"]);
    let codex = Case::new(Provider::Codex, POSIX_OUTSIDE, POSIX_WS).bypass().plan().unwrap();
    assert!(codex.args.is_empty(), "{:?}", codex.args);
    let kilo = Case::new(Provider::Kilo, POSIX_OUTSIDE, POSIX_WS).bypass().plan().unwrap();
    assert_eq!(kilo.posture, "kilo:bypass");
    let opencode = Case::new(Provider::OpenCode, POSIX_OUTSIDE, POSIX_WS).bypass().plan().unwrap();
    assert!(opencode.opencode_permission.is_none());
}

#[test]
fn interactive_sessions_keep_the_asking_posture_but_still_scope_the_root() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let grant = Case::new(provider, POSIX_INSIDE, POSIX_WS).interactive().plan().unwrap();
        if provider == Provider::Codex {
            // A sandbox is isolation, not approval: an operator cannot approve
            // a write out of a read-only sandbox, so Codex keeps its grant.
            assert_eq!(args_of(&grant), ["--sandbox", "workspace-write"]);
            continue;
        }
        assert!(grant.args.is_empty() && grant.env.is_empty(), "{provider}: {grant:?}");
    }
    let claude = Case::new(Provider::Claude, WIN_OUTSIDE, WIN_WS).interactive().plan().unwrap();
    assert_eq!(args_of(&claude), ["--add-dir", r"D:\Other Docs"]);
    let opencode = Case::new(Provider::OpenCode, POSIX_OUTSIDE, POSIX_WS).interactive().plan().unwrap();
    assert!(opencode.opencode_permission.is_none(), "the operator answers the ask");
    assert_eq!(opencode.posture, "opencode:external-directory=ask");
}

#[test]
fn explicit_denies_refuse_before_spawn_and_are_never_widened() {
    let rows: &[Row] = &[
        (Provider::Claude, &["--permission-mode", "plan"], &[], "--permission-mode plan"),
        (Provider::Claude, &["--disallowedTools", "Bash,Edit"], &[], "--disallowedTools Bash,Edit"),
        (Provider::Codex, &["--sandbox", "read-only"], &[], "--sandbox read-only"),
        (Provider::Codex, &["-s", "read-only"], &[], "--sandbox read-only"),
        (Provider::Gemini, &["--approval-mode=plan"], &[], "--approval-mode plan"),
        (Provider::Gemini, &["--approval-mode", "default"], &[], "--approval-mode default"),
        (Provider::QwenCode, &["--exclude-tools", "WriteFile,Edit"], &[], "--exclude-tools WriteFile,Edit"),
        (Provider::Goose, &[], &[("GOOSE_MODE", "chat")], "GOOSE_MODE=chat"),
        (Provider::Goose, &[], &[("GOOSE_MODE", "smart_approve")], "GOOSE_MODE=smart_approve"),
        (Provider::KimiCode, &["--plan"], &[], "--plan"),
        (Provider::OpenCode, &[], &[("OPENCODE_PERMISSION", r#"{"edit":"deny"}"#)], r#"OPENCODE_PERMISSION {"edit":"deny"}"#),
        (Provider::OpenCode, &[], &[("OPENCODE_PERMISSION", r#"{"*":"deny"}"#)], r#"OPENCODE_PERMISSION {"*":"deny"}"#),
        (Provider::Antigravity, &["--mode", "plan"], &[], "--mode plan"),
    ];
    for (provider, args, env, expected) in rows {
        let err = Case::new(*provider, POSIX_INSIDE, POSIX_WS)
            .args(args)
            .env(env)
            .plan()
            .expect_err(&format!("{provider} {args:?} {env:?} must refuse"));
        let WriteGrantError::Denied { provider: p, document, denial } = &err else {
            panic!("{provider}: expected Denied, got {err:?}");
        };
        assert_eq!(p, provider);
        assert_eq!(document, Path::new(POSIX_INSIDE));
        assert_eq!(denial, expected, "{provider}");
        let text = err.to_string();
        assert!(text.contains(&provider.to_string()) && text.contains(POSIX_INSIDE) && text.contains(expected), "{text}");
        assert!(!text.contains("yolo") || text.contains("--yolo"), "no silent widening: {text}");
    }
}

#[test]
fn a_caller_pinned_writable_mode_is_kept_rather_than_duplicated() {
    let claude = Case::new(Provider::Claude, POSIX_INSIDE, POSIX_WS)
        .args(&["--permission-mode", "acceptEdits"])
        .plan()
        .unwrap();
    assert!(claude.args.is_empty(), "{:?}", claude.args);
    assert_eq!(claude.posture, "claude:permission-mode=acceptEdits");

    let codex = Case::new(Provider::Codex, POSIX_OUTSIDE, POSIX_WS)
        .args(&["--sandbox", "danger-full-access"])
        .plan()
        .unwrap();
    assert_eq!(args_of(&codex), ["--add-dir", "/Users/ken/other docs"]);
    assert_eq!(codex.posture, "codex:sandbox=danger-full-access+add-dir=/Users/ken/other docs");

    let goose = Case::new(Provider::Goose, POSIX_INSIDE, POSIX_WS)
        .env(&[("GOOSE_MODE", "auto")])
        .plan()
        .unwrap();
    assert!(goose.env.is_empty(), "already auto");
    assert_eq!(goose.posture, "goose:goose-mode=auto");

    let opencode = Case::new(Provider::OpenCode, POSIX_INSIDE, POSIX_WS)
        .env(&[("OPENCODE_PERMISSION", r#"{"*":"deny","edit":"allow"}"#)])
        .plan()
        .unwrap();
    assert_eq!(opencode.posture, "opencode:default");
}

#[test]
fn kilo_outside_its_worktree_is_a_typed_unsupported_capability() {
    let err = Case::new(Provider::Kilo, UNC_OUTSIDE, UNC_WS).plan().unwrap_err();
    let WriteGrantError::Unsupported { provider, document, capability } = &err else {
        panic!("{err:?}");
    };
    assert_eq!(*provider, Provider::Kilo);
    assert_eq!(document, Path::new(UNC_OUTSIDE));
    assert!(capability.contains("external_directory"));
    let text = err.to_string();
    assert!(text.contains("Kilo") && text.contains(r"\\nas\share\elsewhere\voip.md") && text.contains("--yolo"), "{text}");
}

#[test]
fn windows_verbatim_documents_are_granted_with_the_legacy_spelling() {
    let grant = Case::new(Provider::Claude, r"\\?\D:\Other Docs\voip.md", WIN_WS).plan().unwrap();
    assert_eq!(
        args_of(&grant),
        ["--permission-mode", "acceptEdits", "--add-dir", r"D:\Other Docs"]
    );
}
