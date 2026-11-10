//! The render-surface examples in the published docs are valid against the
//! current CLI: every `md` command in a shell fence parses through `Cli`
//! (so a removed flag or a misplaced control fails here), every render
//! command prepares, and its `style…=VALUE` setters prepare through the same
//! style preparation `md render` runs. Every YAML fence with a root `style:`
//! key prepares as a document layer with no warning.
//!
//! An example that shows a usage failure says so in a trailing shell comment
//! (or, in YAML, any comment) containing `exit 2` or `usage error`; such an
//! example must fail, so the marker cannot hide a valid one. A comment that
//! names a runtime failure is not a marker: this test judges only what is
//! decided before a document is read.
//!
//! Each doc is read with `include_str!` inside its own `#[test]`, so a change
//! to that doc runs its test.

use clap::Parser as _;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::style::StylePreparationPolicy;
use darkmatter_cli::Cli;
use darkmatter_cli::args::{Command, PreparedRenderOptions};
use darkmatter_cli::commands::validate_subcommand_usage;
use darkmatter_cli::render_style::{document_style_layer, prepare_cli_style_inputs};
use darkmatter_cli::request::MdRequest;
use tempfile::TempDir;

const FAILURE_MARKERS: [&str; 2] = ["exit 2", "usage error"];

fn marks_failure(comment: &str) -> bool {
    let comment = comment.to_ascii_lowercase();
    FAILURE_MARKERS.iter().any(|marker| comment.contains(marker))
}

/// One fenced code block: its info string and body.
struct Fence {
    info: String,
    body: String,
    line: usize,
}

fn fences(doc: &str) -> Vec<Fence> {
    let mut fences = Vec::new();
    let mut open: Option<(String, String, usize, String)> = None;
    for (index, line) in doc.lines().enumerate() {
        let trimmed = line.trim_start();
        match &mut open {
            None => {
                for marker in ["```", "~~~"] {
                    if let Some(info) = trimmed.strip_prefix(marker) {
                        let info = info.trim_start_matches(marker.chars().next().unwrap_or('`'));
                        open = Some((marker.to_string(), info.trim().to_string(), index + 2, String::new()));
                        break;
                    }
                }
            }
            Some((marker, info, start, body)) => {
                if trimmed.starts_with(marker.as_str()) {
                    fences.push(Fence { info: std::mem::take(info), body: std::mem::take(body), line: *start });
                    open = None;
                } else {
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }
    }
    fences
}

/// Splits one shell line into words (single and double quotes, backslash
/// escapes) and its trailing `#` comment.
fn shell_words(line: &str) -> (Vec<String>, String) {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\'' => {
                in_word = true;
                for quoted in chars.by_ref() {
                    if quoted == '\'' {
                        break;
                    }
                    word.push(quoted);
                }
            }
            '"' => {
                in_word = true;
                while let Some(quoted) = chars.next() {
                    match quoted {
                        '"' => break,
                        '\\' => word.extend(chars.next()),
                        other => word.push(other),
                    }
                }
            }
            '\\' => {
                in_word = true;
                word.extend(chars.next());
            }
            '#' if !in_word => return (words, chars.collect()),
            whitespace if whitespace.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            other => {
                in_word = true;
                word.push(other);
            }
        }
    }
    if in_word {
        words.push(word);
    }
    (words, String::new())
}

/// Every `md` command in the doc's shell fences: its argv (with `md` first),
/// its comment, and its line.
fn md_commands(doc: &str) -> Vec<(Vec<String>, String, usize)> {
    let mut commands = Vec::new();
    for fence in fences(doc) {
        if !matches!(fence.info.split_whitespace().next(), Some("sh" | "bash" | "shell" | "console" | "zsh")) {
            continue;
        }
        let mut pending = String::new();
        for (offset, raw) in fence.body.lines().enumerate() {
            let line = raw.trim();
            if let Some(continued) = line.strip_suffix('\\') {
                pending.push_str(continued);
                pending.push(' ');
                continue;
            }
            pending.push_str(line);
            let logical = std::mem::take(&mut pending);
            let logical = logical.trim().strip_prefix("$ ").unwrap_or(logical.trim()).to_string();
            let (words, comment) = shell_words(&logical);
            for segment in words.split(|word| matches!(word.as_str(), "|" | "&&" | "||" | ";")) {
                let mut argv: Vec<String> = Vec::new();
                let mut skip_next = false;
                let mut started = false;
                for word in segment {
                    if skip_next {
                        skip_next = false;
                        continue;
                    }
                    if !started {
                        // Leading `NAME=value` environment assignments.
                        if word.contains('=') && !word.starts_with('-') && word != "md" {
                            continue;
                        }
                        if word != "md" {
                            break;
                        }
                        started = true;
                    }
                    if matches!(word.as_str(), ">" | ">>" | "<" | "2>" | "2>&1" | "&>") {
                        skip_next = word != "2>&1";
                        continue;
                    }
                    if word.starts_with('>') || word.starts_with("2>") {
                        continue;
                    }
                    argv.push(word.clone());
                }
                if started {
                    commands.push((argv, comment.clone(), fence.line + offset));
                }
            }
        }
    }
    commands
}

/// A temporary launch directory with its own home.
struct Launch {
    _dir: TempDir,
    request: MdRequest,
}

impl Launch {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("home")).expect("home");
        let request = MdRequest::new(RequestSnapshot::new(&root).with_home(Some(root.join("home"))));
        Self { _dir: dir, request }
    }

    /// Parses `argv` as `md` does and, for a render, prepares its options and
    /// its setters (style files and the input are not opened).
    fn check_command(&self, argv: &[String]) -> Result<(), String> {
        let cli = Cli::try_parse_from(argv).map_err(|error| format!("parse: {error}"))?;
        if cli.command.is_some() {
            validate_subcommand_usage(&cli).map_err(|error| format!("placement: {error}"))?;
        }
        let Command::Render(options) = cli.command_or_render() else {
            return Ok(());
        };
        let prepared = PreparedRenderOptions::prepare(&options).map_err(|error| format!("prepare: {error}"))?;
        if prepared.setters.is_empty() {
            return Ok(());
        }
        let markdown = Markdown::from("# Example\n");
        let inputs = prepare_cli_style_inputs(&markdown, None, &[], &prepared.setters, &self.request)
            .map_err(|error| format!("setters: {error:#}"))?;
        inputs
            .prepare(None, StylePreparationPolicy::render())
            .map(|_| ())
            .map_err(|error| format!("style: {error:#}"))
    }

    /// Prepares `frontmatter` (a YAML mapping with a root `style:`) as a
    /// document's style layer, failing on any warning.
    fn check_frontmatter(&self, frontmatter: &str) -> Result<(), String> {
        let markdown = Markdown::from(format!("---\n{frontmatter}---\n# Example\n").as_str());
        let context = self.request.launch_context().map_err(|error| format!("{error:#}"))?.clone();
        let layer = document_style_layer(&markdown, None, context);
        let inputs = prepare_cli_style_inputs(&markdown, None, &[], &[], &self.request).map_err(|error| format!("{error:#}"))?;
        let prepared = inputs.prepare(Some(layer), StylePreparationPolicy::render()).map_err(|error| format!("{error:#}"))?;
        let warnings: Vec<String> = inputs.warnings(&prepared).into_iter().map(|warning| warning.message).collect();
        if warnings.is_empty() { Ok(()) } else { Err(format!("warnings: {warnings:?}")) }
    }
}

/// Whether a YAML fence is a document's frontmatter with a root `style:` key.
fn is_style_frontmatter(body: &str) -> bool {
    body.lines().any(|line| line.starts_with("style:"))
}

fn check_doc(name: &str, doc: &str) {
    let launch = Launch::new();
    let mut problems = Vec::new();
    let mut checked = 0;
    for (argv, comment, line) in md_commands(doc) {
        checked += 1;
        let expect_failure = marks_failure(&comment);
        match (launch.check_command(&argv), expect_failure) {
            (Ok(()), false) | (Err(_), true) => {}
            (Ok(()), true) => problems.push(format!("{name}:{line}: {argv:?} is marked as failing but succeeds")),
            (Err(error), false) => problems.push(format!("{name}:{line}: {argv:?}: {error}")),
        }
    }
    for fence in fences(doc) {
        if !matches!(fence.info.split_whitespace().next(), Some("yaml" | "yml")) || !is_style_frontmatter(&fence.body) {
            continue;
        }
        checked += 1;
        let body = fence.body.trim_start_matches("---\n").replace("\n---\n", "\n");
        let expect_failure = body.lines().filter_map(|line| line.split_once('#').map(|(_, comment)| comment)).any(marks_failure);
        match (launch.check_frontmatter(&body), expect_failure) {
            (Ok(()), false) | (Err(_), true) => {}
            (Ok(()), true) => problems.push(format!("{name}:{}: YAML example is marked as failing but prepares", fence.line)),
            (Err(error), false) => problems.push(format!("{name}:{}: YAML example: {error}", fence.line)),
        }
    }
    assert!(checked > 0, "{name}: no example was found; the extractor no longer matches the doc");
    assert!(problems.is_empty(), "{} invalid example(s):\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn shell_words_handle_quotes_and_comments() {
    let (words, comment) = shell_words(r#"md doc.md 'style.page.margin-left=4' "a b" # exit 2 here"#);
    assert_eq!(words, ["md", "doc.md", "style.page.margin-left=4", "a b"]);
    assert!(marks_failure(&comment));
}

#[test]
fn a_removed_flag_in_an_example_is_reported() {
    let doc = "```sh\nmd doc.md --max-width 40\nmd doc.md --max-width 40 # exit 2\n```\n";
    let launch = Launch::new();
    let commands = md_commands(doc);
    assert_eq!(commands.len(), 2);
    assert!(launch.check_command(&commands[0].0).is_err());
    assert!(marks_failure(&commands[1].1));
}

#[test]
fn an_invalid_setter_in_an_example_is_reported() {
    let launch = Launch::new();
    let argv: Vec<String> = ["md", "doc.md", "style.page.margin-left=wide"].map(String::from).to_vec();
    assert!(launch.check_command(&argv).is_err());
    assert!(launch.check_frontmatter("style:\n  page:\n    glow: 1\n").is_err(), "an unknown key warns");
    assert!(launch.check_frontmatter("style:\n  page:\n    margin-left: 4\n").is_ok());
}

#[test]
fn render_doc_examples_are_valid() {
    check_doc("docs/cli/render.md", include_str!("../../../docs/cli/render.md"));
}

#[test]
fn index_doc_examples_are_valid() {
    check_doc("docs/cli/index.md", include_str!("../../../docs/cli/index.md"));
}

#[test]
fn output_and_width_doc_examples_are_valid() {
    check_doc("docs/cli/output-and-width.md", include_str!("../../../docs/cli/output-and-width.md"));
}

#[test]
fn data_contracts_doc_examples_are_valid() {
    check_doc("docs/cli/data-contracts.md", include_str!("../../../docs/cli/data-contracts.md"));
}

#[test]
fn policy_doc_examples_are_valid() {
    check_doc("docs/cli/policy.md", include_str!("../../../docs/cli/policy.md"));
}

#[test]
fn cli_readme_examples_are_valid() {
    check_doc("cli/README.md", include_str!("../../README.md"));
}

#[test]
fn layouts_and_style_doc_examples_are_valid() {
    check_doc("docs/topics/layouts-and-style.md", include_str!("../../../docs/topics/layouts-and-style.md"));
}

#[test]
fn style_doc_examples_are_valid() {
    check_doc("docs/rendering/style.md", include_str!("../../../docs/rendering/style.md"));
}

#[test]
fn mermaid_doc_examples_are_valid() {
    check_doc("docs/rendering/mermaid.md", include_str!("../../../docs/rendering/mermaid.md"));
}

#[test]
fn code_highlighting_doc_examples_are_valid() {
    check_doc("docs/rendering/code-highlighting.md", include_str!("../../../docs/rendering/code-highlighting.md"));
}

/// `docs/cli/index.md` shows `md --help` verbatim (up to trailing spaces),
/// so a flag added, removed, or reworded must update the sample.
#[test]
fn index_doc_help_sample_matches_generated_help() {
    use clap::CommandFactory as _;
    let doc = include_str!("../../../docs/cli/index.md");
    let sample = fences(doc)
        .into_iter()
        .find(|fence| fence.body.contains("Usage: md"))
        .expect("docs/cli/index.md shows `md --help`");
    let generated = Cli::command().bin_name("md").render_long_help().to_string();
    let normalize = |text: &str| text.trim().lines().map(str::trim_end).collect::<Vec<_>>().join("\n");
    assert_eq!(normalize(&sample.body), normalize(&generated), "regenerate the sample in docs/cli/index.md with `md --help`");
}
