//! Format checks that need Darkmatter's request capture: the repository
//! and package-area catalog `build_resolution_context` discovers.

use simplified_schema::format::resolve_file_reference_in_context;

#[test]
fn file_format_package_reference_prefers_package_area_over_repository() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let root = std::fs::canonicalize(dir.path()).unwrap();
    gix::init(&root).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"darkmatter/lib\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    let package_area = root.join("darkmatter");
    let member = package_area.join("lib");
    std::fs::create_dir_all(member.join("src")).unwrap();
    std::fs::create_dir_all(member.join("docs")).unwrap();
    std::fs::write(
        member.join("Cargo.toml"),
        "[package]\nname = \"fixture-darkmatter\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(member.join("src/lib.rs"), "").unwrap();
    std::fs::write(root.join("shared.md"), "repository decoy").unwrap();
    let package_target = package_area.join("shared.md");
    std::fs::write(&package_target, "package").unwrap();

    let base = member.join("docs");
    let context = crate::markdown::compose::build_resolution_context(
        &crate::markdown::compose::RequestSnapshot::new(&base),
    )
    .unwrap();
    assert_eq!(
        std::fs::canonicalize(
            resolve_file_reference_in_context(
                "^shared.md",
                Some(&base),
                None,
                &context,
            )
            .unwrap(),
        )
        .unwrap(),
        std::fs::canonicalize(package_target).unwrap(),
    );
}
