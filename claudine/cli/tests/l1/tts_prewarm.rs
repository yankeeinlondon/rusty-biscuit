//! TTS pre-warm through the real command flows: which phrases each flow
//! submits, that submission never delays the provider launch, and that every
//! gate prepares nothing.
//!
//! Preparation is observed through the `test-fixtures` recorder: with
//! `CLAUDINE_TEST_PREWARM=record:<path>` each preparation the worker starts
//! appends its text and its playback configuration to `<path>` instead of
//! synthesizing speech, and with
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
    recorded_with_config(path).into_iter().map(|(text, _)| text).collect()
}

/// Each recorded preparation as its text and its playback configuration
/// serialized as JSON, in preparation order.
fn recorded_with_config(path: &Path) -> Vec<(String, String)> {
    fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .map(|line| {
                    let (text, config) = line.split_once('\t').expect("text, a tab, and a config");
                    (text.to_owned(), config.to_owned())
                })
                .collect()
        })
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

/// Writes three prompt documents, each with a never-firing early phrase, and
/// a sequence over them whose step one has a never-firing `setup:` phrase.
fn write_three_prompt_sequence(fixture: &CliProcessFixture) -> PathBuf {
    for (name, title) in [("a.md", "Alpha"), ("b.md", "Beta"), ("c.md", "Gamma")] {
        write_document(
            fixture,
            name,
            &format!("---\ntitle: {title}\nfailure:\n    - say: \"{{{{ title }}}} failed\"\n---\nWork on {title}.\n"),
        );
    }
    write_document(
        fixture,
        "seq.md",
        "---\nsequence:\n    - name: one\n      prompt: a.md\n      setup:\n          - when: \"false\"\n            then:\n                - say: setting up\n    - name: two\n      prompt: b.md\n    - name: three\n      prompt: c.md\n---\nSequence.\n",
    )
}

/// Every prompt document of a three-step sequence is submitted during
/// preflight. Steps two and three are not otherwise prepared until step one
/// has exited, so step one's provider waits (bounded) for their phrases to
/// appear in the recorder and reports what it saw: only preflight submission
/// can put them there while it is still running. The assertion after the run
/// would also pass if ordinary per-step preparation produced them later.
#[test]
fn a_three_prompt_sequence_submits_every_document_before_step_one() {
    let fixture = CliProcessFixture::named("prewarm-sequence");
    let record = record_path(&fixture);
    let launches = common::launch_recorder::install(
        &fixture,
        "goose",
        &format!(
            "if [ \"$n\" = 0 ]; then\n\
             seen=timeout; i=0\n\
             while [ $i -lt 200 ]; do\n\
             if grep -q '^Beta failed' '{record}' 2>/dev/null && grep -q '^Gamma failed' '{record}' 2>/dev/null; then seen=prepared; break; fi\n\
             sleep 0.05; i=$((i + 1))\n\
             done\n\
             echo $seen > \"$dir/step-one-saw\"\n\
             fi\n\
             exit 0",
            record = record.display()
        ),
    );
    let sequence = write_three_prompt_sequence(&fixture);

    let output = observed(&fixture, "record")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 3, "{stderr}");
    assert_eq!(
        fs::read_to_string(launches.join("step-one-saw")).unwrap().trim(),
        "prepared",
        "steps two and three were prepared while step one ran\n{stderr}"
    );
    assert_eq!(
        sorted(recorded(&record)),
        ["Alpha failed", "Beta failed", "Gamma failed", "setting up"],
        "{stderr}"
    );
    assert!(!fixture.audio_spool().exists(), "nothing was spoken");
}

/// A preparation that never finishes delays none of a sequence's steps: the
/// first phrase queued (step one's `setup:`) starts and blocks the worker,
/// and all three steps still launch. This does not show when the later
/// prompts were submitted; the test above does.
#[test]
fn a_blocked_preparation_does_not_delay_any_sequence_step() {
    let fixture = CliProcessFixture::named("prewarm-sequence-blocked");
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    let sequence = write_three_prompt_sequence(&fixture);

    let output = observed(&fixture, "block")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 3, "{stderr}");
    assert_eq!(recorded(&record_path(&fixture)), ["setting up"], "{stderr}");
    assert!(!fixture.audio_spool().exists(), "nothing was spoken");
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

/// Under `{ "tts": { "provider": "kokoro", "voice": "af_heart" } }`, a mixed
/// sequence prepares each phrase with what its path plays with: the
/// `setup:`/`teardown:` of a top-level task, a serial member and a parallel
/// member with the task stacks' playback configuration (automatic provider, no
/// requested voice), and every prompt document lifecycle with the user's.
#[test]
fn a_mixed_sequence_prepares_task_stacks_and_prompts_with_their_playback_configuration() {
    let fixture = CliProcessFixture::named("prewarm-mixed-config");
    common::write(
        &fixture.home().join(".claudine/config.json"),
        r#"{ "tts": { "provider": "kokoro", "voice": "af_heart" } }"#,
    );
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    for name in ["a.md", "m.md", "p.md"] {
        write_document(
            &fixture,
            name,
            &format!("---\nfailure:\n    - say: \"{name} failed\"\n---\nWork on {name}.\n"),
        );
    }
    let stacks = |label: &str, indent: &str| {
        format!(
            "{indent}setup:\n{indent}    - when: \"false\"\n{indent}      then:\n{indent}          - say: {label} setup\n\
             {indent}teardown:\n{indent}    - when: \"false\"\n{indent}      then:\n{indent}          - say: {label} teardown\n"
        )
    };
    let sequence = write_document(
        &fixture,
        "seq.md",
        &format!(
            "---\nsequence:\n    - name: top\n      prompt: a.md\n{top}\
             \x20   - name: serial\n      group:\n          execution: serial\n          tasks:\n              - name: member\n                prompt: m.md\n{member}\
             \x20   - name: parallel\n      group:\n          execution: parallel\n          tasks:\n              - name: leg\n                prompt: p.md\n{leg}\
             ---\nSequence.\n",
            top = stacks("top", "      "),
            member = stacks("serial member", "                "),
            leg = stacks("parallel member", "                "),
        ),
    );

    let output = observed(&fixture, "record")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 3, "{stderr}");
    let task_config = serde_json::to_string(&biscuit_speaks::TtsConfig::new()).unwrap();
    let mut prepared = recorded_with_config(&record_path(&fixture));
    prepared.sort();
    let user_configs: Vec<&String> = prepared
        .iter()
        .filter(|(text, _)| text.ends_with(".md failed"))
        .map(|(_, config)| config)
        .collect();
    assert_eq!(user_configs.len(), 3, "{prepared:?}");
    assert!(
        user_configs.iter().all(|config| *config == user_configs[0]
            && config.contains("af_heart")
            && config.to_lowercase().contains("kokoro")),
        "prompt lifecycles use the user's provider and voice: {user_configs:?}"
    );
    let task_phrases: Vec<(String, String)> = prepared
        .iter()
        .filter(|(text, _)| !text.ends_with(".md failed"))
        .cloned()
        .collect();
    let mut expected: Vec<(String, String)> = [
        "top setup",
        "top teardown",
        "serial member setup",
        "serial member teardown",
        "parallel member setup",
        "parallel member teardown",
    ]
    .iter()
    .map(|text| ((*text).to_owned(), task_config.clone()))
    .collect();
    expected.sort();
    assert_eq!(task_phrases, expected, "task stacks use the task playback configuration\n{stderr}");
    assert!(!fixture.audio_spool().exists(), "nothing was spoken");
}

/// The prompt document every deferred-task test runs: its own `env` and
/// `ctx` reads are the control, prepared when the document is prepared.
const DEFERRED_DOCUMENT: &str = "---\nfailure:\n    - when: \"false\"\n      then:\n          - say: \"doc env {{ env.PREWARM_NAME }}\"\n          - say: \"doc ctx {{ ctx.today_utc }}\"\n---\nWork.\n";

/// A guarded `property:` stack, indented by `indent`, whose phrases read what
/// sequence preflight cannot know: the task's `env` and `ctx` captures and a
/// frontmatter value only a template produces. A literal is the control
/// preflight already queues.
fn deferred_stack(property: &str, indent: &str) -> String {
    format!(
        "{indent}{property}:\n{indent}    - when: \"false\"\n{indent}      then:\n\
         {indent}          - say: literal control\n\
         {indent}          - say: \"task env {{{{ env.PREWARM_NAME }}}}\"\n\
         {indent}          - say: \"task ctx {{{{ ctx.today_utc }}}}\"\n\
         {indent}          - say: \"task derived {{{{ derived }}}}\"\n"
    )
}

/// The sequence document for one task site: a top-level task, a serial group
/// member, or a parallel group member, carrying `stack` (`None` for none).
fn deferred_sequence(site: &str, stack: Option<&str>) -> String {
    let (head, indent) = match site {
        "top" => ("    - name: first\n      prompt: doc.md\n", "      "),
        "serial" | "parallel" => (
            if site == "serial" {
                "    - name: group\n      group:\n          execution: serial\n          tasks:\n              - name: member\n                prompt: doc.md\n"
            } else {
                "    - name: group\n      group:\n          execution: parallel\n          tasks:\n              - name: member\n                prompt: doc.md\n"
            },
            "                ",
        ),
        other => panic!("unknown site {other}"),
    };
    let stack = stack.map(|property| deferred_stack(property, indent)).unwrap_or_default();
    format!("---\nproject: Widget\nderived: \"{{{{ project }}}} ready\"\nsequence:\n{head}{stack}---\nSequence.\n")
}

/// Run `claudine sequence` over one task site with the child environment
/// `PREWARM_NAME=Ada`, returning every preparation and the UTC dates the run
/// could have captured.
fn run_deferred(name: &str, site: &str, stack: Option<&str>) -> (Vec<(String, String)>, Vec<String>) {
    let fixture = CliProcessFixture::named(name);
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    write_document(&fixture, "doc.md", DEFERRED_DOCUMENT);
    let sequence = write_document(&fixture, "seq.md", &deferred_sequence(site, stack));

    let today = || chrono::Utc::now().date_naive().to_string();
    let before = today();
    let output = observed(&fixture, "record")
        .env("PREWARM_NAME", "Ada")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .output()
        .unwrap();
    let after = today();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&launches).len(), 1, "{stderr}");
    assert!(!fixture.audio_spool().exists(), "nothing was spoken");
    let mut prepared = recorded_with_config(&record_path(&fixture));
    prepared.sort();
    (prepared, vec![before, after])
}

/// Assert `prepared` holds exactly the document control and each resolved
/// task phrase once, every task phrase with the task playback configuration.
fn assert_deferred_task_phrases(prepared: &[(String, String)], dates: &[String]) {
    let task_config = serde_json::to_string(&biscuit_speaks::TtsConfig::new()).unwrap();
    let date = dates
        .iter()
        .find(|date| prepared.iter().any(|(text, _)| *text == format!("task ctx {date}")))
        .unwrap_or_else(|| panic!("`task ctx` resolved to the captured date {dates:?}: {prepared:?}"));
    let texts: Vec<&str> = prepared.iter().map(|(text, _)| text.as_str()).collect();
    let mut expected = vec![
        format!("doc ctx {date}"),
        "doc env Ada".to_owned(),
        "literal control".to_owned(),
        format!("task ctx {date}"),
        "task derived Widget ready".to_owned(),
        "task env Ada".to_owned(),
    ];
    expected.sort();
    assert_eq!(texts, expected, "each phrase is prepared exactly once");
    for (text, config) in prepared.iter().filter(|(text, _)| !text.starts_with("doc ")) {
        assert_eq!(config, &task_config, "{text}: the task playback configuration");
    }
}

/// One test per task site: a `setup:` or `teardown:` phrase preflight skipped
/// is prepared, resolved, once the site establishes the task's context.
macro_rules! deferred_task_stack_tests {
    ($($test:ident: $site:literal, $property:literal;)*) => {$(
        #[test]
        fn $test() {
            let (prepared, dates) = run_deferred(stringify!($test), $site, Some($property));
            assert_deferred_task_phrases(&prepared, &dates);
        }
    )*};
}

deferred_task_stack_tests! {
    deferred_phrases_of_a_top_level_task_setup_are_prepared: "top", "setup";
    deferred_phrases_of_a_top_level_task_teardown_are_prepared: "top", "teardown";
    deferred_phrases_of_a_serial_member_setup_are_prepared: "serial", "setup";
    deferred_phrases_of_a_serial_member_teardown_are_prepared: "serial", "teardown";
    deferred_phrases_of_a_parallel_member_setup_are_prepared: "parallel", "setup";
    deferred_phrases_of_a_parallel_member_teardown_are_prepared: "parallel", "teardown";
}

/// The control: a prompt task's own document prepares its `env` and `ctx`
/// reads with the values the task captured.
#[test]
fn a_prompt_documents_env_and_ctx_phrases_are_prepared_resolved() {
    let (prepared, dates) = run_deferred("prewarm-deferred-document", "top", None);
    let texts: Vec<&str> = prepared.iter().map(|(text, _)| text.as_str()).collect();
    assert!(
        dates
            .iter()
            .any(|date| texts == [format!("doc ctx {date}").as_str(), "doc env Ada"]),
        "{prepared:?}"
    );
}

/// A group container that authors `setup:` is rejected before anything runs,
/// so a group adds no speech site of its own.
#[test]
fn a_group_container_with_setup_is_rejected_before_any_preparation() {
    let fixture = CliProcessFixture::named("prewarm-group-container");
    let launches = common::launch_recorder::install(&fixture, "goose", "exit 0");
    write_document(&fixture, "doc.md", DEFERRED_DOCUMENT);
    let sequence = write_document(
        &fixture,
        "seq.md",
        &format!(
            "---\nsequence:\n    - name: group\n{}      group:\n          execution: serial\n          tasks:\n              - name: member\n                prompt: doc.md\n---\nSequence.\n",
            deferred_stack("setup", "      "),
        ),
    );

    let output = observed(&fixture, "record")
        .args(["sequence", "--goose", sequence.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("which is not valid for a `group`"), "{stderr}");
    assert!(common::launch_recorder::launches(&launches).is_empty(), "{stderr}");
    assert!(!record_path(&fixture).exists(), "nothing was prepared\n{stderr}");
}
