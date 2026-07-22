use super::*;

#[test]
fn loop_file_functions_reuse_request_home_magic_and_package_roots() {
    let request = tempfile::tempdir().unwrap();
    let source_path = request.path().join("prompt.md");
    let home = request.path().join("home");
    let magic = request.path().join("magic");
    let package = request.path().join("package");
    for dir in [&home, &magic, &package] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(home.join("home.flag"), "ready").unwrap();
    std::fs::write(magic.join("magic.flag"), "ready").unwrap();
    std::fs::write(package.join("package.flag"), "ready").unwrap();
    let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
        request.path(),
        Some(home),
        std::collections::HashMap::new(),
    )
    .with_repository_root(request.path())
    .with_package_area(package)
    .add_magic_path(magic, biscuit_file::PathPosition::Start);
    let fm = map(json!({}));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient)
        .with_base_dir(Some(request.path()))
        .with_file_resolution_context(Some(&snapshot), &source_path);

    for expression in [
        "file_exists('~/home.flag')",
        "file_exists('@magic.flag')",
        "file_exists('!package.flag')",
    ] {
        assert!(
            evaluate_condition(&LoopCondition::While(expression.into()), &lookup).unwrap(),
            "{expression} must use the request snapshot"
        );
    }
}
