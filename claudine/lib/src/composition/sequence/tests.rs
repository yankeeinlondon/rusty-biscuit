use super::*;
use darkmatter::markdown::{Frontmatter, Markdown};
use serde_json::json;
use serial_test::serial;
use std::fs;
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

// -- resolve_sequence_plan: external YAML (kind/list/template form) -------

#[test]
fn external_kind_list_template_loads_and_applies_templates() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("agents.yaml");
    fs::write(
        &yaml_path,
        r#"kind: sequence
template:
  desc: "{{name}} (site: {{site}})"
list:
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
list:
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
list:
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
        "kind: sequence\ntemplate: not-an-object\nlist:\n  - name: One\n",
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("bad.yaml"))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceExternalWrongType(ref msg) if msg.contains("`template`")),
        "got: {err}"
    );
}

#[test]
fn external_template_rejected_in_plain_sequence_form() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("bad.yaml");
    fs::write(
        &yaml_path,
        r#"sequence:
  - name: One
template:
  desc: "{{name}}"
"#,
    )
    .unwrap();

    let source = make_source(&dir, &[("sequence", json!("bad.yaml"))], "Prompt");
    let err = resolve_sequence_plan(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::SequenceExternalWrongType(ref msg) if msg.contains("template")),
        "got: {err}"
    );
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
    fs::write(
        &yaml_path,
        "kind: sequence\ntemplate:\n  count: 42\nlist:\n  - name: One\n",
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

#[test]
fn overlay_for_single_step_sequence() {
    let plan = SequencePlan {
        source: SequenceSource::Inline,
        steps: vec![SequenceStep {
            index: 0,
            name: "only".to_string(),
            raw_state: json!("only"),
        }],
        document_fail_fast: true,
    };
    let overlay = build_step_overlay(&plan, 0);
    assert!(overlay.is_first);
    assert!(overlay.is_last);
    assert_eq!(overlay.step, 1);
    assert_eq!(overlay.total_steps, 1);
    assert!(overlay.previous_state.is_null());
    assert!(overlay.next_state.is_null());
}

#[test]
fn overlay_for_middle_step() {
    let plan = SequencePlan {
        source: SequenceSource::Inline,
        steps: vec![
            SequenceStep {
                index: 0,
                name: "a".into(),
                raw_state: json!("a"),
            },
            SequenceStep {
                index: 1,
                name: "b".into(),
                raw_state: json!("b"),
            },
            SequenceStep {
                index: 2,
                name: "c".into(),
                raw_state: json!("c"),
            },
        ],
        document_fail_fast: true,
    };
    let overlay = build_step_overlay(&plan, 1);
    assert!(!overlay.is_first);
    assert!(!overlay.is_last);
    assert_eq!(overlay.step, 2);
    assert_eq!(overlay.total_steps, 3);
    assert_eq!(overlay.state, json!("b"));
    assert_eq!(overlay.previous_state, json!("a"));
    assert_eq!(overlay.next_state, json!("c"));
}

// -- render_simple_template -----------------------------------------------

#[test]
fn template_replaces_known_keys() {
    let mut fields = serde_json::Map::new();
    fields.insert("name".into(), json!("Foo"));
    fields.insert("site".into(), json!("https://example.com"));
    let result = render_simple_template("{{name}} at {{site}}", &fields);
    assert_eq!(result, "Foo at https://example.com");
}

#[test]
fn template_uses_fallback_for_missing_key() {
    let fields = serde_json::Map::new();
    let result = render_simple_template("repo: {{repo || 'n/a'}}", &fields);
    assert_eq!(result, "repo: n/a");
}

#[test]
fn template_uses_fallback_for_null_value() {
    let mut fields = serde_json::Map::new();
    fields.insert("repo".into(), serde_json::Value::Null);
    let result = render_simple_template("repo: {{ repo || 'none' }}", &fields);
    assert_eq!(result, "repo: none");
}
