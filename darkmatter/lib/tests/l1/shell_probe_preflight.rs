//! Shell-command preflight never launches a login-shell profile (R8), while
//! the terminal compose pass does.
//!
//! The login shell is a stub named `bash` that logs each launch, taken from
//! the request's captured environment rather than the process's.
#![cfg(unix)]

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};

fn launches(log: &Path) -> usize {
    std::fs::read_to_string(log).map(|text| text.lines().count()).unwrap_or(0)
}

#[test]
fn preflight_discovery_does_not_launch_the_login_shell() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("launches.log");
    let shell = dir.path().join("bash");
    std::fs::write(
        &shell,
        format!("#!/bin/sh\necho launched >> '{}'\necho darkmatter-shell-probe:alias\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();

    let content = "alias={{ has_alias(\"ll\") }}\n";
    let document = Markdown::from(content);
    let mut context = ComposeContext::capture_for_content(dir.path(), content);
    context.env_mut().insert("SHELL".to_string(), shell.to_string_lossy().into_owned());
    let options = ComposeOptions::new_with_context(context).with_source_file(dir.path().join("doc.md"));

    document.compose_preflight(&options).expect("preflight succeeds");
    assert_eq!(launches(&log), 0, "preflight launched the login shell");

    let (composed, _) = document.compose_with(options).expect("compose succeeds");
    assert!(composed.content().contains("alias=true"), "{}", composed.content());
    assert_eq!(launches(&log), 1, "only the terminal pass probes the request's login shell");
}
