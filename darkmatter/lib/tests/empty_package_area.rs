//! Sniff stores the repository-root package area as `""` (no `"root"`
//! sentinel). Composition must keep resolving scope correctly from that data,
//! including when a top-level package precedes an area in Sniff's package
//! list: an empty area has no directory, so it must never claim the paths
//! beneath the repository root.

use std::collections::HashMap;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, ContextCaptureEvidence, ContextRequirements,
};

const BODY: &str = "area=[{{ ctx.area }}]\n\ncpa=[{{ ctx.current_package_area }}]\n\ncp=[{{ ctx.current_package }}]\n\nroot=[{{ ctx.package_area_root }}]\n";

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A Cargo workspace whose top-level package `aaa` sorts before area `zeta`.
fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"aaa\", \"zeta/lib\"]\n",
    );
    for (relative, name) in [("aaa", "aaa"), ("zeta/lib", "zeta-lib")] {
        write(
            &root.join(relative).join("Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        );
        write(&root.join(relative).join("src/lib.rs"), "");
    }
    dir
}

/// Compose `BODY` from `dir` with Sniff's own repository observation supplied
/// as evidence, returning the rendered `name=[value]` pairs.
fn scope_at(root: &Path, dir: &Path) -> HashMap<String, String> {
    let repo = sniff::filesystem::repo::detect_repo(root)
        .unwrap()
        .expect("fixture is a repository");
    let packages = repo.packages.as_deref().unwrap();
    assert_eq!(packages[0].name, "aaa", "the top-level package is listed first");
    assert_eq!(packages[0].package_area, "", "Sniff stores the empty area");

    let document = dir.join("scope.md");
    write(&document, BODY);
    let evidence = ContextCaptureEvidence::new(HashMap::new())
        .with_git(None)
        .with_repository(Some(root.to_path_buf()), Some(repo));
    let context = ComposeContext::capture_with_evidence(
        dir,
        &ContextRequirements::for_content(BODY),
        &evidence,
    );
    let (composed, _) = Markdown::try_from(document.as_path())
        .unwrap()
        .compose_with(ComposeOptions::new_with_context(context).with_source_file(&document))
        .unwrap();
    composed
        .content()
        .lines()
        .filter_map(|line| {
            let (name, value) = line.trim().split_once("=[")?;
            Some((name.to_string(), value.strip_suffix(']')?.to_string()))
        })
        .collect()
}

#[test]
fn an_area_directory_resolves_to_its_area_not_the_empty_top_level_area() {
    let fixture = workspace();
    let root = fixture.path();
    let scope = scope_at(root, &root.join("zeta"));

    assert_eq!(scope["area"], "zeta");
    assert_eq!(scope["cpa"], "zeta");
    assert_eq!(scope["cp"], "");
    assert!(scope["root"].ends_with("/zeta"), "{scope:?}");
}

#[test]
fn a_top_level_package_projects_the_empty_package_area() {
    let fixture = workspace();
    let root = fixture.path();
    let scope = scope_at(root, &root.join("aaa/src"));

    assert_eq!(scope["area"], "aaa");
    assert_eq!(scope["cpa"], "");
    assert_eq!(scope["cp"], "aaa");
}

#[test]
fn the_monorepo_root_has_no_area() {
    let fixture = workspace();
    let root = fixture.path();
    let scope = scope_at(root, root);

    assert_eq!(scope["area"], "");
    assert_eq!(scope["cpa"], "");
    assert_eq!(scope["cp"], "");
    assert_eq!(scope["root"], "", "no area root at the monorepo root");
}

#[test]
fn a_package_inside_an_area_keeps_its_area() {
    let fixture = workspace();
    let root = fixture.path();
    let scope = scope_at(root, &root.join("zeta/lib/src"));

    assert_eq!(scope["area"], "zeta-lib");
    assert_eq!(scope["cpa"], "zeta");
    assert_eq!(scope["cp"], "zeta-lib");
}
