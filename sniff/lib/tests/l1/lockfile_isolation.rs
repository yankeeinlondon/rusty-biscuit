//! Lockfile ownership isolation, request-local caching, and request cost
//! through the public detection API (`2026-09-26-lockfile-corroboration`,
//! AC3 and AC4).
//!
//! Engine-level cases that need the crate-private metadata-failure seam
//! (ruling R9) live in the library's `lockfile::tests` and `detection` unit
//! tests; everything here uses only files a test can create on every OS,
//! except the symlink cases, which are Unix-only extras.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sniff::filesystem::repo::{
    LockfileObservation, LockfileReason, LockfileStatus, detect_repo_with_request,
};
use sniff::filesystem::{MonorepoLayer, MonorepoStandard, PackageProvenance, RepoInfo};
use sniff::performance::{PerformanceCollector, counters, with_current_collector};
use sniff::request::RepoRequest;
use tempfile::TempDir;

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture file has a parent")).expect("create parent");
    fs::write(path, content).expect("write fixture file");
}

/// The pnpm lockfile recording exactly `web` and `ui` under `packages/`.
const PNPM_MATCHING: &str =
    "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  packages/web: {}\n  packages/ui: {}\n";

/// A pnpm workspace at `root` with members `packages/web` and `packages/ui`,
/// and `lockfile` as its `pnpm-lock.yaml` when given.
fn pnpm_workspace(root: &Path, lockfile: Option<&str>) {
    write(root, "package.json", r#"{"name":"root","private":true}"#);
    write(root, "pnpm-workspace.yaml", "packages:\n  - \"packages/*\"\n");
    for name in ["web", "ui"] {
        write(
            root,
            &format!("packages/{name}/package.json"),
            &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        );
    }
    if let Some(lockfile) = lockfile {
        write(root, "pnpm-lock.yaml", lockfile);
    }
}

/// An npm workspace at `root` with members `packages/web` and `packages/ui`.
fn npm_workspace(root: &Path) {
    write(
        root,
        "package.json",
        r#"{"name":"root","private":true,"workspaces":["packages/*"]}"#,
    );
    for name in ["web", "ui"] {
        write(
            root,
            &format!("packages/{name}/package.json"),
            &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        );
    }
}

/// A `package-lock.json` v3 recording exactly the npm workspace's members.
const NPM_MATCHING: &str = r#"{
  "name": "root",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": { "name": "root", "workspaces": ["packages/*"] },
    "node_modules/ui": { "resolved": "packages/ui", "link": true },
    "node_modules/web": { "resolved": "packages/web", "link": true },
    "packages/ui": { "name": "ui", "version": "1.0.0" },
    "packages/web": { "name": "web", "version": "1.0.0" }
  }
}
"#;

/// A Cargo workspace at `root` with members `crates/alpha` and `crates/beta`.
fn cargo_workspace(root: &Path) {
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"2\"\n",
    );
    for name in ["alpha", "beta"] {
        write(
            root,
            &format!("crates/{name}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        );
    }
}

fn detect(root: &Path, request: &RepoRequest) -> (RepoInfo, BTreeMap<String, u64>) {
    let collector = PerformanceCollector::new_shared();
    let repo = with_current_collector(Some(Arc::clone(&collector)), || {
        detect_repo_with_request(root, request)
    });
    let repo = repo
        .expect("detection succeeds")
        .expect("fixture is a workspace");
    (repo, collector.snapshot(Duration::ZERO).counters)
}

fn corroborating() -> RepoRequest {
    RepoRequest::structure().with_lockfile_provenance(true)
}

fn counter(counts: &BTreeMap<String, u64>, name: &str) -> u64 {
    counts.get(name).copied().unwrap_or(0)
}

fn layer_at<'a>(repo: &'a RepoInfo, authority: MonorepoStandard, root: &Path) -> &'a MonorepoLayer {
    repo.monorepo_layers
        .iter()
        .find(|layer| layer.authority == authority && layer.root == root)
        .unwrap_or_else(|| {
            panic!(
                "no {authority:?} layer at {}: {:?}",
                root.display(),
                repo.monorepo_layers
            )
        })
}

fn observation(
    status: LockfileStatus,
    paths: &[&str],
    reason: Option<LockfileReason>,
    extra: &[&str],
    missing: &[&str],
) -> LockfileObservation {
    let owned = |items: &[&str]| items.iter().map(|item| (*item).to_owned()).collect();
    LockfileObservation {
        status,
        paths: owned(paths),
        reason,
        extra: owned(extra),
        missing: owned(missing),
    }
}

fn matched(lockfile: &str) -> LockfileObservation {
    observation(LockfileStatus::Match, &[lockfile], None, &[], &[])
}

/// `(relative path with '/', standard, provenance)` for every package.
fn package_provenance(repo: &RepoInfo) -> Vec<(String, MonorepoStandard, PackageProvenance)> {
    let mut rows: Vec<_> = repo
        .packages
        .as_deref()
        .expect("a monorepo reports its package catalog")
        .iter()
        .map(|package| {
            (
                package.relative.replace('\\', "/"),
                package.standard,
                package.provenance,
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

// ---------------------------------------------------------------------------
// Ownership isolation
// ---------------------------------------------------------------------------

/// A root Cargo workspace hosting two nested pnpm workspaces: `web/` has a
/// lockfile recording exactly its members, `docs/` has none. A decoy
/// `pnpm-lock.yaml` at the repository root belongs to no pnpm layer. Each
/// layer reads only its own root's lockfile, compares members relative to
/// its own root, and upgrades only the packages it owns.
#[test]
fn nested_layers_with_different_authorities_observe_only_their_own_lockfile() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    cargo_workspace(root);
    write(
        root,
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"beta\"\nversion = \"0.1.0\"\n",
    );
    // Records `web/…` paths from the repository root; a layer that borrowed
    // it would report a mismatch.
    write(
        root,
        "pnpm-lock.yaml",
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  web/packages/web: {}\n",
    );
    pnpm_workspace(&root.join("web"), Some(PNPM_MATCHING));
    pnpm_workspace(&root.join("docs"), None);

    let (repo, counts) = detect(root, &corroborating());

    let context = format!("{:#?}", repo.monorepo_layers);
    assert_eq!(repo.monorepo_layers.len(), 3, "{context}");
    let cargo = layer_at(&repo, MonorepoStandard::CargoWorkspace, root);
    assert_eq!(
        cargo.lockfile,
        observation(
            LockfileStatus::MembersPresent,
            &["Cargo.lock"],
            Some(LockfileReason::SubsetOnly),
            &[],
            &[]
        ),
        "{context}"
    );
    assert_eq!(cargo.provenance, PackageProvenance::Globbed, "{context}");

    let web = layer_at(&repo, MonorepoStandard::PnpmWorkspaces, &root.join("web"));
    assert_eq!(web.lockfile, matched("pnpm-lock.yaml"), "{context}");
    assert_eq!(web.provenance, PackageProvenance::Lockfile, "{context}");

    let docs = layer_at(&repo, MonorepoStandard::PnpmWorkspaces, &root.join("docs"));
    assert_eq!(
        docs.lockfile,
        observation(LockfileStatus::Absent, &[], None, &[], &[]),
        "an ancestor's lockfile is never borrowed: {context}"
    );
    assert_eq!(docs.provenance, PackageProvenance::Globbed, "{context}");

    // Only `web/`'s packages are upgraded; Cargo's subset evidence and the
    // lockfile-less `docs/` layer leave theirs manifest-derived.
    let upgraded: Vec<String> = package_provenance(&repo)
        .into_iter()
        .filter(|(_, _, provenance)| *provenance == PackageProvenance::Lockfile)
        .map(|(relative, standard, _)| {
            assert_eq!(standard, MonorepoStandard::PnpmWorkspaces, "{relative}");
            relative
        })
        .collect();
    assert_eq!(upgraded, ["web/packages/ui", "web/packages/web"], "{context}");

    // `Cargo.lock` and `web/pnpm-lock.yaml` only: the decoy is never read.
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 2, "{counts:?}");
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 2, "{counts:?}");
}

/// Yarn and npm both claim the same `package.json` workspaces at one root.
/// The Yarn layer matches its `yarn.lock`; the overlapping npm layer has no
/// npm lockfile and never adopts Yarn's, and does not take over the packages
/// Yarn owns.
#[test]
fn overlapping_layers_at_one_root_never_share_another_authority_lockfile() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    npm_workspace(root);
    write(
        root,
        "yarn.lock",
        "__metadata:\n  version: 8\n  cacheKey: 10c0\n\n\
         \"root@workspace:.\":\n  version: 0.0.0-use.local\n  resolution: \"root@workspace:.\"\n  languageName: unknown\n  linkType: soft\n\n\
         \"ui@workspace:packages/ui\":\n  version: 0.0.0-use.local\n  resolution: \"ui@workspace:packages/ui\"\n  languageName: unknown\n  linkType: soft\n\n\
         \"web@workspace:packages/web\":\n  version: 0.0.0-use.local\n  resolution: \"web@workspace:packages/web\"\n  languageName: unknown\n  linkType: soft\n",
    );

    let (repo, counts) = detect(root, &corroborating());

    let context = format!("{:#?}", repo.monorepo_layers);
    let yarn = layer_at(&repo, MonorepoStandard::YarnWorkspaces, root);
    assert_eq!(yarn.lockfile, matched("yarn.lock"), "{context}");
    assert_eq!(yarn.provenance, PackageProvenance::Lockfile, "{context}");
    let npm = layer_at(&repo, MonorepoStandard::NpmWorkspaces, root);
    assert_eq!(
        npm.lockfile,
        observation(LockfileStatus::Absent, &[], None, &[], &[]),
        "{context}"
    );
    assert_eq!(npm.provenance, PackageProvenance::Globbed, "{context}");
    assert_eq!(
        package_provenance(&repo),
        [
            (
                "packages/ui".to_owned(),
                MonorepoStandard::YarnWorkspaces,
                PackageProvenance::Lockfile
            ),
            (
                "packages/web".to_owned(),
                MonorepoStandard::YarnWorkspaces,
                PackageProvenance::Lockfile
            ),
        ]
    );
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{counts:?}");
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 1, "{counts:?}");
}

/// A corroborating call, then cheaper declining calls, then an edit and a
/// corroborating call on one tree: nothing carries over between requests. The
/// declining call neither inherits the earlier upgrade nor reads the file,
/// and the edited lockfile is seen by the next call.
#[test]
fn repeat_calls_with_a_cheaper_request_inherit_no_upgrade() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    pnpm_workspace(root, Some(PNPM_MATCHING));
    let every_package = |repo: &RepoInfo| -> Vec<PackageProvenance> {
        package_provenance(repo)
            .into_iter()
            .map(|(_, _, provenance)| provenance)
            .collect()
    };

    let (repo, counts) = detect(root, &corroborating());
    assert_eq!(repo.monorepo_layers[0].lockfile, matched("pnpm-lock.yaml"));
    assert_eq!(every_package(&repo), [PackageProvenance::Lockfile; 2]);
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{counts:?}");

    let not_requested = observation(
        LockfileStatus::NotRequested,
        &["pnpm-lock.yaml"],
        Some(LockfileReason::RequestDisabled),
        &[],
        &[],
    );
    for (label, request) in [
        ("structure", RepoRequest::structure()),
        ("full, declined", RepoRequest::full().with_lockfile_provenance(false)),
    ] {
        let (repo, counts) = detect(root, &request);
        let layer = &repo.monorepo_layers[0];
        assert_eq!(layer.lockfile, not_requested, "{label}");
        assert_eq!(layer.provenance, PackageProvenance::Globbed, "{label}");
        assert_eq!(every_package(&repo), [PackageProvenance::Globbed; 2], "{label}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 0, "{label}: {counts:?}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 0, "{label}: {counts:?}");
    }

    // No cross-request cache hides an edited file.
    write(
        root,
        "pnpm-lock.yaml",
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  packages/web: {}\n",
    );
    let (repo, counts) = detect(root, &corroborating());
    assert_eq!(
        repo.monorepo_layers[0].lockfile,
        observation(
            LockfileStatus::Mismatch,
            &["pnpm-lock.yaml"],
            None,
            &[],
            &["packages/ui"]
        )
    );
    assert_eq!(every_package(&repo), [PackageProvenance::Globbed; 2]);
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{counts:?}");
}

// ---------------------------------------------------------------------------
// Filename precedence and failures
// ---------------------------------------------------------------------------

/// The selected higher-priority file fails and the lower-priority file is
/// never consulted, even though it would match (ruling R6 and "do not retry
/// a lower-priority file"). A directory in the selected slot is a read
/// failure on every OS (ruling R9).
#[test]
fn a_failed_selected_lockfile_is_never_retried_with_a_lower_priority_file() {
    enum Selected {
        File(&'static str),
        Directory,
    }
    struct Case {
        label: &'static str,
        selected: &'static str,
        selected_content: Selected,
        lower: &'static str,
        lower_content: &'static str,
        expected: LockfileObservation,
        parses: u64,
    }
    let failed = |selected, status, reason| {
        observation(status, &[selected], Some(reason), &[], &[])
    };
    let cases = [
        Case {
            label: "malformed shrinkwrap",
            selected: "npm-shrinkwrap.json",
            selected_content: Selected::File("{\"lockfileVersion\": 3, \"packages\": {}} trailing"),
            lower: "package-lock.json",
            lower_content: NPM_MATCHING,
            expected: failed(
                "npm-shrinkwrap.json",
                LockfileStatus::Unreadable,
                LockfileReason::ParseFailed,
            ),
            parses: 1,
        },
        Case {
            label: "unsupported shrinkwrap version",
            selected: "npm-shrinkwrap.json",
            selected_content: Selected::File("{\"lockfileVersion\": 1, \"dependencies\": {}}"),
            lower: "package-lock.json",
            lower_content: NPM_MATCHING,
            expected: failed(
                "npm-shrinkwrap.json",
                LockfileStatus::Unverifiable,
                LockfileReason::UnsupportedVersion,
            ),
            parses: 1,
        },
        Case {
            label: "directory as shrinkwrap",
            selected: "npm-shrinkwrap.json",
            selected_content: Selected::Directory,
            lower: "package-lock.json",
            lower_content: NPM_MATCHING,
            expected: failed(
                "npm-shrinkwrap.json",
                LockfileStatus::Unreadable,
                LockfileReason::ReadFailed,
            ),
            parses: 0,
        },
        // A `bun.lock*` file makes the `package.json` workspace a Bun layer.
        Case {
            label: "malformed text bun.lock beside bun.lockb",
            selected: "bun.lock",
            selected_content: Selected::File("{\"lockfileVersion\": 1, \"workspaces\": {"),
            lower: "bun.lockb",
            lower_content: "binary",
            expected: failed("bun.lock", LockfileStatus::Unreadable, LockfileReason::ParseFailed),
            parses: 1,
        },
    ];
    for case in cases {
        let label = case.label;
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        npm_workspace(root);
        match case.selected_content {
            Selected::File(content) => write(root, case.selected, content),
            Selected::Directory => {
                fs::create_dir(root.join(case.selected)).expect("create directory");
            }
        }
        write(root, case.lower, case.lower_content);

        let (repo, counts) = detect(root, &corroborating());

        let layer = repo
            .monorepo_layers
            .iter()
            .find(|layer| layer.lockfile.paths.iter().any(|path| path == case.selected))
            .unwrap_or_else(|| panic!("{label}: {:#?}", repo.monorepo_layers));
        assert_eq!(layer.lockfile, case.expected, "{label}");
        assert_eq!(layer.provenance, PackageProvenance::Globbed, "{label}");
        // Exactly one attempted open, of the selected file.
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{label}: {counts:?}");
        assert_eq!(
            counter(&counts, counters::REPO_LOCKFILE_PARSES),
            case.parses,
            "{label}: {counts:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Path spellings
// ---------------------------------------------------------------------------

/// Members under dot-directories keep their leading dots in both sets: a
/// `.tools/lint` member matches the lockfile's `.tools/lint` importer, and a
/// stale `.cache/old` importer is reported by that exact spelling.
#[test]
fn hidden_members_keep_their_leading_dots() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(root, "package.json", r#"{"name":"root","private":true}"#);
    write(
        root,
        "pnpm-workspace.yaml",
        "packages:\n  - \"packages/*\"\n  - \".tools/*\"\n",
    );
    for member in ["packages/web", ".tools/lint"] {
        let name = member.rsplit('/').next().expect("member name");
        write(
            root,
            &format!("{member}/package.json"),
            &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        );
    }
    let lockfile = |importers: &str| format!("lockfileVersion: '9.0'\nimporters:\n  .: {{}}\n{importers}");

    write(
        root,
        "pnpm-lock.yaml",
        &lockfile("  ./.tools/lint/: {}\n  packages/web: {}\n"),
    );
    let (repo, _) = detect(root, &corroborating());
    assert_eq!(repo.monorepo_layers[0].lockfile, matched("pnpm-lock.yaml"));
    let mut members = repo.monorepo_layers[0].packages.clone();
    members.iter_mut().for_each(|member| *member = member.replace('\\', "/"));
    members.sort();
    assert_eq!(members, [".tools/lint", "packages/web"]);

    write(
        root,
        "pnpm-lock.yaml",
        &lockfile("  .cache/old: {}\n  packages/web: {}\n"),
    );
    let (repo, _) = detect(root, &corroborating());
    assert_eq!(
        repo.monorepo_layers[0].lockfile,
        observation(
            LockfileStatus::Mismatch,
            &["pnpm-lock.yaml"],
            None,
            &[".cache/old"],
            &[".tools/lint"]
        )
    );
}

/// The same tree reached through a root spelled with `.` and `..`
/// components reports the same lockfile observation as the plain spelling:
/// `paths` and members are layer-relative, never the spelling of the root.
#[test]
fn a_root_spelled_with_dot_components_reports_the_same_observation() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("repo");
    pnpm_workspace(&root, Some(PNPM_MATCHING));

    let plain = detect(&root, &corroborating()).0.monorepo_layers[0].lockfile.clone();
    let dotted = root.join(".").join("packages").join("..");
    let spelled = detect(&dotted, &corroborating()).0.monorepo_layers[0]
        .lockfile
        .clone();

    assert_eq!(plain, matched("pnpm-lock.yaml"));
    assert_eq!(spelled, plain);
}

/// A symlinked lockfile is followed: its target is read once and
/// corroborates. A dangling symlink has no file behind it, so the candidate
/// is `absent` and nothing is opened. The macOS temporary directory is itself
/// behind a symlink, so the canonical and the symlinked spellings of the root
/// are both exercised, and a root reached through a directory symlink reports
/// the same observation.
#[cfg(unix)]
#[test]
fn symlinked_lockfiles_and_roots_are_followed() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new().expect("tempdir");
    let canonical = dir.path().canonicalize().expect("canonical tempdir");
    let root = canonical.join("repo");
    pnpm_workspace(&root, None);
    write(&canonical, "shared/pnpm-lock.yaml", PNPM_MATCHING);
    symlink(
        canonical.join("shared/pnpm-lock.yaml"),
        root.join("pnpm-lock.yaml"),
    )
    .expect("symlink lockfile");
    let linked_root = canonical.join("linked-repo");
    symlink(&root, &linked_root).expect("symlink root");

    for spelling in [
        root.clone(),
        linked_root.clone(),
        // The uncanonicalized temporary directory (`/var/…` on macOS).
        dir.path().join("repo"),
    ] {
        let (repo, counts) = detect(&spelling, &corroborating());
        assert_eq!(
            repo.monorepo_layers[0].lockfile,
            matched("pnpm-lock.yaml"),
            "{}",
            spelling.display()
        );
        assert_eq!(
            repo.monorepo_layers[0].provenance,
            PackageProvenance::Lockfile,
            "{}",
            spelling.display()
        );
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{counts:?}");
    }

    fs::remove_file(canonical.join("shared/pnpm-lock.yaml")).expect("dangle the symlink");
    let (repo, counts) = detect(&root, &corroborating());
    assert_eq!(
        repo.monorepo_layers[0].lockfile,
        observation(LockfileStatus::Absent, &[], None, &[], &[])
    );
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 0, "{counts:?}");
}

/// Windows-native spellings: a root given with `\` separators and a
/// lower-cased drive letter still yields `/`-separated, layer-relative member
/// paths, so the lockfile's `/` importers match.
#[cfg(windows)]
#[test]
fn a_windows_native_root_spelling_yields_slash_separated_members() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("repo");
    write(&root, "package.json", r#"{"name":"root","private":true}"#);
    write(&root, "pnpm-workspace.yaml", "packages:\n  - \"packages\\\\*\"\n");
    for name in ["web", "ui"] {
        write(
            &root,
            &format!("packages\\{name}\\package.json"),
            &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        );
    }
    write(&root, "pnpm-lock.yaml", PNPM_MATCHING);

    let text = root.to_str().expect("UTF-8 temporary root");
    let mut spellings = vec![root.clone(), PathBuf::from(text.replace('/', "\\"))];
    if let Some(rest) = text.strip_prefix(|c: char| c.is_ascii_uppercase()) {
        let drive = text.chars().next().expect("drive letter").to_ascii_lowercase();
        spellings.push(PathBuf::from(format!("{drive}{rest}")));
    }
    for spelling in spellings {
        let (repo, _) = detect(&spelling, &corroborating());
        assert_eq!(
            repo.monorepo_layers[0].lockfile,
            matched("pnpm-lock.yaml"),
            "{}",
            spelling.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Request cost
// ---------------------------------------------------------------------------

/// Counters that start or extend a descendant walk.
const WALK_COUNTERS: [&str; 5] = [
    counters::FS_WALK_STARTS,
    counters::FS_WALK_ENTRIES,
    counters::FS_READ_DIRS,
    counters::REPO_NESTED_MARKER_WALKS,
    counters::REPO_MEMBERSHIP_GLOB_WALKS,
];

fn walk_work(counts: &BTreeMap<String, u64>) -> Vec<(&'static str, u64)> {
    WALK_COUNTERS
        .iter()
        .map(|name| (*name, counter(counts, name)))
        .collect()
}

/// A declining structure request reads and parses no lockfile but still
/// probes, and lockfile work adds no walk: the walk counters are identical
/// whether the lockfile is absent, present and declined, or present and
/// corroborated.
#[test]
fn a_declining_structure_request_probes_without_reading_or_walking() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    pnpm_workspace(root, None);
    let (_, without_lockfile) = detect(root, &RepoRequest::structure());

    write(root, "pnpm-lock.yaml", PNPM_MATCHING);
    let (repo, declined) = detect(root, &RepoRequest::structure());
    assert_eq!(
        repo.monorepo_layers[0].lockfile,
        observation(
            LockfileStatus::NotRequested,
            &["pnpm-lock.yaml"],
            Some(LockfileReason::RequestDisabled),
            &[],
            &[]
        )
    );
    assert_eq!(counter(&declined, counters::REPO_LOCKFILE_READS), 0, "{declined:?}");
    assert_eq!(counter(&declined, counters::REPO_LOCKFILE_PARSES), 0, "{declined:?}");
    assert!(
        counter(&declined, counters::REPO_LOCKFILE_PROBES) > 0,
        "{declined:?}"
    );
    // Probing a present file costs what probing an absent one does.
    assert_eq!(
        counter(&declined, counters::REPO_LOCKFILE_PROBES),
        counter(&without_lockfile, counters::REPO_LOCKFILE_PROBES),
    );

    let (_, corroborated) = detect(root, &corroborating());
    assert_eq!(walk_work(&declined), walk_work(&without_lockfile));
    assert_eq!(walk_work(&corroborated), walk_work(&declined));
    assert_eq!(counter(&corroborated, counters::REPO_LOCKFILE_READS), 1);
}

/// Across a Cargo, an npm, and a pnpm layer, each selected lockfile is
/// opened once and parsed once. The npm lockfile is malformed; its cached
/// parse failure is not retried by the full request's later consumers.
#[test]
fn an_enabled_request_reads_and_parses_each_selected_lockfile_once() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    cargo_workspace(root);
    write(
        root,
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"beta\"\nversion = \"0.1.0\"\n",
    );
    npm_workspace(&root.join("js"));
    write(root, "js/package-lock.json", "{\"lockfileVersion\": 3, \"packages\": {");
    pnpm_workspace(&root.join("web"), Some(PNPM_MATCHING));

    for (label, request) in [("structure", corroborating()), ("full", RepoRequest::full())] {
        let (repo, counts) = detect(root, &request);
        let npm = layer_at(&repo, MonorepoStandard::NpmWorkspaces, &root.join("js"));
        assert_eq!(
            npm.lockfile,
            observation(
                LockfileStatus::Unreadable,
                &["package-lock.json"],
                Some(LockfileReason::ParseFailed),
                &[],
                &[]
            ),
            "{label}"
        );
        assert_eq!(
            layer_at(&repo, MonorepoStandard::PnpmWorkspaces, &root.join("web")).lockfile,
            matched("pnpm-lock.yaml"),
            "{label}"
        );
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 3, "{label}: {counts:?}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 3, "{label}: {counts:?}");
    }
}

/// A full request that declines corroboration still resolves dependency
/// versions from `Cargo.lock` (an authorized read, shared with the same
/// request cache), reports `not_requested`, and never upgrades provenance.
/// A malformed `Cargo.lock` is read and parsed once and leaves the versions
/// unresolved.
#[test]
fn a_full_request_declining_corroboration_still_reads_cargo_lock_for_versions() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    cargo_workspace(root);
    write(
        root,
        "crates/alpha/Cargo.toml",
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    write(
        root,
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"beta\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"serde\"\nversion = \"1.0.228\"\n\
         source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    );
    let request = RepoRequest::full().with_lockfile_provenance(false);
    let serde_version = |repo: &RepoInfo| -> Option<String> {
        let alpha = repo
            .packages
            .as_deref()
            .expect("packages")
            .iter()
            .find(|package| package.name == "alpha")
            .expect("alpha");
        alpha
            .dependencies
            .as_deref()?
            .iter()
            .find(|dependency| dependency.name == "serde")?
            .actual_version
            .clone()
    };

    let (repo, counts) = detect(root, &request);
    let layer = &repo.monorepo_layers[0];
    assert_eq!(
        layer.lockfile,
        observation(
            LockfileStatus::NotRequested,
            &["Cargo.lock"],
            Some(LockfileReason::RequestDisabled),
            &[],
            &[]
        )
    );
    assert_eq!(layer.provenance, PackageProvenance::Globbed);
    for package in repo.packages.as_deref().expect("packages") {
        assert_ne!(package.provenance, PackageProvenance::Lockfile, "{package:?}");
    }
    assert_eq!(serde_version(&repo).as_deref(), Some("1.0.228"));
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{counts:?}");
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 1, "{counts:?}");

    write(root, "Cargo.lock", "version = 3\n[[package]\n");
    let (repo, counts) = detect(root, &request);
    assert_eq!(repo.monorepo_layers[0].lockfile.status, LockfileStatus::NotRequested);
    assert_eq!(serde_version(&repo), None);
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{counts:?}");
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 1, "{counts:?}");
}

/// Regression (found by the Phase 5 corpus pass on this repository): a
/// directory both matched by `members` and listed in `[workspace].exclude`
/// is not a workspace member, so Cargo never locks it as one. Its absence
/// from `Cargo.lock` is not a missing member, and an excluded directory whose
/// manifest cannot even be parsed does not make discovery incomplete.
#[test]
fn a_cargo_workspace_exclude_is_not_a_missing_member() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    cargo_workspace(root);
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/legacy\", \"crates/broken\"]\n\
         resolver = \"2\"\n",
    );
    write(
        root,
        "crates/legacy/Cargo.toml",
        "[package]\nname = \"legacy\"\nversion = \"0.1.0\"\n",
    );
    write(root, "crates/broken/Cargo.toml", "[package\nname = \n");
    write(
        root,
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"beta\"\nversion = \"0.1.0\"\n",
    );
    let members_present = observation(
        LockfileStatus::MembersPresent,
        &["Cargo.lock"],
        Some(LockfileReason::SubsetOnly),
        &[],
        &[],
    );

    for (label, request) in [("structure", corroborating()), ("full", RepoRequest::full())] {
        let (repo, _) = detect(root, &request);
        let layer = layer_at(&repo, MonorepoStandard::CargoWorkspace, root);
        assert_eq!(layer.lockfile, members_present, "{label}: {layer:?}");
        assert_eq!(layer.provenance, PackageProvenance::Globbed, "{label}");
    }

    // A real member missing from the lockfile is still reported.
    write(
        root,
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
    );
    let (repo, _) = detect(root, &corroborating());
    assert_eq!(
        layer_at(&repo, MonorepoStandard::CargoWorkspace, root).lockfile,
        observation(
            LockfileStatus::MembersMissing,
            &["Cargo.lock"],
            Some(LockfileReason::SubsetOnly),
            &[],
            &["crates/beta"]
        )
    );
}

// ---------------------------------------------------------------------------
// Incomplete manifest discovery (ruling R10)
// ---------------------------------------------------------------------------

fn incomplete(lockfile: &str) -> LockfileObservation {
    observation(
        LockfileStatus::Unverifiable,
        &[lockfile],
        Some(LockfileReason::IncompleteManifestDiscovery),
        &[],
        &[],
    )
}

/// Asserts the layer at `root` is `unverifiable` for incomplete discovery
/// under both a corroborating structure request and a full request, and that
/// neither the layer nor any package is upgraded to lockfile provenance.
fn assert_incomplete(
    root: &Path,
    authority: MonorepoStandard,
    lockfile: &str,
    provenance: PackageProvenance,
) {
    for (label, request) in [("structure", corroborating()), ("full", RepoRequest::full())] {
        let (repo, _) = detect(root, &request);
        let layer = layer_at(&repo, authority, root);
        assert_eq!(layer.lockfile, incomplete(lockfile), "{label}: {layer:?}");
        assert_eq!(layer.provenance, provenance, "{label}");
        for (relative, standard, package_provenance) in package_provenance(&repo) {
            if standard == authority {
                assert_eq!(package_provenance, provenance, "{label}: {relative}");
            }
        }
    }
}

/// A malformed member `package.json` leaves the manifest side incomplete,
/// even though the lockfile records exactly the discovered members: the
/// layer is `unverifiable` with manifest-derived provenance, and detection
/// still reports the layer, its members (`members`, sorted), and its
/// lockfile path.
///
/// The control repairs the member and expects `match`, so the malformed
/// manifest was the only thing blocking comparison.
fn assert_malformed_js_member_is_incomplete(
    root: &Path,
    authority: MonorepoStandard,
    lockfile: &str,
    provenance: PackageProvenance,
    member: &str,
    members: &[&str],
) {
    let manifest = format!("{member}/package.json");
    let name = member.rsplit('/').next().expect("member has a name");
    write(root, &manifest, &format!(r#"{{"name":"{name}","#));

    assert_incomplete(root, authority, lockfile, provenance);
    for (label, request) in [("structure", corroborating()), ("full", RepoRequest::full())] {
        let (repo, _) = detect(root, &request);
        let layer = layer_at(&repo, authority, root);
        let mut layer_members = layer.packages.clone();
        layer_members.sort();
        assert_eq!(layer_members, members, "{label}: {layer:?}");
        let catalog: Vec<String> = package_provenance(&repo)
            .into_iter()
            .filter(|(_, standard, _)| *standard == authority)
            .map(|(relative, _, _)| relative)
            .collect();
        assert_eq!(catalog, members, "{label}");
    }

    write(
        root,
        &manifest,
        &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
    );
    let (repo, _) = detect(root, &corroborating());
    let layer = layer_at(&repo, authority, root);
    assert_eq!(layer.lockfile, matched(lockfile), "control: {layer:?}");
    assert_eq!(layer.provenance, PackageProvenance::Lockfile, "control");
}

#[test]
fn a_malformed_pnpm_member_manifest_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    pnpm_workspace(root, Some(PNPM_MATCHING));

    assert_malformed_js_member_is_incomplete(
        root,
        MonorepoStandard::PnpmWorkspaces,
        "pnpm-lock.yaml",
        PackageProvenance::Globbed,
        "packages/ui",
        &["packages/ui", "packages/web"],
    );
}

#[test]
fn a_malformed_npm_member_manifest_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    npm_workspace(root);
    write(root, "package-lock.json", NPM_MATCHING);

    assert_malformed_js_member_is_incomplete(
        root,
        MonorepoStandard::NpmWorkspaces,
        "package-lock.json",
        PackageProvenance::Globbed,
        "packages/web",
        &["packages/ui", "packages/web"],
    );
}

#[test]
fn a_malformed_yarn_member_manifest_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    npm_workspace(root);
    write(
        root,
        "yarn.lock",
        "__metadata:\n  version: 8\n  cacheKey: 10c0\n\n\
         \"root@workspace:.\":\n  version: 0.0.0-use.local\n  resolution: \"root@workspace:.\"\n  languageName: unknown\n  linkType: soft\n\n\
         \"ui@workspace:packages/ui\":\n  version: 0.0.0-use.local\n  resolution: \"ui@workspace:packages/ui\"\n  languageName: unknown\n  linkType: soft\n\n\
         \"web@workspace:packages/web\":\n  version: 0.0.0-use.local\n  resolution: \"web@workspace:packages/web\"\n  languageName: unknown\n  linkType: soft\n",
    );

    assert_malformed_js_member_is_incomplete(
        root,
        MonorepoStandard::YarnWorkspaces,
        "yarn.lock",
        PackageProvenance::Globbed,
        "packages/ui",
        &["packages/ui", "packages/web"],
    );
}

#[test]
fn a_malformed_bun_member_manifest_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    npm_workspace(root);
    write(
        root,
        "bun.lock",
        r#"{
  "lockfileVersion": 1,
  "workspaces": {
    "": { "name": "root" },
    "packages/ui": { "name": "ui", "version": "1.0.0" },
    "packages/web": { "name": "web", "version": "1.0.0" },
  },
  "packages": {},
}
"#,
    );

    assert_malformed_js_member_is_incomplete(
        root,
        MonorepoStandard::BunWorkspaces,
        "bun.lock",
        PackageProvenance::Globbed,
        "packages/web",
        &["packages/ui", "packages/web"],
    );
}

#[test]
fn a_malformed_rush_member_manifest_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(
        root,
        "rush.json",
        r#"{ "pnpmVersion": "9.15.9", "projects": [
            { "packageName": "web", "projectFolder": "apps/web" },
            { "packageName": "ui", "projectFolder": "apps/ui" }
        ] }"#,
    );
    for name in ["web", "ui"] {
        write(
            root,
            &format!("apps/{name}/package.json"),
            &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        );
    }
    write(
        root,
        "common/config/rush/pnpm-config.json",
        r#"{ "useWorkspaces": true }"#,
    );
    write(
        root,
        "common/config/rush/pnpm-lock.yaml",
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  ../../apps/ui: {}\n  ../../apps/web: {}\n",
    );

    assert_malformed_js_member_is_incomplete(
        root,
        MonorepoStandard::RushStack,
        "common/config/rush/pnpm-lock.yaml",
        PackageProvenance::Explicit,
        "apps/ui",
        &["apps/ui", "apps/web"],
    );
}

/// Unlike a member, a malformed workspace root `package.json` still fails
/// detection: it is the authority's own declaration, so there is no layer
/// whose observation could carry the failure.
#[test]
fn a_malformed_workspace_root_manifest_fails_detection() {
    for (label, setup) in [
        ("npm", npm_workspace as fn(&Path)),
        ("pnpm", |root: &Path| pnpm_workspace(root, Some(PNPM_MATCHING))),
    ] {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        setup(root);
        write(root, "package.json", r#"{"name":"root","#);

        for request in [corroborating(), RepoRequest::full()] {
            let error = detect_repo_with_request(root, &request)
                .expect_err(&format!("{label}: a malformed root manifest fails detection"));
            assert!(
                error.to_string().contains("EOF while parsing"),
                "{label}: unexpected error {error}"
            );
        }
    }
}

/// A member `pyproject.toml` that fails to parse leaves the manifest side
/// incomplete, even though the lockfile records exactly the discovered
/// members and would otherwise be a `match`.
#[test]
fn a_malformed_uv_member_pyproject_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(
        root,
        "pyproject.toml",
        "[project]\nname = \"root-py\"\nversion = \"0.1.0\"\n\n\
         [tool.uv.workspace]\nmembers = [\"py/*\"]\n",
    );
    write(
        root,
        "py/lib-a/pyproject.toml",
        "[project]\nname = \"lib-a\"\nversion = \"0.1.0\"\n",
    );
    write(root, "py/lib-b/pyproject.toml", "[project\nname = \n");
    write(
        root,
        "uv.lock",
        "version = 1\nrevision = 3\n\n[manifest]\nmembers = [\"lib-a\", \"lib-b\", \"root-py\"]\n\n\
         [[package]]\nname = \"lib-a\"\nversion = \"0.1.0\"\nsource = { editable = \"py/lib-a\" }\n\n\
         [[package]]\nname = \"lib-b\"\nversion = \"0.1.0\"\nsource = { editable = \"py/lib-b\" }\n\n\
         [[package]]\nname = \"root-py\"\nversion = \"0.1.0\"\nsource = { virtual = \".\" }\n",
    );

    assert_incomplete(
        root,
        MonorepoStandard::UvWorkspace,
        "uv.lock",
        PackageProvenance::Globbed,
    );
}

/// An unparseable membership glob is dropped by the expander, which reports
/// the set incomplete: the lockfile recording exactly the members the other
/// pattern found is not a `match`.
#[test]
fn a_dropped_workspace_glob_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    pnpm_workspace(root, Some(PNPM_MATCHING));
    write(
        root,
        "pnpm-workspace.yaml",
        "packages:\n  - \"packages/*\"\n  - \"tools/[\"\n",
    );

    assert_incomplete(
        root,
        MonorepoStandard::PnpmWorkspaces,
        "pnpm-lock.yaml",
        PackageProvenance::Globbed,
    );
}

/// Cargo's subset check respects the same signal: a member pattern outside
/// Cargo's glob subset, or a literal member that does not exist (Cargo
/// rejects both), never reports `members_present`.
#[test]
fn an_unresolved_cargo_member_pattern_is_incomplete_manifest_discovery() {
    let lockfile = "version = 3\n\n[[package]]\nname = \"alpha\"\nversion = \"0.1.0\"\n\n\
                    [[package]]\nname = \"beta\"\nversion = \"0.1.0\"\n";
    for members in [
        r#"["crates/*", "tools/{a,b}"]"#,
        r#"["crates/*", "crates/missing"]"#,
    ] {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        cargo_workspace(root);
        write(
            root,
            "Cargo.toml",
            &format!("[workspace]\nmembers = {members}\nresolver = \"2\"\n"),
        );
        write(root, "Cargo.lock", lockfile);

        assert_incomplete(
            root,
            MonorepoStandard::CargoWorkspace,
            "Cargo.lock",
            PackageProvenance::Globbed,
        );
    }
}

/// Patterns that legitimately resolve to nothing are not incompleteness: a
/// glob matching no directory, and, for the Node tools that expand every
/// entry as a glob, a literal member path that does not exist.
#[test]
fn patterns_matching_nothing_still_match_the_lockfile() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    pnpm_workspace(root, Some(PNPM_MATCHING));
    write(
        root,
        "pnpm-workspace.yaml",
        "packages:\n  - \"packages/*\"\n  - \"tools/*\"\n  - \"apps/missing\"\n",
    );

    let (repo, _) = detect(root, &corroborating());
    let layer = layer_at(&repo, MonorepoStandard::PnpmWorkspaces, root);
    assert_eq!(layer.lockfile, matched("pnpm-lock.yaml"));
    assert_eq!(layer.provenance, PackageProvenance::Lockfile);
}

/// Rush requires every declared project folder to exist, so a missing one
/// leaves the member set incomplete even when the lockfile records exactly
/// the projects that do exist.
#[test]
fn a_missing_rush_project_folder_is_incomplete_manifest_discovery() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    write(
        root,
        "rush.json",
        r#"{ "pnpmVersion": "9.15.9", "projects": [
            { "packageName": "web", "projectFolder": "apps/web" },
            { "packageName": "gone", "projectFolder": "apps/gone" }
        ] }"#,
    );
    write(root, "apps/web/package.json", r#"{"name":"web","version":"1.0.0"}"#);
    write(
        root,
        "common/config/rush/pnpm-config.json",
        r#"{ "useWorkspaces": true }"#,
    );
    write(
        root,
        "common/config/rush/pnpm-lock.yaml",
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  ../../apps/web: {}\n",
    );

    assert_incomplete(
        root,
        MonorepoStandard::RushStack,
        "common/config/rush/pnpm-lock.yaml",
        PackageProvenance::Explicit,
    );

    // The control: once the folder exists, the same lockfile is a mismatch
    // naming it, so the missing folder was the only thing blocking comparison.
    write(root, "apps/gone/package.json", r#"{"name":"gone","version":"1.0.0"}"#);
    let (repo, _) = detect(root, &corroborating());
    assert_eq!(
        layer_at(&repo, MonorepoStandard::RushStack, root).lockfile,
        observation(
            LockfileStatus::Mismatch,
            &["common/config/rush/pnpm-lock.yaml"],
            None,
            &[],
            &["apps/gone"]
        )
    );
}

/// A Rush pnpm workspace whose lockfile records both members, plus one
/// importer key that no member path can have: an absolute path, a drive
/// path, or a name containing NUL. Only the synthetic `.` importer is
/// Rush's own; the invalid key reaches the shared validator, so the layer is
/// `invalid_member_path` rather than a `match` over the valid keys. A decoy
/// copy of the lockfile at `common/temp`, where `rush install` places one,
/// is never read in its place.
#[test]
fn an_invalid_rush_importer_is_an_invalid_member_path() {
    const LOCKFILE: &str = "common/config/rush/pnpm-lock.yaml";
    const VALID: &str =
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  ../../apps/web: {}\n  ../../apps/ui: {}\n";
    for (label, invalid) in [
        ("absolute", "'/opt/elsewhere/pkg': {}"),
        ("drive", "'C:/elsewhere/pkg': {}"),
        ("nul", "\"../../apps/a\\0b\": {}"),
    ] {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        write(
            root,
            "rush.json",
            r#"{ "pnpmVersion": "9.15.9", "projects": [
                { "packageName": "web", "projectFolder": "apps/web" },
                { "packageName": "ui", "projectFolder": "apps/ui" }
            ] }"#,
        );
        for name in ["web", "ui"] {
            write(
                root,
                &format!("apps/{name}/package.json"),
                &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
            );
        }
        write(
            root,
            "common/config/rush/pnpm-config.json",
            r#"{ "useWorkspaces": true }"#,
        );
        write(root, LOCKFILE, &format!("{VALID}  {invalid}\n"));
        write(root, "common/temp/pnpm-lock.yaml", VALID);

        let (repo, counts) = detect(root, &corroborating());

        let layer = layer_at(&repo, MonorepoStandard::RushStack, root);
        assert_eq!(
            layer.lockfile,
            observation(
                LockfileStatus::Unverifiable,
                &[LOCKFILE],
                Some(LockfileReason::InvalidMemberPath),
                &[],
                &[]
            ),
            "{label}"
        );
        assert_eq!(layer.provenance, PackageProvenance::Explicit, "{label}");
        for (relative, standard, provenance) in package_provenance(&repo) {
            if standard == MonorepoStandard::RushStack {
                assert_eq!(provenance, PackageProvenance::Explicit, "{label}: {relative}");
            }
        }
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{label}: {counts:?}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 1, "{label}: {counts:?}");

        // The control: without the invalid key the same lockfile matches,
        // so that key was the only thing blocking the upgrade.
        write(root, LOCKFILE, VALID);
        let (repo, _) = detect(root, &corroborating());
        let layer = layer_at(&repo, MonorepoStandard::RushStack, root);
        assert_eq!(layer.lockfile, matched(LOCKFILE), "{label} control");
        assert_eq!(layer.provenance, PackageProvenance::Lockfile, "{label} control");
    }
}

// ---------------------------------------------------------------------------
// uv membership records that are absent or incomplete
// ---------------------------------------------------------------------------

/// A uv workspace at `root` whose `py/*` members are `members`; with none,
/// the discovered member set is empty once the root is excluded. `lockfile`
/// is written as its `uv.lock`.
fn uv_workspace(root: &Path, members: &[&str], lockfile: &str) {
    write(
        root,
        "pyproject.toml",
        "[project]\nname = \"root-py\"\nversion = \"0.1.0\"\n\n\
         [tool.uv.workspace]\nmembers = [\"py/*\"]\n",
    );
    for name in members {
        write(
            root,
            &format!("py/{name}/pyproject.toml"),
            &format!("[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        );
    }
    write(root, "uv.lock", lockfile);
}

const UV_ROOT_PACKAGE: &str =
    "[[package]]\nname = \"root-py\"\nversion = \"0.1.0\"\nsource = { virtual = \".\" }\n";
const UV_LIB_A_PACKAGE: &str =
    "[[package]]\nname = \"lib-a\"\nversion = \"0.1.0\"\nsource = { editable = \"py/lib-a\" }\n";

/// Asserts the uv layer at `root` reports `expected` under a corroborating
/// structure request and a full request, with `provenance` on the layer and
/// on every uv package.
fn assert_uv_layer(root: &Path, expected: &LockfileObservation, provenance: PackageProvenance) {
    for (label, request) in [("structure", corroborating()), ("full", RepoRequest::full())] {
        let (repo, _) = detect(root, &request);
        let layer = layer_at(&repo, MonorepoStandard::UvWorkspace, root);
        assert_eq!(&layer.lockfile, expected, "{label}: {layer:?}");
        assert_eq!(layer.provenance, provenance, "{label}");
        for (relative, standard, package_provenance) in package_provenance(&repo) {
            if standard == MonorepoStandard::UvWorkspace {
                assert_eq!(package_provenance, provenance, "{label}: {relative}");
            }
        }
    }
}

/// A `[manifest]` table without `members` is an invalid required membership
/// field, not an empty member list: with no discovered members an empty list
/// would be a false `match`, and with one it would be a `mismatch`.
#[test]
fn a_uv_manifest_without_members_is_a_parse_failure() {
    for members in [&[][..], &["lib-a"][..]] {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        uv_workspace(
            root,
            members,
            &format!(
                "version = 1\nrevision = 3\n\n[manifest]\n\
                 constraints = [{{ name = \"idna\", specifier = \"<4\" }}]\n\n\
                 {UV_ROOT_PACKAGE}\n{UV_LIB_A_PACKAGE}"
            ),
        );

        assert_uv_layer(
            root,
            &observation(
                LockfileStatus::Unreadable,
                &["uv.lock"],
                Some(LockfileReason::ParseFailed),
                &[],
                &[],
            ),
            PackageProvenance::Globbed,
        );
    }
}

/// Without `[manifest]`, a local package other than the root means the
/// membership record is gone rather than root-only: never a false `match`
/// against an empty discovered set, nor a `mismatch` against a real one.
#[test]
fn an_absent_uv_manifest_beside_other_local_packages_has_no_membership_data() {
    for members in [&[][..], &["lib-a"][..]] {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        uv_workspace(
            root,
            members,
            &format!("version = 1\nrevision = 3\n\n{UV_ROOT_PACKAGE}\n{UV_LIB_A_PACKAGE}"),
        );

        assert_uv_layer(
            root,
            &observation(
                LockfileStatus::Unverifiable,
                &["uv.lock"],
                Some(LockfileReason::NoMembershipData),
                &[],
                &[],
            ),
            PackageProvenance::Globbed,
        );
    }
}

/// The root-only shape uv writes (no `[manifest]`, the root as the sole
/// local package) establishes the empty set, so it matches an empty
/// discovered set.
#[test]
fn an_absent_uv_manifest_with_only_the_root_package_matches_an_empty_member_set() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    uv_workspace(
        root,
        &[],
        &format!("version = 1\nrevision = 3\n\n{UV_ROOT_PACKAGE}"),
    );

    assert_uv_layer(root, &matched("uv.lock"), PackageProvenance::Lockfile);
}
