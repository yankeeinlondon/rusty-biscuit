use darkmatter::markdown::Markdown;
use serde_json::Value;
use std::path::{Path, PathBuf};

mod common;
#[cfg(unix)]
use common::{CliProcessFixture, write, write_executable};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine/cli parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
    {
        let path = entry.expect("prompt directory entry").path();
        if path.is_dir() {
            collect_markdown_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

fn frontmatter_value(markdown: &Markdown) -> Value {
    Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

fn inspect_mapping_only_set(
    value: &Value,
    path: &str,
    witnesses: &mut usize,
    defects: &mut Vec<String>,
) {
    match value {
        Value::Object(map) => {
            if let Some(payload) = map.get("set") {
                *witnesses += 1;
                if !payload.is_object() {
                    defects.push(format!(
                        "{path}.set uses {}, not the required mapping payload",
                        match payload {
                            Value::Null => "null",
                            Value::Bool(_) => "a boolean",
                            Value::Number(_) => "a number",
                            Value::String(_) => "a string",
                            Value::Array(_) => "an array",
                            Value::Object(_) => unreachable!(),
                        }
                    ));
                }
            }
            if map.get("action") == Some(&Value::String("set".to_string())) {
                defects.push(format!(
                    "{path}.action uses the removed explicit `action: set` form"
                ));
            }
            for (key, child) in map {
                inspect_mapping_only_set(
                    child,
                    &format!("{path}.{key}"),
                    witnesses,
                    defects,
                );
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                inspect_mapping_only_set(
                    child,
                    &format!("{path}[{index}]"),
                    witnesses,
                    defects,
                );
            }
        }
        _ => {}
    }
}

fn collect_artifact_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
    {
        let path = entry.expect("artifact directory entry").path();
        if path.is_dir() {
            collect_artifact_files(&path, files);
        } else if path.extension().is_some_and(|extension| {
            matches!(extension.to_str(), Some("md" | "yaml" | "yml" | "json"))
        }) {
            files.push(path);
        }
    }
}

#[test]
fn shipped_lifecycle_artifacts_use_mapping_only_set() {
    use claudine::composition::lifecycle::parse_lifecycle_config;

    let root = workspace_root();
    let roots = [
        root.join("prompts"),
        root.join("claudine/cli/tests/fixtures"),
        root.join("claudine/gen/tests/fixtures"),
        root.join("claudine/schemas"),
        root.join("claudine/docs/schemas"),
        root.join("darkmatter/dmls/tests/fixtures/sequence_descent"),
    ];
    let mut files = Vec::new();
    for artifact_root in &roots {
        collect_artifact_files(artifact_root, &mut files);
    }
    files.sort();
    assert!(!files.is_empty(), "the shipped artifact corpus must not be empty");

    let mut mapping_witnesses = 0;
    let mut defects = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            for line in text.lines() {
                let authored = line.trim_start().trim_start_matches("- ");
                for removed in ["set: [", "\"set\": [", "action: set", "action: \"set\""] {
                    if authored.starts_with(removed) {
                        defects.push(format!(
                            "{} advertises removed lifecycle syntax `{removed}`",
                            path.display()
                        ));
                    }
                }
            }
        }

        match path.extension().and_then(|extension| extension.to_str()) {
            Some("md") => match Markdown::try_from(path.as_path()) {
                Ok(markdown) => {
                    let frontmatter = frontmatter_value(&markdown);
                    let mut shape_defects = Vec::new();
                    let mut document_witnesses = 0;
                    inspect_mapping_only_set(
                        &frontmatter,
                        "$",
                        &mut document_witnesses,
                        &mut shape_defects,
                    );
                    mapping_witnesses += document_witnesses;
                    defects.extend(
                        shape_defects
                            .into_iter()
                            .map(|defect| format!("{}: {defect}", path.display())),
                    );
                    if document_witnesses > 0
                        && let Err(error) = parse_lifecycle_config(&frontmatter, path)
                    {
                        defects.push(format!("{}: {error}", path.display()));
                    }
                }
                Err(error) => defects.push(format!("{}: {error}", path.display())),
            },
            Some("json") => match serde_json::from_str::<Value>(&text) {
                Ok(value) => {
                    inspect_mapping_only_set(
                        &value,
                        &path.display().to_string(),
                        &mut mapping_witnesses,
                        &mut defects,
                    );
                }
                Err(error) => defects.push(format!("{}: {error}", path.display())),
            },
            _ => {}
        }
    }

    assert!(
        mapping_witnesses > 0,
        "the corpus must contain a lifecycle `set` mapping witness"
    );
    assert!(
        defects.is_empty(),
        "shipped artifacts with removed or invalid lifecycle `set` syntax:\n{}",
        defects.join("\n")
    );
}

#[test]
fn shipped_prompt_corpus_parses_frontmatter() {
    let prompts = workspace_root().join("prompts");
    let mut files = Vec::new();
    collect_markdown_files(&prompts, &mut files);
    files.sort();

    assert!(!files.is_empty(), "the shipped prompt corpus must not be empty");
    let implement_plan = std::fs::read_to_string(prompts.join("_implement/implement-plan.md"))
        .expect("shipped implementation prompt");
    assert!(
        implement_plan.contains("epilog: null"),
        "the passive corpus must include the mapping-only lifecycle set artifact"
    );
    assert!(
        !implement_plan.contains("set: ["),
        "the shipped implementation prompt must not retain positional lifecycle set syntax"
    );
    let failures = files
        .iter()
        .filter_map(|path| {
            Markdown::try_from(path.as_path())
                .err()
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "shipped prompts with invalid Markdown/frontmatter:\n{}",
        failures.join("\n")
    );
}

#[cfg(unix)]
#[test]
fn shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch() {
    let fixture = CliProcessFixture::named("shipped-implement-mapping-set");
    fixture.initialize_repository();
    fixture.seed_user_config();

    let root = workspace_root();
    let prompts = fixture.cwd().join("prompts");
    let target = prompts.join("_implement/implement-plan.md");
    let source_prompts = root.join("prompts");
    let mut shipped = Vec::new();
    collect_markdown_files(&source_prompts, &mut shipped);
    for source in shipped {
        let relative = source.strip_prefix(&source_prompts).unwrap();
        let destination = prompts.join(relative);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(source, &destination).unwrap();
    }

    let case = fixture.cwd().join("fixes/case");
    std::fs::create_dir_all(&case).unwrap();
    let spec = case.join("spec.md");
    let plan = case.join("plan.md");
    let log = case.join("implementation-log.md");
    write(&spec, "---\nimplemented: false\n---\n# Spec\n");
    write(&plan, "---\ntotal_phases: 2\nphase: 2\n---\n# Plan\n");
    let original_log = "---\nmessage_to_agent: PRIOR-HANDOFF\n---\n# Implementation Log\n";
    write(&log, original_log);

    let capture = fixture.cwd().join("delivered-prompt.txt");
    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
{
  printf '%s\n' "$@"
  cat
} > "$CLAUDINE_PROMPT_CAPTURE"
exit 17
"#,
    );
    let original_prompt = std::fs::read_to_string(&target).unwrap();
    assert!(
        original_prompt.contains("epilog: \"{{message_to_agent}}\"")
            && original_prompt.contains("message_to_agent: null"),
        "the regression must execute the reported multi-key mapping"
    );

    let output = fixture
        .command()
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .arg("compose")
        .arg(&target)
        .arg(format!("plan={}", plan.display()))
        .arg("phase=2")
        .arg("message_to_agent=PRIOR-HANDOFF")
        .args(["--goose", "-y"])
        .output()
        .unwrap();
    let rendered = common::strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ));

    assert!(!output.status.success(), "the fake provider must fail:\n{rendered}");
    let delivered = std::fs::read_to_string(&capture)
        .unwrap_or_else(|_| panic!("the provider must launch after initialize:\n{rendered}"));
    assert!(delivered.contains("# Implement Phase 2 of 2"), "{delivered}");
    assert!(rendered.contains("PRIOR-HANDOFF"), "{rendered}");
    for rejected in [
        "object value not allowed here",
        "set: {property: value}",
        "unknown root `message_to_agent`",
    ] {
        assert!(
            !rendered.contains(rejected),
            "the shipped mapping failed before provider launch ({rejected}):\n{rendered}"
        );
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), original_prompt);
    assert_eq!(std::fs::read_to_string(&log).unwrap(), original_log);
    assert!(!fixture.audio_spool().exists(), "no audio may be published");
}

#[cfg(unix)]
#[test]
fn shipped_implement_router_runs_real_proxy_handoff() {
    let fixture = CliProcessFixture::named("shipped-prompts-router");
    // The router prompt is resolved from its own repository, so the launch
    // directory is a second, separate repository inside the same workspace.
    let prompt_fixture = fixture.cwd().join("prompt-repo");
    let feature = fixture.cwd().join("features/2026-07-20-router-fixture");
    let review = feature.join("review.md");
    write(&review, "---\nimplemented: false\n---\n# Router fixture\n");

    let capture = fixture.cwd().join("delivered-prompt.txt");
    write_executable(
        &fixture.bin_dir().join("claude"),
        r#"#!/bin/sh
{
  for arg in "$@"; do
    if [ -f "$arg" ]; then cat "$arg"; else printf '%s\n' "$arg"; fi
  done
  cat
} >> "$CLAUDINE_PROMPT_CAPTURE" 2>/dev/null
exit 0
"#,
    );

    let root = workspace_root();
    let prompt_dir = prompt_fixture.join("prompts");
    let target_dir = prompt_dir.join("_implement");
    std::fs::create_dir_all(&target_dir).expect("prompt fixture directory");
    // The *shipped* router is the artifact under test; only its repository is
    // isolated, so its relative references still resolve as authored.
    std::fs::copy(root.join("prompts/implement.md"), prompt_dir.join("implement.md"))
        .expect("copy shipped router prompt");
    write(
        &target_dir.join("implement-review.md"),
        "---\ntitle: Router target fixture\n---\n# Implementation of Review Findings\n",
    );
    assert!(common::init_git_repo(&prompt_fixture));

    let router = prompt_dir.join("implement.md");
    let review_arg = format!("review={}", review.display());
    fixture
        .command_builder()
        // The router resolves its target relative to the repository it is
        // launched from, which is the prompt repository staged above.
        .ambient_context(&prompt_fixture)
        .build()
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .args([
            "compose",
            "--claude",
            router.to_str().expect("UTF-8 router path"),
            &review_arg,
        ])
        .assert()
        .success();

    let delivered = std::fs::read_to_string(&capture).expect("captured provider prompt");
    assert!(
        delivered.contains("Implementation of Review Findings"),
        "the shipped router must deliver the resolved target prompt:\n{delivered}"
    );
    assert!(
        !delivered.contains("This prompt should never be reached"),
        "the router body must not reach the provider after proxy handoff:\n{delivered}"
    );
}
