//! The data runtime's observation log, through the expression functions that
//! record into it: every input-reading family observes what it read or
//! probed, a memo hit observes again, and a failure keeps its dependency.

use std::path::Path;

use serde_json::{Value, json};

use super::{DataSourceIdentity, Observation, ObservationKind};
use crate::markdown::compose::expression::ResolutionContext;
use crate::markdown::compose::expression::functions::call_function;

fn context(dir: &Path) -> ResolutionContext {
    ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir))
}

fn call(context: &ResolutionContext, name: &str, args: &[Value]) -> Result<Value, String> {
    call_function(name, args, Some(context)).expect("registered").map_err(|error| error.to_string())
}

fn entry(path: &Path) -> String {
    DataSourceIdentity::local(path).read_set_entry()
}

fn kinds_of(observations: &[Observation], path: &Path) -> Vec<ObservationKind> {
    let entry = entry(path);
    observations.iter().filter(|observation| observation.entry == entry).map(|observation| observation.kind).collect()
}

#[test]
fn a_memo_hit_observes_the_same_read_again() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("os.yaml");
    std::fs::write(&file, "- macOS\n- Linux\n").unwrap();
    let context = context(dir.path());
    let first_mark = context.data.read_mark();
    assert_eq!(call(&context, "load_data", &[json!("os.yaml"), json!("string[]")]).unwrap(), json!(["macOS", "Linux"]));
    let second_mark = context.data.read_mark();
    assert_eq!(call(&context, "load_data", &[json!("os.yaml"), json!("string[]")]).unwrap(), json!(["macOS", "Linux"]));
    assert_eq!(context.data.stats().source_hits, 1, "the second call is a memo hit");

    let expected = Observation {
        entry: entry(&file),
        kind: ObservationKind::File,
        hash: Some(biscuit_hash::blake3_hash_bytes(b"- macOS\n- Linux\n")),
    };
    let first = context.data.observations_since(first_mark);
    let second = context.data.observations_since(second_mark);
    assert!(first.contains(&expected), "{first:?}");
    assert_eq!(second, vec![expected], "the memo hit observes only the same read again");
}

#[test]
fn a_failed_read_keeps_its_dependency_without_a_hash() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken.yaml");
    std::fs::write(&file, "a: [\n").unwrap();
    let context = context(dir.path());
    let error = call(&context, "load_data", &[json!("broken.yaml"), json!("any[]")]).unwrap_err();
    assert!(error.contains("broken.yaml"), "{error}");
    let observed = context.data.observations_since(Default::default());
    assert!(observed.contains(&Observation { entry: entry(&file), kind: ObservationKind::File, hash: None }), "{observed:?}");
    assert!(context.data.read_set().contains(&entry(&file)));
}

#[test]
fn a_missing_data_file_records_the_candidates_it_probed() {
    let dir = tempfile::tempdir().unwrap();
    let context = context(dir.path());
    assert!(call(&context, "load_data", &[json!("./absent.yaml"), json!("string[]")]).is_err());
    let absent = dir.path().join("absent.yaml");
    assert_eq!(kinds_of(&context.data.observations_since(Default::default()), &absent), [ObservationKind::MissingProbe]);
    assert!(context.data.read_set().contains(&entry(&absent)), "{:?}", context.data.read_set());
}

#[test]
fn a_missing_contract_import_is_a_dependency() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.yaml"), "- a\n").unwrap();
    let context = context(dir.path());
    assert!(call(&context, "load_data", &[json!("data.yaml"), json!("{ a: Name@./types.yaml }[]")]).is_err());
    let types = dir.path().join("types.yaml");
    assert!(kinds_of(&context.data.observations_since(Default::default()), &types).contains(&ObservationKind::MissingProbe));
    assert!(context.data.read_set().contains(&entry(&types)));
}

#[test]
fn a_frontmatter_read_observes_its_hash_and_a_missing_one_its_probes() {
    let dir = tempfile::tempdir().unwrap();
    let page = dir.path().join("page.md");
    std::fs::write(&page, "---\ntitle: T\n---\n").unwrap();
    let context = context(dir.path());
    assert_eq!(call(&context, "frontmatter", &[json!("./page.md"), json!("title")]).unwrap(), json!("T"));
    assert!(call(&context, "frontmatter", &[json!("./gone.md")]).is_err());
    let observed = context.data.observations_since(Default::default());
    assert!(observed.contains(&Observation {
        entry: entry(&page),
        kind: ObservationKind::File,
        hash: Some(biscuit_hash::blake3_hash_bytes(b"---\ntitle: T\n---\n")),
    }));
    assert_eq!(kinds_of(&observed, &dir.path().join("gone.md")), [ObservationKind::MissingProbe]);
}

#[test]
fn a_path_function_observes_its_probes_up_to_the_file_found() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("here.md"), "").unwrap();
    let context = context(dir.path());
    assert_eq!(call(&context, "basename", &[json!("./here.md")]).unwrap(), json!("here.md"));
    assert_eq!(call(&context, "basename", &[json!("./later.md")]).unwrap(), json!("later.md"));
    let observed = context.data.observations_since(Default::default());
    assert_eq!(kinds_of(&observed, &dir.path().join("here.md")), [ObservationKind::Exists]);
    assert_eq!(kinds_of(&observed, &dir.path().join("later.md")), [ObservationKind::MissingProbe]);
}

#[test]
fn a_skill_probe_observes_each_folder_it_looked_in() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let context = context(dir.path()).with_home_dir(home.path().to_path_buf()).with_ctx_value("agent", json!("claude"));
    assert_eq!(call(&context, "has_skill", &[json!("absent-skill")]).unwrap(), json!(false));
    let observed = context.data.observations_since(Default::default());
    let probed = home.path().join(".claude").join("skills").join("absent-skill");
    assert_eq!(kinds_of(&observed, &probed), [ObservationKind::MissingProbe]);
}

#[test]
fn listings_observe_the_sorted_names_and_a_glob_stays_out_of_the_read_set() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["review-2.md", "review-1.md"] {
        std::fs::write(dir.path().join(name), "").unwrap();
    }
    let context = context(dir.path());
    let first = call(&context, "find_first_index", &[json!("./review.md")]).unwrap();
    assert_eq!(first, json!("review-1.md"));
    let folder = entry(dir.path());
    let observed = context.data.observations_since(Default::default());
    let listing = observed.iter().find(|observation| observation.entry == folder).expect("the folder listing");
    assert_eq!(listing.kind, ObservationKind::Listing);
    assert_eq!(listing.hash, Some(biscuit_hash::blake3_hash_bytes(b"review-1.md\0review-2.md")));
    assert!(context.data.read_set().contains(&folder));

    let mark = context.data.read_mark();
    let before = context.data.read_set();
    let found = call(&context, "find_files", &[json!("./*.md")]).unwrap();
    assert_eq!(found.as_array().map(Vec::len), Some(2));
    let glob = context.data.observations_since(mark);
    assert_eq!(glob.len(), 1, "{glob:?}");
    assert_eq!((glob[0].kind, glob[0].entry.as_str()), (ObservationKind::Glob, "./*.md"));
    assert_eq!(context.data.read_set(), before, "a glob pattern is not a read-set entry");
}

#[test]
fn the_memo_generation_grows_on_each_invalidation() {
    let runtime = super::DataRuntime::new();
    assert_eq!(runtime.memo_generation(), 0);
    runtime.invalidate_memo();
    runtime.invalidate_memo();
    assert_eq!(runtime.memo_generation(), 2);
}

#[test]
fn a_recorded_entry_is_replayed_into_the_read_set_as_written() {
    let runtime = super::DataRuntime::new();
    runtime.record_entry("https://example.com/os.yaml");
    assert_eq!(runtime.read_set(), ["https://example.com/os.yaml"]);
    assert!(runtime.observations_since(Default::default()).is_empty(), "replaying a read observes nothing new");
}
