use super::*;
use super::expr::{SourceExpressionLookup, render_interpolated};
use super::grammar::{SequenceSourceSpec, SourceOperator, classify_source};
use super::source::resolve_sequence_reference;
use serde_json::Value;
use crate::composition::error::SequenceLoadCause;
use darkmatter::markdown::{Frontmatter, Markdown};
use serde_json::json;
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

use crate::composition::types::ResolvedCompositionSource;

fn make_source(
    dir: &TempDir,
    frontmatter: &[(&str, serde_json::Value)],
    content: &str,
) -> ResolvedCompositionSource {
    let file = dir.path().join("test.md");
    let mut fm = Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = Markdown::with_frontmatter(fm, content);
    fs::write(&file, md.as_string()).unwrap();

    let original_text = fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_str().unwrap().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

fn init_git_repo(path: &Path) {
    assert!(
        Command::new("git")
            .arg("init")
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
}

// -- resolve_sequence_plan: no sequence key --------------------------------

#[test]
fn no_sequence_key_returns_none() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Content");
    let result = resolve_sequence_plan(&source).unwrap();
    assert!(result.is_none());
}

// -- resolve_sequence_plan: inline scalar list ----------------------------

#[test]
fn inline_scalar_list_normalizes_correctly() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("sequence", json!(["one", "two", "three"]))],
        "Prompt: {{state}}",
    );
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert!(matches!(plan.source, SequenceSource::Inline));
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0].name, "one");
    assert_eq!(plan.steps[1].name, "two");
    assert_eq!(plan.steps[2].name, "three");
    assert_eq!(plan.steps[0].raw_state, json!("one"));
    assert!(plan.document_fail_fast); // default
}

// -- resolve_sequence_plan: inline object list ----------------------------

#[test]
fn inline_object_list_requires_name() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "sequence",
            json!([
                {"name": "one", "color": "red"},
                {"name": "two", "color": "blue"}
            ]),
        )],
        "Prompt",
    );
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].name, "one");
    assert_eq!(
        plan.steps[0].raw_state,
        json!({"name": "one", "color": "red"})
    );
}

#[test]
fn inline_object_step_missing_name_fails() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("sequence", json!([{"color": "red"}]))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceStepNameMissing { index: 0 }),
        "got: {err}"
    );
}

#[test]
fn inline_object_step_name_wrong_type_fails() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("sequence", json!([{"name": 42}]))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SequenceStepNameWrongType { index: 0, .. }
        ),
        "got: {err}"
    );
}

// -- resolve_sequence_plan: empty list ------------------------------------

#[test]
fn empty_list_fails() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("sequence", json!([]))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(matches!(err, CompositionError::SequenceEmpty), "got: {err}");
}

// -- resolve_sequence_plan: invalid type -----------------------------------

#[test]
fn invalid_sequence_type_fails() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("sequence", json!(42))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceInvalid(_)),
        "got: {err}"
    );
}

// -- resolve_sequence_plan: fail_fast frontmatter -------------------------

#[test]
fn fail_fast_false_from_frontmatter() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("sequence", json!(["one", "two"])),
            ("fail_fast", json!(false)),
        ],
        "Prompt",
    );
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert!(!plan.document_fail_fast);
}

// -- resolve_sequence_plan: external YAML (sequence: form) ----------------

#[test]
fn external_sequence_form_loads() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("steps.yaml");
    fs::write(
        &yaml_path,
        "sequence:\n  - name: alpha\n    color: red\n  - name: beta\n    color: blue\n",
    )
    .unwrap();

    let source = make_source(
        &dir,
        &[("sequence", json!("steps.yaml"))],
        "Prompt: {{state.name}}",
    );
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert!(matches!(plan.source, SequenceSource::External { .. }));
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].name, "alpha");
}

// -- resolve_sequence_plan: formal sequence documents + templates ---------

#[test]
fn external_sequence_template_loads_and_applies_templates() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("agents.yaml");
    fs::write(
        &yaml_path,
        r#"kind: sequence
template:
  desc: "{{name}} (site: {{site}})"
sequence:
  - name: Claude Code
    site: https://code.claude.com
  - name: Codex CLI
    site: https://codex.openai.com
"#,
    )
    .unwrap();

    let source = make_source(
        &dir,
        &[("sequence", json!("agents.yaml"))],
        "Research {{state.name}}",
    );
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].name, "Claude Code");

    // Template should have been applied
    let desc0 = plan.steps[0]
        .raw_state
        .as_object()
        .unwrap()
        .get("desc")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(desc0, "Claude Code (site: https://code.claude.com)");
}

#[test]
fn external_template_with_fallback_default() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("items.yaml");
    fs::write(
        &yaml_path,
        r#"kind: sequence
template:
  summary: "{{name}} - repo: {{repo || 'n/a'}}"
sequence:
  - name: Tool A
    repo: https://github.com/a
  - name: Tool B
"#,
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("items.yaml"))], "Prompt");
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    let summary0 = plan.steps[0]
        .raw_state
        .as_object()
        .unwrap()
        .get("summary")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(summary0, "Tool A - repo: https://github.com/a");

    let summary1 = plan.steps[1]
        .raw_state
        .as_object()
        .unwrap()
        .get("summary")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(summary1, "Tool B - repo: n/a");
}

#[test]
fn external_template_reserved_key_collision_fails() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("bad.yaml");
    fs::write(
        &yaml_path,
        r#"kind: sequence
template:
  state: "{{name}}"
sequence:
  - name: One
"#,
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("bad.yaml"))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceReservedTemplateKey(ref k) if k == "state"),
        "got: {err}"
    );
}

#[test]
fn external_template_non_object_fails() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("bad.yaml");
    fs::write(
        &yaml_path,
        "kind: sequence\ntemplate: not-an-object\nsequence:\n  - name: One\n",
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("bad.yaml"))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceExternalWrongType(ref msg) if msg.contains("`template`")),
        "got: {err}"
    );
}

/// The retired `list:` form paired `template:` with `kind: sequence`; the
/// ratified single shape pairs it with `sequence:`. A `kind`-less document
/// carrying both is the plainest expression of that — and it must now be
/// accepted, where the pre-Sequence-Plus loader rejected it.
#[test]
fn template_is_supported_without_an_explicit_kind() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("templated.yaml");
    fs::write(
        &yaml_path,
        r#"sequence:
  - name: One
template:
  desc: "{{name}}!"
"#,
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("templated.yaml"))], "Prompt");
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert_eq!(plan.steps[0].state.extra["desc"], json!("One!"));
}

#[test]
fn fail_fast_wrong_type_fails() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("sequence", json!(["one", "two"])),
            ("fail_fast", json!("false")),
        ],
        "Prompt",
    );
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceInvalid(ref msg) if msg.contains("fail_fast")),
        "got: {err}"
    );
}

#[test]
fn relative_path_resolves_from_source_dir() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    let yaml_path = sub.join("steps.yaml");
    fs::write(&yaml_path, "sequence:\n  - alpha\n  - beta\n").unwrap();

    // Source file lives in subdir; sequence reference is relative.
    let file = sub.join("doc.md");
    let md_text = "---\nsequence: steps.yaml\n---\nBody\n";
    fs::write(&file, md_text).unwrap();

    let original_text = fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    let source = ResolvedCompositionSource {
        original_ref: file.to_str().unwrap().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    };

    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].name, "alpha");
}

#[test]
fn absolute_path_is_honored() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("abs.yaml");
    fs::write(&yaml_path, "sequence:\n  - one\n").unwrap();

    // Source file lives in a DIFFERENT directory from the YAML.
    let other = dir.path().join("elsewhere");
    fs::create_dir(&other).unwrap();
    let file = other.join("doc.md");
    let md_text = format!(
        "---\nsequence: {}\n---\nBody\n",
        yaml_path.to_str().unwrap()
    );
    fs::write(&file, &md_text).unwrap();

    let original_text = fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    let source = ResolvedCompositionSource {
        original_ref: file.to_str().unwrap().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    };

    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].name, "one");
}

#[test]
fn magic_reference_resolves_from_source_git_root() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    let prompts_dir = dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir).unwrap();
    let target = prompts_dir.join("steps.yaml");
    fs::write(&target, "sequence:\n  - alpha\n").unwrap();

    let source_path = dir.path().join("docs/guide.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    let resolved = resolve_sequence_reference("@prompts/steps.yaml", &source_path).unwrap();
    assert_eq!(
        resolved.canonicalize().unwrap(),
        target.canonicalize().unwrap()
    );
}

#[test]
fn package_reference_resolves_from_current_package_area() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["claudine/lib"]
"#,
    )
    .unwrap();

    let package_root = dir.path().join("claudine");
    fs::create_dir_all(package_root.join("lib/src")).unwrap();
    fs::write(
        package_root.join("lib/Cargo.toml"),
        r#"[package]
name = "claudine"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(package_root.join("lib/src/lib.rs"), "pub fn run() {}\n").unwrap();

    let target = package_root.join("README.md");
    fs::write(&target, "# Claudine\n").unwrap();

    let source_path = package_root.join("lib/docs/guide.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    let resolved = resolve_sequence_reference("!README.md", &source_path).unwrap();
    assert_eq!(
        resolved.canonicalize().unwrap(),
        target.canonicalize().unwrap()
    );
}

/// The adapter captures the package area at the request boundary (via `sniff`)
/// and supplies it on the explicit context, so a `!` reference resolves under
/// the package area even when a same-named file also sits at the repository
/// root. A plain repository-root fallback would have selected the root twin;
/// the captured package-area anchor is authoritative (D2).
#[test]
fn package_reference_uses_captured_package_area_over_repository_root() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["claudine/lib"]
"#,
    )
    .unwrap();

    let package_root = dir.path().join("claudine");
    fs::create_dir_all(package_root.join("lib/src")).unwrap();
    fs::write(
        package_root.join("lib/Cargo.toml"),
        r#"[package]
name = "claudine"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(package_root.join("lib/src/lib.rs"), "pub fn run() {}\n").unwrap();

    // Same basename at the repository root and the package area. Package
    // resolution must select the package-area copy, proving the captured area
    // anchor is used rather than a repository-root fallback.
    fs::write(dir.path().join("steps.yaml"), "sequence:\n  - root\n").unwrap();
    let target = package_root.join("steps.yaml");
    fs::write(&target, "sequence:\n  - pkg\n").unwrap();

    let source_path = package_root.join("lib/docs/guide.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    let resolved = resolve_sequence_reference("!steps.yaml", &source_path).unwrap();
    assert_eq!(
        resolved.canonicalize().unwrap(),
        target.canonicalize().unwrap(),
        "package reference must resolve under the captured package area, not the repository root",
    );
}

#[test]
#[serial]
fn vault_reference_uses_vault_environment_roots() {
    let dir = TempDir::new().unwrap();
    let vault_root = dir.path().join("vault");
    let target = vault_root.join("notes/steps.yaml");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "sequence:\n  - alpha\n").unwrap();

    let source_path = dir.path().join("docs/source.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    unsafe {
        std::env::set_var("VAULT", &vault_root);
    }
    let resolved = resolve_sequence_reference("vault:notes/steps.yaml", &source_path).unwrap();
    unsafe {
        std::env::remove_var("VAULT");
    }

    assert_eq!(resolved, target);
}

#[test]
#[serial]
fn env_reference_expands_environment_variables() {
    let dir = TempDir::new().unwrap();
    let target_dir = dir.path().join("shared");
    let target = target_dir.join("steps.yaml");
    fs::create_dir_all(&target_dir).unwrap();
    fs::write(&target, "sequence:\n  - alpha\n").unwrap();

    let source_path = dir.path().join("docs/source.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    unsafe {
        std::env::set_var("SEQ_ROOT", &target_dir);
    }
    let resolved = resolve_sequence_reference("{{SEQ_ROOT}}/steps.yaml", &source_path).unwrap();
    unsafe {
        std::env::remove_var("SEQ_ROOT");
    }

    assert_eq!(resolved, target);
}

/// A `~`-prefixed sequence reference now resolves through [`FileReference`]'s
/// shared `Home` kind (D11) rather than a private tilde expansion, so it is
/// home-pinned AND existence-checked: an existing `~/steps.yaml` resolves to
/// the home-relative path.
#[test]
#[serial]
fn tilde_reference_expands_against_home_directory() {
    let dir = TempDir::new().unwrap();
    let home_dir = dir.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();
    // The `Home` kind probes the filesystem: the target must exist to match.
    fs::write(home_dir.join("steps.yaml"), "sequence:\n  - alpha\n").unwrap();

    let source_path = dir.path().join("docs/source.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    unsafe {
        std::env::set_var("HOME", &home_dir);
    }
    let resolved = resolve_sequence_reference("~/steps.yaml", &source_path).unwrap();
    unsafe {
        std::env::remove_var("HOME");
    }

    assert_eq!(resolved, home_dir.join("steps.yaml"));
}

/// An implicit (bare) external sequence reference is repository-first (D4):
/// with the same basename at the repository root and the source directory, the
/// repository-root copy wins. The migration replaced the old source-relative
/// join that never tried the repository root.
#[test]
fn implicit_reference_is_repository_first() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    // Same basename at the repo root and the source directory.
    fs::write(dir.path().join("steps.yaml"), "sequence:\n  - repo\n").unwrap();
    let source_path = dir.path().join("docs/guide.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(
        source_path.parent().unwrap().join("steps.yaml"),
        "sequence:\n  - source\n",
    )
    .unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    let resolved = resolve_sequence_reference("steps.yaml", &source_path).unwrap();
    assert_eq!(
        resolved.canonicalize().unwrap(),
        dir.path().join("steps.yaml").canonicalize().unwrap(),
        "implicit reference must resolve repository-first, not source-relative",
    );
}

/// An explicit `./` external sequence reference is pinned to the source
/// directory with no repository-root fallback — the explicit/implicit
/// distinction the private grammar collapsed.
#[test]
fn explicit_dot_slash_is_source_relative_only() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    // Only the repository root holds the file; the source directory does not.
    fs::write(dir.path().join("steps.yaml"), "sequence:\n  - repo\n").unwrap();
    let source_path = dir.path().join("docs/guide.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    let err = resolve_sequence_reference("./steps.yaml", &source_path).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SequenceExternalLoad {
                source: SequenceLoadCause::NotFound,
                ..
            }
        ),
        "explicit `./` must be source-relative only and must not fall back to the repo root: {err:?}",
    );
}

/// `@/x` normalizes to `@x` (magic-root search) — the normalization is
/// preserved as explicit `FileReference` input, resolving identically to `@x`.
#[test]
fn at_slash_normalizes_to_magic_search() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    let prompts_dir = dir.path().join("prompts");
    fs::create_dir_all(&prompts_dir).unwrap();
    fs::write(prompts_dir.join("steps.yaml"), "sequence:\n  - alpha\n").unwrap();
    let source_path = dir.path().join("docs/guide.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    let via_at = resolve_sequence_reference("@prompts/steps.yaml", &source_path).unwrap();
    let via_at_slash = resolve_sequence_reference("@/prompts/steps.yaml", &source_path).unwrap();
    assert_eq!(
        via_at.canonicalize().unwrap(),
        via_at_slash.canonicalize().unwrap(),
        "`@/prompts/...` must resolve identically to `@prompts/...`",
    );
}

#[test]
#[serial]
fn missing_environment_variable_surface_is_preserved() {
    let dir = TempDir::new().unwrap();
    let source_path = dir.path().join("docs/source.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();

    unsafe {
        std::env::remove_var("SEQ_ROOT");
    }
    let error =
        resolve_sequence_reference("{{SEQ_ROOT}}/steps.yaml", &source_path).unwrap_err();

    assert!(
        matches!(
            error,
            CompositionError::SequenceExternalLoad {
                source: SequenceLoadCause::Reference(_),
                ..
            }
        ),
        "expected a typed Reference cause, got: {error:?}"
    );
    assert!(
        error.to_string().contains("SEQ_ROOT"),
        "message should still name the unresolved env var: {error}"
    );
}

#[test]
fn external_template_non_string_value_fails() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("bad.yaml");
    // `rank` is a non-reserved template key with a non-string value; the
    // wrong-type check must fire (a reserved key like `count` would trip the
    // reserved-key check first — see `external_template_reserved_key_fails`).
    fs::write(
        &yaml_path,
        "kind: sequence\ntemplate:\n  rank: 42\nsequence:\n  - name: One\n",
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("bad.yaml"))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceTemplateWrongType { .. }),
        "got: {err}"
    );
}

#[test]
fn external_malformed_yaml_carries_typed_yaml_cause() {
    use std::error::Error as _;

    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("broken.yaml");
    // Unbalanced flow mapping: a YAML parse failure, surfaced by Yaml::new.
    fs::write(&yaml_path, "sequence: [unterminated\n").unwrap();

    let source = make_source(&dir, &[("sequence", json!("broken.yaml"))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SequenceExternalLoad {
                source: SequenceLoadCause::Yaml(_),
                ..
            }
        ),
        "expected a typed Yaml cause, got: {err:?}"
    );

    let load_cause = err
        .source()
        .and_then(|s| s.downcast_ref::<SequenceLoadCause>())
        .expect("source must be a SequenceLoadCause");
    assert!(matches!(load_cause, SequenceLoadCause::Yaml(_)));
}

#[test]
#[serial]
fn external_missing_file_reference_yields_not_found() {
    let dir = TempDir::new().unwrap();
    let source_path = dir.path().join("docs/source.md");
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(&source_path, "---\n---\nbody\n").unwrap();
    init_git_repo(dir.path());

    // A magic reference that resolves to nothing under the source's git
    // scope surfaces the NotFound synthetic.
    let err =
        resolve_sequence_reference("@no-such-dir/missing.yaml", &source_path).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SequenceExternalLoad {
                source: SequenceLoadCause::NotFound,
                ..
            }
        ),
        "expected NotFound, got: {err:?}"
    );
}

#[test]
fn sequence_load_cause_home_dir_display() {
    // Triggering a missing HOME cross-platform is awkward; assert the
    // synthetic cause's Display directly.
    assert_eq!(
        SequenceLoadCause::HomeDir.to_string(),
        "unable to resolve home directory"
    );
    assert_eq!(SequenceLoadCause::NotFound.to_string(), "file not found");
}

// -- build_step_overlay ---------------------------------------------------

/// Normalize a list of scalar names into a plan for overlay assertions.
fn scalar_plan(names: &[&str]) -> SequencePlan {
    let items: Vec<serde_json::Value> = names.iter().map(|n| json!(n)).collect();
    normalize::normalize_plan(&items, SequenceSource::Inline, Path::new("/seq/doc.md"), true)
        .expect("scalar plan normalizes")
}

#[test]
fn overlay_for_single_step_sequence() {
    let plan = scalar_plan(&["only"]);
    let overlay = build_step_overlay(&plan, 0);
    assert!(overlay.state.is_first);
    assert!(overlay.state.is_last);
    assert_eq!(overlay.state.index, 1);
    assert_eq!(overlay.state.count, 1);
    // Absent neighbors are `None` (rendered as `null`), never empty-named states.
    assert!(overlay.previous.is_none());
    assert!(overlay.next.is_none());
}

#[test]
fn overlay_for_middle_step() {
    let plan = scalar_plan(&["a", "b", "c"]);
    let overlay = build_step_overlay(&plan, 1);
    assert!(!overlay.state.is_first);
    assert!(!overlay.state.is_last);
    assert_eq!(overlay.state.index, 2);
    assert_eq!(overlay.state.count, 3);
    assert_eq!(overlay.state.name, "b");
    assert_eq!(overlay.previous.as_ref().unwrap().name, "a");
    assert_eq!(overlay.next.as_ref().unwrap().name, "c");
}

// -- Sequence Plus: state normalization, ids, sequence_id, overlay ---------

#[test]
fn scalar_step_normalizes_to_named_state() {
    let plan = scalar_plan(&["alpha", "beta"]);
    let state = &plan.steps[0].state;
    assert_eq!(state.name, "alpha");
    assert_eq!(state.id, "alpha");
    assert_eq!(state.index, 1);
    assert_eq!(state.count, 2);
    assert!(state.is_first);
    assert!(!state.is_last);
    // Authored raw value is preserved verbatim for provenance.
    assert_eq!(plan.steps[0].raw_state, json!("alpha"));
}

#[test]
fn duplicate_names_get_deterministic_id_suffixes() {
    let plan = scalar_plan(&["Build", "build", "build"]);
    // Dasherized bases collide; the first keeps the base, later ones take the
    // lowest free `-<n>` starting at `-2`.
    assert_eq!(plan.steps[0].state.id, "build");
    assert_eq!(plan.steps[1].state.id, "build-2");
    assert_eq!(plan.steps[2].state.id, "build-3");
    // Names are unchanged; only the generated id disambiguates.
    assert_eq!(plan.steps[1].state.name, "build");
}

#[test]
fn dasherize_handles_punctuation_and_empty_fallback() {
    let plan = scalar_plan(&["Claude Code!", "***", "***"]);
    assert_eq!(plan.steps[0].state.id, "claude-code");
    // No alphanumerics → the `state` fallback, then dedup.
    assert_eq!(plan.steps[1].state.id, "state");
    assert_eq!(plan.steps[2].state.id, "state-2");
}

#[test]
fn sequence_id_is_lowercase_hex_and_copied_into_every_state() {
    let plan = scalar_plan(&["a", "b", "c"]);
    assert_eq!(plan.sequence_id.len(), 16);
    assert!(
        plan.sequence_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "sequence_id must be lowercase hex: {}",
        plan.sequence_id
    );
    for step in &plan.steps {
        assert_eq!(step.state.sequence_id, plan.sequence_id);
    }
    let overlay = build_step_overlay(&plan, 0);
    assert_eq!(overlay.sequence_id, plan.sequence_id);
}

#[test]
fn separate_invocations_get_distinct_sequence_ids() {
    let a = scalar_plan(&["a", "b"]);
    let b = scalar_plan(&["a", "b"]);
    // The monotonic counter guarantees distinct tokens even for identical input.
    assert_ne!(a.sequence_id, b.sequence_id);
}

#[test]
fn overlay_as_set_overrides_emits_new_root_keys_and_reserves_them() {
    let plan = scalar_plan(&["a", "b"]);
    let user_set = json!({"color": "red", "state": "should-lose", "sequence_id": "nope"});
    let overrides = build_step_overlay(&plan, 0).as_set_overrides(Some(user_set));
    let map = overrides.as_object().unwrap();

    // Always-present root keys.
    assert!(map["state"].is_object());
    assert_eq!(map["state"]["name"], json!("a"));
    assert_eq!(map["state"]["index"], json!(1));
    assert!(map["outputs"].is_array());
    assert_eq!(map["outputs"], json!([]));
    assert!(map["sequence_id"].is_string());
    // First step: previous is null, next is the second state object.
    assert!(map["previous"].is_null());
    assert_eq!(map["next"]["name"], json!("b"));

    // Reserved overlay keys always win over user setters.
    assert_eq!(map["state"]["name"], json!("a"));
    assert_ne!(map["sequence_id"], json!("nope"));
    // Non-reserved user keys survive.
    assert_eq!(map["color"], json!("red"));
}

#[test]
fn object_step_extracts_state_and_rejects_reserved_state_key() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "sequence",
            json!([{"name": "alpha", "id": "hand-picked"}]),
        )],
        "Prompt",
    );
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SequenceReservedStateKey { index: 0, ref key } if key == "id"
        ),
        "authoring the generated `id` key must be rejected, got: {err:?}"
    );
}

#[test]
fn object_step_rejects_two_executables() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "sequence",
            json!([{"name": "alpha", "shell": "just test", "prompt": "@p.md"}]),
        )],
        "Prompt",
    );
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceExclusiveExecutable { index: 0, .. }),
        "two executable fields must be rejected, got: {err:?}"
    );
}

#[test]
fn shell_step_rejects_prompt_only_task_option() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "sequence",
            json!([{"name": "alpha", "shell": "just test", "params": {"x": 1}}]),
        )],
        "Prompt",
    );
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SequenceInvalidTaskField { index: 0, ref field, ref executable }
                if field == "params" && executable == "shell"
        ),
        "`params` on a shell task must be rejected, got: {err:?}"
    );
}

#[test]
fn single_executable_step_extracts_executable_and_options() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "sequence",
            json!([{"name": "review", "prompt": "@p.md", "params": {"topic": "x"}, "color": "blue"}]),
        )],
        "Prompt",
    );
    let plan = resolve_sequence_plan(&source).unwrap().unwrap();
    let step = &plan.steps[0];
    let executable = step.executable.as_ref().expect("executable extracted");
    assert_eq!(executable.field, ExecutableField::Prompt);
    assert_eq!(executable.options.get("params"), Some(&json!({"topic": "x"})));
    // Arbitrary state stays in the generated state; task keys never leak into it.
    assert_eq!(step.state.extra.get("color"), Some(&json!("blue")));
    assert!(!step.state.extra.contains_key("prompt"));
    assert!(!step.state.extra.contains_key("params"));
}

// -- template rendering (Darkmatter expression engine) --------------------
//
// Phase 4 replaced the bespoke `{{key || default}}` placeholder renderer with
// the real expression engine, so these pin the engine's behavior over item
// fields rather than a private helper's.

fn render_template(template: &str, fields: serde_json::Map<String, serde_json::Value>) -> Value {
    let globals = serde_json::Map::new();
    let lookup = SourceExpressionLookup::new(&globals, Path::new(".")).with_item(&fields);
    render_interpolated(template, &lookup).unwrap()
}

#[test]
fn template_replaces_known_keys() {
    let mut fields = serde_json::Map::new();
    fields.insert("name".into(), json!("Foo"));
    fields.insert("site".into(), json!("https://example.com"));
    assert_eq!(
        render_template("{{name}} at {{site}}", fields),
        json!("Foo at https://example.com")
    );
}

#[test]
fn template_uses_fallback_for_missing_key() {
    assert_eq!(
        render_template("repo: {{repo || 'n/a'}}", serde_json::Map::new()),
        json!("repo: n/a")
    );
}

#[test]
fn template_uses_fallback_for_null_value() {
    let mut fields = serde_json::Map::new();
    fields.insert("repo".into(), Value::Null);
    assert_eq!(
        render_template("repo: {{ repo || 'none' }}", fields),
        json!("repo: none")
    );
}

/// A template that is exactly one span keeps its typed value — the bespoke
/// renderer it replaced could only ever produce a string.
#[test]
fn whole_span_template_preserves_type() {
    let mut fields = serde_json::Map::new();
    fields.insert("rank".into(), json!(5));
    assert_eq!(render_template("{{ rank }}", fields), json!(5));
}

/// Item fields shadow the invoking document's frontmatter, so an item's
/// `color` wins over a same-named global.
#[test]
fn item_fields_shadow_document_globals() {
    let mut globals = serde_json::Map::new();
    globals.insert("color".into(), json!("global-blue"));
    let mut item = serde_json::Map::new();
    item.insert("color".into(), json!("item-red"));

    let lookup = SourceExpressionLookup::new(&globals, Path::new(".")).with_item(&item);
    assert_eq!(
        render_interpolated("{{ color }}", &lookup).unwrap(),
        json!("item-red")
    );

    // Without an item, the global is still visible.
    let bare = SourceExpressionLookup::new(&globals, Path::new("."));
    assert_eq!(
        render_interpolated("{{ color }}", &bare).unwrap(),
        json!("global-blue")
    );
}

// -- Sequence Plus: characterization + clean-break guardrails --------------
//
// Phase 1 of the Sequence Plus refactor freezes the retained pre-refactor
// contract and pins the deliberate removals. The clean-break/blocked tests
// below encode the *target* behavior and are `#[ignore]`d until the phase that
// implements each removal; un-ignoring them (and updating the paired
// characterization test) is the checkpoint for that phase. They are kept green-
// suite-neutral on purpose: the harness gates each phase on a passing `just
// test`, so a permanently-red test would block every subsequent phase.

mod clean_break {
    use super::*;

    fn two_step_plan() -> SequencePlan {
        scalar_plan(&["a", "b"])
    }

    /// Characterization guardrail (post-Sequence-Plus, Phase 3). The per-step
    /// overlay now emits exactly these five root keys; the retired
    /// `previous_state`/`next_state`/`step`/`total_steps`/`is_first`/`is_last`
    /// names moved inside each `step_state` as `index`/`count`/`is_first`/
    /// `is_last`, and `previous`/`next` carry full `step_state` objects.
    #[test]
    fn characterize_current_overlay_keys() {
        let plan = two_step_plan();
        let overrides = build_step_overlay(&plan, 0).as_set_overrides(None);
        let map = overrides.as_object().expect("overlay is a JSON object");

        for key in ["state", "previous", "next", "sequence_id", "outputs"] {
            assert!(map.contains_key(key), "overlay must emit `{key}`");
        }
        assert_eq!(
            map.len(),
            5,
            "overlay emits exactly five reserved root keys: {map:?}"
        );
        // The generated position fields live inside each state, not at the root.
        let state = map["state"].as_object().unwrap();
        for inner in ["index", "count", "is_first", "is_last", "id", "sequence_id"] {
            assert!(state.contains_key(inner), "state must carry `{inner}`");
        }
    }

    /// Clean break (RATIFIED 2026-07-12): the legacy overlay names
    /// `previous_state`, `next_state`, `step`, and `total_steps` are retired.
    /// `previous`/`next` carry full `step_state` objects and `index`/`count`
    /// move inside each state; no deprecation aliases are provided.
    #[test]
    fn legacy_overlay_names_removed() {
        let plan = two_step_plan();
        let overrides = build_step_overlay(&plan, 0).as_set_overrides(None);
        let map = overrides.as_object().expect("overlay is a JSON object");

        for retired in ["previous_state", "next_state", "step", "total_steps"] {
            assert!(
                !map.contains_key(retired),
                "retired overlay key `{retired}` must not be emitted",
            );
        }
        // The retired root booleans are also gone (they live inside `state`).
        assert!(!map.contains_key("is_first"));
        assert!(!map.contains_key("is_last"));
    }

    /// Clean break (RATIFIED 2026-07-12): the external-only `kind: sequence` +
    /// `list:` document form is retired. Every sequence YAML uses `sequence:`,
    /// whether invoked directly or referenced. A file carrying only `list:`
    /// must be rejected rather than silently accepted.
    #[test]
    fn external_list_shape_rejected() {
        let dir = TempDir::new().unwrap();
        let yaml_path = dir.path().join("legacy.yaml");
        fs::write(&yaml_path, "kind: sequence\nlist:\n  - name: one\n").unwrap();

        let source = make_source(&dir, &[("sequence", json!("legacy.yaml"))], "Prompt");
        let error = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceExternalWrongType(ref msg) if msg.contains("`sequence:`")
            ),
            "the rejection must name the replacement property, got: {error}",
        );
    }

    /// Group execution without `loop` is in scope, but group-loop commit
    /// semantics remain unratified (spec Open Questions), so a group carrying
    /// `loop` must be rejected with a typed, actionable error rather than
    /// executed. Groups are parsed by the Phase 5 preflight walk, which is
    /// where the rejection lands — plan resolution stays shape-agnostic about
    /// a step's executable value.
    #[test]
    fn group_loop_rejected() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[(
                "sequence",
                json!([
                    {
                        "name": "one",
                        "group": {
                            "loop": {"while": "true"},
                            "tasks": [{"prompt": "x"}]
                        }
                    }
                ]),
            )],
            "Prompt",
        );
        let plan = resolve_sequence_plan(&source)
            .expect("the plan itself is well-formed")
            .expect("the fixture declares a sequence");
        let error = super::preflight::build_preflight_graph(&plan, &source)
            .expect_err("a group carrying `loop` must be rejected");
        assert!(
            matches!(
                &error,
                CompositionError::SequenceUnsupportedConstruct { construct, .. }
                    if construct.contains("group `loop`")
            ),
            "the rejection must be typed and name the construct, got: {error}",
        );
    }
}

// -- Phase 4: source grammar ----------------------------------------------
//
// The suffix parser is span-aware: an `->` or `::` inside a quoted operator
// argument or a `{{ }}` interpolation segment is literal text belonging to the
// file reference, not a separator.

mod grammar_tests {
    use super::*;

    fn reference(raw: &str) -> super::super::grammar::SequenceReference {
        match classify_source(&json!(raw)).unwrap() {
            SequenceSourceSpec::Reference(reference) => reference,
            other => panic!("expected a reference for `{raw}`, got {other:?}"),
        }
    }

    #[test]
    fn plain_reference_has_no_suffix() {
        let parsed = reference("steps.yaml");
        assert_eq!(parsed.reference, "steps.yaml");
        assert_eq!(parsed.offset, None);
        assert_eq!(parsed.operator, None);
    }

    #[test]
    fn offset_is_parsed_and_reference_is_left_untouched() {
        let parsed = reference("things.yaml -> colors.data");
        assert_eq!(parsed.reference, "things.yaml");
        assert_eq!(parsed.offset.as_deref(), Some("colors.data"));
    }

    #[test]
    fn every_operator_form_parses() {
        assert_eq!(
            reference("t.yaml -> d::map(color, name)").operator,
            Some(SourceOperator::Map {
                from: "color".into(),
                to: "name".into()
            })
        );
        assert_eq!(
            reference("t.yaml::name(color)").operator,
            Some(SourceOperator::Name {
                from: "color".into()
            })
        );
        assert_eq!(
            reference("t.yaml::template(color + '-is-great')").operator,
            Some(SourceOperator::Template {
                expr: "color + '-is-great'".into()
            })
        );
    }

    /// Quoted arguments keep their delimiters: a `,` inside quotes is part of
    /// the argument, not an argument boundary.
    #[test]
    fn quoted_operator_arguments_survive() {
        assert_eq!(
            reference("t.yaml::map('a,b', name)").operator,
            Some(SourceOperator::Map {
                from: "a,b".into(),
                to: "name".into()
            })
        );
    }

    /// A `template` expression is one argument even when it contains commas —
    /// splitting it on commas would corrupt any nested function call.
    #[test]
    fn template_expression_keeps_interior_commas() {
        assert_eq!(
            reference("t.yaml::template(join(first, last))").operator,
            Some(SourceOperator::Template {
                expr: "join(first, last)".into()
            })
        );
    }

    /// The reference families that would break a naive splitter: `@` magic,
    /// `!` package, `~`, `vault:`, spaces, and `{{ }}` interpolation.
    #[test]
    fn reference_families_are_preserved_verbatim() {
        for raw in [
            "@prompts/steps.yaml",
            "!lib/steps.yaml",
            "~/steps.yaml",
            "vault:notes/steps.yaml",
            "./my folder/steps.yaml",
            "steps@v2.yaml",
        ] {
            assert_eq!(reference(raw).reference, raw, "`{raw}` must survive intact");
        }
    }

    /// An interpolation segment may contain the suffix delimiters; they belong
    /// to the reference, not to the grammar.
    #[test]
    fn interpolation_segments_are_not_split() {
        let parsed = reference("{{ env.ROOT }}/steps.yaml -> data");
        assert_eq!(parsed.reference, "{{ env.ROOT }}/steps.yaml");
        assert_eq!(parsed.offset.as_deref(), Some("data"));
    }

    #[test]
    fn whole_value_spans_classify_as_dynamic_sources() {
        assert!(matches!(
            classify_source(&json!("{{ ctx.dirty_files }}")).unwrap(),
            SequenceSourceSpec::Expression(ref e) if e == "ctx.dirty_files"
        ));
        assert!(matches!(
            classify_source(&json!("$(ls -la)")).unwrap(),
            SequenceSourceSpec::Shell(ref c) if c == "ls -la"
        ));
    }

    /// Two adjacent spans are not a *whole-value* span, so the value stays a
    /// file reference rather than becoming an expression source.
    #[test]
    fn adjacent_spans_are_not_a_whole_value_expression() {
        assert!(matches!(
            classify_source(&json!("{{a}}{{b}}.yaml")).unwrap(),
            SequenceSourceSpec::Reference(_)
        ));
    }

    #[test]
    fn typed_arrays_classify_as_inline() {
        assert!(matches!(
            classify_source(&json!(["a", "b"])).unwrap(),
            SequenceSourceSpec::Inline(ref items) if items.len() == 2
        ));
    }

    #[test]
    fn non_list_non_string_values_are_rejected() {
        let error = classify_source(&json!(42)).unwrap_err();
        assert!(matches!(error, CompositionError::SequenceInvalid(_)), "got: {error}");
    }

    // -- negative suffix syntax --------------------------------------------

    #[test]
    fn more_than_one_operator_is_rejected() {
        let error = classify_source(&json!("t.yaml::name(a)::name(b)")).unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceSourceSyntax { ref problem, .. }
                    if problem.contains("only one operator")
            ),
            "got: {error}"
        );
    }

    #[test]
    fn trailing_text_after_an_operator_is_rejected() {
        let error = classify_source(&json!("t.yaml::name(a) leftover")).unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceSourceSyntax { ref problem, .. }
                    if problem.contains("trailing text")
            ),
            "got: {error}"
        );
    }

    #[test]
    fn an_empty_offset_is_rejected() {
        let error = classify_source(&json!("t.yaml ->")).unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceSourceSyntax { .. }),
            "got: {error}"
        );
    }

    #[test]
    fn an_unclosed_operator_is_rejected() {
        let error = classify_source(&json!("t.yaml::name(a")).unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceSourceSyntax { ref problem, .. }
                    if problem.contains("closing")
            ),
            "got: {error}"
        );
    }

    #[test]
    fn an_unknown_operator_is_rejected() {
        let error = classify_source(&json!("t.yaml::shuffle(a)")).unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceUnknownOperator(ref v) if v == "shuffle"),
            "got: {error}"
        );
    }

    #[test]
    fn operator_arity_is_enforced() {
        for (raw, operator, expected, found) in [
            ("t.yaml::map(a)", "map", 2usize, 1usize),
            ("t.yaml::map(a, b, c)", "map", 2, 3),
            ("t.yaml::name(a, b)", "name", 1, 2),
        ] {
            let error = classify_source(&json!(raw)).unwrap_err();
            assert!(
                matches!(
                    error,
                    CompositionError::SequenceOperatorArity {
                        operator: ref o, expected: e, found: f
                    } if o == operator && e == expected && f == found
                ),
                "`{raw}` got: {error}"
            );
        }
    }
}

// -- Phase 4: format coverage, offsets, operators, strictness -------------

mod source_resolution {
    use super::*;

    /// Resolve a plan from a data file written into a temp dir, referenced by
    /// the given `sequence:` source string.
    fn plan_from(file: &str, contents: &str, sequence: &str) -> Result<SequencePlan, CompositionError> {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(file), contents).unwrap();
        let source = make_source(&dir, &[("sequence", json!(sequence))], "Prompt");
        resolve_sequence_plan(&source).map(|plan| plan.expect("sequence key present"))
    }

    fn names(plan: &SequencePlan) -> Vec<String> {
        plan.steps.iter().map(|s| s.name.clone()).collect()
    }

    // -- every supported format reaches the same normalized plan -----------

    #[test]
    fn yaml_json_and_json5_offsets_produce_equivalent_plans() {
        let yaml = plan_from(
            "d.yaml",
            "description: x\ndata:\n  - blue\n  - green\n",
            "d.yaml -> data",
        )
        .unwrap();
        let json = plan_from(
            "d.json",
            r#"{"description": "x", "data": ["blue", "green"]}"#,
            "d.json -> data",
        )
        .unwrap();
        let json5 = plan_from(
            "d.json5",
            "{ description: 'x', data: ['blue', 'green'] } ",
            "d.json5 -> data",
        )
        .unwrap();

        for plan in [&yaml, &json, &json5] {
            assert_eq!(names(plan), vec!["blue", "green"]);
            // Reached through an offset, so this is foreign data.
            assert!(matches!(plan.source, SequenceSource::DataFile { .. }));
        }
    }

    #[test]
    fn jsonl_and_ndjson_load_from_their_root() {
        for (file, sequence) in [("d.jsonl", "d.jsonl"), ("d.ndjson", "d.ndjson")] {
            let plan = plan_from(
                file,
                "{\"name\": \"one\"}\n\n{\"name\": \"two\"}\n",
                sequence,
            )
            .unwrap();
            assert_eq!(names(&plan), vec!["one", "two"], "for {file}");
        }
    }

    /// JSONL/NDJSON roots are always the list, so an offset has nothing to
    /// select and must be a typed error rather than a lookup miss.
    #[test]
    fn offsets_are_rejected_for_line_delimited_files() {
        for (file, sequence, label) in [
            ("d.jsonl", "d.jsonl -> data", "JSONL"),
            ("d.ndjson", "d.ndjson -> data", "NDJSON"),
        ] {
            let error = plan_from(file, "{\"name\": \"one\"}\n", sequence).unwrap_err();
            assert!(
                matches!(
                    error,
                    CompositionError::SequenceOffsetUnsupported { ref format, .. }
                        if format == label
                ),
                "for {file} got: {error}"
            );
        }
    }

    #[test]
    fn malformed_jsonl_names_the_offending_line() {
        let error = plan_from("d.jsonl", "{\"name\": \"ok\"}\nnot json\n", "d.jsonl").unwrap_err();
        assert!(
            error.to_string().contains("line 2"),
            "the parse failure must name the line: {error}"
        );
    }

    // -- offsets ----------------------------------------------------------

    #[test]
    fn deep_dot_paths_traverse_nested_structures() {
        let plan = plan_from(
            "t.yaml",
            "colors:\n  data:\n    - color: blue\n    - color: green\n",
            "t.yaml -> colors.data",
        )
        .unwrap();
        // Nameless foreign objects take their one-based ordinal.
        assert_eq!(names(&plan), vec!["1", "2"]);
    }

    #[test]
    fn a_missing_offset_path_reports_where_it_failed() {
        let error = plan_from("t.yaml", "colors:\n  data: [1]\n", "t.yaml -> colors.missing")
            .unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceOffsetMissing { ref path, ref failed_at, .. }
                    if path == "colors.missing" && failed_at == "colors.missing"
            ),
            "got: {error}"
        );
    }

    #[test]
    fn an_offset_to_a_non_list_reports_the_observed_type() {
        let error =
            plan_from("t.yaml", "colors:\n  data: hello\n", "t.yaml -> colors.data").unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceOffsetNotAList { ref path, ref found }
                    if path == "colors.data" && found == "string"
            ),
            "got: {error}"
        );
    }

    // -- operators --------------------------------------------------------

    const NESTED: &str = "colors:\n  data:\n    - color: blue\n    - color: green\n";

    #[test]
    fn map_renames_the_key_and_removes_the_original() {
        let plan = plan_from("t.yaml", NESTED, "t.yaml -> colors.data::map(color, name)").unwrap();
        assert_eq!(names(&plan), vec!["blue", "green"]);
        assert!(
            !plan.steps[0].state.extra.contains_key("color"),
            "map removes the source key: {:?}",
            plan.steps[0].state.extra
        );
    }

    #[test]
    fn name_copies_the_value_and_retains_the_original() {
        let plan = plan_from("t.yaml", NESTED, "t.yaml -> colors.data::name(color)").unwrap();
        assert_eq!(names(&plan), vec!["blue", "green"]);
        assert_eq!(
            plan.steps[0].state.extra.get("color"),
            Some(&json!("blue")),
            "name retains the source key"
        );
    }

    #[test]
    fn template_computes_a_name_from_item_fields() {
        let plan = plan_from(
            "t.yaml",
            NESTED,
            "t.yaml -> colors.data::template(color + '-is-great')",
        )
        .unwrap();
        assert_eq!(names(&plan), vec!["blue-is-great", "green-is-great"]);
    }

    #[test]
    fn operator_failures_name_the_item_index() {
        let missing = plan_from(
            "t.yaml",
            "data:\n  - color: blue\n  - shade: green\n",
            "t.yaml -> data::name(color)",
        )
        .unwrap_err();
        assert!(
            matches!(
                missing,
                CompositionError::SequenceOperatorMissingField { index: 1, ref field, .. }
                    if field == "color"
            ),
            "got: {missing}"
        );

        let scalar = plan_from(
            "t.yaml",
            "data:\n  - color: blue\n  - just-a-string\n",
            "t.yaml -> data::name(color)",
        )
        .unwrap_err();
        assert!(
            matches!(
                scalar,
                CompositionError::SequenceOperatorItemNotObject { index: 1, .. }
            ),
            "got: {scalar}"
        );
    }

    /// A `template` that resolves to nothing is an error: a name must be a
    /// non-empty string, and a silently-blank name would produce an unusable id.
    #[test]
    fn an_empty_template_name_is_rejected() {
        let error = plan_from(
            "t.yaml",
            "data:\n  - color: ''\n",
            "t.yaml -> data::template(color)",
        )
        .unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceOperatorEmptyName { index: 0, .. }),
            "got: {error}"
        );
    }

    // -- strictness split by provenance -----------------------------------

    /// Foreign data coerces number and boolean scalars into string names; the
    /// same list authored inline stays strict and is rejected.
    #[test]
    fn foreign_scalars_coerce_but_inline_scalars_do_not() {
        let lenient = plan_from("d.json", "[1, 2, true]", "d.json").unwrap();
        assert_eq!(names(&lenient), vec!["1", "2", "true"]);

        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("sequence", json!([1, 2]))], "Prompt");
        let error = resolve_sequence_plan(&source).unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceInvalid(_)),
            "an authored numeric step is a typo, got: {error}"
        );
    }

    /// A formal `sequence:` document is authored *for* sequences, so it keeps
    /// the strict contract even though it arrives from a file.
    #[test]
    fn formal_documents_stay_strict() {
        let error = plan_from("s.yaml", "sequence:\n  - color: red\n", "s.yaml").unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceStepNameMissing { index: 0 }),
            "got: {error}"
        );
    }

    /// Reading the same formal document through an offset makes it data, and
    /// the missing `name` becomes an ordinal instead of an error.
    #[test]
    fn an_offset_turns_a_formal_document_into_data() {
        let plan = plan_from(
            "s.yaml",
            "sequence:\n  - color: red\n  - color: blue\n",
            "s.yaml -> sequence",
        )
        .unwrap();
        assert_eq!(names(&plan), vec!["1", "2"]);
        assert!(matches!(plan.source, SequenceSource::DataFile { .. }));
    }

    /// `null` is a typed error under *both* contracts: leniency coerces a
    /// value that is something, it does not invent one.
    #[test]
    fn null_items_are_rejected_even_leniently() {
        let error = plan_from("d.json", "[\"ok\", null]", "d.json").unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceNullItem { index: 1 }),
            "got: {error}"
        );
    }

    /// Generated names still dasherize into ids exactly as authored names do.
    #[test]
    fn generated_ordinal_names_still_produce_ids() {
        let plan = plan_from("d.json", "[{\"a\": 1}, {\"a\": 2}]", "d.json").unwrap();
        assert_eq!(
            plan.steps.iter().map(|s| s.state.id.clone()).collect::<Vec<_>>(),
            vec!["1", "2"]
        );
    }

    // -- the retired `list:` shape ----------------------------------------

    #[test]
    fn a_root_that_is_neither_a_list_nor_a_sequence_is_rejected() {
        let error = plan_from("d.yaml", "description: nothing here\n", "d.yaml").unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceExternalWrongType(_)),
            "got: {error}"
        );
    }
}

// -- Phase 4: dynamic sources, list formats, empty-list behavior ----------

mod dynamic_sources {
    use super::*;

    fn plan_from_frontmatter(
        frontmatter: &[(&str, serde_json::Value)],
    ) -> Result<SequencePlan, CompositionError> {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, frontmatter, "Prompt");
        resolve_sequence_plan(&source).map(|plan| plan.expect("sequence key present"))
    }

    fn names(plan: &SequencePlan) -> Vec<String> {
        plan.steps.iter().map(|s| s.name.clone()).collect()
    }

    /// A whole-value expression yielding a typed array is already a list of
    /// entries — it must bypass `ListFormat` entirely, so an entry containing a
    /// comma stays one entry rather than being re-split as CSV.
    #[test]
    fn typed_expression_arrays_bypass_list_classification() {
        let plan = plan_from_frontmatter(&[
            ("items", json!(["a,b", "c"])),
            ("sequence", json!("{{ items }}")),
        ])
        .unwrap();
        assert_eq!(names(&plan), vec!["a,b", "c"]);
        assert!(matches!(plan.source, SequenceSource::Expression));
    }

    /// An expression yielding a *string* is classified — this is what makes the
    /// CSV-by-default `ctx.*` list variables work without asking the author to
    /// convert them first.
    #[test]
    fn every_list_format_is_accepted_from_a_string_expression() {
        for (label, text) in [
            ("csv", "a, b, c"),
            ("tsv", "a\tb\tc"),
            ("space", "a b c"),
            ("lines", "a\nb\nc"),
            ("crlf lines", "a\r\nb\r\nc"),
            ("markdown unordered", "- a\n- b\n- c"),
            ("markdown ordered", "1. a\n2. b\n3. c"),
        ] {
            let plan = plan_from_frontmatter(&[
                ("raw", json!(text)),
                ("sequence", json!("{{ raw }}")),
            ])
            .unwrap();
            assert_eq!(names(&plan), vec!["a", "b", "c"], "for {label}");
        }
    }

    /// A quoted delimiter inside a CSV entry is literal, not a boundary.
    #[test]
    fn quoted_csv_delimiters_survive_classification() {
        let plan = plan_from_frontmatter(&[
            ("raw", json!("\"a,b\", c")),
            ("sequence", json!("{{ raw }}")),
        ])
        .unwrap();
        assert_eq!(names(&plan), vec!["a,b", "c"]);
    }

    /// A single scalar is a one-item list, not an error.
    #[test]
    fn a_scalar_expression_result_is_a_single_step() {
        let plan =
            plan_from_frontmatter(&[("raw", json!("solo")), ("sequence", json!("{{ raw }}"))])
                .unwrap();
        assert_eq!(names(&plan), vec!["solo"]);
    }

    /// Expression sources are foreign data, so numeric entries coerce.
    #[test]
    fn expression_sources_normalize_leniently() {
        let plan = plan_from_frontmatter(&[
            ("items", json!([1, 2])),
            ("sequence", json!("{{ items }}")),
        ])
        .unwrap();
        assert_eq!(names(&plan), vec!["1", "2"]);
    }

    #[test]
    fn a_failing_expression_is_a_typed_error() {
        let error = plan_from_frontmatter(&[("sequence", json!("{{ nope( }}"))]).unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceExpressionFailed { .. }),
            "got: {error}"
        );
    }

    // -- empty static vs empty dynamic ------------------------------------

    /// The ratified split: a static empty list is an authoring error, while a
    /// dynamic source resolving to nothing is a graceful no-op that yields a
    /// zero-step plan for the caller to report and exit `0` on.
    #[test]
    fn empty_static_errors_but_empty_dynamic_is_a_no_op() {
        let static_error = plan_from_frontmatter(&[("sequence", json!([]))]).unwrap_err();
        assert!(
            matches!(static_error, CompositionError::SequenceEmpty),
            "got: {static_error}"
        );

        for (label, frontmatter) in [
            (
                "empty typed array",
                vec![("items", json!([])), ("sequence", json!("{{ items }}"))],
            ),
            (
                "empty string",
                vec![("raw", json!("")), ("sequence", json!("{{ raw }}"))],
            ),
            (
                "whitespace-only string",
                vec![("raw", json!("  \n ")), ("sequence", json!("{{ raw }}"))],
            ),
            (
                "null result",
                vec![("sequence", json!("{{ missing_key }}"))],
            ),
        ] {
            let plan = plan_from_frontmatter(&frontmatter).unwrap();
            assert!(
                plan.steps.is_empty(),
                "{label} must resolve to zero steps, got {:?}",
                names(&plan)
            );
        }
    }

    /// An empty *data file* is dynamic too — its emptiness is a runtime fact,
    /// not an authoring mistake.
    #[test]
    fn an_empty_data_file_resolves_to_zero_steps() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("d.json"), "[]").unwrap();
        let source = make_source(&dir, &[("sequence", json!("d.json"))], "Prompt");
        let plan = resolve_sequence_plan(&source).unwrap().unwrap();
        assert!(plan.steps.is_empty());
    }

    /// A *formal* sequence document is authored for sequences, so an empty
    /// `sequence:` list there is still the typo it looks like.
    #[test]
    fn an_empty_formal_sequence_document_is_an_error() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("s.yaml"), "sequence: []\n").unwrap();
        let source = make_source(&dir, &[("sequence", json!("s.yaml"))], "Prompt");
        let error = resolve_sequence_plan(&source).unwrap_err();
        assert!(matches!(error, CompositionError::SequenceEmpty), "got: {error}");
    }

    // -- shell sources ----------------------------------------------------

    /// Without an approval-capable runner a `$( … )` source is a typed error,
    /// never a silent unapproved execution.
    #[test]
    fn shell_sources_require_a_runner() {
        let error = plan_from_frontmatter(&[("sequence", json!("$(ls)"))]).unwrap_err();
        assert!(
            matches!(error, CompositionError::SequenceShellFailed { ref command, .. } if command == "ls"),
            "got: {error}"
        );
    }

    /// With a runner, the command's stdout is classified like any other
    /// textual list and normalized leniently.
    #[test]
    fn shell_output_is_classified_as_a_list() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("sequence", json!("$(echo one two)"))], "Prompt");

        let runner = |command: &str| {
            assert_eq!(command, "echo one two");
            Ok("one\ntwo\n".to_string())
        };
        let plan = resolve_sequence_plan_with(
            &source,
            SequenceSourceOptions {
                shell_runner: Some(&runner),
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(names(&plan), vec!["one", "two"]);
        assert!(matches!(plan.source, SequenceSource::Shell));
    }

    /// A runner failure propagates as the typed shell error rather than being
    /// swallowed into an empty sequence.
    #[test]
    fn a_failing_shell_runner_propagates() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("sequence", json!("$(false)"))], "Prompt");
        let runner = |command: &str| {
            Err(CompositionError::SequenceShellFailed {
                command: command.to_string(),
                source: crate::composition::error::SequenceShellCause::Exited {
                    status: "exit status: 1".to_string(),
                    stderr: "boom".to_string(),
                },
            })
        };
        let error = resolve_sequence_plan_with(
            &source,
            SequenceSourceOptions {
                shell_runner: Some(&runner),
            },
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                CompositionError::SequenceShellFailed {
                    source: crate::composition::error::SequenceShellCause::Exited { .. },
                    ..
                }
            ),
            "got: {error}"
        );
    }
}

// -- Phase 4: formal `template` + `$schema` on step state -----------------

mod formal_documents {
    use super::*;

    fn plan_from(contents: &str) -> Result<SequencePlan, CompositionError> {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("s.yaml"), contents).unwrap();
        let source = make_source(&dir, &[("sequence", json!("s.yaml"))], "Prompt");
        resolve_sequence_plan(&source).map(|plan| plan.expect("sequence key present"))
    }

    /// Template values land before generated fields, so a templated key is
    /// ordinary authored state by the time ids and positions are made — and it
    /// can be validated by `$schema` alongside the authored keys.
    #[test]
    fn templates_apply_before_generated_fields_and_satisfy_the_schema() {
        let plan = plan_from(
            r#"kind: sequence
sequence:
  - name: blue
    color: blue
    rank: 5
  - name: red
    color: red
    rank: 3
template:
  desc: "{{ color }}({{ rank }})"
$schema:
  color: string(required)
  rank: number(required)
  desc: string(required)
"#,
        )
        .unwrap();

        assert_eq!(plan.steps[0].state.extra["desc"], json!("blue(5)"));
        assert_eq!(plan.steps[1].state.extra["desc"], json!("red(3)"));
        // Generated fields are present and were not displaced by the template.
        assert_eq!(plan.steps[0].state.index, 1);
        assert_eq!(plan.steps[0].state.count, 2);
        assert_eq!(plan.steps[0].state.id, "blue");
    }

    /// An item that already defines a templated key keeps its own value.
    #[test]
    fn an_item_value_wins_over_the_template() {
        let plan = plan_from(
            r#"sequence:
  - name: one
    desc: authored
  - name: two
template:
  desc: "generated-{{ name }}"
"#,
        )
        .unwrap();
        assert_eq!(plan.steps[0].state.extra["desc"], json!("authored"));
        assert_eq!(plan.steps[1].state.extra["desc"], json!("generated-two"));
    }

    /// A schema violation names the step index, its generated id, and the
    /// failing property.
    #[test]
    fn a_schema_violation_reports_the_step_and_property() {
        let error = plan_from(
            r#"kind: sequence
sequence:
  - name: blue
    color: blue
  - name: red
$schema:
  color: string(required)
"#,
        )
        .unwrap_err();

        assert!(
            matches!(
                error,
                CompositionError::SequenceStateSchemaViolation {
                    index: 1, ref id, ref property, ..
                } if id == "red" && property == "color"
            ),
            "got: {error}"
        );
    }

    /// The schema judges the *state* portion only: executable and task keys
    /// are not state and must not be validated as such.
    #[test]
    fn executable_keys_are_excluded_from_state_validation() {
        let plan = plan_from(
            r#"kind: sequence
sequence:
  - name: one
    color: blue
    shell: just test
$schema:
  color: string(required)
"#,
        )
        .unwrap();

        assert!(plan.steps[0].executable.is_some());
        assert!(!plan.steps[0].state.extra.contains_key("shell"));
    }
}
