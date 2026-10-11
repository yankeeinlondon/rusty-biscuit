//! One compose run asks the login shell about a name at most once.
//!
//! Preflight collection, the pre-approval check, and execution each resolve
//! a name that is not on PATH as a possible alias. The stub login shell logs
//! every start, so the test counts how often each name was looked up across a
//! full preflight-plus-compose run, driven the way `md compose` drives it.
//!
//! The missing name sits where preflight sees it but execution never runs it
//! (a false page block, an untaken ternary branch): run, it would fail the
//! compose as not found, and handled-failure fallbacks do not cover that.
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ShellExpansionError};

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn lookups(log: &Path, name: &str) -> usize {
    let query = format!("alias {name}");
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| *line == query)
        .count()
}

#[test]
fn a_compose_run_looks_each_name_up_once() {
    use std::os::unix::fs::PermissionsExt;

    let started = Instant::now();
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("lookups.log");
    let shell = dir.path().join("stub-shell");
    // `$2` is the `alias <name>` query; only `dm-memo-alias` is an alias.
    std::fs::write(
        &shell,
        format!(
            "#!/bin/sh\necho \"$2\" >> '{}'\nif [ \"$2\" = 'alias dm-memo-alias' ]; then echo \"dm-memo-alias='echo aliased'\"; else exit 1; fi\n",
            log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();

    let source = dir.path().join("doc.md");
    let content = "---\nfm_alias: $(dm-memo-alias x)\nfm_missing: $( false ? dm-memo-missing x : echo skipped )\n---\n\
                   ::shell dm-memo-alias x\n\n\
                   ::block when=\"false\"\n::shell dm-memo-missing x\n::end-block\n\n\
                   Frontmatter: {{ fm_alias }} / {{ fm_missing }}\n";
    std::fs::write(&source, content).unwrap();
    let document = Markdown::try_from(source.as_path()).unwrap();

    let mut context = ComposeContext::capture_for_content(dir.path(), content);
    context.env_mut().insert("SHELL".to_string(), shell.to_string_lossy().into_owned());
    let options = ComposeOptions::new_with_context(context)
        .with_source_file(&source)
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(dir.path());
    let request = crate::request_support::request(options);

    let approvals = document
        .compose_preflight_approvals(&request, Some(Arc::new(AllowAll)))
        .expect("preflight approvals");
    let request = request.map_options(|options| {
        options
            .with_pre_approved_commands(approvals.pre_approved_commands)
            .with_preflight_graph(approvals.preflight_graph)
    });
    let (composed, _report) = document.compose_with(&request).expect("compose succeeds");

    let output = composed.content();
    assert_eq!(output.matches("aliased x").count(), 2, "{output}");
    assert!(!output.contains("dm-memo-missing"), "{output}");
    assert!(output.contains("Frontmatter: aliased x / skipped"), "{output}");
    assert_eq!(lookups(&log, "dm-memo-alias"), 1, "alias lookups");
    assert_eq!(lookups(&log, "dm-memo-missing"), 1, "missing-name lookups");
    assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
}
