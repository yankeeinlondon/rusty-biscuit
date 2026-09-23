//! A transcluded source that is the first to name a `ctx.*` group.
//!
//! The root of each graph here names no discovery-backed group, so every group
//! a child reads must be introduced by the request's context authority when
//! the child becomes reachable (spec verification #7-#10). Contexts are built
//! from supplied evidence and grown by a counting `ContextExtension` whose
//! every capture projects a distinct repository root, so a second capture of
//! the same group within one request is visible in the rendered output — no
//! host discovery runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::ExpressionError;
use darkmatter::markdown::compose::{
    CacheAccessMode, ComposeContext, ComposeOptions, ContextAuthority, ContextCaptureEvidence,
    ContextExtension, ContextGroup, ContextRequirements,
};
use darkmatter::markdown::MarkdownError;

/// Grows a context from supplied evidence, projecting `<label>-<n>` as the
/// repository root of the `n`th capture it performs.
#[derive(Debug)]
struct CountingExtension {
    label: String,
    captures: AtomicUsize,
    groups: Mutex<Vec<ContextGroup>>,
}

impl CountingExtension {
    fn new(label: &str) -> Arc<Self> {
        Arc::new(Self {
            label: label.to_string(),
            captures: AtomicUsize::new(0),
            groups: Mutex::new(Vec::new()),
        })
    }

    fn captures(&self) -> usize {
        self.captures.load(Ordering::SeqCst)
    }

    fn captured_groups(&self) -> Vec<ContextGroup> {
        self.groups.lock().unwrap().clone()
    }
}

impl ContextExtension for CountingExtension {
    fn extend(&self, context: &mut ComposeContext, required: &ContextRequirements) -> bool {
        let missing: Vec<_> = context.missing_requirements(required).iter().collect();
        if missing.is_empty() {
            return false;
        }
        let capture = self.captures.fetch_add(1, Ordering::SeqCst) + 1;
        self.groups.lock().unwrap().extend(missing);
        let evidence = ContextCaptureEvidence::new(HashMap::new())
            .with_git(None)
            .with_repository(Some(PathBuf::from(format!("{}-{capture}", self.label))), None);
        context.extend_with_evidence(required, &evidence)
    }
}

/// A root context naming no discovery-backed group.
fn root_context(anchor: &Path) -> ComposeContext {
    ComposeContext::capture_with_evidence(
        anchor,
        &ContextRequirements::for_content(""),
        &ContextCaptureEvidence::new(HashMap::new()),
    )
}

fn write(directory: &Path, name: &str, contents: &str) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn compose(root: &Path, options: ComposeOptions) -> Result<String, MarkdownError> {
    let markdown = Markdown::try_from(root).expect("root loads");
    markdown
        .compose_with(options.with_source_file(root))
        .map(|(composed, _)| composed.content().to_string())
}

fn extended(anchor: &Path, extension: &Arc<CountingExtension>) -> ComposeOptions {
    ComposeOptions::new_with_context(root_context(anchor))
        .with_context_authority(ContextAuthority::CallerExtended(extension.clone()))
}

/// The value rendered after `name=` on its own line.
fn rendered<'a>(output: &'a str, name: &str) -> &'a str {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(&format!("{name}=")))
        .unwrap_or_else(|| panic!("`{name}=` missing from {output:?}"))
}

/// Sibling children and a nested grandchild, resolved concurrently, all first
/// read `ctx.repo_root` below a root that never names it. The group is captured
/// once for the request and every read renders that one capture.
#[test]
fn siblings_and_nested_children_read_one_capture_of_a_child_introduced_group() {
    let directory = tempfile::tempdir().unwrap();
    let root = write(directory.path(), "root.md", "root\n\n::file ./a.md\n\n::file ./b.md\n\n::file ./c.md\n");
    write(directory.path(), "a.md", "a={{ ctx.repo_root }}\n\n::file ./nested.md\n");
    write(directory.path(), "b.md", "b={{ ctx.repo_root }}\n");
    write(directory.path(), "c.md", "c={{ ctx.repo_root }}\n");
    write(directory.path(), "nested.md", "nested={{ ctx.repo_root }}\n");
    let extension = CountingExtension::new("request");

    let output = compose(&root, extended(directory.path(), &extension)).expect("the graph composes");

    let first = rendered(&output, "a");
    assert!(first.contains("request-1"), "{output}");
    for name in ["b", "c", "nested"] {
        assert_eq!(rendered(&output, name), first, "{name} read another capture: {output}");
    }
    assert_eq!(extension.captures(), 1, "one capture per request");
    assert_eq!(extension.captured_groups(), vec![ContextGroup::Repo]);
}

/// A transclusion whose condition is false never becomes reachable, so the
/// group only it names is never captured.
#[test]
fn a_group_named_only_by_an_unreachable_child_is_never_captured() {
    let directory = tempfile::tempdir().unwrap();
    let root = write(
        directory.path(),
        "root.md",
        "root\n\n::file ./gpu.md when=\"false\"\n\n::file ./repo.md\n",
    );
    write(directory.path(), "gpu.md", "gpu={{ ctx.gpu }}\n");
    write(directory.path(), "repo.md", "repo={{ ctx.repo_root }}\n");
    let extension = CountingExtension::new("lazy");

    let output = compose(&root, extended(directory.path(), &extension)).expect("the graph composes");

    assert!(!output.contains("gpu="), "{output}");
    assert!(rendered(&output, "repo").contains("lazy-1"), "{output}");
    assert_eq!(extension.captured_groups(), vec![ContextGroup::Repo]);
}

/// A graph whose documents name only `DateTime`-owned keys, frontmatter,
/// unknown `ctx.*` names, and literal `ctx.*` text never asks the authority to
/// grow the request context — not at the root, not for a condition-gated child,
/// not for a nested child, and not when a persistent entry is read back
/// (spec verification #11). The authority grows the context through the same
/// handoff for every kind, so a counting extension observes what ambient
/// discovery would have run.
#[test]
fn a_graph_naming_no_discovery_backed_group_captures_none() {
    let directory = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let root = write(
        directory.path(),
        "root.md",
        "---\ntitle: Graph\n---\nroot={{ title }} {{ ctx.today }}\n\n::file ./child.md when=\"ctx.year > 2000\"\n",
    );
    write(
        directory.path(),
        "child.md",
        "child={{ ctx.year }} {{ ctx.oss }} {{{ ctx.repo_root }}}\n\n::file ./nested.md\n",
    );
    write(directory.path(), "nested.md", "nested={{ ctx.month }}\n");

    for pass in ["cold", "warm"] {
        let extension = CountingExtension::new(pass);
        let options = extended(directory.path(), &extension).with_cache_root(cache.path());

        let output = compose(&root, options).expect("the graph composes");

        assert!(rendered(&output, "root").starts_with("Graph "), "{pass}: {output}");
        assert!(rendered(&output, "child").contains("{{ ctx.repo_root }}"), "{pass}: {output}");
        assert!(!rendered(&output, "nested").is_empty(), "{pass}: {output}");
        assert_eq!(
            extension.captured_groups(),
            Vec::<ContextGroup>::new(),
            "{pass}: a group was captured for a graph that names none"
        );
        assert_eq!(extension.captures(), 0, "{pass}");
    }
}

/// The same graph under a frozen caller-supplied context fails on the child's
/// first read, naming the variable, the group, and the child — the context is
/// never grown and no output is produced.
#[test]
fn a_frozen_context_fails_when_a_child_first_names_a_group() {
    let directory = tempfile::tempdir().unwrap();
    let root = write(directory.path(), "root.md", "root\n\n::file ./child.md\n");
    write(directory.path(), "child.md", "child={{ ctx.repo_root }}\n");

    for options in [
        ComposeOptions::new_with_context(root_context(directory.path())),
        ComposeOptions::new()
            .with_context(root_context(directory.path()))
            .with_fail_fast(false),
    ] {
        assert!(!options.context_authority().is_extendable());
        let error = compose(&root, options).expect_err("a frozen context cannot grow");

        match error.missing_runtime_context() {
            Some(ExpressionError::ContextNotCaptured { key, group }) => {
                assert_eq!(key, "repo_root");
                assert_eq!(group, &ContextGroup::Repo);
            }
            other => panic!("expected ContextNotCaptured, got {other:?} from {error:?}"),
        }
        assert!(format!("{error:?}").contains("child.md"), "the child is the source: {error:?}");
    }
}

/// A remote child is handed the request context the same way a local one is.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remote_child_first_naming_a_group_reads_the_request_capture() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/remote.md"))
        .respond_with(ResponseTemplate::new(200).set_body_string("remote={{ ctx.repo_root }}\n"))
        .mount(&server)
        .await;
    let directory = tempfile::tempdir().unwrap();
    let root = write(
        directory.path(),
        "root.md",
        &format!("root\n\n::file {}/remote.md\n\n::file ./local.md\n", server.uri()),
    );
    write(directory.path(), "local.md", "local={{ ctx.repo_root }}\n");
    let extension = CountingExtension::new("remote");
    let options = extended(directory.path(), &extension)
        .with_allow_remote_transclusion(true)
        .with_remote_read_config(darkmatter::markdown::compose::RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        });

    let output = tokio::task::spawn_blocking(move || compose(&root, options))
        .await
        .unwrap()
        .expect("the graph composes");

    assert!(rendered(&output, "remote").contains("remote-1"), "{output}");
    assert_eq!(rendered(&output, "local"), rendered(&output, "remote"), "{output}");
    assert_eq!(extension.captures(), 1);
}

/// Pre-flight discovery hands each child the same context the compose pass
/// will: a child's `$()` branch reading a group its root never names is
/// discovered with the grown value under an extendable authority, and reported
/// as the missing capture (not a dynamic command shape) under a frozen one.
#[test]
fn preflight_discovers_a_child_branch_reading_a_child_introduced_group() {
    let directory = tempfile::tempdir().unwrap();
    let root = write(directory.path(), "root.md", "root\n\n::file ./child.md\n");
    write(
        directory.path(),
        "child.md",
        "---\nplatform: \"$(true ? echo {{ ctx.repo_root }} : echo none)\"\n---\nchild\n",
    );
    let preflight = |options: ComposeOptions| {
        Markdown::try_from(root.as_path())
            .expect("root loads")
            .compose_preflight(&options.with_source_file(&root))
    };

    let extension = CountingExtension::new("preflight");
    let report = preflight(extended(directory.path(), &extension)).expect("discovery succeeds");
    let commands: Vec<_> = report.entries.iter().map(|entry| entry.normalized.as_str()).collect();
    assert!(commands.contains(&"echo none"), "{commands:?}");
    assert!(
        commands.iter().any(|command| command.contains("preflight-1")),
        "the child branch is discovered with the grown value: {commands:?}"
    );

    let error = preflight(ComposeOptions::new_with_context(root_context(directory.path())))
        .expect_err("a frozen context cannot answer the child's branch");
    assert!(
        matches!(
            error.missing_runtime_context(),
            Some(ExpressionError::ContextNotCaptured { group: ContextGroup::Repo, .. })
        ),
        "{error:?}"
    );
}

/// Grows a context with no supplied evidence at all, so every group it
/// captures projects its typed null/empty value plus a `PartialRuntimeCapture`
/// diagnostic.
#[derive(Debug)]
struct UnavailableEvidence;

impl ContextExtension for UnavailableEvidence {
    fn extend(&self, context: &mut ComposeContext, required: &ContextRequirements) -> bool {
        context.extend_with_evidence(required, &ContextCaptureEvidence::new(HashMap::new()))
    }
}

/// A partial-capture compose: the output, the `Partial runtime capture`
/// warnings in report order, and the run-local cache hits.
struct PartialCompose {
    output: String,
    warnings: Vec<String>,
    hits: usize,
}

/// Composes `root` with a request context grown from unavailable evidence.
fn compose_partial(root: &Path, options: ComposeOptions) -> PartialCompose {
    let options =
        options.with_context_authority(ContextAuthority::CallerExtended(Arc::new(UnavailableEvidence)));
    let markdown = Markdown::try_from(root).expect("root loads");
    let (composed, report) = markdown
        .compose_with(options.with_source_file(root))
        .expect("a partial capture renders");
    PartialCompose {
        output: composed.content().to_string(),
        warnings: report
            .warnings
            .iter()
            .filter(|warning| warning.message.contains("Partial runtime capture"))
            .map(|warning| warning.message.clone())
            .collect(),
        hits: report.cache_stats.map_or(0, |stats| stats.hits),
    }
}

#[track_caller]
fn assert_git_partial_capture(warnings: &[String]) {
    assert!(
        !warnings.is_empty()
            && warnings.iter().all(|message| message.contains("Partial runtime capture for git")),
        "the child's partial capture must warn: {warnings:?}"
    );
}

/// A child transcluded twice is composed once and served from the run-local
/// cache the second time. The hit replays the child's report, so its
/// partial-capture warnings match a run that recomposes every child
/// (spec Requirement 3, verification #4).
#[test]
fn a_run_local_hit_replays_a_child_only_partial_capture_warning() {
    let directory = tempfile::tempdir().unwrap();
    let root = write(directory.path(), "root.md", "root\n\n::file ./child.md\n\n::file ./child.md\n");
    write(directory.path(), "child.md", "child=[{{ ctx.branch }}]\n");
    let options =
        |mode| ComposeOptions::new_with_context(root_context(directory.path())).with_cache_access_mode(mode);

    let uncached = compose_partial(&root, options(CacheAccessMode::Off));
    let cached = compose_partial(&root, options(CacheAccessMode::ReadWrite));

    assert!(cached.hits >= 1, "the second transclusion must be a run-local hit");
    assert_git_partial_capture(&uncached.warnings);
    assert_eq!(cached.warnings, uncached.warnings, "a run-local hit changed the diagnostics");
    assert_eq!(cached.output, uncached.output);
}

mod persistent_cache {
    use super::*;

    /// Composes `root` against a persistent cache, with the request context
    /// grown from evidence projecting `label` as the repository root.
    fn compose_cached(
        root: &Path,
        cache: &Path,
        label: &str,
    ) -> String {
        let extension = CountingExtension::new(label);
        let options = extended(root.parent().unwrap(), &extension).with_cache_root(cache);
        let markdown = Markdown::try_from(root).expect("root loads");
        let (composed, _) = markdown
            .compose_with(options.with_source_file(root))
            .expect("the graph composes");
        composed.content().to_string()
    }

    fn assert_nothing_persisted(cache: &Path) {
        let entries: Vec<_> = std::fs::read_dir(cache).unwrap().collect();
        assert!(entries.is_empty(), "the cache root gained entries: {entries:?}");
    }

    /// A cache root persists no composed output (R18,
    /// `fixes/2026-09-16-content-policy-no-cache`), so every run recomposes:
    /// a changed value is never stale and nothing lands under the root.
    fn assert_child_only_value_is_never_stale(root: &Path, cache: &Path, reader: &str) {
        let first = compose_cached(root, cache, "one");
        assert!(rendered(&first, reader).contains("one-1"), "{first}");

        let second = compose_cached(root, cache, "two");
        assert!(
            rendered(&second, reader).contains("two-1"),
            "a changed child-only value reused stale composed output: {second}"
        );

        let third = compose_cached(root, cache, "two");
        assert_eq!(third, second);
        assert_nothing_persisted(cache);
    }

    /// A child whose own key covers the group it reads.
    #[test]
    fn a_changed_child_only_value_invalidates_the_cached_child() {
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let root = write(directory.path(), "root.md", "root\n\n::file ./child.md\n");
        write(directory.path(), "child.md", "child={{ ctx.repo_root }}\n");

        assert_child_only_value_is_never_stale(&root, cache.path(), "child");
    }

    /// A middle document naming no group transcludes the reader: the middle
    /// document's own key never sees the group, so a persisted entry keyed on
    /// it alone would be stale.
    #[test]
    fn a_changed_grandchild_only_value_invalidates_the_cached_parent() {
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let root = write(directory.path(), "root.md", "root\n\n::file ./middle.md\n");
        write(directory.path(), "middle.md", "middle\n\n::file ./leaf.md\n");
        write(directory.path(), "leaf.md", "leaf={{ ctx.repo_root }}\n");

        assert_child_only_value_is_never_stale(&root, cache.path(), "leaf");
    }

    /// A root naming no discovery-backed group transcludes a child whose first
    /// `ctx.*` read is captured from unavailable evidence. A warm run against
    /// the same cache root recomposes the child, so it reports the child's
    /// partial-capture warning exactly as the cold run did — the cache cannot
    /// replay output while dropping the diagnostic that explains it (review-1,
    /// "Persistent cache hits discard child partial-capture diagnostics").
    #[test]
    fn a_child_only_partial_capture_warns_identically_on_cold_and_warm_runs() {
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let root = write(directory.path(), "root.md", "root\n\n::file ./child.md\n");
        write(directory.path(), "child.md", "child=[{{ ctx.branch }}]\n");
        let options = || ComposeOptions::new_with_context(root_context(directory.path())).with_cache_root(cache.path());

        let cold = compose_partial(&root, options());
        let warm = compose_partial(&root, options());

        assert_eq!(rendered(&cold.output, "child"), "[]", "{}", cold.output);
        assert_git_partial_capture(&cold.warnings);
        assert_eq!(warm.warnings, cold.warnings, "the warm run changed the warning count or content");
        assert_eq!(warm.output, cold.output);
        assert_nothing_persisted(cache.path());
    }

    /// A request that could capture the group leaves nothing persisted that a
    /// frozen request could read around the missing-capture contract — under
    /// any run-local cache access mode.
    #[test]
    fn a_persistent_entry_cannot_bypass_a_frozen_missing_capture() {
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let root = write(directory.path(), "root.md", "root\n\n::file ./middle.md\n");
        write(directory.path(), "middle.md", "middle\n\n::file ./leaf.md\n");
        write(directory.path(), "leaf.md", "leaf={{ ctx.repo_root }}\n");
        compose_cached(&root, cache.path(), "one");
        assert_nothing_persisted(cache.path());

        for mode in [
            CacheAccessMode::Off,
            CacheAccessMode::ReadOnly,
            CacheAccessMode::ReadWrite,
            CacheAccessMode::Refresh,
        ] {
            let options = ComposeOptions::new_with_context(root_context(directory.path()))
                .with_cache_root(cache.path())
                .with_cache_access_mode(mode);
            let error = compose(&root, options).expect_err("a frozen request cannot read the group");
            assert!(
                matches!(
                    error.missing_runtime_context(),
                    Some(ExpressionError::ContextNotCaptured { group: ContextGroup::Repo, .. })
                ),
                "{mode:?}: {error:?}"
            );
        }
    }
}
