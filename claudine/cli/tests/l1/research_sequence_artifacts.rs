//! Shipped research and documentation sequences retain lexical row bindings.

use crate::common::{CliProcessFixture, strip_ansi};

#[test]
fn shipped_research_sequence_corpus_uses_lexical_row_bindings() {
    let artifacts = [
        ("claudine/docs/research/providers/acp/_fleet.md", include_str!("../../../../claudine/docs/research/providers/acp/_fleet.md")),
        ("claudine/docs/research/providers/agent-cli/_fleet.md", include_str!("../../../../claudine/docs/research/providers/agent-cli/_fleet.md")),
        ("claudine/docs/research/providers/agent-errors/_fleet.md", include_str!("../../../../claudine/docs/research/providers/agent-errors/_fleet.md")),
        ("claudine/docs/research/providers/agent-logging/_fleet.md", include_str!("../../../../claudine/docs/research/providers/agent-logging/_fleet.md")),
        ("claudine/docs/research/providers/agent-models/_fleet.md", include_str!("../../../../claudine/docs/research/providers/agent-models/_fleet.md")),
        ("claudine/docs/research/providers/agent-permissions/_fleet.md", include_str!("../../../../claudine/docs/research/providers/agent-permissions/_fleet.md")),
        ("claudine/docs/research/hooks/_fleet.md", include_str!("../../../../claudine/docs/research/hooks/_fleet.md")),
        ("claudine/docs/research/local-ai/local_runners/_fleet.md", include_str!("../../../../claudine/docs/research/local-ai/local_runners/_fleet.md")),
        ("claudine/docs/research/providers/mcp/_fleet.md", include_str!("../../../../claudine/docs/research/providers/mcp/_fleet.md")),
        ("claudine/docs/research/providers/memory/_fleet.md", include_str!("../../../../claudine/docs/research/providers/memory/_fleet.md")),
        ("claudine/docs/research/providers/model-config/_fleet.md", include_str!("../../../../claudine/docs/research/providers/model-config/_fleet.md")),
        ("claudine/docs/research/providers/non-interactive-sessions/_fleet.md", include_str!("../../../../claudine/docs/research/providers/non-interactive-sessions/_fleet.md")),
        ("claudine/docs/research/providers/plugins/_fleet.md", include_str!("../../../../claudine/docs/research/providers/plugins/_fleet.md")),
        ("claudine/docs/research/providers/reasoning-level/_fleet.md", include_str!("../../../../claudine/docs/research/providers/reasoning-level/_fleet.md")),
        ("claudine/docs/research/providers/resume/_fleet.md", include_str!("../../../../claudine/docs/research/providers/resume/_fleet.md")),
        ("claudine/docs/research/providers/signals/_fleet.md", include_str!("../../../../claudine/docs/research/providers/signals/_fleet.md")),
        ("claudine/docs/research/providers/skills/_fleet.md", include_str!("../../../../claudine/docs/research/providers/skills/_fleet.md")),
        ("claudine/docs/research/providers/slash-commands/_fleet.md", include_str!("../../../../claudine/docs/research/providers/slash-commands/_fleet.md")),
        ("claudine/docs/research/providers/steering/_fleet.md", include_str!("../../../../claudine/docs/research/providers/steering/_fleet.md")),
        ("claudine/docs/research/providers/subagents/_fleet.md", include_str!("../../../../claudine/docs/research/providers/subagents/_fleet.md")),
        ("claudine/docs/research/providers/system-prompt/_fleet.md", include_str!("../../../../claudine/docs/research/providers/system-prompt/_fleet.md")),
        ("claudine/docs/research/providers/usage/_fleet.md", include_str!("../../../../claudine/docs/research/providers/usage/_fleet.md")),
        ("claudine/docs/research/providers/acp/_research.md", include_str!("../../../../claudine/docs/research/providers/acp/_research.md")),
        ("claudine/docs/research/providers/agent-cli/_research.md", include_str!("../../../../claudine/docs/research/providers/agent-cli/_research.md")),
        ("claudine/docs/research/providers/agent-errors/_research.md", include_str!("../../../../claudine/docs/research/providers/agent-errors/_research.md")),
        ("claudine/docs/research/providers/agent-logging/_research.md", include_str!("../../../../claudine/docs/research/providers/agent-logging/_research.md")),
        ("claudine/docs/research/providers/agent-models/_research.md", include_str!("../../../../claudine/docs/research/providers/agent-models/_research.md")),
        ("claudine/docs/research/providers/agent-permissions/_research.md", include_str!("../../../../claudine/docs/research/providers/agent-permissions/_research.md")),
        ("claudine/docs/research/hooks/_research.md", include_str!("../../../../claudine/docs/research/hooks/_research.md")),
        ("claudine/docs/research/local-ai/local_runners/_research.md", include_str!("../../../../claudine/docs/research/local-ai/local_runners/_research.md")),
        ("claudine/docs/research/providers/mcp/_research.md", include_str!("../../../../claudine/docs/research/providers/mcp/_research.md")),
        ("claudine/docs/research/providers/memory/_research.md", include_str!("../../../../claudine/docs/research/providers/memory/_research.md")),
        ("claudine/docs/research/providers/model-config/_research.md", include_str!("../../../../claudine/docs/research/providers/model-config/_research.md")),
        ("claudine/docs/research/providers/non-interactive-sessions/_research.md", include_str!("../../../../claudine/docs/research/providers/non-interactive-sessions/_research.md")),
        ("claudine/docs/research/providers/plugins/_research.md", include_str!("../../../../claudine/docs/research/providers/plugins/_research.md")),
        ("claudine/docs/research/providers/reasoning-level/_research.md", include_str!("../../../../claudine/docs/research/providers/reasoning-level/_research.md")),
        ("claudine/docs/research/providers/resume/_research.md", include_str!("../../../../claudine/docs/research/providers/resume/_research.md")),
        ("claudine/docs/research/providers/signals/_research.md", include_str!("../../../../claudine/docs/research/providers/signals/_research.md")),
        ("claudine/docs/research/providers/skills/_research.md", include_str!("../../../../claudine/docs/research/providers/skills/_research.md")),
        ("claudine/docs/research/providers/slash-commands/_research.md", include_str!("../../../../claudine/docs/research/providers/slash-commands/_research.md")),
        ("claudine/docs/research/providers/steering/_research.md", include_str!("../../../../claudine/docs/research/providers/steering/_research.md")),
        ("claudine/docs/research/providers/subagents/_research.md", include_str!("../../../../claudine/docs/research/providers/subagents/_research.md")),
        ("claudine/docs/research/providers/system-prompt/_research.md", include_str!("../../../../claudine/docs/research/providers/system-prompt/_research.md")),
        ("claudine/docs/research/providers/usage/_research.md", include_str!("../../../../claudine/docs/research/providers/usage/_research.md")),
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
    // A topic is a folder of the research tree or of one of its subject
    // areas (`providers/`, `local-ai/`).
    let folders = |dir: &std::path::Path| -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).filter(|path| path.is_dir()).collect()
    };
    let mut seen = 0;
    for area in folders(&research) {
        for path in std::iter::once(area.clone()).chain(folders(&area)) {
            let name = path.strip_prefix(&research).unwrap().to_string_lossy().replace('\\', "/");
            for file in ["_fleet.md", "_research.md"] {
                if path.join(file).is_file() {
                    seen += 1;
                    let relative = format!("claudine/docs/research/{name}/{file}");
                    assert!(listed.contains(relative.as_str()), "uncovered research artifact: {relative}");
                }
            }
        }
    }
    assert!(seen > 0, "the walk found no research artifact under {}", research.display());
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

/// A real run of the shipped component sequence, on a Goose stub that
/// records the prompt it receives, renders the row's `seq.name`.
#[test]
fn shipped_component_sequence_renders_rows_through_the_normal_cli() {
    let fixture = CliProcessFixture::named("shipped-component-sequence");
    crate::common::provenance::install_goose(&fixture);
    let document = include_str!("../../../../biscuit-tui/docs/components/_build.md");
    let source = fixture.cwd().join("components.md");
    std::fs::write(&source, document.replace("@biscuit-tui/components.yaml", "components.yaml")).unwrap();
    std::fs::write(fixture.cwd().join("components.yaml"), "- name: contract-component\n").unwrap();
    let output = fixture.command()
        .args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(&source).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{stderr}");
    let prompt = std::fs::read_to_string(fixture.home().join("provider-prompt")).unwrap_or_default();
    assert!(prompt.contains("documenting the \"contract-component\" component"), "{prompt}\n{stderr}");
    assert!(prompt.contains("biscuit-tui/docs/components/contract-component"), "{prompt}");
    assert!(!prompt.contains("seq.name"), "{prompt}");
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

/// One test per shipped fleet, each building that fleet's preflight graph
/// alone, so nextest schedules the graph builds in parallel. Each case names
/// its repository files as literals inside its own `#[test]` body, so an edit
/// to one fleet schedules only its case in CI.
mod shipped_fleet_graphs_resolve_per_row_prompt_lifecycles {
    use super::*;

    macro_rules! fleet_graph_tests {
        ($(#[test] fn $name:ident() { $fleet:literal, $research:literal, $roster:literal, $style:literal $(,)? })*) => {
            $(#[test]
            fn $name() {
                assert_fleet_graph($fleet, $research, $roster, $style);
            })*

            /// The fleet of every case above, checked against the shipped set.
            const CASES: &[&str] = &[$($fleet),*];
        };
    }

    /// Stages the fleet, its row prompt, and its roster at their repository
    /// paths in a fresh repository, with the frozen output-style prompt the
    /// row prompt includes, and builds the fleet's preflight graph.
    fn assert_fleet_graph(fleet: &str, research: &str, roster: &str, style: &str) {
        let root = biscuit_test_harness::manifest_dir!().join("../..");
        assert_eq!(std::path::Path::new(research).parent(), std::path::Path::new(fleet).parent(), "{fleet}");
        let fixture = CliProcessFixture::named("research-corpus-context");
        fixture.initialize_repository();
        for name in [fleet, research, roster] {
            let destination = fixture.cwd().join(name);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::copy(root.join(name), destination).unwrap();
        }
        // Graph discovery follows includes; keep its internal prompt dependency frozen.
        let output_style = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join(style)).unwrap();
        std::fs::create_dir_all(fixture.cwd().join("prompts")).unwrap();
        std::fs::write(fixture.cwd().join("prompts/make-it-markdown.md"), output_style).unwrap();
        let context = darkmatter::markdown::compose::ComposeContext::capture_for_dir(fixture.cwd());
        let resolution = biscuit_file::FileResolutionContext::new(fixture.cwd());
        let path = fixture.cwd().join(fleet);
        let markdown = darkmatter::markdown::Markdown::try_from(path.as_path()).unwrap();
        // The shared roster sees no property of the fleet, so the fleet's one
        // root hook writes the topic prompt the roster's template reads; row
        // hooks live on the prompt.
        let initialize = markdown.frontmatter().as_map().get("initialize").cloned();
        let writes = initialize.as_ref().and_then(|stack| stack.as_array()).and_then(|stack| match stack.as_slice() {
            [only] => only.get("set").filter(|writes| writes.as_object().is_some_and(|map| map.len() == 1 && map.contains_key("research_prompt"))).cloned(),
            _ => None,
        });
        let writes = writes.unwrap_or_else(|| panic!("{}: `initialize` must be one `set` of `research_prompt`: {initialize:?}", path.display()));
        let layers = claudine::composition::CallerInputLayers::from_caller_overrides(Some(writes), resolution.clone());
        let source = claudine::composition::ResolvedCompositionSource {
            original_ref: path.display().to_string(), resolved_path: path.clone(),
            original_text: markdown.as_string(), markdown,
        };
        let plan = claudine::composition::resolve_sequence_plan_with(
            &source, claudine::composition::SequenceSourceOptions::new(&resolution).with_caller_layers(&layers),
        ).unwrap().unwrap();
        let graph = claudine::composition::sequence::preflight::build_preflight_graph_with_context(
            &plan, &source, context, &resolution,
        ).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(graph.prompt_documents.len(), 1, "{}", path.display());
        assert!(graph.prompt_documents[0].path.ends_with("_research.md"));
        assert_eq!(graph.steps.len(), plan.steps.len());
        assert!(graph.steps.iter().all(|step| step.task.is_some()), "every roster row must be executable");
        assert!(!graph.prompt_references.is_empty());
    }

    fleet_graph_tests! {
    #[test] fn hooks() {
        "claudine/docs/research/hooks/_fleet.md",
        "claudine/docs/research/hooks/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn local_runners() {
        "claudine/docs/research/local-ai/local_runners/_fleet.md",
        "claudine/docs/research/local-ai/local_runners/_research.md",
        "claudine/docs/research/_local-runner-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn acp() {
        "claudine/docs/research/providers/acp/_fleet.md",
        "claudine/docs/research/providers/acp/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn agent_cli() {
        "claudine/docs/research/providers/agent-cli/_fleet.md",
        "claudine/docs/research/providers/agent-cli/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn agent_errors() {
        "claudine/docs/research/providers/agent-errors/_fleet.md",
        "claudine/docs/research/providers/agent-errors/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn agent_logging() {
        "claudine/docs/research/providers/agent-logging/_fleet.md",
        "claudine/docs/research/providers/agent-logging/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn agent_models() {
        "claudine/docs/research/providers/agent-models/_fleet.md",
        "claudine/docs/research/providers/agent-models/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn agent_permissions() {
        "claudine/docs/research/providers/agent-permissions/_fleet.md",
        "claudine/docs/research/providers/agent-permissions/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn mcp() {
        "claudine/docs/research/providers/mcp/_fleet.md",
        "claudine/docs/research/providers/mcp/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn memory() {
        "claudine/docs/research/providers/memory/_fleet.md",
        "claudine/docs/research/providers/memory/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn model_config() {
        "claudine/docs/research/providers/model-config/_fleet.md",
        "claudine/docs/research/providers/model-config/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn non_interactive_sessions() {
        "claudine/docs/research/providers/non-interactive-sessions/_fleet.md",
        "claudine/docs/research/providers/non-interactive-sessions/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn plugins() {
        "claudine/docs/research/providers/plugins/_fleet.md",
        "claudine/docs/research/providers/plugins/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn reasoning_level() {
        "claudine/docs/research/providers/reasoning-level/_fleet.md",
        "claudine/docs/research/providers/reasoning-level/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn resume() {
        "claudine/docs/research/providers/resume/_fleet.md",
        "claudine/docs/research/providers/resume/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn signals() {
        "claudine/docs/research/providers/signals/_fleet.md",
        "claudine/docs/research/providers/signals/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn skills() {
        "claudine/docs/research/providers/skills/_fleet.md",
        "claudine/docs/research/providers/skills/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn slash_commands() {
        "claudine/docs/research/providers/slash-commands/_fleet.md",
        "claudine/docs/research/providers/slash-commands/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn steering() {
        "claudine/docs/research/providers/steering/_fleet.md",
        "claudine/docs/research/providers/steering/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn subagents() {
        "claudine/docs/research/providers/subagents/_fleet.md",
        "claudine/docs/research/providers/subagents/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn system_prompt() {
        "claudine/docs/research/providers/system-prompt/_fleet.md",
        "claudine/docs/research/providers/system-prompt/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    #[test] fn usage() {
        "claudine/docs/research/providers/usage/_fleet.md",
        "claudine/docs/research/providers/usage/_research.md",
        "claudine/docs/research/_provider-tasks.yaml",
        "tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
    }
    }

    /// Every shipped fleet has exactly one case above: a new fleet without a
    /// case, or a case whose fleet is gone, fails here.
    #[test]
    fn every_shipped_fleet_has_a_case() {
        let research = biscuit_test_harness::manifest_dir!().join("../../claudine/docs/research");
        // A topic is a folder of the research tree or of one of its subject
        // areas (`providers/`, `local-ai/`).
        let folders = |dir: &std::path::Path| -> Vec<std::path::PathBuf> {
            std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).filter(|path| path.is_dir()).collect()
        };
        let mut shipped = Vec::new();
        for area in folders(&research) {
            for topic in std::iter::once(area.clone()).chain(folders(&area)) {
                if topic.join("_fleet.md").is_file() {
                    let name = topic.strip_prefix(&research).unwrap().to_string_lossy().replace('\\', "/");
                    shipped.push(format!("claudine/docs/research/{name}/_fleet.md"));
                }
            }
        }
        shipped.sort();
        let mut cases: Vec<String> = CASES.iter().map(ToString::to_string).collect();
        cases.sort();
        assert_eq!(cases, shipped, "add or remove a `fleet_graph_tests!` case");
        assert_eq!(cases.len(), 22);
    }
}

/// A shipped fleet over two roster rows: the fresh first row skips, and the
/// stale second row launches and must leave a validated research document.
/// Re-running it skips both, including the row just validated.
#[cfg(unix)]
mod shipped_research_fleet_runs_row_freshness_and_success_through_normal_cli {
    use super::*;

    /// What the codex row's provider stub writes on `today`.
    fn written_research(today: &str) -> String {
        format!("---\nlast_updated: '{today}'\n---\nUpdated research.\n")
    }

    /// The fleet, its row prompt, and a two-row roster under the fixture's
    /// repository, with a current `claude.md`; returns the fixture, the fleet
    /// source, the research directory, and `today`.
    fn staged(
        coordinator: &str,
        prompt: &str,
        roster: &str,
    ) -> (CliProcessFixture, std::path::PathBuf, std::path::PathBuf, String) {
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
        (fixture, source, directory, today)
    }

    #[test]
    fn the_stale_row_launches_and_is_validated() {
        let coordinator = include_str!("../../../../claudine/docs/research/hooks/_fleet.md");
        let prompt = include_str!("../../../../claudine/docs/research/hooks/_research.md");
        let roster = include_str!("../../../../claudine/docs/research/_provider-tasks.yaml");
        let (fixture, source, directory, today) = staged(coordinator, prompt, roster);
        let recordings = crate::common::launch_recorder::install(&fixture, "claude", &format!(
            "printf '%s\n' '---' \"last_updated: '{today}'\" '---' 'Updated research.' > '{}'\nprintf 'research complete\n'", directory.join("codex.md").display(),
        ));
        fixture.command().args(["sequence", "--claude", "--yolo", "--silent"])
            .arg(&source).assert().success();
        let prompts = crate::common::launch_recorder::prompts(&recordings);
        assert_eq!(prompts.len(), 1, "the fresh row must skip");
        assert!(prompts[0].contains("Hooks Research on Codex CLI"), "{}", prompts[0]);
        assert!(!prompts[0].contains("{{seq."));
        let written = std::fs::read_to_string(directory.join("codex.md")).unwrap();
        assert_eq!(written, written_research(&today), "the stub's write is the row's whole state");
        assert!(!fixture.audio_spool().exists());
    }

    /// The re-run's starting state is exactly what the run above leaves: the
    /// provider's write is the only change it makes that freshness reads.
    #[test]
    fn a_rerun_skips_every_fresh_row_including_the_one_just_validated() {
        let coordinator = include_str!("../../../../claudine/docs/research/hooks/_fleet.md");
        let prompt = include_str!("../../../../claudine/docs/research/hooks/_research.md");
        let roster = include_str!("../../../../claudine/docs/research/_provider-tasks.yaml");
        let (fixture, source, directory, today) = staged(coordinator, prompt, roster);
        std::fs::write(directory.join("codex.md"), written_research(&today)).unwrap();
        let recordings = crate::common::launch_recorder::install(&fixture, "claude", "exit 0");
        fixture.command().args(["sequence", "--claude", "--yolo", "--silent"])
            .arg(&source).assert().success();
        assert!(
            crate::common::launch_recorder::prompts(&recordings).is_empty(),
            "fresh rows must skip, including the just-validated row on reread"
        );
        assert!(!fixture.audio_spool().exists());
    }
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
