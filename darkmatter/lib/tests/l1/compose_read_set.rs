//! The read set a composition reports, and the command-wide data runtime
//! behind it.
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there, so no test reads this repository or the process
//! directory. Each test asserts the public result: the report's or the
//! failure's `read_set`, or a caller-attached runtime's counters.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, ComposeReport, DataRuntime};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// The read-set spelling of `name` in `dir`.
fn entry(dir: &Path, name: &str) -> String {
    let canonical = biscuit_file::canonicalize_simplified(dir).unwrap();
    biscuit_file::to_portable_string(&canonical.join(name))
}

fn options_for(path: &Path) -> ComposeOptions {
    ComposeOptions::new().with_source_file(path)
}

fn compose_options(path: &Path, options: ComposeOptions) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    let request = crate::request_support::request(options);
    Markdown::try_from(path).unwrap().compose_with_warnings(&request)
}

fn compose(dir: &Path, content: &str) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    let path = write(dir, "doc.md", content);
    compose_options(&path, options_for(&path))
}

fn read_set(dir: &Path, content: &str) -> Vec<String> {
    match compose(dir, content) {
        Ok((_, report)) => report.read_set,
        Err(failure) => panic!("{content:?} failed: {}", failure.error),
    }
}

#[test]
fn a_frontmatter_read_and_a_missing_file_probe_are_both_recorded() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "b.md", "---\ntitle: Bee\n---\nbody\n");
    let (composed, report) =
        compose(dir.path(), "{{ frontmatter('b.md', 'title') }} {{ file_exists('c.md') }}\n").unwrap();
    assert!(composed.content().contains("Bee false"), "{}", composed.content());
    assert_eq!(report.read_set, vec![entry(dir.path(), "b.md"), entry(dir.path(), "c.md")]);
}

#[test]
fn a_read_that_fails_after_resolving_is_on_the_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "bad.md", "---\ntitle: [unclosed\n---\nbody\n");
    let failure = compose(dir.path(), "{{ frontmatter('bad.md') }}\n").expect_err("an unparsable file fails");
    assert_eq!(failure.read_set, vec![entry(dir.path(), "bad.md")]);
}

#[test]
fn a_malformed_reference_and_a_null_argument_record_nothing() {
    let dir = TempDir::new().unwrap();
    assert!(read_set(dir.path(), "{{ file_exists('') }} {{ file_exists(null) }}\n").is_empty());
}

#[test]
fn repeated_reads_are_recorded_once_in_lexical_order() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "z.md", "# Zed\n");
    write(dir.path(), "a.md", "# Ay\n");
    let reads = read_set(
        dir.path(),
        "{{ markdown_title('z.md') }} {{ markdown_title('a.md') }} {{ markdown_title('./z.md') }} \
         {{ markdown_body_empty('a.md') }}\n",
    );
    assert_eq!(reads, vec![entry(dir.path(), "a.md"), entry(dir.path(), "z.md")]);
}

#[test]
fn frontmatter_interpolation_records_its_reads() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "b.md", "# Bee\n");
    let reads = read_set(dir.path(), "---\ntitle: \"{{ markdown_title('b.md') }}\"\n---\n{{ title }}\n");
    assert_eq!(reads, vec![entry(dir.path(), "b.md")]);
}

#[test]
fn a_block_condition_records_its_reads() {
    let dir = TempDir::new().unwrap();
    let reads = read_set(dir.path(), "::block when=\"file_exists('gate.md')\"\nhidden\n::end-block\n");
    assert_eq!(reads, vec![entry(dir.path(), "gate.md")]);
}

#[test]
fn a_transclusion_condition_records_its_reads() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "child\n");
    let reads = read_set(dir.path(), "::file ./child.md when=\"file_exists('gate.md')\"\n");
    assert_eq!(reads, vec![entry(dir.path(), "gate.md")]);
}

#[test]
fn a_transclusion_target_records_its_reads() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "child body\n");
    write(dir.path(), "pointer.md", "---\ntarget: ./child.md\n---\n");
    let (composed, report) = compose(dir.path(), "::file {{ frontmatter('pointer.md', 'target') }}\n").unwrap();
    assert!(composed.content().contains("child body"), "{}", composed.content());
    assert_eq!(report.read_set, vec![entry(dir.path(), "pointer.md")]);
}

#[test]
fn map_and_filter_callbacks_record_their_reads() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "one.md", "# One\n");
    let reads = read_set(
        dir.path(),
        "---\n$schema:\n  names: string[]\nnames: [one.md, two.md]\n---\n\
         ::map names \"n => file_exists(n)\"\n- item\n::end-map\n\n\
         ::filter names \"n => markdown_title('one.md') == 'One'\"\n- kept\n::end-filter\n",
    );
    assert_eq!(reads, vec![entry(dir.path(), "one.md"), entry(dir.path(), "two.md")]);
}

#[test]
fn separate_compositions_share_no_runtime() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.md", "# Ay\n");
    write(dir.path(), "b.md", "# Bee\n");
    let first = write(dir.path(), "first.md", "{{ markdown_title('a.md') }}\n");
    let second = write(dir.path(), "second.md", "{{ markdown_title('b.md') }}\n");
    let first_request = crate::request_support::request(options_for(&first));
    let (_, report) = Markdown::try_from(first.as_path()).unwrap().compose_with_warnings(&first_request).unwrap();
    assert_eq!(report.read_set, vec![entry(dir.path(), "a.md")]);
    assert!(first_request.read_set().is_empty(), "the caller's request holds no runtime of its own");

    let (_, report) = compose_options(&second, options_for(&second)).unwrap();
    assert_eq!(report.read_set, vec![entry(dir.path(), "b.md")], "nothing carried over from the first");
}

#[test]
fn an_attached_runtime_spans_every_pass_of_one_command() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.md", "# Ay\n");
    write(dir.path(), "b.md", "# Bee\n");
    let first = write(dir.path(), "first.md", "{{ markdown_title('a.md') }}\n");
    let second = write(dir.path(), "second.md", "{{ markdown_title('b.md') }}\n");
    let runtime = DataRuntime::new();
    compose_options(&first, options_for(&first).with_data_runtime(runtime.clone())).unwrap();
    let request = crate::request_support::request(options_for(&second).with_data_runtime(runtime.clone()));
    let (_, report) = Markdown::try_from(second.as_path()).unwrap().compose_with_warnings(&request).unwrap();
    let both = vec![entry(dir.path(), "a.md"), entry(dir.path(), "b.md")];
    assert_eq!(report.read_set, both);
    assert_eq!(request.read_set(), both);
    assert_eq!(runtime.read_set(), both);
}

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// `git` is on every host these tests run on, and `--version` writes nothing.
fn shell_runtime_after(dir: &Path, content: &str) -> DataRuntime {
    let path = write(dir, "doc.md", content);
    let runtime = DataRuntime::new();
    let options = options_for(&path)
        .with_shell(ShellExpansionOptions { policy_root: Some(dir.to_path_buf()), ..Default::default() })
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(Arc::new(AllowAll))
        .with_data_runtime(runtime.clone());
    compose_options(&path, options).unwrap_or_else(|failure| panic!("{content:?} failed: {}", failure.error));
    runtime
}

#[test]
fn an_executed_command_empties_the_memo_and_keeps_the_read_set() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.md", "# Ay\n");
    let runtime = shell_runtime_after(dir.path(), "{{ markdown_title('a.md') }}\n\n::shell git --version\n");
    assert_eq!(runtime.stats().invalidations, 1);
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "a.md")]);

    let frontmatter = shell_runtime_after(dir.path(), "---\nversion: $(git --version)\n---\n{{ version }}\n");
    assert_eq!(frontmatter.stats().invalidations, 1);
}

#[test]
fn a_cached_or_skipped_command_does_not_empty_the_memo() {
    let dir = TempDir::new().unwrap();
    let cached = shell_runtime_after(dir.path(), "::shell git --version\n\n::shell git --version\n");
    assert_eq!(cached.stats().invalidations, 1, "the second directive reused the first's outcome");
    let skipped = shell_runtime_after(dir.path(), "::block when=\"false\"\n::shell git --version\n::end-block\n");
    assert_eq!(skipped.stats().invalidations, 0);
}
