//! A dry run observes each external source once per invocation (L1): its
//! passes (input check, the condition-blind reference walk, the preview and
//! its retry) and every task occurrence of one prompt share the source's
//! bytes, so a report never validates one version of a remote input and
//! previews another. Each case counts every request a loopback responder
//! receives, repeats included, under `permissions.network: allow`; changing
//! a URL adds its own observation. The `ask`/`deny` cells, which send
//! nothing, are `dry_run_network_approval.rs`. Local sources follow the same
//! rule, counted by opens (`one_open_per_source`).

use std::fs;

use serde_json::Value;

use crate::dry_run_network_approval::{Responder, dry_run, fixture, rows};

/// A sequence running `ask.md` twice with the same inputs.
const TWICE: &str = "---\nsequence:\n  - name: first\n    prompt: ./ask.md\n  - name: second\n    prompt: ./ask.md\n---\nBody.\n";

/// The exit code is 0 and every route was requested exactly once.
fn assert_once(server: &Responder, (code, envelope): (Option<i32>, Value), routes: &[&str]) {
    let rows = rows(&envelope);
    assert_eq!(code, Some(0), "{rows:#?}");
    let hits = server.all_hits();
    assert_eq!(hits.len(), routes.len(), "{hits:?} {rows:#?}");
    for route in routes {
        assert_eq!(server.hits(route), 1, "{route}: {hits:?} {rows:#?}");
    }
}

fn load_data_body(server: &Responder) -> String {
    format!("Data {{{{ load_data('{}', 'string[]') }}}}.\n", server.url("/data.json"))
}

/// R2-F5.1: a body `load_data` URL in a direct compose.
fn compose_load_data() {
    let server = Responder::start();
    let fixture = fixture("allow");
    fs::write(fixture.cwd().join("doc.md"), format!("---\nagent: claude\n---\n{}", load_data_body(&server))).unwrap();
    assert_once(&server, dry_run(&fixture, "compose", "doc.md"), &["/data.json"]);
}

/// R2-F5.2: the same read in an inline `prompt:`.
fn inline_compose_load_data() {
    let server = Responder::start();
    let fixture = fixture("allow");
    let text = format!(
        "---\nagent: claude\nprompt: \"Summarize {{{{ load_data('{}', 'string[]') }}}}\"\n---\nBody.\n",
        server.url("/data.json")
    );
    fs::write(fixture.cwd().join("doc.md"), text).unwrap();
    assert_once(&server, dry_run(&fixture, "inline-compose", "doc.md"), &["/data.json"]);
}

/// R2-F5.3: two task occurrences of one prompt file and its URL, each
/// still previewed and reported as its own task.
fn sequence_load_data() {
    let server = Responder::start();
    let fixture = fixture("allow");
    fs::write(
        fixture.cwd().join("ask.md"),
        format!("---\nagent: claude\n---\n{}Then {{{{ missing_value }}}}.\n", load_data_body(&server)),
    )
    .unwrap();
    fs::write(fixture.cwd().join("seq.md"), TWICE).unwrap();
    let (code, envelope) = dry_run(&fixture, "sequence", "seq.md");
    // Each occurrence keeps its own finding for the undeclared read.
    let labels = task_labels(&envelope, "missing_value");
    assert!(labels.contains(&"first".to_string()) && labels.contains(&"second".to_string()), "{labels:?} {:#?}", rows(&envelope));
    let rows = rows(&envelope);
    assert!(code == Some(0) || code == Some(2), "{rows:#?}");
    assert_eq!(server.hits("/data.json"), 1, "{rows:#?}");
    assert_eq!(server.requests(), 1, "{rows:#?}");
}

/// The last trail frame's label of every row whose message names `needle`.
fn task_labels(envelope: &Value, needle: &str) -> Vec<String> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["message"].as_str().is_some_and(|message| message.contains(needle)))
        .filter_map(|row| row["trail"].as_array()?.last()?["label"].as_str().map(str::to_string))
        .collect()
}

/// A prompt with a remote `::file`, `::code`, `prologue:`, and `epilogue:`.
fn transclusions(server: &Responder) -> String {
    format!(
        "---\nagent: claude\nprologue: {}\nepilogue: {}\n---\nBefore.\n\n::file {}\n\n::code {}\n",
        server.url("/prologue.md"),
        server.url("/epilogue.md"),
        server.url("/child.md"),
        server.url("/snippet.rs"),
    )
}

const TRANSCLUDED: [&str; 4] = ["/prologue.md", "/epilogue.md", "/child.md", "/snippet.rs"];

/// R2-F5.4: each remote inclusion once in a single-document compose.
fn compose_transclusions() {
    let server = Responder::start();
    let fixture = fixture("allow");
    fs::write(fixture.cwd().join("doc.md"), transclusions(&server)).unwrap();
    assert_once(&server, dry_run(&fixture, "compose", "doc.md"), &TRANSCLUDED);
}

/// R2-F5.4: each remote inclusion once across two occurrences of a prompt.
fn sequence_transclusions() {
    let server = Responder::start();
    let fixture = fixture("allow");
    fs::write(fixture.cwd().join("ask.md"), transclusions(&server)).unwrap();
    fs::write(fixture.cwd().join("seq.md"), TWICE).unwrap();
    assert_once(&server, dry_run(&fixture, "sequence", "seq.md"), &TRANSCLUDED);
}

/// A schema whose enum slot reads the responder.
fn write_slot_schema(fixture: &crate::common::CliProcessFixture, server: &Responder) {
    fs::write(
        fixture.cwd().join("schema.yaml"),
        format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{}\\\", \\\"string[]\\\") }}}})\"\n", server.url("/os.yaml")),
    )
    .unwrap();
}

/// R2-F5.5: a legal slot value in a direct compose.
fn compose_schema_slot() {
    let server = Responder::start();
    let fixture = fixture("allow");
    write_slot_schema(&fixture, &server);
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n$schema: ./schema.yaml\nos: macOS\n---\nBody.\n").unwrap();
    assert_once(&server, dry_run(&fixture, "compose", "doc.md"), &["/os.yaml"]);
}

/// R2-F5.5: the same slot in an inline prompt.
fn inline_compose_schema_slot() {
    let server = Responder::start();
    let fixture = fixture("allow");
    write_slot_schema(&fixture, &server);
    fs::write(
        fixture.cwd().join("doc.md"),
        "---\nagent: claude\n$schema: ./schema.yaml\nos: macOS\nprompt: \"Check {{ os }}.\"\n---\nBody.\n",
    )
    .unwrap();
    assert_once(&server, dry_run(&fixture, "inline-compose", "doc.md"), &["/os.yaml"]);
}

/// R2-F5.5: two occurrences read the slot once, and each keeps its own
/// verdict: the second's `with:` value is outside the enum.
fn sequence_schema_slot() {
    let server = Responder::start();
    let fixture = fixture("allow");
    write_slot_schema(&fixture, &server);
    fs::write(fixture.cwd().join("ask.md"), "---\nagent: claude\n$schema: ./schema.yaml\nos: macOS\n---\nCheck {{ os }}.\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nsequence:\n  - name: first\n    prompt: ./ask.md\n  - name: second\n    prompt: ./ask.md\n    with:\n      os: Plan9\n---\nBody.\n",
    )
    .unwrap();
    let (code, envelope) = dry_run(&fixture, "sequence", "seq.md");
    let rows = rows(&envelope);
    assert_eq!(code, Some(2), "{rows:#?}");
    assert_eq!(task_labels(&envelope, "Plan9"), ["second"], "{rows:#?}");
    assert_eq!(server.hits("/os.yaml"), 1, "{rows:#?}");
    assert_eq!(server.requests(), 1, "{rows:#?}");
}

/// R2-F5.7: two reads of one URL in one evaluation are one request.
fn one_url_read_twice() {
    let server = Responder::start();
    let fixture = fixture("allow");
    let url = server.url("/data.json");
    fs::write(
        fixture.cwd().join("doc.md"),
        format!("---\nagent: claude\n---\nA {{{{ load_data('{url}', 'string[]') }}}} B {{{{ load_data('{url}', 'string[]') }}}}.\n"),
    )
    .unwrap();
    assert_once(&server, dry_run(&fixture, "compose", "doc.md"), &["/data.json"]);
}

/// R2-F5.7: changing only the second URL adds one request for it.
fn two_urls_read_once_each() {
    let server = Responder::start();
    let fixture = fixture("allow");
    fs::write(
        fixture.cwd().join("doc.md"),
        format!(
            "---\nagent: claude\n---\nA {{{{ load_data('{}', 'string[]') }}}} B {{{{ load_data('{}', 'string[]') }}}}.\n",
            server.url("/data.json"),
            server.url("/other.json"),
        ),
    )
    .unwrap();
    assert_once(&server, dry_run(&fixture, "compose", "doc.md"), &["/data.json", "/other.json"]);
}

macro_rules! rows {
    ($($case:ident),* $(,)?) => {
        $(#[test]
        fn $case() { super::$case(); })*
    };
}

mod one_request_per_url {
    rows!(
        compose_load_data,
        inline_compose_load_data,
        sequence_load_data,
        compose_transclusions,
        sequence_transclusions,
        compose_schema_slot,
        inline_compose_schema_slot,
        sequence_schema_slot,
        one_url_read_twice,
        two_urls_read_once_each,
    );
}

/// Local sources (R2-F5.6, R3-F4.1–.4): every pass of one dry run reads a
/// prompt, an inclusion, a `prologue`/`epilogue`, a `$schema` file, or a
/// referenced sequence file from one observation, so each is opened once
/// whether one task or two use it. The `test-fixtures` seam
/// `CLAUDINE_TEST_SOURCE_OPENS` logs every open the shared reader makes.
#[cfg(feature = "test-fixtures")]
mod one_open_per_source {
    use std::fs;

    use crate::common::CliProcessFixture;
    use crate::dry_run_network_approval::{fixture, rows};

    /// `claudine <command> <file> --dry-run --json` with the open log: the
    /// exit code, the rows, and how many times each of `watched` was opened.
    type Rows = Vec<(String, String, String)>;

    fn opens(fixture: &CliProcessFixture, command: &str, file: &str, watched: &[&str]) -> (Option<i32>, Rows, Vec<usize>) {
        let log = fixture.home().join("opens.log");
        let _ = fs::remove_file(&log);
        let output = fixture
            .command()
            .args([command, file, "--dry-run", "--json"])
            .env("CLAUDINE_TEST_SOURCE_OPENS", &log)
            .output()
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stdout)));
        let opened = fs::read_to_string(&log).unwrap_or_default();
        let count = |name: &str| opened.lines().filter(|line| std::path::Path::new(line).file_name().is_some_and(|file| file == name)).count();
        (output.status.code(), rows(&envelope), watched.iter().map(|name| count(name)).collect())
    }

    /// A prompt with an external schema, a prologue and epilogue, a body
    /// inclusion, and a `::code` target, plus `extra` appended to its body.
    fn prompt_fixture(extra: &str) -> CliProcessFixture {
        let fixture = fixture("deny");
        let cwd = fixture.cwd();
        fs::write(cwd.join("s.yaml"), "$schema:\n  topic: string\n").unwrap();
        fs::write(cwd.join("pro.md"), "Prologue.\n").unwrap();
        fs::write(cwd.join("epi.md"), "Epilogue.\n").unwrap();
        fs::write(cwd.join("inc.md"), "Included.\n").unwrap();
        fs::write(cwd.join("other.md"), "Other.\n").unwrap();
        fs::write(cwd.join("snippet.txt"), "let x = 1;\n").unwrap();
        fs::write(
            cwd.join("ask.md"),
            format!("---\nagent: claude\n$schema: ./s.yaml\ntopic: hello\nprologue: ./pro.md\nepilogue: ./epi.md\n---\nAsk about {{{{ topic }}}}.\n\n::file ./inc.md\n\n::code ./snippet.txt\n{extra}"),
        )
        .unwrap();
        fixture
    }

    fn sequence_of(fixture: &CliProcessFixture, prompts: &[&str]) {
        let steps: String = prompts.iter().enumerate().map(|(index, prompt)| format!("  - name: t{index}\n    prompt: ./{prompt}\n")).collect();
        fs::write(fixture.cwd().join("seq.md"), format!("---\nsequence:\n{steps}---\nBody.\n")).unwrap();
    }

    const PROMPT_SOURCES: [&str; 7] = ["seq.md", "ask.md", "s.yaml", "pro.md", "epi.md", "inc.md", "snippet.txt"];

    /// R2-F5.6, R3-F4.2, R3-F4.3: one task and two tasks of one prompt open
    /// the prompt and each source it reads once, and each occurrence keeps
    /// its own verdict.
    #[test]
    fn one_or_two_prompt_tasks_open_each_source_once() {
        for occurrences in [1, 2] {
            let fixture = prompt_fixture("");
            sequence_of(&fixture, &vec!["ask.md"; occurrences]);
            let (code, rows, opened) = opens(&fixture, "sequence", "seq.md", &PROMPT_SOURCES);
            assert_eq!(code, Some(0), "{rows:#?}");
            assert_eq!(opened, [1; 7], "{occurrences} occurrence(s) of {PROMPT_SOURCES:?}: {rows:#?}");
        }
    }

    /// R3-F4.2: a separate missing inclusion is one row per occurrence, and
    /// the check and its recovery still open the good inclusion once.
    #[test]
    fn a_missing_inclusion_beside_a_good_one_opens_the_good_one_once() {
        for occurrences in [1, 2] {
            let fixture = prompt_fixture("\n::file ./missing.md\n");
            sequence_of(&fixture, &vec!["ask.md"; occurrences]);
            let (code, rows, opened) = opens(&fixture, "sequence", "seq.md", &PROMPT_SOURCES);
            assert_eq!(code, Some(2), "{rows:#?}");
            let missing = rows.iter().filter(|(code, _, _)| code == "dm.reference.missing_transclusion").count();
            assert_eq!(missing, occurrences, "{rows:#?}");
            assert_eq!(opened, [1; 7], "{occurrences} occurrence(s) of {PROMPT_SOURCES:?}: {rows:#?}");
        }
    }

    /// Two prompts including different files: each file is its own source,
    /// opened once.
    #[test]
    fn distinct_prompts_and_inclusions_are_each_opened_once() {
        let fixture = prompt_fixture("");
        fs::write(fixture.cwd().join("ask2.md"), "---\nagent: claude\n---\nSecond.\n\n::file ./other.md\n").unwrap();
        sequence_of(&fixture, &["ask.md", "ask2.md", "ask.md"]);
        let (code, rows, opened) = opens(&fixture, "sequence", "seq.md", &["ask.md", "ask2.md", "inc.md", "other.md"]);
        assert_eq!(code, Some(0), "{rows:#?}");
        assert_eq!(opened, [1, 1, 1, 1], "{rows:#?}");
    }

    /// R3-F4.1: a direct composition's `::file` and an inline one's
    /// `prologue` are opened once for the reference check and the
    /// composition; a missing target is still one missing-transclusion row.
    #[test]
    fn a_direct_or_inline_composition_opens_its_inclusion_once() {
        let fixture = prompt_fixture("");
        fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello.\n\n::file ./inc.md\n").unwrap();
        let (code, rows, opened) = opens(&fixture, "compose", "doc.md", &["doc.md", "inc.md"]);
        assert_eq!(code, Some(0), "{rows:#?}");
        assert_eq!(opened, [1, 1], "{rows:#?}");

        fs::write(fixture.cwd().join("inl.md"), "---\nagent: claude\nprompt: \"Do it\"\nprologue: ./inc.md\n---\nBody.\n").unwrap();
        let (code, rows, opened) = opens(&fixture, "inline-compose", "inl.md", &["inl.md", "inc.md"]);
        assert_eq!(code, Some(0), "{rows:#?}");
        assert_eq!(opened, [1, 1], "{rows:#?}");

        fs::remove_file(fixture.cwd().join("inc.md")).unwrap();
        let (code, rows, _) = opens(&fixture, "compose", "doc.md", &[]);
        assert_eq!(code, Some(2), "{rows:#?}");
        assert_eq!(rows.iter().filter(|(code, _, _)| code == "dm.reference.missing_transclusion").count(), 1, "{rows:#?}");
    }

    /// R3-F4.4: a referenced sequence, task, group, and selected catalog
    /// entry, each reached `occurrences` times (the task with a key the
    /// schema refuses, so key validation and its location projection read
    /// it too), are each opened once.
    fn referenced_sequence_files_opened_once(occurrences: usize) {
        let step = "  - name: sNN\n    sequence: ./child.yaml\n  - name: tNN\n    task: ./t.yaml\n  - name: gNN\n    group: ./g.yaml\n  - name: cNN\n    group: first@cat.yaml\n";
        let fixture = fixture("deny");
        let cwd = fixture.cwd();
        fs::write(cwd.join("x.md"), "---\nagent: claude\n---\nDo x.\n").unwrap();
        fs::write(cwd.join("child.yaml"), "kind: sequence\nsequence:\n  - name: c1\n    prompt: ./x.md\n").unwrap();
        fs::write(cwd.join("t.yaml"), "kind: task\nprompt: ./x.md\nwhem: \"true\"\n").unwrap();
        fs::write(cwd.join("g.yaml"), "kind: group\ntasks:\n  - name: leg\n    prompt: ./x.md\n").unwrap();
        fs::write(
            cwd.join("cat.yaml"),
            "kind: group-catalog\ngroups:\n  - name: first\n    tasks:\n      - name: a\n        prompt: ./x.md\n  - name: second\n    tasks:\n      - name: b\n        prompt: ./x.md\n",
        )
        .unwrap();
        let list: String = (0..occurrences).map(|index| step.replace("NN", &index.to_string())).collect();
        fs::write(cwd.join("seq.md"), format!("---\nagent: claude\nsequence:\n{list}---\nBody.\n")).unwrap();

        let watched = ["seq.md", "x.md", "child.yaml", "t.yaml", "g.yaml", "cat.yaml"];
        let (code, rows, opened) = opens(&fixture, "sequence", "seq.md", &watched);

        assert_eq!(code, Some(2), "{rows:#?}");
        let keys = rows.iter().filter(|(code, _, message)| code == "dm.schema.unknown_key" && message.contains("whem")).count();
        assert_eq!(keys, occurrences, "{rows:#?}");
        assert_eq!(opened, [1; 6], "{occurrences} occurrence(s) of {watched:?}: {rows:#?}");
    }

    #[test]
    fn referenced_sequence_files_reached_once_are_opened_once() {
        referenced_sequence_files_opened_once(1);
    }

    #[test]
    fn referenced_sequence_files_reached_twice_are_opened_once() {
        referenced_sequence_files_opened_once(2);
    }

    /// R3-F4.6: a local `load_data` file read by every pass and two tasks
    /// is opened once.
    #[test]
    fn a_local_data_file_is_opened_once() {
        let fixture = fixture("deny");
        fs::write(fixture.cwd().join("rows.json"), "[\"a\", \"b\"]").unwrap();
        fs::write(fixture.cwd().join("ask.md"), "---\nagent: claude\n---\nData {{ load_data('./rows.json', 'string[]') }}.\n").unwrap();
        sequence_of(&fixture, &["ask.md", "ask.md"]);
        let (code, rows, opened) = opens(&fixture, "sequence", "seq.md", &["rows.json"]);
        assert_eq!(code, Some(0), "{rows:#?}");
        assert_eq!(opened, [1], "{rows:#?}");
    }
}

/// R3-F4.3: every pass of one dry run prepares a prompt's effective schema
/// once per distinct input (the `$schema` file, the context, the triggers
/// its values match), so one task and two tasks of one prompt cost the same
/// preparations, while each occurrence keeps its own verdict. The
/// `test-fixtures` seam `CLAUDINE_TEST_SCHEMA_PREPARATIONS` logs each
/// preparation's document.
#[cfg(feature = "test-fixtures")]
mod one_preparation_per_schema {
    use std::fs;

    use crate::common::CliProcessFixture;
    use crate::dry_run_network_approval::{fixture, rows};

    type Rows = Vec<(String, String, String)>;

    /// `claudine sequence seq.md --dry-run --json` with the preparation
    /// log: the exit code, the rows, the envelope, and how many times each
    /// of `watched` had its schema prepared.
    fn preparations(fixture: &CliProcessFixture, watched: &[&str]) -> (Option<i32>, Rows, serde_json::Value, Vec<usize>) {
        let log = fixture.home().join("preparations.log");
        let _ = fs::remove_file(&log);
        let output = fixture
            .command()
            .args(["sequence", "seq.md", "--dry-run", "--json"])
            .env("CLAUDINE_TEST_SCHEMA_PREPARATIONS", &log)
            .output()
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stdout)));
        let logged = fs::read_to_string(&log).unwrap_or_default();
        let count = |name: &str| logged.lines().filter(|line| std::path::Path::new(line).file_name().is_some_and(|file| file == name)).count();
        (output.status.code(), rows(&envelope), envelope.clone(), watched.iter().map(|name| count(name)).collect())
    }

    /// A sequence of `steps` (each a task's name, prompt, and `count:`).
    fn sequence_of(fixture: &CliProcessFixture, steps: &[(&str, &str, i64)]) {
        let list: String = steps
            .iter()
            .map(|(name, prompt, count)| format!("  - name: {name}\n    prompt: ./{prompt}\n    with:\n      count: {count}\n"))
            .collect();
        fs::write(fixture.cwd().join("seq.md"), format!("---\nsequence:\n{list}---\nBody.\n")).unwrap();
    }

    /// Prompts `ask.md` (`$schema: ./s.yaml`, `count` at least 1) and
    /// `other.md` (`$schema: ./t.yaml`, `count` at most 2).
    fn prompts() -> CliProcessFixture {
        let fixture = fixture("deny");
        let cwd = fixture.cwd();
        fs::write(cwd.join("s.yaml"), "$schema:\n  count: number(min(1))\n").unwrap();
        fs::write(cwd.join("t.yaml"), "$schema:\n  count: number(max(2))\n").unwrap();
        fs::write(cwd.join("ask.md"), "---\nagent: claude\n$schema: ./s.yaml\ncount: 1\n---\nCount {{ count }}.\n").unwrap();
        fs::write(cwd.join("other.md"), "---\nagent: claude\n$schema: ./t.yaml\ncount: 1\n---\nOther {{ count }}.\n").unwrap();
        fixture
    }

    /// The labels of the tasks whose rows have `code`.
    fn tasks_with(envelope: &serde_json::Value, code: &str) -> Vec<String> {
        envelope["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["code"] == code)
            .filter_map(|row| row["trail"].as_array()?.iter().find(|frame| frame["host"] == "task")?["label"].as_str().map(String::from))
            .collect()
    }

    #[test]
    fn one_or_two_tasks_of_one_prompt_prepare_its_schema_alike() {
        let fixture = prompts();
        sequence_of(&fixture, &[("t0", "ask.md", 2)]);
        let (code, rows, _, one) = preparations(&fixture, &["ask.md"]);
        assert_eq!(code, Some(0), "{rows:#?}");
        sequence_of(&fixture, &[("t0", "ask.md", 2), ("t1", "ask.md", 3)]);
        let (code, rows, _, two) = preparations(&fixture, &["ask.md"]);
        assert_eq!(code, Some(0), "{rows:#?}");
        // Measured 2 (two distinct preparation inputs across the passes);
        // before the memo every pass prepared again, 14 times per task.
        assert!((1..=2).contains(&one[0]), "the prompt's schema is prepared once per distinct input: {one:?}");
        assert_eq!(one, two, "a second task of the same prompt prepares nothing new");
    }

    #[test]
    fn a_different_schema_file_is_prepared_separately_and_each_task_keeps_its_verdict() {
        let fixture = prompts();
        sequence_of(&fixture, &[("good", "ask.md", 2), ("bad", "ask.md", 0), ("over", "other.md", 3)]);
        let (code, rows, envelope, prepared) = preparations(&fixture, &["ask.md", "other.md"]);
        assert_eq!(code, Some(2), "{rows:#?}");
        assert!(prepared.iter().all(|count| *count >= 1), "{prepared:?}");
        assert_eq!(prepared[0], prepared[1], "each schema file is prepared as often as the other: {prepared:?}");
        let mut refused = tasks_with(&envelope, "dm.schema.constraint");
        refused.sort();
        assert_eq!(refused, ["bad", "over"], "{rows:#?}");
    }
}
