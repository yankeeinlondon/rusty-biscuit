//! Body `::block when=` guards at shell sites, through the public compose and
//! preflight APIs: discovery types each body `::shell` and `::shell-block`
//! command on the true side of the blocks around it, so a missing value a
//! guard proves present waits for its command as a template, and the compose
//! pass types the site under the same guards, so the shape discovery
//! approved is the shape that runs.
//!
//! Every document lives in its own temp dir; the only program is `git`, run
//! with `--version` so its output proves the command ran.

use darkmatter::markdown::compose::preflight::ComposePreflightReport;
use darkmatter::markdown::compose::shell_expansion::template::{
    HoleTiming, ModeRequirement, ShellCandidate, ShellPreApprovals,
};
use darkmatter::markdown::compose::shell_expansion::types::{ShellCommandOrigin, ShellExpansionError};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write(dir: &Path, content: &str) -> PathBuf {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    path
}

fn options_for(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_timeout(std::time::Duration::from_secs(60))
}

fn preflight(content: &str) -> Result<ComposePreflightReport, MarkdownError> {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), content);
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_preflight(&crate::request_support::request(options_for(dir.path(), &path)))
}

/// Discovers `content`, approves exactly the shapes discovery reported, and
/// composes it under those approvals alone.
fn compose_approving_discovered_shapes(content: &str) -> (ComposePreflightReport, Result<String, MarkdownError>) {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), content);
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let report = markdown
        .compose_preflight(&crate::request_support::request(options_for(dir.path(), &path)))
        .expect("discovery passes");
    let mut approvals = ShellPreApprovals::new();
    for candidate in &report.candidates {
        approvals.insert_template(candidate.shape().expect("every site is a template"));
    }
    let options = options_for(dir.path(), &path).with_pre_approved_commands(approvals);
    let composed = markdown
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.content().to_string());
    (report, composed)
}

fn missing_expression(error: &MarkdownError) -> Option<(&str, &ShellCommandOrigin)> {
    match error {
        MarkdownError::ShellExpansion(inner) => match &**inner {
            ShellExpansionError::MissingValue { expression, origin, .. } => Some((expression.as_str(), origin)),
            _ => None,
        },
        _ => None,
    }
}

/// The guard label of each placeholder `HoleTiming::Guarded` holds, or `None`
/// for any other timing.
fn guards(candidate: &ShellCandidate) -> Vec<Option<String>> {
    candidate
        .template
        .as_ref()
        .expect("valid template")
        .placeholders()
        .map(|placeholder| match &placeholder.timing {
            HoleTiming::Guarded { guard, .. } => Some(guard.label().to_string()),
            _ => None,
        })
        .collect()
}

// ── Discovery ────────────────────────────────────────────────────────────

/// A body `::shell` and a `::shell-block` command whose value is missing at
/// discovery pass it inside `::block when="x"`: the hole is guarded by that
/// block, the site is template-required, and it has no resolved bytes.
#[test]
fn a_missing_value_its_block_proves_present_waits_as_a_template() {
    for (surface, body) in [
        ("::shell", "::block when=\"x\"\n::shell git --version {{ x }}\n::end-block\n"),
        ("::shell-block", "::block when=\"x\"\n::shell-block\ngit --version {{ x }}\n::end-block\n::end-block\n"),
        ("`fm.` read", "::block when=\"fm.x\"\n::shell git --version {{ x }}\n::end-block\n"),
        ("predicate guard", "::block when=\"is_string(x)\"\n::shell git --version {{ x }}\n::end-block\n"),
        ("comparison guard", "::block when=\"x != null\"\n::shell git --version {{ x }}\n::end-block\n"),
        ("negated null equality", "::block when=\"!(x == null)\"\n::shell git --version {{ x }}\n::end-block\n"),
        ("reversed comparison guard", "::block when=\"null != x\"\n::shell git --version {{ x }}\n::end-block\n"),
        // Strict equality: `null == ''` is false, so only a present `x` equals `''`.
        ("empty-string equality", "::block when=\"x == ''\"\n::shell git --version {{ x }}\n::end-block\n"),
        ("conjunction guard", "::block when=\"a && x\"\n::shell git --version {{ x }}\n::end-block\n"),
        // The guard is never evaluated, so no filesystem is read for it.
        ("file guard", "::block when=\"file_exists(x)\"\n::shell git --version {{ x }}\n::end-block\n"),
    ] {
        let report = preflight(&format!("# Doc\n{body}")).unwrap_or_else(|error| panic!("{surface}: {error}"));
        let [site] = report.candidates.as_slice() else {
            panic!("{surface}: one site: {:?}", report.candidates);
        };
        assert_eq!(site.requirement, ModeRequirement::TemplateRequired, "{surface}");
        assert!(site.resolved.is_none(), "{surface}");
        let guards = guards(site);
        let [Some(guard)] = guards.as_slice() else {
            panic!("{surface}: the hole is guarded");
        };
        assert!(guard.starts_with("::block when="), "{surface}: {guard}");
        assert!(guard.ends_with("(line 2)"), "{surface}: {guard}");
        assert!(report.entries.is_empty(), "{surface}: no resolved bytes to approve");
    }
}

/// Without an enclosing guard that proves the value, a missing value still
/// fails preflight: outside the block, under a false side, and under a guard
/// that proves nothing about it.
#[test]
fn a_missing_value_no_enclosing_block_proves_still_fails() {
    for (row, body, expression) in [
        ("after the block", "::block when=\"x\"\ntext\n::end-block\n::shell git --version {{ x }}\n", "x"),
        ("negated guard", "::block when=\"!x\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        ("absence predicate", "::block when=\"is_null(x)\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        ("disjunction", "::block when=\"x || y\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        // Strict equality: a missing `x` is `null`, and `null != ''` is true.
        ("empty-string inequality", "::block when=\"x != ''\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        ("reversed empty-string inequality", "::block when=\"'' != x\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        ("null equality", "::block when=\"x == null\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        ("negated null inequality", "::block when=\"!(x != null)\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        ("sibling member", "::block when=\"x.a\"\n::shell git --version {{ x.b }}\n::end-block\n", "x.b"),
        ("parent of a member", "::block when=\"x.a\"\n::shell git --version {{ x }}\n::end-block\n", "x"),
        (
            "another value in the same command",
            "::block when=\"x\"\n::shell git --version {{ x }} {{ y }}\n::end-block\n",
            "y",
        ),
        (
            "another value in a shell block",
            "::block when=\"x\"\n::shell-block\ngit --version {{ y }}\n::end-block\n::end-block\n",
            "y",
        ),
    ] {
        let error = preflight(&format!("# Doc\n{body}")).expect_err(row);
        let (got, _) = missing_expression(&error).unwrap_or_else(|| panic!("{row}: {error:?}"));
        assert_eq!(got, expression, "{row}");
    }
}

/// Nested blocks compose: each guard proves its own value at the inner site.
#[test]
fn nested_blocks_prove_every_enclosing_guard() {
    let report = preflight(
        "# Doc\n::block when=\"x\"\n::block when=\"y\"\n::shell git --version {{ x }} {{ y }}\n::end-block\n::end-block\n",
    )
    .unwrap();
    let guards = guards(&report.candidates[0]);
    assert_eq!(
        guards,
        [Some("::block when=\"x\" (line 2)".to_string()), Some("::block when=\"y\" (line 3)".to_string())]
    );
}

/// A standalone frontmatter `$( … )` value is never guarded by a body block,
/// even one that reads the key.
#[test]
fn a_body_block_never_guards_a_frontmatter_shell_value() {
    let error = preflight("---\nlisting: $(git --version {{ x }})\n---\n::block when=\"x\"\n{{ listing }}\n::end-block\n")
        .expect_err("the frontmatter value is unguarded");
    let (expression, origin) = missing_expression(&error).unwrap_or_else(|| panic!("{error:?}"));
    assert_eq!(expression, "x");
    assert!(matches!(origin, ShellCommandOrigin::Frontmatter { .. }), "{origin:?}");
}

// ── Discovery and the compose pass agree ────────────────────────────────

/// A guarded site whose value is missing never runs: the false block is
/// removed, so composing under the discovered approvals succeeds without the
/// command.
#[test]
fn a_guarded_missing_value_skips_its_command() {
    let (_, composed) =
        compose_approving_discovered_shapes("# Doc\n::block when=\"x\"\n::shell git --version {{ x }}\n::end-block\nend\n");
    let composed = composed.unwrap();
    assert!(!composed.contains("git version"), "{composed}");
    assert!(composed.contains("end"), "{composed}");
}

/// When the block is selected, the guarded site runs with the live value,
/// approved by nothing but the shape discovery reported.
#[test]
fn a_selected_guarded_site_runs_with_its_live_value() {
    for body in [
        "::block when=\"flag\"\n::shell git {{ flag }}\n::end-block\n",
        "::block when=\"flag\"\n::shell-block\ngit {{ flag }}\n::end-block\n::end-block\n",
    ] {
        let (_, composed) = compose_approving_discovered_shapes(&format!("---\nflag: --version\n---\n{body}"));
        let composed = composed.unwrap_or_else(|error| panic!("{body}: {error}"));
        assert!(composed.contains("git version"), "{body}: {composed}");
    }
}

/// A guard that narrows a hole's type changes its approval shape, and the
/// compose pass types the site under the same guard, so the narrowed shape
/// discovery approved is the one that runs: the narrowed enum (a shape the
/// unguarded site would not match) and acceptance 6's `is_string(target)`.
#[test]
fn the_compose_pass_types_a_site_under_the_guards_discovery_used() {
    let enum_schema = "$schema:\n  mode: \"enum(release, debug, test; required)\"\nmode: debug\n";
    let target_schema = "$schema:\n  target: [string, number, \"null\"]\ntarget: hello\n";
    for (row, frontmatter, body, shape) in [
        (
            "narrowed enum, ::shell",
            enum_schema,
            "::block when=\"mode != 'release'\"\n::shell git --version {{ mode }}\n::end-block\n",
            "git --version {{ enum(debug,test) }}",
        ),
        (
            "narrowed enum, ::shell-block",
            enum_schema,
            "::block when=\"mode != 'release'\"\n::shell-block\ngit --version {{ mode }}\n::end-block\n::end-block\n",
            "git --version {{ enum(debug,test) }}",
        ),
        (
            "acceptance 6",
            target_schema,
            "::block when=\"is_string(target)\"\n::shell git --version {{ target }}\n::end-block\n",
            "git --version {{ string }}",
        ),
    ] {
        let (report, composed) = compose_approving_discovered_shapes(&format!("---\n{frontmatter}---\n{body}"));
        let [site] = report.candidates.as_slice() else {
            panic!("{row}: one site: {:?}", report.candidates);
        };
        assert_eq!(site.shape().unwrap().canonical_text(), shape, "{row}");
        let refinement = site.template.as_ref().unwrap().placeholders().next().unwrap().typing.refinement.clone();
        assert!(refinement.is_some(), "{row}: the guard refined the hole");
        let composed = composed.unwrap_or_else(|error| panic!("{row}: {error}"));
        assert!(composed.contains("git version"), "{row}: {composed}");
    }
}
