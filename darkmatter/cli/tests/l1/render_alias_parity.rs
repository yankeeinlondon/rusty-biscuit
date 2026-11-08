//! `md ARGS…` is `md render ARGS…`: the two spellings parse, classify,
//! prepare, and style every render argument the same way, and print the same
//! bytes.
//!
//! The argument inventory is read from clap (`RenderOptions`' own
//! definition), and the setter inventory from the document's effective
//! schema, so a new option or style field joins these checks or fails a
//! guard; nothing is skipped silently. The comparisons run in process
//! against a temporary directory; only the byte, usage-error, and input-form
//! rows spawn `md`, one `#[test]` per row.
//!
//! Related coverage elsewhere: `render_dispatch` (spelling parity per
//! target, injected no-input stdin), `args::render::tests`
//! (`every_root_render_argument_before_a_subcommand_is_a_usage_error`, in
//! process), `help` (root and render help, `--version`, `--completions`),
//! and `path_completion` (the `COMPLETE=` dynamic path). The controlled-PTY
//! no-input wiring test is `level2::level2_render_no_input`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use biscuit_terminal::output::OutputTarget;
use clap::{Args as _, CommandFactory as _, Parser as _};
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::schemas::completion::{
    PathCandidateKind, PathCommand, PathPosition, PathQuery, PathSchema, PathStep, declared_shape, path_candidates,
};
use darkmatter::style::StylePreparationPolicy;
use darkmatter_cli::Cli;
use darkmatter_cli::args::{Command, PreparedRenderOptions, RenderOptions};
use darkmatter_cli::io::load_opened_document;
use darkmatter_cli::render_style::{document_style_layer, prepare_cli_style_inputs, prepare_render_style};
use darkmatter_cli::request::MdRequest;
use serde_json::Value;
use simplified_schema::coercion::Shape;
use tempfile::TempDir;

use crate::common::CliProcessFixture;

// ── Inventory ───────────────────────────────────────────────────────────────

/// The document every in-process row renders: its own `style:` is the bottom
/// layer the CLI layers merge over.
const DOCUMENT: &str = "---\ntitle: Parity\nstyle:\n  page:\n    margin-left: 3\n---\n# Title\n\nSome body text.\n";

/// The `--style` file the rows name.
const STYLE_FILE: &str = "page:\n  margin-top: 1\n";

/// Shared controls: `global` on the root, so either spelling accepts them in
/// any position.
const SHARED: &[&str] = &["width", "verbose", "debug_level"];

/// Root-only launch configuration and early actions: never render arguments,
/// so `md render` does not take them (`root_only_arguments_are_not_render_arguments`).
const ROOT_ONLY: &[(&str, &[&str])] = &[
    ("magic_roots", &["--magic-root", "."]),
    ("completions", &["--completions", "zsh"]),
    ("version", &["--version"]),
];

/// One representative per render argument (clap id → tokens), appended after
/// the input `doc.md`. Every id clap declares must be here
/// (`every_render_argument_has_a_representative`).
fn representatives() -> BTreeMap<&'static str, &'static [&'static str]> {
    BTreeMap::from([
        ("args", &["style.page.margin-left=2"][..]),
        // Its row is in `OVERRIDES`: it replaces the `doc.md` input.
        ("literal_args", &["--", "doc.md"][..]),
        ("style", &["--style", "style.yaml"][..]),
        ("output", &["--output", "html"][..]),
        ("show", &["--show"][..]),
        ("indent", &["--indent", "4"][..]),
        ("deny", &["--deny", "style"][..]),
        ("features", &["--feature", "embed-mermaid-font"][..]),
        ("mermaid", &["--mermaid"][..]),
    ])
}

/// Rows whose whole argument list is given, because the representative alone
/// would conflict with the `doc.md` every other row starts with, or because
/// the combination is the point.
const OVERRIDES: &[(&str, &[&str])] = &[
    // After `--` the input is the literal argument; `doc.md` before it too
    // would be two inputs.
    ("literal_args", &["--", "doc.md"]),
    ("two inputs", &["doc.md", "other.md"]),
    ("an input and a literal input", &["doc.md", "--", "other.md"]),
    ("a setter after the terminator is a file", &["--", "style.page.margin-left=2"]),
    ("an indent the parser refuses", &["doc.md", "--indent", "3"]),
    ("an unknown target", &["doc.md", "--output", "bogus"]),
    ("an unknown feature", &["doc.md", "--feature", "bogus"]),
    ("a setter that does not parse", &["doc.md", "style..x=1"]),
    ("a setter the schema refuses", &["doc.md", "style.page.margin-left=abc"]),
    ("repeated setters, the last wins", &["doc.md", "style.ml=1", "style.ml=5"]),
    ("a whole style object", &["doc.md", "style={ page: { margin-left: 4 } }"]),
    ("a missing style file", &["doc.md", "--style", "missing.yaml"]),
    (
        "every argument at once",
        &[
            "style.page.margin-left=2",
            "--style",
            "style.yaml",
            "--output",
            "json",
            "--show",
            "--indent",
            "2",
            "--deny",
            "style",
            "--feature",
            "no-mermaid-rendering",
            "--mermaid",
            "--width",
            "60",
            "-v",
            "--",
            "doc.md",
        ],
    ),
    ("shared controls only", &["doc.md", "--width", "60", "-v", "--debug", "1"]),
];

/// The ids clap declares for `RenderOptions`, in declaration order.
fn render_arg_ids() -> Vec<String> {
    RenderOptions::augment_args(clap::Command::new("render"))
        .get_arguments()
        .map(|arg| arg.get_id().as_str().to_string())
        .collect()
}

/// Every in-process row: each representative, each value clap lists for an
/// option with possible values, then the overrides.
fn argument_rows() -> Vec<(String, Vec<String>)> {
    let representatives = representatives();
    let mut rows = Vec::new();
    let definition = RenderOptions::augment_args(clap::Command::new("render"));
    for arg in definition.get_arguments() {
        let id = arg.get_id().as_str();
        let tokens = representatives
            .get(id)
            .unwrap_or_else(|| panic!("render argument `{id}` has no representative in `representatives()`"));
        if !OVERRIDES.iter().any(|(name, _)| *name == id) {
            rows.push((id.to_string(), with_input(tokens)));
        }
        if let Some(long) = arg.get_long() {
            for value in arg.get_possible_values() {
                let value = value.get_name();
                rows.push((format!("{id}={value}"), with_input(&[&format!("--{long}"), value])));
            }
        }
    }
    rows.extend(OVERRIDES.iter().map(|(name, tokens)| (name.to_string(), strings(tokens))));
    rows
}

fn with_input(tokens: &[&str]) -> Vec<String> {
    std::iter::once("doc.md").chain(tokens.iter().copied()).map(String::from).collect()
}

fn strings(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|token| (*token).to_string()).collect()
}

#[test]
fn every_render_argument_has_a_representative() {
    let declared: BTreeSet<String> = render_arg_ids().into_iter().collect();
    let represented: BTreeSet<String> = representatives().keys().map(|id| (*id).to_string()).collect();
    assert_eq!(declared, represented, "each render argument needs exactly one representative");
}

/// Every root argument is a render argument, a shared control, or root-only,
/// so a new root argument has to be placed before these checks pass.
#[test]
fn every_root_argument_is_classified() {
    let mut root = Cli::command();
    root.build();
    let declared: BTreeSet<String> = root.get_arguments().map(|arg| arg.get_id().as_str().to_string()).collect();
    let classified: BTreeSet<String> = render_arg_ids()
        .into_iter()
        .chain(SHARED.iter().map(|id| (*id).to_string()))
        .chain(ROOT_ONLY.iter().map(|(id, _)| (*id).to_string()))
        .chain(["help".to_string()])
        .collect();
    assert_eq!(declared, classified);

    // The `render` subcommand has every render argument and shared control.
    let render = root.find_subcommand("render").expect("render subcommand");
    let explicit: BTreeSet<String> = render.get_arguments().map(|arg| arg.get_id().as_str().to_string()).collect();
    for id in render_arg_ids().iter().map(String::as_str).chain(SHARED.iter().copied()) {
        assert!(explicit.contains(id), "`md render` lacks `{id}`: {explicit:?}");
    }
}

#[test]
fn root_only_arguments_are_not_render_arguments() {
    for (id, tokens) in ROOT_ONLY {
        let explicit: Vec<&str> = ["md", "render"].into_iter().chain(tokens.iter().copied()).collect();
        let error = Cli::try_parse_from(&explicit).err().unwrap_or_else(|| panic!("{id}: {explicit:?} parsed"));
        assert_eq!(error.exit_code(), 2, "{id}: {error}");
        let root: Vec<&str> = std::iter::once("md").chain(tokens.iter().copied()).collect();
        let parsed = Cli::try_parse_from(&root);
        // `--version` is clap's own early exit (status 0), not a parse.
        if let Err(error) = parsed {
            assert_eq!(error.exit_code(), 0, "{id}: {error}");
        }
    }
}

// ── In-process comparison ───────────────────────────────────────────────────

/// A temporary launch directory holding the document and the style file,
/// with its own home; every request is anchored here.
struct Local {
    _dir: TempDir,
    request: MdRequest,
}

impl Local {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("home")).expect("home");
        std::fs::write(root.join("doc.md"), DOCUMENT).expect("doc.md");
        std::fs::write(root.join("style.yaml"), STYLE_FILE).expect("style.yaml");
        let request = MdRequest::new(RequestSnapshot::new(&root).with_home(Some(root.join("home"))));
        Self { _dir: dir, request }
    }

    /// Everything one spelling decided, as text: the parse (or its error kind
    /// and status), the shared controls, the prepared options, and the
    /// effective style.
    fn outcome(&self, argv: &[&str]) -> String {
        let cli = match Cli::try_parse_from(argv) {
            Ok(cli) => cli,
            Err(error) => return format!("usage error {:?}, status {}", error.kind(), error.exit_code()),
        };
        let options = match cli.command_or_render() {
            Command::Render(options) => options,
            other => panic!("{argv:?} should render, got {other:?}"),
        };
        if cli.command.is_some() {
            assert!(cli.render.has_explicit_arguments().is_empty(), "{argv:?}: a root value beside `render`");
        }
        let shared = format!(
            "width {:?}, verbose {}, debug {:?}, data output {}",
            cli.width,
            cli.verbose,
            cli.debug_level,
            cli.requests_data_output()
        );
        let prepared = match PreparedRenderOptions::prepare(&options) {
            Ok(prepared) => prepared,
            Err(error) => return format!("{shared}\nprepare error {:?}, status {}: {error}", error.kind(), error.exit_code()),
        };
        format!("{shared}\n{prepared:?}\n{}", self.effective_style(&prepared))
    }

    /// The effective style of a prepared render, as `md render` prepares it
    /// for a page, or the failure's message chain.
    fn effective_style(&self, prepared: &PreparedRenderOptions) -> String {
        let request = &self.request;
        let style = (|| {
            let (markdown, opened) = load_opened_document(prepared.input.as_ref(), request)?;
            let inputs = prepare_cli_style_inputs(&markdown, opened.as_ref(), &prepared.styles, &prepared.setters, request)?;
            let context = match &opened {
                Some(opened) => opened.document_context(request)?,
                None => request.launch_context()?.clone(),
            };
            let layer = document_style_layer(&markdown, opened.as_ref().map(|opened| opened.path()), context);
            let policy = StylePreparationPolicy::render().with_deny_style(prepared.policy.denies("style"));
            prepare_render_style(&inputs, layer, policy, true)
        })();
        match style {
            Ok(style) => {
                let warnings: Vec<&str> = style.warnings.iter().map(|warning| warning.record.message.as_str()).collect();
                // The origin's context is the launch or document context the
                // layer was read in, the same for both spellings by construction.
                let origins: Vec<String> = style
                    .style
                    .origins()
                    .iter()
                    .map(|(path, origin)| format!("{path} <- {:?} {} ({}) {:?}", origin.kind, origin.source, origin.written, origin.span))
                    .collect();
                format!(
                    "value {}\nretained {}\nprojection {:?}\norigins {origins:?}\nwarnings {warnings:?}",
                    style.style.value(),
                    style.style.retained(),
                    style.style.style(),
                )
            }
            Err(error) => format!("style error: {error:#}"),
        }
    }

    /// Asserts `md TOKENS…` and `md render TOKENS…` decide the same, and
    /// returns that outcome.
    fn assert_spellings_agree(&self, name: &str, tokens: &[String]) -> String {
        let tokens: Vec<&str> = tokens.iter().map(String::as_str).collect();
        let implicit: Vec<&str> = std::iter::once("md").chain(tokens.iter().copied()).collect();
        let explicit: Vec<&str> = ["md", "render"].into_iter().chain(tokens.iter().copied()).collect();
        let outcome = self.outcome(&implicit);
        assert_eq!(outcome, self.outcome(&explicit), "{name}: {tokens:?}");
        outcome
    }
}

#[test]
fn every_render_argument_decides_the_same_in_both_spellings() {
    let local = Local::new();
    let mut outcomes = BTreeMap::new();
    for (name, tokens) in argument_rows() {
        outcomes.insert(name.clone(), local.assert_spellings_agree(&name, &tokens));
    }
    // The rows are not vacuous: the representatives parse and style, and
    // the refusals are refused.
    let succeeded = |name: &str| outcomes[name].contains("\nvalue {");
    for name in [
        "args",
        "literal_args",
        "style",
        "output",
        "deny",
        "repeated setters, the last wins",
        "a whole style object",
        "every argument at once",
        "shared controls only",
    ] {
        assert!(succeeded(name), "{name}: {}", outcomes[name]);
    }
    assert!(outcomes["args"].contains("\"margin-left\":\"2\""), "{}", outcomes["args"]);
    assert!(outcomes["style"].contains("\"margin-top\":1"), "{}", outcomes["style"]);
    for name in ["an indent the parser refuses", "an unknown target", "an unknown feature"] {
        assert!(outcomes[name].starts_with("usage error"), "{name}: {}", outcomes[name]);
    }
    for name in ["two inputs", "an input and a literal input"] {
        assert!(outcomes[name].contains("prepare error") && outcomes[name].contains("status 2"), "{name}: {}", outcomes[name]);
    }
    for name in ["a setter that does not parse", "a setter the schema refuses", "a missing style file", "a setter after the terminator is a file"] {
        assert!(outcomes[name].contains("style error"), "{name}: {}", outcomes[name]);
    }
}

// ── Setter inventory ────────────────────────────────────────────────────────

/// The value family of a declared shape: what kind of text a setter for it
/// must hold.
fn family(shape: &Shape) -> String {
    match shape {
        Shape::Deferrable(inner) | Shape::Assert(inner, _) => family(inner),
        Shape::Union(arms) => {
            let mut families: Vec<String> = arms.iter().map(family).collect();
            families.sort();
            families.dedup();
            families.join("|")
        }
        Shape::Any => "any".into(),
        Shape::Null => "null".into(),
        Shape::Number(_) => "number".into(),
        Shape::Boolean { .. } => "boolean".into(),
        Shape::Text(text) if text.members.is_some() => "enum".into(),
        Shape::Text(text) if text.pattern.is_some() => "pattern".into(),
        Shape::Text(_) => "text".into(),
        Shape::Literal(_) => "literal".into(),
        Shape::Content(_) => "content".into(),
        Shape::Array(_) => "array".into(),
        Shape::Tuple(_) => "tuple".into(),
        Shape::Object(_) => "object".into(),
        Shape::Ref(_) => "ref".into(),
    }
}

/// A representative value for each family whose paths the schema offers no
/// value for. A family that completes values uses its first completion.
const FAMILY_VALUES: &[(&str, &str)] = &[
    ("number", "2"),
    ("number|pattern", "60"),
    ("number|text", "1"),
    ("object", "{}"),
    ("text", "red"),
];

/// Every settable `style` path the document's effective schema offers,
/// walked through the completion `md render` uses, with its value family.
struct SetterInventory {
    schema: darkmatter::markdown::schemas::EffectiveSchema,
    /// `(key, family)`, in path order.
    paths: Vec<(String, String)>,
}

impl SetterInventory {
    fn walk(local: &Local) -> Self {
        let (markdown, opened) = load_opened_document(Some(&PathBuf::from("doc.md")), &local.request).expect("doc.md opens");
        let inputs = prepare_cli_style_inputs(&markdown, opened.as_ref(), &[], &[], &local.request).expect("schema selection");
        let schema = inputs.schema.expect("the document has an effective schema");
        let mut inventory = Self { schema, paths: Vec::new() };
        let mut pending = vec![String::from("style.")];
        let mut keys = BTreeSet::new();
        while let Some(prefix) = pending.pop() {
            for candidate in inventory.candidates(PathPosition::Path(&prefix)) {
                match candidate.kind {
                    PathCandidateKind::Descend => pending.push(candidate.insert),
                    PathCandidateKind::Assign => {
                        keys.insert(candidate.insert.strip_suffix('=').expect("an assignment ends with `=`").to_string());
                    }
                    other => panic!("`{}` is a {other:?} candidate, which a key walk never offers", candidate.insert),
                }
            }
        }
        let no_frontmatter = Value::Object(serde_json::Map::new());
        inventory.paths = keys
            .into_iter()
            .map(|key| {
                let steps: Vec<PathStep<'_>> = key.split('.').map(PathStep::Key).collect();
                let family = declared_shape(&inventory.schema, &no_frontmatter, &steps, None)
                    .map_or_else(|| "undeclared".to_string(), |shape| family(&shape));
                (key, family)
            })
            .collect();
        inventory
    }

    fn candidates(&self, position: PathPosition<'_>) -> Vec<darkmatter::markdown::schemas::completion::PathCandidate> {
        let no_frontmatter = Value::Object(serde_json::Map::new());
        path_candidates(&PathQuery {
            schema: PathSchema::Effective(&self.schema),
            frontmatter: &no_frontmatter,
            command: PathCommand::Render,
            position,
            earlier_paths: &[],
        })
    }

    /// The setters compared: one per value family and final key (`page`'s
    /// `margin-left` stands for every component's, which share one field
    /// validator), the first in path order. Each value is the path's first
    /// value completion, else its family's [`FAMILY_VALUES`] entry.
    ///
    /// ## Panics
    ///
    /// A sampled path with neither, naming every such family.
    fn sample(&self) -> Vec<(String, &str)> {
        let family_values: BTreeMap<&str, &str> = FAMILY_VALUES.iter().copied().collect();
        let mut seen = BTreeSet::new();
        let mut unrepresented = BTreeSet::new();
        let sample = self
            .paths
            .iter()
            .filter(|(key, family)| seen.insert((family.as_str(), key.rsplit('.').next().unwrap_or(key))))
            .filter_map(|(key, family)| {
                let lead = format!("{key}=");
                if let Some(first) = self.candidates(PathPosition::Path(&lead)).into_iter().next() {
                    return Some((first.insert, family.as_str()));
                }
                match family_values.get(family.as_str()) {
                    Some(value) => Some((format!("{lead}{value}"), family.as_str())),
                    None => {
                        unrepresented.insert(format!("{key}: {family}"));
                        None
                    }
                }
            })
            .collect();
        assert!(unrepresented.is_empty(), "add a `FAMILY_VALUES` entry for: {unrepresented:#?}");
        sample
    }
}

#[test]
fn every_style_setter_family_decides_the_same_in_both_spellings() {
    let local = Local::new();
    let inventory = SetterInventory::walk(&local);
    // The built-in style alone declares hundreds of paths; a short walk means
    // the schema was not found.
    assert!(inventory.paths.len() > 300, "only {} paths: {:#?}", inventory.paths.len(), inventory.paths);
    let sample = inventory.sample();
    let families: BTreeSet<&str> = inventory.paths.iter().map(|(_, family)| family.as_str()).collect();
    assert!(!families.contains("undeclared"), "{:#?}", inventory.paths);
    let sampled: BTreeSet<&str> = sample.iter().map(|(_, family)| *family).collect();
    assert_eq!(families, sampled, "every value family is sampled");

    let mut refused = Vec::new();
    for (setter, family) in &sample {
        let outcome = local.assert_spellings_agree(setter, &["doc.md".to_string(), setter.clone()]);
        if !outcome.contains("\nvalue {") {
            refused.push(format!("{setter} ({family}): {}", outcome.lines().last().unwrap_or_default()));
        }
    }
    assert!(refused.is_empty(), "each representative must be a value the style accepts:\n{}", refused.join("\n"));
}

/// No subcommand name after `--` is a command: each is the input, in both
/// spellings.
#[test]
fn every_command_name_after_the_terminator_is_an_input() {
    let root = Cli::command();
    let names: Vec<&str> = root.get_subcommands().flat_map(|command| std::iter::once(command.get_name()).chain(command.get_all_aliases())).collect();
    assert!(names.contains(&"toc") && names.contains(&"render"), "{names:?}");
    for name in names {
        for argv in [&["md", "--", name][..], &["md", "render", "--", name][..]] {
            let cli = Cli::try_parse_from(argv).unwrap_or_else(|error| panic!("{argv:?}: {error}"));
            let Command::Render(options) = cli.command_or_render() else { panic!("{argv:?} should render") };
            let prepared = PreparedRenderOptions::prepare(&options).expect("one input");
            assert_eq!(prepared.input, Some(PathBuf::from(name)), "{argv:?}");
            assert!(prepared.setters.is_empty(), "{argv:?}");
        }
    }
}

// ── Spawned rows ────────────────────────────────────────────────────────────

/// How long a spawned row may run: a row that waits on stdin fails here
/// instead of hanging the suite.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);

/// `md` in `fixture` with `MD_DRY_RUN=fail` (so `--show` never opens
/// anything) and `stdin` written, or an empty stdin for `None`.
fn spawn(fixture: &CliProcessFixture, args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = fixture.command_builder().application_input("MD_DRY_RUN", "fail").build();
    command.args(args).timeout(SPAWN_TIMEOUT);
    // `assert_cmd` always pipes stdin and closes it after any written input,
    // so `None` is an empty pipe: a read sees end of file, never a wait.
    if let Some(text) = stdin {
        command.write_stdin(text);
    }
    command.output().unwrap_or_else(|error| panic!("{args:?}: {error}"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Asserts both spellings of `tail` exit 0 with the same stdout and stderr,
/// and returns the root spelling's output.
fn assert_spawned_spellings_agree(fixture: &CliProcessFixture, tail: &[&str], stdin: Option<&str>) -> Output {
    let implicit = spawn(fixture, tail, stdin);
    let explicit = spawn(fixture, &[&["render"][..], tail].concat(), stdin);
    assert_eq!(implicit.status.code(), Some(0), "{tail:?}: {}", text(&implicit.stderr));
    assert_eq!(explicit.status.code(), Some(0), "render {tail:?}: {}", text(&explicit.stderr));
    assert_eq!(text(&implicit.stdout), text(&explicit.stdout), "{tail:?}");
    assert_eq!(text(&implicit.stderr), text(&explicit.stderr), "{tail:?}");
    implicit
}

// ── Root render arguments before a subcommand ───────────────────────────────

/// `md <representative> toc doc.md` is a usage error through the real
/// binary; `args::render::tests` checks every argument's conflict in process.
fn assert_rejected_before_a_subcommand(id: &str) {
    let tokens = representatives()[id];
    let fixture = CliProcessFixture::named(&format!("alias-rejection-{id}"));
    fixture.write_file("cwd/doc.md", "# Title\n");
    fixture.write_file("cwd/style.yaml", STYLE_FILE);
    let args: Vec<&str> = tokens.iter().copied().chain(["toc", "doc.md"]).collect();
    let cli = Cli::try_parse_from(std::iter::once("md").chain(args.iter().copied())).expect("parses");
    let spellings = cli.render.has_explicit_arguments();
    assert!(!spellings.is_empty(), "{id}: {args:?} sets no root render argument");

    let output = spawn(&fixture, &args, None);
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
    assert!(output.stdout.is_empty(), "{args:?}: {}", text(&output.stdout));
    assert!(stderr.contains("subcommands cannot be combined with top-level options"), "{args:?}: {stderr}");
    for spelling in spellings {
        assert!(stderr.contains(spelling), "{args:?} does not name {spelling}: {stderr}");
    }
}

macro_rules! rejected_before_a_subcommand {
    ($($name:ident => $id:literal;)+) => {
        $(
            #[test]
            fn $name() {
                assert_rejected_before_a_subcommand($id);
            }
        )+

        /// Every render argument but `literal_args`: nothing after `--` is a
        /// subcommand (`every_command_name_after_the_terminator_is_an_input`).
        #[test]
        fn rejection_rows_cover_every_render_argument() {
            let covered: BTreeSet<String> = [$($id),+].into_iter().map(String::from).collect();
            let expected: BTreeSet<String> = render_arg_ids().into_iter().filter(|id| id != "literal_args").collect();
            assert_eq!(covered, expected);
        }
    };
}

rejected_before_a_subcommand! {
    setter_before_a_subcommand_is_a_usage_error => "args";
    style_file_before_a_subcommand_is_a_usage_error => "style";
    output_before_a_subcommand_is_a_usage_error => "output";
    show_before_a_subcommand_is_a_usage_error => "show";
    indent_before_a_subcommand_is_a_usage_error => "indent";
    deny_before_a_subcommand_is_a_usage_error => "deny";
    feature_before_a_subcommand_is_a_usage_error => "features";
    mermaid_before_a_subcommand_is_a_usage_error => "mermaid";
}

// ── Root-only early actions ─────────────────────────────────────────────────

/// Help, version, and completion output happen before any input is read:
/// with an empty stdin and render arguments beside them, they print and exit 0.
/// `help` checks their content; `path_completion` drives `COMPLETE=`.
fn assert_early_action(args: &[&str], complete: Option<&str>) -> Output {
    let fixture = CliProcessFixture::named("alias-early-action");
    let mut command = fixture.command_builder().application_input("MD_DRY_RUN", "fail").build();
    if let Some(shell) = complete {
        command.env("COMPLETE", shell);
    }
    let output = command.args(args).timeout(SPAWN_TIMEOUT).output().expect("md runs");
    assert_eq!(output.status.code(), Some(0), "{args:?}: {}", text(&output.stderr));
    assert!(!output.stdout.is_empty(), "{args:?} printed nothing");
    assert!(output.stderr.is_empty(), "{args:?}: {}", text(&output.stderr));
    output
}

macro_rules! early_actions {
    ($($name:ident => ($early:literal, [$($arg:literal),*], $complete:expr);)+) => {
        $(
            #[test]
            fn $name() {
                assert_early_action(&[$($arg),*], $complete);
            }
        )+

        /// Each root-only early action (`ROOT_ONLY` but `magic_roots`, plus
        /// help) and the dynamic completion path has a row.
        #[test]
        fn early_action_rows_cover_every_root_only_action() {
            let covered: BTreeSet<&str> = [$($early),+].into_iter().collect();
            let expected: BTreeSet<&str> =
                ROOT_ONLY.iter().map(|(id, _)| *id).filter(|id| *id != "magic_roots").chain(["help", "COMPLETE"]).collect();
            assert_eq!(covered, expected);
        }
    };
}

early_actions! {
    help_is_an_early_action => ("help", ["--help", "missing.md", "--output", "json"], None);
    version_is_an_early_action => ("version", ["--version", "missing.md"], None);
    completions_are_an_early_action => ("completions", ["--completions", "zsh"], None);
    dynamic_completion_is_an_early_action => ("COMPLETE", ["missing.md", "--output", "json"], Some("zsh"));
}

#[test]
fn the_short_version_flag_is_the_long_one() {
    let short = assert_early_action(&["-V"], None);
    let long = assert_early_action(&["--version"], None);
    assert_eq!(short.stdout, long.stdout);
}

// ── Byte parity with every CLI style layer ──────────────────────────────────

/// A MarkdownPlus document with its own `style:` and most block kinds.
const STYLED_DOCUMENT: &str = "\
---
title: Styled
style:
  page:
    margin-left: 2
    theme: github
  code-block:
    border: true
---
# Styled parity

Some **bold** body text with `inline code` and a [link](https://example.com).

- one
- two

| a | b |
|---|---|
| 1 | 2 |

```rust
fn main() {}
```

> A quote.
";

/// `md` and `md render` print the same bytes for `target` with the
/// document's style, a `--style` file, and setters, on a pinned capable
/// color terminal; and the CLI layers change every laid-out page.
fn assert_styled_spellings_agree(target: &str) {
    let fixture = CliProcessFixture::named(&format!("alias-styled-{target}"));
    fixture.write_file("cwd/doc.md", STYLED_DOCUMENT);
    fixture.write_file("cwd/style.yaml", "page:\n  padding-left: 1\nul:\n  padding-left: 2\n");
    let run = |args: &[&str]| {
        fixture
            .command_builder()
            .application_input("MD_DRY_RUN", "fail")
            .rendering_input("COLUMNS", "100")
            .rendering_input("LINES", "40")
            .rendering_input("TERM", "xterm-256color")
            .rendering_input("COLORTERM", "truecolor")
            .rendering_input("FORCE_COLOR", "1")
            .rendering_input_removed("NO_COLOR")
            .rendering_input("THEME", "dark")
            .build()
            .args(args)
            .timeout(SPAWN_TIMEOUT)
            .output()
            .expect("md runs")
    };
    let base = ["doc.md", "--output", target, "--width", "60"];
    let layers = ["--style", "style.yaml", "style.page.code.theme=gruvbox", "style.table.alignment=center"];
    let tail: Vec<&str> = base.iter().chain(&layers).copied().collect();
    let implicit = run(&tail);
    let explicit = run(&[&["render"][..], &tail].concat());
    assert_eq!(implicit.status.code(), Some(0), "{target}: {}", text(&implicit.stderr));
    assert_eq!(explicit.status.code(), Some(0), "{target}: {}", text(&explicit.stderr));
    assert_eq!(text(&implicit.stdout), text(&explicit.stdout), "{target}");
    assert_eq!(text(&implicit.stderr), text(&explicit.stderr), "{target}");

    let unstyled = run(&base);
    assert_eq!(unstyled.status.code(), Some(0), "{target}: {}", text(&unstyled.stderr));
    // MarkdownPlus keeps this document's Markdown text, which no page style
    // rewrites; the other targets lay the page out.
    let restyled = text(&implicit.stdout) != text(&unstyled.stdout);
    assert_eq!(restyled, target != "markdown-plus", "{target}: whether the CLI layers change the output");
}

macro_rules! styled_parity {
    ($($name:ident => $target:literal;)+) => {
        $(
            #[test]
            fn $name() {
                assert_styled_spellings_agree($target);
            }
        )+

        /// Every target that builds a page; the source and data targets
        /// print no page and are compared by `render_dispatch`.
        #[test]
        fn styled_parity_covers_every_page_target() {
            let covered: BTreeSet<&str> = [$($target),+].into_iter().collect();
            let pages: BTreeSet<&str> = OutputTarget::ALL
                .iter()
                .filter(|target| !target.is_data() && **target != OutputTarget::Markdown)
                .map(|target| target.as_str())
                .collect();
            assert_eq!(covered, pages);
        }
    };
}

styled_parity! {
    styled_terminal_bytes_agree => "terminal";
    styled_plain_bytes_agree => "plain";
    styled_html_bytes_agree => "html";
    styled_markdown_plus_bytes_agree => "markdown-plus";
}

// ── Input forms ─────────────────────────────────────────────────────────────

const STDIN_DOCUMENT: &str = "# From stdin\n\nPiped body.\n";

/// Redirected stdin (empty or not), `-`, and shared controls alone render
/// the same in both spellings, for a formatted and a data output.
fn assert_stdin_form_agrees(stdin: &str, tail: &[&str]) {
    let fixture = CliProcessFixture::named("alias-stdin");
    let output = assert_spawned_spellings_agree(&fixture, tail, Some(stdin));
    let stdout = text(&output.stdout);
    if tail.contains(&"json") {
        let value: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{tail:?}: {error}: {stdout}"));
        assert_eq!(value["outcome"], "success", "{tail:?}: {value:#}");
        assert!(value["file"].is_null(), "stdin has no file: {value:#}");
        assert!(output.stderr.is_empty(), "{tail:?}: {}", text(&output.stderr));
    }
    assert_eq!(stdout.contains("From stdin"), !stdin.is_empty(), "{tail:?}: {stdout}");
}

macro_rules! stdin_forms {
    ($($name:ident => ($stdin:expr, [$($arg:literal),*]);)+) => {
        $(
            #[test]
            fn $name() {
                assert_stdin_form_agrees($stdin, &[$($arg),*]);
            }
        )+

        /// Empty and nonempty stdin, implicit and `-`, formatted and data,
        /// and the shared controls alone.
        #[test]
        fn stdin_rows_cover_every_form() {
            let rows: Vec<(&str, Vec<&str>)> = vec![$(($stdin, vec![$($arg),*])),+];
            let forms: BTreeSet<(bool, bool, bool)> = rows
                .iter()
                .map(|(stdin, args)| (stdin.is_empty(), args.contains(&"-"), args.contains(&"json")))
                .collect();
            assert_eq!(forms.len(), 8, "{forms:?}");
            for shared in [&["--width", "60"][..], &["-v"][..]] {
                assert!(rows.iter().any(|(_, args)| args.as_slice() == shared), "{shared:?}");
            }
        }
    };
}

stdin_forms! {
    redirected_stdin_agrees => (STDIN_DOCUMENT, ["--output", "plain"]);
    redirected_empty_stdin_agrees => ("", ["--output", "plain"]);
    dash_stdin_agrees => (STDIN_DOCUMENT, ["-", "--output", "plain"]);
    dash_empty_stdin_agrees => ("", ["-", "--output", "plain"]);
    redirected_stdin_data_agrees => (STDIN_DOCUMENT, ["--output", "json"]);
    redirected_empty_stdin_data_agrees => ("", ["--output", "json"]);
    dash_stdin_data_agrees => (STDIN_DOCUMENT, ["-", "--output", "json"]);
    dash_empty_stdin_data_agrees => ("", ["-", "--output", "json"]);
    width_alone_reads_stdin => (STDIN_DOCUMENT, ["--width", "60"]);
    verbose_alone_reads_stdin => (STDIN_DOCUMENT, ["-v"]);
}

/// A file named like a command renders as that file, with an empty stdin.
fn assert_command_named_file_renders(args: &[&str]) {
    let fixture = CliProcessFixture::named("alias-command-name");
    fixture.write_file("cwd/toc", "# From a file named toc\n");
    let output = spawn(&fixture, args, None);
    assert_eq!(output.status.code(), Some(0), "{args:?}: {}", text(&output.stderr));
    let reference = spawn(&fixture, &["render", "./toc"], None);
    assert_eq!(text(&output.stdout), text(&reference.stdout), "{args:?}");
    assert!(text(&output.stdout).contains("From a file named toc"), "{args:?}: {}", text(&output.stdout));
}

macro_rules! command_named_files {
    ($($name:ident => [$($arg:literal),*];)+) => {
        $(
            #[test]
            fn $name() {
                assert_command_named_file_renders(&[$($arg),*]);
            }
        )+

        /// Each spelling: `--` and `./`, with and without `render`.
        #[test]
        fn command_named_file_rows_cover_both_spellings() {
            let rows: BTreeSet<Vec<&str>> = [$(vec![$($arg),*]),+].into_iter().collect();
            let expected: BTreeSet<Vec<&str>> =
                [vec!["--", "toc"], vec!["render", "--", "toc"], vec!["./toc"], vec!["render", "./toc"]].into_iter().collect();
            assert_eq!(rows, expected);
        }
    };
}

command_named_files! {
    a_file_named_toc_after_the_terminator_renders => ["--", "toc"];
    render_a_file_named_toc_after_the_terminator => ["render", "--", "toc"];
    a_file_named_toc_with_a_dot_slash_renders => ["./toc"];
    render_a_file_named_toc_with_a_dot_slash => ["render", "./toc"];
}
