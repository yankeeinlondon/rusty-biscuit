//! The lazy `current` / `current_env` roots through the real compose pipeline
//! (AC27, AC31, AC32, and the Darkmatter-owned half of AC28/AC36).
//!
//! Every test drives a controlled in-process refresh capability, so nothing
//! here depends on the host's repository, branch, or clock. The two halves of
//! the Q2 memo scope are told apart by a provider whose every observation
//! differs: reads that agree happened inside one expression evaluation, and
//! reads that differ happened in separate ones.

use std::sync::Arc;

use super::*;

use crate::markdown::compose::context::current::test_support::{ScriptedRefresh, SequenceRefresh};
use crate::markdown::compose::expression::ExpressionError;

/// Options whose only refresh capability is `provider`, over a pinned context.
///
/// The context authority stays caller-supplied so no ambient refresh is
/// installed behind the scripted one.
fn options(provider: Arc<dyn crate::markdown::compose::CurrentProvider>) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::fixed_for_testing_with([(
        "branch",
        serde_json::json!("captured-branch"),
    )]))
    .with_current_provider(provider)
}

fn compose(document: &str, options: ComposeOptions) -> (Markdown, ComposeReport) {
    Markdown::from(document)
        .compose_with(options)
        .unwrap_or_else(|error| panic!("compose must succeed: {error}"))
}

fn compose_error(document: &str, options: ComposeOptions) -> MarkdownError {
    Markdown::from(document)
        .compose_with(options)
        .expect_err("compose must fail")
}

/// AC27: one expression's repeated reads agree, and the next expression
/// observes the fact afresh — proven in both directions inside one compose.
#[test]
fn the_memo_scope_is_one_expression_evaluation() {
    let provider = SequenceRefresh::new("branch");
    let (composed, _) = compose(
        "one span: {{ current.branch == current.branch ? current.branch : 'DIFFERED' }}\n",
        options(provider.clone()),
    );

    assert_eq!(
        composed.content().trim(),
        "one span: observation-1",
        "three reads in one span must share one observation",
    );
    assert_eq!(provider.observations(), 1);

    // A second span is a second expression evaluation, so it refreshes. Body
    // interpolation rewrites spans right to left, which is why this compares
    // the two renderings rather than pinning an observation number to a side.
    let provider = SequenceRefresh::new("branch");
    let (composed, _) = compose(
        "left={{ current.branch }} right={{ current.branch }}\n",
        options(provider.clone()),
    );

    let rendered = composed.content().trim().to_string();
    assert!(
        rendered.contains("left=observation-") && rendered.contains("right=observation-"),
        "{rendered}",
    );
    assert!(
        rendered.contains("observation-1") && rendered.contains("observation-2"),
        "two spans must observe the fact twice: {rendered}",
    );
    assert_eq!(provider.observations(), 2);
}

/// AC27: the eager snapshot never moves while the lazy root does.
#[test]
fn ctx_stays_at_the_capture_while_current_refreshes() {
    let provider = SequenceRefresh::new("branch");
    let (composed, _) = compose(
        "eager: {{ ctx.branch }} then {{ ctx.branch }}\n\nlazy: {{ current.branch }} then {{ current.branch }}\n",
        options(provider.clone()),
    );

    let composed = composed.content().to_string();
    assert!(
        composed.contains("eager: captured-branch then captured-branch"),
        "the eager snapshot never moves: {composed}",
    );
    assert!(
        composed.contains("observation-1") && composed.contains("observation-2"),
        "the lazy root refreshed between the two spans: {composed}",
    );
}

/// AC27: the roots resolve on every Darkmatter expression surface —
/// frontmatter, body, and a page-block `when=` condition.
#[test]
fn every_expression_surface_resolves_the_lazy_roots() {
    let provider = ScriptedRefresh::new([
        ("branch", serde_json::json!("release")),
        ("os", serde_json::json!("macos")),
    ]);
    let (composed, _) = compose(
        "---\nfrom_frontmatter: '{{ current.branch }}'\n---\n\
         body {{ current.os }}\n\
         ::block when=\"current.branch == 'release'\"\nshipped\n::end-block\n",
        options(provider.clone()),
    );

    assert_eq!(
        composed.frontmatter().as_map().get("from_frontmatter"),
        Some(&serde_json::json!("release")),
    );
    assert!(composed.content().contains("body macos"), "{}", composed.content());
    assert!(composed.content().contains("shipped"), "{}", composed.content());
    // Every surface reached the provider; none of them answered from `ctx`.
    assert!(provider.calls().contains(&"branch".to_string()));
    assert!(provider.calls().contains(&"os".to_string()));
}

/// AC31: an unreached lazy reference observes nothing. A document that names
/// `current.branch` never asks the provider for the expensive history key, and
/// an unchosen ternary branch never observes its operand.
#[test]
fn an_unreached_lazy_reference_observes_nothing() {
    let provider = ScriptedRefresh::new([
        ("branch", serde_json::json!("main")),
        ("recent_commits", serde_json::json!(["would be expensive"])),
    ]);
    let (composed, _) = compose(
        "{{ true ? current.branch : current.recent_commits }}\n",
        options(provider.clone()),
    );

    assert_eq!(composed.content().trim(), "main");
    assert_eq!(
        provider.calls(),
        ["branch"],
        "the unchosen branch's key must never be observed",
    );
}

/// AC31: a capability the invocation does not supply is `null` plus a
/// `PartialRuntimeCapture` warning — never a stale eager value, never a host
/// probe.
#[test]
fn a_missing_capability_fails_closed_with_a_typed_diagnostic() {
    let provider = ScriptedRefresh::new([("os", serde_json::json!("macos"))]);
    let (composed, report) = compose(
        "branch=[{{ current.branch }}] os=[{{ current.os }}]\n",
        options(provider.clone()),
    );

    assert_eq!(
        composed.content().trim(),
        "branch=[] os=[macos]",
        "an unsupplied capability must not fall back to the captured `ctx.branch`",
    );
    let partial: Vec<_> = report
        .warnings
        .iter()
        .filter(|warning| warning.message.contains("Partial runtime capture for current"))
        .collect();
    assert_eq!(partial.len(), 1, "{:?}", report.warnings);
    assert!(partial[0].message.contains("current.branch"), "{:?}", partial[0]);
}

/// A request that installs no capability at all still fails closed rather than
/// discovering the fact from the host.
#[test]
fn a_caller_supplied_request_without_a_provider_observes_nothing() {
    let options = ComposeOptions::new_with_context(ComposeContext::fixed_for_testing_with([(
        "branch",
        serde_json::json!("captured-branch"),
    )]));
    let (composed, report) = compose("branch=[{{ current.branch }}]\n", options);

    assert_eq!(composed.content().trim(), "branch=[]");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("Partial runtime capture for current")),
        "{:?}",
        report.warnings,
    );
}

/// AC27: the removed nesting is an unknown path, not an alias for `ctx`.
#[test]
fn the_removed_nesting_fails_instead_of_aliasing_ctx() {
    let provider = ScriptedRefresh::new([("branch", serde_json::json!("main"))]);

    for document in [
        "{{ current.ctx.branch }}\n",
        "{{ current.env.HOME }}\n",
        "{{ current_env.ctx.branch }}\n",
    ] {
        let error = compose_error(document, options(provider.clone()));
        assert!(
            error.to_string().contains("is not a member of the reserved"),
            "{document}: {error}",
        );
    }
}

/// A `current.*` reference adds no eager capture, so the group that would have
/// answered it stays uncaptured and `ctx.*` still reports the missing capture.
#[test]
fn a_lazy_reference_never_satisfies_or_creates_an_eager_requirement() {
    let provider = ScriptedRefresh::new([("repo_root", serde_json::json!("/lazy/root"))]);
    let (composed, _) = compose("{{ current.repo_root }}\n", options(provider.clone()));
    assert_eq!(composed.content().trim(), "/lazy/root");

    let error = compose_error("{{ current.repo_root }} {{ ctx.repo_root }}\n", options(provider));
    assert!(
        matches!(
            error.missing_runtime_context(),
            Some(ExpressionError::ContextNotCaptured { .. })
        ),
        "an eager read must still demand its capture: {error:?}",
    );
}

/// Request-owned identity is fixed for the request: `current` reads the same
/// captured value `ctx` does and never asks the provider for it.
#[test]
fn request_owned_identity_is_fixed_even_for_current() {
    let provider = ScriptedRefresh::new([
        ("cwd", serde_json::json!("/provider/should/not/answer")),
        ("branch", serde_json::json!("main")),
    ]);
    let context = ComposeContext::fixed_for_testing_with([("cwd", serde_json::json!("/launch"))]);
    let options = ComposeOptions::new_with_context(context).with_current_provider(provider.clone());

    let (composed, _) = compose("{{ ctx.cwd }} == {{ current.cwd }}\n", options);

    assert_eq!(composed.content().trim(), "/launch == /launch");
    assert!(provider.calls().is_empty(), "{:?}", provider.calls());
}

/// The reserved roots cannot be shadowed by an injected global of the same
/// name: subtree compose resolves them through the request, not the caller's
/// map.
#[test]
fn an_injected_global_cannot_shadow_a_reserved_root() {
    use crate::markdown::compose::subtree::{InjectedGlobal, SubtreeCompose};

    let provider = ScriptedRefresh::new([("branch", serde_json::json!("from-the-request"))]);
    let state = EffectiveStateBuilder::new()
        .with_context(ComposeContext::fixed_for_testing())
        .with_current_authority(
            crate::markdown::compose::CurrentAuthority::default().with_provider(provider.clone()),
        )
        .build()
        .unwrap();

    let resolved = SubtreeCompose::new(&serde_json::json!("{{ current.branch }}"), &state)
        .with_global(
            "current",
            InjectedGlobal::eager(serde_json::json!({ "branch": "from-the-global" })),
        )
        .compose()
        .unwrap();

    assert_eq!(resolved, serde_json::json!("from-the-request"));
    assert_eq!(provider.calls(), ["branch"]);
}

/// A frontmatter key named `current` cannot shadow the reserved root either.
#[test]
fn a_frontmatter_key_cannot_shadow_a_reserved_root() {
    let provider = ScriptedRefresh::new([("branch", serde_json::json!("from-the-request"))]);
    let (composed, _) = compose(
        "---\ncurrent:\n  branch: from-the-frontmatter\n---\n{{ current.branch }}\n",
        options(provider),
    );

    assert_eq!(composed.content().trim(), "from-the-request");
}

/// AC32: pre-flight discovery walks the document's expressions but observes no
/// lazy fact, and reports the reachable reads as metadata instead.
#[test]
fn preflight_plans_the_lazy_reads_without_observing_them() {
    let provider = ScriptedRefresh::new([("branch", serde_json::json!("main"))]);
    let document: Markdown =
        "---\nb: '{{ current.branch }}'\n---\n{{ current_env.PATH }} {{ recent_commits(2) }}\n"
            .into();

    let report = document
        .compose_preflight(&options(provider.clone()))
        .expect("preflight must succeed");

    assert_eq!(report.deferred_context.current_keys().collect::<Vec<_>>(), ["branch"]);
    assert_eq!(report.deferred_context.current_env_keys().collect::<Vec<_>>(), ["PATH"]);
    assert!(report.deferred_context.functions().any(|name| name == "recent_commits"));
    assert!(provider.calls().is_empty(), "preflight must observe nothing: {:?}", provider.calls());
}

/// `current_env` rereads the live process environment at reference time, under
/// the same memo scope as `current`, while `env` keeps the frozen snapshot.
#[test]
#[serial_test::serial(current_env_pipeline)]
fn current_env_rereads_the_live_environment_while_env_stays_frozen() {
    let key = "DARKMATTER_LAZY_ROOT_PIPELINE_TEST";
    unsafe { std::env::set_var(key, "live") };

    let mut context = ComposeContext::fixed_for_testing();
    context.env_mut().insert(key.to_string(), "frozen".to_string());
    let options = ComposeOptions::new_with_context(context);

    let (composed, _) = compose(
        &format!("frozen={{{{ env.{key} }}}} live={{{{ current_env.{key} }}}} missing=[{{{{ current_env.NO_SUCH_LAZY_ROOT_VAR }}}}]\n"),
        options,
    );

    unsafe { std::env::remove_var(key) };
    assert_eq!(composed.content().trim(), "frozen=frozen live=live missing=[]");
}

// ── The ambient provider's repository observation ───────────────────────────
//
// `md compose` (a `DarkmatterOwned` request) installs `AnchoredRefresh`. These
// cases run inside a purpose-built workspace repository so the repository facts
// have values, and read the capture work counters so "no rediscovery" is
// proven rather than inferred from timing. The counters are process-global:
// the deltas below are exact only because nextest runs each test in its own
// process, so no sibling's capture can land between a test's two readings.

mod ambient_repository {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering;

    use super::*;

    use crate::markdown::compose::context::capture::{GIT_DISCOVERY_COUNT, REPOSITORY_DISCOVERY_COUNT};
    use crate::markdown::compose::context::current::CurrentScope;
    use crate::markdown::compose::CurrentAuthority;

    /// A two-package Cargo workspace under a fresh Git repository, anchored
    /// inside the `alpha` package so the current-package keys have values.
    fn workspace_repository() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        // Canonical so a File source under the anchor compares equal to the
        // discovered root on macOS, where the temp dir is a symlink.
        let root = temp.path().canonicalize().unwrap().join("repo");
        write(&root, "Cargo.toml", "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\", \"alpha/cli\"]\n");
        write(&root, "alpha/lib/Cargo.toml", "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        write(&root, "alpha/lib/src/lib.rs", "");
        write(&root, "alpha/cli/Cargo.toml", "[package]\nname = \"alpha-cli\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        write(&root, "alpha/cli/src/main.rs", "fn main() {}\n");
        gix::init(&root).expect("initialize repository");
        let anchor = root.join("alpha").join("lib");
        (temp, anchor)
    }

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    /// Grows the workspace by a third member and names the repository through
    /// a remote, so both the topology and the `repo` metadata change on disk.
    fn change_repository_on_disk(anchor: &Path) {
        let root = anchor.parent().unwrap().parent().unwrap();
        write(root, "Cargo.toml", "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\", \"alpha/cli\", \"gamma\"]\n");
        write(root, "gamma/Cargo.toml", "[package]\nname = \"gamma\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        write(root, "gamma/src/lib.rs", "");
        let config = root.join(".git").join("config");
        let mut contents = std::fs::read_to_string(&config).unwrap();
        contents.push_str("[remote \"origin\"]\n\turl = https://github.com/acme/widget.git\n");
        std::fs::write(config, contents).unwrap();
    }

    /// A Darkmatter-owned request built through the older constructors, with
    /// no embedder-supplied refresh capability: its observation is fixed by
    /// the root pipeline entry rather than at creation.
    fn darkmatter_owned(anchor: &Path, document: &str) -> ComposeOptions {
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(anchor, document))
            .with_context_authority(ContextAuthority::DarkmatterOwned)
    }

    /// `(root discoveries, topology walks)` so far in this process.
    fn discovery_counts() -> (usize, usize) {
        (
            GIT_DISCOVERY_COUNT.load(Ordering::Relaxed),
            REPOSITORY_DISCOVERY_COUNT.load(Ordering::Relaxed),
        )
    }

    /// One expression's read of `path` through `scope`.
    fn read(scope: &CurrentScope, context: &ComposeContext, path: &str) -> Option<serde_json::Value> {
        scope.begin_expression_scope();
        scope.resolve(path, context).unwrap().unwrap()
    }

    /// A `string[]` value as a sorted list: `ctx.packages` promises the set of
    /// package names, not the order the topology walk met them in.
    fn sorted_names(value: Option<&serde_json::Value>) -> Vec<String> {
        let mut names: Vec<String> = value
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("a string array, got {value:?}"))
            .iter()
            .map(|name| name.as_str().unwrap().to_string())
            .collect();
        names.sort();
        names
    }

    /// Repository facts a Darkmatter-owned request captured eagerly answer
    /// `current.*` from that observation: several separate expressions read
    /// the root, the topology, and the current package with zero rediscovery,
    /// and every value equals its `ctx.*` twin.
    #[test]
    fn repository_facts_read_the_request_observation_without_rediscovery() {
        let (_temp, anchor) = workspace_repository();
        // Separate paragraphs: cleanup joins single-newline prose lines.
        let document = "eager: {{ ctx.repo_root }} | {{ ctx.packages }} | {{ ctx.current_package }} | {{ ctx.is_monorepo }}\n\n\
                        lazy: {{ current.repo_root }} | {{ current.packages }} | {{ current.current_package }} | {{ current.is_monorepo }}\n\n\
                        again: {{ current.repo_root }}\n";
        let options = darkmatter_owned(&anchor, document);
        assert!(options.context().capture_requirements().contains(ContextGroup::Repo));

        let before = discovery_counts();
        let (composed, report) = compose(document, options);

        assert_eq!(
            discovery_counts(),
            before,
            "a Repo-group `current.*` read must neither rediscover the root nor rewalk the topology",
        );
        let lines: Vec<&str> = composed.content().lines().filter(|line| !line.is_empty()).collect();
        assert_eq!(lines.len(), 3, "{}", composed.content());
        let eager = lines[0].strip_prefix("eager: ").unwrap();
        let lazy = lines[1].strip_prefix("lazy: ").unwrap();
        assert_eq!(lazy, eager, "`current` must mirror `ctx` for every repository fact");
        assert!(
            eager.contains("alpha-cli") && eager.ends_with("alpha | true"),
            "the fixture must give the facts values, or the equality is vacuous: {eager}",
        );
        let root = eager.split(" | ").next().unwrap();
        assert!(root.ends_with("/repo"), "{root}");
        assert_eq!(lines[2], format!("again: {root}"));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    /// Changing the repository on disk mid-request — a new workspace member and
    /// a remote that names the repository — does not move `current.repo`,
    /// `current.repo_root`, or `current.packages`, while a mutable key observed
    /// through the same provider still refreshes at the anchor.
    #[test]
    fn repository_facts_hold_while_mutable_facts_refresh_after_on_disk_changes() {
        let (_temp, anchor) = workspace_repository();
        let context = ComposeContext::capture_for_content(&anchor, "{{ ctx.packages }}");
        let scope = CurrentScope::new(CurrentAuthority::default().with_ambient_refresh(&context));

        let packages = read(&scope, &context, "current.packages");
        let root = read(&scope, &context, "current.repo_root");
        assert_eq!(sorted_names(packages.as_ref()), ["alpha", "alpha-cli"]);
        assert_eq!(read(&scope, &context, "current.repo"), Some(serde_json::Value::Null));
        // Nothing is committed in the fixture, so every file is untracked; the
        // mutable fact under test is whether a file added later shows up.
        let untracked_before = read(&scope, &context, "current.untracked_files").unwrap();
        assert!(!untracked_before.to_string().contains("scratch.md"), "{untracked_before}");

        change_repository_on_disk(&anchor);
        std::fs::write(anchor.join("scratch.md"), "untracked\n").unwrap();

        // The change is real: a fresh observation sees all of it.
        let fresh = ComposeContext::capture_for_content(&anchor, "{{ ctx.packages }} {{ ctx.repo }}");
        assert_eq!(sorted_names(fresh.get("packages")), ["alpha", "alpha-cli", "gamma"]);
        assert_eq!(fresh.get("repo"), Some(&serde_json::json!("widget")));

        let discoveries = discovery_counts();
        assert_eq!(read(&scope, &context, "current.packages"), packages);
        assert_eq!(read(&scope, &context, "current.repo_root"), root);
        assert_eq!(read(&scope, &context, "current.repo"), Some(serde_json::Value::Null));
        assert_eq!(discovery_counts(), discoveries, "the retained observation answers without discovery");

        let untracked = read(&scope, &context, "current.untracked_files").unwrap();
        assert!(
            untracked.as_array().unwrap().iter().any(|path| path.as_str().unwrap().ends_with("scratch.md")),
            "a mutable key must still observe the anchor now: {untracked}",
        );
    }

    /// A request that never captured the `Repo` group eagerly discovers it
    /// once, at the retained anchor, and every later `Repo`-group read of the
    /// request — across expressions and across the providers the request
    /// builds — answers from that one observation. The count covers the whole
    /// compose, so the file-resolution snapshot must project that same
    /// observation rather than discover its own.
    #[test]
    fn a_request_that_never_captured_repo_discovers_it_once() {
        let (_temp, anchor) = workspace_repository();
        let document = "a={{ current.packages }}\nb={{ current.repo_root }}\nc={{ current.current_package }}\n";
        let options = darkmatter_owned(&anchor, document);
        assert!(!options.context().capture_requirements().contains(ContextGroup::Repo));

        let (roots, walks) = discovery_counts();
        let (composed, report) = compose(document, options);

        assert_eq!(
            discovery_counts(),
            (roots + 1, walks + 1),
            "three Repo-group expressions must cost exactly one discovery",
        );
        assert!(composed.content().contains("alpha-cli") && composed.content().contains("c=alpha"), "{}", composed.content());
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        // The pipeline builds a fresh provider per resolution context; the
        // observation is shared through the request's authority, so a later
        // provider answers from the one the request established even after
        // the repository changed on disk.
        let context = ComposeContext::capture_for_content(&anchor, "no eager references");
        let authority = CurrentAuthority::default();
        authority.establish_ambient_repository(&context);
        let first = CurrentScope::new(authority.with_ambient_refresh(&context));
        let packages = read(&first, &context, "current.packages");
        assert_eq!(sorted_names(packages.as_ref()), ["alpha", "alpha-cli"]);

        change_repository_on_disk(&anchor);
        let discoveries = discovery_counts();
        let second = CurrentScope::new(authority.with_ambient_refresh(&context));
        assert_eq!(read(&second, &context, "current.packages"), packages);
        assert_eq!(discovery_counts(), discoveries, "a later provider reuses the request's observation");
    }

    /// A root document and its `::file` child written under `anchor`, so the
    /// request has an on-disk source for transclusion and validation.
    fn write_documents(anchor: &Path, root: &str, child: &str) -> PathBuf {
        let root_path = anchor.join("root.md");
        std::fs::write(&root_path, root).unwrap();
        std::fs::write(anchor.join("child.md"), child).unwrap();
        root_path
    }

    /// The three repository facts the request must hold at their
    /// request-creation values: no remote yet, the two-member topology.
    fn assert_request_start_values(rendered: &str) {
        assert!(rendered.contains("repo=[]"), "`current.repo` must be the request-start value (no remote yet): {rendered}");
        assert!(!rendered.contains("gamma"), "`current.packages` must be the request-start topology: {rendered}");
        assert!(rendered.contains("alpha-cli"), "{rendered}");
    }

    /// The change made by [`change_repository_on_disk`] is visible to a new
    /// observation, so holding the old values is a property of the request,
    /// not of the fixture.
    fn assert_change_is_real(anchor: &Path) {
        let fresh = ComposeContext::capture_for_content(anchor, "{{ ctx.packages }} {{ ctx.repo }}");
        assert_eq!(fresh.get("repo"), Some(&serde_json::json!("widget")));
        assert!(sorted_names(fresh.get("packages")).contains(&"gamma".to_string()));
    }

    /// D3 through the public request boundary: `ComposeOptions::for_document`
    /// observes the repository once, at creation. The repository's metadata
    /// and topology then change on disk before anything is evaluated, and
    /// every phase `md compose` runs with the same options — reference
    /// validation, pre-flight, compose, and a second compose — answers every
    /// lazy repository read with the creation-time observation at zero
    /// further discovery. No private setup call stands in for the boundary.
    #[test]
    fn the_observation_is_fixed_at_request_creation() {
        let (_temp, anchor) = workspace_repository();
        let document = "repo=[{{ current.repo }}] packages={{ current.packages }} root={{ current.repo_root }}\n";
        let root_path = write_documents(&anchor, document, "");
        let markdown = Markdown::try_from(root_path.as_path()).unwrap();

        let (roots, walks) = discovery_counts();
        let options = ComposeOptions::for_document(&anchor, &markdown).with_source_file(&root_path);
        assert_eq!(discovery_counts(), (roots + 1, walks + 1), "creating the request observes exactly once");
        assert!(!options.context().capture_requirements().contains(ContextGroup::Repo));

        change_repository_on_disk(&anchor);
        assert_change_is_real(&anchor);
        let discoveries = discovery_counts();

        use crate::markdown::reference::ReferenceGraphOptions;
        use crate::markdown::reference::validate::ReferenceValidationOptions;
        let validation = markdown
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(options.clone())))
            .unwrap();
        assert!(validation.issues.is_empty(), "{:?}", validation.issues);
        let preflight = markdown.compose_preflight(&options).unwrap();
        assert_eq!(preflight.deferred_context.current_keys().count(), 3, "{:?}", preflight.deferred_context);
        let (composed, report) = compose(document, options.clone());
        let (again, _) = compose(document, options);

        assert_eq!(discovery_counts(), discoveries, "no phase may rediscover the root or rewalk the topology");
        assert_request_start_values(composed.content());
        assert_eq!(again.content(), composed.content(), "a later phase of the request reads the same observation");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    /// Approves every command and, while doing so, changes the repository on
    /// disk: the mutation lands inside the frontmatter shell stage, after the
    /// request was created and before transclusion runs.
    struct MutatingApproval {
        anchor: PathBuf,
        mutations: std::sync::atomic::AtomicUsize,
    }

    impl crate::markdown::compose::shell_expansion::types::ShellApprovalHandler for MutatingApproval {
        fn approve(
            &self,
            _request: crate::markdown::compose::shell_expansion::types::ShellApprovalRequest,
        ) -> Result<
            crate::markdown::compose::shell_expansion::types::ShellApprovalDecision,
            crate::markdown::compose::shell_expansion::types::ShellExpansionError,
        > {
            change_repository_on_disk(&self.anchor);
            self.mutations.fetch_add(1, Ordering::Relaxed);
            Ok(crate::markdown::compose::shell_expansion::types::ShellApprovalDecision::AllowOnce)
        }
    }

    /// D3 for a descendant-only reader: the root names no repository fact and
    /// only its transcluded child reads `current.repo`, `current.repo_root`,
    /// and `current.packages`. The repository changes during the root's
    /// frontmatter shell stage, well before the child is composed; the child
    /// still observes the request-creation values, and neither the root's
    /// stages nor the child pipeline discover anything after the request was
    /// created. A child pipeline never establishes request state.
    #[test]
    fn a_child_reads_the_observation_fixed_at_request_creation() {
        use crate::markdown::compose::shell_expansion::types::ShellExpansionOptions;

        let (_temp, anchor) = workspace_repository();
        let policy = tempfile::tempdir().unwrap();
        let root = "---\nmarker: \"$(echo mutated)\"\n---\nroot {{ marker }}\n\n::file child.md\n";
        let child = "repo=[{{ current.repo }}] packages={{ current.packages }} root={{ current.repo_root }}\n";
        let root_path = write_documents(&anchor, root, child);
        let markdown = Markdown::try_from(root_path.as_path()).unwrap();
        let approval = Arc::new(MutatingApproval { anchor: anchor.clone(), mutations: Default::default() });

        let (roots, walks) = discovery_counts();
        let options = ComposeOptions::for_document(&anchor, &markdown)
            .with_source_file(&root_path)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(policy.path().to_path_buf()),
                approval_handler: Some(approval.clone()),
                ..Default::default()
            });
        assert_eq!(discovery_counts(), (roots + 1, walks + 1), "creating the request observes exactly once");
        let discoveries = discovery_counts();

        let (composed, report) = compose(root, options);

        assert_eq!(approval.mutations.load(Ordering::Relaxed), 1, "the shell stage must have run the mutation");
        assert_eq!(discovery_counts(), discoveries, "neither the root's stages nor the child pipeline may discover");
        assert!(composed.content().contains("root mutated"), "the mutation preceded transclusion: {}", composed.content());
        assert_request_start_values(composed.content());
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_change_is_real(&anchor);
    }

    /// The fallback boundary: a Darkmatter-owned request built through the
    /// older constructors is fixed at the root pipeline entry, before any
    /// stage runs, even when only a transcluded child reads the repository.
    /// The child pipeline itself performs no discovery.
    #[test]
    fn an_older_constructor_request_is_fixed_at_the_root_entry_not_by_the_child() {
        let (_temp, anchor) = workspace_repository();
        let root = "root only\n\n::file child.md\n";
        let child = "packages={{ current.packages }} root={{ current.repo_root }}\n";
        let root_path = write_documents(&anchor, root, child);
        let options = darkmatter_owned(&anchor, root).with_source_file(&root_path);
        assert!(!options.context().capture_requirements().contains(ContextGroup::Repo));

        let (roots, walks) = discovery_counts();
        let (composed, report) = compose(root, options);

        assert_eq!(
            discovery_counts(),
            (roots + 1, walks + 1),
            "the root entry observes once for the whole request; the child adds nothing",
        );
        assert!(composed.content().contains("alpha-cli"), "{}", composed.content());
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }
}
