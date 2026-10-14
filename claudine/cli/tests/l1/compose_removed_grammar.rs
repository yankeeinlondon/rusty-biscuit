//! CLI-boundary coverage for `CompositionError::RemovedGrammar`.
//!
//! Every retired lifecycle spelling — the `stack:` key, the `action:` item
//! aggregator and its place beside `when`, `say_first`, the communication
//! `message`/`sound`/`route` parameters — and a read of a retired `_loop_*` name
//! in a frontmatter expression or a body span stops `compose`,
//! `inline-compose`, and `sequence` with the typed diagnostic, naming the
//! authored path and the replacement, before any provider launches.
//!
//! One `#[test]` per row and command keeps each process spawn its own test;
//! [`every_table_row_generates_its_tests`] keeps the generated list and the
//! table in step.

use std::fs;

use crate::common;
use common::{CliProcessFixture, strip_ansi, write_executable};

/// One retired spelling: the document fragment that authors it and what the
/// diagnostic must say.
struct Row {
    name: &'static str,
    /// Frontmatter lines (each ending in `\n`) added after the mode's own keys.
    frontmatter: &'static str,
    body: &'static str,
    /// The authored path the diagnostic names.
    path: &'static str,
    /// The retired spelling the diagnostic names.
    removed: &'static str,
    /// A fragment of the replacement guidance.
    replacement: &'static str,
}

const ROWS: &[Row] = &[
    Row {
        name: "stack_key",
        frontmatter: "start:\n    stack:\n        - info: hi\n",
        body: "Body\n",
        path: "start.stack",
        removed: "stack",
        replacement: "ordered list of actions",
    },
    Row {
        name: "action_aggregator",
        frontmatter: "start:\n    - action:\n          - info: hi\n",
        body: "Body\n",
        path: "start[0].action",
        removed: "action",
        replacement: "list the actions directly",
    },
    Row {
        name: "action_beside_when",
        frontmatter: "start:\n    - when: \"true\"\n      action:\n          - info: hi\n",
        body: "Body\n",
        path: "start[0].action",
        removed: "action",
        replacement: "under `then:`",
    },
    Row {
        name: "say_first",
        frontmatter: "success:\n    - say_first: true\n      say: done\n",
        body: "Body\n",
        path: "success[0].say_first",
        removed: "say_first",
        replacement: "items run in the order written",
    },
    Row {
        name: "communication_message",
        frontmatter: "success:\n    - say:\n          message: done\n",
        body: "Body\n",
        path: "success[0].say.message",
        removed: "say.message",
        replacement: "name the payload `text`",
    },
    Row {
        name: "communication_sound",
        frontmatter: "success:\n    - info:\n          text: done\n          sound: ping\n",
        body: "Body\n",
        path: "success[0].info.sound",
        removed: "info.sound",
        replacement: "name the payload `text`",
    },
    Row {
        name: "communication_route",
        frontmatter: "success:\n    - message:\n          text: done\n          route: ops\n",
        body: "Body\n",
        path: "success[0].message.route",
        removed: "message.route",
        replacement: "remove `route`",
    },
    Row {
        name: "loop_read_in_frontmatter",
        frontmatter: "note: \"pass {{ _loop_count }}\"\n",
        body: "Body\n",
        path: "note",
        removed: "_loop_count",
        replacement: "loop.count",
    },
    Row {
        name: "loop_read_in_body",
        frontmatter: "",
        body: "Body\n\nLast: {{ _loop_last_output }}\n",
        path: "body:",
        removed: "_loop_last_output",
        replacement: "loop.last_output",
    },
];

/// The command a row runs under, and the frontmatter that command requires.
#[derive(Clone, Copy)]
enum Mode {
    Compose,
    InlineCompose,
    Sequence,
}

impl Mode {
    fn subcommand(self) -> &'static str {
        match self {
            Mode::Compose => "compose",
            Mode::InlineCompose => "inline-compose",
            Mode::Sequence => "sequence",
        }
    }

    fn own_frontmatter(self) -> &'static str {
        match self {
            Mode::Compose => "",
            Mode::InlineCompose => "prompt: Generate the body\n",
            Mode::Sequence => "sequence:\n    - alpha\n",
        }
    }
}

/// Collapse whitespace and block gutters so substring assertions survive the
/// renderer's terminal-width wrapping.
fn normalize(text: &str) -> String {
    text.replace(['┃', '│'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Run `mode` on `document` (plus any `extra` files beside it) with a `goose`
/// stub that writes a marker on any launch; returns (exit code, normalized
/// stderr, whether the provider launched).
fn run(mode: Mode, document: &str, extra: &[(&str, &str)]) -> (Option<i32>, String, bool) {
    let fixture = CliProcessFixture::named("compose-removed-grammar");
    let marker = fixture.cwd().join("provider-launched.txt");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!("#!/bin/sh\necho touched >> '{}'\nexit 0\n", marker.display()),
    );
    let md = fixture.cwd().join("prompt.md");
    fs::write(&md, document).unwrap();
    for (name, content) in extra {
        fs::write(fixture.cwd().join(name), content).unwrap();
    }

    let output = fixture
        .command()
        .args([mode.subcommand(), "--goose", md.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = normalize(&strip_ansi(&String::from_utf8_lossy(&output.stderr)));
    (output.status.code(), stderr, marker.exists())
}

fn assert_rejected(row_name: &str, mode: Mode) {
    let row = ROWS
        .iter()
        .find(|row| row.name == row_name)
        .unwrap_or_else(|| panic!("no table row `{row_name}`"));
    let document = format!("---\n{}{}---\n{}", mode.own_frontmatter(), row.frontmatter, row.body);
    let (code, stderr, launched) = run(mode, &document, &[]);
    let command = mode.subcommand();

    assert_eq!(code, Some(1), "`{command}` exits 1 for `{row_name}`:\n{stderr}");
    assert!(stderr.contains("removed grammar"), "`{command}`/`{row_name}` renders the typed diagnostic:\n{stderr}");
    for needle in [row.path, row.removed, row.replacement] {
        assert!(stderr.contains(needle), "`{command}`/`{row_name}` names `{needle}`:\n{stderr}");
    }
    assert!(!launched, "`{command}`/`{row_name}` must not launch the provider:\n{stderr}");
}

macro_rules! removed_grammar_rows {
    ($($row:ident),* $(,)?) => {
        /// Every row the macro generated tests for, in table order.
        const GENERATED: &[&str] = &[$(stringify!($row)),*];

        $(
            mod $row {
                #[test]
                fn compose() {
                    super::assert_rejected(stringify!($row), super::Mode::Compose);
                }

                #[test]
                fn inline_compose() {
                    super::assert_rejected(stringify!($row), super::Mode::InlineCompose);
                }

                #[test]
                fn sequence() {
                    super::assert_rejected(stringify!($row), super::Mode::Sequence);
                }
            }
        )*
    };
}

removed_grammar_rows!(
    stack_key,
    action_aggregator,
    action_beside_when,
    say_first,
    communication_message,
    communication_sound,
    communication_route,
    loop_read_in_frontmatter,
    loop_read_in_body,
);

#[test]
fn every_table_row_generates_its_tests() {
    let table: Vec<&str> = ROWS.iter().map(|row| row.name).collect();
    assert_eq!(table, GENERATED, "add the row to `removed_grammar_rows!` too");
}

/// A retired read that only a transcluded document introduces is found when
/// the body renders, not by the pre-launch scan, and still stops the run with
/// the typed diagnostic before the provider launches.
#[test]
fn a_retired_read_introduced_by_a_transclusion_is_rejected_at_render() {
    let (code, stderr, launched) = run(
        Mode::Compose,
        "---\ntitle: root\n---\n::file ./part.md\n",
        &[("part.md", "---\ntitle: part\n---\npass {{ _loop_count }}\n")],
    );

    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("removed grammar"), "{stderr}");
    assert!(stderr.contains("_loop_count") && stderr.contains("loop.count"), "{stderr}");
    assert!(!launched, "the provider must not launch:\n{stderr}");
}
