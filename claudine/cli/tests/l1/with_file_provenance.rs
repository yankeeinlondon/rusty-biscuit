//! Level 1 coverage for the one provenance rule every `with:` follows: a
//! relative file value resolves from the file that authored it.
//!
//! Every row runs the real `claudine` binary against a stub provider. The
//! fixture puts a `notes.md` with a distinct `marker` in each directory a
//! value could resolve from, so the target's rendered `ROUTE=<marker>` names
//! the directory that won:
//!
//! - `prompts/router/` — the authoring document of a `proxy`, `retry`, or `prep`
//! - `prompts/target/` — the target document, the old `proxy.with` rule
//! - the launch directory — where a caller setter resolves from

use crate::common;

use common::provenance::{install_goose, run_compose};
use common::{CliProcessFixture, strip_ansi, write};
#[cfg(unix)]
use common::write_executable;

/// The fixture with a stub provider and a `notes.md` marker in each candidate
/// directory.
fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.initialize_repository();
    fixture.seed_user_config();
    install_goose(&fixture);
    for (folder, marker) in [("prompts/router", "authoring"), ("prompts/target", "target"), ("", "launch")] {
        write(&fixture.cwd().join(folder).join("notes.md"), &format!("---\nmarker: {marker}\n---\n"));
    }
    fixture
}

/// The shared target: `notes` is a file, and the body names its marker.
fn write_target(fixture: &CliProcessFixture) {
    write(
        &fixture.cwd().join("prompts/target/target.md"),
        "---\n$schema:\n  notes: 'file(required)'\n  choice: string\nmarker: \"{{ frontmatter(notes, 'marker') }}\"\n---\nROUTE={{ marker }}\n",
    );
}

/// A target that renders `notes` itself, under `schema`, so its raw or bound
/// value is visible.
fn write_raw_target(fixture: &CliProcessFixture, schema: &str) {
    write(
        &fixture.cwd().join("prompts/target/raw.md"),
        &format!("---\n$schema:\n  notes: {schema}\n---\nNOTES={{{{ notes }}}}\n"),
    );
}

/// The `NOTES=` value of the prompt the provider received, with separators
/// made portable. Read from the stub's record, since the terminal wraps a
/// long path.
fn launched_notes(fixture: &CliProcessFixture, stderr: &str) -> String {
    let prompt = std::fs::read_to_string(fixture.home().join("provider-prompt"))
        .unwrap_or_else(|error| panic!("the provider did not launch ({error}); stderr:\n{stderr}"));
    prompt
        .lines()
        // The stub records argv joined by spaces; the prompt is followed by
        // the system-prompt flag.
        .find_map(|line| line.split_once("NOTES=").map(|(_, value)| value.split(" --system").next().unwrap_or(value).trim().to_string()))
        .unwrap_or_else(|| panic!("no NOTES= line; prompt:\n{prompt}"))
        .replace('\\', "/")
}

/// Run `claudine compose --goose document` from the fixture root, expecting
/// failure; returns stderr.
fn run_compose_failing(fixture: &CliProcessFixture, document: &std::path::Path) -> String {
    let output = fixture
        .command()
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["compose", "--goose", document.to_str().unwrap()])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(!fixture.audio_spool().exists(), "provenance tests must not publish audio");
    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

/// The provider stub records each launch here; its absence means nothing
/// launched.
fn provider_launched(fixture: &CliProcessFixture) -> bool {
    fixture.home().join("provider-prompt").exists()
}

#[test]
fn a_proxy_with_file_resolves_from_the_router_not_the_target_or_the_launch_directory() {
    let fixture = fixture("with-provenance-proxy");
    write_target(&fixture);
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/target.md', with: {notes: ./notes.md}}}\n---\nRouter.\n",
    );

    // With no caller setters Darkmatter would otherwise treat the overlay as
    // launch-area input; with an unrelated setter, as target-authored. Both
    // must give way to the authoring file.
    for setters in [&[][..], &["choice=caller"][..]] {
        let output = run_compose(&fixture, fixture.cwd(), &router, setters);
        assert!(output.contains("ROUTE=authoring"), "setters {setters:?}; stderr:\n{output}");
    }
}

#[test]
fn a_caller_setter_still_resolves_from_the_launch_directory_and_outranks_proxy_with() {
    let fixture = fixture("with-provenance-caller-wins");
    write_target(&fixture);
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/target.md', with: {notes: ./notes.md}}}\n---\nRouter.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &router, &["notes=notes.md"]);

    assert!(output.contains("ROUTE=launch"), "stderr:\n{output}");
}

/// A `retry` is authored by the document it re-enters, so its `with` resolves
/// from that document's directory, not the launch directory.
#[cfg(unix)]
#[test]
fn a_retry_with_file_resolves_from_the_retried_document() {
    let fixture = fixture("with-provenance-retry");
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nif [ ! -f \"$HOME/provider-attempted\" ]; then\n  : > \"$HOME/provider-attempted\"\n  exit 1\nfi\nprintf 'provider reached\\n'\n",
    );
    let document = fixture.cwd().join("prompts/router/retry.md");
    write(
        &document,
        "---\n$schema:\n  notes: file\nmarker: \"{{ notes ? frontmatter(notes, 'marker') : 'none' }}\"\nstart:\n    - {append_line: ['events.log', 'attempt={{ marker }}']}\nfailure:\n    - {retry: {max_attempts: 1, with: {notes: ./notes.md}}}\n---\nRETRY={{ marker }}\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &document, &[]);

    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
    assert!(
        events.contains("attempt=none") && events.contains("attempt=authoring"),
        "events:\n{events}\nstderr:\n{output}"
    );
}

/// A `prep` target receives `with` as caller values, but those values were
/// authored by the document whose `prep` fired, not typed at the launch.
#[test]
fn a_prep_with_file_resolves_from_the_calling_document() {
    let fixture = fixture("with-provenance-prep");
    write_target(&fixture);
    let caller = fixture.cwd().join("prompts/router/caller.md");
    write(
        &caller,
        "---\ninitialize:\n    - {prep: {target: '../target/target.md', with: {notes: ./notes.md}}}\n---\nCaller.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &caller, &[]);

    assert!(output.contains("ROUTE=authoring"), "stderr:\n{output}");
}

/// A task written in a `kind: task` file resolves its `with` from that file.
#[test]
fn a_task_file_with_resolves_from_the_task_file_in_another_directory() {
    let fixture = fixture("with-provenance-task-file");
    write_target(&fixture);
    write(&fixture.cwd().join("tasks/notes.md"), "---\nmarker: task-file\n---\n");
    write(
        &fixture.cwd().join("tasks/task.yaml"),
        "kind: task\nprompt: ../prompts/target/target.md\nwith:\n  notes: ./notes.md\n",
    );
    let sequence = fixture.cwd().join("sequence.md");
    write(&sequence, "---\nsequence:\n  - name: one\n    task: tasks/task.yaml\n---\nSequence.\n");

    let assertion = fixture
        .command()
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .assert()
        .success();

    let prompt = std::fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    assert!(prompt.contains("ROUTE=task-file"), "prompt:\n{prompt}\nstderr:\n{stderr}");
}

/// A lazy file value that does not exist binds to its first candidate, from
/// the authoring file.
#[test]
fn a_lazy_missing_file_binds_from_the_authoring_file() {
    let fixture = fixture("with-provenance-lazy-missing");
    write_raw_target(&fixture, "file");
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/raw.md', with: {notes: ./missing.md}}}\n---\nRouter.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &router, &[]);

    let notes = launched_notes(&fixture, &output);
    assert!(notes.ends_with("prompts/router/missing.md"), "{notes}; stderr:\n{output}");
}

/// An eager file value resolves from the authoring file before the target
/// launches.
#[test]
fn an_eager_file_resolves_from_the_authoring_file() {
    let fixture = fixture("with-provenance-eager");
    write_raw_target(&fixture, "'file(required;eager)'");
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/raw.md', with: {notes: ./notes.md}}}\n---\nRouter.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &router, &[]);

    let notes = launched_notes(&fixture, &output);
    assert!(notes.ends_with("prompts/router/notes.md"), "{notes}; stderr:\n{output}");
}

/// An eager file value that does not exist fails before the target launches,
/// naming the path the authoring file selects.
#[test]
fn an_eager_missing_file_fails_from_the_authoring_file_and_launches_nothing() {
    let fixture = fixture("with-provenance-eager-missing");
    write_raw_target(&fixture, "'file(required;eager)'");
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/raw.md', with: {notes: ./missing.md}}}\n---\nRouter.\n",
    );

    let stderr = run_compose_failing(&fixture, &router);

    let portable = stderr.replace('\\', "/");
    assert!(portable.contains("missing.md"), "stderr:\n{stderr}");
    assert!(!portable.contains("prompts/target/missing.md"), "the target's directory is not the origin; stderr:\n{stderr}");
    assert!(!provider_launched(&fixture), "nothing launches after the failure");
}

/// A lazy remote value stays the URL, and nothing is fetched.
#[test]
fn a_lazy_remote_value_stays_a_url() {
    let fixture = fixture("with-provenance-remote");
    write_raw_target(&fixture, "file");
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/raw.md', with: {notes: 'https://example.invalid/notes.md'}}}\n---\nRouter.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &router, &[]);

    assert_eq!(launched_notes(&fixture, &output), "https://example.invalid/notes.md", "stderr:\n{output}");
}

/// A schema union with both a file and a string arm cannot say which the
/// value is, so the value stays as authored.
#[test]
fn an_ambiguous_file_or_string_value_stays_raw() {
    let fixture = fixture("with-provenance-ambiguous");
    write_raw_target(&fixture, "[file, string]");
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/raw.md', with: {notes: ./notes.md}}}\n---\nRouter.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &router, &[]);

    assert_eq!(launched_notes(&fixture, &output), "./notes.md", "stderr:\n{output}");
}

/// A `with` value that copies the router's own, already anchored file value
/// keeps that file's identity.
#[test]
fn a_forwarded_anchored_value_keeps_its_identity() {
    let fixture = fixture("with-provenance-forwarded");
    write_target(&fixture);
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\n$schema:\n  notes: 'file(required)'\nnotes: ./notes.md\ninitialize:\n    - {proxy: {target: '../target/target.md', with: {notes: '{{ notes }}'}}}\n---\nRouter.\n",
    );

    let output = run_compose(&fixture, fixture.cwd(), &router, &[]);

    assert!(output.contains("ROUTE=authoring"), "stderr:\n{output}");
}

/// A chained handoff whose middle document types the value differently: a
/// string there is a newly produced relative string at the next hop, so it
/// takes the middle document's origin; a file there was anchored at the
/// first hop and keeps that identity.
#[test]
fn a_chained_handoff_follows_each_hops_schema() {
    let fixture = fixture("with-provenance-chained");
    write_target(&fixture);
    write(&fixture.cwd().join("prompts/middle/notes.md"), "---\nmarker: middle\n---\n");
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../middle/middle.md', with: {notes: ./notes.md}}}\n---\nRouter.\n",
    );

    for (middle_schema, expected) in [("string", "ROUTE=middle"), ("'file(required)'", "ROUTE=authoring")] {
        write(
            &fixture.cwd().join("prompts/middle/middle.md"),
            &format!("---\n$schema:\n  notes: {middle_schema}\ninitialize:\n    - {{proxy: {{target: '../target/target.md', with: {{notes: '{{{{ notes }}}}'}}}}}}\n---\nMiddle.\n"),
        );

        let output = run_compose(&fixture, fixture.cwd(), &router, &[]);

        assert!(output.contains(expected), "middle `notes: {middle_schema}`; stderr:\n{output}");
    }
}

/// A `with` mapping evaluates atomically: one failing value installs no
/// overlay and launches no target.
#[test]
fn an_evaluation_error_installs_no_partial_overlay() {
    let fixture = fixture("with-provenance-atomic");
    write_target(&fixture);
    let router = fixture.cwd().join("prompts/router/router.md");
    write(
        &router,
        "---\ninitialize:\n    - {proxy: {target: '../target/target.md', with: {notes: ./notes.md, broken: '{{ no_such_fn() }}'}}}\n---\nRouter.\n",
    );

    let stderr = run_compose_failing(&fixture, &router);

    assert!(stderr.contains("could not be resolved for the `proxy`"), "stderr:\n{stderr}");
    assert!(stderr.contains("no_such_fn"), "stderr:\n{stderr}");
    assert!(!stderr.contains("ROUTE="), "the target was never composed; stderr:\n{stderr}");
    assert!(!provider_launched(&fixture), "nothing launches after the failure");
}

/// A `prep.with` evaluates atomically too: one failing value calls no target,
/// and the caller's event fails before anything launches.
#[test]
fn a_prep_evaluation_error_calls_no_target() {
    let fixture = fixture("with-provenance-prep-atomic");
    write_target(&fixture);
    let caller = fixture.cwd().join("prompts/router/caller.md");
    write(
        &caller,
        "---\ninitialize:\n    - {prep: {target: '../target/target.md', with: {notes: ./notes.md, broken: '{{ no_such_fn() }}'}}}\n---\nCaller.\n",
    );

    let stderr = run_compose_failing(&fixture, &caller);

    assert!(stderr.contains("could not be resolved for the `prep`"), "stderr:\n{stderr}");
    assert!(!provider_launched(&fixture), "neither the target nor the caller launched");
}

/// A sequence task's document that proxies on hands its `with` to the target
/// through the sequence's own handoff loop, with the same origin rule.
#[test]
fn a_proxy_from_a_sequence_task_resolves_from_the_router() {
    let fixture = fixture("with-provenance-sequence-proxy");
    write_target(&fixture);
    write(
        &fixture.cwd().join("prompts/router/router.md"),
        "---\ninitialize:\n    - {proxy: {target: '../target/target.md', with: {notes: ./notes.md}}}\n---\nRouter.\n",
    );
    let sequence = fixture.cwd().join("sequence.md");
    write(&sequence, "---\nsequence:\n  - name: one\n    prompt: prompts/router/router.md\n---\nSequence.\n");

    let assertion = fixture
        .command()
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .assert()
        .success();

    let prompt = std::fs::read_to_string(fixture.home().join("provider-prompt")).unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr));
    assert!(prompt.contains("ROUTE=authoring"), "prompt:\n{prompt}\nstderr:\n{stderr}");
}
