//! TTS pre-warm through the real command flows: which phrases each flow
//! submits, that submission never delays the provider launch, and that every
//! gate prepares nothing.
//!
//! Preparation is observed through the `test-fixtures` recorder: with
//! `CLAUDINE_TEST_PREWARM=record:<path>` each preparation the worker starts
//! appends its text to `<path>` instead of synthesizing speech, and with
//! `block:<path>` it appends and then never finishes. A run that observes the
//! recorder sets `PLAYA_DRY_RUN=0`, so pre-warm is live; every phrase it uses
//! sits behind a guard that never holds or in an event that never fires, so
//! nothing is ever spoken, and each test asserts the audio spool stays absent.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, strip_ansi};

const SEAM: &str = "CLAUDINE_TEST_PREWARM";

/// Where the recorder writes, in the fixture's own workspace.
fn record_path(fixture: &CliProcessFixture) -> PathBuf {
    fixture.workspace_path().join("prewarm.log")
}

/// The texts the recorder saw, in preparation order; empty when the worker
/// never prepared anything.
fn recorded(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn sorted(mut texts: Vec<String>) -> Vec<String> {
    texts.sort();
    texts
}

/// A command with pre-warm live and observed by the recorder in `mode`.
fn observed(fixture: &CliProcessFixture, mode: &str) -> assert_cmd::Command {
    let mut command = fixture.command();
    command
        .env("PLAYA_DRY_RUN", "0")
        .env(SEAM, format!("{mode}:{}", record_path(fixture).display()));
    command
}

fn write_document(fixture: &CliProcessFixture, name: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(name);
    common::write(&path, content);
    path
}

/// Early phrases are prepared with their resolved text, a phrase reading
/// `err` is not, and a guard is never evaluated to decide.
const EARLY_AND_LATE: &str = r#"---
title: Widget
failure:
    - say: "{{ title }} failed"
    - say: "failed: {{ err.msg }}"
    - when: "false"
      then:
          - say: "nested {{ fm['title'] }}"
---
Build the widget.
"#;

#[test]
fn compose_prepares_every_early_phrase_once_with_its_resolved_text() {
    let fixture = CliProcessFixture::named("prewarm-compose");
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    let document = write_document(&fixture, "doc.md", EARLY_AND_LATE);

    let output = observed(&fixture, "record")
        .args(["compose", "--goose", document.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    assert_eq!(
        sorted(recorded(&record_path(&fixture))),
        ["Widget failed", "nested Widget"],
        "{stderr}"
    );
    assert!(!fixture.audio_spool().exists(), "nothing was spoken");
}

/// A preparation that never finishes neither delays the launch nor holds the
/// command open, and pre-warm changes nothing about the outcome.
#[test]
fn a_blocked_preparation_does_not_delay_the_provider_launch() {
    let fixture = CliProcessFixture::named("prewarm-blocked");
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    let document = write_document(&fixture, "doc.md", EARLY_AND_LATE);

    let output = observed(&fixture, "block")
        .args(["compose", "--goose", document.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    assert_eq!(
        recorded(&record_path(&fixture)).len(),
        1,
        "one preparation started and never finished; the rest stayed queued\n{stderr}"
    );
    assert!(!stderr.contains("pre-warm"), "nothing about pre-warm reaches the terminal:\n{stderr}");
}

#[test]
fn dry_run_prepares_nothing() {
    let fixture = CliProcessFixture::named("prewarm-dry-run");
    common::write_dry_run_provider_stub(fixture.bin_dir(), "goose");
    let document = write_document(&fixture, "doc.md", EARLY_AND_LATE);

    let output = observed(&fixture, "record")
        .args(["compose", "--goose", "--dry-run", document.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!record_path(&fixture).exists(), "`--dry-run` starts no preparation");
}

#[test]
fn prewarm_false_in_config_prepares_nothing_and_still_launches() {
    let fixture = CliProcessFixture::named("prewarm-setting-off");
    common::write(
        &fixture.home().join(".claudine/config.json"),
        r#"{ "tts": { "prewarm": false } }"#,
    );
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    let document = write_document(&fixture, "doc.md", EARLY_AND_LATE);

    let output = observed(&fixture, "record")
        .args(["compose", "--goose", document.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    assert!(!record_path(&fixture).exists(), "`prewarm: false` starts no preparation");
}

/// Under the fixture's own `PLAYA_DRY_RUN=1`, the production preparer is
/// never reached: a shipped prompt whose `start` speaks leaves no audio cache
/// file, no voice-capability cache, and no spool entry in fixture-owned
/// locations.
#[test]
fn playa_dry_run_prepares_nothing_and_leaves_no_audio_artifacts() {
    let fixture = CliProcessFixture::named("prewarm-playa-dry-run");
    let root = fixture.cwd().to_path_buf();
    common::prompt_staging::stage_frozen_prompts(&root.join("prompts"), &["clarify.md"]);
    common::write(&root.join("specs/demo-spec.md"), "# Demo\n");
    fixture.initialize_repository();
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    let temp = fixture.workspace_path().join("tmp");
    fs::create_dir_all(&temp).unwrap();
    let voice_cache = fixture.workspace_path().join("voice-capabilities.json");

    // The production path, with the fixture's default dry-run value.
    let output = fixture
        .command()
        .env("TMPDIR", &temp)
        .env("TMP", &temp)
        .env("TEMP", &temp)
        .env("BISCUIT_SPEAKS_CACHE", &voice_cache)
        .args(["compose", "--goose", "prompts/clarify.md", "spec=specs/demo-spec.md"])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    let audio: Vec<_> = fs::read_dir(&temp)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("biscuit-speaks-"))
        .collect();
    assert!(audio.is_empty(), "no audio cache file: {audio:?}");
    assert!(!voice_cache.exists(), "no voice-capability cache");
    assert!(!fixture.audio_spool().exists(), "no spool entry");
}

/// Every accepted `PLAYA_DRY_RUN` spelling stops the worker before it prepares
/// anything.
#[test]
fn every_accepted_playa_dry_run_spelling_prepares_nothing() {
    let fixture = CliProcessFixture::named("prewarm-playa-spellings");
    common::launch_recorder::install(&fixture, "goose", "exit 0");
    let document = write_document(&fixture, "doc.md", EARLY_AND_LATE);
    for value in ["1", "true", "TRUE", "yes"] {
        let output = observed(&fixture, "record")
            .env("PLAYA_DRY_RUN", value)
            .args(["compose", "--goose", document.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(output.status.success(), "PLAYA_DRY_RUN={value}");
        assert!(!record_path(&fixture).exists(), "PLAYA_DRY_RUN={value}");
    }
}

/// Every prompt document of a three-step sequence, and step one's `setup:`,
/// are submitted during preflight. Preflight queues the `setup:` phrase
/// first, and only preflight collects it, so with a preparation that never
/// finishes it is the one preparation that starts: the queue was filled
/// before step one prepared anything. All three steps still launch. With a
/// recorder that finishes at once, every phrase is prepared.
#[test]
fn a_three_prompt_sequence_submits_every_document_before_step_one() {
    for (mode, expected) in [
        ("block", vec!["setting up"]),
        ("record", vec!["Alpha failed", "Beta failed", "Gamma failed", "setting up"]),
    ] {
        let fixture = CliProcessFixture::named("prewarm-sequence");
        let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
        for (name, title) in [("a.md", "Alpha"), ("b.md", "Beta"), ("c.md", "Gamma")] {
            write_document(
                &fixture,
                name,
                &format!("---\ntitle: {title}\nfailure:\n    - say: \"{{{{ title }}}} failed\"\n---\nWork on {title}.\n"),
            );
        }
        let sequence = write_document(
            &fixture,
            "seq.md",
            "---\nsequence:\n    - name: one\n      prompt: a.md\n      setup:\n          - when: \"false\"\n            then:\n                - say: setting up\n    - name: two\n      prompt: b.md\n    - name: three\n      prompt: c.md\n---\nSequence.\n",
        );

        let output = observed(&fixture, mode)
            .args(["sequence", "--goose", sequence.to_str().unwrap()])
            .output()
            .unwrap();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

        assert!(output.status.success(), "{mode}: {stderr}");
        assert_eq!(common::launch_recorder::launches(&launches).len(), 3, "{mode}: {stderr}");
        assert_eq!(sorted(recorded(&record_path(&fixture))), expected, "{mode}: {stderr}");
        assert!(!fixture.audio_spool().exists(), "{mode}: nothing was spoken");
    }
}

/// A `proxy` target's phrases are prepared when the target is prepared, with
/// its `with` overlay applied; the authored value it replaced never is.
#[test]
fn a_proxy_target_is_prepared_with_its_overlay() {
    let fixture = CliProcessFixture::named("prewarm-proxy");
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    write_document(
        &fixture,
        "target.md",
        "---\nname: Authored\nfailure:\n    - say: \"{{ name }} failed\"\n---\nTarget work.\n",
    );
    let router = write_document(
        &fixture,
        "router.md",
        "---\ninitialize:\n    - proxy:\n          target: ./target.md\n          with:\n              name: Overlaid\n---\nRouter.\n",
    );

    let output = observed(&fixture, "record")
        .args(["compose", "--goose", router.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    assert_eq!(recorded(&record_path(&fixture)), ["Overlaid failed"], "{stderr}");
}

/// A retry re-reads the document with its `retry.with` overlay: the retried
/// attempt's new text is prepared, and the unchanged phrase is not prepared
/// again.
#[test]
fn a_retry_prepares_its_new_snapshot_without_repeating_a_candidate() {
    let fixture = CliProcessFixture::named("prewarm-retry");
    // Launch 0 fails, every later launch succeeds.
    let launches =
        common::launch_recorder::install(&fixture, "goose", "[ \"$n\" = 0 ] && exit 1\nexit 0");
    let document = write_document(
        &fixture,
        "retry.md",
        r#"---
reason: first try
failure:
    - when: "false"
      then:
          - say: "never spoken: {{ fm.reason }}"
          - say: "never spoken either"
    - retry:
        max_attempts: 1
        with:
            reason: "{{ fm.reason }}, then retried"
---
reason={{ fm.reason }}
"#,
    );

    let output = observed(&fixture, "record")
        .args(["compose", "--goose", document.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 2, "{stderr}");
    assert_eq!(
        sorted(recorded(&record_path(&fixture))),
        [
            "never spoken either",
            "never spoken: first try",
            "never spoken: first try, then retried",
        ],
        "{stderr}"
    );
    assert!(!fixture.audio_spool().exists(), "nothing was spoken");
}

/// The bootstrap read before `initialize` and the stabilized reread after it
/// submit to one worker: when `initialize` rewrites the document, the changed
/// text is prepared with its new value, and the unchanged phrase is not
/// prepared again.
#[test]
fn initialize_changes_are_discovered_by_the_reread_without_repeating_a_candidate() {
    let fixture = CliProcessFixture::named("prewarm-staged");
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    let document = write_document(
        &fixture,
        "staged.md",
        r#"---
title: Before
initialize:
    - set_frontmatter: ["staged.md", "title", "After"]
failure:
    - when: "false"
      then:
          - say: "{{ title }} is done"
          - say: "unchanged"
---
Work on {{ title }}.
"#,
    );

    let output = observed(&fixture, "record")
        .args(["compose", "--goose", document.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    assert_eq!(
        recorded(&record_path(&fixture)),
        ["Before is done", "unchanged", "After is done"],
        "{stderr}"
    );
}
