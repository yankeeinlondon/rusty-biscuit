//! Contract tests for the prompt documents shipped with Claudine.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{ExpressionFinder, parse_condition};
use darkmatter::markdown::schemas::DarkmatterSchemas;
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

#[test]
fn shipped_markdown_generators_use_the_explicit_raw_markdown_boundary() {
    let mut markdown_generators = Vec::new();
    let root = repository_root();
    for prompt in shipped_prompt_paths() {
        let markdown = Markdown::try_from(prompt.as_path()).unwrap();
        for expression in ExpressionFinder::new(markdown.content()).find_all() {
            if expression.expression.contains("as_unordered_list(") {
                markdown_generators.push((
                    prompt.strip_prefix(&root).unwrap().to_path_buf(),
                    expression.expression,
                ));
            }
        }
    }

    assert_eq!(
        markdown_generators,
        [
            (
                PathBuf::from("prompts/code-comment-quality.md"),
                "raw_markdown(as_unordered_list(ctx.current_packages))".to_string(),
            ),
            (
                PathBuf::from("prompts/context.md"),
                "raw_markdown(as_unordered_list(ctx.package_areas))".to_string(),
            ),
            (
                PathBuf::from("prompts/faster-builds-and-tests.md"),
                "raw_markdown(as_unordered_list(ctx.current_packages))".to_string(),
            ),
        ],
        "the shipped prompt corpus must opt Markdown-producing expressions into raw syntax"
    );
}

#[cfg(unix)]
#[test]
fn shipped_context_prompt_renders_its_package_area_list_through_the_cli() {
    use std::fs;

    use common::wrap::seed_minimal_config;
    use common::write_executable;

    let repo = repository_root();
    let workspace = tempfile::tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    let captured_prompt = workspace.path().join("stdin.txt");
    fs::create_dir_all(&bin_dir).unwrap();
    seed_minimal_config(workspace.path());
    write_executable(
        &bin_dir.join("codex"),
        "#!/bin/sh\n/bin/cat > \"$CLAUDINE_STDIN_FILE\"\nexit 0\n",
    );

    assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &bin_dir)
        .env("CLAUDINE_STDIN_FILE", &captured_prompt)
        .current_dir(&repo)
        .arg("compose")
        .arg(repo.join("prompts/context.md"))
        .args(["-y", "--codex"])
        .assert()
        .success();

    let captured = fs::read_to_string(captured_prompt).unwrap();
    assert!(captured.contains("this repo is a monorepo, with the following package areas:"));
    assert!(
        captured.contains("- claudine"),
        "the shipped context prompt did not render the Claudine package-area list: {captured}"
    );
    assert!(!captured.contains("raw_markdown(as_unordered_list"));
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
