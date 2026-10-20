//! Shipped research and documentation sequences retain lexical row bindings.

use crate::common::{CliProcessFixture, strip_ansi};

#[test]
fn shipped_research_sequence_corpus_uses_lexical_row_bindings() {
    let artifacts = [
        ("claudine/docs/research/acp/_fleet.md", include_str!("../../../../claudine/docs/research/acp/_fleet.md")),
        ("claudine/docs/research/agent-cli/_fleet.md", include_str!("../../../../claudine/docs/research/agent-cli/_fleet.md")),
        ("claudine/docs/research/agent-errors/_fleet.md", include_str!("../../../../claudine/docs/research/agent-errors/_fleet.md")),
        ("claudine/docs/research/agent-logging/_fleet.md", include_str!("../../../../claudine/docs/research/agent-logging/_fleet.md")),
        ("claudine/docs/research/agent-models/_fleet.md", include_str!("../../../../claudine/docs/research/agent-models/_fleet.md")),
        ("claudine/docs/research/agent-permissions/_fleet.md", include_str!("../../../../claudine/docs/research/agent-permissions/_fleet.md")),
        ("claudine/docs/research/hooks/_fleet.md", include_str!("../../../../claudine/docs/research/hooks/_fleet.md")),
        ("claudine/docs/research/local_runners/_fleet.md", include_str!("../../../../claudine/docs/research/local_runners/_fleet.md")),
        ("claudine/docs/research/mcp/_fleet.md", include_str!("../../../../claudine/docs/research/mcp/_fleet.md")),
        ("claudine/docs/research/memory/_fleet.md", include_str!("../../../../claudine/docs/research/memory/_fleet.md")),
        ("claudine/docs/research/model-config/_fleet.md", include_str!("../../../../claudine/docs/research/model-config/_fleet.md")),
        ("claudine/docs/research/non-interactive-sessions/_fleet.md", include_str!("../../../../claudine/docs/research/non-interactive-sessions/_fleet.md")),
        ("claudine/docs/research/plugins/_fleet.md", include_str!("../../../../claudine/docs/research/plugins/_fleet.md")),
        ("claudine/docs/research/reasoning-level/_fleet.md", include_str!("../../../../claudine/docs/research/reasoning-level/_fleet.md")),
        ("claudine/docs/research/resume/_fleet.md", include_str!("../../../../claudine/docs/research/resume/_fleet.md")),
        ("claudine/docs/research/signals/_fleet.md", include_str!("../../../../claudine/docs/research/signals/_fleet.md")),
        ("claudine/docs/research/skills/_fleet.md", include_str!("../../../../claudine/docs/research/skills/_fleet.md")),
        ("claudine/docs/research/slash-commands/_fleet.md", include_str!("../../../../claudine/docs/research/slash-commands/_fleet.md")),
        ("claudine/docs/research/steering/_fleet.md", include_str!("../../../../claudine/docs/research/steering/_fleet.md")),
        ("claudine/docs/research/subagents/_fleet.md", include_str!("../../../../claudine/docs/research/subagents/_fleet.md")),
        ("claudine/docs/research/system-prompt/_fleet.md", include_str!("../../../../claudine/docs/research/system-prompt/_fleet.md")),
        ("claudine/docs/research/usage/_fleet.md", include_str!("../../../../claudine/docs/research/usage/_fleet.md")),
        ("claudine/docs/research/acp/_research.md", include_str!("../../../../claudine/docs/research/acp/_research.md")),
        ("claudine/docs/research/agent-cli/_research.md", include_str!("../../../../claudine/docs/research/agent-cli/_research.md")),
        ("claudine/docs/research/agent-errors/_research.md", include_str!("../../../../claudine/docs/research/agent-errors/_research.md")),
        ("claudine/docs/research/agent-logging/_research.md", include_str!("../../../../claudine/docs/research/agent-logging/_research.md")),
        ("claudine/docs/research/agent-models/_research.md", include_str!("../../../../claudine/docs/research/agent-models/_research.md")),
        ("claudine/docs/research/agent-permissions/_research.md", include_str!("../../../../claudine/docs/research/agent-permissions/_research.md")),
        ("claudine/docs/research/hooks/_research.md", include_str!("../../../../claudine/docs/research/hooks/_research.md")),
        ("claudine/docs/research/local_runners/_research.md", include_str!("../../../../claudine/docs/research/local_runners/_research.md")),
        ("claudine/docs/research/mcp/_research.md", include_str!("../../../../claudine/docs/research/mcp/_research.md")),
        ("claudine/docs/research/memory/_research.md", include_str!("../../../../claudine/docs/research/memory/_research.md")),
        ("claudine/docs/research/model-config/_research.md", include_str!("../../../../claudine/docs/research/model-config/_research.md")),
        ("claudine/docs/research/non-interactive-sessions/_research.md", include_str!("../../../../claudine/docs/research/non-interactive-sessions/_research.md")),
        ("claudine/docs/research/plugins/_research.md", include_str!("../../../../claudine/docs/research/plugins/_research.md")),
        ("claudine/docs/research/reasoning-level/_research.md", include_str!("../../../../claudine/docs/research/reasoning-level/_research.md")),
        ("claudine/docs/research/resume/_research.md", include_str!("../../../../claudine/docs/research/resume/_research.md")),
        ("claudine/docs/research/signals/_research.md", include_str!("../../../../claudine/docs/research/signals/_research.md")),
        ("claudine/docs/research/skills/_research.md", include_str!("../../../../claudine/docs/research/skills/_research.md")),
        ("claudine/docs/research/slash-commands/_research.md", include_str!("../../../../claudine/docs/research/slash-commands/_research.md")),
        ("claudine/docs/research/steering/_research.md", include_str!("../../../../claudine/docs/research/steering/_research.md")),
        ("claudine/docs/research/subagents/_research.md", include_str!("../../../../claudine/docs/research/subagents/_research.md")),
        ("claudine/docs/research/system-prompt/_research.md", include_str!("../../../../claudine/docs/research/system-prompt/_research.md")),
        ("claudine/docs/research/usage/_research.md", include_str!("../../../../claudine/docs/research/usage/_research.md")),
        ("claudine/docs/research/summary/acp.md", include_str!("../../../../claudine/docs/research/summary/acp.md")),
        ("claudine/docs/research/summary/agent-cli.md", include_str!("../../../../claudine/docs/research/summary/agent-cli.md")),
        ("claudine/docs/research/summary/agent-logging.md", include_str!("../../../../claudine/docs/research/summary/agent-logging.md")),
        ("claudine/docs/research/summary/agent-models.md", include_str!("../../../../claudine/docs/research/summary/agent-models.md")),
        ("claudine/docs/research/summary/agent-permissions.md", include_str!("../../../../claudine/docs/research/summary/agent-permissions.md")),
        ("claudine/docs/research/summary/agent-plugins.md", include_str!("../../../../claudine/docs/research/summary/agent-plugins.md")),
        ("claudine/docs/research/summary/agent-skills.md", include_str!("../../../../claudine/docs/research/summary/agent-skills.md")),
        ("claudine/docs/research/summary/hooks.md", include_str!("../../../../claudine/docs/research/summary/hooks.md")),
        ("claudine/docs/research/summary/local-runners.md", include_str!("../../../../claudine/docs/research/summary/local-runners.md")),
        ("claudine/docs/research/summary/mcp.md", include_str!("../../../../claudine/docs/research/summary/mcp.md")),
        ("claudine/docs/research/summary/model-config.md", include_str!("../../../../claudine/docs/research/summary/model-config.md")),
        ("claudine/docs/research/summary/non-interactive-sessions.md", include_str!("../../../../claudine/docs/research/summary/non-interactive-sessions.md")),
        ("claudine/docs/research/summary/plugins.md", include_str!("../../../../claudine/docs/research/summary/plugins.md")),
        ("claudine/docs/research/summary/session-resumption.md", include_str!("../../../../claudine/docs/research/summary/session-resumption.md")),
        ("claudine/docs/research/summary/slash-commands.md", include_str!("../../../../claudine/docs/research/summary/slash-commands.md")),
        ("claudine/docs/research/summary/steering.md", include_str!("../../../../claudine/docs/research/summary/steering.md")),
        ("claudine/docs/research/summary/subagents.md", include_str!("../../../../claudine/docs/research/summary/subagents.md")),
        ("claudine/docs/research/summary/system-prompt.md", include_str!("../../../../claudine/docs/research/summary/system-prompt.md")),
        ("claudine/docs/research/_TEMPLATE.md", include_str!("../../../../claudine/docs/research/_TEMPLATE.md")),
        ("claudine/docs/research/model-serving-api-standards/_research.md", include_str!("../../../../claudine/docs/research/model-serving-api-standards/_research.md")),
        ("schematic/docs/research/project-mgmt/canonical.md", include_str!("../../../../schematic/docs/research/project-mgmt/canonical.md")),
        ("biscuit-tui/docs/components/_build.md", include_str!("../../../../biscuit-tui/docs/components/_build.md")),
        ("biscuit-tui/docs/style-guide/_style-guide.md", include_str!("../../../../biscuit-tui/docs/style-guide/_style-guide.md")),
        ("reaper/docs/research/schemas.md", include_str!("../../../../reaper/docs/research/schemas.md")),
        ("darkmatter/docs/research/parsing/overview.md", include_str!("../../../../darkmatter/docs/research/parsing/overview.md")),
        ("darkmatter/docs/inline/preflight-checks.md", include_str!("../../../../darkmatter/docs/inline/preflight-checks.md")),
    ];
    let listed: std::collections::BTreeSet<_> = artifacts.iter().map(|(path, _)| *path).collect();
    let research = biscuit_test_harness::manifest_dir!().join("../../claudine/docs/research");
    for entry in std::fs::read_dir(&research).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        for file in ["_fleet.md", "_research.md"] {
            if path.join(file).is_file() {
                let relative = format!("claudine/docs/research/{name}/{file}");
                assert!(listed.contains(relative.as_str()), "uncovered research artifact: {relative}");
            }
        }
    }
    for entry in std::fs::read_dir(research.join("summary")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "md") {
            let relative = format!("claudine/docs/research/summary/{}", path.file_name().unwrap().to_string_lossy());
            assert!(listed.contains(relative.as_str()), "uncovered summary artifact: {relative}");
        }
    }
    for (path, text) in artifacts {
        let markdown = darkmatter::markdown::Markdown::try_from_content(text.to_owned())
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let frontmatter = serde_json::Value::Object(markdown.frontmatter().as_map().iter()
            .map(|(key, value)| (key.clone(), value.clone())).collect());
        let lifecycle = claudine::composition::lifecycle::parse_lifecycle_config(
            &frontmatter, std::path::Path::new(path),
        ).unwrap_or_else(|error| panic!("{path}: {error}"));
        let body = format!("{}\n{}", markdown.content(), frontmatter.get("prompt").and_then(serde_json::Value::as_str).unwrap_or(""));
        claudine::composition::lifecycle::bindings::reject_removed_sequence_reads(
            std::path::Path::new(path),
            claudine::composition::lifecycle::bindings::AuthoredDocument {
                frontmatter: &frontmatter, body: &body, body_line_offset: 0,
            }, None, Some(&lifecycle),
        ).unwrap_or_else(|error| panic!("{path}: {error}"));
    }
}

#[test]
fn shipped_component_sequence_renders_rows_through_the_normal_cli() {
    let fixture = CliProcessFixture::named("shipped-component-sequence");
    let document = include_str!("../../../../biscuit-tui/docs/components/_build.md");
    let source = fixture.cwd().join("components.md");
    std::fs::write(&source, document.replace("@biscuit-tui/components.yaml", "components.yaml")).unwrap();
    std::fs::write(fixture.cwd().join("components.yaml"), "- name: contract-component\n").unwrap();
    let output = fixture.command()
        .args(["sequence", "--dry-run", "--goose", "--silent"])
        .arg(&source).assert().success().get_output().clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(stdout.contains("documenting the \"contract-component\" component"), "{stdout}");
    assert!(stdout.contains("biscuit-tui/docs/components/contract-component"), "{stdout}");
    assert!(!stdout.contains("seq.name"), "{stdout}");
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn research_task_rosters_preserve_canonical_rows_and_templates() {
    let rosters = [
        (include_str!("../../../../claudine/docs/providers.yaml"), include_str!("../../../../claudine/docs/research/_provider-tasks.yaml")),
        (include_str!("../../../../claudine/docs/local-runners.yaml"), include_str!("../../../../claudine/docs/research/_local-runner-tasks.yaml")),
    ];
    for (canonical, projection) in rosters {
        let canonical: serde_json::Value = yaml_value(canonical);
        let mut projection: serde_json::Value = yaml_value(projection);
        let prompt = projection["template"].as_object_mut().unwrap().remove("prompt").unwrap();
        assert_eq!(prompt, "{{ research_prompt }}");
        assert_eq!(canonical, projection, "research roster changed canonical row fields, order, or defaults");
    }
}

#[test]
fn shipped_fleet_graphs_resolve_per_row_prompt_lifecycles() {
    let root = biscuit_test_harness::manifest_dir!().join("../..");
    let directory = root.join("claudine/docs/research");
    let fixture = CliProcessFixture::named("research-corpus-context");
    fixture.initialize_repository();
    let staged = fixture.cwd().join("claudine/docs/research");
    std::fs::create_dir_all(&staged).unwrap();
    for name in ["_provider-tasks.yaml", "_local-runner-tasks.yaml"] {
        std::fs::copy(directory.join(name), staged.join(name)).unwrap();
    }
    for entry in std::fs::read_dir(&directory).unwrap() {
        let topic = entry.unwrap().path();
        if !topic.join("_fleet.md").is_file() { continue; }
        let destination = staged.join(topic.file_name().unwrap());
        std::fs::create_dir_all(&destination).unwrap();
        for name in ["_fleet.md", "_research.md"] {
            std::fs::copy(topic.join(name), destination.join(name)).unwrap();
        }
    }
    // Graph discovery follows includes; keep its internal prompt dependency frozen.
    let output_style = include_str!("../fixtures/frozen_prompts/prompts/make-it-markdown.md");
    std::fs::create_dir_all(fixture.cwd().join("prompts")).unwrap();
    std::fs::write(fixture.cwd().join("prompts/make-it-markdown.md"), output_style).unwrap();
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_dir(fixture.cwd());
    let resolution = biscuit_file::FileResolutionContext::new(fixture.cwd());
    let mut count = 0;
    for entry in std::fs::read_dir(staged).unwrap() {
        let path = entry.unwrap().path().join("_fleet.md");
        if !path.is_file() { continue; }
        count += 1;
        let markdown = darkmatter::markdown::Markdown::try_from(path.as_path()).unwrap();
        assert!(!markdown.frontmatter().as_map().contains_key("initialize"), "row hooks must live on the prompt");
        let source = claudine::composition::ResolvedCompositionSource {
            original_ref: path.display().to_string(), resolved_path: path.clone(),
            original_text: markdown.as_string(), markdown,
        };
        let plan = claudine::composition::resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
        let graph = claudine::composition::sequence::preflight::build_preflight_graph_with_context(
            &plan, &source, context.clone(), &resolution,
        ).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(graph.prompt_documents.len(), 1, "{}", path.display());
        assert!(graph.prompt_documents[0].path.ends_with("_research.md"));
        assert_eq!(graph.steps.len(), plan.steps.len());
        assert!(graph.steps.iter().all(|step| step.task.is_some()), "every roster row must be executable");
        assert!(!graph.prompt_references.is_empty());
    }
    assert_eq!(count, 22);
}

#[cfg(unix)]
#[test]
fn shipped_research_fleet_runs_row_freshness_and_success_through_normal_cli() {
    let coordinator = include_str!("../../../../claudine/docs/research/hooks/_fleet.md");
    let prompt = include_str!("../../../../claudine/docs/research/hooks/_research.md");
    let roster = include_str!("../../../../claudine/docs/research/_provider-tasks.yaml");
    let fixture = CliProcessFixture::named("shipped-research-fleet");
    fixture.initialize_repository();
    let directory = fixture.cwd().join("claudine/docs/research/hooks");
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("_fleet.md");
    std::fs::write(&source, coordinator).unwrap();
    // The output style dependency is unrelated to lifecycle behavior and is an internal draft.
    std::fs::write(directory.join("_research.md"), prompt.replace("::file @prompts/make-it-markdown.md", "Write clear Markdown.")).unwrap();
    let mut roster: serde_json::Value = yaml_value(roster);
    roster["sequence"].as_array_mut().unwrap().truncate(2);
    std::fs::write(directory.parent().unwrap().join("_provider-tasks.yaml"), serde_json::to_string_pretty(&roster).unwrap()).unwrap();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    std::fs::write(directory.join("claude.md"), format!("---\nlast_updated: '{today}'\n---\nCurrent research.\n")).unwrap();
    let recordings = crate::common::launch_recorder::install(&fixture, "claude", &format!(
        "printf '%s\n' '---' \"last_updated: '{today}'\" '---' 'Updated research.' > '{}'\nprintf 'research complete\n'", directory.join("codex.md").display(),
    ));
    for _ in 0..2 {
        fixture.command().args(["sequence", "--claude", "--yolo", "--silent"])
            .arg(&source).assert().success();
    }
    let prompts = crate::common::launch_recorder::prompts(&recordings);
    assert_eq!(prompts.len(), 1, "fresh rows must skip, including the just-validated row on reread");
    assert!(prompts[0].contains("Hooks Research on Codex CLI"), "{}", prompts[0]);
    assert!(!prompts[0].contains("{{seq."));
    let written = std::fs::read_to_string(directory.join("codex.md")).unwrap();
    assert!(written.contains(&today));
    assert!(written.contains("Updated research."));
    assert!(!fixture.audio_spool().exists());
}

#[cfg(unix)]
#[test]
fn shipped_research_fleet_refuses_zero_exit_without_a_research_document() {
    let coordinator = include_str!("../../../../claudine/docs/research/hooks/_fleet.md");
    let prompt = include_str!("../../../../claudine/docs/research/hooks/_research.md");
    let roster = include_str!("../../../../claudine/docs/research/_provider-tasks.yaml");
    let fixture = CliProcessFixture::named("shipped-research-fleet-missing-output");
    fixture.initialize_repository();
    let directory = fixture.cwd().join("claudine/docs/research/hooks");
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("_fleet.md");
    std::fs::write(&source, coordinator).unwrap();
    // Freeze the style dependency; the subject is the shipped row lifecycle.
    std::fs::write(directory.join("_research.md"), prompt.replace("::file @prompts/make-it-markdown.md", "Write clear Markdown.")).unwrap();
    let mut roster: serde_json::Value = yaml_value(roster);
    roster["sequence"].as_array_mut().unwrap().truncate(1);
    std::fs::write(directory.parent().unwrap().join("_provider-tasks.yaml"), serde_json::to_string_pretty(&roster).unwrap()).unwrap();
    let recordings = crate::common::launch_recorder::install(&fixture, "claude", "printf 'claimed success\\n'");
    let rejected = fixture.command().args(["sequence", "--claude", "--yolo", "--silent"])
        .arg(&source).assert().failure().get_output().clone();
    let errors = strip_ansi(&String::from_utf8_lossy(&rejected.stderr));
    assert!(errors.contains("invalid file path"), "{errors}");
    assert!(!directory.join("claude.md").exists(), "a provider exit zero must not manufacture research");
    assert_eq!(crate::common::launch_recorder::prompts(&recordings).len(), 1);
    assert!(!fixture.audio_spool().exists());
}

fn yaml_value(source: &str) -> serde_json::Value {
    let markdown = darkmatter::markdown::Markdown::try_from_content(format!("---\n{source}\n---\n")).unwrap();
    serde_json::Value::Object(markdown.frontmatter().as_map().iter()
        .map(|(key, value)| (key.clone(), value.clone())).collect())
}
