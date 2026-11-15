//! The lifecycle mutation boundary lasts for the whole run, while the base of
//! a relative target follows the active document.
//!
//! The caller's root (the git working tree the run is launched from) is
//! captured once. Switching documents with `prep` or `proxy`, a retried or
//! resumed attempt, loop passes, nested and referenced sequences and groups,
//! and parallel group members all keep that boundary; each relative target is
//! created beside the document whose action wrote it.
//!
//! Each document in these fixtures probes both halves: a relative
//! `marker.log` (the base), and an absolute target outside the caller's root
//! with `no_error` (the boundary is not widened) or a `../` target inside it
//! (the boundary is not narrowed to the document's directory). Where the
//! assertion is the refusal itself, the run fails and its message is read.

use std::fs;
use std::path::Path;
use std::process::ExitStatus;

use crate::common;
use common::mutation_paths::{
    CallerTree, assert_no_refusal, assert_nothing_under, assert_only_beside, assert_refused_outside_root,
    assert_written_beside, yaml_path,
};
use common::strip_ansi;

const MARKER: &str = "marker.log";

/// A Claude stub that reports one successful result with a session id (so
/// `resume` has a session to continue) and exits 0.
fn install_claude(tree: &CallerTree) {
    #[cfg(unix)]
    common::write_executable(
        &tree.fixture.bin_dir().join("claude"),
        "#!/bin/sh\n\
         cat > /dev/null\n\
         printf '%s\\n' '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}'\n\
         printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}'\n\
         exit 0\n",
    );
    #[cfg(windows)]
    common::write(
        &tree.fixture.bin_dir().join("claude.cmd"),
        "@echo off\r\n\
         echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}\r\n\
         echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
         exit /b 0\r\n",
    );
}

/// Run `claudine <command> --claude --yolo <entry>`; returns the status and
/// the plain combined output (sequence routes report on both streams).
fn run(tree: &CallerTree, command: &str, entry: &Path) -> (ExitStatus, String) {
    let output = tree
        .fixture
        .command()
        .args([command, "--claude", "--yolo"])
        .arg(entry)
        .output()
        .expect("spawn claudine");
    let combined = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(!tree.fixture.audio_spool().exists());
    (output.status, strip_ansi(&combined))
}

fn assert_succeeded(status: ExitStatus, output: &str) {
    assert!(status.success(), "the run succeeds ({status:?}); output was:\n{output}");
    assert_no_refusal(output);
}

/// An action that tries an absolute target outside the caller's root and
/// tolerates the refusal, as a compact list item.
fn outside_probe(tree: &CallerTree, name: &str) -> String {
    format!("{{append_line: [{}, \"escaped\"], no_error: true}}", tree.outside_target(name))
}

// --- `prep` and `proxy` ---------------------------------------------------

/// A `prep` target in another directory writes beside itself; when it
/// returns, the caller's next write lands beside the caller again. The target
/// reaches the caller's root through `../` (the boundary is not its own
/// directory) and cannot write outside it (the boundary did not move).
#[test]
fn prep_switches_the_base_to_the_target_and_back_while_the_boundary_stays() {
    let tree = CallerTree::stage("mutation-lifetime-prep");
    install_claude(&tree);
    let caller = tree.write(
        tree.in_root("prompts/caller.md"),
        "---\n\
         start:\n  \
           - append_line: [marker.log, \"caller before\"]\n  \
           - prep: \"../tools/helper.md\"\n  \
           - append_line: [marker.log, \"caller after\"]\n\
         ---\n\
         Caller\n",
    );
    let helper = tree.write(
        tree.in_root("tools/helper.md"),
        &format!(
            "---\n\
             initialize:\n  \
               - append_line: [marker.log, \"helper initialize\"]\n\
             start:\n  \
               - append_line: [marker.log, \"helper start\"]\n  \
               - append_line: [../helper-reached-root.log, \"helper\"]\n  \
               - {}\n\
             ---\n\
             Helper\n",
            outside_probe(&tree, "helper.log"),
        ),
    );

    let (status, output) = run(&tree, "compose", &caller);

    assert_succeeded(status, &output);
    assert_written_beside(&caller, MARKER, &["caller before", "caller after"]);
    assert_written_beside(&helper, MARKER, &["helper initialize", "helper start"]);
    assert_only_beside(tree.root(), &[&caller, &helper], MARKER);
    assert!(tree.in_root("helper-reached-root.log").exists(), "`../` from the target stays inside the caller root");
    tree.assert_outside_untouched();
}

/// A chain of handoffs: the caller hands off to a document in another
/// directory, which writes beside itself and hands off to a document in a
/// foreign library. That last document may write an absolute target inside
/// the caller's root, but its relative target resolves beside it, outside the
/// root, and is refused before anything is created.
#[test]
fn proxy_hops_move_the_base_with_the_active_document_but_never_the_boundary() {
    let tree = CallerTree::stage("mutation-lifetime-proxy");
    install_claude(&tree);
    let landed = tree.in_root("foreign-landed.log");
    let foreign = tree.write(
        tree.library.join("prompts/foreign.md"),
        &format!(
            "---\n\
             start:\n  \
               - append_line: [{}, \"foreign\"]\n  \
               - append_line: [marker.log, \"foreign\"]\n\
             ---\n\
             Foreign\n",
            yaml_path(&landed),
        ),
    );
    let next = tree.write(
        tree.in_root("handoff/next.md"),
        &format!(
            "---\n\
             start:\n  \
               - append_line: [marker.log, \"next\"]\n  \
               - proxy: {}\n\
             ---\n\
             Next\n",
            yaml_path(&foreign),
        ),
    );
    let caller = tree.write(
        tree.in_root("prompts/caller.md"),
        "---\n\
         start:\n  \
           - append_line: [marker.log, \"caller\"]\n  \
           - proxy: \"../handoff/next.md\"\n\
         ---\n\
         Caller\n",
    );

    let (status, output) = run(&tree, "compose", &caller);

    assert!(!status.success(), "the refused write fails the run; output was:\n{output}");
    assert_refused_outside_root(&output);
    assert_written_beside(&caller, MARKER, &["caller"]);
    assert_written_beside(&next, MARKER, &["next"]);
    assert_only_beside(tree.root(), &[&caller, &next], MARKER);
    assert_eq!(fs::read_to_string(&landed).unwrap().trim(), "foreign", "an absolute in-root target is allowed");
    assert_nothing_under(&tree.library, MARKER);
}

// --- retry and resume -----------------------------------------------------

/// A document whose first attempt fails from `success` (while `recovered.txt`
/// is absent, which it then creates) and whose `recovery` stack starts the
/// next attempt. Every attempt records `success` beside the document, reaches
/// the caller's root through `../../`, and probes outside the root.
fn recovering(tree: &CallerTree, recovery: &str) -> String {
    format!(
        "---\n\
         success:\n    \
             - append_line: [marker.log, \"success\"]\n    \
             - append_line: [../../attempt-reached-root.log, \"attempt\"]\n    \
             - {}\n    \
             - when: \"!file_exists('recovered.txt')\"\n      \
               then:\n          \
                   - append_line: [recovered.txt, \"first\"]\n          \
                   - error: \"first attempt\"\n\
         {recovery}\
         ---\n\
         Body\n",
        outside_probe(tree, "attempt.log"),
    )
}

/// One `#[test]` per recovery directive: the recovered attempt writes beside
/// the same document, and the boundary is unchanged across attempts.
macro_rules! recovery_rows {
    ($($name:ident: $recovery:expr;)+) => {$(
        #[test]
        fn $name() {
            let tree = CallerTree::stage("mutation-lifetime-recovery");
            install_claude(&tree);
            let document = tree.write(tree.in_root("prompts/recover/doc.md"), &recovering(&tree, $recovery));

            let (status, output) = run(&tree, "compose", &document);

            assert_succeeded(status, &output);
            assert_written_beside(&document, MARKER, &["success", "success"]);
            assert_written_beside(&document, "recovered.txt", &["first"]);
            assert_only_beside(tree.root(), &[&document], MARKER);
            assert_only_beside(tree.root(), &[&document], "recovered.txt");
            let reached = fs::read_to_string(tree.in_root("attempt-reached-root.log")).unwrap();
            assert_eq!(reached.lines().count(), 2, "both attempts reach the caller root");
            tree.assert_outside_untouched();
        }
    )+};
}

recovery_rows! {
    a_retried_attempt_writes_beside_the_same_document_inside_the_same_boundary:
        "finalize:\n    - when: \"err\"\n      then: { retry: 1 }\n";
    a_resumed_attempt_writes_beside_the_same_document_inside_the_same_boundary:
        "failure:\n    - resume: { message: \"try again\", max_attempts: 1 }\n";
}

// --- loop progression -----------------------------------------------------

/// A looping document under `compose`: every pass's `start` and every loop
/// gate write beside the document, and no pass may write outside the root.
#[test]
fn every_pass_and_gate_of_a_document_loop_writes_beside_the_document() {
    let tree = CallerTree::stage("mutation-lifetime-loop-document");
    install_claude(&tree);
    let document = tree.write(
        tree.in_root("prompts/loop/doc.md"),
        &format!(
            "---\n\
             loop:\n  \
               until: \"loop.count >= 2\"\n  \
               max: 3\n  \
               gate:\n    \
                 - append_line: [marker.log, \"gate {{{{ loop.count }}}}\"]\n\
             start:\n  \
               - append_line: [marker.log, \"start {{{{ loop.count }}}}\"]\n  \
               - {}\n\
             ---\n\
             Pass\n",
            outside_probe(&tree, "loop.log"),
        ),
    );

    let (status, output) = run(&tree, "compose", &document);

    assert_succeeded(status, &output);
    assert_written_beside(&document, MARKER, &["start 1", "gate 1", "start 2", "gate 2"]);
    assert_only_beside(tree.root(), &[&document], MARKER);
    tree.assert_outside_untouched();
}

/// A looping sequence: on every pass its own task writes beside the sequence
/// document and its referenced sequence writes beside that file.
#[test]
fn every_pass_of_a_sequence_loop_keeps_each_documents_base() {
    let tree = CallerTree::stage("mutation-lifetime-loop-sequence");
    let step = tree.write(
        tree.in_root("parts/step.yaml"),
        &format!(
            "initialize:\n  \
               - {}\n\
             sequence:\n  \
               - name: step\n    \
                 side_effect: {{ append_line: [marker.log, \"step\"] }}\n",
            outside_probe(&tree, "step.log"),
        ),
    );
    let sequence = tree.write(
        tree.in_root("flows/loop/seq.md"),
        "---\n\
         loop:\n  \
           until: \"loop.count >= 2\"\n  \
           max: 3\n\
         sequence:\n  \
           - name: own\n    \
             side_effect: { append_line: [marker.log, \"pass {{ loop.count }}\"] }\n  \
           - name: referenced\n    \
             sequence: ../../parts/step.yaml\n\
         ---\n",
    );

    let (status, output) = run(&tree, "sequence", &sequence);

    assert_succeeded(status, &output);
    assert_written_beside(&sequence, MARKER, &["pass 1", "pass 2"]);
    assert_written_beside(&step, MARKER, &["step", "step"]);
    assert_only_beside(tree.root(), &[&sequence, &step], MARKER);
    tree.assert_outside_untouched();
}

// --- nested sequences and groups ------------------------------------------

/// A referenced sequence and a referenced group, each in its own directory,
/// write relative targets beside their own documents; an inline group and an
/// inline sequence use the document that contains them. The referenced
/// sequence reaches the caller's root through `../`.
#[test]
fn referenced_containers_write_beside_themselves_and_inline_ones_beside_their_container() {
    let tree = CallerTree::stage("mutation-lifetime-nested");
    let nested = tree.write(
        tree.in_root("parts/nested.yaml"),
        &format!(
            "initialize:\n  \
               - append_line: [marker.log, \"nested initialize\"]\n  \
               - append_line: [../nested-reached-root.log, \"nested\"]\n  \
               - {}\n\
             sequence:\n  \
               - name: task\n    \
                 side_effect: {{ append_line: [marker.log, \"nested task\"] }}\n",
            outside_probe(&tree, "nested.log"),
        ),
    );
    let group = tree.write(
        tree.in_root("groups/bundle.yaml"),
        "kind: group\n\
         name: bundle\n\
         tasks:\n  \
           - name: leg\n    \
             side_effect: { append_line: [marker.log, \"group leg\"] }\n",
    );
    let root = tree.write(
        tree.in_root("flows/root.md"),
        "---\n\
         initialize:\n  \
           - append_line: [marker.log, \"root initialize\"]\n\
         sequence:\n  \
           - name: own\n    \
             side_effect: { append_line: [marker.log, \"root task\"] }\n  \
           - name: inline-group\n    \
             group:\n      \
               name: inline\n      \
               tasks:\n        \
                 - name: leg\n          \
                   side_effect: { append_line: [marker.log, \"inline group leg\"] }\n  \
           - name: inline-sequence\n    \
             sequence:\n      \
               - name: inner\n        \
                 side_effect: { append_line: [marker.log, \"inline sequence task\"] }\n  \
           - name: nested\n    \
             sequence: ../parts/nested.yaml\n  \
           - name: grouped\n    \
             group: ../groups/bundle.yaml\n\
         ---\n",
    );

    let (status, output) = run(&tree, "sequence", &root);

    assert_succeeded(status, &output);
    assert_written_beside(&root, MARKER, &["root initialize", "root task", "inline group leg", "inline sequence task"]);
    assert_written_beside(&nested, MARKER, &["nested initialize", "nested task"]);
    assert_written_beside(&group, MARKER, &["group leg"]);
    assert_only_beside(tree.root(), &[&root, &nested, &group], MARKER);
    assert!(tree.in_root("nested-reached-root.log").exists(), "`../` from the nested sequence stays inside the root");
    tree.assert_outside_untouched();
}

/// A referenced sequence from a foreign library runs inside the caller's
/// boundary: its absolute in-root target is written, while its relative
/// target resolves beside it, outside the root, and is refused.
#[test]
fn a_foreign_referenced_sequence_is_bounded_by_the_callers_root() {
    let tree = CallerTree::stage("mutation-lifetime-nested-foreign");
    let landed = tree.in_root("foreign-landed.log");
    let foreign = tree.write(
        tree.library.join("sequences/foreign.yaml"),
        &format!(
            "initialize:\n  \
               - append_line: [{}, \"foreign\"]\n\
             sequence:\n  \
               - name: task\n    \
                 side_effect: {{ append_line: [marker.log, \"foreign task\"] }}\n",
            yaml_path(&landed),
        ),
    );
    let root = tree.write(
        tree.in_root("flows/root.md"),
        &format!(
            "---\n\
             sequence:\n  \
               - name: own\n    \
                 side_effect: {{ append_line: [marker.log, \"root task\"] }}\n  \
               - name: foreign\n    \
                 sequence: {}\n\
             ---\n",
            yaml_path(&foreign),
        ),
    );

    let (status, output) = run(&tree, "sequence", &root);

    assert!(!status.success(), "the refused write fails the run; output was:\n{output}");
    assert_refused_outside_root(&output);
    assert_written_beside(&root, MARKER, &["root task"]);
    assert_only_beside(tree.root(), &[&root], MARKER);
    assert_eq!(fs::read_to_string(&landed).unwrap().trim(), "foreign", "an absolute in-root target is allowed");
    assert_nothing_under(&tree.library, MARKER);
}

// --- parallel members -----------------------------------------------------

/// The legs of one group run concurrently from different directories: two
/// referenced sequences, a prompt document, and an inline task. Each writes
/// several times; every line lands beside its own document, so no leg reads a
/// base another leg set.
#[test]
fn parallel_members_resolve_relative_targets_against_their_own_documents() {
    let tree = CallerTree::stage("mutation-lifetime-parallel");
    install_claude(&tree);
    let leg_sequence = |leg: &str| {
        format!(
            "initialize:\n  \
               - append_line: [marker.log, \"{leg} initialize\"]\n  \
               - {}\n\
             sequence:\n  \
               - {{ name: one, side_effect: {{ append_line: [marker.log, \"{leg} 1\"] }} }}\n  \
               - {{ name: two, side_effect: {{ append_line: [marker.log, \"{leg} 2\"] }} }}\n  \
               - {{ name: three, side_effect: {{ append_line: [marker.log, \"{leg} 3\"] }} }}\n",
            outside_probe(&tree, &format!("{leg}.log")),
        )
    };
    let alpha = tree.write(tree.in_root("alpha/leg.yaml"), &leg_sequence("alpha"));
    let beta = tree.write(tree.in_root("beta/leg.yaml"), &leg_sequence("beta"));
    let gamma = tree.write(
        tree.in_root("gamma/leg.md"),
        &format!(
            "---\n\
             initialize:\n  \
               - append_line: [marker.log, \"gamma initialize\"]\n\
             start:\n  \
               - append_line: [marker.log, \"gamma start\"]\n  \
               - {}\n\
             success:\n  \
               - append_line: [marker.log, \"gamma success\"]\n\
             ---\n\
             Gamma\n",
            outside_probe(&tree, "gamma.log"),
        ),
    );
    let fan = tree.write(
        tree.in_root("flows/fan.md"),
        "---\n\
         sequence:\n  \
           - name: fan\n    \
             group:\n      \
               name: fan\n      \
               tasks:\n        \
                 - { name: alpha, sequence: ../alpha/leg.yaml }\n        \
                 - { name: beta, sequence: ../beta/leg.yaml }\n        \
                 - { name: gamma, prompt: ../gamma/leg.md }\n        \
                 - { name: inline, side_effect: { append_line: [marker.log, \"inline\"] } }\n\
         ---\n",
    );

    let (status, output) = run(&tree, "sequence", &fan);

    assert_succeeded(status, &output);
    assert_written_beside(&alpha, MARKER, &["alpha initialize", "alpha 1", "alpha 2", "alpha 3"]);
    assert_written_beside(&beta, MARKER, &["beta initialize", "beta 1", "beta 2", "beta 3"]);
    assert_written_beside(&gamma, MARKER, &["gamma initialize", "gamma start", "gamma success"]);
    assert_written_beside(&fan, MARKER, &["inline"]);
    assert_only_beside(tree.root(), &[&alpha, &beta, &gamma, &fan], MARKER);
    tree.assert_outside_untouched();
}

// --- host wiring ----------------------------------------------------------

/// `compose` of a document in a subdirectory: the staged `initialize` and the
/// wrapper pipeline's `start`, `success`, and `finalize` all write beside the
/// document, inside the caller's root.
#[test]
fn compose_staged_initialize_and_pipeline_events_write_beside_the_document() {
    let tree = CallerTree::stage("mutation-lifetime-compose");
    install_claude(&tree);
    let document = tree.write(
        tree.in_root("prompts/wired/doc.md"),
        &format!(
            "---\n\
             initialize:\n  \
               - append_line: [marker.log, initialize]\n  \
               - {}\n\
             start:\n  \
               - append_line: [marker.log, start]\n\
             success:\n  \
               - append_line: [marker.log, success]\n\
             finalize:\n  \
               - append_line: [marker.log, finalize]\n\
             ---\n\
             Body\n",
            outside_probe(&tree, "compose.log"),
        ),
    );

    let (status, output) = run(&tree, "compose", &document);

    assert_succeeded(status, &output);
    assert_written_beside(&document, MARKER, &["initialize", "start", "success", "finalize"]);
    assert_only_beside(tree.root(), &[&document], MARKER);
    tree.assert_outside_untouched();
}

/// `inline-compose` of a document in a subdirectory: the same events write
/// beside the document while the agent rewrites its body.
#[cfg(unix)]
#[test]
fn inline_compose_events_write_beside_the_document() {
    let tree = CallerTree::stage("mutation-lifetime-inline");
    let document = tree.write(
        tree.in_root("prompts/wired/inline.md"),
        &format!(
            "---\n\
             prompt: \"Rewrite the body\"\n\
             initialize:\n  \
               - append_line: [marker.log, initialize]\n  \
               - {}\n\
             start:\n  \
               - append_line: [marker.log, start]\n\
             success:\n  \
               - append_line: [marker.log, success]\n\
             finalize:\n  \
               - append_line: [marker.log, finalize]\n\
             ---\n\
             Original body\n",
            outside_probe(&tree, "inline.log"),
        ),
    );
    common::InlineAgentStub::new(&document).install(tree.fixture.bin_dir(), "claude");

    let (status, output) = run(&tree, "inline-compose", &document);

    assert_succeeded(status, &output);
    assert_written_beside(&document, MARKER, &["initialize", "start", "success", "finalize"]);
    assert_only_beside(tree.root(), &[&document], MARKER);
    tree.assert_outside_untouched();
}
