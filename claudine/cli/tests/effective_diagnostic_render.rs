//! End-to-end proof that the CLI renders the **effective diagnostic**.
//!
//! Feature: `claudine/features/2026-07-13-error-propogation/` (Phase 5).
//!
//! Where `characterization_error_routes.rs` pins what the migration must *not*
//! change (exit code, event order, emission count), this file asserts what it
//! exists to change: a typed failure reaches the user as a rendered
//! `StatusBlock` carrying actionable content, instead of the generic `Error:`
//! line it produced before the walker resolved through
//! `claudine::diagnostics::select_effective_diagnostic`.
//!
//! These drive the real binary through the normal invocation path — the fix is
//! in the CLI's top-level render boundary, so a unit test on the walker cannot
//! prove the wiring reaches it.
//!
//! ## Placement
//!
//! Un-prefixed (runs under `just test`): every route here fails headlessly, so
//! no TTY, tmux, or PTY is needed. Phase 7 adds the L2 coverage for the surfaces
//! whose output contract depends on a real terminal (TTY vs `NO_COLOR` vs
//! `FORCE_COLOR`, OSC 8 links, SGR).

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::{TestWorkspace, augmented_path, strip_ansi, write, write_executable};

#[cfg(windows)]
fn shim_name(name: &str) -> String {
    format!("{name}.cmd")
}

#[cfg(not(windows))]
fn shim_name(name: &str) -> String {
    name.to_string()
}

#[cfg(windows)]
const PROVIDER_SHIM: &str = "@echo off\r\nexit /b 0\r\n";

#[cfg(not(windows))]
const PROVIDER_SHIM: &str = "#!/bin/sh\nexit 0\n";

/// A provider that fails, so the `failure` lifecycle stack runs.
#[cfg(windows)]
fn failing_provider_shim() -> String {
    "@echo off\r\nexit /b 3\r\n".to_string()
}

#[cfg(not(windows))]
fn failing_provider_shim() -> String {
    "#!/bin/sh\nexit 3\n".to_string()
}

/// Run `claudine compose --claude <doc>` in an isolated workspace, returning
/// plain (escape-stripped) stderr.
fn compose_stderr(doc_body: &str, extra_args: &[&str]) -> String {
    let workspace = TestWorkspace::named("claudine-effective-diagnostic");
    let root = workspace.path();
    let bin_dir = root.join("bin");
    write_executable(&bin_dir.join(shim_name("claude")), PROVIDER_SHIM);

    let doc = root.join("route.md");
    write(&doc, doc_body);

    let mut args: Vec<&str> = vec!["compose", "--claude"];
    args.extend_from_slice(extra_args);
    let doc_arg = doc.to_str().unwrap().to_string();
    args.push(&doc_arg);

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(root)
        .args(&args)
        .output()
        .expect("spawn claudine");

    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

/// A rendered `StatusBlock`, detected structurally: an error glyph line
/// followed by a `┃` body line. The glyph alone is not enough — the wrapper
/// prints ordinary status lines with it too.
fn has_status_block(stderr: &str) -> bool {
    let lines: Vec<&str> = stderr.lines().collect();
    lines.windows(2).any(|pair| {
        pair[0].trim_start().starts_with('\u{292B}') && pair[1].trim_start().starts_with('\u{2503}')
    })
}

fn has_generic_error_line(stderr: &str) -> bool {
    stderr
        .lines()
        .any(|line| line.trim_start().starts_with("Error:"))
}

/// The motivating incident (spec §"Motivating Incident").
///
/// Before Phase 5 this reported:
///
/// ```text
/// Error: lifecycle initialize proxy: path resolution failed for "…": proxy target does not exist: …
/// ```
///
/// The typed `HarnessError` was in the chain the whole time — the walker's own
/// downcast list just could not see it.
#[test]
fn initialize_proxy_resolution_failure_renders_a_status_block() {
    let stderr = compose_stderr(
        r#"---
title: motivating incident
initialize:
  stack:
    - action: {proxy: "no/such/target.md"}
---
Body
"#,
        &[],
    );

    assert!(
        has_status_block(&stderr),
        "the motivating failure must render a StatusBlock; stderr:\n{stderr}"
    );
    assert!(
        !has_generic_error_line(&stderr),
        "the motivating failure must not fall back to the generic `Error:` line; \
         stderr:\n{stderr}"
    );

    // Actionable content, not a `to_string()` substring: the block must name
    // the authored reference, the surface that authored it, and the contract it
    // violated.
    assert!(
        stderr.contains("no/such/target.md"),
        "the authored reference must be named; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("initialize"),
        "the authoring event must be named; stderr:\n{stderr}"
    );
    // Structured `property` path, driven by the real `route_initialize`
    // constructor: it names the authoring stack location, not a bare event
    // root. Fails against the pre-fix `"initialize"` value, which lacks
    // `.stack[*].proxy`.
    assert!(
        stderr.contains("initialize.stack[*].proxy"),
        "the block must name the authored `initialize.stack[*].proxy` property; \
         stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("route.md"),
        "the authoring document must be named; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("does not exist"),
        "the resolution failure must be stated; stderr:\n{stderr}"
    );

    // The original flattened report text must be gone.
    assert!(
        !stderr.contains("lifecycle initialize proxy:"),
        "the pre-migration flattened report text survived; stderr:\n{stderr}"
    );
}

/// Proxy hand-off from a terminal (`failure`) event — the second proxy route.
///
/// Regression test for the `Box` un-downcastability recorded in `decisions.md`
/// D-7, caught at Phase 7 by driving this route through a real terminal.
///
/// `preflight_proxy_target` returns `Result<(), Box<CompositionError>>`, and
/// `loop_control.rs` handed that straight to `Report::from`. A `Report` built
/// from `Box<E>` publishes **`Box<E>`** to the cause chain — `Box<E>` is itself
/// an `Error` — so `as_diagnostic`'s downcast allowlist, which is keyed on the
/// concrete type, could not see the `CompositionError` inside. Worse, `Box<E>`'s
/// own `source()` delegates to `E::source()`, so the walk skipped past `E`
/// entirely and it was never a chain member at any depth.
///
/// The registry listed the type and the source-parity guard passed; the error
/// was simply unreachable, and the route degraded to:
///
/// ```text
/// Error: failed to load Markdown: …: No such file or directory (os error 2)
/// ```
///
/// This is the same failure shape as the motivating incident — a typed error
/// present in the chain but invisible to selection — reached by a different
/// mechanism, which is why it survived Phase 5.
#[test]
fn terminal_proxy_resolution_failure_renders_a_status_block() {
    let workspace = TestWorkspace::named("claudine-terminal-proxy");
    let root = workspace.path();
    let bin_dir = root.join("bin");
    // Exit non-zero so the `failure` stack — and its proxy hand-off — runs.
    write_executable(&bin_dir.join(shim_name("claude")), &failing_provider_shim());

    let doc = root.join("route.md");
    write(
        &doc,
        r#"---
title: terminal proxy
failure:
  stack:
    - action: {proxy: "no/such/target.md"}
---
Body
"#,
    );

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(root)
        .args(["compose", "--claude", doc.to_str().unwrap()])
        .output()
        .expect("spawn claudine");
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(
        has_status_block(&stderr),
        "the terminal-proxy failure must render a StatusBlock; stderr:\n{stderr}"
    );
    assert!(
        !has_generic_error_line(&stderr),
        "the terminal-proxy failure must not fall back to the generic `Error:` \
         line — a `Box<CompositionError>` reaching `Report::from` unboxed is how \
         this regresses; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("target.md"),
        "the adopted target must be named; stderr:\n{stderr}"
    );
    // Structured `property` path, driven by the real `dispatch_terminal_control`
    // constructor: it names the terminal stack location (`failure.stack[*].proxy`),
    // not a bare `proxy`. Fails against the pre-fix `"proxy"` value.
    assert!(
        stderr.contains("failure.stack[*].proxy"),
        "the block must name the authored `failure.stack[*].proxy` property; \
         stderr:\n{stderr}"
    );
}

/// Plain / piped stderr carries the same information with no escape bytes —
/// the block is not a TTY-only surface.
#[test]
fn status_block_is_plain_and_informative_when_piped() {
    let raw = {
        let workspace = TestWorkspace::named("claudine-effective-diagnostic-plain");
        let root = workspace.path();
        let bin_dir = root.join("bin");
        write_executable(&bin_dir.join(shim_name("claude")), PROVIDER_SHIM);
        let doc = root.join("route.md");
        write(
            &doc,
            "---\ntitle: t\ninitialize:\n  stack:\n    - action: {proxy: \"no/such/target.md\"}\n---\nBody\n",
        );
        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .env("PATH", augmented_path(&bin_dir))
            .current_dir(root)
            .args(["compose", "--claude", doc.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        String::from_utf8_lossy(&output.stderr).into_owned()
    };

    assert!(
        !raw.contains('\u{1b}'),
        "NO_COLOR / piped stderr must carry no ANSI or OSC 8 bytes; stderr:\n{raw:?}"
    );
    assert!(
        raw.contains("no/such/target.md"),
        "plain output lost the reference; stderr:\n{raw}"
    );
}

/// The control: a genuinely unstructured error must **still** reach the generic
/// fallback. That path stays valid (spec §D5), and if it starts rendering a
/// block the walker has become too eager.
///
/// Argument-shape rejection is the honest unstructured case — it is authored
/// prose with no typed error behind it. (`--timeout not-a-duration` looks
/// unstructured but is **not**: it carries a typed
/// `HarnessError::InvalidTimeout`, and since Phase 5 it correctly renders a
/// block. See `characterization_error_routes.rs` route 5.)
#[test]
fn unstructured_failure_still_uses_the_generic_fallback() {
    let stderr = compose_stderr("---\ntitle: t\n---\nBody\n", &["also-a-file.md"]);

    assert!(
        has_generic_error_line(&stderr),
        "an unstructured error must still reach the generic `Error:` line; \
         stderr:\n{stderr}"
    );
    assert!(
        !has_status_block(&stderr),
        "an unstructured error must not render a StatusBlock; stderr:\n{stderr}"
    );
}

/// A typed error that merely *looks* unstructured still renders a block.
///
/// The counterpart to the control above, and the reason the control had to
/// change: "no block" is a claim about the error's **type**, never about how
/// incidental the failure feels.
#[test]
fn typed_argument_error_renders_a_block_not_the_generic_line() {
    let stderr = compose_stderr(
        "---\ntitle: t\n---\nBody\n",
        &["--timeout", "not-a-duration"],
    );

    assert!(
        has_status_block(&stderr),
        "a typed `HarnessError::InvalidTimeout` must render a StatusBlock; \
         stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("not-a-duration"),
        "the rejected value must be named; stderr:\n{stderr}"
    );
}
