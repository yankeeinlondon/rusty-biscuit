//! The shared output dispatch (`biscuit_terminal::output::write_output`):
//! which source each target reads, where overrides apply, and that nothing
//! is written when output cannot be produced.

use std::cell::RefCell;
use std::io::{self, Write};

use biscuit_terminal::output::{OutputError, OutputTarget, render_output, write_output};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{Document, RenderNode};
use renderable::{CommandOutput, OutputOptions, TerminalProfile, Width};
use serde::Serialize;

#[derive(Serialize)]
struct Data {
    name: &'static str,
    note: Option<&'static str>,
    items: Vec<Option<u32>>,
}

/// A value with no overrides: every formatted target uses a generic fold.
struct Generic {
    items: Vec<Option<u32>>,
}

impl CommandOutput for Generic {
    type Data = Data;

    fn as_prose(&self) -> Document {
        Document::new(RenderNode::root(vec![
            RenderNode::heading(renderable::tree::HeadingDepth::new(1).unwrap(), vec![RenderNode::text("Report")]),
            RenderNode::paragraph(vec![
                RenderNode::text("one two three four five six seven eight nine ten"),
                RenderNode::soft_break(),
                RenderNode::strong(vec![RenderNode::text("bold")]),
            ]),
        ]))
    }

    fn as_data(&self) -> Data {
        Data {
            name: "report",
            note: None,
            items: self.items.clone(),
        }
    }
}

/// A value that overrides `terminal` and records the options it was given.
struct TerminalOverride {
    seen: RefCell<Vec<OutputOptions>>,
}

impl CommandOutput for TerminalOverride {
    type Data = u8;

    fn as_prose(&self) -> Document {
        Document::new(RenderNode::root(Vec::new()))
    }

    fn as_data(&self) -> u8 {
        1
    }

    fn render_terminal(&self, options: &OutputOptions) -> Option<String> {
        self.seen.borrow_mut().push(options.clone());
        Some(if options.terminal_profile.is_plain() {
            "override plain\n".to_owned()
        } else {
            "\u{1b}[1moverride styled\u{1b}[0m\n".to_owned()
        })
    }

    fn render_markdown(&self, options: &OutputOptions) -> Option<String> {
        Some(format!("markdown at {:?}\n", options.width))
    }
}

fn rich(width: Width) -> OutputOptions {
    OutputOptions::new(width, Terminal::new_optimistic(80).output_profile())
}

fn written(value: &impl CommandOutput, target: OutputTarget, options: &OutputOptions) -> String {
    let mut out = Vec::new();
    write_output(value, target, options, &mut out).expect("output is produced");
    String::from_utf8(out).expect("output is UTF-8")
}

#[test]
fn plain_is_the_generic_terminal_fold_with_the_plain_profile() {
    let value = Generic { items: vec![] };
    let plain = written(&value, OutputTarget::Plain, &rich(Width::Columns(20)));
    assert!(!plain.contains('\u{1b}'), "{plain:?}");
    assert!(plain.contains("Report") && !plain.contains("# Report"), "{plain}");
    assert!(plain.contains("bold") && !plain.contains("**"), "{plain}");
    assert!(plain.lines().all(|line| line.chars().count() <= 20), "wrapped at 20: {plain}");
    assert!(plain.lines().all(|line| line == line.trim_end()), "{plain:?}");
}

#[test]
fn terminal_degrades_to_exactly_plain_off_a_color_tty() {
    let value = Generic { items: vec![] };
    let mut piped = rich(Width::Columns(30));
    piped.terminal_profile.is_tty = false;
    assert_eq!(
        written(&value, OutputTarget::Terminal, &piped),
        written(&value, OutputTarget::Plain, &rich(Width::Columns(30)))
    );
}

#[test]
fn styled_terminal_uses_the_profile() {
    let value = Generic { items: vec![] };
    let styled = written(&value, OutputTarget::Terminal, &rich(Width::Columns(30)));
    assert!(styled.contains('\u{1b}'), "a color TTY gets styling: {styled:?}");
}

#[test]
fn a_terminal_override_also_serves_plain_with_the_plain_profile() {
    let value = TerminalOverride {
        seen: RefCell::new(Vec::new()),
    };
    let options = rich(Width::Columns(50)).with_verbosity(1);
    assert_eq!(written(&value, OutputTarget::Terminal, &options), "\u{1b}[1moverride styled\u{1b}[0m\n");
    assert_eq!(written(&value, OutputTarget::Plain, &options), "override plain\n");
    let seen = value.seen.borrow();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].terminal_profile, TerminalProfile::plain());
    assert_eq!(seen[1].width, Width::Columns(50));
    assert_eq!(seen[1].verbosity, 1);
}

#[test]
fn overrides_apply_per_target_and_others_fall_back() {
    let value = TerminalOverride {
        seen: RefCell::new(Vec::new()),
    };
    let options = rich(Width::Unlimited);
    assert_eq!(written(&value, OutputTarget::Markdown, &options), "markdown at Unlimited\n");
    // No markdown-plus or html override: the generic folds run.
    let html = written(&value, OutputTarget::Html, &options);
    assert!(html.contains("<html") || html.contains("<!DOCTYPE"), "{html}");
}

#[test]
fn markdown_falls_back_to_the_fold_at_the_given_width() {
    let value = Generic { items: vec![] };
    let source = written(&value, OutputTarget::Markdown, &rich(Width::Source));
    assert!(source.contains("# Report"), "{source}");
    assert!(source.contains("ten\n**bold**"), "source keeps the soft break: {source:?}");
    let unlimited = written(&value, OutputTarget::Markdown, &rich(Width::Unlimited));
    assert!(unlimited.contains("ten **bold**"), "unlimited joins it: {unlimited:?}");
    let plus = written(&value, OutputTarget::MarkdownPlus, &rich(Width::Source));
    assert_eq!(plus, source, "no richer form: markdown-plus equals markdown");
}

#[test]
fn html_columns_constrain_prose() {
    let value = Generic { items: vec![] };
    let html = written(&value, OutputTarget::Html, &rich(Width::Columns(64)));
    assert!(html.contains("max-width: 64ch"), "{html}");
    let unconstrained = written(&value, OutputTarget::Html, &rich(Width::Unlimited));
    assert!(!unconstrained.contains("ch;}"), "{unconstrained}");
}

#[test]
fn data_targets_serialize_the_same_data() {
    let value = Generic {
        items: vec![Some(1), Some(2)],
    };
    let options = rich(Width::Columns(10));
    let json = written(&value, OutputTarget::Json, &options);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, serde_json::json!({ "name": "report", "note": null, "items": [1, 2] }));
    let json5 = written(&value, OutputTarget::Json5, &options);
    assert!(json5.contains("note: null"), "{json5}");
    let yaml = written(&value, OutputTarget::Yaml, &options);
    assert!(yaml.contains("note: null"), "{yaml}");
    let toml = written(&value, OutputTarget::Toml, &options);
    assert!(!toml.contains("note"), "TOML omits a null field: {toml}");
    assert!(toml.contains("name = \"report\""), "{toml}");
    let reparsed: biscuit_file::toml_crate::Table = toml.parse().expect("valid TOML");
    assert_eq!(reparsed["items"].as_array().map(Vec::len), Some(2), "{toml}");
}

#[test]
fn a_serialization_failure_writes_nothing() {
    let value = Generic {
        items: vec![Some(1), None],
    };
    let mut out = Vec::new();
    let err = write_output(&value, OutputTarget::Toml, &rich(Width::Unlimited), &mut out).unwrap_err();
    match err {
        OutputError::Serialize { target, path, .. } => {
            assert_eq!(target, OutputTarget::Toml);
            assert_eq!(path.as_deref(), Some("/items/1"));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(out.is_empty(), "nothing reaches the writer");
    // The same data is fine where null is representable.
    assert!(render_output(&value, OutputTarget::Json, &rich(Width::Unlimited)).is_ok());
}

struct BrokenPipe;

impl Write for BrokenPipe {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_broken_writer_is_an_io_error() {
    let value = Generic { items: vec![] };
    let err = write_output(&value, OutputTarget::Json, &rich(Width::Unlimited), &mut BrokenPipe)
        .unwrap_err();
    assert!(matches!(err, OutputError::Io(ref io) if io.kind() == io::ErrorKind::BrokenPipe), "{err:?}");
}

#[test]
fn unlimited_terminal_output_has_its_natural_size() {
    let value = Generic { items: vec![] };
    let mut options = rich(Width::Unlimited);
    let styled = written(&value, OutputTarget::Terminal, &options);
    let widest = styled.lines().map(str::len).max().unwrap_or(0);
    assert!(widest < 200, "no line is filled to a sentinel width: {widest}");
    options.terminal_profile = TerminalProfile::plain();
    let plain = written(&value, OutputTarget::Plain, &options);
    assert!(
        plain.contains("one two three four five six seven eight nine ten bold"),
        "unlimited joins soft breaks and never wraps: {plain:?}"
    );
}
