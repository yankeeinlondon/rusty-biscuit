use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::{Scratch, cache_entries, json_of, stdout_of};

fn assert_sums(report: &Value) {
    let groups = report["groups"].as_array().expect("groups");
    for field in ["by_kind", "by_visibility", "by_language"] {
        let mut sums = BTreeMap::<String, u64>::new();
        for group in groups {
            for (key, value) in group["stats"][field].as_object().unwrap() {
                *sums.entry(key.clone()).or_default() += value.as_u64().unwrap();
            }
        }
        assert_eq!(serde_json::to_value(sums).unwrap(), report["stats"][field], "{field}: {report}");
    }
    for field in ["documented", "total", "unavailable"] {
        let sum: u64 = groups.iter().map(|g| g["stats"]["doc_coverage"][field].as_u64().unwrap()).sum();
        assert_eq!(sum, report["stats"]["doc_coverage"][field], "{field}: {report}");
    }
    let sum: u64 = groups.iter().map(|g| g["stats"]["synthetic_exports"].as_u64().unwrap()).sum();
    assert_eq!(sum, report["stats"]["synthetic_exports"]);
    for stats in std::iter::once(&report["stats"]).chain(groups.iter().map(|g| &g["stats"])) {
        let coverage = &stats["doc_coverage"];
        let total = coverage["total"].as_u64().unwrap();
        if total == 0 {
            assert!(coverage["ratio"].is_null());
        } else {
            assert_eq!(coverage["ratio"].as_f64().unwrap(), coverage["documented"].as_u64().unwrap() as f64 / total as f64);
        }
    }
}

fn report(scratch: &Scratch, args: &[&str]) -> Value {
    let report = json_of(&scratch.stdout(&[&["--json"][..], args].concat()));
    assert_sums(&report);
    assert_eq!(report["errors"], json!([]));
    report
}

fn package<'a>(report: &'a Value, path: &str) -> &'a Value {
    report["groups"].as_array().unwrap().iter().find(|g| g["package"]["path"] == path).unwrap_or_else(|| panic!("missing {path}: {report}"))
}

fn root(path: &Path) -> String {
    biscuit_file::to_portable_string(&biscuit_file::canonicalize_simplified(path).unwrap())
}

#[test]
fn cargo_membership_uses_deepest_declared_identity() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[package]\nname = 'root'\nversion = '0.1.0'\n[workspace]\nmembers = ['.', 'pkg-a', 'pkg-a/nested', 'pkg-a/unnamed', 'pkg-a2', 'unused']\n"),
        ("src/lib.rs", "pub fn root() {}\n"),
        ("pkg-a/Cargo.toml", "[package]\nname = 'same'\nversion = '0.1.0'\n"),
        ("pkg-a/src/lib.rs", "pub fn parent() {}\n"),
        ("pkg-a/nested/Cargo.toml", "[package]\nname = 'nested'\nversion = '0.1.0'\n"),
        ("pkg-a/nested/src/lib.rs", "pub fn nested() {}\n"),
        ("pkg-a/unnamed/Cargo.toml", "[package]\nversion = '0.1.0'\n"),
        ("pkg-a/unnamed/src/lib.rs", "pub fn unnamed() {}\n"),
        ("pkg-a2/Cargo.toml", "[package]\nname = 'same'\nversion = '0.1.0'\n"),
        ("pkg-a2/src/lib.rs", "pub fn sibling() {}\n"),
        ("unused/Cargo.toml", "[package]\nname = 'unused'\nversion = '0.1.0'\n"),
    ]);
    let output = report(&scratch, &["functions", "--stats"]);
    let identities: Vec<_> = output["groups"].as_array().unwrap().iter().map(|g| g["package"].clone()).collect();
    assert_eq!(identities, [json!({"name":"root", "path":"."}), json!({"name":"same", "path":"pkg-a"}), json!({"name":"nested", "path":"pkg-a/nested"}), json!({"name":"same", "path":"pkg-a2"}), Value::Null]);
    for group in output["groups"].as_array().unwrap() { assert_eq!(group["stats"]["by_kind"], json!({"Function":1})); }
    assert_eq!(output["stats"]["by_kind"], json!({"Function":5}));

    std::fs::write(scratch.repo.path().join("Cargo.toml"), "[workspace]\nmembers = ['pkg-a', 'pkg-a/nested', 'pkg-a/unnamed', 'pkg-a2', 'unused']\n").unwrap();
    let virtual_root = report(&scratch, &["functions", "--stats"]);
    assert_eq!(virtual_root["groups"].as_array().unwrap().len(), 4);
    assert_eq!(virtual_root["groups"][3]["package"], Value::Null);
    assert_eq!(virtual_root["groups"][3]["stats"]["by_kind"], json!({"Function":2}));
}

#[test]
fn shipped_area_manifests_remain_separate_packages() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ['tree-hugger/lib', 'tree-hugger/cli']\n"),
        ("tree-hugger/lib/Cargo.toml", include_str!("../../../lib/Cargo.toml")),
        ("tree-hugger/cli/Cargo.toml", include_str!("../../Cargo.toml")),
        ("tree-hugger/lib/src/lib.rs", "pub fn library() {}\n"),
        ("tree-hugger/cli/src/main.rs", "fn main() {}\n"),
    ]);
    let output = report(&scratch, &["functions", "--stats"]);
    assert_eq!(output["groups"].as_array().unwrap().len(), 2);
    for (path, name) in [("tree-hugger/lib", "tree-hugger"), ("tree-hugger/cli", "tree-hugger-cli")] {
        assert_eq!(package(&output, path)["package"]["name"], name);
        assert_eq!(package(&output, path)["stats"]["by_kind"], json!({"Function":1}));
    }
}

#[test]
fn npm_membership_and_standalone_without_git() {
    let scratch = Scratch::new(&[
        ("package.json", r#"{"private":true,"workspaces":["packages/*"]}"#),
        ("packages/a/package.json", r#"{"name":"@example/alpha","version":"1.0.0"}"#),
        ("packages/a/index.ts", "export function alpha() {}\n"),
        ("packages/b/package.json", r#"{"name":"beta","version":"1.0.0"}"#),
        ("packages/b/index.js", "export function beta() {}\n"),
        ("loose.rs", "pub fn loose() {}\n"),
    ]);
    let output = report(&scratch, &["functions", "--stats"]);
    assert_eq!(output["groups"].as_array().unwrap().len(), 3);
    assert_eq!(package(&output, "packages/a")["package"]["name"], "@example/alpha");
    assert_eq!(package(&output, "packages/a")["stats"]["by_language"], json!({"TypeScript":1}));
    assert_eq!(package(&output, "packages/b")["stats"]["by_language"], json!({"JavaScript":1}));
    assert!(output["groups"][2]["package"].is_null());

    for manifest in ["Cargo.toml", "package.json"] {
        let scratch = Scratch::new(&[("lib.rs", "pub fn selected() {}\n")]);
        let text = if manifest == "Cargo.toml" { "[package]\nname = 'standalone'\nversion = '0.1.0'\n" } else { r#"{"name":"standalone","version":"1.0.0"}"# };
        std::fs::write(scratch.repo.path().join(manifest), text).unwrap();
        let with_git = report(&scratch, &["functions", "--stats", "lib.rs"]);
        std::fs::remove_dir_all(scratch.repo.path().join(".git")).unwrap();
        let without_git = report(&scratch, &["functions", "--stats", "lib.rs"]);
        assert_eq!(with_git, without_git);
        assert_eq!(without_git["groups"][0]["package"], json!({"name":"standalone", "path":"."}));
    }
}

#[test]
fn linked_worktree_uses_its_own_catalog() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ['pkg']\n"),
        ("pkg/Cargo.toml", "[package]\nname = 'main-only'\nversion = '0.1.0'\n"),
        ("pkg/src/lib.rs", "pub fn selected() {}\n"),
    ]);
    let repo = git2::Repository::open(scratch.repo.path()).unwrap();
    let tree = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[]).unwrap();
    let linked_parent = tempfile::TempDir::new().unwrap();
    let linked = linked_parent.path().join("linked");
    repo.worktree("linked", &linked, None).unwrap();
    std::fs::create_dir_all(linked.join("pkg/src")).unwrap();
    std::fs::write(linked.join("Cargo.toml"), "[workspace]\nmembers = ['pkg']\n").unwrap();
    std::fs::write(linked.join("pkg/Cargo.toml"), "[package]\nname = 'linked-only'\nversion = '0.1.0'\n").unwrap();
    std::fs::write(linked.join("pkg/src/lib.rs"), "pub fn selected() {}\n").unwrap();
    let args = ["--json", "functions", "--stats"];
    let output = scratch.process(&args).current_dir(linked.join("pkg")).output().unwrap();
    let output = json_of(&stdout_of(output, &args));
    assert_sums(&output);
    assert_eq!(output["root_dir"], root(&linked.join("pkg")));
    assert_eq!(output["package_root_dir"], root(&linked));
    assert_eq!(output["groups"].as_array().unwrap().len(), 1);
    assert_eq!(output["groups"][0]["package"], json!({"name":"linked-only", "path":"pkg"}));
    assert_eq!(output["groups"][0]["stats"], output["stats"]);
    assert_eq!(output["errors"], json!([]));
}

#[test]
fn outside_file_stays_unassigned_even_with_broken_external_manifest() {
    let scratch = Scratch::new(&[("Cargo.toml", "[package]\nname = 'inside'\nversion = '0.1.0'\n"), ("lib.rs", "pub fn inside() {}\n")]);
    let outside = tempfile::TempDir::new().unwrap();
    std::fs::write(outside.path().join("Cargo.toml"), "[workspace]\nmembers = [").unwrap();
    std::fs::write(outside.path().join("outside.rs"), "pub fn outside() {}\n").unwrap();
    let file = outside.path().join("outside.rs");
    let output = report(&scratch, &["functions", "--stats", "lib.rs", "--file", file.to_str().unwrap()]);
    assert_eq!(output["package_root_dir"], root(scratch.repo.path()));
    assert_eq!(output["groups"].as_array().unwrap().len(), 2);
    assert_eq!(output["groups"][1]["package"], Value::Null);
    assert_eq!(output["groups"][1]["stats"]["by_kind"], json!({"Function":1}));
    assert_eq!(package(&output, ".")["stats"]["by_kind"], json!({"Function":1}));
}

#[cfg(unix)]
#[test]
fn aliases_count_once_in_the_target_package() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ['pkg']\n"),
        ("pkg/Cargo.toml", "[package]\nname = 'target'\nversion = '0.1.0'\n"),
        ("pkg/src/lib.rs", "pub fn selected() {}\n"),
    ]);
    std::os::unix::fs::symlink("pkg/src/lib.rs", scratch.repo.path().join("alias.rs")).unwrap();
    std::os::unix::fs::symlink("pkg", scratch.repo.path().join("alias-dir")).unwrap();
    let native = scratch.repo.path().join("pkg/src/lib.rs");
    let spellings = ["pkg/src/lib.rs", "pkg/src/../src/lib.rs", "alias.rs", "alias-dir/src/lib.rs", native.to_str().unwrap()];
    for inputs in spellings.iter().map(|s| vec![*s]).chain(std::iter::once(spellings.to_vec())) {
        let args = [&["functions", "--stats", "--language", "rust"][..], &inputs].concat();
        let output = report(&scratch, &args);
        assert_eq!(output["groups"].as_array().unwrap().len(), 1);
        assert_eq!(output["groups"][0]["package"], json!({"name":"target", "path":"pkg"}));
        assert_eq!(output["stats"]["by_kind"], json!({"Function":1}));
    }
}

#[cfg(windows)]
#[test]
fn windows_spellings_share_package_identity() {
    let scratch = Scratch::new(&[("Cargo.toml", "[package]\nname = 'native'\nversion = '0.1.0'\n"), ("src/lib.rs", "pub fn selected() {}\n")]);
    let verbatim = std::fs::canonicalize(scratch.repo.path().join("src/lib.rs")).unwrap();
    let native = biscuit_file::canonicalize_simplified(&verbatim).unwrap();
    let spellings = ["src/lib.rs", r"src\lib.rs", native.to_str().unwrap()];
    for inputs in spellings.iter().map(|s| vec![*s]).chain(std::iter::once(spellings.to_vec())) {
        let args = [&["functions", "--stats", "--language", "rust"][..], &inputs].concat();
        let output = report(&scratch, &args);
        assert_eq!(output["groups"].as_array().unwrap().len(), 1);
        assert_eq!(output["groups"][0]["package"], json!({"name":"native", "path":"."}));
        assert_eq!(output["stats"]["by_kind"], json!({"Function":1}));
    }
    // A verbatim cwd reaches native identity normalization without the `?`
    // passing through positional glob syntax.
    let cwd = std::fs::canonicalize(scratch.repo.path()).unwrap();
    let args = ["--json", "functions", "--stats", "--file", r"src\lib.rs"];
    let output = scratch.process(&args).current_dir(cwd).output().unwrap();
    let output = json_of(&stdout_of(output, &args));
    assert_sums(&output);
    assert_eq!(output["package_root_dir"], root(scratch.repo.path()));
    assert_eq!(output["groups"].as_array().unwrap().len(), 1);
    assert_eq!(output["groups"][0]["package"], json!({"name":"native", "path":"."}));
    assert_eq!(output["groups"][0]["stats"], output["stats"]);
    assert_eq!(output["stats"]["by_kind"], json!({"Function":1}));
    assert_eq!(output["errors"], json!([]));
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_package_paths_do_not_merge() {
    use std::os::unix::ffi::OsStringExt;
    let scratch = Scratch::new(&[("Cargo.toml", "[workspace]\nmembers = ['packages/*']\n")]);
    for byte in [0xfe, 0xff] {
        let dir = scratch.repo.path().join("packages").join(std::ffi::OsString::from_vec(vec![b'p', byte]));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = 'same'\nversion = '0.1.0'\n").unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn selected() {}\n").unwrap();
    }
    let output = report(&scratch, &["functions", "--stats"]);
    assert_eq!(output["groups"].as_array().unwrap().len(), 2);
    for group in output["groups"].as_array().unwrap() {
        assert_eq!(group["package"]["name"], "same");
        assert_eq!(group["stats"]["by_kind"], json!({"Function":1}));
    }
}

fn filtered_workspace() -> Scratch {
    Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ['a', 'b']\n"),
        ("a/Cargo.toml", "[package]\nname = 'a'\nversion = '0.1.0'\n"),
        ("b/Cargo.toml", "[package]\nname = 'b'\nversion = '0.1.0'\n"),
        ("a/src/lib.rs", "/// Docs.\npub async fn chosen(x: u32) -> u32 { x }\nfn private() {}\n#[deprecated]\npub fn old() {}\n#[cfg(test)]\nmod tests { #[test] fn check() {} }\npub struct Item { pub field: u8 }\nimpl Item { pub fn method(&self) {} }\n"),
        ("b/src/lib.rs", "pub fn chosen() {}\npub struct Other;\nimpl Other { pub fn method(&self) {} }\n"),
    ])
}

#[test]
fn selectors_preserve_group_sums_and_count() {
    let scratch = filtered_workspace();
    for filters in [vec![], vec!["--file", "a/**"], vec!["--name", "chosen"], vec!["--scope", "member"], vec!["--visibility", "public"], vec!["--tests-only"], vec!["--exclude-tests"], vec!["--undocumented"], vec!["--deprecated"], vec!["--async"], vec!["--returns", "u32"], vec!["--param-type", "u32"], vec!["--min-params", "1"]] {
        let args = [&["functions", "--stats"][..], &filters].concat();
        let output = report(&scratch, &args);
        let count_args = [&["functions", "--count"][..], &filters].concat();
        let count: u64 = scratch.stdout(&count_args).trim().parse().unwrap();
        assert!(count > 0, "load-bearing selector {filters:?}");
        let actual: u64 = output["stats"]["by_kind"].as_object().unwrap().values().map(|v| v.as_u64().unwrap()).sum();
        assert_eq!(actual, count, "{filters:?}");
        if filters.first() == Some(&"--file") {
            assert_eq!(output["groups"].as_array().unwrap().len(), 1);
        } else {
            assert_eq!(output["groups"].as_array().unwrap().len(), 2, "zero groups retained: {filters:?}");
        }
    }
}

#[test]
fn classes_count_containers_in_each_package() {
    let scratch = filtered_workspace();
    let output = report(&scratch, &["classes", "--stats"]);
    assert_eq!(scratch.stdout(&["classes", "--count"]), "2\n");
    assert_eq!(output["stats"]["by_kind"], json!({"Type":2}));
    for group in output["groups"].as_array().unwrap() { assert_eq!(group["stats"]["by_kind"], json!({"Type":1})); }
}

#[test]
fn failed_and_zero_match_packages_keep_zero_groups() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ['good', 'empty', 'failed', 'unknown']\n"),
        ("good/Cargo.toml", "[package]\nname = 'good'\nversion = '0.1.0'\n"),
        ("empty/Cargo.toml", "[package]\nname = 'empty'\nversion = '0.1.0'\n"),
        ("failed/Cargo.toml", "[package]\nname = 'failed'\nversion = '0.1.0'\n"),
        ("unknown/Cargo.toml", "[package]\nname = 'unknown'\nversion = '0.1.0'\n"),
        ("good/lib.rs", "/// Docs.\npub fn documented() {}\npub fn undocumented() {}\n"),
        ("empty/lib.rs", "pub struct Empty;\n"),
        ("failed/lib.rs", ""),
        ("unknown/run.sh", "run() { echo hello; }\n"),
    ]);
    std::fs::write(scratch.repo.path().join("failed/lib.rs"), b"fn ok() {}\n\xff\xfe\n").unwrap();
    for filters in [vec![], vec!["--name", "no_matching_declarations"]] {
        let args = [&["--json", "functions", "--stats"][..], &filters].concat();
        let output = scratch.hug(&args);
        assert_eq!(output.status.code(), Some(1));
        let output = json_of(std::str::from_utf8(&output.stdout).unwrap());
        assert_sums(&output);
        assert_eq!(output["groups"].as_array().unwrap().len(), 4);
        assert_eq!(output["errors"].as_array().unwrap().len(), 1);
        assert_eq!(output["errors"][0]["file"], "failed/lib.rs");
        for path in ["empty", "failed"] {
            let stats = &package(&output, path)["stats"];
            assert_eq!(stats["by_kind"], json!({}));
            assert_eq!(stats["doc_coverage"], json!({"documented":0,"total":0,"unavailable":0,"ratio":null}));
        }
        if filters.is_empty() {
            assert_eq!(output["stats"]["doc_coverage"]["ratio"], 0.5);
            assert_eq!(package(&output, "unknown")["stats"]["doc_coverage"], json!({"documented":0,"total":0,"unavailable":1,"ratio":null}));
        } else { assert_eq!(output["stats"]["by_kind"], json!({})); }
    }
}

#[test]
fn cross_package_prelude_counts_only_the_exporting_package() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[package]\nname = 'exporter'\nversion = '0.1.0'\n[workspace]\nmembers = ['.', 'src/target']\n"),
        ("src/lib.rs", "pub mod target;\npub mod prelude;\n"),
        ("src/target/Cargo.toml", "[package]\nname = 'target'\nversion = '0.1.0'\n"),
        ("src/target/mod.rs", "/// Target documentation.\npub struct Exported;\n"),
        ("src/prelude.rs", "pub use crate::target::Exported;\n"),
    ]);
    let declarations = json_of(&scratch.stdout(&["--json", "symbols", "--prelude", "src/lib.rs"]));
    let export = &declarations["files"][0]["symbols"][0];
    assert_eq!(export["source"]["file_path"], "src/target/mod.rs");
    assert_eq!(export["visibility"]["is_synthetic"], true);
    let output = report(&scratch, &["symbols", "--stats", "--prelude", "src/lib.rs"]);
    assert_eq!(output["groups"].as_array().unwrap().len(), 1);
    assert_eq!(output["groups"][0]["package"], json!({"name":"exporter", "path":"."}));
    assert_eq!(output["groups"][0]["stats"], output["stats"]);
    assert_eq!(output["stats"]["synthetic_exports"], 1);
    assert_eq!(output["stats"]["by_kind"], json!({"Type":1}));
    assert_eq!(output["stats"]["doc_coverage"], json!({"documented":0,"total":0,"unavailable":0,"ratio":null}));
}

#[test]
fn text_modes_match_json_sections_and_counts() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ['a', 'z']\n"),
        ("a/Cargo.toml", "[package]\nname = 'same'\nversion = '0.1.0'\n"),
        ("z/Cargo.toml", "[package]\nname = 'same'\nversion = '0.1.0'\n"),
        ("a/lib.rs", "/// Docs.\npub fn first() {}\npub fn second() {}\n"),
        ("z/lib.rs", "pub fn third() {}\n"),
    ]);
    let output = report(&scratch, &["functions", "--stats"]);
    assert_eq!(output["stats"]["doc_coverage"]["ratio"], 1.0 / 3.0);
    for mode in [vec![], vec!["--format", "plain"], vec!["--format", "table"]] {
        let args = [&["functions", "--stats"][..], &mode].concat();
        let text = scratch.stdout(&args);
        assert!(!text.contains('\u{1b}'));
        let headings = ["All selected declarations", "same (a)", "same (z)"];
        let positions: Vec<_> = headings.iter().map(|h| text.find(h).unwrap()).chain(std::iter::once(text.len())).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
        for (section, (count, documented, total)) in [(3, 1, 3), (2, 1, 2), (1, 0, 1)].iter().enumerate() {
            let section_text = &text[positions[section]..positions[section + 1]];
            for label in ["function", "public", "Rust"] {
                let row = section_text.lines().find(|l| l.contains(label)).unwrap();
                assert!(row.split(|c: char| !c.is_ascii_digit()).any(|n| n == count.to_string()), "{row}");
            }
            assert!(section_text.contains(&format!("Doc coverage: {documented} of {total} documented")), "{section_text}");
        }
    }
}

#[test]
fn manifest_renames_refresh_groups_on_actual_cache_hits() {
    let scratch = Scratch::new(&[("Cargo.toml", "[package]\nname = 'before'\nversion = '0.1.0'\n"), ("src/lib.rs", "/// Docs.\npub fn selected() {}\n")]);
    let args = ["--verbose", "--json", "functions", "--stats", "src/lib.rs"];
    let cold = scratch.hug(&args);
    assert!(String::from_utf8_lossy(&cold.stderr).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"));
    let cold = json_of(&stdout_of(cold, &args));
    let entries = cache_entries(scratch.temp.path());
    assert_eq!(entries.len(), 1);
    let entry = std::fs::read(&entries[0]).unwrap();
    for name in ["after", "another name", "before"] {
        std::fs::write(scratch.repo.path().join("Cargo.toml"), format!("[package]\nname = '{name}'\nversion = '0.1.0'\n")).unwrap();
        for _ in 0..2 {
            let warm = scratch.hug(&args);
            assert!(String::from_utf8_lossy(&warm.stderr).contains("1 hits, 0 partial hits, 0 misses; parsed 0 files, 0 definition captures, 0 diagnostics runs"), "{warm:?}");
            let warm = json_of(&stdout_of(warm, &args));
            assert_sums(&warm);
            assert_eq!(warm["groups"][0]["package"], json!({"name":name,"path":"."}));
            assert_eq!(warm["stats"], cold["stats"]);
            assert_eq!(warm["groups"][0]["stats"], cold["groups"][0]["stats"]);
            assert_eq!(warm["errors"], json!([]));
            assert_eq!(cache_entries(scratch.temp.path()), entries);
            assert_eq!(std::fs::read(&entries[0]).unwrap(), entry);
        }
    }
}
