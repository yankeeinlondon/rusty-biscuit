use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_end_to_end_link_interpolation() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    gix::init(&repo).unwrap();

    let root_dir = repo.join("docs");
    let child_dir = repo.join("components");
    let assets = repo.join("assets");

    fs::create_dir_all(&root_dir).unwrap();
    fs::create_dir_all(&child_dir).unwrap();
    fs::create_dir_all(&assets).unwrap();

    let root_file = root_dir.join("root.md");
    let child_file = child_dir.join("child.md");
    let image_file = assets.join("image.png");

    fs::write(&image_file, "png").unwrap();
    // The child refers to an asset relative to its own directory
    fs::write(&child_file, "![child_img](../assets/image.png)").unwrap();
    // The root includes the child
    fs::write(&root_file, "# Root\n\n::file ../components/child.md\n").unwrap();

    let options = ComposeOptions::new().with_source_file(&root_file).only(&[
        ComposeOperation::LinkResolve,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::LinkNormalization,
    ]);

    let md = Markdown::try_from(root_file.as_path()).unwrap();
    let (composed, _) = md.compose_with(options).unwrap();
    let content = composed.content();

    // In the final composed document (which is anchored at root.md), the path to the asset
    // should be relative to root.md's location.
    // root is in docs/
    // asset is in assets/
    // So relative from root to asset is ../assets/image.png
    assert!(
        content.contains("../assets/image.png"),
        "Final content should have path relative to root, got: {}",
        content
    );
}

#[test]
fn test_home_dir_interpolation() {
    let home = dirs::home_dir().expect("Has home dir");
    let target = home.join("integration_test_home.txt");
    fs::write(&target, "home content").unwrap();
    let abs_target = std::fs::canonicalize(&target).unwrap();
    let content = format!(
        "[home]({})",
        abs_target
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/")
    );
    let md = Markdown::new(&content);
    let options = ComposeOptions::new().only(&[
        ComposeOperation::LinkResolve,
        ComposeOperation::LinkNormalization,
    ]);
    let (composed, _) = md.compose_with(options).unwrap();
    assert!(
        composed.content().contains("~/integration_test_home.txt"),
        "Content was: {}",
        composed.content()
    );
    fs::remove_file(&target).ok();
}

#[test]
fn test_env_var_interpolation() {
    let dir = tempdir().unwrap();
    let project_root = dir.path().join("project");
    fs::create_dir_all(&project_root).unwrap();
    let target = project_root.join("config.json");
    fs::write(&target, "{}").unwrap();
    let abs_target = std::fs::canonicalize(&target).unwrap();
    let abs_root = std::fs::canonicalize(&project_root).unwrap();
    let mut env = std::collections::HashMap::new();
    env.insert(
        "PROJECT_ROOT_INTEGRATION".to_string(),
        abs_root.to_string_lossy().into_owned(),
    );
    let snapshot = biscuit_file::FileResolutionContext::new(&project_root)
        .without_home_dir()
        .with_env(env);
    let content = format!(
        "[config]({})",
        abs_target
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/")
    );
    let md = Markdown::new(&content);
    let options = ComposeOptions::new()
        .with_env_path_whitelist(vec!["PROJECT_ROOT_INTEGRATION".to_string()])
        .with_file_resolution_context(snapshot)
        .only(&[
            ComposeOperation::LinkResolve,
            ComposeOperation::LinkNormalization,
        ]);
    let (composed, _) = md.compose_with(options).unwrap();
    assert!(
        composed
            .content()
            .contains("${PROJECT_ROOT_INTEGRATION}/config.json"),
        "Content was: {}",
        composed.content()
    );
}

#[test]
fn test_child_no_normalization() {
    let dir = tempdir().unwrap();
    let root_dir = dir.path().join("root_dir");
    let child_dir = dir.path().join("child_dir");
    fs::create_dir_all(&root_dir).unwrap();
    fs::create_dir_all(&child_dir).unwrap();

    let root_file = root_dir.join("root.md");
    let child_file = child_dir.join("child.md");
    let target_file = child_dir.join("target.txt");
    fs::write(&target_file, "target").unwrap();
    fs::write(&child_file, "[link](target.txt)").unwrap();
    fs::write(&root_file, "::file ../child_dir/child.md").unwrap();

    let options = ComposeOptions::new().with_source_file(&root_file).only(&[
        ComposeOperation::LinkResolve,
        ComposeOperation::BlockTransclusion,
    ]);
    let md = Markdown::try_from(root_file.as_path()).unwrap();
    let (composed, _) = md.compose_with(options).unwrap();

    let abs_path = std::fs::canonicalize(&target_file).unwrap();
    let abs_path_str = abs_path.to_string_lossy();
    #[cfg(windows)]
    let abs_path_str = abs_path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&abs_path_str)
        .replace('\\', "/");
    #[cfg(not(windows))]
    let abs_path_str = abs_path_str.into_owned();
    let abs_path_clean = if abs_path_str.starts_with("/private/") {
        &abs_path_str[8..]
    } else {
        &abs_path_str
    };

    assert!(
        composed.content().contains(abs_path_clean),
        "Link should be resolved to absolute path in child. Content was: {}",
        composed.content()
    );
}
