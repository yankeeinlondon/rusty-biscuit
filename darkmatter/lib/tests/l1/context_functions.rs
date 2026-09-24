//! `package()`, `package_area()`, `recent_commits()`, `ipv4()`, and `ipv6()`
//! through the library compose entry point.
//!
//! Repository fixtures are self-contained Git repositories built in-process;
//! interface data is supplied as capture evidence, so no test observes the
//! host's real network or the rusty-biscuit checkout.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOperation, ComposeOptions, ContextCaptureEvidence, ContextGroup,
};
use git2::{IndexAddOption, Repository, RepositoryInitOptions, Signature, Time};
use serde_json::Value;
use sniff::network::{DefaultGateways, InterfaceFlags, Ipv4Cidr, Ipv6Cidr, NetworkInterface};

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// 2020-01-01T00:00:00Z: old enough that no commit renders `Today`/`Yesterday`.
const EPOCH: i64 = 1_577_836_800;

struct Repo {
    _temp: tempfile::TempDir,
    root: PathBuf,
    repo: Repository,
}

impl Repo {
    fn init() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repo = Repository::init_opts(&root, &options).unwrap();
        Self { _temp: temp, root, repo }
    }

    /// A Cargo workspace with top-level package `aaa` and `zeta-lib` in area `zeta`.
    fn monorepo() -> Self {
        let fixture = Self::init();
        write(
            &fixture.root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"aaa\", \"zeta/lib\"]\n",
        );
        for (relative, name) in [("aaa", "aaa"), ("zeta/lib", "zeta-lib")] {
            write(
                &fixture.root.join(relative).join("Cargo.toml"),
                &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            );
            write(&fixture.root.join(relative).join("src/lib.rs"), "");
        }
        fixture.commit("chore: workspace", 0);
        fixture
    }

    /// Stages every change and commits it at `EPOCH` plus `days`.
    fn commit(&self, message: &str, days: i64) {
        let mut index = self.repo.index().unwrap();
        index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree = self.repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            Signature::new("fixture", "fixture@example.com", &Time::new(EPOCH + days * 86_400, 0))
                .unwrap();
        let parent = self.repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<_> = parent.iter().collect();
        self.repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
            .unwrap();
    }

    fn document(&self, relative: &str, content: &str) -> (PathBuf, Markdown) {
        let path = self.root.join(relative);
        write(&path, content);
        (path.clone(), Markdown::try_from(path.as_path()).unwrap())
    }
}

/// Composes `document` with an ambient capture anchored on its own directory.
fn compose_ambient(
    path: &Path,
    document: &Markdown,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> String {
    let context = ComposeContext::capture_for_document(path.parent().unwrap(), document);
    let options = configure(fixture_options(context, path));
    let (composed, report) = document.clone().compose_with(options).expect("compose must succeed");
    assert!(report.warnings.is_empty(), "unexpected warnings: {:?}", report.warnings);
    composed.content().to_string()
}

/// Options for the stages these functions run in. Link resolution and inline
/// cleanup would rewrite the bracketed `name=[value]` probe lines.
fn fixture_options(context: ComposeContext, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(context).with_source_file(path).only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::FrontmatterShellExpansion,
        ComposeOperation::Interpolation,
    ])
}

/// `name=[value]` lines of a composed body.
fn fields(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let (name, value) = line.trim().split_once("=[")?;
            Some((name.to_string(), value.strip_suffix(']')?.to_string()))
        })
        .collect()
}

fn json_array(text: &str) -> Vec<String> {
    let value: Value = serde_json::from_str(text).unwrap_or_else(|error| panic!("{text}: {error}"));
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {text}"))
        .iter()
        .map(|item| item.as_str().unwrap().to_string())
        .collect()
}

const PACKAGE_BODY: &str = "\
nested=[{{ package(\"../zeta/lib/src/new.rs\") }}]

nested_area=[{{ package_area(\"../zeta/lib/src/new.rs\") }}]

area_only=[{{ package_area(\"../zeta/docs/guide.md\") }}]

area_only_package=[{{ package(\"../zeta/docs/guide.md\") }}]

top=[{{ package(\"aaa/src/lib.rs\") }}]

top_area=[{{ package_area(\"aaa/src/lib.rs\") }}]

sibling=[{{ package(\"../zetas/lib/src/lib.rs\") }}]

root_file=[{{ package(\"../Cargo.toml\") }}]

root_file_area=[{{ package_area(\"../Cargo.toml\") }}]
";

/// AC17/AC35 through compose: the functions alone demand the repository
/// capture, the existing implicit reference resolves at the repository root,
/// and misses are empty strings.
#[test]
fn package_lookups_compose_from_the_captured_topology() {
    let fixture = Repo::monorepo();
    let (path, document) = fixture.document("docs/prompt.md", PACKAGE_BODY);
    let context = ComposeContext::capture_for_document(path.parent().unwrap(), &document);
    assert!(context.capture_requirements().contains(ContextGroup::Repo));

    let values = fields(&compose_ambient(&path, &document, |options| options));

    assert_eq!(values["nested"], "zeta-lib");
    assert_eq!(values["nested_area"], "zeta");
    assert_eq!(values["area_only"], "zeta");
    assert_eq!(values["area_only_package"], "");
    assert_eq!(values["top"], "aaa");
    assert_eq!(values["top_area"], "");
    assert_eq!(values["sibling"], "");
    assert_eq!(values["root_file"], "");
    assert_eq!(values["root_file_area"], "");
}

#[test]
fn package_lookups_miss_outside_the_repository_and_in_a_plain_repository() {
    let fixture = Repo::init();
    write(
        &fixture.root.join("Cargo.toml"),
        "[package]\nname = \"solo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&fixture.root.join("src/lib.rs"), "");
    fixture.commit("chore: solo", 0);
    let outside = tempfile::tempdir().unwrap();
    let outside_path = biscuit_file::to_portable_string(&outside.path().join("zeta/lib/src/lib.rs"));
    let (path, document) = fixture.document(
        "prompt.md",
        &format!(
            "plain=[{{{{ package(\"src/lib.rs\") }}}}]\n\nplain_area=[{{{{ package_area(\"src/lib.rs\") }}}}]\n\noutside=[{{{{ package(\"{outside_path}\") }}}}]\n"
        ),
    );

    let values = fields(&compose_ambient(&path, &document, |options| options));

    assert_eq!(values["plain"], "");
    assert_eq!(values["plain_area"], "");
    assert_eq!(values["outside"], "");
}

/// A monorepo fixture with twelve commits, each touching one file.
fn history() -> Repo {
    let fixture = Repo::monorepo();
    for day in 1..=12 {
        write(&fixture.root.join(format!("history/{day:02}.txt")), "entry\n");
        fixture.commit(&format!("feat(history): entry {day:02}\n\n- adds entry {day:02}"), day);
    }
    fixture
}

/// AC25 and the R29 pair rule: a call renders the same per-commit blocks as
/// `ctx.recent_commits`, honors counts above ten, and sees no fewer commits
/// than exist.
#[test]
fn recent_commits_calls_match_the_eager_capture_format() {
    let fixture = history();
    let (path, document) = fixture.document(
        "docs/prompt.md",
        "eager=[{{ ctx.recent_commits }}]\n\nten=[{{ recent_commits(10) }}]\n\nthree=[{{ recent_commits(3) }}]\n\ntwelve=[{{ recent_commits(12) }}]\n\nall=[{{ recent_commits(50) }}]\n",
    );

    let values = fields(&compose_ambient(&path, &document, |options| options));

    let eager = json_array(&values["eager"]);
    assert_eq!(eager.len(), 10);
    assert!(eager[0].starts_with("- [") && eager[0].contains("entry 12"), "{}", eager[0]);
    assert_eq!(json_array(&values["ten"]), eager);
    assert_eq!(json_array(&values["three"]), eager[..3]);
    let twelve = json_array(&values["twelve"]);
    assert_eq!(twelve.len(), 12);
    assert_eq!(twelve[..10], eager[..]);
    assert_eq!(json_array(&values["all"]).len(), 13, "workspace commit plus twelve entries");
}

/// The blocks `sniff repo recent-commits --plain` prints for `root`.
///
/// `sniff/cli/tests/l1/cli.rs::test_repo_recent_commits_plain_is_the_concatenated_per_commit_blocks`
/// pins that CLI's stdout to exactly the concatenation of these blocks, so
/// agreeing with them here closes AC24's parity chain without this crate
/// spawning the `sniff` binary.
fn sniff_plain_blocks(root: &Path, count: usize) -> Vec<String> {
    use sniff::filesystem::git::{GitRepo, RecentCommits, RecentCommitsOptions};
    let repo = GitRepo::discover(root)
        .expect("the fixture is a readable repository")
        .expect("the fixture is a repository");
    let set = RecentCommits::collect(&repo, &RecentCommitsOptions::new().count(count))
        .expect("the fixture's history is readable");
    let options = RecentCommitsOptions::new();
    set.commits()
        .iter()
        .zip(set.plain_blocks(&options))
        .filter(|(commit, _)| !commit.files.is_empty())
        .map(|(_, block)| block)
        .collect()
}

/// AC24: each element of `ctx.recent_commits` is byte-equal to the
/// corresponding block of `sniff repo recent-commits --plain`.
///
/// Both sides reach `plain_blocks`, so what this proves is not that the
/// renderer is correct — that is Sniff's own contract — but that a block
/// survives Darkmatter's capture, JSON array projection, and body
/// interpolation *unaltered*. Commit blocks are multi-line, indented Markdown,
/// which is exactly the shape a pipeline stage can silently reflow, trim, or
/// re-escape. The structural assertions below pin the shape absolutely, so the
/// case does not rest only on the comparison.
#[test]
fn ctx_recent_commits_is_byte_equal_to_the_sniff_plain_rendering() {
    let fixture = history();
    let (path, document) = fixture.document("prompt.md", "eager=[{{ ctx.recent_commits }}]\n");

    let eager = json_array(&fields(&compose_ambient(&path, &document, |options| options))["eager"]);

    assert_eq!(eager.len(), 10, "R27 captures ten commits");
    assert_eq!(eager, sniff_plain_blocks(&fixture.root, 10));

    let newest = &eager[0];
    assert!(newest.starts_with("- ["), "a block opens with its list marker: {newest:?}");
    assert!(newest.contains("entry 12"), "the newest commit leads: {newest:?}");
    assert!(
        newest.contains("  - added: history/12.txt\n"),
        "the files block keeps its two-space continuation indent: {newest:?}"
    );
    assert!(!newest.contains("**"), "plain blocks carry no markup: {newest:?}");
    assert!(
        !eager.iter().any(|block| block.contains("Today") || block.contains("Yesterday")),
        "the fixture's commits are old enough to carry absolute dates: {eager:?}"
    );
}

/// AC37: a commit that touched no files renders no element, so the captured
/// list is shorter than the ten commits the capture walked — and it is still
/// byte-equal to what Sniff renders for the same ten.
#[test]
fn a_commit_that_touched_no_files_renders_no_element() {
    let fixture = history();
    fixture.commit("chore: nothing changed", 13);
    let (path, document) = fixture.document("prompt.md", "eager=[{{ ctx.recent_commits }}]\n");

    let eager = json_array(&fields(&compose_ambient(&path, &document, |options| options))["eager"]);

    assert_eq!(eager.len(), 9, "the empty commit is walked but renders nothing: {eager:?}");
    assert_eq!(eager, sniff_plain_blocks(&fixture.root, 10));
}

/// AC25: a commit made after capture but before body interpolation appears in
/// a call and not in the eager snapshot.
#[test]
fn recent_commits_observes_a_commit_made_mid_compose() {
    let fixture = history();
    write(&fixture.root.join("history/mid.txt"), "mid\n");
    let mut index = fixture.repo.index().unwrap();
    index.add_path(Path::new("history/mid.txt")).unwrap();
    index.write().unwrap();
    // Explicit flags keep an ambient hook environment, identity, signing
    // policy, or commit hook from reaching the fixture commit.
    let command = "git --git-dir=.git --work-tree=. -c user.name=fixture -c user.email=fixture@example.com -c commit.gpgsign=false commit --no-verify -q -m mid-compose";
    let (path, document) = fixture.document(
        "prompt.md",
        &format!("---\nmutate: $({command})\n---\neager=[{{{{ ctx.recent_commits }}}}]\n\nlatest=[{{{{ recent_commits(1) }}}}]\n"),
    );
    let values = fields(&compose_ambient(&path, &document, |options| {
        options
            .with_shell_working_directory(&fixture.root)
            .with_pre_approved_commands(std::collections::HashSet::from([command.to_string()]))
    }));

    let latest = json_array(&values["latest"]);
    assert_eq!(latest.len(), 1);
    assert!(latest[0].contains("mid-compose"), "{}", latest[0]);
    let eager = json_array(&values["eager"]);
    assert!(eager[0].contains("entry 12"), "{}", eager[0]);
    assert!(eager.iter().all(|block| !block.contains("mid-compose")));
}

#[test]
fn recent_commits_is_empty_for_an_unborn_repository_and_outside_one() {
    let unborn = Repo::init();
    let (path, document) = unborn.document("prompt.md", "latest=[{{ recent_commits(3) }}]\n");
    assert_eq!(fields(&compose_ambient(&path, &document, |options| options))["latest"], "[]");

    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("prompt.md");
    write(&path, "latest=[{{ recent_commits(3) }}]\n");
    let document = Markdown::try_from(path.as_path()).unwrap();
    assert_eq!(fields(&compose_ambient(&path, &document, |options| options))["latest"], "[]");
}

#[test]
fn recent_commits_rejects_a_zero_count_during_compose() {
    let fixture = history();
    let (path, document) = fixture.document("prompt.md", "---\nlatest: '{{ recent_commits(0) }}'\n---\nbody\n");
    let context = ComposeContext::capture_for_document(path.parent().unwrap(), &document);
    let error = document
        .compose_with(fixture_options(context, &path))
        .expect_err("a zero count is a compose error");
    assert!(error.to_string().contains("count must be at least 1"), "{error}");
}

/// R28/AC25: an out-of-domain count stops composition in the body, with or
/// without `fail_fast`.
#[test]
fn recent_commits_rejects_invalid_counts_in_the_body() {
    let fixture = history();
    for count in ["0", "-1", "1.5"] {
        let (path, document) =
            fixture.document("prompt.md", &format!("latest=[{{{{ recent_commits({count}) }}}}]\n"));
        let context = ComposeContext::capture_for_document(path.parent().unwrap(), &document);
        let error = document
            .compose_with(fixture_options(context, &path).with_fail_fast(false))
            .expect_err("an invalid count is a compose error in the body");
        assert!(error.to_string().contains("recent_commits"), "{count}: {error}");
    }

    // Every body failure is fatal (dasherized-identifiers R2), so an ordinary
    // type error aborts too, with its own cause rather than the domain one.
    let (path, document) = fixture.document("prompt.md", "latest=[{{ recent_commits(\"3\") }}]\n");
    let context = ComposeContext::capture_for_document(path.parent().unwrap(), &document);
    let error = document
        .compose_with(fixture_options(context, &path).with_fail_fast(false))
        .expect_err("a type error is a compose error in the body");
    assert!(error.to_string().contains("recent_commits"), "{error}");
}

/// R12 exempts only lookup misses: a remote reference in the body is a
/// compose error, not a warning or an empty string.
#[test]
fn package_rejects_a_remote_reference_in_the_body() {
    let fixture = Repo::monorepo();
    let (path, document) =
        fixture.document("prompt.md", "p=[{{ package(\"https://example.com/zeta/lib\") }}]\n");
    let context = ComposeContext::capture_for_document(path.parent().unwrap(), &document);
    let error = document
        .compose_with(fixture_options(context, &path).with_fail_fast(false))
        .expect_err("a remote reference is a compose error in the body");
    assert!(error.to_string().contains("does not accept HTTP(S) URLs"), "{error}");
}

fn interface(name: &str, index: u32, v4: &[Ipv4Addr], v6: &[Ipv6Addr]) -> NetworkInterface {
    NetworkInterface {
        name: name.to_string(),
        mac_address: None,
        ipv4_addresses: v4.iter().map(|address| Ipv4Cidr { address: *address, prefix_len: None }).collect(),
        ipv6_addresses: v6.iter().map(|address| Ipv6Cidr { address: *address, prefix_len: None }).collect(),
        flags: InterfaceFlags::default(),
        index: Some(index),
    }
}

/// AC20 through supplied evidence: the functions alone demand the network
/// capture, and filtering reads the captured interface addresses.
#[test]
fn address_filters_compose_from_supplied_interface_evidence() {
    let interfaces = vec![
        interface("lo0", 1, &[Ipv4Addr::LOCALHOST], &[Ipv6Addr::LOCALHOST]),
        interface(
            "en0",
            7,
            &[Ipv4Addr::new(192, 168, 10, 5), Ipv4Addr::new(169, 254, 1, 2)],
            &["fe80::1".parse().unwrap(), "fd00::5".parse().unwrap()],
        ),
    ];
    let link_local = sniff::network::host_addresses(&interfaces)
        .into_iter()
        .find(|address| address.is_link_local() && address.address().is_ipv6())
        .unwrap()
        .to_string();
    let evidence = ContextCaptureEvidence::new(HashMap::new())
        .with_network_interfaces(Some(interfaces))
        .with_gateways(Some(DefaultGateways::default()));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("prompt.md");
    write(
        &path,
        "v4=[{{ ipv4() }}]\n\nloopback=[{{ ipv4(\"127.0.0.0/8\") }}]\n\nsubstring=[{{ ipv4(\"192.168.10\") }}]\n\nmalformed=[{{ ipv4(\"192.168.10/99\") }}]\n\nv6=[{{ ipv6() }}]\n\nlink_local=[{{ ipv6(\"fe80::/10\") }}]\n\ncross=[{{ ipv6(\"0.0.0.0/0\") }}]\n",
    );
    let document = Markdown::try_from(path.as_path()).unwrap();
    let context = ComposeContext::capture_for_document_with_evidence(temp.path(), &document, &evidence);
    assert!(context.capture_requirements().contains(ContextGroup::Network));

    let (composed, report) = document
        .compose_with(fixture_options(context, &path))
        .expect("compose must succeed");
    assert!(report.warnings.is_empty(), "unexpected warnings: {:?}", report.warnings);
    let values = fields(composed.content());

    assert_eq!(json_array(&values["v4"]), ["192.168.10.5"]);
    assert_eq!(json_array(&values["loopback"]), ["127.0.0.1"]);
    assert_eq!(json_array(&values["substring"]), ["192.168.10.5"]);
    assert_eq!(json_array(&values["malformed"]), Vec::<String>::new());
    assert_eq!(json_array(&values["v6"]), ["fd00::5"]);
    assert_eq!(json_array(&values["link_local"]), [link_local]);
    assert_eq!(json_array(&values["cross"]), Vec::<String>::new());
}
