//! Partial generation through the public entry points.
//!
//! A partial run regenerates the complete module groups of the selected APIs,
//! keeps every other group of the existing output byte-for-byte, and refuses
//! an output it cannot account for before changing anything. These tests use
//! a small fixture registry, generate it into temporary directories, and
//! compare file lists and bytes before and after each run. The 18 real APIs
//! get the same partial run inside the fresh-output guard, which also
//! compiles the result (`fresh_output_compile`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_test_harness::bin_exe;
use schematic_define::{DefinitionContext, Endpoint, RestApi};
use schematic_gen::diagnostics::Output;
use schematic_gen::errors::GeneratorError;
use schematic_gen::output::{
    GENERATED_FILE_HEADER, RustOutputPlan, RustOutputRequest, Selection, prepare_rust_output,
};
use schematic_gen::pipeline::{BatchScope, GenerateOpts, OpenApiFormat, generate_batch};

use crate::common::fresh_output::{TreeSnapshot, diff_summary, snapshot_tree};

fn endpoint(id: &str, path: &str) -> Endpoint {
    let mut context = DefinitionContext::types_in("::fixture");
    Endpoint::get(id, path).describes(format!("{id} fixture endpoint"))
        .returns_json::<serde_json::Value>(&mut context).build().unwrap()
}

fn api(name: &str, module: &str, suffix: Option<&str>, endpoints: Vec<Endpoint>) -> RestApi {
    let mut context = DefinitionContext::types_in("::fixture");
    context.register::<serde_json::Value>().unwrap();
    let builder = RestApi::builder(&context, name, "https://fixture.example/v1")
        .describes(format!("{name} fixture API")).no_auth().module_path(module).endpoints(endpoints);
    if let Some(suffix) = suffix { builder.request_suffix(suffix).build().unwrap() }
    else { builder.build().unwrap() }
}

/// Four APIs in three module groups: `alpha` holds two APIs, like Ollama.
fn registry() -> Vec<RestApi> {
    vec![
        api("AlphaOne", "alpha", Some("OneRequest"), vec![endpoint("ListItems", "/one/items")]),
        api("AlphaTwo", "alpha", Some("TwoRequest"), vec![endpoint("ListItems", "/two/items")]),
        api("Beta", "beta", None, vec![endpoint("GetStatus", "/status")]),
        api("Gamma", "gamma", None, vec![endpoint("ListThings", "/things")]),
    ]
}

/// A single API whose module exceeds the split threshold, so it is written
/// as a directory.
fn large_api() -> RestApi {
    let endpoints = (0..150)
        .map(|i| endpoint(&format!("Endpoint{i}"), &format!("/endpoint/{i}/{{id}}")))
        .collect();
    api("Large", "large", None, endpoints)
}

fn refs(apis: &[RestApi]) -> Vec<&RestApi> {
    apis.iter().collect()
}

fn prepare(
    registry: &[&RestApi],
    selection: Selection<'_>,
    root: &Path,
) -> Result<RustOutputPlan, GeneratorError> {
    prepare_rust_output(&RustOutputRequest {
        registry,
        registrations: schematic_definitions::registry::registrations(),
        selection,
        output_dir: root,
        manifest_dir: None,
    })
}

fn generate(registry: &[&RestApi], selection: Selection<'_>, root: &Path) -> RustOutputPlan {
    let prepared = prepare(registry, selection, root).expect("prepare");
    prepared.plan.publish().expect("publish");
    prepared
}

fn partial(registry: &[&RestApi], name: &str, root: &Path) -> Result<RustOutputPlan, GeneratorError> {
    let selected: Vec<&RestApi> = registry.iter().copied().filter(|a| a.name == name).collect();
    prepare(registry, Selection::Apis(&selected), root)
}

fn append(path: &Path, text: &str) {
    let mut bytes = fs::read(path).unwrap();
    bytes.extend_from_slice(text.as_bytes());
    fs::write(path, bytes).unwrap();
}

fn owned(text: &str) -> String {
    format!("{GENERATED_FILE_HEADER}\n\n{text}")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A full generation of the fixture registry, copied for each case.
struct Pristine {
    _temp: tempfile::TempDir,
    root: PathBuf,
    tree: TreeSnapshot,
}

impl Pristine {
    fn new(registry: &[&RestApi]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("pristine");
        generate(registry, Selection::All, &root);
        let tree = snapshot_tree(&root);
        Self { _temp: temp, root, tree }
    }

    /// A fresh copy of the full output under `parent`.
    fn copy_into(&self, parent: &Path) -> PathBuf {
        let root = parent.join("src");
        copy_tree(&self.root, &root);
        root
    }
}

#[test]
fn partial_run_regenerates_the_whole_group_and_keeps_every_other_module() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());

    // Edits that keep the ownership header show which files the run rewrote.
    append(&root.join("alpha.rs"), "// edited alpha\n");
    append(&root.join("beta.rs"), "// edited beta\n");
    let beta_edited = fs::read(root.join("beta.rs")).unwrap();

    // Selecting one member regenerates both members of `alpha`.
    let prepared = generate(&registry, Selection::Apis(&[registry[1]]), &root);
    assert_eq!(prepared.regenerated, ["alpha"]);
    assert_eq!(prepared.retained, ["beta", "gamma"]);

    assert_eq!(fs::read(root.join("beta.rs")).unwrap(), beta_edited, "retained module untouched");
    let mut after = snapshot_tree(&root);
    after.insert("beta.rs".into(), pristine.tree["beta.rs"].clone());
    assert_eq!(
        diff_summary(&pristine.tree, &after),
        Vec::<String>::new(),
        "alpha.rs, lib.rs, and prelude.rs equal a full generation"
    );
    let prelude = fs::read_to_string(root.join("prelude.rs")).unwrap();
    for name in ["AlphaOne", "AlphaTwo", "Beta", "Gamma"] {
        assert!(prelude.contains(&format!("::{name};")), "prelude exports {name}");
    }

    // A second partial run is idempotent.
    fs::write(root.join("beta.rs"), pristine.tree["beta.rs"].as_ref().unwrap()).unwrap();
    generate(&registry, Selection::Apis(&[registry[0]]), &root);
    assert_eq!(diff_summary(&pristine.tree, &snapshot_tree(&root)), Vec::<String>::new());
}

#[test]
fn absent_or_empty_root_gets_only_the_selected_groups() {
    let apis = registry();
    let registry = refs(&apis);
    let temp = tempfile::tempdir().unwrap();
    for root in [temp.path().join("absent/src"), temp.path().join("empty")] {
        if root.ends_with("empty") {
            fs::create_dir(&root).unwrap();
        }
        let prepared = generate(&registry, Selection::Apis(&[registry[2]]), &root);
        assert_eq!(prepared.regenerated, ["beta"]);
        assert!(prepared.retained.is_empty());
        assert!(root.join("beta.rs").is_file());
        assert!(!root.join("alpha.rs").exists() && !root.join("gamma.rs").exists());
        let lib = fs::read_to_string(root.join("lib.rs")).unwrap();
        assert!(lib.contains("pub mod beta;") && !lib.contains("pub mod alpha;"), "{lib}");
    }
}

#[test]
fn owned_stale_files_in_a_regenerated_group_go_and_hand_written_content_stays() {
    let apis = vec![large_api(), api("Beta", "beta", None, vec![endpoint("GetStatus", "/status")])];
    let registry = refs(&apis);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("src");
    generate(&registry, Selection::All, &root);
    assert!(root.join("large/mod.rs").is_file(), "the large module is split into a directory");

    fs::write(root.join("large/stale.rs"), owned("// stale")).unwrap();
    fs::write(root.join("large/hand.rs"), "pub fn keep() {}\n").unwrap();
    fs::write(root.join("large/notes.md"), "notes\n").unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("bin/main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("handwritten.rs"), "pub fn keep() {}\n").unwrap();

    // Regenerating another group deletes nothing outside it.
    generate(&registry, Selection::Apis(&[registry[1]]), &root);
    assert!(root.join("large/stale.rs").exists(), "outside the regenerated group");

    // Regenerating the large group removes only its owned stale file.
    generate(&registry, Selection::Apis(&[registry[0]]), &root);
    assert!(!root.join("large/stale.rs").exists(), "owned stale file deleted");
    for kept in ["large/hand.rs", "large/notes.md", "bin/main.rs", "handwritten.rs"] {
        assert!(root.join(kept).exists(), "{kept} must survive");
    }
}

/// Runs a partial generation of `AlphaOne` against a copy of the full output
/// after `damage`, and asserts the refusal names `needle` and changes nothing.
fn assert_refused(
    damage: impl FnOnce(&Path),
    needle: &str,
    expect: fn(&GeneratorError) -> bool,
) {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    damage(&root);
    let before = snapshot_tree(&root);

    let err = partial(&registry, "AlphaOne", &root).expect_err("the partial run must refuse");
    assert!(expect(&err), "unexpected error kind: {err:?}");
    let message = err.to_string();
    assert!(message.contains(needle), "message should name `{needle}`:\n{message}");
    assert!(message.contains("nothing was changed"), "{message}");
    assert_eq!(diff_summary(&before, &snapshot_tree(&root)), Vec::<String>::new());
}

fn unsafe_output(err: &GeneratorError) -> bool {
    matches!(err, GeneratorError::UnsafeOutput { .. })
}

fn requires_full(err: &GeneratorError) -> bool {
    matches!(err, GeneratorError::RequiresFullGeneration { .. })
}

/// One test per refusal case, so the cases spread across test threads.
macro_rules! refusal_tests {
    ($($name:ident: $damage:expr, $needle:expr, $kind:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_refused($damage, $needle, $kind);
            }
        )*
    };
}

refusal_tests! {
    refuses_corrupt_lib_rs:
        |root: &Path| append(&root.join("lib.rs"), "pub mod {"),
        "lib.rs does not parse", unsafe_output;
    refuses_hand_edited_lib_rs:
        |root: &Path| fs::write(root.join("lib.rs"), "pub mod beta;\n").unwrap(),
        "lib.rs was not written by schematic-gen", unsafe_output;
    refuses_missing_lib_rs:
        |root: &Path| fs::remove_file(root.join("lib.rs")).unwrap(),
        "lib.rs is missing", unsafe_output;
    refuses_missing_member:
        |root: &Path| fs::remove_file(root.join("beta.rs")).unwrap(),
        "neither beta.rs nor beta/mod.rs", unsafe_output;
    refuses_undeclared_owned_module:
        |root: &Path| fs::write(root.join("delta.rs"), owned("pub struct Delta;")).unwrap(),
        "`delta` is not declared in lib.rs", unsafe_output;
    refuses_unknown_declared_module:
        |root: &Path| {
            append(&root.join("lib.rs"), "pub mod delta;\n");
            fs::write(root.join("delta.rs"), owned("pub struct Delta;")).unwrap();
        },
        "no current definition generates", unsafe_output;
    refuses_export_mismatch:
        |root: &Path| {
            let prelude = fs::read_to_string(root.join("prelude.rs")).unwrap();
            let edited = prelude.replace("gamma::Gamma;", "gamma::{Gamma, GammaClient};");
            assert_ne!(prelude, edited);
            fs::write(root.join("prelude.rs"), edited).unwrap();
        },
        "prelude.rs exports [\"Gamma\", \"GammaClient\"] from `gamma`", unsafe_output;
    refuses_conflicting_layouts:
        |root: &Path| {
            fs::create_dir(root.join("beta")).unwrap();
            fs::write(root.join("beta/mod.rs"), owned("")).unwrap();
        },
        "exists both as beta.rs and as beta/", unsafe_output;
    refuses_unowned_shared_file:
        |root: &Path| fs::write(root.join("elevenlabs_ws.rs"), "// mine\n").unwrap(),
        "elevenlabs_ws.rs was not written by schematic-gen", unsafe_output;
    refuses_directory_at_a_planned_file:
        |root: &Path| {
            fs::remove_file(root.join("alpha.rs")).unwrap();
            fs::create_dir(root.join("alpha.rs")).unwrap();
        },
        "alpha.rs is a directory where a file is planned", unsafe_output;
    refuses_layout_conversion_with_unowned_content:
        |root: &Path| {
            fs::create_dir(root.join("alpha")).unwrap();
            fs::write(root.join("alpha/mod.rs"), "// mine\n").unwrap();
        },
        "it is left intact", unsafe_output;
    refuses_changed_shared_runtime:
        |root: &Path| append(&root.join("shared.rs"), "// older runtime\n"),
        "the shared runtime would change (shared.rs)", requires_full;
    refuses_changed_websocket_runtime:
        |root: &Path| append(&root.join("ws_shared.rs"), "// older runtime\n"),
        "the shared runtime would change (ws_shared.rs)", requires_full;
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_never_followed() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();

    // A retained module replaced by a link to an owned file is not evidence.
    let root = pristine.copy_into(&temp.path().join("retained"));
    let elsewhere = temp.path().join("beta.rs");
    fs::rename(root.join("beta.rs"), &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, root.join("beta.rs")).unwrap();
    let before = snapshot_tree(&root);
    let err = partial(&registry, "AlphaOne", &root).unwrap_err();
    assert!(err.to_string().contains("symbolic link"), "{err}");
    assert_eq!(diff_summary(&before, &snapshot_tree(&root)), Vec::<String>::new());

    // A link at a planned path is never written through.
    let root = pristine.copy_into(&temp.path().join("planned"));
    let target = temp.path().join("outside.rs");
    fs::write(&target, owned("// outside")).unwrap();
    fs::remove_file(root.join("alpha.rs")).unwrap();
    std::os::unix::fs::symlink(&target, root.join("alpha.rs")).unwrap();
    let err = partial(&registry, "AlphaOne", &root).unwrap_err();
    assert!(err.to_string().contains("symbolic link"), "{err}");
    assert_eq!(fs::read_to_string(&target).unwrap(), owned("// outside"));
}

#[test]
fn full_generation_recovers_after_a_compatibility_refusal() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    append(&root.join("shared.rs"), "// older runtime\n");

    let err = partial(&registry, "Beta", &root).unwrap_err();
    assert!(requires_full(&err), "{err:?}");
    generate(&registry, Selection::All, &root);
    assert_eq!(diff_summary(&pristine.tree, &snapshot_tree(&root)), Vec::<String>::new());
}

/// The marker item exactly as full generation renders it into `lib.rs`.
fn abi_marker_item(value: &str) -> String {
    format!("pub const SCHEMATIC_RUNTIME_ABI: u32 = {value};")
}

/// Rewrites a copy's `lib.rs` with `edit`, asserting the edit changed it.
fn edit_lib(root: &Path, edit: impl FnOnce(&str) -> String) {
    let lib = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(lib.contains(&current_marker()), "{lib}");
    let edited = edit(&lib);
    assert_ne!(lib, edited, "the edit must change lib.rs");
    fs::write(root.join("lib.rs"), edited).unwrap();
}

/// Removes the marker, its doc line, and the compile-time assertion that
/// names it, so `lib.rs` reads like output from before the runtime crate.
fn without_marker(lib: &str) -> String {
    let marker = current_marker();
    let mut out = String::new();
    let mut in_assert = false;
    for line in lib.split_inclusive('\n') {
        let trimmed = line.trim();
        if in_assert {
            in_assert = !trimmed.ends_with(");");
            continue;
        }
        if trimmed.starts_with("const _: () = assert!(") {
            in_assert = !trimmed.ends_with(");");
            continue;
        }
        if trimmed == marker || trimmed == "/// The `schematic-runtime` ABI this crate was generated against." {
            continue;
        }
        out.push_str(line);
    }
    assert!(!out.contains("SCHEMATIC_RUNTIME_ABI"), "{out}");
    out
}

fn current_marker() -> String {
    abi_marker_item(&schematic_gen::RUNTIME_ABI.to_string())
}

/// Runs a partial generation of `AlphaOne` against a copy of the full output
/// whose `lib.rs` was changed by `edit`, and asserts a full-generation refusal
/// whose only change is the `lib.rs` marker described by `needle`, with the
/// tree left byte-for-byte as it was.
fn assert_abi_refused(edit: impl FnOnce(&str) -> String, needle: &str) {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    edit_lib(&root, edit);
    let before = snapshot_tree(&root);

    let err = partial(&registry, "AlphaOne", &root).expect_err("the partial run must refuse");
    let GeneratorError::RequiresFullGeneration { changed, retained, .. } = &err else {
        panic!("expected RequiresFullGeneration, got {err:?}");
    };
    assert_eq!(changed.len(), 1, "only the marker changes: {changed:?}");
    assert!(changed[0].starts_with("lib.rs"), "{changed:?}");
    assert!(changed[0].contains(needle), "`{needle}` in {changed:?}");
    assert_eq!(retained, &["beta", "gamma"]);
    let message = err.to_string();
    assert!(message.contains("nothing was changed") && message.contains("generate --api all"), "{message}");
    assert_eq!(diff_summary(&before, &snapshot_tree(&root)), Vec::<String>::new());
}

#[test]
fn abi_marker_control_unedited_output_allows_a_partial_run() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    let lib = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert_eq!(lib.matches(&current_marker()).count(), 1, "{lib}");

    let prepared = partial(&registry, "AlphaOne", &root).expect("the unedited output is accepted");
    assert_eq!(prepared.retained, ["beta", "gamma"], "modules are retained, so the marker was checked");
    prepared.plan.publish().expect("publish");
    assert_eq!(diff_summary(&pristine.tree, &snapshot_tree(&root)), Vec::<String>::new());
}

/// One test per marker case, so the cases spread across test threads.
macro_rules! abi_refusal_tests {
    ($($name:ident: $edit:expr, $needle:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_abi_refused($edit, &$needle);
            }
        )*
    };
}

abi_refusal_tests! {
    refuses_differing_abi_marker:
        |lib: &str| lib.replace(
            &current_marker(),
            &abi_marker_item(&(schematic_gen::RUNTIME_ABI + 1).to_string()),
        ),
        format!(
            "records schematic-runtime ABI {}, this generator emits ABI {}",
            schematic_gen::RUNTIME_ABI + 1,
            schematic_gen::RUNTIME_ABI
        );
    // A group generated before endpoint methods (ABI 2) has none, so a
    // partial run must not keep it beside regenerated groups.
    refuses_modules_generated_against_abi_2:
        |lib: &str| lib.replace(&current_marker(), &abi_marker_item("2")),
        format!(
            "records schematic-runtime ABI 2, this generator emits ABI {}",
            schematic_gen::RUNTIME_ABI
        );
    refuses_missing_legacy_abi_marker:
        without_marker,
        "records no schematic-runtime ABI";
    refuses_abi_marker_that_is_not_an_integer_literal:
        |lib: &str| lib.replace(&current_marker(), &abi_marker_item(&format!("{} + 0", schematic_gen::RUNTIME_ABI))),
        "is not an integer literal";
    refuses_abi_marker_of_the_wrong_type:
        |lib: &str| lib.replace(&current_marker(), &current_marker().replace(": u32", ": u64")),
        "is not a `u32`";
    refuses_conflicting_abi_marker_declarations:
        |lib: &str| lib.replace(&current_marker(), &format!("{}\n{}", current_marker(), current_marker())),
        "is declared 2 times";
}

#[test]
fn matching_abi_marker_does_not_bypass_the_export_check() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    let lib = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert_eq!(lib.matches(&current_marker()).count(), 1, "the marker matches: {lib}");
    let prelude = fs::read_to_string(root.join("prelude.rs")).unwrap();
    let edited = prelude.replace("gamma::Gamma;", "gamma::{Gamma, GammaExtra};");
    assert_ne!(prelude, edited);
    fs::write(root.join("prelude.rs"), edited).unwrap();
    let before = snapshot_tree(&root);

    let err = partial(&registry, "AlphaOne", &root).expect_err("the partial run must refuse");
    assert!(unsafe_output(&err), "unexpected error kind: {err:?}");
    let message = err.to_string();
    assert!(message.contains("GammaExtra") && message.contains("`gamma`"), "{message}");
    assert!(message.contains("nothing was changed"), "{message}");
    assert_eq!(diff_summary(&before, &snapshot_tree(&root)), Vec::<String>::new());
}

#[test]
fn missing_abi_marker_is_irrelevant_when_nothing_is_retained() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);

    // Every module group selected: the legacy lib.rs is fully rewritten.
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    edit_lib(&root, without_marker);
    let every_group = [registry[0], registry[2], registry[3]];
    let prepared = partial_selection(&registry, &every_group, &root);
    assert!(prepared.retained.is_empty(), "{:?}", prepared.retained);
    prepared.plan.publish().expect("publish");
    assert_eq!(diff_summary(&pristine.tree, &snapshot_tree(&root)), Vec::<String>::new());

    // An empty output directory has no lib.rs and retains nothing.
    let empty = temp.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let prepared = partial_selection(&registry, &[registry[2]], &empty);
    assert!(prepared.retained.is_empty(), "{:?}", prepared.retained);
    prepared.plan.publish().expect("publish");
    let lib = fs::read_to_string(empty.join("lib.rs")).unwrap();
    assert!(lib.contains(&current_marker()), "{lib}");
}

fn partial_selection(registry: &[&RestApi], selected: &[&RestApi], root: &Path) -> RustOutputPlan {
    prepare(registry, Selection::Apis(selected), root).expect("a run that retains nothing is accepted")
}

#[test]
fn unowned_manifest_is_never_overwritten() {
    let apis = registry();
    let registry = refs(&apis);
    let temp = tempfile::tempdir().unwrap();
    let crate_dir = temp.path().join("schema");
    fs::create_dir_all(&crate_dir).unwrap();
    fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"mine\"\n").unwrap();

    let err = prepare_rust_output(&RustOutputRequest {
        registry: &registry,
        registrations: schematic_definitions::registry::registrations(),
        selection: Selection::All,
        output_dir: &crate_dir.join("src"),
        manifest_dir: Some(&crate_dir),
    })
    .unwrap_err();
    assert!(err.to_string().contains("Cargo.toml was not written by schematic-gen"), "{err}");
    assert!(!crate_dir.join("src").exists(), "nothing was written");
}

#[test]
fn failure_during_publication_lists_completed_and_pending_mutations() {
    let apis = registry();
    let registry = refs(&apis);
    let pristine = Pristine::new(&registry);

    // A failure on a shared file: rerunning the partial run completes.
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    append(&root.join("alpha.rs"), "// edited\n");
    let lib_before = fs::read(root.join("lib.rs")).unwrap();
    let prepared = partial(&registry, "AlphaOne", &root).unwrap();
    // Injected after the checks: a directory where a file is planned makes
    // the rename fail on every OS.
    fs::remove_file(root.join("elevenlabs_ws.rs")).unwrap();
    fs::create_dir(root.join("elevenlabs_ws.rs")).unwrap();
    let err = prepared.plan.publish().unwrap_err();
    let GeneratorError::PublicationFailed { failed, completed, pending, .. } = &err else {
        panic!("expected PublicationFailed, got {err:?}");
    };
    assert!(failed.ends_with("elevenlabs_ws.rs") && failed.starts_with("write "), "{failed}");
    assert!(completed.iter().any(|m| m.ends_with("alpha.rs")), "{completed:?}");
    assert!(pending.iter().any(|m| m.ends_with("lib.rs")), "{pending:?}");
    assert_eq!(fs::read(root.join("lib.rs")).unwrap(), lib_before, "lib.rs is written last");
    assert_eq!(fs::read(root.join("alpha.rs")).unwrap(), pristine.tree["alpha.rs"].clone().unwrap());

    fs::remove_dir(root.join("elevenlabs_ws.rs")).unwrap();
    generate(&registry, Selection::Apis(&[registry[0]]), &root);
    assert_eq!(diff_summary(&pristine.tree, &snapshot_tree(&root)), Vec::<String>::new());

    // A failure on prelude.rs: the retry refuses without changing anything,
    // and full generation repairs the output.
    let temp = tempfile::tempdir().unwrap();
    let root = pristine.copy_into(temp.path());
    let prepared = partial(&registry, "AlphaOne", &root).unwrap();
    fs::remove_file(root.join("prelude.rs")).unwrap();
    fs::create_dir(root.join("prelude.rs")).unwrap();
    assert!(prepared.plan.publish().is_err());
    fs::remove_dir(root.join("prelude.rs")).unwrap();
    let before = snapshot_tree(&root);
    let err = partial(&registry, "AlphaOne", &root).unwrap_err();
    assert!(err.to_string().contains("prelude.rs is missing"), "{err}");
    assert_eq!(diff_summary(&before, &snapshot_tree(&root)), Vec::<String>::new());
    generate(&registry, Selection::All, &root);
    assert_eq!(diff_summary(&pristine.tree, &snapshot_tree(&root)), Vec::<String>::new());
}

/// Output roots under one temporary directory.
struct Roots {
    _temp: tempfile::TempDir,
    rust: String,
    openapi: String,
    postman: String,
}

impl Roots {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let path = |relative: &str| temp.path().join(relative).to_str().unwrap().to_string();
        Self {
            rust: path("schema/src"),
            openapi: path("openapi"),
            postman: path("postman"),
            _temp: temp,
        }
    }

    fn opts(&self, dry_run: bool) -> GenerateOpts<'_> {
        GenerateOpts {
            output: &self.rust,
            dry_run,
            verbose: 0,
            openapi_out: Some(&self.openapi),
            openapi_format: OpenApiFormat::Json,
            openapi_version: None,
            postman_out: Some(&self.postman),
            no_openapi: false,
            no_postman: false,
        }
    }
}

fn selection(name: &str) -> BatchScope {
    BatchScope::Selection(vec![name.to_string()])
}

#[test]
fn exports_rewrite_only_the_selected_groups() {
    let apis = registry();
    let roots = Roots::new();
    let report = generate_batch(&apis, &BatchScope::Complete, &roots.opts(false));
    assert!(report.failed().is_empty(), "{:?}", report.outputs);
    let openapi = Path::new(&roots.openapi);
    let postman = Path::new(&roots.postman);
    let full_openapi = snapshot_tree(openapi);
    let full_postman = snapshot_tree(postman);
    assert_eq!(
        full_openapi.keys().collect::<Vec<_>>(),
        ["alpha.json", "beta.json", "gamma.json"]
    );

    fs::write(openapi.join("alpha.json"), "{}").unwrap();
    fs::write(openapi.join("beta.json"), "{}").unwrap();
    fs::write(postman.join("beta.postman_collection.json"), "{}").unwrap();
    fs::write(openapi.join("old.json"), "{}").unwrap();
    fs::write(openapi.join("README.md"), "notes").unwrap();

    let report = generate_batch(&apis, &selection("AlphaTwo"), &roots.opts(false));
    assert!(report.failed().is_empty(), "{:?}", report.outputs);
    assert_eq!(fs::read(openapi.join("alpha.json")).unwrap(), full_openapi["alpha.json"].clone().unwrap());
    assert_eq!(fs::read_to_string(openapi.join("beta.json")).unwrap(), "{}", "unselected kept");
    assert_eq!(fs::read_to_string(postman.join("beta.postman_collection.json")).unwrap(), "{}");
    assert!(openapi.join("old.json").exists(), "a partial run deletes nothing");

    // A complete run removes the stale artifact but not unrelated files.
    let report = generate_batch(&apis, &BatchScope::Complete, &roots.opts(false));
    assert!(report.failed().is_empty(), "{:?}", report.outputs);
    assert!(!openapi.join("old.json").exists());
    assert!(openapi.join("README.md").exists());
    let mut after = snapshot_tree(openapi);
    after.remove("README.md");
    assert_eq!(diff_summary(&full_openapi, &after), Vec::<String>::new());
    assert_eq!(diff_summary(&full_postman, &snapshot_tree(postman)), Vec::<String>::new());
}

#[test]
fn each_root_establishes_its_own_safety() {
    let apis = registry();
    let roots = Roots::new();
    fs::create_dir_all(Path::new(&roots.openapi).join("alpha.json")).unwrap();
    let openapi_before = snapshot_tree(Path::new(&roots.openapi));

    let report = generate_batch(&apis, &selection("AlphaOne"), &roots.opts(false));
    assert_eq!(report.failed(), vec![Output::OpenApi]);
    assert_eq!(report.succeeded(), vec![Output::Rust, Output::Postman]);
    let err = report.get(Output::OpenApi).unwrap().result.as_ref().unwrap_err();
    assert!(err.to_string().contains("alpha.json is a directory"), "{err}");
    assert_eq!(snapshot_tree(Path::new(&roots.openapi)), openapi_before);
    assert!(Path::new(&roots.rust).join("alpha.rs").is_file());
    assert!(!Path::new(&roots.rust).join("beta.rs").exists());
    assert!(Path::new(&roots.postman).join("alpha.postman_collection.json").is_file());
    assert!(!Path::new(&roots.postman).join("beta.postman_collection.json").exists());
}

#[test]
fn dry_run_plans_every_root_and_creates_nothing() {
    let apis = registry();
    let roots = Roots::new();
    for scope in [BatchScope::Complete, selection("Gamma")] {
        let report = generate_batch(&apis, &scope, &roots.opts(true));
        assert!(report.failed().is_empty(), "{:?}", report.outputs);
        for root in [&roots.rust, &roots.openapi, &roots.postman] {
            let schema_dir = Path::new(root).parent().unwrap();
            assert!(!Path::new(root).exists(), "{root} must not be created");
            assert!(
                !schema_dir.join("schema").exists() && !schema_dir.join("Cargo.toml").exists(),
                "no directory or manifest is created"
            );
        }
    }

    // The dry run reports the plan publication would apply.
    let registry = refs(&apis);
    let root = Path::new(&roots.rust);
    let prepared = partial(&registry, "Gamma", root).unwrap();
    let written: Vec<_> = prepared
        .plan
        .mutations()
        .iter()
        .map(|m| m.path().strip_prefix(root).unwrap().to_path_buf())
        .collect();
    assert_eq!(written.first().unwrap(), Path::new("gamma.rs"));
    assert_eq!(written.last().unwrap(), Path::new("lib.rs"));
    assert!(!root.exists());
}

#[test]
fn cli_refuses_an_unsafe_partial_run_and_claims_no_update() {
    let temp = tempfile::tempdir().unwrap();
    let src = temp.path().join("schema/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "// hand-written crate\n").unwrap();
    let before = snapshot_tree(temp.path());

    let output = Command::new(bin_exe!("schematic-gen"))
        .args(["generate", "--api", "ollama-native", "--no-openapi", "--no-postman", "--output"])
        .arg(&src)
        .env("NO_COLOR", "1")
        .output()
        .expect("run schematic-gen");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(stderr.contains("refusing to update") && stderr.contains("lib.rs was not written"), "{stderr}");
    assert!(stderr.contains("generate --api all"), "{stderr}");
    assert!(!stdout.contains("[OK] Generated"), "no success claim:\n{stdout}");
    assert!(stdout.contains("[FAILED] rust output"), "{stdout}");
    assert_eq!(diff_summary(&before, &snapshot_tree(temp.path())), Vec::<String>::new());
}
