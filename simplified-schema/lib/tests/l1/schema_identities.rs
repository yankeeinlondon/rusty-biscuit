//! Typed validator-cache identities: canonical digests, layered identities,
//! and the cache keys built from them (`validate::{SchemaIdentity,
//! SchemaDigest, LayeredIdentity}`).
//!
//! Every test observes the public result: which compiled validator a lookup
//! returns, what it decides, and the work counters. A wrong identity would
//! serve a validator compiled for another schema, so the cache tests pair two
//! schemas whose verdicts on one instance differ.

use std::sync::Arc;

use serde_json::{Value, json};
use simplified_schema::validate::{
    CANONICAL_IDENTITY_REVISION, LAYERED_IDENTITY_REVISION, LayerRole, LayeredIdentity, SchemaDigest, SchemaIdentity,
    judges_files,
};
use simplified_schema::{SchemaPhase, ValidatorCache};

fn requires_title() -> Value {
    json!({"type": "object", "required": ["title"], "properties": {"title": {"type": "string"}}})
}

fn allows_anything() -> Value {
    json!({"type": "object"})
}

/// A schema whose only file value is judged eagerly against a context.
fn eager_file() -> Value {
    json!({"type": "object", "properties": {"spec": {"type": "string", "format": "darkmatter-file"}}})
}

fn digest_of(value: &Value) -> SchemaDigest {
    SchemaDigest::canonical(value)
}

/// The fixed layered identity this file pins: a baseline, two triggers, and
/// a document, projected to launch.
fn fixed_identity() -> SchemaDigest {
    LayeredIdentity::new()
        .layer(LayerRole::Baseline, digest_of(&allows_anything()))
        .layer(LayerRole::Trigger, digest_of(&requires_title()))
        .layer(LayerRole::Trigger, digest_of(&eager_file()))
        .layer(LayerRole::Document, digest_of(&json!({"type": "object", "properties": {"n": {"type": "number"}}})))
        .finish()
}

fn fixed_projection() -> SchemaDigest {
    LayeredIdentity::derived_from(fixed_identity())
        .phase_projection(SchemaPhase::Launch, Some(digest_of(&json!({"type": "object", "required": ["n"]}))))
        .finish()
}

fn layered_digest(digest: SchemaDigest) -> u64 {
    match digest.identity {
        SchemaIdentity::Layered { revision, digest } => {
            assert_eq!(revision, LAYERED_IDENTITY_REVISION);
            digest
        }
        SchemaIdentity::Canonical { .. } => panic!("expected a layered identity, got {digest:?}"),
    }
}

// ── Encoding ────────────────────────────────────────────────────────────────

/// The encoding is fixed: little-endian fixed-width numbers, fixed role
/// tags, no pointers, maps, or platform-sized integers. A change to it must
/// be deliberate (bump `LAYERED_IDENTITY_REVISION`), so the values are pinned.
#[test]
fn a_layered_identity_is_pinned_for_fixed_inputs() {
    assert_eq!(format!("{:016x}", layered_digest(fixed_identity())), "83df5d9770344f89");
    assert_eq!(format!("{:016x}", layered_digest(fixed_projection())), "e89539b57da19d99");
    assert!(fixed_identity().judges_files, "a layer judging files makes the merge judge files");
}

const HELPER_ARG: &str = "simplified-schema-identity-helper";

/// What a re-executed copy of this binary does instead of testing: print the
/// fixed identities.
#[test]
fn identity_helper_entrypoint() {
    if std::env::args().any(|arg| arg == HELPER_ARG) {
        println!("identity={:016x}/{:016x}", layered_digest(fixed_identity()), layered_digest(fixed_projection()));
    }
}

/// Two fresh processes compute the identity this process computes: nothing
/// in it depends on addresses, hash-map seeds, or process state.
#[test]
fn a_layered_identity_is_stable_across_processes() {
    let expected = format!("identity={:016x}/{:016x}", layered_digest(fixed_identity()), layered_digest(fixed_projection()));
    let module = module_path!().split_once("::").map(|(_, rest)| rest).expect("module path has a crate segment");
    for _ in 0..2 {
        let output = std::process::Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", &format!("{module}::identity_helper_entrypoint"), "--nocapture", HELPER_ARG])
            .output()
            .expect("re-run this test binary");
        assert!(output.status.success(), "helper failed: {}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().find(|line| line.starts_with("identity=")).expect("the helper prints its identity");
        assert_eq!(line, expected);
    }
}

#[test]
fn layer_order_role_and_projection_each_change_the_identity() {
    let (low, high) = (digest_of(&allows_anything()), digest_of(&requires_title()));
    let identity = |layers: &[(LayerRole, SchemaDigest)]| {
        layers.iter().fold(LayeredIdentity::new(), |identity, (role, digest)| identity.layer(*role, *digest)).finish()
    };
    let control = identity(&[(LayerRole::Baseline, low), (LayerRole::Document, high)]);
    assert_eq!(control, identity(&[(LayerRole::Baseline, low), (LayerRole::Document, high)]), "the control is stable");

    let variants = [
        ("reversed contents", identity(&[(LayerRole::Baseline, high), (LayerRole::Document, low)])),
        ("document as trigger", identity(&[(LayerRole::Baseline, low), (LayerRole::Trigger, high)])),
        ("baseline as trigger", identity(&[(LayerRole::Trigger, low), (LayerRole::Document, high)])),
        ("document alone", identity(&[(LayerRole::Document, high)])),
        ("a repeated layer", identity(&[(LayerRole::Baseline, low), (LayerRole::Document, high), (LayerRole::Document, high)])),
        ("projected to launch", LayeredIdentity::derived_from(control).phase_projection(SchemaPhase::Launch, None).finish()),
        ("projected to completion", LayeredIdentity::derived_from(control).phase_projection(SchemaPhase::Completion, None).finish()),
        (
            "projected with a document",
            LayeredIdentity::derived_from(control).phase_projection(SchemaPhase::Launch, Some(high)).finish(),
        ),
    ];
    for (index, (name, variant)) in variants.iter().enumerate() {
        assert_ne!(variant.identity, control.identity, "{name}");
        for (other, other_variant) in &variants[index + 1..] {
            assert_ne!(variant.identity, other_variant.identity, "{name} vs {other}");
        }
    }

    // Two triggers in either order.
    let (first, second) = (digest_of(&requires_title()), digest_of(&eager_file()));
    assert_ne!(
        identity(&[(LayerRole::Trigger, first), (LayerRole::Trigger, second)]).identity,
        identity(&[(LayerRole::Trigger, second), (LayerRole::Trigger, first)]).identity
    );
}

#[test]
fn a_canonical_digest_ignores_object_key_order_and_keeps_array_order() {
    let authored: Value =
        serde_json::from_str(r#"{"type":"object","required":["a","b"],"properties":{"a":{"type":"string"},"b":{"type":"number"}}}"#)
            .unwrap();
    let reordered: Value =
        serde_json::from_str(r#"{"properties":{"b":{"type":"number"},"a":{"type":"string"}},"required":["a","b"],"type":"object"}"#)
            .unwrap();
    let array_reordered: Value =
        serde_json::from_str(r#"{"type":"object","required":["b","a"],"properties":{"a":{"type":"string"},"b":{"type":"number"}}}"#)
            .unwrap();
    assert_eq!(digest_of(&authored), digest_of(&reordered));
    assert_ne!(digest_of(&authored), digest_of(&array_reordered));
    assert!(matches!(
        digest_of(&authored).identity,
        SchemaIdentity::Canonical { revision: CANONICAL_IDENTITY_REVISION, .. }
    ));
}

#[test]
fn only_the_eager_file_format_and_match_judge_files() {
    let classify = |schema: Value| (digest_of(&schema).judges_files, judges_files(&biscuit_hash::canonical_json(&schema)));
    assert_eq!(classify(eager_file()), (true, true));
    assert_eq!(
        classify(json!({"type": "object", "properties": {"doc": {"type": "string", "x-darkmatter-match": ["*.md"]}}})),
        (true, true)
    );
    assert_eq!(
        classify(json!({"type": "object", "properties": {"doc": {"type": "string", "format": "darkmatter-file-reference"}}})),
        (false, false),
        "the lazy reference format checks syntax only"
    );
    assert_eq!(classify(allows_anything()), (false, false));

    let reference_only = digest_of(&json!({"format": "darkmatter-file-reference"}));
    let layered = |digests: &[SchemaDigest]| {
        digests.iter().fold(LayeredIdentity::new(), |identity, digest| identity.layer(LayerRole::Trigger, *digest)).finish()
    };
    assert!(!layered(&[reference_only, digest_of(&allows_anything())]).judges_files);
    assert!(layered(&[reference_only, digest_of(&eager_file())]).judges_files, "any judging layer counts");
    assert!(
        LayeredIdentity::derived_from(layered(&[reference_only]))
            .phase_projection(SchemaPhase::Launch, Some(digest_of(&eager_file())))
            .finish()
            .judges_files,
        "a judging projected document counts"
    );
}

// ── The cache ───────────────────────────────────────────────────────────────

fn context(dir: &std::path::Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(dir)
}

/// A canonical and a layered identity holding the same 64-bit digest are
/// different keys: the second schema compiles its own validator.
#[test]
fn canonical_and_layered_identities_never_share_a_validator() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ValidatorCache::with_capacity(8);
    let untitled = json!({});
    let digest = 0x0123_4567_89ab_cdef;

    let strict = cache
        .validator_for_shared_with_digest(
            &requires_title(),
            SchemaDigest { identity: SchemaIdentity::canonical(digest), judges_files: false },
            None,
            &context(dir.path()),
        )
        .unwrap();
    let lenient = cache
        .validator_for_shared_with_digest(
            &allows_anything(),
            SchemaDigest {
                identity: SchemaIdentity::Layered { revision: LAYERED_IDENTITY_REVISION, digest },
                judges_files: false,
            },
            None,
            &context(dir.path()),
        )
        .unwrap();
    assert!(!strict.is_valid(&untitled));
    assert!(lenient.is_valid(&untitled), "the layered identity got its own validator");
    assert!(!Arc::ptr_eq(&strict, &lenient));
    assert_eq!(cache.len(), 2);
}

/// One layered identity serves every data document: the validator compiled
/// for the first is returned for the next, and each verdict is its own.
#[test]
fn documents_with_one_layered_identity_share_its_validator() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ValidatorCache::with_capacity(8);
    let schema = Arc::new(requires_title());
    let digest = LayeredIdentity::new().layer(LayerRole::Document, digest_of(&schema)).finish();

    let first = cache.validator_for_shared_with_digest(&schema, digest, None, &context(dir.path())).unwrap();
    let second = cache.validator_for_shared_with_digest(&schema, digest, None, &context(dir.path())).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(first.is_valid(&json!({"title": "A"})));
    assert!(!second.is_valid(&json!({"title": 5})));
    assert_eq!(cache.len(), 1);
}

/// A file-judging layered identity stays keyed by the document folder: the
/// same identity in two folders compiles two validators, and each judges
/// from its own folder.
#[test]
fn a_file_judging_layered_identity_is_keyed_by_document_folder_and_caller_origins() {
    let root = tempfile::tempdir().unwrap();
    let (with_file, without_file) = (root.path().join("with"), root.path().join("without"));
    std::fs::create_dir_all(&with_file).unwrap();
    std::fs::create_dir_all(&without_file).unwrap();
    std::fs::write(with_file.join("spec.md"), "# Spec\n").unwrap();
    let schema = Arc::new(eager_file());
    let digest = LayeredIdentity::new().layer(LayerRole::Document, digest_of(&schema)).finish();
    assert!(digest.judges_files);

    let cache = ValidatorCache::with_capacity(8);
    let found = cache.validator_for_shared_with_digest(&schema, digest, Some(&with_file), &context(&with_file)).unwrap();
    let missing =
        cache.validator_for_shared_with_digest(&schema, digest, Some(&without_file), &context(&without_file)).unwrap();
    assert!(!Arc::ptr_eq(&found, &missing));
    // Without the `file` feature a file value cannot be resolved anywhere.
    #[cfg(feature = "file")]
    {
        let instance = json!({"spec": "spec.md"});
        assert!(found.is_valid(&instance));
        assert!(!missing.is_valid(&instance), "the other folder's validator would accept it");
    }

    let with_callers = cache.clone().with_caller_origins(simplified_schema::validate::CallerOrigins::new([(
        "spec".to_string(),
        context(&without_file),
    )]));
    let caller = with_callers
        .validator_for_shared_with_digest(&schema, digest, Some(&with_file), &context(&with_file))
        .unwrap();
    assert!(!Arc::ptr_eq(&found, &caller), "caller origins are part of the key");

    let file_free = Arc::new(allows_anything());
    let free_digest = LayeredIdentity::new().layer(LayerRole::Document, digest_of(&file_free)).finish();
    let a = cache.validator_for_shared_with_digest(&file_free, free_digest, Some(&with_file), &context(&with_file)).unwrap();
    let b = cache
        .validator_for_shared_with_digest(&file_free, free_digest, Some(&without_file), &context(&without_file))
        .unwrap();
    assert!(Arc::ptr_eq(&a, &b), "a file-free layered identity shares one structural validator");
}

/// The canonical route still finds a schema by content: a different
/// allocation of an equal schema reuses the validator.
#[test]
fn the_canonical_route_shares_equal_schemas_across_allocations() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ValidatorCache::with_capacity(8);
    let first = cache.validator_for_shared(&Arc::new(requires_title()), None, &context(dir.path())).unwrap();
    let second = cache.validator_for_shared(&Arc::new(requires_title()), None, &context(dir.path())).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    let held = Arc::new(requires_title());
    assert_eq!(cache.shared_schema_hash(&held), biscuit_hash::xx_hash_json(&requires_title()), "the content hash stays canonical");
}

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use simplified_schema::work_counters::{Work, snapshot};

    /// A known digest compiles and finds its validator without serializing
    /// the schema; the canonical route serializes once per allocation.
    #[test]
    fn a_known_digest_never_serializes_the_schema() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ValidatorCache::with_capacity(8);
        let schema = Arc::new(requires_title());
        let digest = LayeredIdentity::new().layer(LayerRole::Document, SchemaDigest::canonical(&schema)).finish();

        let before = snapshot();
        let first = cache.validator_for_shared_with_digest(&schema, digest, None, &context(dir.path())).unwrap();
        let second = cache.validator_for_shared_with_digest(&schema, digest, None, &context(dir.path())).unwrap();
        let work = snapshot().since(&before);
        assert_eq!(work.get(Work::CanonicalSchemaSerialization), 0);
        assert_eq!(work.get(Work::ValidatorCompile), 1, "compiled once, then reused");
        assert!(Arc::ptr_eq(&first, &second));

        let before = snapshot();
        cache.validator_for_shared(&schema, None, &context(dir.path())).unwrap();
        cache.validator_for_shared(&schema, None, &context(dir.path())).unwrap();
        let work = snapshot().since(&before);
        assert_eq!(work.get(Work::CanonicalSchemaSerialization), 1, "once per allocation");
        assert_eq!(work.get(Work::ValidatorCompile), 1, "the canonical identity is its own slot");
    }

    /// A conversion plan found by a layered identity finds the structural
    /// validator compiled under it rather than compiling another.
    #[test]
    fn a_conversion_plan_by_identity_reuses_the_validator_compiled_under_it() {
        let schema = Arc::new(json!({"type": "object", "properties": {"n": {"type": "number"}}}));
        let digest = LayeredIdentity::new().layer(LayerRole::Document, SchemaDigest::canonical(&schema)).finish();
        let dir = tempfile::tempdir().unwrap();
        simplified_schema::validate::SHARED_VALIDATOR_CACHE
            .validator_for_shared_with_digest(&schema, digest, None, &context(dir.path()))
            .unwrap();

        let before = snapshot();
        let plan = simplified_schema::coerce::ConversionPlan::for_identity(digest.identity);
        let outcome = simplified_schema::coerce::coerce_with_plan(&plan, &schema, &json!({"n": "4"}), &Default::default());
        let work = snapshot().since(&before);
        assert_eq!(outcome.value, json!({"n": 4}));
        assert_eq!(work.get(Work::ValidatorCompile), 0);
        assert_eq!(work.get(Work::CanonicalSchemaSerialization), 0);
    }
}
