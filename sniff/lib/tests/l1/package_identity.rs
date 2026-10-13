use std::path::Path;
use sniff::filesystem::repo::{Package, PackageNameSource, PackageOwnershipLookup, RepoInfo, detect_repo_structure_or_root_package};

const CARGO: &str = include_str!("../fixtures/lockfiles/cargo-1.52.0/workspace-v2/crates/alpha/Cargo.toml");
const NPM: &str = include_str!("../fixtures/lockfiles/pnpm-9.15.9/workspace/packages/alpha/package.json");
const PYTHON: &str = include_str!("../fixtures/lockfiles/uv-0.9.5/virtual-root/packages/alpha/pyproject.toml");
const COMPOSER: &str = include_str!("../fixtures/lockfiles/composer-2.10.3/single-project/composer.json");
const GO: &str = include_str!("../fixtures/lockfiles/go-1.27.1/workspace/packages/alpha/go.mod");

fn discover(filename: &str, content: &str) -> (tempfile::TempDir, RepoInfo) {
    let temp = tempfile::tempdir().unwrap();
    let (workspace, declaration) = match filename {
        "Cargo.toml" => ("Cargo.toml", "[workspace]\nmembers = [\"pkg\"]\n"),
        "package.json" => ("package.json", r#"{"private":true,"workspaces":["pkg"]}"#),
        "pyproject.toml" => ("pyproject.toml", "[tool.uv.workspace]\nmembers = [\"pkg\"]\n"),
        "go.mod" => ("go.work", "go 1.27.1\nuse ./pkg\n"),
        _ => {
            std::fs::write(temp.path().join(filename), content).unwrap();
            let repo = detect_repo_structure_or_root_package(temp.path()).unwrap().unwrap();
            return (temp, repo);
        }
    };
    std::fs::write(temp.path().join(workspace), declaration).unwrap();
    std::fs::create_dir(temp.path().join("pkg")).unwrap();
    std::fs::write(temp.path().join("pkg").join(filename), content).unwrap();
    let repo = detect_repo_structure_or_root_package(temp.path()).unwrap().unwrap();
    (temp, repo)
}

fn assert_identity(filename: &str, content: &str, expected: Option<&str>) {
    let (temp, repo) = discover(filename, content);
    let lookup = PackageOwnershipLookup::new(&repo);
    let package = lookup.package_for_path(&temp.path().join("pkg/src/lib.source")).unwrap();
    let source = if expected.is_some() { PackageNameSource::Manifest } else { PackageNameSource::PathFallback };
    assert_eq!(package.name_source, source, "{filename}: {content}");
    assert_eq!(package.name, expected.unwrap_or("pkg"), "{filename}: {content}");
    assert_eq!(package.relative, "pkg");
    assert_eq!(repo.package_for_dir(&temp.path().join("pkg")).unwrap().name, package.name);
    let value = serde_json::to_value(package).unwrap();
    if expected.is_some() { assert_eq!(value["name_source"], "manifest"); }
    else { assert!(value.get("name_source").is_none()); }
}

fn toml_matrix(filename: &str, control: &str, table: &str) {
    assert_identity(filename, control, Some("alpha"));
    for replacement in ["", "name = null", "name = 123", "name = [\"alpha\", 123]", "name = [123]", "name = []", "name = {}", "name = \"\"", "name = \"  \\t\"", "name = \"alpha\"\nname = \"other\""] {
        assert_identity(filename, &control.replace("name = \"alpha\"", replacement), None);
    }
    for replacement in [format!("[[{table}]]"), format!("{table} = [\"alpha\", 123]"), format!("{table} = [123]")] {
        assert_identity(filename, &control.replace(&format!("[{table}]"), &replacement), None);
    }
    assert_identity(filename, &format!("{control}\ngarbage ???"), None);
    assert_identity(filename, &control.replace("name = \"alpha\"", "name = 'alpha'"), Some("alpha"));
}

#[test]
fn cargo_name_matrix() { toml_matrix("Cargo.toml", CARGO, "package"); }
#[test]
fn python_name_matrix() { toml_matrix("pyproject.toml", PYTHON, "project"); }

#[test]
fn npm_name_matrix() {
    assert_identity("package.json", NPM, Some("alpha"));
    assert_identity("package.json", &NPM.replace("\"name\": \"alpha\",", ""), None);
    for replacement in ["null", "123", "[\"alpha\",123]", "[123]", "[]", "{}", "\"\"", "\"  \\t\""] {
        assert_identity("package.json", &NPM.replace("\"alpha\"", replacement), None);
    }
    // Preserve the generic JSON reader's documented last-wins behavior.
    assert_identity("package.json", &NPM.replace("\"name\": \"alpha\"", "\"name\": \"ignored\", \"name\": \"alpha\""), Some("alpha"));
    assert_identity("package.json", &NPM.replace("\"name\": \"alpha\"", "\"name\": \"alpha\", \"name\": null"), None);
    assert_identity("package.json", &format!("{NPM} garbage"), None);
    assert_identity("package.json", &NPM.replace("\"alpha\"", "\"a\\u006cpha\""), Some("alpha"));
}

#[test]
fn go_name_matrix() {
    let module_line = GO.lines().find(|line| line.starts_with("module ")).unwrap();
    let name = module_line.strip_prefix("module ").unwrap();
    assert_identity("go.mod", GO, Some(name));
    for replacement in ["", "module", "module ", "module \"\"", "module \"  \"", "module [alpha 123]", "module [123]", "module []", "module {}", "module alpha extra", "module alpha\nmodule other"] {
        assert_identity("go.mod", &GO.replace(module_line, replacement), None);
    }
    for suffix in ["garbage ???", "require (", "require (\ngolang.org/x/text v0.3.8", "module \"unterminated", "module \"bad\\q\""] {
        assert_identity("go.mod", &format!("{GO}\n{suffix}"), None);
    }
    assert_identity("go.mod", &format!("{GO}\nrequire (\nexample.com/other v1.0.0\n)\n"), Some(name));
    for representation in [format!("module \"{name}\""), format!("module `{name}`"), format!("module\t{name} // comment"), format!("module \"{}\"", name.replace("/", "\\x2f")), format!("module \"{}\"", name.replace("/", "\\057"))] {
        assert_identity("go.mod", &GO.replace(module_line, &representation), Some(name));
    }
    // go.mod has no typed null/number values: these are ordinary path tokens.
    for name in ["null", "123"] { assert_identity("go.mod", &GO.replace(module_line, &format!("module {name}")), Some(name)); }
}

#[test]
fn shipped_manifest_corpus() {
    for (filename, content, expected) in [("Cargo.toml", CARGO, "alpha"), ("package.json", NPM, "alpha"), ("pyproject.toml", PYTHON, "alpha"), ("go.mod", GO, "example.com/fixture/alpha")] {
        assert_identity(filename, content, Some(expected));
    }
    let (temp, repo) = discover("composer.json", COMPOSER);
    let owners = PackageOwnershipLookup::new(&repo);
    let composer = owners.package_for_path(&temp.path().join("source.php")).unwrap();
    assert_eq!(composer.name, "fixture/fixture-root");
    assert_eq!(composer.name_source, PackageNameSource::Manifest);
    for filename in ["requirements.txt", "composer.json"] {
        let (_temp, repo) = discover(filename, if filename.ends_with("json") { "{}" } else { "" });
        let package = &repo.packages.as_ref().unwrap()[0];
        assert_eq!(package.name_source, PackageNameSource::PathFallback);
        assert_eq!(package.relative, "");
        assert_eq!(package.name, "");
    }
}

#[test]
fn catalog_round_trip() {
    let (_temp, repo) = discover("Cargo.toml", CARGO);
    let mut value = serde_json::to_value(&repo).unwrap();
    value["packages"][0].as_object_mut().unwrap().remove("name_source");
    let legacy: RepoInfo = serde_json::from_value(value).unwrap();
    assert_eq!(legacy.packages.as_ref().unwrap()[0].name_source, PackageNameSource::PathFallback);
    for original in [repo, legacy] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("catalog.json");
        let mut current = original.clone();
        for _ in 0..2 {
            std::fs::write(&path, serde_json::to_vec(&current).unwrap()).unwrap();
            current = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let lookup = PackageOwnershipLookup::new(&current);
            let owner = lookup.package_for_path(&original.root.join("pkg/source.rs")).unwrap();
            let expected = &original.packages.as_ref().unwrap()[0];
            assert_eq!((owner.name.as_str(), owner.name_source), (expected.name.as_str(), expected.name_source));
        }
    }
    for source in [serde_json::json!("unknown"), serde_json::Value::Null, serde_json::json!(123), serde_json::json!(["manifest", 123]), serde_json::json!([123]), serde_json::json!([]), serde_json::json!({}), serde_json::json!(""), serde_json::json!("   ")] {
        let mut invalid = serde_json::to_value(Package::default()).unwrap();
        invalid["name_source"] = source;
        assert!(serde_json::from_value::<Package>(invalid).is_err());
    }
    let encoded = serde_json::to_string(&Package::default()).unwrap();
    let duplicate = encoded.replacen('{', r#"{"name_source":"manifest","name_source":"path_fallback","#, 1);
    assert!(serde_json::from_str::<Package>(&duplicate).is_err());
    assert!(serde_json::from_str::<Package>(&format!("{encoded} garbage")).is_err());
}

#[test]
fn ownership_boundaries() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let packages = [("", "root"), ("pkg-a", "a"), ("pkg-a/nested", ""), ("pkg-a2", "a2")].into_iter().map(|(relative, name)| {
        std::fs::create_dir_all(root.join(relative)).unwrap();
        Package { path: root.join(relative), relative: relative.into(), name: name.into(), name_source: if name.is_empty() { PackageNameSource::PathFallback } else { PackageNameSource::Manifest }, ..Package::default() }
    }).collect();
    let repo = RepoInfo { root: root.to_path_buf(), packages: Some(packages), ..RepoInfo::default() };
    let lookup = PackageOwnershipLookup::new(&repo);
    for (relative, expected) in [("source.rs", "root"), ("pkg-a/src/x.rs", "a"), ("pkg-a/nested/src/x.rs", ""), ("pkg-a2/src/x.rs", "a2"), ("pkg-a20/x.rs", "root")] {
        let path = root.join(relative);
        assert_eq!(lookup.package_for_path(&path).unwrap().name, expected);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
        assert_eq!(repo.package_for_dir(&path).unwrap().name, expected);
    }
    let outside = tempfile::tempdir().unwrap();
    assert!(lookup.package_for_path(&outside.path().join("x.rs")).is_none());
    assert!(lookup.package_for_path(Path::new("pkg-a/source.rs")).is_none());
    assert!(PackageOwnershipLookup::new(&RepoInfo::default()).package_for_path(root).is_none());
}

#[cfg(unix)]
#[test]
fn ownership_symlink_alias() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    let external = temp.path().join("external");
    std::fs::create_dir_all(root.join("pkg")).unwrap();
    std::fs::create_dir_all(&external).unwrap();
    std::os::unix::fs::symlink(root.join("pkg"), root.join("alias")).unwrap();
    std::os::unix::fs::symlink(&external, root.join("outside")).unwrap();
    let repo = RepoInfo { root: root.clone(), packages: Some(vec![Package { path: root.join("pkg"), relative: "pkg".into(), name: "alpha".into(), ..Package::default() }]), ..RepoInfo::default() };
    let lookup = PackageOwnershipLookup::new(&repo);
    assert_eq!(lookup.package_for_path(&root.join("alias/source.rs")).unwrap().name, "alpha");
    assert!(lookup.package_for_path(&root.join("outside/source.rs")).is_none());
}

#[test]
fn malformed_root_discovery_is_an_error() {
    for (filename, content) in [("Cargo.toml", format!("{CARGO} garbage ???")), ("package.json", format!("{NPM} garbage")), ("pyproject.toml", format!("{PYTHON} garbage ???"))] {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(filename), content).unwrap();
        assert!(detect_repo_structure_or_root_package(temp.path()).is_err(), "{filename}");
    }
}

#[test]
fn ownership_alias_precedence() {
    let (temp, mut repo) = discover("Cargo.toml", CARGO);
    let mut duplicate = repo.packages.as_ref().unwrap()[0].clone();
    duplicate.name = "second-alias".into();
    repo.packages.as_mut().unwrap().push(duplicate);
    let lookup = PackageOwnershipLookup::new(&repo);
    assert_eq!(lookup.package_for_path(&temp.path().join("pkg/source.rs")).unwrap().name, "alpha");
}

#[cfg(unix)]
#[test]
fn ownership_non_utf8_paths_remain_distinct() {
    use std::os::unix::ffi::OsStrExt;
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let mut repo = RepoInfo { root: root.clone(), packages: Some(Vec::new()), ..RepoInfo::default() };
    for (bytes, name) in [(b"pkg-\xff".as_slice(), "one"), (b"pkg-\xfe".as_slice(), "two")] {
        // APFS rejects these names; a captured native catalog can still carry them.
        let path = root.join(std::ffi::OsStr::from_bytes(bytes));
        repo.packages.as_mut().unwrap().push(Package { path, name: name.into(), ..Package::default() });
    }
    let lookup = PackageOwnershipLookup::new(&repo);
    for package in repo.packages.as_ref().unwrap() {
        assert_eq!(lookup.package_for_path(&package.path.join("source.rs")).unwrap().name, package.name);
    }
}

#[test]
fn manifest_name_priority_and_literal_values() {
    let (temp, repo) = discover("Cargo.toml", &CARGO.replace("name = \"alpha\"", "name = \"  α <x>  \""));
    assert_eq!(repo.packages.as_ref().unwrap()[0].name, "  α <x>  ");
    std::fs::write(temp.path().join("pkg/package.json"), NPM).unwrap();
    std::fs::write(temp.path().join("pkg/Cargo.toml"), CARGO.replace("name = \"alpha\"", "name = \"   \"")).unwrap();
    let repo = detect_repo_structure_or_root_package(temp.path()).unwrap().unwrap();
    let owners = PackageOwnershipLookup::new(&repo);
    let package = owners.package_for_path(&temp.path().join("pkg/source.rs")).unwrap();
    assert_eq!(package.name, "alpha");
    assert_eq!(package.name_source, PackageNameSource::Manifest);
}

#[test]
fn composer_name_matrix() {
    let control = COMPOSER;
    let original: serde_json::Value = serde_json::from_str(control).unwrap();
    let name = original["name"].as_str().unwrap();
    let token = format!("\"{name}\"");
    let mut cases = vec![(control.to_string(), Some(name))];
    let mut absent = original.clone();
    absent.as_object_mut().unwrap().remove("name");
    cases.push((absent.to_string(), None));
    for replacement in ["null", "123", "[\"alpha\",123]", "[123]", "[]", "{}", "\"\"", "\"   \""] {
        cases.push((control.replace(&token, replacement), None));
    }
    cases.push((format!("{control} garbage"), None));
    cases.push((control.replace(&format!("\"name\": {token}"), &format!("\"name\": \"ignored\", \"name\": {token}")), Some(name)));
    for (content, expected) in cases {
        let (temp, repo) = discover("composer.json", &content);
        let owners = PackageOwnershipLookup::new(&repo);
        let package = owners.package_for_path(&temp.path().join("source.php")).unwrap();
        assert_eq!(package.name, expected.unwrap_or(""), "{content}");
        assert_eq!(package.name_source, if expected.is_some() { PackageNameSource::Manifest } else { PackageNameSource::PathFallback }, "{content}");
        let value = serde_json::to_value(package).unwrap();
        assert_eq!(value.get("name_source").is_some(), expected.is_some());
    }
}
