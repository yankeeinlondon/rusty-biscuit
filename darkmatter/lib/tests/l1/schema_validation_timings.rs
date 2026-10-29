//! Stage timings of one schema validation (`ValidationTimings`): which
//! stages report work and the compile-cache state, never durations' values.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    CompileCache, DarkmatterSchemas, ValidationStage, ValidationTimings, reset_process_caches_for_bench,
};

const STAGE_NAMES: [&str; 5] = ["discover", "match", "resolve", "compile", "validate"];

/// A canonical temp repository root; the guard keeps it alive.
fn repository() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    (dir, root)
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

fn schemas(root: &Path) -> DarkmatterSchemas {
    let context =
        biscuit_file::FileResolutionContext::from_snapshot(root, None, HashMap::new()).with_repository_root(root);
    DarkmatterSchemas::new(context)
}

fn document(root: &Path, name: &str, text: &str) -> Markdown {
    let path = root.join(name);
    write(&path, text);
    Markdown::try_from(path.as_path()).expect("document")
}

fn stage_names(timings: &ValidationTimings) -> Vec<&'static str> {
    timings.stages().map(|(stage, _)| stage.name()).collect()
}

#[test]
fn second_validation_of_a_document_is_a_full_compile_cache_hit() {
    let (_dir, root) = repository();
    let doc = document(&root, "doc.md", "---\n$schema:\n  full_hit_title: string(required)\nfull_hit_title: x\n---\nbody\n");
    let api = schemas(&root);
    reset_process_caches_for_bench();

    let (cold_result, cold) = api.validate_timed(&doc);
    let (warm_result, warm) = api.validate_timed(&doc);

    assert!(cold_result.expect("cold validation").valid);
    assert!(warm_result.expect("warm validation").valid);
    assert_eq!(cold.compile_cache(), CompileCache::Miss, "a cold validation builds its validator");
    assert!(cold.stage(ValidationStage::Compile) > Duration::ZERO, "a build is timed as compile");
    assert_eq!(warm.compile_cache(), CompileCache::Hit);
    assert_eq!(warm.stage(ValidationStage::Compile), Duration::ZERO, "cache hits count as zero");
    assert_eq!(stage_names(&warm), STAGE_NAMES);
    assert!(warm.stage(ValidationStage::Resolve) > Duration::ZERO);
    assert!(warm.stage(ValidationStage::Validate) > Duration::ZERO);
    assert!(warm.total() > Duration::ZERO);
    let staged: Duration = warm.stages().map(|(_, elapsed)| elapsed).sum();
    assert_eq!(staged + warm.unassigned(), warm.total(), "stages and unassigned partition the total");
}

#[test]
fn one_cached_arm_and_one_new_arm_is_a_compile_miss() {
    let (_dir, root) = repository();
    // Arms carry a per-test property so no other test shares their validators.
    let warmed = document(
        &root,
        "warmed.md",
        "---\n$schema:\n  - mixed_hit_a: string(required)\n  - mixed_hit_b: number(required)\n---\n",
    );
    let mixed = document(
        &root,
        "mixed.md",
        "---\n$schema:\n  - mixed_hit_a: string(required)\n  - mixed_miss_c: boolean(required)\n---\n",
    );
    let api = schemas(&root);
    reset_process_caches_for_bench();
    api.validate(&warmed).expect("warming validation");
    let cached_before = api.cache().len();

    let (result, timings) = api.validate_timed(&mixed);

    result.expect("mixed validation");
    assert_eq!(api.cache().len(), cached_before + 1, "exactly one arm was built; the other was cached");
    assert_eq!(timings.compile_cache(), CompileCache::Miss);
    assert!(timings.stage(ValidationStage::Compile) > Duration::ZERO);
}

#[test]
fn a_document_without_any_schema_requests_no_compilation() {
    let (_dir, root) = repository();
    let doc = document(&root, "plain.md", "---\ntitle: plain\n---\nbody\n");

    let (result, timings) = schemas(&root).validate_timed(&doc);

    assert!(result.expect("validation").valid);
    assert_eq!(timings.compile_cache(), CompileCache::NotRequested);
    assert_eq!(timings.stage(ValidationStage::Compile), Duration::ZERO);
    assert_eq!(timings.stage(ValidationStage::Validate), Duration::ZERO, "nothing was checked");
    assert!(timings.matched_triggers().is_empty());
    assert_eq!(stage_names(&timings), STAGE_NAMES);
}

#[test]
fn an_unresolvable_schema_keeps_its_error_and_reports_partial_timings() {
    let (_dir, root) = repository();
    let doc = document(&root, "broken.md", "---\n$schema: ./missing-schema.yaml\ntitle: x\n---\n");
    let api = schemas(&root);

    let untimed = api.validate(&doc).expect_err("the schema reference does not resolve");
    let (timed, timings) = api.validate_timed(&doc);

    let timed = timed.expect_err("the timed validation fails the same way");
    assert_eq!(format!("{timed:?}"), format!("{untimed:?}"), "timing does not replace the error");
    assert!(timings.stage(ValidationStage::Resolve) > Duration::ZERO, "resolution ran before failing");
    assert_eq!(timings.compile_cache(), CompileCache::NotRequested, "it stopped before any validator");
    assert_eq!(timings.stage(ValidationStage::Validate), Duration::ZERO);
    assert!(timings.total() > Duration::ZERO);
}

#[test]
fn a_trigger_matched_document_names_its_triggers_and_discovery_is_recorded() {
    let (_dir, root) = repository();
    write(
        &root.join("schemas/timed-trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  timed_prompt: string(required)\n$schema: timed-payload.yaml\n",
    );
    write(&root.join("schemas/timed-payload.yaml"), "$schema:\n  timed_model: string(required)\n");
    write(
        &root.join("schemas/idle-trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  idle_marker: string(required)\n$schema: timed-payload.yaml\n",
    );
    let doc = document(&root, "doc.md", "---\ntimed_prompt: hello\ntimed_model: m\n---\n");

    let (api, discovery) = ValidationTimings::record(|| schemas(&root).with_trigger_discovery());
    let api = api.expect("trigger discovery");
    let (result, mut timings) = api.validate_timed(&doc);
    timings.absorb(discovery);

    assert!(result.expect("validation").valid);
    assert_eq!(timings.matched_triggers(), ["timed-trigger.yaml".to_string()]);
    assert!(timings.stage(ValidationStage::Discover) > Duration::ZERO, "the scan is discovery");
    assert!(timings.stage(ValidationStage::Match) > Duration::ZERO);
    assert!(timings.stage(ValidationStage::Resolve) > Duration::ZERO);
    assert_ne!(timings.compile_cache(), CompileCache::NotRequested);
}

#[test]
fn timed_and_untimed_validation_report_the_same_verdict() {
    let (_dir, root) = repository();
    let doc = document(&root, "doc.md", "---\n$schema:\n  same_count: number(required)\nsame_count: many\n---\n");
    let api = schemas(&root);

    let untimed = api.validate(&doc).expect("untimed validation");
    let (timed, _) = api.validate_timed(&doc);

    assert!(!untimed.valid);
    assert_eq!(format!("{:?}", timed.expect("timed validation")), format!("{untimed:?}"));
}
