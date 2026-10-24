//! Every shipped document Claudine runs, judged by both of its gates: the
//! schema verdict (trigger discovery, effective schema, and validation over
//! the shipped `claudine/schemas/` bundle, the path DMLS and
//! `md schema validate` take) and Claudine's lifecycle parser
//! (`2026-10-30-claudine-recursive-schemas`, Phase 5).
//!
//! The corpus is the shipped examples under `claudine/docs/examples/`, the
//! research fleet coordinators and row prompts under
//! `claudine/docs/research/<topic>/`, and the frozen prompt copies under
//! both Claudine packages' `tests/fixtures/frozen_prompts/`. Each document
//! has its own test, which reads it with `include_str!`, so an edit to one
//! schedules exactly its test; `every_shipped_document_has_one_test` fails
//! when a document is added to one of those folders without a test.
//!
//! The schema verdict is the passive one: frontmatter is judged as written,
//! before interpolation. So a whole-value `{{ … }}` template under an
//! authored `$schema` parameter, and a caller-supplied required parameter
//! that is absent, are problems here even though a launch supplies or renders
//! them. Every table row that expects problems lists them exactly, and
//! `assert_shipped` also requires each problem to belong to a parameter the
//! document's own `$schema` declares. A problem on a Claudine control would
//! mean the shipped schemas reject a shipped document.
//!
//! The parser verdict parses only: the root lifecycle stacks, the document
//! `loop:` controls, and an inline `sequence:` list. A referenced sequence
//! (the fleets' roster) is not loaded here; the CLI suite's
//! `shipped_fleet_graphs_resolve_per_row_prompt_lifecycles` resolves it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use claudine::composition::{
    CompositionError, parse_lifecycle_config, resolve_composition_source_in_context, resolve_loop_config,
    resolve_sequence_plan,
};
use darkmatter::markdown::Markdown;
use serde_json::Value;

use crate::claudine_schemas::{SchemaRepo, frontmatter_json, problems, shipped_bundle};

/// Which shipped triggers select a document.
struct Selects {
    claudine: bool,
    inline: bool,
    sequence: bool,
}

const NONE: Selects = Selects { claudine: false, inline: false, sequence: false };
const CLAUDINE: Selects = Selects { claudine: true, inline: false, sequence: false };
const SEQUENCE: Selects = Selects { claudine: false, inline: false, sequence: true };
const CLAUDINE_AND_SEQUENCE: Selects = Selects { claudine: true, inline: false, sequence: true };

/// The schema verdict a document must get.
enum Schema {
    Valid,
    /// Exactly these `(instance path, text in the message)` problems.
    Invalid(&'static [(&'static str, &'static str)]),
}

use Schema::{Invalid, Valid};

/// Whether Claudine's lifecycle parser accepts the document.
const ACCEPTS: bool = true;

/// One shipped document and its declared verdicts.
struct Shipped {
    /// The document's path below `claudine/`, for messages.
    label: &'static str,
    text: &'static str,
    selects: Selects,
    schema: Schema,
    parser: bool,
}

/// The parameter names a document's own `$schema` declares: the keys of its
/// mapping, or of every arm of its list.
fn authored_parameters(frontmatter: &Value) -> BTreeSet<String> {
    let arms: Vec<&Value> = match frontmatter.get("$schema") {
        None => Vec::new(),
        Some(Value::Array(arms)) => arms.iter().collect(),
        Some(single) => vec![single],
    };
    arms.into_iter()
        .map(|arm| arm.as_object().expect("every `$schema` arm in the corpus is a mapping"))
        .flat_map(|arm| arm.keys().cloned())
        .collect()
}

/// The top-level property a problem belongs to: the first pointer segment,
/// or for a root problem the property its message quotes first
/// (`"spec" is a required property`).
fn owning_property(pointer: &str, message: &str) -> String {
    match pointer.split('/').nth(1) {
        Some(segment) => segment.to_string(),
        None => message
            .split('"')
            .nth(1)
            .unwrap_or_else(|| panic!("a root problem quotes its property: {message}"))
            .to_string(),
    }
}

/// Claudine's lifecycle parser over the document, parse only: the first
/// error it raises, or `None` when it accepts the document.
fn parser_rejection(repo: &SchemaRepo, path: &Path) -> Option<CompositionError> {
    let context = repo.context();
    let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context)
        .expect("the document resolves");
    let frontmatter = frontmatter_json(&source.markdown);
    let inline_sequence = frontmatter.get("sequence").is_some_and(Value::is_array);
    parse_lifecycle_config(&frontmatter, path)
        .err()
        .or_else(|| resolve_loop_config(&source).err())
        .or_else(|| inline_sequence.then(|| resolve_sequence_plan(&source, &context).err()).flatten())
}

fn assert_shipped(shipped: &Shipped) {
    let Shipped { label, text, selects, schema, parser } = shipped;
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document(&label.replace('/', "--"), text);
    let judged = discovered
        .judge(
            &path,
            include_str!("../../../schemas/claudine.yaml"),
            include_str!("../../../schemas/inline-compose.yaml"),
        )
        .expect("judged");
    let sequence = discovered.selects(&path, include_str!("../../../schemas/sequence.yaml"));

    assert_eq!(
        (judged.claudine, judged.inline, sequence),
        (selects.claudine, selects.inline, selects.sequence),
        "{label}: (claudine.yaml, inline-compose.yaml, sequence.yaml) selection"
    );
    // The selected payloads are what judged the document, so a valid verdict
    // is not vacuous. A list `$schema` lowers to an `anyOf` root whose
    // properties live in its arms, so only a mapping root is read here.
    let authored = Markdown::try_from(path.as_path()).expect("document loads");
    let authored = frontmatter_json(&authored);
    if !authored.get("$schema").is_some_and(Value::is_array) {
        assert_eq!(judged.properties.contains("initialize"), selects.claudine, "{label}: Claudine's payload applies");
        assert_eq!(judged.properties.contains("sequence"), selects.sequence, "{label}: the sequence payload applies");
    }

    let found = problems(&judged.report);
    match schema {
        Valid => assert!(judged.report.valid, "{label}: {found:?}"),
        Invalid(expected) => {
            assert!(!judged.report.valid, "{label} is invalid");
            for (pointer, needle) in *expected {
                assert!(
                    found.iter().any(|(path, message)| path == pointer && message.contains(needle)),
                    "{label}: expected a problem at `{pointer}` naming `{needle}`, got {found:?}"
                );
            }
            assert_eq!(found.len(), expected.len(), "{label}: no other problem: {found:?}");
        }
    }
    let authored = authored_parameters(&authored);
    for (pointer, message) in &found {
        let property = owning_property(pointer, message);
        assert!(
            authored.contains(&property),
            "{label}: `{pointer}` ({message}) is not an authored `$schema` parameter, so a shipped schema rejects it"
        );
    }

    let rejection = parser_rejection(&repo, &path);
    assert_eq!(rejection.is_none(), *parser, "{label}: lifecycle parser verdict: {rejection:?}");
}

/// Every `.md` file below `directory`.
fn markdown_below(directory: PathBuf) -> Vec<PathBuf> {
    let mut pending = vec![directory];
    let mut found = Vec::new();
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())) {
                pending.push(entry.expect("directory entry").path());
            }
        } else if path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
    found
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The corpus folders hold exactly the documents this file's tests read,
/// each read by one test. The tests' reads are taken from this file's own
/// `include_str!` literals, so the list cannot drift from the tests.
#[test]
fn every_shipped_document_has_one_test() {
    let manifest = biscuit_test_harness::manifest_dir!();
    let mut listed = markdown_below(manifest.join("../docs/examples"));
    listed.extend(markdown_below(manifest.join("tests/fixtures/frozen_prompts")));
    listed.extend(markdown_below(manifest.join("../cli/tests/fixtures/frozen_prompts")));
    let research = manifest.join("../docs/research");
    for entry in std::fs::read_dir(&research).unwrap_or_else(|error| panic!("{}: {error}", research.display())) {
        let topic = entry.expect("directory entry").path();
        for name in ["_fleet.md", "_research.md"] {
            if topic.join(name).is_file() {
                listed.push(topic.join(name));
            }
        }
    }
    let mut listed: Vec<PathBuf> = listed.iter().map(|path| canonical(path)).collect();
    listed.sort();

    let tests = manifest.join("tests/l1");
    let mut declared: Vec<PathBuf> = include_str!("claudine_schema_shipped_corpus.rs")
        .split("include_str!(")
        .skip(1)
        .filter_map(|rest| rest.trim_start().strip_prefix('"'))
        .map(|rest| rest.split_once('"').expect("a closed literal").0)
        .filter(|literal| literal.ends_with(".md"))
        .map(|literal| canonical(&tests.join(literal)))
        .collect();
    declared.sort();

    assert!(listed.len() > 40, "listed only {} documents", listed.len());
    assert_eq!(declared, listed, "each shipped document is read by exactly one test");
}

// ── Shipped examples ───────────────────────────────────────────────────────

#[test]
fn example_sequence_containers() {
    assert_shipped(&Shipped {
        label: "docs/examples/sequence-containers.md",
        text: include_str!("../../../docs/examples/sequence-containers.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

// ── Frozen prompt copies (claudine) ────────────────────────────────────────

#[test]
fn frozen_lib_clarify() {
    assert_shipped(&Shipped {
        label: "lib/tests/fixtures/frozen_prompts/prompts/clarify.md",
        text: include_str!("../fixtures/frozen_prompts/prompts/clarify.md"),
        selects: CLAUDINE,
        schema: Invalid(&[("", "\"spec\" is a required property"), ("/doc", "is not a valid file reference")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_lib_review() {
    assert_shipped(&Shipped {
        label: "lib/tests/fixtures/frozen_prompts/prompts/review.md",
        text: include_str!("../fixtures/frozen_prompts/prompts/review.md"),
        selects: CLAUDINE,
        schema: Invalid(&[("", "\"spec\" is a required property")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_lib_reviews_cross_platform() {
    assert_shipped(&Shipped {
        label: "lib/tests/fixtures/frozen_prompts/prompts/_reviews/cross-platform.md",
        text: include_str!("../fixtures/frozen_prompts/prompts/_reviews/cross-platform.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_lib_set_spec_schema() {
    assert_shipped(&Shipped {
        label: "lib/tests/fixtures/frozen_prompts/prompts/_set_spec_schema.md",
        text: include_str!("../fixtures/frozen_prompts/prompts/_set_spec_schema.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

// ── Frozen prompt copies (claudine-cli) ────────────────────────────────────

/// The problems of both frozen `implement-plan.md` copies: whole-value
/// templates under authored `file` and `number` parameters, judged as written.
const IMPLEMENT_PLAN_PROBLEMS: &[(&str, &str)] = &[
    ("/plan", "is not a valid file reference"),
    ("/phase", "is not of type \"number\""),
    ("/total_phases", "is not of type \"number\""),
    ("/spec", "is not a valid file reference"),
    ("/log", "is not a valid file reference"),
];

#[test]
fn frozen_cli_implement_plan() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_implement/implement-plan.md"),
        selects: CLAUDINE,
        schema: Invalid(IMPLEMENT_PLAN_PROBLEMS),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_shipped_implement_route_implement_plan() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md",
        text: include_str!(
            "../../../cli/tests/fixtures/frozen_prompts/shipped_implement_route/_implement/implement-plan.md"
        ),
        selects: CLAUDINE,
        schema: Invalid(IMPLEMENT_PLAN_PROBLEMS),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_clarify() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/clarify.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/clarify.md"),
        selects: CLAUDINE,
        schema: Invalid(&[("", "\"spec\" is a required property"), ("/doc", "is not a valid file reference")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_format() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/format.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/format.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_implement() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/implement.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/implement.md"),
        selects: CLAUDINE,
        schema: Invalid(&[("", "\"spec\" is a required property")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_plan() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/plan.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/plan.md"),
        selects: CLAUDINE,
        schema: Invalid(&[("", "\"spec\" is a required property"), ("/plan", "is not a valid file reference")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_review_loop() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/review-loop.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/review-loop.md"),
        selects: SEQUENCE,
        schema: Invalid(&[("", "\"spec\" is a required property")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_review() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/review.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/review.md"),
        selects: CLAUDINE,
        schema: Invalid(&[("", "\"spec\" is a required property")]),
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_headless_orchestration() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_headless-orchestration.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_headless-orchestration.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_input_robustness() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_input-robustness.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_input-robustness.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_no_formatting() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_no_formatting.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_no_formatting.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_os() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_os.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_os.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_set_spec_schema() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_set_spec_schema.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_set_spec_schema.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_test_tiers() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_test-tiers.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_test-tiers.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_testing_anti_patterns() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/_testing-anti-patterns.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/_testing-anti-patterns.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn frozen_cli_make_it_markdown() {
    assert_shipped(&Shipped {
        label: "cli/tests/fixtures/frozen_prompts/prompts/make-it-markdown.md",
        text: include_str!("../../../cli/tests/fixtures/frozen_prompts/prompts/make-it-markdown.md"),
        selects: NONE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

// ── Research fleets and row prompts ────────────────────────────────────────
// A fleet coordinator references its roster and names its row prompt; the
// ones that also set `agent` and `model` select Claudine's trigger too. A row
// prompt carries lifecycle stacks, agent, and model, except
// `model-serving-api-standards`, whose only control is an inline `sequence`.

#[test]
fn research_acp_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/acp/_fleet.md",
        text: include_str!("../../../docs/research/acp/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_acp_research() {
    assert_shipped(&Shipped {
        label: "docs/research/acp/_research.md",
        text: include_str!("../../../docs/research/acp/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_cli_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-cli/_fleet.md",
        text: include_str!("../../../docs/research/agent-cli/_fleet.md"),
        selects: SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_cli_research() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-cli/_research.md",
        text: include_str!("../../../docs/research/agent-cli/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_errors_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-errors/_fleet.md",
        text: include_str!("../../../docs/research/agent-errors/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_errors_research() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-errors/_research.md",
        text: include_str!("../../../docs/research/agent-errors/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_logging_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-logging/_fleet.md",
        text: include_str!("../../../docs/research/agent-logging/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_logging_research() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-logging/_research.md",
        text: include_str!("../../../docs/research/agent-logging/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_models_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-models/_fleet.md",
        text: include_str!("../../../docs/research/agent-models/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_models_research() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-models/_research.md",
        text: include_str!("../../../docs/research/agent-models/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_permissions_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-permissions/_fleet.md",
        text: include_str!("../../../docs/research/agent-permissions/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_agent_permissions_research() {
    assert_shipped(&Shipped {
        label: "docs/research/agent-permissions/_research.md",
        text: include_str!("../../../docs/research/agent-permissions/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_hooks_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/hooks/_fleet.md",
        text: include_str!("../../../docs/research/hooks/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_hooks_research() {
    assert_shipped(&Shipped {
        label: "docs/research/hooks/_research.md",
        text: include_str!("../../../docs/research/hooks/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_local_runners_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/local_runners/_fleet.md",
        text: include_str!("../../../docs/research/local_runners/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_local_runners_research() {
    assert_shipped(&Shipped {
        label: "docs/research/local_runners/_research.md",
        text: include_str!("../../../docs/research/local_runners/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_mcp_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/mcp/_fleet.md",
        text: include_str!("../../../docs/research/mcp/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_mcp_research() {
    assert_shipped(&Shipped {
        label: "docs/research/mcp/_research.md",
        text: include_str!("../../../docs/research/mcp/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_memory_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/memory/_fleet.md",
        text: include_str!("../../../docs/research/memory/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_memory_research() {
    assert_shipped(&Shipped {
        label: "docs/research/memory/_research.md",
        text: include_str!("../../../docs/research/memory/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_model_config_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/model-config/_fleet.md",
        text: include_str!("../../../docs/research/model-config/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_model_config_research() {
    assert_shipped(&Shipped {
        label: "docs/research/model-config/_research.md",
        text: include_str!("../../../docs/research/model-config/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_model_serving_api_standards_research() {
    assert_shipped(&Shipped {
        label: "docs/research/model-serving-api-standards/_research.md",
        text: include_str!("../../../docs/research/model-serving-api-standards/_research.md"),
        selects: SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_non_interactive_sessions_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/non-interactive-sessions/_fleet.md",
        text: include_str!("../../../docs/research/non-interactive-sessions/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_non_interactive_sessions_research() {
    assert_shipped(&Shipped {
        label: "docs/research/non-interactive-sessions/_research.md",
        text: include_str!("../../../docs/research/non-interactive-sessions/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_plugins_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/plugins/_fleet.md",
        text: include_str!("../../../docs/research/plugins/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_plugins_research() {
    assert_shipped(&Shipped {
        label: "docs/research/plugins/_research.md",
        text: include_str!("../../../docs/research/plugins/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_reasoning_level_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/reasoning-level/_fleet.md",
        text: include_str!("../../../docs/research/reasoning-level/_fleet.md"),
        selects: SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_reasoning_level_research() {
    assert_shipped(&Shipped {
        label: "docs/research/reasoning-level/_research.md",
        text: include_str!("../../../docs/research/reasoning-level/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_resume_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/resume/_fleet.md",
        text: include_str!("../../../docs/research/resume/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_resume_research() {
    assert_shipped(&Shipped {
        label: "docs/research/resume/_research.md",
        text: include_str!("../../../docs/research/resume/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_signals_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/signals/_fleet.md",
        text: include_str!("../../../docs/research/signals/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_signals_research() {
    assert_shipped(&Shipped {
        label: "docs/research/signals/_research.md",
        text: include_str!("../../../docs/research/signals/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_skills_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/skills/_fleet.md",
        text: include_str!("../../../docs/research/skills/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_skills_research() {
    assert_shipped(&Shipped {
        label: "docs/research/skills/_research.md",
        text: include_str!("../../../docs/research/skills/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_slash_commands_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/slash-commands/_fleet.md",
        text: include_str!("../../../docs/research/slash-commands/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_slash_commands_research() {
    assert_shipped(&Shipped {
        label: "docs/research/slash-commands/_research.md",
        text: include_str!("../../../docs/research/slash-commands/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_steering_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/steering/_fleet.md",
        text: include_str!("../../../docs/research/steering/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_steering_research() {
    assert_shipped(&Shipped {
        label: "docs/research/steering/_research.md",
        text: include_str!("../../../docs/research/steering/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_subagents_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/subagents/_fleet.md",
        text: include_str!("../../../docs/research/subagents/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_subagents_research() {
    assert_shipped(&Shipped {
        label: "docs/research/subagents/_research.md",
        text: include_str!("../../../docs/research/subagents/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_system_prompt_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/system-prompt/_fleet.md",
        text: include_str!("../../../docs/research/system-prompt/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_system_prompt_research() {
    assert_shipped(&Shipped {
        label: "docs/research/system-prompt/_research.md",
        text: include_str!("../../../docs/research/system-prompt/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_usage_fleet() {
    assert_shipped(&Shipped {
        label: "docs/research/usage/_fleet.md",
        text: include_str!("../../../docs/research/usage/_fleet.md"),
        selects: CLAUDINE_AND_SEQUENCE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

#[test]
fn research_usage_research() {
    assert_shipped(&Shipped {
        label: "docs/research/usage/_research.md",
        text: include_str!("../../../docs/research/usage/_research.md"),
        selects: CLAUDINE,
        schema: Valid,
        parser: ACCEPTS,
    });
}

