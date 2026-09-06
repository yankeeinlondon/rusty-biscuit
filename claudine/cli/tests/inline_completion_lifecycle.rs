//! Level-1 process coverage for the completion verdict at the lifecycle
//! boundary (fix `2026-09-05-inline-flow-and-validations`, phase 6).
//!
//! Everything here drives the real `inline-compose` / `compose` entry points
//! against a file-aware provider stub, because the contract under test is
//! *ordering*: the verdict runs after the provider and the inline closure, and
//! before `success` or `failure`. A unit test of the evaluator cannot observe
//! which lifecycle event fired, whether the artifact survived, or whether the
//! captured baseline went back.

#![cfg(unix)]

mod common;

use common::wrap::seed_minimal_config;
use common::{InlineAgentStub, strip_ansi, write_executable};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};

/// A document whose schema requires one property the agent must supply.
///
/// `prompt` is `eager`, so the launch gate covers it; `researched_by` is
/// `required` but not eager, which is the half of the split only the completion
/// phase enforces.
const RESEARCH_DOC: &str = concat!(
    "---\n",
    "$schema:\n",
    "  prompt: 'string(required;eager)'\n",
    "  researched_by: 'string(required)'\n",
    "prompt: Research VoIP handsets\n",
    "agent: goose\n",
    "---\n",
    "original body\n",
);

struct Fixture {
    workspace: TempDir,
    bin: PathBuf,
    document: PathBuf,
}

impl Fixture {
    /// A workspace whose active document lives in its own directory, so a test
    /// can make *only* the document's directory unwritable.
    fn new(document_text: &str) -> Self {
        let workspace = tempdir().unwrap();
        let bin = workspace.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let docs = workspace.path().join("docs");
        fs::create_dir_all(&docs).unwrap();
        seed_minimal_config(workspace.path());
        let document = docs.join("doc.md");
        fs::write(&document, document_text).unwrap();
        Self {
            workspace,
            bin,
            document,
        }
    }

    fn path(&self) -> &Path {
        self.workspace.path()
    }

    fn command(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("claudine").unwrap();
        cmd.current_dir(self.path())
            .env("NO_COLOR", "1")
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("HOME", self.path())
            .env("PATH", &self.bin);
        cmd
    }

    /// Run `inline-compose` against the fixture's document.
    fn inline_compose(&self) -> assert_cmd::assert::Assert {
        self.command()
            .args(["inline-compose", self.document.to_str().unwrap()])
            .assert()
    }

    fn document_text(&self) -> String {
        fs::read_to_string(&self.document).unwrap()
    }
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr))
}

fn stdout_of(assert: &assert_cmd::assert::Assert) -> String {
    strip_ansi(&String::from_utf8_lossy(&assert.get_output().stdout))
}

// ---------------------------------------------------------------------------
// AC4 / AC5 / AC9b — the verdict decides the terminal signal
// ---------------------------------------------------------------------------

/// A run that writes the body and every required property exits zero, and the
/// agent's summary reaches the caller without ever reaching the document.
#[test]
fn a_satisfied_inline_run_exits_zero_and_keeps_the_summary_out_of_the_document() {
    let fixture = Fixture::new(RESEARCH_DOC);
    InlineAgentStub::new(&fixture.document)
        .frontmatter_additions("researched_by: goose\n")
        .body("Agent research findings.\n")
        .summary("Summary sentinel for the caller.")
        .install(&fixture.bin, "goose");

    let assert = fixture.inline_compose().success();

    let combined = format!("{}{}", stdout_of(&assert), stderr_of(&assert));
    assert!(
        combined.contains("Summary sentinel for the caller."),
        "the agent's summary must reach the caller; output was:\n{combined}"
    );
    let written = fixture.document_text();
    assert!(written.contains("Agent research findings."), "{written}");
    assert!(written.contains("researched_by: goose"), "{written}");
    assert!(written.contains("hash:"), "an accepted run stamps; {written}");
    assert!(
        !written.contains("Summary sentinel for the caller."),
        "the summary must never be written to the document; {written}"
    );
}

/// AC4 + AC9b: a required property the agent never set fails the run, names the
/// property, and *keeps* the validly written artifact.
#[test]
fn a_missing_completion_property_fails_the_run_and_keeps_the_written_artifact() {
    let fixture = Fixture::new(RESEARCH_DOC);
    InlineAgentStub::new(&fixture.document)
        .body("Agent research findings.\n")
        .summary("Wrote the body but not the metadata.")
        .install(&fixture.bin, "goose");

    let assert = fixture.inline_compose().failure();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("researched_by"),
        "the status block must name the unsatisfied property:\n{stderr}"
    );
    assert!(
        stderr.contains("does not satisfy its `$schema`"),
        "the completion-schema block must render:\n{stderr}"
    );
    let written = fixture.document_text();
    assert!(
        written.contains("Agent research findings."),
        "a schema failure must keep the artifact; {written}"
    );
    assert!(
        written.contains("hash:"),
        "the kept artifact stays stamped and coherent; {written}"
    );
}

/// A present-but-wrong value is reported as a type mismatch, not as absence.
#[test]
fn a_wrong_typed_completion_property_reports_the_type_mismatch() {
    let fixture = Fixture::new(RESEARCH_DOC);
    InlineAgentStub::new(&fixture.document)
        .frontmatter_additions("researched_by: 42\n")
        .body("Agent research findings.\n")
        .install(&fixture.bin, "goose");

    let assert = fixture.inline_compose().failure();

    let stderr = stderr_of(&assert);
    let normalized = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized.contains("researched_by") && normalized.contains("string"),
        "the block must name the property and its declared type:\n{stderr}"
    );
}

/// A `compose` run whose `start` effect leaves a required property invalid
/// enters `failure` before `success` can fire (AC9). Direct compose writes
/// nothing, so the source is untouched either way.
#[test]
fn a_direct_compose_run_fails_completion_when_a_start_effect_invalidates_a_property() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  researched_by: 'string(required)'\n",
        "researched_by: initial\n",
        "agent: goose\n",
        "start:\n",
        "  stack:\n",
        "    - action: {set: [researched_by, '{{ 42 }}']}\n",
        "success:\n",
        "  info: 'SUCCESS-SENTINEL'\n",
        "failure:\n",
        "  info: 'FAILURE code={{ err.code }}'\n",
        "---\n",
        "Body.\n",
    );
    let fixture = Fixture::new(source);
    write_executable(
        &fixture.bin.join("goose"),
        "#!/bin/sh\nprintf 'done\\n'\nexit 0\n",
    );
    let before = fixture.document_text();

    let assert = fixture
        .command()
        .args(["compose", fixture.document.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("FAILURE code=composition.completion_schema"),
        "`failure` must fire with the typed err:\n{stderr}"
    );
    assert!(
        !stderr.contains("SUCCESS-SENTINEL"),
        "`success` must never fire ahead of a failed verdict:\n{stderr}"
    );
    assert_eq!(
        fixture.document_text(),
        before,
        "direct compose performs no source write"
    );
}

/// The same document with the property left valid exits zero through `success`.
#[test]
fn a_direct_compose_run_with_a_satisfied_property_reaches_success() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  researched_by: 'string(required)'\n",
        "researched_by: initial\n",
        "agent: goose\n",
        "success:\n",
        "  info: 'SUCCESS-SENTINEL'\n",
        "failure:\n",
        "  info: 'FAILURE-SENTINEL'\n",
        "---\n",
        "Body.\n",
    );
    let fixture = Fixture::new(source);
    write_executable(
        &fixture.bin.join("goose"),
        "#!/bin/sh\nprintf 'done\\n'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .args(["compose", fixture.document.to_str().unwrap()])
        .assert()
        .success();

    let stderr = stderr_of(&assert);
    assert!(stderr.contains("SUCCESS-SENTINEL"), "{stderr}");
    assert!(!stderr.contains("FAILURE-SENTINEL"), "{stderr}");
}

// ---------------------------------------------------------------------------
// AC8 / AC17 — rollback coverage
// ---------------------------------------------------------------------------

/// AC17: a non-zero provider exit restores the captured baseline atomically and
/// stamps nothing, even though the agent had already written the file.
#[test]
fn a_provider_exit_one_restores_the_captured_baseline() {
    let fixture = Fixture::new(RESEARCH_DOC);
    InlineAgentStub::new(&fixture.document)
        .frontmatter_additions("researched_by: goose\n")
        .body("Half-written research.\n")
        .exit_code(1)
        .install(&fixture.bin, "goose");

    fixture.inline_compose().failure();

    assert_eq!(
        fixture.document_text(),
        RESEARCH_DOC,
        "a failed provider run must leave the document byte-identical"
    );
}

/// AC8: an interrupted agent that wrote half a document leaves the document
/// byte-identical to the snapshot.
#[test]
fn a_provider_exit_130_restores_the_captured_baseline() {
    let fixture = Fixture::new(RESEARCH_DOC);
    InlineAgentStub::new(&fixture.document)
        .body("Half of a doc\n")
        .exit_code(130)
        .install(&fixture.bin, "goose");

    fixture.inline_compose().failure();

    assert_eq!(fixture.document_text(), RESEARCH_DOC);
}

/// AC17: an empty candidate body is refused before any stamp, and the baseline
/// goes back — the agent's blanked file does not survive.
#[test]
fn an_empty_candidate_body_is_refused_and_rolled_back() {
    let fixture = Fixture::new(RESEARCH_DOC);
    InlineAgentStub::new(&fixture.document)
        .body("\n")
        .summary("I emptied it.")
        .install(&fixture.bin, "goose");

    let assert = fixture.inline_compose().failure();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("did not update"),
        "an empty body is reported as a body rejection:\n{stderr}"
    );
    assert_eq!(fixture.document_text(), RESEARCH_DOC);
}

/// AC17: a duplicate owned key is a typed edit failure, and the malformed text
/// the agent left is rolled back rather than persisted.
#[test]
fn a_duplicate_owned_key_is_refused_and_rolled_back() {
    let fixture = Fixture::new(RESEARCH_DOC);
    let duplicated = concat!(
        "---\n",
        "prompt: Research VoIP handsets\n",
        "prompt: A second one\n",
        "---\n",
        "Agent body.\n",
    );
    write_executable(
        &fixture.bin.join("goose"),
        &format!(
            "#!/bin/sh\nprintf '%s' {doc} > {target}\nprintf 'wrote it\\n'\nexit 0\n",
            doc = common::sh_quote(duplicated),
            target = common::sh_quote(&fixture.document.display().to_string()),
        ),
    );

    let assert = fixture.inline_compose().failure();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("could not reconcile the inline document"),
        "a duplicate owned key is a typed edit failure:\n{stderr}"
    );
    assert_eq!(
        fixture.document_text(),
        RESEARCH_DOC,
        "a refused reconciliation restores the baseline"
    );
}

/// When rollback itself fails, the initiating diagnostic is retained and a
/// typed rollback cause naming the path is rendered beside it. Nothing claims
/// the document was restored.
#[test]
fn a_failed_rollback_reports_the_typed_cause_and_keeps_the_initiating_error() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new(RESEARCH_DOC);
    let docs_dir = fixture.document.parent().unwrap().to_path_buf();
    write_executable(
        &fixture.bin.join("goose"),
        "#!/bin/sh\nprintf 'I could not finish.\\n'\nexit 1\n",
    );
    // The restoring write is atomic — it creates a sibling temp file and
    // renames — so sealing the document's own directory is what makes rollback
    // fail while leaving the document readable.
    fs::set_permissions(&docs_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let assert = fixture.inline_compose().failure();

    // Restore write access before any assertion can abort the test and leave an
    // undeletable temp directory behind.
    fs::set_permissions(&docs_dir, fs::Permissions::from_mode(0o755)).unwrap();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("could not restore"),
        "a failed rollback must be reported with its own typed cause:\n{stderr}"
    );
    assert!(
        stderr.contains("doc.md"),
        "the rollback cause names the document:\n{stderr}"
    );
    assert!(
        stderr.contains("exited with error"),
        "the initiating diagnostic is retained, not replaced:\n{stderr}"
    );
    assert!(
        !stderr.to_lowercase().contains("restored the document"),
        "a failed rollback must never claim success:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// AC18 — recovery across a failed verdict
// ---------------------------------------------------------------------------

/// The two-run stub used by the recovery tests: run 1 and run 2 each write a
/// document supplied through the environment, and every run records the text it
/// found on disk before touching it.
///
/// Shell builtins only — an inline fixture's `PATH` is the stub directory
/// alone, so `cat` and `cp` are not available to it.
fn write_two_run_agent(fixture: &Fixture) {
    write_executable(
        &fixture.bin.join("goose"),
        &format!(
            "#!/bin/sh\n\
             CLAUDINE_RUN=0\n\
             if [ -f \"$CLAUDINE_RUNS\" ]; then read -r CLAUDINE_RUN < \"$CLAUDINE_RUNS\"; fi\n\
             CLAUDINE_RUN=$((CLAUDINE_RUN + 1))\n\
             printf '%s' \"$CLAUDINE_RUN\" > \"$CLAUDINE_RUNS\"\n\
             CLAUDINE_SEEN_TEXT=''\n\
             while IFS= read -r CLAUDINE_LINE || [ -n \"$CLAUDINE_LINE\" ]; do\n\
             \x20 CLAUDINE_SEEN_TEXT=\"$CLAUDINE_SEEN_TEXT$CLAUDINE_LINE\n\"\n\
             done < {target}\n\
             printf '%s' \"$CLAUDINE_SEEN_TEXT\" > \"$CLAUDINE_SEEN.$CLAUDINE_RUN\"\n\
             if [ \"$CLAUDINE_RUN\" -eq 1 ]; then\n\
             \x20 printf '%s' \"$CLAUDINE_DOC_ONE\" > {target}\n\
             \x20 printf 'first pass\\n'\n\
             \x20 exit ${{CLAUDINE_EXIT_ONE:-0}}\n\
             fi\n\
             printf '%s' \"$CLAUDINE_DOC_TWO\" > {target}\n\
             printf 'second pass\\n'\n\
             exit 0\n",
            target = common::sh_quote(&fixture.document.display().to_string()),
        ),
    );
}

const RETRYING_DOC: &str = concat!(
    "---\n",
    "$schema:\n",
    "  prompt: 'string(required;eager)'\n",
    "  researched_by: 'string(required)'\n",
    "prompt: Research VoIP handsets\n",
    "agent: goose\n",
    "failure:\n",
    "  stack:\n",
    "    - action:\n",
    "        - action: retry\n",
    "          max_attempts: 2\n",
    "---\n",
    "original body\n",
);

/// AC18: a completion-schema failure recovers through `retry`, and the metadata-
/// only repair is *not* refused as an unchanged body — the operation already
/// produced one on the first attempt.
#[test]
fn a_metadata_only_retry_recovers_a_completion_schema_failure() {
    let fixture = Fixture::new(RETRYING_DOC);
    write_two_run_agent(&fixture);

    // Run 1 writes a real body but no `researched_by`; run 2 repairs only the
    // frontmatter and puts the *baseline* body back, which is exactly the case
    // that would be refused as unchanged without carried evidence.
    let assert = fixture
        .command()
        .env("CLAUDINE_RUNS", fixture.path().join("runs.txt"))
        .env("CLAUDINE_SEEN", fixture.path().join("seen"))
        .env(
            "CLAUDINE_DOC_ONE",
            "---\nprompt: Research VoIP handsets\nagent: goose\n---\nAgent findings.\n",
        )
        .env(
            "CLAUDINE_DOC_TWO",
            "---\nprompt: Research VoIP handsets\nagent: goose\nresearched_by: goose\n---\noriginal body\n",
        )
        .args(["inline-compose", fixture.document.to_str().unwrap()])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(fixture.path().join("runs.txt")).unwrap(),
        "2",
        "the failed verdict must have driven exactly one retry; stderr:\n{}",
        stderr_of(&assert)
    );
    let written = fixture.document_text();
    assert!(written.contains("researched_by: goose"), "{written}");
    assert!(written.contains("hash:"), "the recovery stamps; {written}");
}

/// AC18: a *provider* failure rolls back first, so the retry attempt reads the
/// captured baseline rather than the failed attempt's leftovers.
#[test]
fn a_provider_failure_rolls_back_before_the_retry_reads_the_document() {
    let fixture = Fixture::new(RETRYING_DOC);
    write_two_run_agent(&fixture);

    fixture
        .command()
        .env("CLAUDINE_RUNS", fixture.path().join("runs.txt"))
        .env("CLAUDINE_SEEN", fixture.path().join("seen"))
        .env("CLAUDINE_EXIT_ONE", "0")
        .env(
            "CLAUDINE_DOC_ONE",
            "---\nprompt: Research VoIP handsets\nagent: goose\n---\nGarbage from a failed pass.\n",
        )
        .env(
            "CLAUDINE_DOC_TWO",
            "---\nprompt: Research VoIP handsets\nagent: goose\nresearched_by: goose\n---\nRepaired body.\n",
        )
        .args(["inline-compose", fixture.document.to_str().unwrap()])
        .assert()
        .success();

    let seen_by_second = fs::read_to_string(fixture.path().join("seen.2")).unwrap();
    assert!(
        seen_by_second.contains("Garbage from a failed pass."),
        "a schema failure keeps the artifact, so the retry sees it:\n{seen_by_second}"
    );
}

/// The same shape, but the first attempt's provider *fails*: the guard restores
/// the baseline before recovery, so the second attempt starts from the authored
/// document rather than from the abandoned write.
#[test]
fn a_failed_provider_attempt_hands_the_retry_a_restored_document() {
    let fixture = Fixture::new(RETRYING_DOC);
    write_two_run_agent(&fixture);

    fixture
        .command()
        .env("CLAUDINE_RUNS", fixture.path().join("runs.txt"))
        .env("CLAUDINE_SEEN", fixture.path().join("seen"))
        .env("CLAUDINE_EXIT_ONE", "1")
        .env(
            "CLAUDINE_DOC_ONE",
            "---\nprompt: Research VoIP handsets\nagent: goose\n---\nAbandoned partial write.\n",
        )
        .env(
            "CLAUDINE_DOC_TWO",
            "---\nprompt: Research VoIP handsets\nagent: goose\nresearched_by: goose\n---\nRepaired body.\n",
        )
        .args(["inline-compose", fixture.document.to_str().unwrap()])
        .assert()
        .success();

    let seen_by_second = fs::read_to_string(fixture.path().join("seen.2")).unwrap();
    assert_eq!(
        seen_by_second, RETRYING_DOC,
        "a provider failure rolls back, so recovery starts from the baseline"
    );
}

// ---------------------------------------------------------------------------
// AC9 — lifecycle ordering for the inline verdict
// ---------------------------------------------------------------------------

/// A failed inline verdict fires `failure` with the typed `err`, never
/// `success`, and still runs `finalize`.
#[test]
fn a_failed_inline_verdict_fires_failure_with_the_typed_err_then_finalize() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  prompt: 'string(required;eager)'\n",
        "  researched_by: 'string(required)'\n",
        "prompt: Research VoIP handsets\n",
        "agent: goose\n",
        "success:\n",
        "  info: 'SUCCESS-SENTINEL'\n",
        "failure:\n",
        "  info: 'FAILURE code={{ err.code }} category={{ err.category }}'\n",
        "finalize:\n",
        "  info: 'FINALIZE-SENTINEL'\n",
        "---\n",
        "original body\n",
    );
    let fixture = Fixture::new(source);
    InlineAgentStub::new(&fixture.document)
        .body("Agent research findings.\n")
        .install(&fixture.bin, "goose");

    let assert = fixture.inline_compose().failure();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("FAILURE code=composition.completion_schema category=composition"),
        "the failure stack must see the typed completion err:\n{stderr}"
    );
    assert!(!stderr.contains("SUCCESS-SENTINEL"), "{stderr}");
    assert!(stderr.contains("FINALIZE-SENTINEL"), "{stderr}");
    let failure_at = stderr.find("FAILURE code=").unwrap();
    let finalize_at = stderr.find("FINALIZE-SENTINEL").unwrap();
    assert!(
        failure_at < finalize_at,
        "`failure` precedes `finalize`:\n{stderr}"
    );
}

/// A refused body carries `composition.body_unchanged` rather than the schema
/// code, so a `failure` stack can tell the two verdict halves apart.
#[test]
fn a_refused_body_fires_failure_with_the_body_unchanged_code() {
    let source = concat!(
        "---\n",
        "prompt: Research VoIP handsets\n",
        "agent: goose\n",
        "failure:\n",
        "  info: 'FAILURE code={{ err.code }} reason={{ err.detail.reason }}'\n",
        "---\n",
        "original body\n",
    );
    let fixture = Fixture::new(source);
    write_executable(
        &fixture.bin.join("goose"),
        "#!/bin/sh\nprintf 'I read it and it looked fine.\\n'\nexit 0\n",
    );

    let assert = fixture.inline_compose().failure();

    let stderr = stderr_of(&assert);
    assert!(
        stderr.contains("FAILURE code=composition.body_unchanged reason=unchanged"),
        "the body half projects its own code and reason:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// AC10 — one verdict per composition, in sequences and loops
// ---------------------------------------------------------------------------

/// A sequence applies the verdict to every step: step 2 fails completion and
/// `fail_fast` stops the run before step 3 launches.
#[test]
fn a_sequence_applies_the_verdict_per_step_and_fail_fast_stops_at_step_two() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  marker: 'string(required)'\n",
        "marker: seed\n",
        "agent: goose\n",
        "sequence:\n",
        "  - alpha\n",
        "  - beta\n",
        "  - gamma\n",
        "start:\n",
        "  stack:\n",
        // `state` is the per-step object, so the branch reads its id.
        "    - when: \"state.id == 'beta'\"\n",
        "      action: {set: [marker, '{{ 42 }}']}\n",
        "---\n",
        "Step {{ state }}.\n",
    );
    let fixture = Fixture::new(source);
    let counter = fixture.path().join("calls.txt");
    write_executable(
        &fixture.bin.join("goose"),
        "#!/bin/sh\nprintf 'x' >> \"$CLAUDINE_CALLS\"\nprintf 'done\\n'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .env("CLAUDINE_CALLS", &counter)
        .args(["sequence", fixture.document.to_str().unwrap()])
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(&counter).unwrap_or_default().len(),
        2,
        "step 3 must never launch; stderr:\n{}",
        stderr_of(&assert)
    );
    let stderr = stderr_of(&assert);
    // The step summary is word-wrapped, so compare on collapsed whitespace.
    let normalized = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized.contains("does not satisfy its schema at completion"),
        "step 2 must fail on the completion verdict, not on launch:\n{stderr}"
    );
}

/// A `--loop` run is judged once per iteration: iteration 2 fails completion
/// and the loop stops rather than running a third.
#[test]
fn a_loop_applies_the_verdict_per_iteration() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  marker: 'string(required)'\n",
        "marker: seed\n",
        "counter: 0\n",
        "agent: goose\n",
        "loop:\n",
        "  while: \"counter < 2\"\n",
        "  actions:\n",
        "    - \"increment(counter)\"\n",
        "start:\n",
        "  stack:\n",
        "    - when: \"counter == 1\"\n",
        "      action: {set: [marker, '{{ 42 }}']}\n",
        "---\n",
        "Iteration {{ counter }}.\n",
    );
    let fixture = Fixture::new(source);
    let counter = fixture.path().join("calls.txt");
    write_executable(
        &fixture.bin.join("goose"),
        "#!/bin/sh\nprintf 'x' >> \"$CLAUDINE_CALLS\"\nprintf 'done\\n'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .env("CLAUDINE_CALLS", &counter)
        .args(["compose", fixture.document.to_str().unwrap()])
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(&counter).unwrap_or_default().len(),
        2,
        "the third iteration must not run; stderr:\n{}",
        stderr_of(&assert)
    );
}

/// AC18: a `proxy` out of a failed verdict discards the source guard and the
/// target captures its own baseline — so the target's own body change is what
/// the target's verdict judges, and the source keeps the artifact it wrote.
#[test]
fn a_proxy_out_of_a_failed_verdict_captures_the_targets_own_baseline() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  prompt: 'string(required;eager)'\n",
        "  researched_by: 'string(required)'\n",
        "prompt: Research VoIP handsets\n",
        "agent: goose\n",
        "failure:\n",
        "  stack:\n",
        "    - action: {action: proxy, target: './target.md'}\n",
        "---\n",
        "original body\n",
    );
    let fixture = Fixture::new(source);
    let target = fixture.document.parent().unwrap().join("target.md");
    fs::write(
        &target,
        "---\nprompt: Write the follow-up\nagent: goose\n---\ntarget body\n",
    )
    .unwrap();

    // One stub for both documents: it learns which file it owns from the
    // delivered prompt header, exactly as a real agent does.
    write_executable(
        &fixture.bin.join("goose"),
        &format!(
            "#!/bin/sh\n{doc_from_prompt}\
             CLAUDINE_ADD=''\n\
             CLAUDINE_BODY='Agent wrote this.\n'\n\
             {rewrite}\
             printf 'wrote %s\\n' \"$CLAUDINE_DOC\"\n\
             exit 0\n",
            doc_from_prompt = common::INLINE_DOC_FROM_PROMPT,
            rewrite = common::INLINE_BODY_REWRITE,
        ),
    );

    fixture.inline_compose().success();

    let source_after = fixture.document_text();
    assert!(
        source_after.contains("Agent wrote this."),
        "the source keeps the artifact it wrote before its verdict failed:\n{source_after}"
    );
    let target_after = fs::read_to_string(&target).unwrap();
    assert!(
        target_after.contains("Agent wrote this."),
        "the proxied target is judged against its own baseline:\n{target_after}"
    );
    assert!(
        target_after.contains("hash:"),
        "the target's own closure stamped it:\n{target_after}"
    );
}

/// AC14: a `proxy.with` overlay is a transient effective-frontmatter input, so
/// it satisfies the *target's* completion schema without ever being written to
/// the target file — the same rule `--set` and sequence state follow.
#[test]
fn a_proxy_with_overlay_satisfies_the_targets_completion_schema() {
    let source = concat!(
        "---\n",
        "prompt: Route to the researcher\n",
        "agent: goose\n",
        "initialize:\n",
        "  stack:\n",
        "    - action: {action: proxy, target: './target.md', with: {researched_by: from-the-router}}\n",
        "---\n",
        "router body\n",
    );
    let fixture = Fixture::new(source);
    let target = fixture.document.parent().unwrap().join("target.md");
    let target_source = concat!(
        "---\n",
        "$schema:\n",
        "  prompt: 'string(required;eager)'\n",
        "  researched_by: 'string(required)'\n",
        "prompt: Write the research\n",
        "agent: goose\n",
        "---\n",
        "target body\n",
    );
    fs::write(&target, target_source).unwrap();

    // The agent writes only the body — `researched_by` arrives from the
    // overlay alone.
    write_executable(
        &fixture.bin.join("goose"),
        &format!(
            "#!/bin/sh\n{doc_from_prompt}\
             CLAUDINE_ADD=''\n\
             CLAUDINE_BODY='Agent wrote this.\n'\n\
             {rewrite}\
             printf 'wrote %s\\n' \"$CLAUDINE_DOC\"\n\
             exit 0\n",
            doc_from_prompt = common::INLINE_DOC_FROM_PROMPT,
            rewrite = common::INLINE_BODY_REWRITE,
        ),
    );

    fixture.inline_compose().success();

    let target_after = fs::read_to_string(&target).unwrap();
    assert!(
        target_after.contains("Agent wrote this."),
        "the target's body is the deliverable:\n{target_after}"
    );
    assert!(
        !target_after.contains("from-the-router"),
        "a transient overlay must never be persisted to the target:\n{target_after}"
    );

    // Without the overlay the same target fails completion — proving the
    // overlay, not a missing check, is what made the run succeed.
    fs::write(&target, target_source).unwrap();
    let without = fixture
        .command()
        .args(["inline-compose", target.to_str().unwrap()])
        .assert()
        .failure();
    assert!(
        stderr_of(&without).contains("researched_by"),
        "stderr:\n{}",
        stderr_of(&without)
    );
}

/// AC9a / AC13, inline half: a required-but-not-eager property whose authored
/// expression legitimately resolves to `null` never prompts and never blocks
/// launch. The agent runs, and the gap is reported by the completion verdict.
#[test]
fn a_conditional_required_property_never_blocks_an_inline_launch() {
    let source = concat!(
        "---\n",
        "$schema:\n",
        "  prompt: 'string(required;eager)'\n",
        "  spec: 'string'\n",
        "  foo: 'string(required)'\n",
        "prompt: Research VoIP handsets\n",
        "foo: \"{{ spec ? spec + '-derived' : null }}\"\n",
        "agent: goose\n",
        "---\n",
        "original body\n",
    );
    let fixture = Fixture::new(source);
    let calls = fixture.path().join("calls.txt");
    write_executable(
        &fixture.bin.join("goose"),
        &format!(
            "#!/bin/sh\nprintf 'x' >> \"$CLAUDINE_CALLS\"\n{doc_from_prompt}\
             CLAUDINE_ADD=''\n\
             CLAUDINE_BODY='Agent wrote this.\n'\n\
             {rewrite}\
             printf 'done\\n'\n\
             exit 0\n",
            doc_from_prompt = common::INLINE_DOC_FROM_PROMPT,
            rewrite = common::INLINE_BODY_REWRITE,
        ),
    );

    let assert = fixture
        .command()
        .env("CLAUDINE_CALLS", &calls)
        .args(["inline-compose", fixture.document.to_str().unwrap()])
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(&calls).unwrap_or_default().len(),
        1,
        "inline mode must launch rather than prompt or refuse; stderr:\n{}",
        stderr_of(&assert)
    );
    let stderr = stderr_of(&assert);
    assert!(stderr.contains("foo"), "the verdict must name the gap:\n{stderr}");
    let written = fixture.document_text();
    assert!(
        written.contains("Agent wrote this."),
        "the artifact survives a completion-schema failure:\n{written}"
    );

    // Supplying `spec` lets the document's own expression fill `foo`, and the
    // identical run succeeds.
    fs::write(&fixture.document, source).unwrap();
    fixture
        .command()
        .env("CLAUDINE_CALLS", &calls)
        .args([
            "inline-compose",
            fixture.document.to_str().unwrap(),
            "spec=alpha",
        ])
        .assert()
        .success();
}
