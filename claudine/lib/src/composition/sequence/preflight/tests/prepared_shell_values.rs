//! A shell command's early binding reads the *prepared* values of the
//! document that declares its row: a schema default fills an absent or
//! `null` value and a value is converted as its `$schema` declares, as in the
//! composed state the command runs in. The declaring document is the owner
//! the row is read inside (the invoked document, or a whole referenced
//! document, however it is reached); another document's values never bind
//! it. Every route must agree with running the declaring document directly.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use tempfile::TempDir;

use super::*;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, text).unwrap();
    biscuit_file::canonicalize_simplified(&path).unwrap()
}

fn load(path: &Path) -> ResolvedCompositionSource {
    ResolvedCompositionSource {
        original_ref: path.display().to_string(),
        resolved_path: path.to_path_buf(),
        original_text: fs::read_to_string(path).unwrap(),
        markdown: crate::composition::resolve::load_yaml_document(path).unwrap(),
    }
}

/// How the declaring document `child` (a whole `kind: sequence` document) is
/// reached.
#[derive(Clone, Copy, Debug)]
enum Route {
    /// Run directly: the control every other route must match.
    Direct,
    /// Through the root's own `sequence: sub/child.yaml`.
    RootReference,
    /// As a nested task of the root.
    Nested,
    /// As a leg of a group.
    GroupLeg,
}

const ROUTES: [Route; 4] = [Route::Direct, Route::RootReference, Route::Nested, Route::GroupLeg];

impl Route {
    /// Write the files for `child` with the invoking document's extra root
    /// keys `root`, and return the invoked document.
    fn write(self, dir: &Path, child: &str, root: &str) -> PathBuf {
        match self {
            Self::Direct => write(dir, "sub/child.yaml", child),
            Self::RootReference => {
                write(dir, "sub/child.yaml", child);
                write(dir, "root.yaml", &format!("kind: sequence\n{root}sequence: ./sub/child.yaml\n"))
            }
            Self::Nested => {
                write(dir, "sub/child.yaml", child);
                write(dir, "root.yaml", &format!("kind: sequence\n{root}sequence:\n  - name: outer\n    sequence: ./sub/child.yaml\n"))
            }
            Self::GroupLeg => {
                write(dir, "sub/child.yaml", child);
                write(dir, "root.yaml", &format!(
                    "kind: sequence\n{root}sequence:\n  - name: fan\n    group:\n      name: G\n      tasks:\n        - name: leg\n          sequence: ./sub/child.yaml\n"
                ))
            }
        }
    }
}

/// How the graph is built.
#[derive(Clone, Copy, Debug)]
enum Build {
    /// As the CLI does: input discovery, then the graph reusing its owners.
    AfterDiscovery,
    /// Through an invocation, with no discovery: owners prepared on use.
    WithInvocation,
    /// With neither discovery nor an invocation.
    Plain,
}

const BUILDS: [Build; 3] = [Build::AfterDiscovery, Build::WithInvocation, Build::Plain];

fn graph(path: &Path, dir: &Path, build: Build) -> Result<PreflightGraph, CompositionError> {
    let source = load(path);
    let invocation =
        crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let context = ComposeContext::capture_for_content(dir, "");
    let resolution = source_context.file_resolution_context().clone();
    let plan = crate::composition::sequence::resolve_sequence_plan_with(
        &source,
        crate::composition::SequenceSourceOptions::new(&resolution)
            .with_compose_context(&context)
            .with_invocation(&invocation),
    )?
    .expect("a sequence");
    match build {
        Build::AfterDiscovery => {
            let inventory = discover_sequence_inputs(
                &source,
                InputDiscoveryOptions::new(&resolution, &context).with_invocation(&invocation, &source_context),
            )?;
            build_preflight_graph_after_discovery(&plan, &source, context, &invocation, &source_context, None, &inventory)
        }
        Build::WithInvocation => build_preflight_graph_with_invocation(&plan, &source, context, &invocation, &source_context, None),
        Build::Plain => build_preflight_graph_with_context(&plan, &source, context, &resolution),
    }
}

/// The exact commands `child` approves, or the walk's error, on every route
/// and build, each tagged with its case.
fn every_case(child: &str, root: &str) -> Vec<(String, Result<Vec<String>, String>)> {
    let mut cases = Vec::new();
    for route in ROUTES {
        for build in BUILDS {
            let dir = TempDir::new().unwrap();
            let path = route.write(dir.path(), child, root);
            let result = graph(&path, dir.path(), build)
                .map(|graph| graph.shell_commands.iter().map(|command| command.command.clone()).collect())
                .map_err(|error| error.to_string());
            cases.push((format!("{route:?} / {build:?}"), result));
        }
    }
    cases
}

fn assert_every_case_approves(child: &str, root: &str, expected: &str) {
    let failures: Vec<String> = every_case(child, root).into_iter()
        .filter(|(_, result)| !result.as_ref().is_ok_and(|commands| commands.iter().any(|command| command == expected)))
        .map(|(case, result)| format!("{case}: {result:?}"))
        .collect();
    assert!(failures.is_empty(), "{} case(s) did not approve `{expected}`:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn a_shell_row_reads_a_schema_default_for_an_absent_value_on_every_route() {
    let child = "kind: sequence\n$schema:\n  who: string(default(ada))\nsequence:\n  - name: show\n    shell: \"echo {{ who }}\"\n";
    assert_every_case_approves(child, "", "echo ada");
}

#[test]
fn a_shell_row_reads_a_schema_default_for_a_null_value_on_every_route() {
    let child = "kind: sequence\n$schema:\n  who: string(default(ada))\nwho: null\nsequence:\n  - name: show\n    shell: \"echo {{ who }}\"\n";
    assert_every_case_approves(child, "", "echo ada");
}

#[test]
fn a_shell_row_reads_an_authored_value_on_every_route() {
    let child = "kind: sequence\nwho: ada\nsequence:\n  - name: show\n    shell: \"echo {{ who }}\"\n";
    assert_every_case_approves(child, "", "echo ada");
}

/// The invoking document's value for a key never binds a referenced
/// document's row, whichever reaches it: the row reads its own document's.
#[test]
fn a_referenced_row_reads_its_own_documents_value_over_the_invoking_documents() {
    let child = "kind: sequence\nrank: 1\nsequence:\n  - name: show\n    shell: \"echo {{ rank }}\"\n";
    assert_every_case_approves(child, "rank: 9\n", "echo 1");
}

/// A key only the invoking document defines is missing for a referenced
/// row, as it is when the referenced document runs directly.
#[test]
fn a_value_only_the_invoking_document_defines_is_missing_for_a_referenced_row() {
    let child = "kind: sequence\nsequence:\n  - name: show\n    shell: \"echo {{ only }}\"\n";
    let failures: Vec<String> = every_case(child, "only: leaked\n").into_iter()
        .filter(|(_, result)| !result.as_ref().is_err_and(|message| message.contains("`only`") && message.contains("missing")))
        .map(|(case, result)| format!("{case}: {result:?}"))
        .collect();
    assert!(failures.is_empty(), "{} case(s) bound `only`:\n{}", failures.len(), failures.join("\n"));
}

/// A caller setter is above the default.
#[test]
fn a_setter_is_above_the_default() {
    let dir = TempDir::new().unwrap();
    let path = Route::RootReference.write(
        dir.path(), "kind: sequence\n$schema:\n  who: string(default(ada))\nsequence:\n  - name: show\n    shell: \"echo {{ who }}\"\n", "",
    );
    let source = load(&path);
    let invocation =
        crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let context = ComposeContext::capture_for_content(dir.path(), "");
    let resolution = source_context.file_resolution_context().clone();
    let plan = crate::composition::sequence::resolve_sequence_plan_with(
        &source, crate::composition::SequenceSourceOptions::new(&resolution).with_compose_context(&context).with_invocation(&invocation),
    ).unwrap().unwrap();
    let graph = build_preflight_graph_with_invocation(
        &plan, &source, context, &invocation, &source_context, Some(&json!({ "who": "grace" })),
    ).unwrap();
    let commands: Vec<&str> = graph.shell_commands.iter().map(|command| command.command.as_str()).collect();
    assert_eq!(commands, ["echo grace"]);
}

/// A value derived from another document value binds as the document
/// prepares it, not as its authored expression text.
#[test]
fn a_shell_row_reads_a_derived_value_as_prepared_on_every_route() {
    let child = "kind: sequence\nname: ada\ngreeting: \"hi {{ name }}\"\nsequence:\n  - name: show\n    shell: \"echo {{ greeting }}\"\n";
    assert_every_case_approves(child, "", "echo hi ada");
}

/// One case's approved exact commands and template authored texts.
type SplitCommands = Result<(Vec<String>, Vec<String>), String>;

/// Every route's approved exact commands and template authored texts.
fn every_case_split(child: &str, root: &str) -> Vec<(String, SplitCommands)> {
    let mut cases = Vec::new();
    for route in ROUTES {
        for build in BUILDS {
            let dir = TempDir::new().unwrap();
            let path = route.write(dir.path(), child, root);
            let result = graph(&path, dir.path(), build)
                .map(|graph| (
                    graph.shell_commands.iter().map(|command| command.command.clone()).collect(),
                    graph.template_candidates.iter().map(|candidate| candidate.authored_command.clone()).collect(),
                ))
                .map_err(|error| error.to_string());
            cases.push((format!("{route:?} / {build:?}"), result));
        }
    }
    cases
}

/// A derived value whose launch-time value may not be the one the command
/// runs with is never frozen into exact bytes: the command is approved as
/// its template and filled when it runs.
fn assert_every_case_stays_late(child: &str, root: &str) {
    let failures: Vec<String> = every_case_split(child, root).into_iter()
        .filter(|(_, result)| !result.as_ref().is_ok_and(|(exact, templates)| {
            !exact.iter().any(|command| command.starts_with("echo hi"))
                && templates.iter().any(|command| command == "echo {{ greeting }}")
        }))
        .map(|(case, result)| format!("{case}: {result:?}"))
        .collect();
    assert!(failures.is_empty(), "{} case(s) bound `greeting` at launch:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn a_derived_value_reading_a_late_root_stays_late_on_every_route() {
    let child = "kind: sequence\ngreeting: \"hi {{ seq.name }}\"\nsequence:\n  - name: show\n    shell: \"echo {{ greeting }}\"\n";
    assert_every_case_stays_late(child, "");
}

#[test]
fn a_derived_value_reading_a_runtime_written_key_stays_late_on_every_route() {
    let child = "kind: sequence\nname: ada\ngreeting: \"hi {{ name }}\"\nstart:\n  - set:\n      name: bob\nsequence:\n  - name: show\n    shell: \"echo {{ greeting }}\"\n";
    assert_every_case_stays_late(child, "");
}

#[test]
fn a_derived_value_reading_the_environment_stays_late_on_every_route() {
    let child = "kind: sequence\ngreeting: \"hi {{ env.HOME }}\"\nsequence:\n  - name: show\n    shell: \"echo {{ greeting }}\"\n";
    assert_every_case_stays_late(child, "");
}

/// A literal value is unaffected by the derived-value rule.
#[test]
fn a_literal_value_still_binds_exactly_on_every_route() {
    let child = "kind: sequence\ngreeting: hi-ada\nsequence:\n  - name: show\n    shell: \"echo {{ greeting }}\"\n";
    assert_every_case_approves(child, "", "echo hi-ada");
}

/// A value derived from a `$( … )` value is never bound to its launch-time
/// preparation: no exact command carries the substitution's text.
#[test]
fn a_derived_value_reading_a_shell_substitution_is_not_bound_on_every_route() {
    let child = "kind: sequence\nwho: \"$(echo ada)\"\ngreeting: \"hi {{ who }}\"\nsequence:\n  - name: show\n    shell: \"echo {{ greeting }}\"\n";
    let failures: Vec<String> = every_case_split(child, "").into_iter()
        .filter(|(_, result)| !result.as_ref().is_ok_and(|(exact, _)| !exact.iter().any(|command| command.starts_with("echo hi "))))
        .map(|(case, result)| format!("{case}: {result:?}"))
        .collect();
    assert!(failures.is_empty(), "{} case(s) bound `greeting` at launch:\n{}", failures.len(), failures.join("\n"));
}
