//! Preflight work that must not grow with the number of other steps.
//!
//! The counted cases (`work-counters`) build one fixture shape at two or three
//! sizes and compare per-step work, while also asserting the graph the walk
//! produced, so a walk that skipped analysis cannot pass. The uncounted cases
//! pin the semantics the shared step view and the bounded scans must keep.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::providers::{HintState, LaunchDecision};
use super::*;
use crate::composition::sequence::resolve_sequence_plan;

/// Write `seq.md` with `extra` frontmatter lines above `sequence: steps`.
fn sequence_document(dir: &Path, extra: &str, steps: &[Value]) -> String {
    let path = dir.join("seq.md");
    fs::write(
        &path,
        format!("---\n{extra}sequence: {}\n---\n\nBody.\n", serde_json::to_string(steps).unwrap()),
    )
    .unwrap();
    path.display().to_string()
}

fn graph(path: &str) -> PreflightGraph {
    let source = crate::composition::resolve_fixture_source(path).unwrap();
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&source, &resolution).unwrap().expect("a sequence");
    build_preflight_graph(&plan, &source, &resolution).unwrap()
}

fn task(graph: &PreflightGraph, step: usize) -> &PreflightTask {
    graph.steps[step].task.as_ref().expect("the step runs a task")
}

/// A6's shape: `steps` shell steps with ten holes each, every hole reading a
/// typed document value.
fn ten_hole_steps(dir: &Path, steps: usize) -> String {
    let holes: String = (0..10).map(|hole| format!(" {{{{ arg_{hole} }}}}")).collect();
    let schema: String = (0..10).map(|hole| format!("    arg_{hole}: string\n")).collect();
    let values: String = (0..10).map(|hole| format!("arg_{hole}: value-{hole}\n")).collect();
    let steps: Vec<Value> = (0..steps)
        .map(|index| json!({ "name": format!("step-{index}"), "shell": format!("echo step-{index}{holes}") }))
        .collect();
    sequence_document(dir, &format!("$schema:\n{schema}{values}"), &steps)
}

/// The first step's command runs as resolved bytes; every step's command is
/// a ten-hole template candidate.
#[cfg(feature = "work-counters")]
fn assert_ten_hole_graph(graph: &PreflightGraph, steps: usize) {
    assert_eq!(graph.steps.len(), steps);
    let values: String = (0..10).map(|hole| format!(" value-{hole}")).collect();
    assert_eq!(graph.shell_commands.len(), 1, "{:?}", graph.shell_commands);
    assert_eq!(graph.shell_commands[0].command, format!("echo step-0{values}"));
    assert_eq!(graph.template_candidates.len(), steps);
    for candidate in &graph.template_candidates {
        let template = candidate.template.as_ref().expect("a valid template");
        assert_eq!(template.placeholders().count(), 10, "{}", candidate.authored_command);
    }
}

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use crate::work_counters::{Work, WorkSnapshot, snapshot};

    /// The Claudine and Darkmatter work one preflight of `path` does.
    fn work(path: &str) -> (PreflightGraph, WorkSnapshot, darkmatter::work_counters::WorkSnapshot) {
        let claudine = snapshot();
        let darkmatter = darkmatter::work_counters::snapshot();
        let graph = graph(path);
        (graph, snapshot().since(&claudine), darkmatter::work_counters::snapshot().since(&darkmatter))
    }

    /// The spec's shell criterion: the same `ExpressionParse` count per step
    /// at 10 and 100 steps with ten holes each.
    #[test]
    fn shell_preflight_parses_the_same_expressions_per_step_at_10_and_100_steps() {
        let mut per_step = Vec::new();
        for steps in [10, 100] {
            let dir = TempDir::new().unwrap();
            let (graph, _, darkmatter) = work(&ten_hole_steps(dir.path(), steps));
            assert_ten_hole_graph(&graph, steps);
            let parses = darkmatter.get(darkmatter::work_counters::Work::ExpressionParse);
            assert_eq!(parses % steps as u64, 0, "{parses} parses for {steps} steps");
            per_step.push(parses / steps as u64);
        }
        assert_eq!(per_step[0], per_step[1], "parses per step at 10 and 100 steps");
    }

    /// Per-step copies and scans of the step frontmatter: a count linear in
    /// the step count has equal increments over equal step increments, which a
    /// count that revisits the other steps' definitions does not.
    #[test]
    fn step_frontmatter_copies_scans_and_states_grow_linearly() {
        let kinds = [Work::PreflightStepFrontmatterNodes, Work::PreflightDeferredScanNodes, Work::EarlyBindingStateNodes];
        let mut counts = Vec::new();
        for steps in [10, 55, 100] {
            let dir = TempDir::new().unwrap();
            let (graph, claudine, _) = work(&ten_hole_steps(dir.path(), steps));
            assert_ten_hole_graph(&graph, steps);
            counts.push(kinds.map(|kind| claudine.get(kind)));
        }
        for (index, kind) in kinds.iter().enumerate() {
            let (small, middle, large) = (counts[0][index], counts[1][index], counts[2][index]);
            assert!(small > 0, "{kind:?} counted nothing");
            assert_eq!(middle - small, large - middle, "{kind:?}: {small} → {middle} → {large}");
        }
    }

    /// A `with` hint that reads an expression copies the hint and the keys it
    /// reads, never the step's other values; a literal one copies nothing.
    #[test]
    fn a_with_hint_copies_only_what_it_reads_at_any_sequence_size() {
        let mut per_task = Vec::new();
        for steps in [10, 100] {
            let dir = TempDir::new().unwrap();
            fs::write(dir.path().join("prompt.md"), "---\nmodel: m\n---\n\nDo the work.\n").unwrap();
            let prompts: Vec<Value> = (0..steps)
                .map(|index| json!({ "name": format!("task-{index}"), "prompt": "prompt.md", "with": { "agent": "{{ chosen }}" } }))
                .collect();
            let (graph, claudine, _) = work(&sequence_document(dir.path(), "chosen: codex\n", &prompts));
            for step in 0..steps {
                let prediction = task(&graph, step).provider.as_ref().expect("a prompt task predicts");
                assert!(
                    matches!(&prediction.agent, HintState::Static { value, .. } if value == "codex"),
                    "step {step}: {:?}", prediction.agent,
                );
            }
            let copied = claudine.get(Work::PredictionWithStateNodes);
            assert_eq!(copied % steps as u64, 0, "{copied} nodes for {steps} tasks");
            per_task.push(copied / steps as u64);

            let literal: Vec<Value> = (0..steps)
                .map(|index| json!({ "name": format!("task-{index}"), "prompt": "prompt.md", "with": { "agent": "codex" } }))
                .collect();
            let (graph, claudine, _) = work(&sequence_document(dir.path(), "", &literal));
            assert!(matches!(&task(&graph, steps - 1).provider.as_ref().unwrap().agent, HintState::Static { value, .. } if value == "codex"));
            assert_eq!(claudine.get(Work::PredictionWithStateNodes), 0);
        }
        // The hint and the one key it reads.
        assert_eq!(per_task, [2, 2]);
    }

    /// A container event's plain `set` command is fixed without materializing
    /// the step state.
    #[test]
    fn a_plain_shell_set_value_never_materializes_the_step_state() {
        let dir = TempDir::new().unwrap();
        let steps: Vec<Value> = (0..20)
            .map(|index| json!({ "name": format!("step-{index}"), "sequence": [], "success": [{ "set": { "v": format!("$(echo {index})") } }] }))
            .collect();
        let (graph, claudine, _) = work(&sequence_document(dir.path(), "", &steps));
        assert_eq!(claudine.get(Work::ShellValueStateNodes), 0);
        let commands: Vec<&str> = graph.shell_commands.iter().map(|command| command.command.as_str()).collect();
        assert_eq!(commands, (0..20).map(|index| format!("echo {index}")).collect::<Vec<_>>());
    }

    /// `steps` steps whose container `success` event assigns `value`: a
    /// nested sequence's own event, or the event of a group of one shell leg.
    fn assignment_steps(dir: &Path, steps: usize, group: bool, value: &str) -> String {
        let success = json!([{ "set": { "result": value } }]);
        let steps: Vec<Value> = (0..steps)
            .map(|index| match group {
                false => json!({ "name": format!("step-{index}"), "sequence": [], "success": success }),
                true => json!({ "name": format!("step-{index}"), "group": { "name": format!("g{index}"),
                    "tasks": [{ "name": "leg", "shell": format!("echo leg-{index}") }], "success": success } }),
            })
            .collect();
        sequence_document(dir, "flag: true\n", &steps)
    }

    /// The step state a conditional assignment's branches read is the step
    /// view, never a copy of the other steps' definitions: the nodes copied
    /// grow linearly with the step count (each step copies only its own `seq`
    /// position, which is a few nodes smaller at the first and last step), and
    /// both branches of every step stay in the approval inventory.
    #[test]
    fn a_conditional_shell_set_value_copies_bounded_state_per_step() {
        for group in [false, true] {
            let mut copied = Vec::new();
            for steps in [10, 55, 100] {
                let dir = TempDir::new().unwrap();
                let path = assignment_steps(dir.path(), steps, group, "$( flag ? echo yes : echo no )");
                let (graph, claudine, _) = work(&path);
                assert_eq!(graph.steps.len(), steps);
                let commands: Vec<&str> = graph.shell_commands.iter().map(|command| command.command.as_str()).collect();
                let assignments: Vec<&str> = commands.iter().copied().filter(|command| !command.starts_with("echo leg-")).collect();
                assert_eq!(assignments, ["echo yes", "echo no"].repeat(steps), "group={group}");
                if group {
                    assert_eq!(commands.len() - assignments.len(), steps, "every leg's command is recorded");
                }
                copied.push(claudine.get(Work::ShellValueStateNodes));
            }
            let [small, middle, large] = copied[..] else { unreachable!() };
            assert!(small > 0, "group={group}: the ternary's branches read no state");
            assert_eq!(middle - small, large - middle, "group={group}: {small} → {middle} → {large}");
        }
    }

    /// The plain controls of the conditional case: a fixed command is never
    /// given the step state at all.
    #[test]
    fn a_plain_shell_set_value_copies_no_state_in_a_sequence_or_group_event() {
        for group in [false, true] {
            for steps in [10, 100] {
                let dir = TempDir::new().unwrap();
                let (graph, claudine, _) = work(&assignment_steps(dir.path(), steps, group, "$(echo yes)"));
                assert_eq!(graph.steps.len(), steps);
                let assignments = graph.shell_commands.iter().filter(|command| command.command == "echo yes").count();
                assert_eq!(assignments, steps, "group={group}");
                assert_eq!(claudine.get(Work::ShellValueStateNodes), 0, "group={group}, {steps} steps");
            }
        }
    }

    /// Distinct prompt references are never compared with one another; a
    /// repeated reference at the same position is still recorded once.
    #[test]
    fn prompt_references_are_compared_only_with_possible_duplicates() {
        let dir = TempDir::new().unwrap();
        for index in 0..30 {
            fs::write(dir.path().join(format!("p{index}.md")), "---\nagent: codex\n---\n\nWork.\n").unwrap();
        }
        let mut steps: Vec<Value> = (0..30)
            .map(|index| json!({ "name": format!("task-{index}"), "prompt": format!("p{index}.md") }))
            .collect();
        steps.push(json!({ "name": "again", "prompt": "p29.md" }));
        let (graph, claudine, _) = work(&sequence_document(dir.path(), "", &steps));
        assert_eq!(graph.prompt_references.len(), 30, "the repeat of p29's `Later` reference is recorded once");
        // One comparison: the repeat with the earlier `Later` reference of p29.
        assert_eq!(claudine.get(Work::PromptReferenceCompare), 1);

        let mut repeated = steps.clone();
        repeated.push(json!({ "name": "third", "prompt": "p29.md" }));
        let (graph, _, _) = work(&sequence_document(dir.path(), "", &repeated));
        assert_eq!(graph.prompt_references.len(), 30, "every identical later reference is recorded once");
    }

    /// Each group checks its owner document for retired names once per walk,
    /// so a sequence of groups parses the same expressions per step at any
    /// size.
    #[test]
    fn groups_parse_the_same_expressions_per_step_at_10_and_100_steps() {
        let mut per_step = Vec::new();
        for steps in [10, 100] {
            let dir = TempDir::new().unwrap();
            let groups: Vec<Value> = (0..steps)
                .map(|index| json!({ "name": format!("step-{index}"), "group": { "name": format!("g{index}"), "tasks": [
                    { "name": "leg", "shell": format!("echo {index} {{{{ arg }}}}") }
                ] } }))
                .collect();
            let (graph, _, darkmatter) = work(&sequence_document(dir.path(), "arg: value\n", &groups));
            assert_eq!(graph.steps.len(), steps);
            assert_eq!(graph.shell_commands[0].command, "echo 0 value");
            let parses = darkmatter.get(darkmatter::work_counters::Work::ExpressionParse);
            assert_eq!(parses % steps as u64, 0, "{parses} parses for {steps} steps");
            per_step.push(parses / steps as u64);
        }
        assert_eq!(per_step[0], per_step[1], "parses per step at 10 and 100 steps");
    }
}

/// Every root step's approval-time state shares one seed: the steps' views
/// and the approvals that keep them own only their `seq` overlay.
#[test]
fn every_root_step_state_shares_the_preparation_seed() {
    let dir = TempDir::new().unwrap();
    let path = ten_hole_steps(dir.path(), 5);
    let source = crate::composition::resolve_fixture_source(&path).unwrap();
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_content(dir.path(), "");
    let mut loader = Loader::new(&source, context, resolution);
    loader.walk_plan(&plan).unwrap();
    let seed = loader.step_seed.clone();
    assert!(seed.get("sequence").and_then(Value::as_array).is_some_and(|steps| steps.len() == 5));
    for index in 0..5 {
        loader.enter_step(&plan, index);
        let view = &loader.step_frontmatter;
        assert!(view.shares_base_with(&seed), "step {index}");
        assert_eq!(view.owned().map(|(key, _)| key.as_str()).collect::<Vec<_>>(), ["seq"], "step {index}");
        assert_eq!(view.get("seq").and_then(|seq| seq.get("index")), Some(&json!(index + 1)));
        // The authored definitions stay readable through the view.
        assert_eq!(view.get("sequence"), seed.get("sequence"));
        assert_eq!(view.get("arg_3"), Some(&json!("value-3")));
    }
}

/// A referenced sequence's operator reads the enclosing step's values
/// through the shared view.
#[test]
fn a_referenced_sequence_operator_reads_the_enclosing_document_values() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("items.yaml"), "data:\n  - color: blue\n    shell: echo blue\n  - color: green\n    shell: echo green\n").unwrap();
    let path = sequence_document(dir.path(), "prefix: say\n", &[
        json!({ "name": "nested", "sequence": "items.yaml -> data::template(prefix + '-' + color)" }),
    ]);
    let graph = graph(&path);
    let PreflightAction::Sequence(nested) = &task(&graph, 0).action else { panic!("a nested sequence") };
    let names: Vec<&str> = nested.plan.steps.iter().map(|step| step.name.as_str()).collect();
    assert_eq!(names, ["say-blue", "say-green"]);
}

/// A `with` hint that reaches a late root, a shell substitution, or the
/// provider's own identity through a document key keeps that class: the
/// bounded classification follows reads transitively.
#[test]
fn a_with_hint_keeps_its_class_through_document_keys() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("prompt.md"), "---\nmodel: m\n---\n\nWork.\n").unwrap();
    let cases = [
        ("pick: \"{{ hop }}\"\nhop: \"{{ seq.index }}\"\n", "late"),
        ("pick: \"{{ hop }}\"\nhop: \"$(echo codex)\"\n", "shell"),
        ("pick: \"{{ hop }}\"\nhop: \"{{ env.AGENT }}\"\n", "circular"),
        ("pick: codex\nhop: \"{{ seq.index }}\"\n", "static"),
    ];
    for (extra, class) in cases {
        let path = sequence_document(dir.path(), &format!("{extra}unrelated: \"{{{{ seq.index }}}}\"\n"), &[
            json!({ "name": "first", "shell": "echo first" }),
            json!({ "name": "run", "prompt": "prompt.md", "with": { "agent": "{{ pick }}" } }),
        ]);
        let graph = graph(&path);
        let agent = &task(&graph, 1).provider.as_ref().unwrap().agent;
        let matched = match class {
            "late" => matches!(agent, HintState::DecidedAtLaunch { reason: LaunchDecision::LateValue, .. }),
            "shell" => matches!(agent, HintState::DecidedAtLaunch { reason: LaunchDecision::ShellSubstitution, .. }),
            "circular" => matches!(agent, HintState::Circular { variable, .. } if variable == "env.AGENT"),
            _ => matches!(agent, HintState::Static { value, .. } if value == "codex"),
        };
        assert!(matched, "{class}: {agent:?}");
    }
}

/// [`overlay_derived`] agrees with a scan of the whole view, including when
/// the overlay changes what the seed derives.
#[test]
fn overlay_late_keys_match_a_whole_view_scan() {
    let object = |value: Value| value.as_object().cloned().unwrap();
    let full = |view: &SharedFrontmatter| derived_deferred_keys(view.iter(), &sequence_deferred_roots(), &HashSet::new());
    let cases = [
        // No shadowing, and no late overlay key: the overlay alone is scanned.
        (json!({ "a": "{{ seq.index }}", "b": "{{ a }}", "c": "plain", "d": "{{ fm.extra }}" }), json!({ "extra": "literal" })),
        // An overlay key that reads a late seed key is late.
        (json!({ "a": "{{ loop.count }}" }), json!({ "extra": "{{ a }}" })),
        // A late overlay key makes the seed key that reads it late.
        (json!({ "reads": "{{ fm.extra }}", "plain": 1 }), json!({ "extra": "{{ group.name }}" })),
        // A literal overlay value shadows a late seed key, and its readers.
        (json!({ "a": "{{ seq.index }}", "b": "{{ a }}" }), json!({ "a": "fixed" })),
    ];
    for (seed, overlay) in cases {
        let seed = SharedFrontmatter::from(object(seed));
        let overlay = object(overlay);
        let seed_derived = full(&seed);
        let mut view = seed.clone();
        view.extend(overlay.clone());
        assert_eq!(overlay_derived(&seed, &seed_derived, &overlay, &view), full(&view), "{overlay:?}");
    }
}
