//! `claudine inline-compose` on a document whose frontmatter root keys are
//! indented.
//!
//! The closure's text editors only edit column-0 frontmatter, so preparation
//! refuses such a baseline before it composes anything: no frontmatter
//! command runs, no guardrail file is written, no provider launches, and the
//! file keeps its bytes. After `md clean --save` moves the keys to column 0,
//! the same document runs and is stamped. A baseline that was editable when
//! the run started is still checked again after the agent edits it.
//!
//! The provider is the compiled `claudine-fake-goose` on every OS, in its
//! marker mode: one line per launch in `PROVIDER_MARKER`, and an edit of
//! `FAKE_INLINE_DOCUMENT`. The frontmatter command is the compiled
//! `claudine-argv-recorder`, which leaves a record when it runs (a shell
//! builtin such as `mkdir` is not a command Windows can find).

use std::fs;
use std::path::{Path, PathBuf};

use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

use crate::common::{self, CliProcessFixture};

/// The reproduction: every root key indented by two spaces, and a frontmatter
/// command whose record proves whether composition ran.
const INDENTED: &str = "---\n  prompt: Append a line to the notes\n  last_updated: 2026-01-01\n  probe: \"$(argv-recorder ran)\"\n---\nOriginal notes.\n";

struct Run {
    success: bool,
    /// ANSI-stripped stderr with block gutters, whitespace, and backticks
    /// removed, so wrapped and code-styled text matches a squeezed needle.
    squeezed: String,
    stderr: String,
}

impl Run {
    #[track_caller]
    fn assert_mentions(&self, text: &str) {
        let needle = squeeze(text);
        assert!(self.squeezed.contains(&needle), "stderr lacks {text:?}:\n{}", self.stderr);
    }
}

fn squeeze(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace() && !matches!(c, '`' | '┃')).collect()
}

struct Inline {
    fixture: CliProcessFixture,
    document: PathBuf,
    marker: PathBuf,
    records: PathBuf,
}

impl Inline {
    fn new(name: &str, source: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        // A repository root is where the guardrail file is written, so its
        // absence after a refusal is meaningful.
        fixture.initialize_repository();
        let suffix = std::env::consts::EXE_SUFFIX;
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{suffix}")),
        )
        .expect("the fake provider is built with `test-fixtures`");
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-argv-recorder"),
            fixture.bin_dir().join(format!("argv-recorder{suffix}")),
        )
        .expect("the recorder is built with `test-fixtures`");
        fs::write(fixture.cwd().join(".darkmatter-shell-whitelist"), "exact argv-recorder ran\n").unwrap();
        let document = fixture.cwd().join("notes.md");
        fs::write(&document, source).unwrap();
        let marker = fixture.cwd().join("provider-runs.log");
        let records = fixture.cwd().join("argv.bin");
        Self { fixture, document, marker, records }
    }

    fn run(&self, extra: &[&str], replacement: Option<&Path>) -> Run {
        let mut command = self.fixture.command();
        command
            .env("PROVIDER_MARKER", &self.marker)
            .env(ARGV_RECORDER_FILE, &self.records)
            .env("FAKE_INLINE_DOCUMENT", &self.document)
            .args(["inline-compose", "--goose"])
            .args(extra)
            .arg(&self.document);
        if let Some(replacement) = replacement {
            command.env("FAKE_INLINE_REPLACEMENT", replacement);
        }
        let output = command.output().unwrap();
        let stderr = common::strip_ansi(&String::from_utf8_lossy(&output.stderr));
        Run { success: output.status.success(), squeezed: squeeze(&stderr), stderr }
    }

    fn launches(&self) -> usize {
        fs::read_to_string(&self.marker).map_or(0, |log| log.lines().count())
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.document).unwrap()
    }

    fn command_ran(&self) -> bool {
        self.records.exists()
            && read_argv_records(&self.records).expect("whole records") == vec![vec!["ran".to_string()]]
    }

    fn guardrails_written(&self) -> bool {
        self.fixture.cwd().join(".claudine/inline-compose.md").exists()
    }

    /// What `md clean --save` does to the file, through the Darkmatter library
    /// transaction behind it, which (unlike a sibling `md` process) is
    /// available in an archived run.
    fn clean_save(&self) {
        use darkmatter::markdown::cleanup::rules::MarkdownRuleOverrides;
        use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, clean_document};
        use darkmatter::markdown::schemas::CleanSchemaConfig;

        let source = self.text();
        let overrides = MarkdownRuleOverrides::default();
        let request = CleanRequest {
            source: &source,
            overrides: &overrides,
            workspace: None,
            document_path: Some(&self.document),
        };
        let context = biscuit_file::FileResolutionContext::new(self.fixture.cwd().to_path_buf());
        let result = clean_document(&request, || Ok::<_, std::convert::Infallible>(CleanSchemaConfig::new(context)))
            .expect("clean runs");
        assert_eq!(result.outcome, CleanOutcome::Changed, "{:?}", result.diagnostics);
        fs::write(&self.document, result.text).unwrap();
    }
}

/// The parsed frontmatter of `text` without the properties the closure
/// manages, so a cleaned or stamped document compares equal to its source.
fn data(text: &str) -> serde_json::Value {
    let markdown: darkmatter::markdown::Markdown = text.to_string().into();
    let mut value = serde_json::to_value(markdown.frontmatter().as_map()).unwrap();
    let map = value.as_object_mut().unwrap();
    map.remove("hash");
    map.remove("last_updated");
    value
}

/// Every non-blank, non-comment frontmatter line of `text` starts at column 0.
#[track_caller]
fn assert_keys_at_column_zero(text: &str) {
    let yaml = text.strip_prefix("---\n").and_then(|rest| rest.split_once("\n---\n")).unwrap().0;
    for line in yaml.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')) {
        assert!(!line.starts_with([' ', '\t']), "indented frontmatter line {line:?} in:\n{text}");
    }
}

/// `text`'s stored hash matches its own content, by the comparison behind
/// `md hash --diff`.
#[track_caller]
fn assert_valid_hash(text: &str) {
    let markdown: darkmatter::markdown::Markdown = text.to_string().into();
    let options = claudine::composition::closure::inline_hash_options();
    let stored = markdown.frontmatter().as_map().get(&options.property).expect("a stored hash");
    let stored = darkmatter::markdown::hash::StoredHash::parse(stored, &options.property).expect("a valid stored hash");
    let comparison = markdown.compare_hash(&stored, &options).expect("a comparable hash");
    assert!(!comparison.frontmatter_changed && !comparison.body_changed, "stale hash in:\n{text}");
}

#[test]
fn an_indented_baseline_is_refused_before_anything_runs() {
    let inline = Inline::new("inline-indented-refused", INDENTED);

    let run = inline.run(&[], None);

    assert!(!run.success, "{}", run.stderr);
    assert_eq!(inline.launches(), 0, "the provider launched:\n{}", run.stderr);
    assert_eq!(inline.text(), INDENTED, "the document changed");
    assert!(!inline.command_ran(), "the frontmatter command ran");
    assert!(!inline.guardrails_written(), "the guardrail file was written");
    run.assert_mentions("inline baseline not editable");
    run.assert_mentions("notes.md");
    run.assert_mentions("indented by 2 spaces");
    run.assert_mentions("md clean --save <file>");
    assert!(!inline.fixture.audio_spool().exists(), "audio was published");
}

#[test]
fn an_indented_baseline_is_refused_by_a_dry_run_too() {
    let inline = Inline::new("inline-indented-dry-run", INDENTED);

    let run = inline.run(&["--dry-run"], None);

    assert!(!run.success, "{}", run.stderr);
    assert_eq!(inline.launches(), 0);
    assert_eq!(inline.text(), INDENTED);
    assert!(!inline.command_ran(), "the frontmatter command ran");
    run.assert_mentions("inline baseline not editable");
    run.assert_mentions("indented by 2 spaces");
}

/// The refusal's recovery path, and the control that makes its absences
/// meaningful: the cleaned document runs its command, launches the provider,
/// accepts the body edit, and is stamped with a valid column-0 hash.
#[test]
fn the_cleaned_baseline_runs_and_is_stamped_at_column_zero() {
    let inline = Inline::new("inline-indented-cleaned", INDENTED);
    let refused = inline.run(&[], None);
    assert!(!refused.success, "{}", refused.stderr);

    inline.clean_save();
    let cleaned = inline.text();
    assert_keys_at_column_zero(&cleaned);
    assert_eq!(data(&cleaned), data(INDENTED), "cleaning changed the data");

    let run = inline.run(&[], None);

    assert!(run.success, "{}", run.stderr);
    assert_eq!(inline.launches(), 1);
    assert!(inline.command_ran(), "the frontmatter command did not run");
    assert!(inline.guardrails_written(), "the guardrail file was not written");
    let stamped = inline.text();
    assert!(stamped.contains("Agent-written body."), "{stamped}");
    assert!(stamped.contains("\nhash: "), "no column-0 hash in:\n{stamped}");
    assert_keys_at_column_zero(&stamped);
    assert_valid_hash(&stamped);
    assert_eq!(data(&stamped), data(INDENTED), "the run changed the data");
}

/// The launch check covers only the baseline: an agent that indents the
/// frontmatter while editing the body still fails the closure, which restores
/// the original bytes.
#[test]
fn an_agent_indented_candidate_fails_closure_and_restores_the_baseline() {
    let baseline = "---\nprompt: Rewrite the notes\nlast_updated: 2026-01-01\n---\nOriginal notes.\n";
    let inline = Inline::new("inline-indented-candidate", baseline);
    let replacement = inline.fixture.cwd().join("candidate.txt");
    fs::write(
        &replacement,
        "---\n  prompt: Rewrite the notes\n  last_updated: 2026-01-01\n---\nRewritten notes.\n",
    )
    .unwrap();

    let run = inline.run(&[], Some(&replacement));

    assert!(!run.success, "{}", run.stderr);
    assert_eq!(inline.launches(), 1, "the provider did not launch:\n{}", run.stderr);
    assert_eq!(inline.text(), baseline, "the baseline was not restored");
    run.assert_mentions("indented by 2 spaces");
}
