//! Contract tests for the prompt documents shipped with Claudine.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{ExpressionFinder, parse_condition};
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaPhase};
use ignore::WalkBuilder;
use serde_json::Value;
use std::path::{Path, PathBuf};

mod common;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn shipped_prompt_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = WalkBuilder::new(repository_root().join("prompts"))
        .hidden(false)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    paths.sort();
    paths
}

fn record_expression_errors(
    prompt: &Path,
    surface: &str,
    text: &str,
    skip_code_blocks: bool,
    errors: &mut Vec<String>,
) {
    let expressions = if skip_code_blocks {
        ExpressionFinder::new(text).find_all()
    } else {
        ExpressionFinder::find_all_plain(text)
    };
    for expression in expressions {
        if let Err(error) = parse_condition(&expression.expression) {
            errors.push(format!(
                "{} [{surface}] `{{{{ {} }}}}`: {error}",
                prompt.display(),
                expression.expression
            ));
        }
    }
}

fn inspect_frontmatter_value(
    prompt: &Path,
    property: &str,
    value: &Value,
    errors: &mut Vec<String>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let property = if property.is_empty() {
                    key.clone()
                } else {
                    format!("{property}.{key}")
                };
                inspect_frontmatter_value(prompt, &property, child, errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                inspect_frontmatter_value(
                    prompt,
                    &format!("{property}[{index}]"),
                    child,
                    errors,
                );
            }
        }
        Value::String(text) => {
            record_expression_errors(prompt, property, text, false, errors);
            let leaf = property.rsplit('.').next().unwrap_or(property);
            if matches!(leaf, "when" | "until" | "while")
                && !text.contains("{{")
                && let Err(error) = parse_condition(text)
            {
                errors.push(format!(
                    "{} [{property}] `{text}`: {error}",
                    prompt.display()
                ));
            }
        }
        _ => {}
    }
}

fn inspect_block_conditions(prompt: &Path, body: &str, errors: &mut Vec<String>) {
    for (index, line) in body.lines().enumerate() {
        let line = line.trim();
        if !line.starts_with("::block ") {
            continue;
        }
        let Some(after_when) = line.split_once("when=\"").map(|(_, value)| value) else {
            continue;
        };
        let Some(condition) = after_when.strip_suffix('"') else {
            errors.push(format!(
                "{} [body line {}] malformed block condition: {line}",
                prompt.display(),
                index + 1
            ));
            continue;
        };
        if condition.contains("{{") {
            continue;
        }
        if let Err(error) = parse_condition(condition) {
            errors.push(format!(
                "{} [body line {}] `{condition}`: {error}",
                prompt.display(),
                index + 1
            ));
        }
    }
}

#[test]
fn shipped_prompts_have_parseable_schemas_and_expressions() {
    let paths = shipped_prompt_paths();
    assert!(!paths.is_empty(), "the shipped prompt corpus must not be empty");

    let mut errors = Vec::new();
    for prompt in paths {
        let markdown = match Markdown::try_from(prompt.as_path()) {
            Ok(markdown) => markdown,
            Err(error) => {
                errors.push(format!("{} [markdown]: {error}", prompt.display()));
                continue;
            }
        };
        if let Err(error) = DarkmatterSchemas::new().effective_for(&markdown) {
            errors.push(format!("{} [$schema]: {error}", prompt.display()));
        }

        for (property, value) in markdown.frontmatter().as_map() {
            inspect_frontmatter_value(&prompt, property, value, &mut errors);
        }
        record_expression_errors(&prompt, "body", markdown.content(), true, &mut errors);
        inspect_block_conditions(&prompt, markdown.content(), &mut errors);
    }

    assert!(
        errors.is_empty(),
        "shipped prompt contract failures:\n{}",
        errors.join("\n")
    );
}

/// Every shipped schema must successfully construct both phase projections.
#[test]
fn shipped_prompt_schemas_project_at_both_phases() {
    let mut failures = Vec::new();
    for prompt in shipped_prompt_paths() {
        let Ok(markdown) = Markdown::try_from(prompt.as_path()) else {
            continue;
        };
        let Ok(Some(effective)) = DarkmatterSchemas::new().effective_for(&markdown) else {
            continue;
        };
        let empty = Value::Object(serde_json::Map::new());
        for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
            if let Err(error) = effective.validate_for_phase(&empty, phase) {
                failures.push(format!("{} ({phase:?}): {error}", prompt.display()));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "shipped prompt phase-projection failures:\n{}",
        failures.join("\n")
    );
}

/// End-to-end through the real shipped artifact and the normal invocation
/// path: `implement-plan.md` derives `spec` from a sibling `spec.md` and
/// deliberately yields `null` when there is none. While `spec` was declared
/// `eager` that combination was a launch requirement the document could not
/// satisfy, and the run failed with ``spec — null is not of type "string"``.
///
/// `--dry-run` stops after provider resolution, so the shipped document's
/// `say:`/`effect:`/`shell:` lifecycle effects never run. The completion half
/// of the same document is covered end-to-end by
/// `level2_shipped_implement_plan_*`, whose fixture also stages no sibling
/// spec.
#[cfg(unix)]
#[test]
fn shipped_implement_plan_launches_without_a_sibling_spec() {
    use std::fs;

    use common::wrap::seed_minimal_config;
    use common::{strip_ansi, write_executable};

    let workspace = tempfile::tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    let feature_dir = workspace.path().join("features/no-spec-here");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&feature_dir).unwrap();
    seed_minimal_config(workspace.path());
    write_executable(&bin_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    let plan = feature_dir.join("plan.md");
    fs::write(&plan, "---\ntotal_phases: 3\nphase: 1\n---\n# Plan\n").unwrap();
    assert!(
        !feature_dir.join("spec.md").exists(),
        "the regression needs a plan with no sibling spec"
    );

    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &bin_dir)
        .current_dir(workspace.path())
        .arg("compose")
        .arg(repository_root().join("prompts/_implement/implement-plan.md"))
        .arg(format!("plan={}", plan.display()))
        .args(["--dry-run", "-y", "--codex"])
        .assert()
        .success();

    let output = assert.get_output();
    let rendered = strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    assert!(
        rendered.contains("spec: null"),
        "the run must actually reach the null branch this regression is about: {rendered}"
    );
    for marker in ["is not of type", "schema validation", "MissingProperties"] {
        assert!(
            !rendered.contains(marker),
            "an absent optional spec must not be a launch problem ({marker}): {rendered}"
        );
    }
}

#[cfg(unix)]
#[test]
fn feature_review_cli_preserves_numeric_iteration_and_dependent_paths() {
    use std::fs;

    use common::wrap::seed_minimal_config;
    use common::write_executable;

    let repo = repository_root();
    let workspace = tempfile::tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    let feature_dir = workspace.path().join("features/example");
    let spec = feature_dir.join("spec.md");
    let review = feature_dir.join("review-3.md");
    let captured_prompt = workspace.path().join("stdin.txt");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&feature_dir).unwrap();
    seed_minimal_config(workspace.path());
    fs::write(&spec, "---\nreview_iterations: '2'\n---\n# Example\n").unwrap();
    write_executable(
        &bin_dir.join("codex"),
        "#!/bin/sh\n/bin/cat > \"$CLAUDINE_STDIN_FILE\"\nprintf '%s\\n' '---' 'ready: true' '---' > \"$CLAUDINE_REVIEW_FILE\"\nexit 0\n",
    );

    let prompt = repo.join("prompts/_reviews/feature-review.md");
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &bin_dir)
        .env("CLAUDINE_STDIN_FILE", &captured_prompt)
        .env("CLAUDINE_REVIEW_FILE", &review)
        .current_dir(workspace.path())
        .arg("compose")
        .arg(&prompt)
        .arg(format!("spec={}", spec.display()))
        .args(["-y", "--codex"])
        .assert()
        .success();

    let composed = fs::read_to_string(captured_prompt).unwrap();
    assert!(
        composed.contains("Review Iteration: #3"),
        "quoted review_iterations must produce numeric iteration 3: {composed}"
    );
    assert!(
        composed.contains("review-3.md"),
        "the output review path must incorporate iteration 3: {composed}"
    );
    assert!(
        composed.contains("review-2.md"),
        "the previous-review instructions must target iteration 2: {composed}"
    );
    assert!(
        !composed.contains("decrement_file_index(review)"),
        "the helper call must be evaluated rather than delivered as literal text: {composed}"
    );
}
