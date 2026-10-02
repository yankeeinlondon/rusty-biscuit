//! Completion after the composition file reads the arguments with the same
//! type-aware ownership as execution.
//!
//! These drive the compiled binary's hidden `__complete` subcommand. A word
//! an open provider switch takes belongs to the agent, so nothing is offered
//! for it; a setter or positional slot keeps its schema-aware completion; a
//! line execution would reject, or one ownership cannot decide, offers
//! nothing, without prompting or writing anything.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::common;
use common::TestWorkspace;
use common::completion::{run_complete, seed_plain_git_repo, write_file};

/// A plan declaring `phase` (number) and `mode` (enum), plus `extra`
/// frontmatter lines.
fn plan(ws: &Path, extra: &str) {
    seed_plain_git_repo(ws);
    write_file(
        &ws.join("prompts").join("plan.md"),
        &format!(
            "---\n{extra}$schema:\n  phase: number\n  mode: enum(fast, slow)\n---\nPhase {{{{phase}}}}.\n"
        ),
    );
}

fn complete(ws: &Path, words: &[&str]) -> Vec<String> {
    let mut argv = vec!["compose", "prompts/plan.md"];
    argv.extend_from_slice(words);
    run_complete(ws, &argv)
}

#[test]
fn a_word_after_a_switch_value_is_claudines_and_completes_as_a_setter() {
    let ws = TestWorkspace::named("complete-ownership-control");
    plan(ws.path(), "");
    // Control: `-c` took `low`, so `ph` is a setter name.
    assert_eq!(complete(ws.path(), &["--codex", "-c", "low", "ph"]), vec!["phase="]);
    assert_eq!(
        complete(ws.path(), &["--codex", "-c", "low", "mode="]),
        vec!["mode='fast'", "mode='slow'"]
    );
    // A removed Claudine option ends the switch's run.
    assert_eq!(complete(ws.path(), &["--codex", "-c", "low", "--plain", "ph"]), vec!["phase="]);
}

#[test]
fn a_word_an_open_switch_takes_is_the_agents_and_offers_nothing() {
    let ws = TestWorkspace::named("complete-ownership-provider-slot");
    plan(ws.path(), "");
    // Unfinished value slot: the cursor is `-c`'s value, empty or partial.
    for cursor in ["", "ph", "mo"] {
        let got = complete(ws.path(), &["--codex", "-c", cursor]);
        assert!(got.is_empty(), "{cursor:?}: {got:?}");
    }
    // A variadic switch keeps taking words.
    let got = complete(ws.path(), &["--claude", "--add-dir", "a", "ph"]);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn a_terminal_missing_value_offers_nothing() {
    let ws = TestWorkspace::named("complete-ownership-missing-value");
    plan(ws.path(), "");
    // `phase=2` is a declared parameter, so `-c` has no value: execution
    // fails, and the setter slots after it offer nothing.
    for cursor in ["ph", "mode="] {
        let got = complete(ws.path(), &["--codex", "-c", "phase=2", cursor]);
        assert!(got.is_empty(), "{cursor:?}: {got:?}");
    }
    // The same holds when the declared parameter is under the cursor.
    let got = complete(ws.path(), &["--codex", "-c", "mode="]);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn candidates_that_disagree_offer_nothing_and_the_files_agent_decides() {
    let ws = TestWorkspace::named("complete-ownership-ambiguous");
    plan(ws.path(), "");
    // No provider named and no `agent`: Claude's `-c` takes nothing, Codex's
    // takes a string, so `low` is ambiguous. Completion never prompts.
    let got = complete(ws.path(), &["-c", "low", "ph"]);
    assert!(got.is_empty(), "{got:?}");

    let narrowed = TestWorkspace::named("complete-ownership-agent");
    plan(narrowed.path(), "agent: codex\n");
    assert_eq!(complete(narrowed.path(), &["-c", "low", "ph"]), vec!["phase="]);
    // A caller `agent=` setter does not narrow ownership.
    let got = complete(ws.path(), &["agent=codex", "-c", "low", "ph"]);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn an_unreadable_file_or_schema_offers_nothing_once_a_switch_needs_it() {
    let ws = TestWorkspace::named("complete-ownership-unreadable");
    seed_plain_git_repo(ws.path());
    write_file(
        &ws.path().join("prompts").join("plan.md"),
        "---\n$schema: ./missing.yaml\n---\nBody.\n",
    );
    write_file(&ws.path().join("docs").join("spec.md"), "# s\n");
    let got = complete(ws.path(), &["--codex", "-c", "low", "spec=@s"]);
    assert!(got.is_empty(), "{got:?}");
    let got = run_complete(ws.path(), &["compose", "prompts/absent.md", "--codex", "-c", "low", "spec=@s"]);
    assert!(got.is_empty(), "{got:?}");

    // With no provider switch the file is not read, so `@` completion keeps
    // working for a file that does not exist yet.
    let got = run_complete(ws.path(), &["compose", "prompts/absent.md", "spec=@s"]);
    assert!(got.iter().any(|c| c == "spec='docs/spec.md'"), "{got:?}");
}

#[test]
fn nothing_is_offered_after_an_authored_separator() {
    let ws = TestWorkspace::named("complete-ownership-separator");
    plan(ws.path(), "");
    for cursor in ["", "ph", "mode=", "-", "--mod"] {
        let got = complete(ws.path(), &["--", cursor]);
        assert!(got.is_empty(), "{cursor:?}: {got:?}");
    }
    // Before the separator the flag surface is still offered.
    let got = complete(ws.path(), &["--mod"]);
    assert!(got.iter().any(|c| c == "--model"), "{got:?}");
}

#[test]
fn every_composition_command_reads_ownership_the_same_way() {
    let ws = TestWorkspace::named("complete-ownership-commands");
    plan(ws.path(), "");
    for command in ["compose", "inline-compose", "sequence"] {
        let control = run_complete(ws.path(), &[command, "prompts/plan.md", "--codex", "-c", "low", "ph"]);
        assert_eq!(control, vec!["phase="], "{command}");
        let slot = run_complete(ws.path(), &[command, "prompts/plan.md", "--codex", "-c", "ph"]);
        assert!(slot.is_empty(), "{command}: {slot:?}");
    }
}

/// Every file under `root` with its contents.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let key = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                out.insert(key, fs::read(&path).unwrap());
            }
        }
    }
    out
}

#[test]
fn completion_reads_ownership_without_writing_anything() {
    let ws = TestWorkspace::named("complete-ownership-read-only");
    plan(ws.path(), "agent: codex\ninteractive: false\n");
    let before = tree(ws.path());
    for words in [
        &["-c", "low", "ph"][..],
        &["-c", "ph"][..],
        &["-c", "phase=2", "ph"][..],
        &["--claude", "-c", "low", "mode="][..],
    ] {
        complete(ws.path(), words);
    }
    assert_eq!(tree(ws.path()), before);
}

#[test]
fn review_probe_completion_sweep() {
    let ws = TestWorkspace::named("review-completion-sweep");
    plan(ws.path(), "");
    for command in ["compose", "inline-compose", "sequence"] {
        for tail in [vec!["--codex", "-c", "phase=2", "ph"], vec!["--codex", "-c", "phase=2", "--mod"], vec!["-c", "low", "--mod"], vec!["--codex", "-c", "low", "--mod"]] {
            let mut args = vec![command, "prompts/plan.md"];
            args.extend(tail);
            eprintln!("REVIEW {:?} => {:?}", args, run_complete(ws.path(), &args));
        }
    }
}
