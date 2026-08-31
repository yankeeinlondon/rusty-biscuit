mod common;

use common::md_cmd;
use predicates::prelude::*;

fn initialize_repository(root: &std::path::Path) {
    let git_dir = root.join(".git");
    std::fs::create_dir_all(git_dir.join("objects")).unwrap();
    std::fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        git_dir.join("config"),
        "[core]\nrepositoryformatversion = 0\nbare = false\n",
    )
    .unwrap();
}

#[test]
fn test_compose_set_variables_available_during_validation() {
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

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .args(["--set", r#"{"plan":"my-plan.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn compose_with_the_shipped_baseline_renders_ctx_cwd_from_the_launch_directory() {
    let launch = tempfile::tempdir().unwrap();
    let prompt = launch.path().join("cwd.md");
    std::fs::write(&prompt, "Launch: {{ ctx.cwd }}\n").unwrap();

    // ctx.cwd carries the child's `current_dir()` spelling: symlink-resolved
    // on Unix (macOS /var/folders), exactly as-launched on Windows, where CI
    // tempdirs use 8.3 short names that `canonicalize` would re-spell.
    #[cfg(windows)]
    let expected_launch = launch.path().to_path_buf();
    #[cfg(not(windows))]
    let expected_launch =
        std::fs::canonicalize(launch.path()).expect("canonical launch directory");

    md_cmd()
        .current_dir(launch.path())
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

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .args(["--state", r#"{"doc":"readme.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Readme content."));
}


#[test]
fn test_compose_link_relative_same_repo() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    initialize_repository(&repo);
    let docs = repo.join("docs");
    let assets = repo.join("assets");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(&assets).unwrap();

    let source_file = docs.join("source.md");
    let logo_file = assets.join("logo.png");
    std::fs::write(&source_file, "# Source\n\n![img](../assets/logo.png)\n").unwrap();
    std::fs::write(&logo_file, "png").unwrap();

    let output = md_cmd().arg("compose").arg(&source_file).output().unwrap();

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
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    initialize_repository(&repo);

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

    let output = md_cmd().arg("compose").arg(&parent_file).output().unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("../components/sibling.md"),
        "stdout should contain normalized sibling path relative to parent, got:\n{stdout}"
    );
    // Should not contain absolute path
    let abs_sibling = std::fs::canonicalize(&sibling_file).unwrap();
    assert!(
        !stdout.contains(abs_sibling.to_string_lossy().as_ref()),
        "stdout should not contain absolute path, got:\n{stdout}"
    );
}

#[test]
fn test_compose_env_var_substitution_one_warning() {
    let dir = tempfile::tempdir().unwrap();
    let project_root = dir.path().join("project");
    std::fs::create_dir_all(&project_root).unwrap();
    let target_file = project_root.join("config.json");
    std::fs::write(&target_file, "{}").unwrap();

    let abs_target = std::fs::canonicalize(&target_file).unwrap();
    let abs_root = std::fs::canonicalize(&project_root).unwrap();
    let abs_target_markdown = abs_target.to_string_lossy();
    #[cfg(windows)]
    let abs_target_markdown = abs_target_markdown
        .strip_prefix(r"\\?\")
        .unwrap_or(&abs_target_markdown)
        .replace('\\', "/");
    #[cfg(not(windows))]
    let abs_target_markdown = abs_target_markdown.into_owned();

    let md_file = dir.path().join("test.md");
    std::fs::write(&md_file, format!("[config]({abs_target_markdown})\n")).unwrap();

    let output = md_cmd()
        .env("PROJECT_ROOT", &abs_root)
        .arg("compose")
        .arg(&md_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    eprintln!("DEBUG stdout:\n{stdout}");
    eprintln!("DEBUG stderr:\n{stderr}");

    #[cfg(not(windows))]
    assert!(
        stdout.contains("${PROJECT_ROOT}/config.json"),
        "stdout should contain env-var abstraction, got:\n{stdout}"
    );
    // Windows places its temp directory beneath the user profile. The
    // normalization contract prefers the home abstraction over environment
    // variables, so this fixture is represented with `~/` on Windows.
    #[cfg(windows)]
    assert!(
        stdout.contains("~/") && stdout.contains("/project/config.json"),
        "stdout should contain the higher-priority home abstraction, got:\n{stdout}"
    );
    // Warning text should NOT be in stdout
    assert!(
        !stdout.contains("environment variable"),
        "stdout should not contain warning text, got:\n{stdout}"
    );

    // Unix temp directories are outside the home directory, so the env-var
    // abstraction is selected and emits one warning. On Windows the
    // higher-priority home abstraction emits no warning.
    let warning_count = stderr.matches("environment variable").count();
    #[cfg(not(windows))]
    assert_eq!(
        warning_count, 1,
        "stderr should contain exactly one env-var warning, got {warning_count} occurrences:\n{stderr}"
    );
    #[cfg(windows)]
    assert_eq!(
        warning_count, 0,
        "home abstraction should not emit an env-var warning, got:\n{stderr}"
    );
}

#[test]
fn test_compose_html_spaced_attributes() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    initialize_repository(&repo);

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

    let output = md_cmd().arg("compose").arg(&page_file).output().unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("other.md"),
        "stdout should contain normalized other.md path, got:\n{stdout}"
    );
    assert!(
        stdout.contains("img.png"),
        "stdout should contain normalized img.png path, got:\n{stdout}"
    );
    // Should not contain the spaced attribute syntax unprocessed
    assert!(
        !stdout.contains("href = \"./other.md\""),
        "stdout should not contain unprocessed spaced href, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("src = \"./img.png\""),
        "stdout should not contain unprocessed spaced src, got:\n{stdout}"
    );
}
