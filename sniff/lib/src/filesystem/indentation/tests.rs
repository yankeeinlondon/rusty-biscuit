//! Work-count tests: these need the crate-private collector, so they live
//! beside the module. Behavior is tested through the public API in
//! `tests/l1/indentation.rs`.

use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::performance::counters;
use crate::performance::testing::measure;

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// A worktree root holding shared manifests that several rows read.
fn shared_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".editorconfig", "[*]\nindent_style = space\n");
    write(root, "package.json", r#"{"name": "x", "prettier": {"tabWidth": 3}}"#);
    write(root, "deno.json", r#"{"fmt": {"indentWidth": 6}}"#);
    write(root, "pyproject.toml", "[tool.ruff]\nindent-width = 2\n[tool.tombi.format.rules]\nindent-width = 8\n");
    write(root, "rustfmt.toml", "tab_spaces = 2\n");
    dir
}

#[test]
fn a_snapshot_reads_and_parses_each_existing_file_once_however_many_rows_share_it() {
    let dir = shared_fixture();
    let (snapshot, load) = measure(|| RepoIndentation::load(dir.path()).unwrap());
    assert_eq!(load.get(counters::INDENT_CONFIG_PARSES), 5, "{:?}", load.all());
    assert_eq!(load.get(counters::FS_FILE_OPENS), 5, "{:?}", load.all());

    // Every query after the load reads nothing.
    let (summary, queries) = measure(|| {
        let ts = snapshot.resolve_for_extension("ts");
        let json = snapshot.resolve_for_extension("json");
        let py = snapshot.resolve_for_extension("py");
        let toml = snapshot.resolve_for_extension("toml");
        let path = snapshot.resolve_for_path(&snapshot.root().join("src/lib.rs")).unwrap();
        (ts, json, py, toml, path, snapshot.summary())
    });
    assert_eq!(queries.get(counters::FS_FILE_OPENS), 0, "{:?}", queries.all());
    assert_eq!(queries.get(counters::INDENT_CONFIG_PARSES), 0);

    let (ts, json, py, toml, path, _) = summary;
    // `package.json` and `deno.json` feed both the `ts` and `json` answers.
    for resolved in [&ts, &json] {
        let sources: Vec<_> = resolved.alt_authorities.iter().map(|opinion| opinion.source).collect();
        assert_eq!(sources, [IndentAuthority::Prettier, IndentAuthority::Deno]);
    }
    // `pyproject.toml` feeds Ruff for `py` and Tombi for `toml`.
    assert_eq!(py.alt_authorities[0].source, IndentAuthority::Ruff);
    assert_eq!(py.indent.get(), 2);
    assert_eq!(toml.alt_authorities[0].source, IndentAuthority::Tombi);
    assert_eq!(toml.indent.get(), 8);
    assert_eq!(path.indent.get(), 2);
}

#[test]
fn a_single_lookup_reads_only_the_files_its_type_can_use() {
    let dir = shared_fixture();
    let (inputs, work) = measure(|| Inputs::load(dir.path(), inputs::Selection::Target(Some("rs"))));
    // `.editorconfig` and `rustfmt.toml`; the JS and Python manifests are
    // never opened.
    assert_eq!(work.get(counters::FS_FILE_OPENS), 2, "{:?}", work.all());
    // Probes: `.editorconfig` and rustfmt's two candidates.
    assert_eq!(work.get(counters::FS_METADATA_PROBES), 3, "{:?}", work.all());
    assert!(matches!(inputs.get("package.json"), inputs::Loaded::Absent));

    let (_, generic) = measure(|| Inputs::load(dir.path(), inputs::Selection::Target(None)));
    assert_eq!(generic.get(counters::FS_FILE_OPENS), 1, "only `.editorconfig`: {:?}", generic.all());
}

#[test]
fn a_shared_manifest_is_parsed_once_for_one_target_that_reads_it_through_several_rows() {
    let dir = shared_fixture();
    // `ts` reads `package.json` (Prettier) and `deno.json` (Deno), and probes
    // the rest of the JS rows' candidates.
    let (resolved, work) = measure(|| resolve(
        Some(&Inputs::load(dir.path(), inputs::Selection::Target(Some("ts")))),
        &Target::extension("ts"),
    ));
    assert_eq!(work.get(counters::INDENT_CONFIG_PARSES), 3, "{:?}", work.all());
    assert_eq!(resolved.alt_authorities.len(), 2);
}
