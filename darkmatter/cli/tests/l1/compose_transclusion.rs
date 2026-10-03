use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn compose_guarded_nullable_target_through_real_preflight_lifecycle() {
    let fixture = CliProcessFixture::named(
        "compose_guarded_nullable_target_through_real_preflight_lifecycle",
    );
    let document = fixture.write_file(
        "cwd/root.md",
        "---\n$schema:\n  log: file\n---\n\n::block when=\"file_exists(log)\"\n::file {{log}}\n::end-block\n",
    );
    let output = fixture
        .command()
        .arg("compose")
        .arg(&document)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "guarded nullable target must compose: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("::file"), "stdout: {stdout}");
}

#[test]
fn compose_rejects_pending_target_before_child_command_can_execute() {
    let fixture = CliProcessFixture::named(
        "compose_rejects_pending_target_before_child_command_can_execute",
    );
    let sentinel = fixture.workspace_path().join("child-command-ran");
    let sentinel_text = biscuit_file::to_portable_string(&sentinel);
    fixture.write_file(
        "cwd/child.md",
        &format!("::shell touch {sentinel_text}\n"),
    );
    let document = fixture.write_file(
        "cwd/root.md",
        "---\nchild: \"$(printf child.md)\"\n---\n::file {{child}}\n",
    );
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix printf\nprefix touch\n");
    let output = fixture
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&document)
        .output()
        .unwrap();
    assert!(!output.status.success(), "pending target must fail closed");
    assert!(!sentinel.exists(), "child command executed before approval completed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dynamic") && stderr.contains("child"),
        "pending directive target must be rejected as dynamic: {stderr}"
    );
}

#[test]
fn compose_shipped_transclusion_fixture_through_normal_cli_path() {
    let fixture = CliProcessFixture::named(
        "compose_shipped_transclusion_fixture_through_normal_cli_path",
    );
    fixture.write_file(
        "cwd/compose_child.md",
        include_str!(
            "../../../benchmarks/fixtures/compose_child.md"
        ),
    );
    let document = fixture.write_file(
        "cwd/compose_schema_transclusion.md",
        include_str!(
            "../../../benchmarks/fixtures/compose_schema_transclusion.md"
        ),
    );

    fixture
        .command()
        .arg("compose")
        .arg(document)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Schema and Transclusion"))
        .stdout(predicate::str::contains("## Included Section"));
}

#[test]
fn test_compose_set_variables_available_during_validation() {
    let fixture =
        CliProcessFixture::named("test_compose_set_variables_available_during_validation");
    // Regression test: --set variables must be available during reference
    // validation so that interpolated transclusion paths resolve correctly.
    // Previously, validation ran before --set was parsed, causing
    // `::file features/{{plan}}` to resolve to `features/` (empty plan).
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create the target file that will be transcluded
    std::fs::create_dir(temp_dir.path().join("features")).unwrap();
    std::fs::write(
        temp_dir.path().join("features/my-plan.md"),
        "# My Plan\n\nPlan content here.",
    )
    .unwrap();

    // Create a template that uses --set variable in a ::file directive
    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Task\n\n::file features/{{plan}}\n").unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(&template_path)
        .args(["--set", r#"{"plan":"my-plan.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn compose_with_the_shipped_baseline_renders_ctx_cwd_from_the_launch_directory() {
    let fixture = CliProcessFixture::named(
        "compose_with_the_shipped_baseline_renders_ctx_cwd_from_the_launch_directory",
    );
    let launch = fixture.cwd();
    let prompt = fixture.write_file("cwd/cwd.md", "Launch: {{ ctx.cwd }}\n");

    // ctx.cwd carries the child's `current_dir()` spelling: symlink-resolved
    // on Unix (macOS /var/folders), exactly as-launched on Windows, where CI
    // tempdirs use 8.3 short names that `canonicalize` would re-spell.
    #[cfg(windows)]
    let expected_launch = launch.to_path_buf();
    #[cfg(not(windows))]
    let expected_launch = std::fs::canonicalize(launch).expect("canonical launch directory");

    fixture
        .command_builder()
        .ambient_context(launch)
        .build()
        .arg("compose")
        .arg(&prompt)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Launch: {}",
            biscuit_file::to_portable_string(&expected_launch)
        )));
}

#[test]
fn test_compose_state_variables_available_during_validation() {
    let fixture =
        CliProcessFixture::named("test_compose_state_variables_available_during_validation");
    // Same as above but using --state instead of --set
    let temp_dir = tempfile::TempDir::new().unwrap();

    std::fs::create_dir(temp_dir.path().join("docs")).unwrap();
    std::fs::write(
        temp_dir.path().join("docs/readme.md"),
        "# Readme\n\nReadme content.",
    )
    .unwrap();

    // `doc` is the reserved frontmatter namespace, so a property literally
    // named `doc` is referenced as `doc.doc` (bare `{{doc}}` is the whole
    // frontmatter object).
    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Docs\n\n::file docs/{{doc.doc}}\n").unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(&template_path)
        .args(["--state", r#"{"doc":"readme.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Readme content."));
}

#[test]
fn test_compose_link_relative_same_repo() {
    let fixture = CliProcessFixture::named("test_compose_link_relative_same_repo");
    let repo = fixture.workspace_path().join("repo");
    assert!(fixture.initialize_repository_at(&repo));
    let docs = repo.join("docs");
    let assets = repo.join("assets");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(&assets).unwrap();

    let source_file = docs.join("source.md");
    let logo_file = assets.join("logo.png");
    std::fs::write(&source_file, "# Source\n\n![img](../assets/logo.png)\n").unwrap();
    std::fs::write(&logo_file, "png").unwrap();

    let output = fixture
        .command()
        .arg("compose")
        .arg(&source_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("../assets/logo.png"),
        "stdout should contain relative path, got:\n{stdout}"
    );
    // Should not contain absolute path
    assert!(
        !stdout.contains(assets.to_string_lossy().as_ref()),
        "stdout should not contain absolute asset path, got:\n{stdout}"
    );
    // No diagnostics in stdout
    assert!(
        !stdout.contains("Total records"),
        "stdout should not contain diagnostics, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("Record kind"),
        "stdout should not contain diagnostics, got:\n{stdout}"
    );
    // No unexpected stderr
    assert!(
        !stderr.contains("link_normalization"),
        "stderr should not contain raw warning tokens, got:\n{stderr}"
    );
}

#[test]
fn test_compose_link_transcluded_child() {
    let fixture = CliProcessFixture::named("test_compose_link_transcluded_child");
    let repo = fixture.workspace_path().join("repo");
    assert!(fixture.initialize_repository_at(&repo));

    let docs = repo.join("docs");
    let components = repo.join("components");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(&components).unwrap();

    let parent_file = docs.join("parent.md");
    let child_file = components.join("child.md");
    let sibling_file = components.join("sibling.md");

    std::fs::write(&parent_file, "# Parent\n\n::file ../components/child.md\n").unwrap();
    std::fs::write(&child_file, "[link](./sibling.md)\n").unwrap();
    std::fs::write(&sibling_file, "sibling content\n").unwrap();

    let output = fixture
        .command()
        .arg("compose")
        .arg(&parent_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("../components/sibling.md"),
        "stdout should contain normalized sibling path relative to parent, got:\n{stdout}"
    );
    // Should not contain absolute path
    let abs_sibling = biscuit_file::canonicalize_simplified(&sibling_file).unwrap();
    assert!(
        !stdout.contains(abs_sibling.to_string_lossy().as_ref()),
        "stdout should not contain absolute path, got:\n{stdout}"
    );
}

/// A variable declared portable through `PORTABLE_ENV_VARIABLES` anchors a
/// link nothing nearer reaches, and the composed output composes again to
/// the same text.
///
/// The document sits two levels below the shared temporary root, outside any
/// repository, so no default relative shape and no repository root applies;
/// `{{VAR}}` precedes `~` in the default strategy, so Windows (whose temporary
/// directory is under the profile) gives the same answer.
#[test]
fn test_compose_portable_env_variable_round_trips() {
    let fixture = CliProcessFixture::named("test_compose_portable_env_variable_round_trips");
    let dir = tempfile::tempdir().unwrap();
    let project_root = dir.path().join("project");
    let docs = dir.path().join("a").join("b");
    std::fs::create_dir_all(&project_root).unwrap();
    std::fs::create_dir_all(&docs).unwrap();
    let target_file = project_root.join("config.json");
    std::fs::write(&target_file, "{}").unwrap();

    let abs_root = std::fs::canonicalize(&project_root).unwrap();
    let abs_target_markdown =
        biscuit_file::to_portable_string(&std::fs::canonicalize(&target_file).unwrap());
    let md_file = docs.join("test.md");
    std::fs::write(&md_file, format!("[config]({abs_target_markdown})\n")).unwrap();

    let compose = |path: &std::path::Path, declared: bool| {
        let mut command = fixture.command();
        // Exported spelling: a verbatim `\\?\` value cannot anchor `{{VAR}}/…`.
        command.env("PROJECT_ROOT", biscuit_file::to_portable_string(&abs_root));
        if declared {
            command.env("PORTABLE_ENV_VARIABLES", "PROJECT_ROOT");
        } else {
            command.env_remove("PORTABLE_ENV_VARIABLES");
        }
        let output = command.arg("compose").arg(path).output().unwrap();
        assert!(
            output.status.success(),
            "command should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };

    let (stdout, stderr) = compose(&md_file, true);
    assert!(
        stdout.contains("[config]({{{PROJECT_ROOT}}}/config.json)"),
        "stdout should contain the escaped env anchor, got:\n{stdout}"
    );
    assert!(
        !stderr.contains("left exactly as authored"),
        "no preservation warning expected, got:\n{stderr}"
    );

    // Compose the output again from the same directory: the literal composes
    // to the `{{PROJECT_ROOT}}` anchor, which resolves and normalizes back.
    let recomposed_file = docs.join("recomposed.md");
    std::fs::write(&recomposed_file, &stdout).unwrap();
    let (recomposed, _) = compose(&recomposed_file, true);
    assert!(
        recomposed.contains("[config]({{{PROJECT_ROOT}}}/config.json)"),
        "recompose should reproduce the anchor, got:\n{recomposed}"
    );

    // Undeclared, the variable is never an anchor: there is no built-in set.
    let (undeclared, _) = compose(&md_file, false);
    assert!(
        !undeclared.contains("PROJECT_ROOT"),
        "an undeclared variable must not be written, got:\n{undeclared}"
    );
}

#[test]
fn test_compose_html_spaced_attributes() {
    let fixture = CliProcessFixture::named("test_compose_html_spaced_attributes");
    let repo = fixture.workspace_path().join("repo");
    assert!(fixture.initialize_repository_at(&repo));

    let page_file = repo.join("page.md");
    let other_file = repo.join("other.md");
    let img_file = repo.join("img.png");

    std::fs::write(
        &page_file,
        "# Page\n\n<a href = \"./other.md\">link</a>\n\n<img src = \"./img.png\">\n",
    )
    .unwrap();
    std::fs::write(&other_file, "other content\n").unwrap();
    std::fs::write(&img_file, "png").unwrap();

    let output = fixture
        .command()
        .arg("compose")
        .arg(&page_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Resolved to absolute and normalized back: a same-directory target is
    // `./name`, so the authored destinations come back unchanged.
    assert!(
        stdout.contains("<a href = \"./other.md\">"),
        "stdout should contain the normalized spaced href, got:\n{stdout}"
    );
    assert!(
        stdout.contains("<img src = \"./img.png\">"),
        "stdout should contain the normalized spaced src, got:\n{stdout}"
    );
    let abs_repo = std::fs::canonicalize(&repo).unwrap();
    assert!(
        !stdout.contains(&biscuit_file::to_portable_string(&abs_repo)),
        "stdout should not contain an absolute path, got:\n{stdout}"
    );
}

/// Through the normal `md compose` path, a document opened as
/// `{{{NOTES}}}/inbox/…` (which composes to the literal `{{NOTES}}`
/// file-reference anchor) takes `$NOTES` as its tree root: an in-tree
/// `../b.md` composes, and a link that leaves `$NOTES` stops the run with the
/// boundary error instead of reading the file outside it.
#[test]
fn compose_env_anchored_child_is_bounded_by_the_variable() {
    let fixture = CliProcessFixture::named("compose_env_anchored_child_is_bounded_by_the_variable");
    let root = fixture.workspace_path().join("anchor");
    let work = root.join("work");
    let notes = root.join("notes");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(notes.join("inbox")).unwrap();
    std::fs::write(notes.join("b.md"), "in-tree-content\n").unwrap();
    std::fs::write(notes.join("inbox/a.md"), "::file ../b.md\n").unwrap();
    std::fs::write(notes.join("inbox/escape.md"), "::file ../../outside.md\n").unwrap();
    std::fs::write(root.join("outside.md"), "outside-content\n").unwrap();
    std::fs::write(work.join("inside.md"), "::file \"{{{NOTES}}}/inbox/a.md\"\n").unwrap();
    std::fs::write(work.join("escape.md"), "::file \"{{{NOTES}}}/inbox/escape.md\"\n").unwrap();
    let compose = |document: &str| {
        fixture
            .command_builder()
            .plain_terminal(400, 50)
            .build()
            .env("NOTES", &notes)
            .arg("compose")
            .arg(work.join(document))
            .output()
            .unwrap()
    };

    let inside = compose("inside.md");
    let stdout = String::from_utf8_lossy(&inside.stdout);
    assert!(
        inside.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&inside.stderr)
    );
    assert!(stdout.contains("in-tree-content"), "{stdout}");

    let escape = compose("escape.md");
    let stdout = String::from_utf8_lossy(&escape.stdout);
    let stderr = String::from_utf8_lossy(&escape.stderr);
    let collapsed = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(!escape.status.success(), "stdout: {stdout}");
    assert!(!stdout.contains("outside-content"), "{stdout}");
    assert!(
        collapsed.contains("relative reference `../../outside.md` leaves file tree"),
        "{collapsed}"
    );
}

/// `md compose` launched from a package directory resolves `&` and `^` in a
/// document two directories down.
///
/// The CLI runs pre-flight before the compose pipeline. When only the
/// pipeline prepared the repository-aware context, pre-flight's resolver had
/// no repository root and every `&`/`^` target failed with "requires a
/// repository containing reference CWD". The `^pkg/...` directive is the
/// shape of the original report (`^claudine/docs/cli/index.md` from
/// `claudine/`).
#[test]
fn test_compose_repository_sigils_from_a_nested_document() {
    let fixture = CliProcessFixture::named("test_compose_repository_sigils_from_a_nested_document");
    let repo = fixture.workspace_path().join("repo");
    assert!(fixture.initialize_repository_at(&repo));
    let launch_dir = repo.join("pkg");
    for (relative, content) in [
        ("amp-target.md", "AMP-TARGET-BODY\n"),
        ("caret-target.md", "CARET-TARGET-BODY\n"),
        ("pkg/docs/cli/index.md", "CLI-INDEX-BODY\n"),
        (
            "pkg/docs/guide/doc.md",
            "# Guide\n\n::file &amp-target.md\n\n::file ^caret-target.md\n\n::file ^pkg/docs/cli/index.md\n",
        ),
    ] {
        common::write(&repo.join(relative), content);
    }

    let output = fixture
        .command_builder()
        .ambient_context(&launch_dir)
        .build()
        .arg("compose")
        .arg("docs/guide/doc.md")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "compose must succeed\nstdout:\n{stdout}\nstderr:\n{stderr}");
    for body in ["AMP-TARGET-BODY", "CARET-TARGET-BODY", "CLI-INDEX-BODY"] {
        assert!(stdout.contains(body), "missing {body} in stdout:\n{stdout}");
    }
    assert!(!stdout.contains("::file"), "a directive leaked into stdout:\n{stdout}");
    assert!(
        !stderr.contains("requires a repository containing reference CWD"),
        "stderr:\n{stderr}"
    );
}
