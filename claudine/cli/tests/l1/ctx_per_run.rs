//! Level-1 process coverage for `ctx` being evaluated once per composition
//! run, and for `ctx` requirements being collected across the transclusion
//! tree.
//!
//! Every vehicle row has the same shape. The fixture repository starts with
//! one untracked file (`a.txt`) and nothing staged. Run A reads
//! `length(ctx.staged_files)`; something between the runs stages the file;
//! run B reads the same expression and must report `1`. Each run records what
//! it saw by appending a line to `runs.log` from its own lifecycle, so the
//! assertion reads the run's prepared `ctx`, not a live observation.

use crate::common;

use common::CliProcessFixture;
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::fs;
use std::path::Path;

/// A Claude stub that reports one successful result, with a session so a
/// `resume` can continue it, and exits 0.
fn write_succeeding_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"session-1","model":"claude-test"}'
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

/// `git` in the fixture repository, with an identity and no signing so the
/// host's configuration can neither change nor block the fixture history.
fn git(fixture: &CliProcessFixture, args: &[&str]) {
    let output = common::helper_command("git")
        .arg("-C")
        .arg(fixture.cwd())
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("run git in the fixture");
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// A repository on `main` with one commit, one untracked `a.txt`, and a shell
/// policy that approves the Git commands the rows run.
fn repo(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    write_succeeding_claude(fixture.bin_dir());
    git(&fixture, &["init", "-q", "--initial-branch=main"]);
    write(&fixture.cwd().join("README.md"), "fixture\n");
    write(
        &fixture.cwd().join(".gitignore"),
        ".darkmatter-shell-whitelist\nruns.log\nmarker.txt\n.claudine/\n",
    );
    git(&fixture, &["add", "README.md", ".gitignore"]);
    git(&fixture, &["commit", "-q", "-m", "initial"]);
    write(&fixture.cwd().join("a.txt"), "untracked\n");
    write(
        &fixture.cwd().join(".darkmatter-shell-whitelist"),
        "exact git add a.txt\nexact git checkout -q -b feature\n",
    );
    fixture
}

struct Run {
    code: Option<i32>,
    output: String,
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let output = fixture
        .command_builder()
        // Lifecycle and sequence `shell` steps run `git`; Git for Windows is
        // outside the minimal system set.
        .host_path()
        .build()
        .args(args)
        .output()
        .expect("claudine runs");
    Run {
        code: output.status.code(),
        output: common::strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )),
    }
}

fn doc(fixture: &CliProcessFixture, name: &str, content: &str) -> String {
    write(&fixture.cwd().join(name), content);
    name.to_string()
}

/// The lines every run appended to `runs.log`, in order.
fn runs(fixture: &CliProcessFixture) -> Vec<String> {
    fs::read_to_string(fixture.cwd().join("runs.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// A stack item appending what this run's `ctx` holds, under `label`.
fn record(label: &str) -> String {
    format!(
        "        - action:\n              - append_line: [\"runs.log\", \"{label} staged={{{{ length(ctx.staged_files) }}}}\"]\n"
    )
}

/// `record`, adding the stable identity a hop must not move and the branch.
fn record_identity(label: &str) -> String {
    format!(
        "        - action:\n              - append_line: [\"runs.log\", \"{label} staged={{{{ length(ctx.staged_files) }}}} branch={{{{ ctx.branch }}}} cwd={{{{ ctx.cwd }}}} root={{{{ ctx.repo_root }}}}\"]\n"
    )
}

fn field<'a>(line: &'a str, name: &str) -> &'a str {
    line.split(' ')
        .find_map(|part| part.strip_prefix(&format!("{name}=")))
        .unwrap_or_default()
}

/// Each `record_identity` line as `{label} staged={n}`, after asserting that
/// every line reports the same, non-empty `cwd` and `root`: a hop never moves
/// the launch identity.
fn staged_with_stable_identity(lines: &[String]) -> Vec<String> {
    let first = lines.first().map(String::as_str).unwrap_or_default();
    for key in ["cwd", "root"] {
        assert!(!field(first, key).is_empty(), "`{key}` is recorded: {lines:?}");
        for line in lines {
            assert_eq!(field(line, key), field(first, key), "`{key}`: {lines:?}");
        }
    }
    lines
        .iter()
        .map(|line| {
            let label = line.split(' ').next().unwrap_or_default();
            format!("{label} staged={}", field(line, "staged"))
        })
        .collect()
}

/// The control: one run holds one observation, even after that run stages the
/// file itself.
#[test]
fn a_run_keeps_its_own_observation_after_it_stages() {
    let fixture = repo("ctx-per-run-control");
    let file = doc(
        &fixture,
        "direct.md",
        &format!(
            "---\nstart:\n    stack:\n        - action:\n              - shell: \"git add a.txt\"\nsuccess:\n    stack:\n{}---\nBODY staged={{{{ length(ctx.staged_files) }}}}\n",
            record("direct")
        ),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert!(run.output.contains("BODY staged=0"), "{}", run.output);
    assert_eq!(runs(&fixture), ["direct staged=0"], "{}", run.output);
}

/// A proxy target is a new run. It observes what its source staged whether or
/// not the source named the property first, and the launch identity does not
/// move across the hop.
#[test]
fn a_proxy_target_observes_its_sources_staging_regardless_of_first_mention() {
    let target = format!(
        "---\nsuccess:\n    stack:\n{}---\ntarget body\n",
        record_identity("target")
    );
    for (label, source_mentions) in [("mentioned", true), ("unmentioned", false)] {
        let fixture = repo(&format!("ctx-per-run-proxy-{label}"));
        doc(&fixture, "target.md", &target);
        let source_record = match source_mentions {
            true => record_identity("source"),
            false => String::new(),
        };
        let file = doc(
            &fixture,
            "router.md",
            &format!(
                "---\nstart:\n    stack:\n{source_record}        - action:\n              - shell: \"git add a.txt\"\n              - proxy: ./target.md\n---\nrouter body\n"
            ),
        );

        let run = run(&fixture, &["compose", "--claude", &file]);

        assert_eq!(run.code, Some(0), "{label}: {}", run.output);
        let lines = runs(&fixture);
        let target_line = lines.last().expect("the target recorded its run");
        assert_eq!(field(target_line, "staged"), "1", "{label}: {lines:?}");
        if source_mentions {
            assert_eq!(field(&lines[0], "staged"), "0", "{lines:?}");
            for key in ["cwd", "root", "branch"] {
                assert_eq!(field(&lines[0], key), field(target_line, key), "`{key}`: {lines:?}");
            }
        }
    }
}

/// A branch created between runs is the next run's `ctx.branch`, while the
/// run that created it keeps the branch it started on.
#[test]
fn a_proxy_target_observes_a_branch_its_source_created() {
    let fixture = repo("ctx-per-run-branch");
    doc(
        &fixture,
        "target.md",
        &format!("---\nsuccess:\n    stack:\n{}---\ntarget body\n", record_identity("target")),
    );
    let file = doc(
        &fixture,
        "router.md",
        &format!(
            "---\nstart:\n    stack:\n        - action:\n              - shell: \"git checkout -q -b feature\"\n{}        - action:\n              - proxy: ./target.md\n---\nrouter body\n",
            record_identity("source")
        ),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    let lines = runs(&fixture);
    assert_eq!(lines.len(), 2, "{lines:?}\n{}", run.output);
    assert_eq!(field(&lines[0], "branch"), "main", "{lines:?}");
    assert_eq!(field(&lines[1], "branch"), "feature", "{lines:?}");
    for key in ["cwd", "root"] {
        assert_eq!(field(&lines[0], key), field(&lines[1], key), "`{key}`: {lines:?}");
    }
}

/// A working-tree edit made between runs, with nothing staged, is the next
/// run's `ctx.dirty_files`, while the run that made it keeps its own
/// observation.
#[test]
fn a_proxy_target_observes_a_working_tree_edit_its_source_made() {
    let fixture = repo("ctx-per-run-worktree");
    let record_dirty = |label: &str| {
        format!(
            "        - action:\n              - append_line: [\"runs.log\", \"{label} dirty={{{{ ctx.dirty_files }}}} cwd={{{{ ctx.cwd }}}} root={{{{ ctx.repo_root }}}}\"]\n"
        )
    };
    doc(
        &fixture,
        "target.md",
        &format!("---\nsuccess:\n    stack:\n{}---\ntarget body\n", record_dirty("target")),
    );
    let file = doc(
        &fixture,
        "router.md",
        &format!(
            "---\nstart:\n    stack:\n{}        - action:\n              - append_line: [\"README.md\", \"edited\"]\n              - proxy: ./target.md\n---\nrouter body\n",
            record_dirty("source")
        ),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    let lines = runs(&fixture);
    assert_eq!(lines.len(), 2, "{lines:?}\n{}", run.output);
    assert!(field(&lines[0], "dirty").contains("\"a.txt\""), "{lines:?}");
    assert!(!field(&lines[0], "dirty").contains("\"README.md\""), "{lines:?}");
    assert!(field(&lines[1], "dirty").contains("\"README.md\""), "{lines:?}");
    for key in ["cwd", "root"] {
        assert_eq!(field(&lines[0], key), field(&lines[1], key), "`{key}`: {lines:?}");
    }
}

/// A sequence step is a run: a later step observes what an earlier step staged.
#[test]
fn a_later_sequence_step_observes_an_earlier_steps_staging() {
    let fixture = repo("ctx-per-run-sequence");
    for (name, label) in [("first.md", "step1"), ("third.md", "step3")] {
        doc(
            &fixture,
            name,
            &format!("---\nsuccess:\n    stack:\n{}---\nstep body\n", record_identity(label)),
        );
    }
    let file = doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n  - name: one\n    prompt: ./first.md\n  - name: two\n    shell: \"git add a.txt\"\n  - name: three\n    prompt: ./third.md\n---\n",
    );

    let run = run(&fixture, &["sequence", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        staged_with_stable_identity(&runs(&fixture)),
        ["step1 staged=0", "step3 staged=1"],
        "{}",
        run.output
    );
}

/// A loop iteration is a run: iteration 2 observes what iteration 1 staged.
#[test]
fn a_later_loop_iteration_observes_the_previous_iterations_staging() {
    let fixture = repo("ctx-per-run-loop");
    let file = doc(
        &fixture,
        "loop.md",
        "---\nn: 0\nloop:\n    while: \"n < 1\"\n    action: \"increment(n)\"\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"runs.log\", \"iteration{{ _loop_count }} staged={{ length(ctx.staged_files) }} cwd={{ ctx.cwd }} root={{ ctx.repo_root }}\"]\n              - shell: \"git add a.txt\"\n---\nloop body\n",
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        staged_with_stable_identity(&runs(&fixture)),
        ["iteration1 staged=0", "iteration2 staged=1"],
        "{}",
        run.output
    );
}

/// The re-entry stack each recovery row appends: on the first attempt only
/// (while `marker.txt` is absent), stage the file, then recover with `verb`.
fn first_attempt_stages(event: &str, recovery: &str) -> String {
    format!(
        "{event}:\n    stack:\n        - when: \"!file_exists('marker.txt')\"\n          action:\n              - append_line: [\"marker.txt\", \"first\"]\n              - shell: \"git add a.txt\"\n{recovery}"
    )
}

/// A retry is a run: the retried attempt observes what the first staged.
#[test]
fn a_retry_attempt_observes_the_previous_attempts_staging() {
    let fixture = repo("ctx-per-run-retry");
    let file = doc(
        &fixture,
        "retry.md",
        &format!(
            "---\nsuccess:\n    stack:\n{}{}---\nbody\n",
            record_identity("attempt"),
            first_attempt_stages("finalize", "              - retry: 1\n")
                .replacen("finalize:\n    stack:\n", "", 1),
        )
        .replacen("---\nbody", "---\nbody", 1),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        staged_with_stable_identity(&runs(&fixture)),
        ["attempt staged=0", "attempt staged=1"],
        "{}",
        run.output
    );
}

/// A resume is a run: the resumed attempt observes what the first staged.
#[test]
fn a_resumed_attempt_observes_the_previous_attempts_staging() {
    let fixture = repo("ctx-per-run-resume");
    let file = doc(
        &fixture,
        "resume.md",
        &format!(
            "---\nsuccess:\n    stack:\n{}{}failure:\n    stack:\n        - action:\n              - action: resume\n                message: \"again\"\n                max_attempts: 1\n---\nbody\n",
            record_identity("attempt"),
            first_attempt_stages("success", "              - error: \"again\"\n")
                .replacen("success:\n    stack:\n", "", 1),
        ),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        staged_with_stable_identity(&runs(&fixture)),
        ["attempt staged=0", "attempt staged=1"],
        "{}",
        run.output
    );
}

/// Each task of a serial group is a run: a member observes what the member
/// before it staged.
#[test]
fn a_serial_group_member_observes_an_earlier_members_staging() {
    let fixture = repo("ctx-per-run-serial-group");
    doc(
        &fixture,
        "read.md",
        &format!("---\nsuccess:\n    stack:\n{}---\nmember body\n", record_identity("member")),
    );
    let file = doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n  - name: bundle\n    group:\n      name: bundle\n      execution: serial\n      tasks:\n        - name: before\n          prompt: ./read.md\n        - name: stage\n          shell: \"git add a.txt\"\n        - name: after\n          prompt: ./read.md\n---\n",
    );

    let run = run(&fixture, &["sequence", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        staged_with_stable_identity(&runs(&fixture)),
        ["member staged=0", "member staged=1"],
        "{}",
        run.output
    );
}

/// A parallel group is one run for every sibling's first attempt: a sibling
/// that starts after another staged still reads the group's shared capture.
/// A sibling that re-enters (here, a retry) is a new run and sees what its
/// siblings changed.
#[test]
fn parallel_siblings_share_one_capture_until_one_reenters() {
    let fixture = repo("ctx-per-run-parallel-group");
    doc(
        &fixture,
        "read.md",
        &format!(
            "---\nsuccess:\n    stack:\n{}finalize:\n    stack:\n        - when: \"!file_exists('marker.txt')\"\n          action:\n              - append_line: [\"marker.txt\", \"first\"]\n              - retry: 1\n---\nmember body\n",
            record_identity("member")
        ),
    );
    // `max_parallel: 1` admits the siblings in declaration order, so the shell
    // sibling has finished staging before the prompt sibling composes.
    let file = doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n  - name: bundle\n    group:\n      name: bundle\n      execution: parallel\n      max_parallel: 1\n      tasks:\n        - name: stage\n          shell: \"git add a.txt\"\n        - name: read\n          prompt: ./read.md\n---\n",
    );

    let run = run(&fixture, &["sequence", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        staged_with_stable_identity(&runs(&fixture)),
        ["member staged=0", "member staged=1"],
        "{}",
        run.output
    );
}

/// The line of `output` that starts with `marker`, trimmed of the prompt
/// block's gutter.
fn line_with<'a>(output: &'a str, marker: &str) -> &'a str {
    output
        .lines()
        .map(|line| line.trim_start_matches(|c: char| c == '┃' || c.is_whitespace()))
        .find(|line| line.starts_with(marker))
        .unwrap_or_else(|| panic!("no line starting `{marker}` in:\n{output}"))
        .trim_end()
}

/// What Darkmatter alone composes for `file`, the way `md compose` does.
fn darkmatter_composes(fixture: &CliProcessFixture, file: &str) -> String {
    let path = fixture.cwd().join(file);
    let markdown = darkmatter::markdown::Markdown::try_from(path.as_path()).expect("load document");
    let options = darkmatter::markdown::compose::ComposeOptions::for_document(fixture.cwd(), &markdown)
        .with_source_file(&path);
    let (composed, _) = markdown.compose_with(options).expect("darkmatter composes the tree");
    composed.content().to_string()
}

/// A property mentioned only in a transcluded file is captured for the run
/// and renders exactly as Darkmatter renders the same tree, however deep the
/// file is, however its path is spelled, whatever the parent already names,
/// and beside a value the include overlays.
#[test]
fn a_partial_only_property_renders_as_darkmatter_renders_it() {
    const KID: &str =
        "KID os=[{{ ctx.os }}] branch=[{{ ctx.branch }}] staged=[{{ length(ctx.staged_files) }}]\n";
    for (label, kid, files) in [
        (
            "direct partial",
            KID,
            vec![("parent.md", "Parent line.\n\n::file ./kid.md\n")],
        ),
        (
            "two levels deep",
            KID,
            vec![
                ("parent.md", "Parent line.\n\n::file ./sub/mid.md\n"),
                ("sub/mid.md", "Middle line.\n\n::file ../kid.md\n"),
            ],
        ),
        (
            "interpolated reference",
            KID,
            vec![("parent.md", "---\nwhich: kid\n---\nParent line.\n\n::file ./{{ which }}.md\n")],
        ),
        // The F5 row where the parent's own mention used to decide the capture.
        (
            "parent names one of the kids properties",
            "KID repo=[{{ ctx.repo }}] os=[{{ ctx.os }}]\n",
            vec![("parent.md", "Parent repo={{ ctx.repo }}.\n\n::file ./kid.md\n")],
        ),
        (
            "local overlay",
            "---\nlabel: default\n---\nKID label=[{{ label }}] branch=[{{ ctx.branch }}] staged=[{{ length(ctx.staged_files) }}]\n",
            vec![("parent.md", "Parent line.\n\n::file ./kid.md set.label=\"local\"\n")],
        ),
    ] {
        let fixture = repo(&format!("ctx-per-run-partial-{}", label.replace(' ', "-")));
        // `ctx.repo` names the repository after its remote.
        git(&fixture, &["remote", "add", "origin", "https://example.com/fixture/widget.git"]);
        doc(&fixture, "kid.md", kid);
        for (name, content) in &files {
            doc(&fixture, name, content);
        }

        let run = run(&fixture, &["compose", "--claude", "--dry-run", "parent.md"]);
        let expected = darkmatter_composes(&fixture, "parent.md");

        assert_eq!(run.code, Some(0), "{label}: {}", run.output);
        let claudine = line_with(&run.output, "KID ");
        assert_eq!(claudine, line_with(&expected, "KID "), "{label}");
        assert!(!claudine.contains("[]"), "{label}: every value is captured: {claudine}");
        match label {
            "local overlay" => assert!(claudine.contains("label=[local]"), "{claudine}"),
            "parent names one of the kids properties" => {
                assert!(claudine.contains("repo=[widget]"), "{claudine}")
            }
            _ => {}
        }
    }
}

/// A partial and its parent are one run: both read one observation.
#[test]
fn a_partial_shares_its_parents_observation() {
    let fixture = repo("ctx-per-run-partial-shares");
    doc(&fixture, "kid.md", "KID staged={{ length(ctx.staged_files) }}\n");
    doc(
        &fixture,
        "parent.md",
        "---\nstart:\n    stack:\n        - action:\n              - shell: \"git add a.txt\"\n---\nPARENT staged={{ length(ctx.staged_files) }}\n\n::file ./kid.md\n",
    );

    let run = run(&fixture, &["compose", "--claude", "--perf", "parent.md"]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(line_with(&run.output, "PARENT "), "PARENT staged=0", "{}", run.output);
    assert_eq!(line_with(&run.output, "KID "), "KID staged=0", "{}", run.output);
    // Equal values could still come from two observations of an unchanged
    // tree; one observation for the whole run is what makes them one.
    assert_eq!(volatile_observations(&run.output), "file_changes", "{}", run.output);
}

/// The `--perf` note's volatile-observation list, with the note's hard wraps
/// and gutter removed.
fn volatile_observations(output: &str) -> String {
    let collapsed = output.replace('▌', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let start = collapsed
        .find("volatile observations [")
        .unwrap_or_else(|| panic!("no perf note in:\n{output}"));
    let rest = &collapsed[start + "volatile observations [".len()..];
    rest[..rest.find(']').expect("the list closes")].to_string()
}

/// A fenced example in a partial is not composed, so it demands no Git state;
/// the same mention outside the fence does.
#[test]
fn a_fenced_example_in_a_partial_observes_no_git_state() {
    for (label, kid, expected) in [
        ("fenced", "Example:\n\n```md\n{{ ctx.staged_files }}\n```\n", ""),
        ("composed", "Staged: {{ length(ctx.staged_files) }}\n", "file_changes"),
    ] {
        let fixture = repo(&format!("ctx-per-run-fenced-{label}"));
        doc(&fixture, "kid.md", kid);
        doc(&fixture, "parent.md", "Parent line.\n\n::file ./kid.md\n");

        let run = run(&fixture, &["compose", "--claude", "--dry-run", "--perf", "parent.md"]);

        assert_eq!(run.code, Some(0), "{label}: {}", run.output);
        assert_eq!(volatile_observations(&run.output), expected, "{label}: {}", run.output);
    }
}

/// `initialize` of both staged rows writes `new.txt`, a second untracked file.
const INITIALIZE_WRITES: &str =
    "initialize:\n    stack:\n        - action:\n              - append_line: [\"new.txt\", \"created by initialize\"]\n";

/// A document with `initialize` reads its root page before `initialize` runs,
/// so a property its body names is captured before `initialize` changes the
/// working tree.
#[test]
fn a_body_property_is_captured_before_initialize() {
    let fixture = repo("ctx-per-run-body-before-initialize");
    let file = doc(
        &fixture,
        "staged.md",
        &format!("---\n{INITIALIZE_WRITES}---\nBODY untracked={{{{ ctx.untracked_files }}}}\n"),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert!(fixture.cwd().join("new.txt").exists(), "{}", run.output);
    let body = line_with(&run.output, "BODY ");
    assert!(body.contains("a.txt") && !body.contains("new.txt"), "{body}\n{}", run.output);
}

/// The one exception: an include cannot be read before `initialize`, so a
/// group that an include is the first to name is captured on first demand,
/// after `initialize`.
#[test]
fn an_include_first_naming_a_group_captures_it_after_initialize() {
    let fixture = repo("ctx-per-run-include-after-initialize");
    doc(&fixture, "part.md", "PART untracked={{ ctx.untracked_files }}\n");
    let file = doc(
        &fixture,
        "staged.md",
        &format!("---\n{INITIALIZE_WRITES}---\nStaged document.\n\n::file ./part.md\n"),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    let part = line_with(&run.output, "PART ");
    assert!(part.contains("new.txt"), "{part}\n{}", run.output);
}

/// `current` is observed once per event: every read within one event's stack
/// sees one value, even after that stack changes the working tree, and the
/// next event observes again.
#[test]
fn current_is_observed_once_per_event() {
    let fixture = repo("ctx-per-run-current-per-event");
    let file = doc(
        &fixture,
        "current.md",
        "---\nstart:\n    stack:\n        - action:\n              - append_line: [\"runs.log\", \"start-before {{ length(current.staged_files) }}\"]\n              - shell: \"git add a.txt\"\n              - append_line: [\"runs.log\", \"start-after {{ length(current.staged_files) }}\"]\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"runs.log\", \"success {{ length(current.staged_files) }}\"]\n---\nbody\n",
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        runs(&fixture),
        ["start-before 0", "start-after 0", "success 1"],
        "{}",
        run.output
    );
}

/// Commit `tick` with a fixture identity: a side effect that portably leaves
/// a countable trace, one commit per execution.
const TICK: &str =
    "git -c user.name=Fixture -c user.email=fixture@example.com -c commit.gpgsign=false commit -q --allow-empty -m tick";

fn commit_count(fixture: &CliProcessFixture) -> String {
    let output = common::helper_command("git")
        .arg("-C")
        .arg(fixture.cwd())
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .expect("run git in the fixture");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Discovering a nested or interpolated include's `ctx` requirements reads
/// it; it does not execute it. A body `::shell` in that include runs exactly
/// once, when the run composes.
#[test]
fn discovering_an_include_executes_none_of_its_commands() {
    let kid = format!("KID staged={{{{ length(ctx.staged_files) }}}}\n\n::shell {TICK}\n");
    for (label, files) in [
        (
            "nested",
            vec![
                ("parent.md", "Parent line.\n\n::file ./sub/mid.md\n"),
                ("sub/mid.md", "Middle line.\n\n::file ../kid.md\n"),
            ],
        ),
        (
            "interpolated",
            vec![("parent.md", "---\nwhich: kid\n---\nParent line.\n\n::file ./{{ which }}.md\n")],
        ),
    ] {
        let fixture = repo(&format!("ctx-per-run-discovery-{label}"));
        write(
            &fixture.cwd().join(".darkmatter-shell-whitelist"),
            &format!("exact {TICK}\n"),
        );
        doc(&fixture, "kid.md", &kid);
        for (name, content) in &files {
            doc(&fixture, name, content);
        }

        let run = run(&fixture, &["compose", "--claude", "parent.md"]);

        assert_eq!(run.code, Some(0), "{label}: {}", run.output);
        assert_eq!(line_with(&run.output, "KID "), "KID staged=0", "{label}: {}", run.output);
        assert_eq!(commit_count(&fixture), "2", "{label}: initial plus one tick\n{}", run.output);
    }
}

/// Same-run extension is not a refresh: a group the root page names is
/// captured before `initialize`, and an include naming the same group reads
/// that capture rather than the tree `initialize` changed.
#[test]
fn an_include_reads_the_group_its_root_captured_before_initialize() {
    let fixture = repo("ctx-per-run-include-shares-root-capture");
    doc(&fixture, "part.md", "PART untracked={{ ctx.untracked_files }}\n");
    let file = doc(
        &fixture,
        "staged.md",
        &format!(
            "---\n{INITIALIZE_WRITES}---\nBODY untracked={{{{ ctx.untracked_files }}}}\n\n::file ./part.md\n"
        ),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert!(fixture.cwd().join("new.txt").exists(), "{}", run.output);
    for marker in ["BODY ", "PART "] {
        let line = line_with(&run.output, marker);
        assert!(line.contains("a.txt") && !line.contains("new.txt"), "{line}\n{}", run.output);
    }
}

/// The root page's frontmatter counts as the root page: a group named only in
/// a lifecycle stack is captured before `initialize`, so an include that
/// names it too does not take the include exception.
#[test]
fn a_lifecycle_only_property_is_captured_before_initialize() {
    let fixture = repo("ctx-per-run-lifecycle-before-initialize");
    doc(&fixture, "part.md", "PART untracked={{ ctx.untracked_files }}\n");
    let file = doc(
        &fixture,
        "staged.md",
        &format!(
            "---\n{INITIALIZE_WRITES}success:\n    stack:\n        - action:\n              - append_line: [\"runs.log\", \"root untracked={{{{ ctx.untracked_files }}}}\"]\n---\nStaged document.\n\n::file ./part.md\n"
        ),
    );

    let run = run(&fixture, &["compose", "--claude", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert!(fixture.cwd().join("new.txt").exists(), "{}", run.output);
    let part = line_with(&run.output, "PART ");
    assert!(part.contains("a.txt") && !part.contains("new.txt"), "{part}\n{}", run.output);
    let lines = runs(&fixture);
    assert_eq!(lines.len(), 1, "{lines:?}\n{}", run.output);
    assert!(lines[0].contains("a.txt") && !lines[0].contains("new.txt"), "{lines:?}");
}

/// A transclusion cycle among files that name `ctx` ends in the existing cycle
/// error, naming both files, under a dry run and a real run alike.
#[test]
fn a_transclusion_cycle_during_ctx_collection_is_the_cycle_error() {
    let fixture = repo("ctx-per-run-cycle");
    doc(&fixture, "a.md", "A root={{ ctx.repo_root }}\n\n::file ./b.md\n");
    doc(&fixture, "b.md", "B root={{ ctx.repo_root }}\n\n::file ./a.md\n");

    for args in [
        &["compose", "--claude", "--dry-run", "a.md"][..],
        &["compose", "--claude", "a.md"][..],
    ] {
        let run = run(&fixture, args);

        assert_eq!(run.code, Some(1), "{args:?}: {}", run.output);
        // The chain's paths wrap with the terminal; compare without layout.
        let collapsed: String =
            run.output.chars().filter(|c| !c.is_whitespace() && *c != '┃').collect();
        assert!(collapsed.contains("TransclusionError:cycledetected"), "{}", run.output);
        for name in ["a.md:line1", "b.md:line1"] {
            assert!(collapsed.contains(name), "{args:?}: `{name}`\n{}", run.output);
        }
    }
    assert!(runs(&fixture).is_empty());
}

/// An included file's lifecycle frontmatter keeps its boundaries: it parses,
/// the parent does not run it, and the `ctx` it names is still part of the
/// run's requirements.
#[test]
fn an_included_files_lifecycle_is_scanned_but_not_run() {
    let fixture = repo("ctx-per-run-included-lifecycle");
    doc(
        &fixture,
        "part.md",
        "---\nsuccess:\n    stack:\n        - action:\n              - append_line: [\"runs.log\", \"part staged={{ length(ctx.staged_files) }}\"]\n---\nPART body\n",
    );
    doc(&fixture, "parent.md", "Parent line.\n\n::file ./part.md\n");

    for args in [
        &["compose", "--claude", "--dry-run", "--perf", "parent.md"][..],
        &["compose", "--claude", "--perf", "parent.md"][..],
    ] {
        let run = run(&fixture, args);

        assert_eq!(run.code, Some(0), "{args:?}: {}", run.output);
        assert_eq!(line_with(&run.output, "PART "), "PART body", "{args:?}");
        assert_eq!(volatile_observations(&run.output), "file_changes", "{args:?}: {}", run.output);
    }
    assert!(runs(&fixture).is_empty(), "the parent never runs the partial's lifecycle");
}

/// A sequence's static pre-flight is one discovery run, however many prompt
/// documents it reads: three steps that each name a Git fact observe it once
/// per step run plus once for the whole pre-flight, not once per document.
#[test]
fn sequence_preflight_observes_git_state_once_for_every_prompt_document() {
    let fixture = repo("ctx-per-run-sequence-preflight");
    for name in ["one.md", "two.md", "three.md"] {
        doc(&fixture, name, "Staged: {{ length(ctx.staged_files) }}\n");
    }
    let file = doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n  - name: one\n    prompt: ./one.md\n  - name: two\n    prompt: ./two.md\n  - name: three\n    prompt: ./three.md\n---\n",
    );

    let run = run(&fixture, &["sequence", "--claude", "--perf", &file]);

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(volatile_observations(&run.output), "file_changes (4)", "{}", run.output);
}
