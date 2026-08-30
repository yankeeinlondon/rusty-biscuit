//! CLI-seam regressions for launch-anchored prepared `ctx.*`
//! (`fixes/2026-08-12-ctx-launch-anchor`).
//!
//! AC1–AC4: equivalent documents run from one launch directory while stored at
//! the repository root, in the launch package area, in an opposing package
//! area, and in an external repository must all report the *launch* repository
//! and package area. Every test drives the real `claudine` binary through the
//! canonical capture owner — no hand-built `ComposeContext` is injected.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod common;
use common::{augmented_path, init_git_repo, write};

#[cfg(unix)]
fn write_command_stub(bin_dir: &Path, name: &str, unix_content: &str, _windows_content: &str) {
    common::write_executable(&bin_dir.join(name), unix_content);
}

#[cfg(windows)]
fn write_command_stub(bin_dir: &Path, name: &str, _unix_content: &str, windows_content: &str) {
    write(&bin_dir.join(format!("{name}.cmd")), windows_content);
}

/// A two-package-area Cargo workspace, so `ctx.area` has a real answer for a
/// launch inside `alpha/lib`.
fn stage_monorepo(root: &Path) {
    init_git_repo(root);
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha/lib\", \"beta/lib\"]\nresolver = \"2\"\n",
    );
    write(
        &root.join("alpha/lib/Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &root.join("beta/lib/Cargo.toml"),
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
}

/// The probe document: body and effective-frontmatter surfaces that both read
/// the prepared launch snapshot.
const PROBE_BODY: &str = "AREA={{ ctx.area }} CWD={{ ctx.cwd }} REPO={{ ctx.repo_root }}";
const PROBE_DOC: &str =
    "---\ntitle: ctx probe\nmy_area: \"{{ ctx.area }}\"\nmy_cwd: \"{{ ctx.cwd }}\"\n---\n";

fn write_probe(path: &Path) {
    write(path, &format!("{PROBE_DOC}{PROBE_BODY}"));
}

/// The spelling the launched CLI reports for a fixture path.
///
/// On Unix the child's `current_dir()` is symlink-resolved (macOS tempdirs
/// live under the `/var` → `/private/var` symlink), so expectations must
/// canonicalize. On Windows `set_current_dir` preserves the given spelling
/// (including 8.3 short names on CI), and `canonicalize` would instead
/// produce a verbatim `\\?\` long-name form the CLI never emits.
fn launched_spelling(path: &Path) -> PathBuf {
    if cfg!(windows) {
        path.to_path_buf()
    } else {
        path.canonicalize().expect("canonical launch path")
    }
}

fn run_dry_run(launch_dir: &Path, home: &Path, doc: &Path) -> String {
    let output = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", home)
        .env("CLAUDE_CODE_EXIT", "0")
        .current_dir(launch_dir)
        .args(["compose", "--claude", "--dry-run", &doc.to_string_lossy()])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).into_owned();
    format!("{stdout}\n--- stderr ---\n{stderr}")
}

/// AC1: prompts at the repository root and inside the launch package area
/// report the same launch area.
#[test]
fn dry_run_reports_the_launch_area_for_root_and_package_prompts() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");
    let at_root = root.join("probe-root.md");
    let in_area = launch_dir.join("probe-area.md");
    write_probe(&at_root);
    write_probe(&in_area);

    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    for doc in [&at_root, &in_area] {
        let output = run_dry_run(&launch_dir, &home, doc);
        assert!(
            output.contains("AREA=alpha"),
            "launch package area must be reported for {}\noutput:\n{output}",
            doc.display()
        );
        assert!(
            output.contains(&format!(
                "CWD={}",
                biscuit_file::to_portable_string(&launched_spelling(&launch_dir))
            )),
            "launch CWD must be reported for {}\noutput:\n{output}",
            doc.display()
        );
        let reported_repo = output
            .split_once("REPO=")
            .map(|(_, rest)| rest.lines().next().unwrap_or_default().to_string())
            .unwrap_or_default();
        assert_eq!(
            fs::canonicalize(reported_repo.trim()).ok(),
            fs::canonicalize(&root).ok(),
            "launch repository must be reported for {}\noutput:\n{output}",
            doc.display()
        );
    }
}

/// AC2: a prompt stored in the opposing package area reports the launch area.
#[test]
fn dry_run_reports_the_launch_area_for_an_opposing_area_prompt() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");
    let opposing = root.join("beta/lib/probe-beta.md");
    write_probe(&opposing);

    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_dry_run(&launch_dir, &home, &opposing);
    assert!(
        output.contains("AREA=alpha"),
        "an opposing-area prompt must report the launch area\noutput:\n{output}"
    );
}

/// AC2/AC10: one opposing-area real invocation discriminates every prepared
/// surface from source-owned file and schema resolution.
#[test]
fn opposing_area_real_route_separates_launch_surfaces_from_source_files() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");
    let source_dir = root.join("beta/lib");
    let doc = source_dir.join("probe-full-matrix.md");

    write(
        &launch_dir.join("schema.yaml"),
        "launch_only: string(required)\n",
    );
    write(&launch_dir.join("fragment.md"), "LAUNCH-FRAGMENT\n");
    write(
        &source_dir.join("schema.yaml"),
        "source_marker: string(required)\nspec: 'file(eager; required)'\nmy_area: string(required)\nmy_cwd: string(required)\n",
    );
    write(&source_dir.join("spec.md"), "SOURCE-SPEC\n");
    write(&source_dir.join("fragment.md"), "SOURCE-FRAGMENT\n");
    write(
        &doc,
        concat!(
            "---\n",
            "$schema: ./schema.yaml\n",
            "source_marker: source-owned\n",
            "spec: spec.md\n",
            "my_area: '{{ ctx.area }}'\n",
            "my_cwd: '{{ ctx.cwd }}'\n",
            "start:\n",
            "  stack:\n",
            "    - when: \"ctx.area == 'alpha'\"\n",
            "      action: {append_line: [\"events.log\", \"when-held\"]}\n",
            "    - action: {shell: \"recordctx {{ ctx.area }}\"}\n",
            "success:\n",
            "  stack:\n",
            "    - action: {append_line: [\"events.log\", \"ctx={{ ctx.area }} cwd={{ ctx.cwd }} fm={{ my_area }}/{{ my_cwd }}\"]}\n",
            "---\n",
            "SOURCE-BODY AREA={{ ctx.area }} CWD={{ ctx.cwd }} FILE={{ file_exists(spec) }}\n",
        ),
    );
    write(
        &root.join(".darkmatter-shell-whitelist"),
        "prefix recordctx\n",
    );

    let bin = workspace.path().join("bin");
    let provider_log = workspace.path().join("provider.log");
    let shell_log = workspace.path().join("shell.log");
    write_command_stub(
        &bin,
        "claude",
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$CTX_PROVIDER_LOG\"\ncat >> \"$CTX_PROVIDER_LOG\"\nexit 0\n",
        "@echo off\r\n> \"%CTX_PROVIDER_LOG%\" echo %*\r\nmore >> \"%CTX_PROVIDER_LOG%\"\r\nexit /b 0\r\n",
    );
    write_command_stub(
        &bin,
        "recordctx",
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CTX_SHELL_LOG\"\n",
        "@echo off\r\n>> \"%CTX_SHELL_LOG%\" echo %*\r\n",
    );
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &home)
        .env("PATH", augmented_path(&bin))
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .env("CTX_PROVIDER_LOG", &provider_log)
        .env("CTX_SHELL_LOG", &shell_log)
        .current_dir(&launch_dir)
        .args(["compose", "--claude", &doc.to_string_lossy()])
        .assert()
        .success();

    let provider = fs::read_to_string(&provider_log).unwrap_or_default();
    assert!(
        provider.contains("AREA=alpha") && provider.contains("FILE=true"),
        "provider-bound body must contain launch area and source-owned eager file: {provider}"
    );
    let expected_cwd = biscuit_file::to_portable_string(&launched_spelling(&launch_dir));
    assert!(provider.contains(&format!("CWD={expected_cwd}")), "{provider}");
    assert!(
        provider.contains("SOURCE-BODY") && !provider.contains("LAUNCH-FRAGMENT"),
        "the provider must receive the opposing source's body: {provider}"
    );
    assert_eq!(
        fs::read_to_string(&shell_log).unwrap_or_default().trim(),
        "alpha",
        "preflight-expanded and executed shell bytes must contain the launch area"
    );
    let events = fs::read_to_string(root.join("events.log")).unwrap_or_default();
    assert!(events.contains("when-held"), "lifecycle when: did not see launch area: {events}");
    assert!(
        events.contains(&format!("ctx=alpha cwd={expected_cwd} fm=alpha/{expected_cwd}")),
        "lifecycle and effective frontmatter must share the launch area: {events}"
    );
}

/// Finalized-reference AC5 regression (2026-08-26): D3 supersedes
/// ctx-launch-anchor review-3 Finding 4 for document-authored references while
/// retaining its caller rule. A caller file parameter is materialized at the
/// launch boundary and must keep that absolute identity when a repo-root shared
/// prompt proxies to another document.
#[test]
fn caller_file_anchor_survives_direct_and_proxy_success_guards() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");
    let launch_spec = launch_dir.join("spec.md");
    write(&launch_spec, "# launch spec\n");
    write(&launch_dir.join("sibling.md"), "# launch sibling\n");
    write(&root.join("spec.md"), "# repository decoy\n");

    let target = root.join("shared-target.md");
    write(
        &target,
        concat!(
            "---\n",
            "$schema:\n",
            "  spec: 'file(eager; required)'\n",
            "success:\n",
            "  stack:\n",
            "    - when: \"file_exists(dirname(spec) + '/sibling.md') && file_exists(parent_dir(spec) + '/sibling.md')\"\n",
            "      action: {append_line: [\"events.log\", \"anchored={{ spec }} cwd={{ ctx.cwd }}\"]}\n",
            "---\n",
            "TARGET SPEC={{ spec }}\n",
        ),
    );
    let router = root.join("shared-router.md");
    write(
        &router,
        concat!(
            "---\n",
            "$schema:\n",
            "  spec: 'file(eager; required)'\n",
            "initialize:\n",
            "  stack:\n",
            "    - action: {proxy: \"shared-target.md\"}\n",
            "---\n",
            "ROUTER\n",
        ),
    );

    let bin = workspace.path().join("bin");
    let provider_log = workspace.path().join("provider.log");
    write_command_stub(
        &bin,
        "claude",
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CTX_PROVIDER_LOG\"\ncat >> \"$CTX_PROVIDER_LOG\"\nexit 0\n",
        "@echo off\r\n>> \"%CTX_PROVIDER_LOG%\" echo %*\r\nmore >> \"%CTX_PROVIDER_LOG%\"\r\nexit /b 0\r\n",
    );
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    for document in [&target, &router] {
        assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", &home)
            .env("PATH", augmented_path(&bin))
            .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
            .env("CTX_PROVIDER_LOG", &provider_log)
            .current_dir(&launch_dir)
            .args([
                "compose",
                "--claude",
                &document.to_string_lossy(),
                "spec=spec.md",
            ])
            .assert()
            .success();
    }

    let expected = biscuit_file::to_portable_string(&launched_spelling(&launch_spec));
    let provider = fs::read_to_string(&provider_log).unwrap_or_default();
    assert_eq!(provider.matches("TARGET SPEC=").count(), 2, "{provider}");
    assert!(provider.contains(&format!("TARGET SPEC={expected}")), "{provider}");
    let events = fs::read_to_string(root.join("events.log")).unwrap_or_default();
    // D8.4: `{{ spec }}` in a lifecycle action carries the eager file()'s
    // NATIVE identity; only body/Markdown presentation is portable. ctx.cwd
    // is portable on every surface.
    let expected_spec_native = launched_spelling(&launch_spec)
        .components()
        .collect::<PathBuf>()
        .display()
        .to_string();
    let expected_cwd = biscuit_file::to_portable_string(&launched_spelling(&launch_dir));
    assert_eq!(
        events
            .matches(&format!("anchored={expected_spec_native} cwd={expected_cwd}"))
            .count(),
        2,
        "{events}"
    );
}

/// AC1/AC8: loop documents at the repository root and in the launch package
/// reuse the launch snapshot through their conditions, bodies, and lifecycle.
#[test]
fn loop_route_keeps_launch_area_for_root_and_package_documents() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");
    let launch_spec = launch_dir.join("spec.md");
    write(&launch_spec, "# launch spec\n");
    write(&root.join("spec.md"), "# repository decoy\n");
    let at_root = root.join("loop-root.md");
    let in_area = launch_dir.join("loop-area.md");
    for (path, label) in [(&at_root, "root"), (&in_area, "area")] {
        write(
            path,
            &format!(
                "---\n$schema:\n  spec: 'file(eager; required)'\nmy_area: '{{{{ ctx.area }}}}'\nmy_cwd: '{{{{ ctx.cwd }}}}'\nloop:\n  while: \"ctx.area == 'alpha' && counter < 1\"\n  actions:\n    - \"increment(counter)\"\nstart:\n  stack:\n    - when: \"ctx.area == 'alpha'\"\n      action: {{append_line: [\"loop-events.log\", \"{label}:ctx={{{{ ctx.area }}}}:cwd={{{{ ctx.cwd }}}}:fm={{{{ my_area }}}}/{{{{ my_cwd }}}}:spec={{{{ spec }}}}\"]}}\n---\n{label}:AREA={{{{ ctx.area }}}}:CWD={{{{ ctx.cwd }}}}:SPEC={{{{ spec }}}}\n"
            ),
        );
    }

    let bin = workspace.path().join("bin");
    let provider_log = workspace.path().join("loop-provider.log");
    write_command_stub(
        &bin,
        "claude",
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CTX_PROVIDER_LOG\"\ncat >> \"$CTX_PROVIDER_LOG\"\nprintf '%s\\n' '--- attempt ---' >> \"$CTX_PROVIDER_LOG\"\nexit 0\n",
        "@echo off\r\n>> \"%CTX_PROVIDER_LOG%\" echo %*\r\nmore >> \"%CTX_PROVIDER_LOG%\"\r\n>> \"%CTX_PROVIDER_LOG%\" echo --- attempt ---\r\nexit /b 0\r\n",
    );
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    for doc in [&at_root, &in_area] {
        assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", &home)
            .env("PATH", augmented_path(&bin))
            .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
            .env("CTX_PROVIDER_LOG", &provider_log)
            .current_dir(&launch_dir)
            .args(["compose", "--claude", &doc.to_string_lossy(), "spec=spec.md"])
            .assert()
            .success();
    }

    let provider = fs::read_to_string(&provider_log).unwrap_or_default();
    assert!(provider.contains("root:AREA=alpha"));
    assert!(provider.contains("area:AREA=alpha"));
    let expected_cwd = biscuit_file::to_portable_string(&launched_spelling(&launch_dir));
    let expected_spec = biscuit_file::to_portable_string(&launched_spelling(&launch_spec));
    assert_eq!(provider.matches(&format!("CWD={expected_cwd}")).count(), 4);
    assert_eq!(provider.matches(&format!("SPEC={expected_spec}")).count(), 4);
    assert_eq!(
        provider.matches("--- attempt ---").count(),
        4,
        "each equivalent loop document must run its seed attempt and one iteration: {provider}"
    );
    let events = fs::read_to_string(root.join("loop-events.log")).unwrap_or_default();
    // D8.4: the lifecycle `{{ spec }}` surface carries the native identity,
    // unlike the portable body rendering asserted through the provider log.
    let expected_spec_native = launched_spelling(&launch_spec)
        .components()
        .collect::<PathBuf>()
        .display()
        .to_string();
    assert!(events.contains(&format!(
        "root:ctx=alpha:cwd={expected_cwd}:fm=alpha/{expected_cwd}:spec={expected_spec_native}"
    )));
    assert!(events.contains(&format!(
        "area:ctx=alpha:cwd={expected_cwd}:fm=alpha/{expected_cwd}:spec={expected_spec_native}"
    )));
}

/// AC3: a prompt stored in another repository reports the launch repository;
/// a prompt inside a repository launched from outside every repository reports
/// no launch facts at all.
#[test]
fn dry_run_external_source_and_outside_launch_matrix() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let external = workspace.path().join("external");
    fs::create_dir_all(&external).unwrap();
    init_git_repo(&external);
    let external_probe = external.join("probe-external.md");
    write_probe(&external_probe);

    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let launch_dir = root.join("alpha/lib");

    let output = run_dry_run(&launch_dir, &home, &external_probe);
    assert!(
        output.contains("AREA=alpha"),
        "an external-repository prompt must report the launch area\noutput:\n{output}"
    );
    let reported_repo = output
        .split_once("REPO=")
        .map(|(_, rest)| rest.lines().next().unwrap_or_default().to_string())
        .unwrap_or_default();
    assert_eq!(
        fs::canonicalize(reported_repo.trim()).ok(),
        fs::canonicalize(&root).ok(),
        "an external-repository prompt must report the launch repository\noutput:\n{output}"
    );

    // Inverse: launch outside every repository while the prompt lives inside
    // one. The prompt's location must not fill the absent launch facts.
    let outside = workspace.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    let inside_probe = root.join("probe-inside.md");
    write_probe(&inside_probe);
    let output = run_dry_run(&outside, &home, &inside_probe);
    assert!(
        output.contains("AREA=") && !output.contains("AREA=alpha") && !output.contains("AREA=beta"),
        "no launch package area exists outside every repository\noutput:\n{output}"
    );
    assert!(
        !output.contains(&format!("REPO={}", biscuit_file::to_portable_string(&root))),
        "no launch repository exists outside every repository; the prompt's own \
         repository must not be substituted\noutput:\n{output}"
    );
}

/// AC1 (lifecycle surface): a real run whose lifecycle `warn:` interpolation
/// and `when:` guard read the prepared snapshot reports the launch area even
/// though the prompt is stored at the repository root.
#[test]
fn lifecycle_warn_and_when_report_the_launch_area() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");

    let doc = root.join("probe-lifecycle.md");
    write(
        &doc,
        concat!(
            "---\n",
            "title: lifecycle probe\n",
            "failure:\n",
            "  stack:\n",
            "    - action: {append_line: [\"events.log\", \"failure-area={{ ctx.area }}\"]}\n",
            "start:\n",
            "  stack:\n",
            "    - when: \"ctx.area == 'alpha'\"\n",
            "      action: {append_line: [\"events.log\", \"when-alpha-held\"]}\n",
            "---\n",
            "Body for the provider.\n",
        ),
    );

    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    // The fake provider exits non-zero so the `failure` event (a terminal
    // interpolation surface) fires alongside `start`.
    let bin = workspace.path().join("bin");
    write_command_stub(
        &bin,
        "claude",
        "#!/bin/sh\nexit 1\n",
        "@echo off\r\nexit /b 1\r\n",
    );

    let output = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &home)
        .env("PATH", augmented_path(&bin))
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .current_dir(&launch_dir)
        .args(["compose", "--claude", "--perf", &doc.to_string_lossy()])
        .assert()
        .failure();

    // Lifecycle effects write relative to the source repository's mutation
    // root, which is the probe's own repository here.
    let events = fs::read_to_string(root.join("events.log")).unwrap_or_default();
    assert!(
        events.contains("failure-area=alpha"),
        "lifecycle failure interpolation must report the launch area; events:\n{events}"
    );
    assert!(
        events.contains("when-alpha-held"),
        "the when: guard must see the launch area; events:\n{events}"
    );
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    let plain = common::strip_ansi(&stderr);
    assert!(
        plain.contains("prepared consumers")
            && plain.contains(
                "[body (2), effective-frontmatter (2), lifecycle, preflight]"
            ),
        "the live direct route must report its complete prepared-context consumer set; \
         perf note:\n{plain}"
    );
}

/// AC5/AC11: the real compose CLI constructs exactly one launch capture per
/// document epoch with zero ambient fallbacks — asserted through the `--perf`
/// work note, which projects the invocation owner's counters.
#[test]
fn perf_reports_one_launch_capture_and_zero_ambient_fallbacks() {
    let workspace = tempdir().unwrap();
    let root = workspace.path().join("repo");
    fs::create_dir_all(root.join("alpha/lib")).unwrap();
    fs::create_dir_all(root.join("beta/lib")).unwrap();
    stage_monorepo(&root);
    let launch_dir = root.join("alpha/lib");
    let at_root = root.join("probe-root.md");
    write_probe(&at_root);
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", &home)
        .current_dir(&launch_dir)
        .args([
            "compose",
            "--claude",
            "--dry-run",
            "--perf",
            &at_root.to_string_lossy(),
        ])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    let plain = common::strip_ansi(&stderr);
    assert!(
        plain.contains("launch captures 1 (extensions 0)"),
        "one document epoch must construct exactly one launch capture with no \
         extension; perf note:\n{plain}"
    );
    assert!(
        plain.contains("ambient fallbacks 0"),
        "no stage may fall through to darkmatter's ambient capture; perf note:\n{plain}"
    );
    assert!(
        plain.contains("prepared consumers")
            && plain.contains("[body, effective-frontmatter, preflight]"),
        "the direct dry-run route must report its exact prepared-context consumer set; \
         perf note:\n{plain}"
    );
}
