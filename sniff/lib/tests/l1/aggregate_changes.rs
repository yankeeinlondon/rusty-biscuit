//! Changed-path projections over a status collected the ordinary way: file
//! changes without patches, so `RepoStatus::dirty` / `untracked` stay empty.

use std::path::{Path, PathBuf};

use sniff::filesystem::FilesystemInfo;
use sniff::filesystem::blast_radius::ChangeScope;
use sniff::filesystem::repo::{area_changed_paths, attribute_paths, observed_scope_paths};
use sniff::request::{DetectionPlan, FilesystemRequest, GitRequest, RepoRequest};

/// A Cargo workspace whose root package (`root-app`) is itself a member beside
/// `area/member`, committed once, plus a non-package `area/member2` directory.
fn root_member_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let files = [
        (
            "Cargo.toml",
            "[package]\nname = \"root-app\"\nversion = \"0.1.0\"\n\n\
             [workspace]\nmembers = [\".\", \"area/member\"]\n",
        ),
        ("src/lib.rs", "pub fn original() {}\n"),
        (
            "area/member/Cargo.toml",
            "[package]\nname = \"member\"\nversion = \"0.1.0\"\n",
        ),
        ("area/member/src/lib.rs", "pub fn original() {}\n"),
    ];
    let repo = git2::Repository::init(root).unwrap();
    let mut index = repo.index().unwrap();
    for (relative, content) in files {
        let full = root.join(relative);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
        index.add_path(Path::new(relative)).unwrap();
    }
    index.write().unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.com").unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
        .unwrap();
    temp
}

/// Detect with the request the CLI's change-oriented commands make.
fn detect_ordinary(root: &Path) -> FilesystemInfo {
    sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(root.to_path_buf())
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(GitRequest::summary().include_file_changes(true))
                    .repo(RepoRequest::structure())
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
            ),
    )
    .unwrap()
    .filesystem
    .expect("filesystem facts")
}

#[test]
fn aggregate_changes_attribute_ordinary_status_to_owning_packages() {
    let temp = root_member_workspace();
    let root = temp.path();
    let repo = git2::Repository::open(root).unwrap();
    std::fs::write(root.join("area/member/src/lib.rs"), "pub fn staged() {}\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("area/member/src/lib.rs")).unwrap();
    index.write().unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn unstaged() {}\n").unwrap();
    std::fs::create_dir_all(root.join("area/member2")).unwrap();
    std::fs::write(root.join("area/member2/notes.rs"), "// untracked\n").unwrap();

    let fs = detect_ordinary(root);
    let git = fs.git.as_ref().expect("git facts");
    let status = git.status.as_ref().expect("status was requested");
    assert!(status.is_dirty);
    assert!(
        status.dirty.is_empty() && status.untracked.is_empty(),
        "an ordinary request must not collect patch detail: {status:?}"
    );
    let repo_info = fs.repo.as_ref().expect("repo facts");
    let packages = repo_info.packages.as_deref().expect("package catalog");

    let dirty = observed_scope_paths(git, ChangeScope::Dirty).expect("observed");
    let mut sorted = dirty.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        [
            PathBuf::from("area/member/src/lib.rs"),
            PathBuf::from("area/member2/notes.rs"),
            PathBuf::from("src/lib.rs"),
        ]
    );
    let attribution = attribute_paths(packages, &dirty);
    assert_eq!(attribution.packages, ["member", "root-app"]);
    assert_eq!(attribution.package_areas, ["", "area"]);

    let staged = observed_scope_paths(git, ChangeScope::Staged).expect("observed");
    assert_eq!(attribute_paths(packages, &staged).packages, ["member"]);

    // An area is a directory: `area/member2` lies inside `area` even though
    // no package there owns it.
    let mut in_area = area_changed_paths(repo_info, "area", git).expect("observed");
    in_area.sort();
    assert_eq!(
        in_area,
        [
            Path::new("area/member/src/lib.rs"),
            Path::new("area/member2/notes.rs")
        ]
    );
    let in_root_area = area_changed_paths(repo_info, "", git).expect("observed");
    assert_eq!(in_root_area, [Path::new("src/lib.rs")]);
}

#[test]
fn aggregate_changes_are_indeterminate_without_observed_file_changes() {
    let temp = root_member_workspace();
    let root = temp.path();
    std::fs::write(root.join("src/lib.rs"), "pub fn unstaged() {}\n").unwrap();

    let summary_only = sniff::detect_with_plan(
        DetectionPlan::new()
            .base_dir(root.to_path_buf())
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(GitRequest::summary())
                    .repo(RepoRequest::structure())
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
            ),
    )
    .unwrap()
    .filesystem
    .unwrap();
    let git = summary_only.git.as_ref().unwrap();
    assert!(git.status.as_ref().unwrap().is_dirty);
    assert_eq!(observed_scope_paths(git, ChangeScope::Dirty), None);
    let repo_info = summary_only.repo.as_ref().unwrap();
    assert_eq!(area_changed_paths(repo_info, "", git), None);

    let mut identity_only = git.clone();
    identity_only.status = None;
    assert_eq!(observed_scope_paths(&identity_only, ChangeScope::Dirty), None);

    // A clean tree is a determinate, empty answer.
    std::fs::write(root.join("src/lib.rs"), "pub fn original() {}\n").unwrap();
    let clean = detect_ordinary(root);
    let git = clean.git.as_ref().unwrap();
    assert_eq!(observed_scope_paths(git, ChangeScope::Dirty), Some(Vec::new()));
}
