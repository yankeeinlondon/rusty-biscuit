//! CLI coverage for repository-scoped trigger schemas.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

fn initialize_repository(root: &Path) {
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

fn fixture(process: &CliProcessFixture, document_frontmatter: &str) -> PathBuf {
    let root = process.cwd();
    initialize_repository(root);
    write(
        root,
        "schemas/prompt.trigger.yaml",
        "kind: trigger-schema\nmatch:\n  kind: enum(prompt; required)\n$schema: prompt.yaml\n",
    );
    write(
        root,
        "schemas/prompt.yaml",
        "$schema:\n  owner: string(required)\n",
    );
    write(
        root,
        "docs/prompt.md",
        &format!("---\n{document_frontmatter}\n---\nBody\n"),
    )
}

fn dialect_fixture(process: &CliProcessFixture) {
    let root = process.cwd();
    initialize_repository(root);
    let source = biscuit_test_harness::manifest_dir!().join("../tests/fixtures/schema-triggers");
    for directory in ["schemas", "docs"] {
        for entry in std::fs::read_dir(source.join(directory)).unwrap() {
            let entry = entry.unwrap();
            write(
                root,
                &format!("{directory}/{}", entry.file_name().to_string_lossy()),
                &std::fs::read_to_string(entry.path()).unwrap(),
            );
        }
    }
}

#[test]
fn schema_validate_honors_triggers_and_raw_mode() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: prompt");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&document)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
    process
        .command()
        .args(["schema", "validate", "--no-trigger-schemas"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn schema_validate_re_resolves_after_assignments() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: note");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&document)
        .arg("kind=prompt")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn compose_honors_triggers_and_raw_mode() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: prompt");
    process
        .command()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"));
    process
        .command()
        .args(["compose", "--no-trigger-schemas"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn compose_re_resolves_trigger_after_shell_value_becomes_concrete() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: $(echo prompt)");
    write(
        process.cwd(),
        "docs/.darkmatter-shell-whitelist",
        "exact echo prompt\n",
    );
    // `$(echo prompt)` is executed as a program, and on Windows `echo` exists
    // only as Git's echo.exe outside System32, so the host PATH is declared.
    process
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"));
}

#[test]
fn triggers_command_prints_shared_trace() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: note");
    process
        .command()
        .args(["schema", "triggers"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("Schema roots"))
        .stdout(predicate::str::contains("prompt.trigger.yaml"))
        .stdout(predicate::str::contains("arm 1"))
        .stdout(predicate::str::contains("defeated"));
}

#[test]
fn sibling_only_bare_reference_suggests_explicit_relative_path() {
    let process = CliProcessFixture::new();
    initialize_repository(process.cwd());
    write(
        process.cwd(),
        "schemas/placeholder.yaml",
        "$schema:\n  title: string\n",
    );
    write(
        process.cwd(),
        "docs/local.yaml",
        "$schema:\n  title: string(required)\n",
    );
    let document = write(
        process.cwd(),
        "docs/doc.md",
        "---\n$schema: local.yaml\ntitle: Test\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(document)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("./local.yaml"));
}

#[test]
fn dialect_family_has_identical_compose_validate_and_trace_activation() {
    let process = CliProcessFixture::new();
    dialect_fixture(&process);
    for (document, trigger, required) in [
        ("claudine.md", "claudine.trigger.yaml", "provider"),
        ("inline-compose.md", "inline-compose.trigger.yaml", "output"),
        ("sequence.md", "sequence.trigger.yaml", "sequence_name"),
    ] {
        let path = process.cwd().join("docs").join(document);
        process
            .command()
            .args(["schema", "validate"])
            .arg(&path)
            .assert()
            .code(1)
            .stdout(predicate::str::contains(required));
        process
            .command()
            .arg("compose")
            .arg(&path)
            .assert()
            .failure()
            .stderr(predicate::str::contains(required));
        process
            .command()
            .args(["schema", "triggers"])
            .arg(&path)
            .assert()
            .success()
            .stdout(predicate::str::contains(trigger))
            .stdout(predicate::str::contains("matched"));
    }

    let plain = process.cwd().join("docs/plain.md");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&plain)
        .assert()
        .success();
    process
        .command()
        .arg("compose")
        .arg(&plain)
        .assert()
        .success();
}

/// A Cargo workspace in the fixture repository holding package `area/pkg`
/// in package area `area`, so a document in the package has all five schema
/// roots.
fn monorepo(process: &CliProcessFixture) {
    let root = process.cwd();
    initialize_repository(root);
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"area/pkg\"]\n");
    write(
        root,
        "area/pkg/Cargo.toml",
        "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(root, "area/pkg/src/lib.rs", "");
}

fn path_trigger(pattern: &str, payload: &str) -> String {
    format!("kind: trigger-schema\nmatch:\n  $path: \"{pattern}\"\n$schema: {payload}\n")
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// `text` with every space and line break removed, so a root line the
/// terminal wrapped at a path's `-` reads as one run.
fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The offset of root `label` in the compacted output, asserting that its
/// line names `detail` before the next root.
fn root_line(stdout: &str, label: &str, detail: &str) -> usize {
    let text = compact(stdout);
    let label = compact(label);
    let start = text
        .find(&format!("{label}:"))
        .unwrap_or_else(|| panic!("no `{label}` root in:\n{stdout}"));
    let line = &text[start..];
    let detail = compact(detail);
    let found = line.find(&detail).unwrap_or_else(|| panic!("`{label}` lacks {detail:?} in:\n{stdout}"));
    // The detail belongs to this root: no later root label comes first.
    for other in ["packageroot:", "package-arearoot:", "filetreeroot:", "SCHEMAS_DIR:", "home:"] {
        if let Some(next) = line[label.len() + 1..].find(other) {
            assert!(found < label.len() + 1 + next, "`{label}` lacks {detail:?} in:\n{stdout}");
        }
    }
    start
}

#[test]
fn triggers_command_prints_the_five_schema_roots_in_search_order() {
    let process = CliProcessFixture::new();
    monorepo(&process);
    let defs = process.workspace_path().join("dm-defs");
    for folder in [
        process.cwd().join("area/pkg/schemas"),
        process.cwd().join("area/schemas"),
        process.cwd().join("schemas"),
        defs.clone(),
    ] {
        std::fs::create_dir_all(folder).unwrap();
    }
    let document = write(process.cwd(), "area/pkg/docs/guide.md", "---\ntitle: x\n---\n");

    let output = process
        .command_builder()
        .plain_terminal(1000, 50)
        .application_input("SCHEMAS_DIR", &defs)
        .build()
        .args(["schema", "triggers"])
        .arg(&document)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = stdout_of(&output);
    assert!(!stdout.contains("Boundary"), "{stdout}");
    let offsets = [
        root_line(&stdout, "package root", &format!("area{}pkg{}schemas", std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR)),
        root_line(&stdout, "package-area root", &format!("area{}schemas", std::path::MAIN_SEPARATOR)),
        root_line(&stdout, "file tree root", "schemas"),
        root_line(&stdout, "SCHEMAS_DIR", "dm-defs"),
        root_line(&stdout, "home", "(absent)"),
    ];
    assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]), "roots out of order:\n{stdout}");
}

#[test]
fn triggers_command_marks_unset_and_invalid_schemas_dir() {
    let process = CliProcessFixture::new();
    monorepo(&process);
    let document = write(process.cwd(), "docs/doc.md", "---\ntitle: x\n---\n");

    let unset = process
        .command_builder()
        .plain_terminal(1000, 50)
        .application_input_removed("SCHEMAS_DIR")
        .build()
        .args(["schema", "triggers"])
        .arg(&document)
        .output()
        .unwrap();
    let stdout = stdout_of(&unset);
    root_line(&stdout, "package root", "not in a package");
    root_line(&stdout, "SCHEMAS_DIR", "unset");

    for (value, verdict) in [("", "invalid (empty)"), ("dm-defs", "invalid (not an absolute path)")] {
        let output = process
            .command_builder()
            .plain_terminal(1000, 50)
            .application_input("SCHEMAS_DIR", value)
            .build()
            .args(["schema", "triggers"])
            .arg(&document)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        root_line(&stdout_of(&output), "SCHEMAS_DIR", verdict);
    }
}

#[test]
fn schema_validate_applies_triggers_from_schemas_dir_and_home() {
    let process = CliProcessFixture::new();
    monorepo(&process);
    let defs = process.workspace_path().join("dm-defs");
    write(&defs, "docs.trigger.yaml", &path_trigger("docs/*.md", "team.yaml"));
    write(&defs, "team.yaml", "$schema:\n  team_owner: string(required)\n");
    write(process.home(), "schemas/notes.trigger.yaml", &path_trigger("&**/notes.md", "notes.yaml"));
    write(process.home(), "schemas/notes.yaml", "$schema:\n  note_owner: string(required)\n");
    let at_root = write(process.cwd(), "docs/notes.md", "---\ntitle: x\n---\n");
    let in_package = write(process.cwd(), "area/pkg/docs/notes.md", "---\ntitle: x\n---\n");

    let validate = |document: &Path, schemas_dir: Option<&Path>| {
        let builder = process.command_builder();
        let builder = match schemas_dir {
            Some(dir) => builder.application_input("SCHEMAS_DIR", dir),
            None => builder.application_input_removed("SCHEMAS_DIR"),
        };
        let output = builder.build().args(["schema", "validate"]).arg(document).output().unwrap();
        stdout_of(&output)
    };

    // The SCHEMAS_DIR trigger's bare `docs/*.md` is read from the repository
    // root; the home trigger's `&**/notes.md` reaches every depth.
    let stdout = validate(&at_root, Some(&defs));
    assert!(stdout.contains("team_owner") && stdout.contains("note_owner"), "{stdout}");
    let stdout = validate(&in_package, Some(&defs));
    assert!(!stdout.contains("team_owner") && stdout.contains("note_owner"), "{stdout}");
    // Without SCHEMAS_DIR its folder is not searched.
    let stdout = validate(&at_root, None);
    assert!(!stdout.contains("team_owner") && stdout.contains("note_owner"), "{stdout}");
}

/// `text` without SGR escape sequences (`ESC [ ... m`).
#[cfg(unix)]
fn strip_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            chars.by_ref().find(|c| *c == 'm');
        } else {
            plain.push(c);
        }
    }
    plain
}

/// A `SCHEMAS_DIR` folder behind an ancestor `md` cannot search is an I/O
/// error naming it from `schema validate` and `schema triggers`; home's
/// same-named `policy.yaml` never stands in for it.
#[cfg(unix)]
#[test]
fn an_inaccessible_schema_root_fails_validate_and_triggers() {
    use std::os::unix::fs::PermissionsExt;

    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    let process = CliProcessFixture::new();
    monorepo(&process);
    let locked = process.workspace_path().join("locked");
    let schemas = locked.join("anchor/schemas");
    write(&schemas, "policy.yaml", "$schema:\n  hidden_rule: string(required)\n");
    write(&schemas, "hidden.trigger.yaml", &path_trigger("**/*.md", "policy.yaml"));
    write(process.home(), "schemas/policy.yaml", "$schema:\n  fallback_rule: string\n");
    let document = write(process.cwd(), "docs/doc.md", "---\n$schema: policy.yaml\n---\nBody\n");
    let run = |subcommand: &str| {
        process
            .command_builder()
            .plain_terminal(1000, 50)
            .application_input("SCHEMAS_DIR", &schemas)
            .build()
            .args(["schema", subcommand])
            .arg(&document)
            .output()
            .unwrap()
    };

    // Readable control: the preferred `policy.yaml` applies.
    let control = run("validate");
    assert_eq!(control.status.code(), Some(1), "{control:?}");
    assert!(stdout_of(&control).contains("hidden_rule"), "{control:?}");

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let _restore = Restore(locked.clone());
    if std::fs::read_dir(&locked).is_ok() {
        eprintln!("skipping: {} is still listable after chmod 000", locked.display());
        return;
    }
    for subcommand in ["validate", "triggers"] {
        let output = run(subcommand);
        let text = compact(&strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )))
        .replace('┃', "");
        assert!(!output.status.success(), "{subcommand}: {output:?}");
        assert!(text.contains("PermissionDenied"), "{subcommand}: {output:?}");
        assert!(text.contains(&compact(&schemas.display().to_string())), "{subcommand}: {output:?}");
        assert!(!text.contains("fallback_rule") && !text.contains("Schemaroots"), "{subcommand}: {output:?}");
    }
}

#[test]
fn schema_validate_reports_a_forbidden_path_prefix_naming_the_pattern() {
    let process = CliProcessFixture::new();
    monorepo(&process);
    write(process.cwd(), "schemas/bad.trigger.yaml", &path_trigger("@prompts/**", "payload.yaml"));
    write(process.cwd(), "schemas/payload.yaml", "$schema:\n  owner: string\n");
    let document = write(process.cwd(), "docs/doc.md", "---\ntitle: x\n---\n");

    let output = process
        .command_builder()
        .plain_terminal(1000, 50)
        .build()
        .args(["schema", "validate"])
        .arg(&document)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let text = format!("{}{}", stdout_of(&output), String::from_utf8_lossy(&output.stderr));
    assert!(text.contains("bad.trigger.yaml"), "{text}");
    assert!(text.contains("`@prompts/**`"), "{text}");
}

#[test]
fn shipped_external_schema_example_names_its_sibling_explicitly() {
    let shipped = biscuit_test_harness::manifest_dir!().join("../example-docs/schemas");
    let document_text = std::fs::read_to_string(shipped.join("external.md")).unwrap();
    assert!(document_text.contains("$schema: ./external.yaml"), "{document_text}");

    // The shipped pair validates in a repository whose root has a schemas
    // folder, where a bare `external.yaml` would be looked up in the roots
    // only: the failure is the example's own missing `name`.
    let process = CliProcessFixture::new();
    initialize_repository(process.cwd());
    std::fs::create_dir_all(process.cwd().join("schemas")).unwrap();
    let document = write(process.cwd(), "example-docs/schemas/external.md", &document_text);
    write(
        process.cwd(),
        "example-docs/schemas/external.yaml",
        &std::fs::read_to_string(shipped.join("external.yaml")).unwrap(),
    );
    process
        .command()
        .args(["schema", "validate"])
        .arg(&document)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("name"));
}
