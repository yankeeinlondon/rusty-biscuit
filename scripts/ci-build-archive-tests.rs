//! Tests for the producer/consumer archive contract.
//!
//! Two layers. The unit fixtures below drive [`verify`] against hand-built
//! manifests, so every rejection code has a test that reaches it and no test
//! has to compile anything. The end-to-end fixture at the bottom builds the
//! checked-in `scripts/ci/fixtures/archive-portability` workspace through the
//! shipped binary, relocates it, and runs all three tiers with no compiler in
//! reach — which is the only way to prove the claim this whole module exists
//! to support.

use super::*;

use std::io::Write as _;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn scratch(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "ci-build-archive-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    ));
    fs::create_dir_all(&base).expect("creating the scratch directory");
    base
}

/// A scratch directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        Self(scratch(tag))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        // Git marks every object file read-only, and Windows refuses to delete
        // one; the retry below is what keeps a fixture that builds a repository
        // from leaving its whole tree behind in the runner's temp directory.
        if self.0.exists() {
            clear_readonly(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

/// Clear the read-only bit on every file under `path`.
///
/// Unix ignores the bit when unlinking — only the parent directory's write
/// permission matters — so this is a no-op there rather than a portability
/// shim that also loosens permissions no one asked about.
#[cfg(windows)]
fn clear_readonly(path: &Path) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            clear_readonly(&child);
        } else if let Ok(metadata) = fs::symlink_metadata(&child) {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            let _ = fs::set_permissions(&child, permissions);
        }
    }
}

#[cfg(not(windows))]
fn clear_readonly(_path: &Path) {}

/// `fs::remove_dir_all` that survives Git's read-only object files.
fn remove_tree(path: &Path) {
    if fs::remove_dir_all(path).is_ok() {
        return;
    }
    clear_readonly(path);
    fs::remove_dir_all(path)
        .unwrap_or_else(|error| panic!("removing {}: {error}", path.display()));
}

fn identity() -> Identity {
    Identity {
        source_commit: "c0ffee".repeat(6) + "abcd",
        lockfile: "0123456789abcdef".to_owned(),
        rust: "1.97.1".to_owned(),
        nextest: "latest".to_owned(),
        host: "x86_64-unknown-linux-gnu".to_owned(),
        target: "x86_64-unknown-linux-gnu".to_owned(),
        profile: "test".to_owned(),
        rustflags: String::new(),
        cargo_config: Vec::new(),
        linker: "cc".to_owned(),
        archive_format: "tar.zst".to_owned(),
        package: "alpha".to_owned(),
        target_kinds: vec!["lib".to_owned(), "test".to_owned()],
        features: String::new(),
        native: Vec::new(),
        archive_includes: Vec::new(),
        sidecars: Vec::new(),
    }
}

fn linux_runtime() -> Runtime {
    Runtime {
        arch: "x86_64".to_owned(),
        abi: "gnu".to_owned(),
        libc: "glibc".to_owned(),
        native_libraries: Vec::new(),
    }
}

/// Write `body` into `dir` and return the record a manifest would carry.
fn emit(dir: &Path, name: &str, body: &[u8]) -> FileRecord {
    let path = dir.join(name);
    let mut file = fs::File::create(&path).expect("writing an emitted file");
    file.write_all(body).expect("writing an emitted file body");
    drop(file);
    file_record(dir, &path).expect("digesting an emitted file")
}

/// A manifest whose emitted files really exist under `dir`, with a digest that
/// already agrees with its own fields.
fn manifest(dir: &Path) -> Manifest {
    let archive = emit(dir, "build-alpha-ubuntu-latest-0123.tar.zst", b"archive body");
    let mut manifest = Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        key: "0123456789abcdef".to_owned(),
        digest: String::new(),
        package: "alpha".to_owned(),
        producer: "ubuntu-latest".to_owned(),
        artifact: "build-alpha-ubuntu-latest-0123".to_owned(),
        compatible_environments: vec!["ubuntu-latest".to_owned(), "wsl2-ubuntu".to_owned()],
        source_commit: identity().source_commit,
        source_tree: "tree0123".to_owned(),
        producer_workspace: "/home/runner/work/rusty-biscuit/rusty-biscuit".to_owned(),
        identity: identity(),
        realized: Realized {
            rustc: "rustc 1.97.1".to_owned(),
            cargo: "cargo 1.97.1".to_owned(),
            nextest: "cargo-nextest 0.9.136".to_owned(),
            host: "x86_64-unknown-linux-gnu".to_owned(),
            target: "x86_64-unknown-linux-gnu".to_owned(),
            linker: "cc".to_owned(),
            runtime: linux_runtime(),
        },
        archive,
        sidecars: Vec::new(),
        runtime_assets: Vec::new(),
        test_binaries: vec!["alpha".to_owned(), "alpha::l1".to_owned()],
        compiler_work: None,
        timings: Timings::default(),
    };
    manifest.digest = realized_digest(&manifest);
    manifest
}

fn options(dir: &Path, environment: &str) -> VerifyOptions {
    VerifyOptions {
        manifest: dir.join("build.manifest.json"),
        environment: environment.to_owned(),
        plan: None,
        root: Some(dir.to_path_buf()),
        workspace: None,
        nextest: vec!["cargo".to_owned(), "nextest".to_owned()],
        json: true,
        verdict_out: None,
    }
}

/// The plan a manifest built from [`manifest`] came from.
fn plan() -> Plan {
    Plan {
        head: identity().source_commit,
        builds: vec![BuildRecord {
            key: "0123456789abcdef".to_owned(),
            package: "alpha".to_owned(),
            producer: "ubuntu-latest".to_owned(),
            artifact: "build-alpha-ubuntu-latest-0123".to_owned(),
            compatible_environments: vec!["ubuntu-latest".to_owned(), "wsl2-ubuntu".to_owned()],
            compatibility_reason: "same arch, ABI and libc".to_owned(),
            consumers: vec![Consumer {
                environment: "ubuntu-latest".to_owned(),
                gate: "L1".to_owned(),
            }],
            identity: identity(),
        }],
        runtimes: BTreeMap::from([
            ("ubuntu-latest".to_owned(), linux_runtime()),
            ("wsl2-ubuntu".to_owned(), linux_runtime()),
            (
                "windows-latest".to_owned(),
                Runtime {
                    arch: "x86_64".to_owned(),
                    abi: "msvc".to_owned(),
                    libc: "msvc".to_owned(),
                    native_libraries: Vec::new(),
                },
            ),
        ]),
    }
}

/// The host predicates a Linux x86_64 consumer reports, so the unit fixtures
/// assert on the manifest's claims rather than on which machine ran them.
const LINUX_HOST: (&str, &str, &str) = ("x86_64", "gnu", "glibc");

fn codes(rejections: &[Rejection]) -> Vec<&str> {
    rejections
        .iter()
        .map(|rejection| rejection.code.as_str())
        .collect()
}

// ---------------------------------------------------------------------------
// Identity and integrity
// ---------------------------------------------------------------------------

#[test]
fn an_untouched_manifest_is_accepted_by_a_compatible_environment() {
    let dir = Scratch::new("accept");
    let manifest = manifest(dir.path());
    let rejections = verify(
        &manifest,
        &options(dir.path(), "ubuntu-latest"),
        Some(&plan()),
        LINUX_HOST,
    );
    assert_eq!(codes(&rejections), Vec::<&str>::new());
}

#[test]
fn the_linux_archive_is_accepted_by_its_wsl2_guest_as_well() {
    // The specification's central claim, as a property of the documents: one
    // Linux build, two distinct result environments.
    let dir = Scratch::new("guest");
    let manifest = manifest(dir.path());
    let rejections = verify(
        &manifest,
        &options(dir.path(), "wsl2-ubuntu"),
        Some(&plan()),
        LINUX_HOST,
    );
    assert_eq!(codes(&rejections), Vec::<&str>::new());
}

#[test]
fn every_rejection_code_is_in_the_shared_vocabulary() {
    // `Rejection::new` debug-asserts this, but only on codes a test happens to
    // reach. This proves the constant itself has no duplicates and no drift.
    let mut seen = BTreeSet::new();
    for code in REJECTIONS {
        assert!(seen.insert(code), "{code} is listed twice");
        assert!(code.starts_with("build-"), "{code} is not build-scoped");
    }
    assert_eq!(seen.len(), REJECTIONS.len());
}

#[test]
fn the_rejection_vocabulary_matches_the_frozen_cross_language_contract() {
    // `scripts/ci/schema.py` is the definition and this is the consumer. A
    // code that exists on only one side is a rejection a workflow cannot act
    // on, which is worse than no rejection at all.
    let contract: Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("scripts/ has a parent")
                .join(".github/ci/schemas/contract.json"),
        )
        .expect("the frozen contract must be readable"),
    )
    .expect("the frozen contract is JSON");
    let frozen: Vec<&str> = contract["vocabulary"]["build_rejections"]
        .as_array()
        .expect("the contract declares build_rejections")
        .iter()
        .map(|code| code.as_str().expect("a code is a string"))
        .collect();
    assert_eq!(frozen, REJECTIONS.to_vec());
}

#[test]
fn an_edited_manifest_field_breaks_its_own_digest() {
    let dir = Scratch::new("tamper-manifest");
    let mut manifest = manifest(dir.path());
    manifest.identity.features.push_str("--all-features");
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert!(codes(&rejections).contains(&"build-digest-mismatch"), "{rejections:?}");
}

#[test]
fn the_digest_ignores_timings_and_compiler_work() {
    // Two runs that produced byte-identical artifacts must agree on the digest,
    // or a consumer could never compare one build to another.
    let dir = Scratch::new("observational");
    let manifest = manifest(dir.path());
    let mut second = manifest.clone();
    second.timings.total_ms = 999_999;
    second.compiler_work = Some(json!({"compiler_invocations": 412}));
    assert_eq!(realized_digest(&manifest), realized_digest(&second));
}

#[test]
fn a_manifest_from_another_generation_misses_by_name() {
    let dir = Scratch::new("generation");
    let path = dir.path().join("build.manifest.json");
    fs::write(&path, r#"{"schema_version": 99}"#).expect("writing");
    let rejection = read_manifest(&path).expect_err("another generation");
    assert_eq!(rejection.code, "build-manifest-schema");
}

#[test]
fn a_manifest_that_is_not_json_is_refused_as_malformed_rather_than_missing() {
    let dir = Scratch::new("malformed");
    let path = dir.path().join("build.manifest.json");
    fs::write(&path, "not json at all").expect("writing");
    let rejection = read_manifest(&path).expect_err("malformed");
    assert_eq!(rejection.code, "build-manifest-malformed");
}

#[test]
fn a_manifest_missing_a_required_field_is_malformed_rather_than_partially_read() {
    let dir = Scratch::new("partial");
    let path = dir.path().join("build.manifest.json");
    fs::write(
        &path,
        format!(r#"{{"schema_version": {MANIFEST_SCHEMA_VERSION}, "key": "0123456789abcdef"}}"#),
    )
    .expect("writing");
    let rejection = read_manifest(&path).expect_err("partial");
    assert_eq!(rejection.code, "build-manifest-malformed");
}

#[test]
fn an_absent_manifest_is_refused_before_anything_else_is_read() {
    let dir = Scratch::new("absent");
    let rejection =
        read_manifest(&dir.path().join("nothing.json")).expect_err("no manifest at all");
    assert_eq!(rejection.code, "build-manifest-missing");
}

// ---------------------------------------------------------------------------
// Plan agreement
// ---------------------------------------------------------------------------

#[test]
fn a_key_the_plan_does_not_name_is_refused() {
    let dir = Scratch::new("stray-key");
    let mut manifest = manifest(dir.path());
    manifest.key = "fedcba9876543210".to_owned();
    manifest.digest = realized_digest(&manifest);
    let rejections = verify(
        &manifest,
        &options(dir.path(), "ubuntu-latest"),
        Some(&plan()),
        LINUX_HOST,
    );
    assert!(codes(&rejections).contains(&"build-key-mismatch"), "{rejections:?}");
}

#[test]
fn an_archive_of_another_revision_is_refused_even_when_intact() {
    let dir = Scratch::new("wrong-tree");
    let mut manifest = manifest(dir.path());
    manifest.source_commit = "dec0de".repeat(6) + "0000";
    manifest.digest = realized_digest(&manifest);
    let rejections = verify(
        &manifest,
        &options(dir.path(), "ubuntu-latest"),
        Some(&plan()),
        LINUX_HOST,
    );
    assert!(codes(&rejections).contains(&"build-source-mismatch"), "{rejections:?}");
}

#[test]
fn a_build_produced_from_another_identity_is_refused() {
    // The archive is intact, the key matches, and the revision matches — but
    // the feature graph is not the one the plan resolved. Only the unhashed
    // identity fields can catch this.
    let dir = Scratch::new("identity");
    let mut manifest = manifest(dir.path());
    manifest.identity.features = "--all-features".to_owned();
    manifest.digest = realized_digest(&manifest);
    let rejections = verify(
        &manifest,
        &options(dir.path(), "ubuntu-latest"),
        Some(&plan()),
        LINUX_HOST,
    );
    assert!(codes(&rejections).contains(&"build-key-mismatch"), "{rejections:?}");
}

#[test]
fn a_windows_consumer_cannot_execute_the_linux_archive() {
    let dir = Scratch::new("cross-abi");
    let manifest = manifest(dir.path());
    let rejections = verify(
        &manifest,
        &options(dir.path(), "windows-latest"),
        Some(&plan()),
        LINUX_HOST,
    );
    let observed = codes(&rejections);
    assert!(observed.contains(&"build-environment-incompatible"), "{rejections:?}");
    assert!(observed.contains(&"build-runtime-incompatible"), "{rejections:?}");
}

#[test]
fn a_host_of_another_architecture_refuses_the_archive() {
    let dir = Scratch::new("wrong-arch");
    let manifest = manifest(dir.path());
    let rejections = verify(
        &manifest,
        &options(dir.path(), "ubuntu-latest"),
        Some(&plan()),
        ("aarch64", "gnu", "glibc"),
    );
    assert!(codes(&rejections).contains(&"build-runtime-incompatible"), "{rejections:?}");
}

// ---------------------------------------------------------------------------
// Emitted files
// ---------------------------------------------------------------------------

#[test]
fn a_missing_archive_is_refused_before_extraction() {
    let dir = Scratch::new("missing-archive");
    let manifest = manifest(dir.path());
    fs::remove_file(dir.path().join(&manifest.archive.file)).expect("removing the archive");
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert!(codes(&rejections).contains(&"build-archive-missing"), "{rejections:?}");
}

#[test]
fn an_altered_archive_is_refused_by_checksum_and_by_size() {
    let dir = Scratch::new("altered-archive");
    let manifest = manifest(dir.path());
    fs::write(dir.path().join(&manifest.archive.file), b"tampered body!!").expect("altering");
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    let observed = codes(&rejections);
    assert_eq!(
        observed
            .iter()
            .filter(|code| **code == "build-archive-corrupt")
            .count(),
        2,
        "size and digest are separate claims: {rejections:?}"
    );
}

#[test]
fn an_archive_altered_without_changing_its_length_is_still_refused() {
    // A length check alone is not integrity. This is the case that proves the
    // BLAKE3 digest is doing the work.
    let dir = Scratch::new("same-length");
    let manifest = manifest(dir.path());
    fs::write(dir.path().join(&manifest.archive.file), b"ARCHIVE BODY").expect("altering");
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert_eq!(codes(&rejections), vec!["build-archive-corrupt"]);
}

#[test]
fn a_missing_sidecar_is_refused() {
    let dir = Scratch::new("missing-sidecar");
    let mut manifest = manifest(dir.path());
    let record = emit(dir.path(), "md", b"the darkmatter fixture");
    manifest.identity.sidecars = vec!["darkmatter-md-fixture".to_owned()];
    manifest.sidecars = vec![NamedFile {
        name: "darkmatter-md-fixture".to_owned(),
        record,
    }];
    manifest.digest = realized_digest(&manifest);
    fs::remove_file(dir.path().join("md")).expect("removing the sidecar");

    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert!(codes(&rejections).contains(&"build-sidecar-missing"), "{rejections:?}");
}

#[test]
fn an_altered_sidecar_is_refused() {
    let dir = Scratch::new("altered-sidecar");
    let mut manifest = manifest(dir.path());
    let record = emit(dir.path(), "md", b"the darkmatter fixture");
    manifest.identity.sidecars = vec!["darkmatter-md-fixture".to_owned()];
    manifest.sidecars = vec![NamedFile {
        name: "darkmatter-md-fixture".to_owned(),
        record,
    }];
    manifest.digest = realized_digest(&manifest);
    fs::write(dir.path().join("md"), b"the darkmatter FIXTURE").expect("altering the sidecar");

    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert!(codes(&rejections).contains(&"build-sidecar-corrupt"), "{rejections:?}");
}

#[test]
fn a_declared_sidecar_that_emitted_no_file_is_refused() {
    let dir = Scratch::new("unemitted-sidecar");
    let mut manifest = manifest(dir.path());
    manifest.identity.sidecars = vec!["harness-broker".to_owned()];
    manifest.digest = realized_digest(&manifest);
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert!(codes(&rejections).contains(&"build-sidecar-missing"), "{rejections:?}");
}

#[test]
fn a_declared_archive_include_that_never_reached_the_manifest_is_refused() {
    let dir = Scratch::new("unemitted-include");
    let mut manifest = manifest(dir.path());
    manifest.identity.archive_includes = vec!["examples/discovery_probe".to_owned()];
    manifest.digest = realized_digest(&manifest);
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    assert!(codes(&rejections).contains(&"build-asset-missing"), "{rejections:?}");
}

// ---------------------------------------------------------------------------
// Include paths and the generated tool config
// ---------------------------------------------------------------------------

#[test]
fn an_include_is_spelled_under_the_triple_and_profile_the_producer_used() {
    let mut identity = identity();
    assert_eq!(
        include_path(&identity, "examples/discovery_probe"),
        "x86_64-unknown-linux-gnu/debug/examples/discovery_probe"
    );
    // No explicit `--target`: Cargo writes straight into `<profile>`, and an
    // include spelled with a triple would name a directory that never exists.
    identity.target = String::new();
    assert_eq!(
        include_path(&identity, "examples/discovery_probe"),
        "debug/examples/discovery_probe"
    );
    // `test` and `bench` are profiles Cargo derives; their output directories
    // are not named after them.
    identity.profile = "bench".to_owned();
    assert_eq!(include_path(&identity, "probe"), "release/probe");
    identity.profile = "release".to_owned();
    assert_eq!(include_path(&identity, "probe"), "release/probe");
}

#[test]
fn a_dynamic_library_include_is_spelled_for_the_producers_own_platform() {
    let expanded = expand_placeholders("{DLL_PREFIX}archive_portability_dylib{DLL_SUFFIX}");
    assert!(expanded.contains("archive_portability_dylib"));
    assert!(
        expanded.ends_with(std::env::consts::DLL_SUFFIX),
        "{expanded} must end with this platform's dynamic-library suffix"
    );
    assert!(!expanded.contains('{'), "{expanded} still has a placeholder");
    assert_eq!(
        expand_placeholders("tool{EXE_SUFFIX}"),
        format!("tool{}", std::env::consts::EXE_SUFFIX)
    );
}

#[test]
fn the_generated_tool_config_keeps_the_repositorys_own_includes() {
    // Dropping them would silently remove files from the archive that shipped
    // tests already depend on — the `discovery_probe` example is one.
    let mut identity = identity();
    identity.archive_includes = vec!["examples/mine".to_owned()];
    let inherited: Vec<toml::Value> = toml::from_str::<toml::Table>(
        "include = [{ path = \"debug/examples/discovery_probe\", relative-to = \"target\", \
         on-missing = \"ignore\" }]\n",
    )
    .expect("the inherited entry parses")["include"]
        .as_array()
        .expect("an array")
        .clone();
    let text = tool_config(&identity, &inherited);
    assert!(
        text.contains(&format!("[[profile.{ARCHIVE_PROFILE}.archive.include]]")),
        "{text}"
    );
    assert!(text.contains("discovery_probe"), "{text}");
    assert!(
        text.contains("x86_64-unknown-linux-gnu/debug/examples/mine"),
        "{text}"
    );
    assert!(text.contains(r#"on-missing = "error""#), "{text}");
    // Parses as TOML, or nextest refuses the whole config.
    let parsed: toml::Table =
        toml::from_str(&text).expect("the generated config must be TOML");
    assert_eq!(
        parsed["profile"][ARCHIVE_PROFILE]["archive"]["include"]
            .as_array()
            .expect("an include array")
            .len(),
        2
    );
}

#[test]
fn a_repository_without_a_nextest_config_inherits_nothing_rather_than_failing() {
    let dir = Scratch::new("no-config");
    assert_eq!(
        inherited_includes(dir.path()).expect("no config"),
        Vec::<toml::Value>::new()
    );
}

#[test]
fn a_repository_config_with_no_archive_includes_inherits_nothing() {
    let dir = Scratch::new("bare-config");
    fs::create_dir_all(dir.path().join(".config")).expect("creating .config");
    fs::write(
        dir.path().join(".config/nextest.toml"),
        "[profile.default]\nfail-fast = false\n",
    )
    .expect("writing");
    assert_eq!(
        inherited_includes(dir.path()).expect("bare config"),
        Vec::<toml::Value>::new()
    );
}

// ---------------------------------------------------------------------------
// Reading the plan and the sidecar table
// ---------------------------------------------------------------------------

#[test]
fn the_shipped_sidecar_table_is_readable_and_names_only_real_packages() {
    // A passive corpus test over the one shipped artifact of this contract:
    // every sidecar the vocabulary offers must parse and declare a package,
    // at least one binary, and a reason.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("scripts/ has a parent");
    let sidecars = read_sidecars(&root.join(".github/ci/sidecars.json"))
        .expect("the shipped sidecar table must parse");
    assert!(!sidecars.is_empty());
    for (name, spec) in &sidecars {
        assert!(!spec.package.is_empty(), "{name} names no package");
        assert!(!spec.bins.is_empty(), "{name} declares no binaries");
        assert!(spec.reason.len() > 40, "{name}'s reason is not a reason");
        assert!(
            root.join(&spec.package).exists() || sidecar_package_exists(root, &spec.package),
            "{name} names package '{}', which is not in this workspace",
            spec.package
        );
    }
}

/// Whether `package` is a member of the monorepo's workspace.
///
/// Read from `Cargo.toml`'s explicit member list rather than `cargo metadata`:
/// this suite runs in the `scripts/` workspace, where invoking Cargo against
/// the outer one would be both slow and a layering violation.
fn sidecar_package_exists(root: &Path, package: &str) -> bool {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest");
    manifest
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split('"').next())
        .any(|member| {
            root.join(member)
                .join("Cargo.toml")
                .to_str()
                .and_then(|path| fs::read_to_string(path).ok())
                .is_some_and(|text| {
                    text.lines()
                        .any(|line| line.trim() == format!("name = \"{package}\""))
                })
        })
}

#[test]
fn a_sidecar_table_from_another_generation_is_refused() {
    let dir = Scratch::new("sidecar-generation");
    let path = dir.path().join("sidecars.json");
    fs::write(&path, r#"{"schema_version": 7, "sidecars": {}}"#).expect("writing");
    let error = read_sidecars(&path).expect_err("another generation");
    assert!(format!("{error:#}").contains("schema version"), "{error:#}");
}

#[test]
fn a_plan_with_no_builds_reads_as_an_empty_owner_slice() {
    let dir = Scratch::new("empty-plan");
    let path = dir.path().join("plan.json");
    fs::write(
        &path,
        serde_json::to_string(&json!({
            "head": "abc",
            "builds": [],
            "environments": [],
        }))
        .unwrap(),
    )
    .expect("writing");
    let plan = read_plan(&path).expect("an all-reused plan is a valid plan");
    assert!(plan.builds.is_empty());
}

#[test]
fn a_plan_whose_build_record_grew_a_field_is_refused_rather_than_partially_read() {
    let dir = Scratch::new("future-plan");
    let path = dir.path().join("plan.json");
    let mut record = serde_json::to_value(&plan().builds[0]).unwrap();
    record["cohort"] = json!("something-new");
    fs::write(
        &path,
        serde_json::to_string(&json!({
            "head": identity().source_commit,
            "builds": [record],
            "environments": [],
        }))
        .unwrap(),
    )
    .expect("writing");
    let error = read_plan(&path).expect_err("an unknown field must not be ignored");
    assert!(format!("{error:#}").contains("cohort"), "{error:#}");
}

#[test]
fn the_plans_environment_runtimes_are_read_for_compatibility_checks() {
    let dir = Scratch::new("plan-runtimes");
    let path = dir.path().join("plan.json");
    fs::write(
        &path,
        serde_json::to_string(&json!({
            "head": "abc",
            "builds": [],
            "environments": [
                {"name": "ubuntu-latest", "build": {"runtime": {
                    "arch": "x86_64", "abi": "gnu", "libc": "glibc", "native_libraries": []
                }}},
                {"name": "no-build-contract"},
            ],
        }))
        .unwrap(),
    )
    .expect("writing");
    let plan = read_plan(&path).expect("reading");
    assert_eq!(plan.runtimes["ubuntu-latest"].libc, "glibc");
    assert!(!plan.runtimes.contains_key("no-build-contract"));
}

// ---------------------------------------------------------------------------
// The verdict document
// ---------------------------------------------------------------------------

#[test]
fn the_verdict_is_a_versioned_document_a_workflow_can_act_on() {
    let dir = Scratch::new("verdict");
    // `run_verify` reads the *real* host's predicates, so this manifest has to
    // claim the machine the suite is running on rather than the Linux fixture.
    let mut manifest = manifest(dir.path());
    let (arch, abi, libc) = host_runtime();
    manifest.realized.runtime = Runtime {
        arch: arch.to_owned(),
        abi: abi.to_owned(),
        libc: libc.to_owned(),
        native_libraries: Vec::new(),
    };
    manifest.digest = realized_digest(&manifest);
    let manifest = manifest;
    let path = dir.path().join("build.manifest.json");
    fs::write(&path, serde_json::to_string(&manifest).unwrap()).expect("writing");

    let term = Terminal::new();
    let mut options = options(dir.path(), "ubuntu-latest");
    options.manifest = path;
    assert!(run_verify(&options, &term).expect("verifying"));

    // And the refusal path answers a document too, rather than only a message.
    fs::write(dir.path().join(&manifest.archive.file), b"x").expect("altering");
    assert!(!run_verify(&options, &term).expect("verifying"));
}

#[test]
fn a_refusal_never_names_a_way_to_rebuild() {
    // The one behavior the whole contract rests on: a consumer states what is
    // wrong and stops. It must not suggest, or take, a compile.
    let dir = Scratch::new("no-fallback");
    let manifest = manifest(dir.path());
    fs::remove_file(dir.path().join(&manifest.archive.file)).expect("removing");
    let rejections = verify(&manifest, &options(dir.path(), "ubuntu-latest"), None, LINUX_HOST);
    let rendered = render_verdict(
        &Verdict {
            schema_version: VERDICT_SCHEMA_VERSION,
            accepted: false,
            environment: "ubuntu-latest".to_owned(),
            key: Some(manifest.key.clone()),
            digest: Some(manifest.digest.clone()),
            package: Some(manifest.package.clone()),
            producer: Some(manifest.producer.clone()),
            inventory_checked: false,
            timings: VerifyTimings::default(),
            rejections,
        },
        &Terminal::new(),
    );
    for forbidden in ["cargo build", "cargo nextest run", "rebuild"] {
        assert!(
            !rendered.contains(forbidden),
            "the refusal must not offer {forbidden}: {rendered}"
        );
    }
    assert!(rendered.contains("Nothing is compiled"), "{rendered}");
}

#[test]
fn an_empty_owner_slice_renders_as_the_expected_shape_of_a_reused_plan() {
    let rendered = render_produced(&[], "macos-latest", &Terminal::new());
    assert!(rendered.contains("macos-latest"), "{rendered}");
    assert!(rendered.contains("No build records"), "{rendered}");
}

// ---------------------------------------------------------------------------
// End to end, through the shipped binary and the checked-in fixture workspace
// ---------------------------------------------------------------------------
//
// `scripts/ci/fixtures/archive-portability` carries one of every payload class
// the contract has to move: three tier-marked test binaries, a non-test
// executable, a dynamically linked crate, build-script output read at run time,
// a repository fixture, a declared archive include, and a build sidecar. The
// fixtures below produce it, relocate it, and run it.
//
// They prove *archive plumbing*. A failure here is never evidence that a
// production terminal or browser backend was unavailable — the fixture drives
// neither.

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("scripts/ has a parent")
        .to_path_buf()
}

fn fixture_workspace() -> PathBuf {
    repo_root().join("scripts/ci/fixtures/archive-portability")
}

/// This host's target triple, as the producer's plan has to spell it.
fn host_triple() -> String {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("rustc must be present to produce an archive");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .expect("rustc -vV reports a host triple")
}

/// A path in the spelling a `just` recipe's `*args` list survives.
///
/// A tier recipe pastes `{{ args }}` raw into `forwarded=({{ args }})`, so bash
/// word-splits the list AND processes backslash escapes: a native Windows
/// spelling arrives as `C:Usersken…` and nextest reports a missing file for a
/// path nobody typed (measured on `build-win-native`, 2026-09-14). CI's
/// consumer steps hand the tier recipes what `_native_path` answers, and so
/// does this — through the shipped recipe rather than a second copy of its
/// rule, which also makes every archive-mode fixture an end-to-end test of it.
fn recipe_path(path: &Path) -> String {
    let output = Command::new("just")
        .arg("--justfile")
        .arg(repo_root().join("justfile"))
        .arg("--working-directory")
        .arg(repo_root())
        .arg("_native_path")
        .arg(path)
        .output()
        .expect("just must be present: it is this repository's canonical runner");
    assert!(
        output.status.success(),
        "_native_path {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The filterset the canonical recipe selects `tier` with.
///
/// Read from `just` rather than restated here: a second copy of these
/// expressions is exactly the drift that would make this fixture pass while the
/// real tier ran something else.
fn tier_filter(tier: &str) -> String {
    tier_filter_for(tier, "archive-portability")
}

fn tier_filter_for(tier: &str, package: &str) -> String {
    let output = Command::new("just")
        .arg("--justfile")
        .arg(repo_root().join("justfile"))
        .arg("--working-directory")
        .arg(repo_root())
        .arg("_tier_filter")
        .arg(tier)
        .arg(package)
        .output()
        .expect("just must be present: it is this repository's canonical runner");
    assert!(
        output.status.success(),
        "_tier_filter {tier} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn fixture_git(workspace: &Path, args: &[&str]) -> String {
    let output = Command::new("git").current_dir(workspace)
        // `gc.auto=0`: committing the repository's ~11k files crosses the loose-object
        // threshold, and the background repack then deletes the very objects a
        // concurrent `git clone` is midway through reading — an intermittent
        // "failed to copy file to .git/objects/…: No such file or directory".
        .args(["-c", "user.name=Fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false", "-c", "gc.auto=0"])
        .args(args).output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn fixture_checkout(dir: &Path, source: &Path) -> String {
    let workspace = dir.join("workspace");
    if !workspace.join(".git").exists() {
        copy_tree(source, &workspace);
        fixture_git(&workspace, &["init", "--quiet"]);
        fixture_git(&workspace, &["add", "-A"]);
        fixture_git(&workspace, &["commit", "--quiet", "-m", "fixture"]);
    }
    fixture_git(&workspace, &["rev-parse", "HEAD"])
}

/// A resolved plan naming exactly the fixture's one build record.
fn fixture_plan(dir: &Path, triple: &str) -> PathBuf {
    let commit = fixture_checkout(dir, &fixture_workspace());
    let (arch, abi, libc) = host_runtime();
    let identity = json!({
        "source_commit": commit,
        "lockfile": "0000000000000000",
        "rust": "1.97.1",
        "nextest": "latest",
        "host": triple,
        "target": triple,
        "profile": "test",
        "rustflags": "",
        "cargo_config": [],
        "linker": if cfg!(windows) { "link.exe" } else { "cc" },
        "archive_format": "tar.zst",
        "package": "archive-portability",
        "target_kinds": ["lib", "bin", "test"],
        "features": "",
        "native": [],
        "archive_includes": [
            "{DLL_PREFIX}archive_portability_dylib{DLL_SUFFIX}",
            "examples/probe{EXE_SUFFIX}",
        ],
        "sidecars": ["fixture-tool"],
    });
    let plan = json!({
        "head": commit,
        "environments": [{
            "name": "local-host",
            "build": {"runtime": {
                "arch": arch, "abi": abi, "libc": libc, "native_libraries": [],
            }},
        }],
        "builds": [{
            "key": "0f1e2d3c4b5a6978",
            "package": "archive-portability",
            "producer": "local-host",
            "artifact": "build-archive-portability-local-host-0f1e2d3c4b5a6978",
            "compatible_environments": ["local-host"],
            "compatibility_reason": "the fixture's producer is its only consumer",
            "consumers": [
                {"environment": "local-host", "gate": "L1"},
                {"environment": "local-host", "gate": "L2"},
                {"environment": "local-host", "gate": "browser"},
            ],
            "identity": identity,
        }],
    });
    let path = dir.join("plan.json");
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");
    path
}

fn fixture_sidecars(dir: &Path) -> PathBuf {
    let path = dir.join("sidecars.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": SIDECAR_SCHEMA_VERSION,
            "sidecars": {
                "fixture-tool": {
                    "package": "archive-portability-sidecar",
                    "features": [],
                    "bins": ["archive-portability-sidecar"],
                    "reason": "stands in for the monorepo's compile-time tools: another package's binary that a consumer cannot build for itself.",
                },
            },
        }))
        .unwrap(),
    )
    .expect("writing the fixture sidecar table");
    path
}

/// Everything one produced fixture build leaves behind.
struct Produced {
    out: PathBuf,
    target: PathBuf,
    manifest_path: PathBuf,
    manifest: Manifest,
    plan: PathBuf,
    triple: String,
}

/// Drive the shipped `ci-build produce` over the fixture workspace.
fn produce_fixture(dir: &Path) -> Produced {
    let binary = crate::tests::shipped_wrapper();
    let triple = host_triple();
    let plan = fixture_plan(dir, &triple);
    let sidecars = fixture_sidecars(dir);
    let out = dir.join("out");
    let target = dir.join("target");

    let output = Command::new(&binary)
        .arg("produce")
        .arg("--plan")
        .arg(&plan)
        .arg("--producer")
        .arg("local-host")
        .arg("--out-dir")
        .arg(&out)
        .arg("--workspace")
        .arg(dir.join("workspace"))
        .arg("--target-dir")
        .arg(&target)
        .arg("--sidecars")
        .arg(&sidecars)
        .arg("--json")
        .output()
        .expect("running ci-build produce");
    assert!(
        output.status.success(),
        "produce failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest_path = out.join("build-archive-portability-local-host-0f1e2d3c4b5a6978.manifest.json");
    let manifest = read_manifest(&manifest_path).expect("the producer's own manifest must parse");
    Produced {
        out,
        target,
        manifest_path,
        manifest,
        plan,
        triple,
    }
}

/// Run the shipped `ci-build verify` and answer its verdict document.
fn verify_fixture(produced: &Produced, extra: &[&str]) -> (bool, Value) {
    let binary = crate::tests::shipped_wrapper();
    let output = Command::new(&binary)
        .arg("verify")
        .arg("--manifest")
        .arg(&produced.manifest_path)
        .arg("--environment")
        .arg("local-host")
        .arg("--plan")
        .arg(&produced.plan)
        .arg("--json")
        .args(extra)
        .output()
        .expect("running ci-build verify");
    let verdict: Value =
        serde_json::from_slice(&output.stdout).expect("the verdict is a JSON document");
    (output.status.success(), verdict)
}

#[test]
fn the_fixture_archive_carries_every_declared_payload_class() {
    let dir = Scratch::new("produce");
    let produced = produce_fixture(dir.path());
    let manifest = &produced.manifest;

    assert_eq!(manifest.key, "0f1e2d3c4b5a6978");
    assert_eq!(manifest.digest, realized_digest(manifest));
    assert_eq!(manifest.source_tree, fixture_git(&dir.path().join("workspace"), &["rev-parse", "HEAD^{tree}"]));
    assert_eq!(manifest.realized.target, produced.triple);
    assert!(manifest.archive.bytes > 0);
    assert_eq!(manifest.archive.blake3.len(), 64);

    // All three tier binaries, the library, and the non-test executable.
    for expected in [
        "archive-portability",
        "archive-portability::l1",
        "archive-portability::level2_marker",
        "archive-portability::browser_marker",
        "archive-portability::bin/archive-portability-tool",
    ] {
        assert!(
            manifest.test_binaries.iter().any(|id| id == expected),
            "{expected} missing from {:?}",
            manifest.test_binaries
        );
    }

    // The declared dynamic-library include, under this producer's own triple
    // and profile directory, with a real digest.
    let dylib = manifest
        .runtime_assets
        .iter()
        .find(|asset| asset.name.contains("archive_portability_dylib"))
        .expect("the declared archive include must be recorded");
    assert!(dylib.record.file.starts_with(&produced.triple), "{dylib:?}");
    assert!(dylib.record.bytes > 0);
    assert_eq!(dylib.record.blake3.len(), 64);

    // The example target. `cargo nextest archive` never builds one, so its
    // presence is proof the producer recognized the declaration and built it
    // before archiving — the class that would otherwise reach every consumer
    // as `discovery_probe example not found`.
    let probe = manifest
        .runtime_assets
        .iter()
        .find(|asset| asset.name.starts_with("examples/probe"))
        .expect("the declared example include must be recorded");
    assert_eq!(
        probe.record.file,
        format!(
            "{}/debug/examples/probe{}",
            produced.triple,
            std::env::consts::EXE_SUFFIX
        ),
    );
    assert!(probe.record.bytes > 0);

    // The build script's output directory and its linked path, discovered from
    // the archive rather than declared.
    assert!(
        manifest
            .runtime_assets
            .iter()
            .any(|asset| asset.record.file.contains("build/")),
        "the build-script output directory must be recorded: {:?}",
        manifest.runtime_assets
    );

    // The sidecar, emitted beside the archive.
    assert_eq!(manifest.sidecars.len(), 1);
    assert_eq!(manifest.sidecars[0].name, "fixture-tool");
    assert!(
        produced
            .out
            .join(&manifest.sidecars[0].record.file)
            .is_file(),
        "the sidecar binary must exist where the manifest says"
    );

    // Producing twice from the same inputs answers the same digest: an
    // observational field must never make one build look like another.
    assert!(manifest.timings.total_ms > 0);
    assert!(manifest.compiler_work.is_none(), "unmeasured runs record no counts");
}

#[test]
fn a_produced_fixture_verifies_including_its_own_archive_inventory() {
    let dir = Scratch::new("verify-e2e");
    let produced = produce_fixture(dir.path());
    let (accepted, verdict) = verify_fixture(
        &produced,
        &[
            "--workspace",
            &dir.path().join("workspace").to_string_lossy(),
        ],
    );
    assert!(accepted, "{verdict:#}");
    assert_eq!(verdict["accepted"], true);
    assert_eq!(verdict["inventory_checked"], true);
    assert_eq!(verdict["key"], "0f1e2d3c4b5a6978");
    assert_eq!(verdict["rejections"].as_array().unwrap().len(), 0);
}

#[test]
fn a_consumer_reports_its_extraction_apart_from_its_identity_checks() {
    // The reporting contract requires extraction to be a stage of its own
    // rather than a cost folded into test time. The verifier extracts the whole
    // archive to list it, and that is the window reported.
    let dir = Scratch::new("verify-timings");
    let produced = produce_fixture(dir.path());
    let out = dir.path().join("nested/verdict.json");
    let (accepted, verdict) = verify_fixture(
        &produced,
        &[
            "--workspace",
            &dir.path().join("workspace").to_string_lossy(),
            "--verdict-out",
            &out.to_string_lossy(),
        ],
    );
    assert!(accepted, "{verdict:#}");

    let extract = verdict["timings"]["extract_ms"]
        .as_u64()
        .expect("the verdict reports an extraction window");
    let total = verdict["timings"]["total_ms"]
        .as_u64()
        .expect("the verdict reports a total");
    assert!(extract > 0, "extracting a real archive takes time: {verdict:#}");
    assert!(total >= extract, "{verdict:#}");
    assert!(
        total >= verdict["timings"]["identity_ms"].as_u64().unwrap(),
        "{verdict:#}"
    );

    // The document a consumer's status artifact reads, written beside the
    // rendered verdict rather than parsed back out of the log.
    let written: Value = serde_json::from_str(&fs::read_to_string(&out).expect("the verdict file"))
        .expect("the written verdict is JSON");
    assert_eq!(written["timings"]["extract_ms"], extract);
    assert_eq!(written["key"], "0f1e2d3c4b5a6978");
}

#[test]
fn a_refused_consumer_still_reports_what_its_transfer_cost() {
    // A cell that never started is exactly the one whose stage costs a reader
    // wants: the verdict document is written for a refusal too.
    let dir = Scratch::new("verify-timings-refused");
    let produced = produce_fixture(dir.path());
    fs::remove_file(produced.out.join(&produced.manifest.archive.file)).expect("removing");
    let out = dir.path().join("verdict.json");
    let (accepted, verdict) =
        verify_fixture(&produced, &["--verdict-out", &out.to_string_lossy()]);
    assert!(!accepted, "{verdict:#}");
    let written: Value = serde_json::from_str(&fs::read_to_string(&out).expect("the verdict file"))
        .expect("the written verdict is JSON");
    assert_eq!(written["accepted"], false);
    assert_eq!(written["timings"]["extract_ms"], 0);
    assert!(written["timings"]["total_ms"].is_u64(), "{written:#}");
}

#[test]
fn a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status() {
    let dir = Scratch::new("tamper-e2e");
    let produced = produce_fixture(dir.path());
    let archive = produced.out.join(&produced.manifest.archive.file);
    let mut body = fs::read(&archive).expect("reading the archive");
    // One byte, deep inside the compressed stream: the length is unchanged, so
    // only the digest can catch it.
    let middle = body.len() / 2;
    body[middle] ^= 0xff;
    fs::write(&archive, body).expect("tampering");

    let (accepted, verdict) = verify_fixture(&produced, &[]);
    assert!(!accepted, "a tampered archive must not be accepted");
    let codes: Vec<&str> = verdict["rejections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rejection| rejection["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, vec!["build-archive-corrupt"], "{verdict:#}");
}

#[test]
fn a_missing_sidecar_is_refused_end_to_end() {
    let dir = Scratch::new("sidecar-e2e");
    let produced = produce_fixture(dir.path());
    fs::remove_file(produced.out.join(&produced.manifest.sidecars[0].record.file))
        .expect("removing the sidecar");
    let (accepted, verdict) = verify_fixture(&produced, &[]);
    assert!(!accepted);
    assert_eq!(verdict["rejections"][0]["code"], "build-sidecar-missing");
}

#[test]
fn an_archive_offered_to_the_wrong_environment_is_refused_end_to_end() {
    let dir = Scratch::new("environment-e2e");
    let produced = produce_fixture(dir.path());
    let binary = crate::tests::shipped_wrapper();
    let output = Command::new(&binary)
        .arg("verify")
        .arg("--manifest")
        .arg(&produced.manifest_path)
        .arg("--environment")
        .arg("windows-latest")
        .arg("--json")
        .output()
        .expect("running ci-build verify");
    assert_eq!(
        output.status.code(),
        Some(3),
        "a refusal has its own exit status, distinct from a tool failure"
    );
    let verdict: Value = serde_json::from_slice(&output.stdout).expect("a verdict");
    assert_eq!(
        verdict["rejections"][0]["code"],
        "build-environment-incompatible"
    );
}

/// A `PATH` containing only the standalone Nextest driver, the verified
/// sidecar, and the tools a canonical recipe shells out to — with no Cargo, no
/// rustc, and no linker anywhere in it.
///
/// This is the consumer contract made testable: an archive run that quietly
/// recompiled would simply fail here.
fn toolchain_free_path(dir: &Path, sidecar_dir: &Path) -> OsString {
    let bin = dir.join("consumer-bin");
    fs::create_dir_all(&bin).expect("creating the consumer bin directory");
    // Copied, not referenced in place: on Windows both of these live beside
    // `cargo.exe` in `~/.cargo/bin`, so the directory itself can never be on a
    // toolchain-free PATH.
    for tool in ["cargo-nextest", "just"] {
        let source = which(tool).unwrap_or_else(|| panic!("{tool} must be installed"));
        fs::copy(&source, bin.join(source.file_name().expect("a file name")))
            .unwrap_or_else(|error| panic!("copying {tool}: {error}"));
    }

    let mut entries = vec![bin, sidecar_dir.to_path_buf()];
    // The shell `just` hands a `#!/usr/bin/env bash` recipe to. On Windows that
    // is git-bash, which is nowhere near the Unix directories below.
    if let Some(shell) = which("bash").and_then(|path| path.parent().map(Path::to_path_buf)) {
        entries.push(shell);
    }
    // Ordinary system directories. Each is skipped if it somehow holds a Cargo,
    // because the whole point of this PATH is that a consumer cannot compile.
    let system: &[&str] = if cfg!(windows) {
        &["C:/Windows/System32", "C:/Windows", "C:/Program Files/Git/usr/bin"]
    } else {
        &["/usr/bin", "/bin", "/usr/local/bin", "/opt/homebrew/bin"]
    };
    for candidate in system {
        let path = PathBuf::from(candidate);
        if path.is_dir() && !path.join(format!("cargo{}", std::env::consts::EXE_SUFFIX)).exists() {
            entries.push(path);
        }
    }
    std::env::join_paths(entries).expect("joining the consumer PATH")
}

/// The first `name` on this process's `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|entry| entry.join(&file))
        .find(|candidate| candidate.is_file())
}

#[test]
fn the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach() {
    let dir = Scratch::new("relocate");
    let produced = produce_fixture(dir.path());

    // A different checkout path, and the producer's target tree renamed out of
    // the way: every compile-time absolute path the binaries baked in is now
    // wrong, so anything that still resolves did so at run time.
    let checkout = dir.path().join("second-checkout");
    copy_tree(&dir.path().join("workspace"), &checkout);
    let hidden = dir.path().join("target-hidden");
    fs::rename(&produced.target, &hidden).expect("hiding the producer target directory");

    let sidecar_dir = produced
        .out
        .join(&produced.manifest.sidecars[0].record.file)
        .parent()
        .expect("the sidecar has a directory")
        .to_path_buf();
    let path = toolchain_free_path(dir.path(), &sidecar_dir);

    // The consumer's own extraction, somewhere neither the producer nor the
    // source checkout has ever written.
    let extract = dir.path().join("extracted");
    fs::create_dir_all(&extract).expect("creating the extraction directory");
    let archive = produced.out.join(&produced.manifest.archive.file);

    for (tier, expected) in [("L1", 6_usize), ("L2", 1), ("browser", 1)] {
        let output = Command::new("cargo-nextest")
            .arg("nextest")
            .arg("run")
            .arg("--archive-file")
            .arg(&archive)
            .arg("--workspace-remap")
            .arg(&checkout)
            .arg("--extract-to")
            .arg(&extract)
            .arg("--extract-overwrite")
            .arg("-E")
            .arg(tier_filter(tier))
            .arg("--no-tests=fail")
            .env("PATH", &path)
            .env_remove("RUSTC")
            .env_remove("CARGO")
            .current_dir(dir.path())
            .output()
            .expect("running the archive");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "the {tier} tier must run from the archive:\n{stderr}"
        );
        // nextest says "1 test run" and "2 tests run"; match the part that
        // does not inflect.
        assert!(
            stderr.contains(&format!("run: {expected} passed")),
            "the {tier} tier must run {expected} test(s):\n{stderr}"
        );
        assert!(
            !stderr.contains("Compiling") && !stderr.contains("Finished"),
            "a consumer must not compile:\n{stderr}"
        );
    }

    // And the producer's target tree stayed hidden throughout, so nothing
    // silently fell back to it.
    assert!(!produced.target.exists());
    assert!(hidden.exists());
}

/// Copy `source` to `destination`, skipping the build outputs a second checkout
/// must not inherit.
fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("creating the destination");
    for entry in fs::read_dir(source).expect("reading the source tree") {
        let entry = entry.expect("a directory entry");
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let from = entry.path();
        let to = destination.join(&name);
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).expect("copying a file");
        }
    }
}

#[test]
fn the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it() {
    // Validation checkpoint 3's recipe half: the same three `just` entry points
    // CI calls, in archive mode, over the relocated fixture.
    let dir = Scratch::new("recipes");
    let produced = produce_fixture(dir.path());
    let checkout = dir.path().join("second-checkout");
    copy_tree(&dir.path().join("workspace"), &checkout);
    fs::rename(&produced.target, dir.path().join("target-hidden"))
        .expect("hiding the producer target directory");

    let archive = produced.out.join(&produced.manifest.archive.file);
    let extract = dir.path().join("extracted");
    fs::create_dir_all(&extract).expect("creating the extraction directory");
    let sidecar_dir = produced
        .out
        .join(&produced.manifest.sidecars[0].record.file)
        .parent()
        .expect("the sidecar has a directory")
        .to_path_buf();
    let path = toolchain_free_path(dir.path(), &sidecar_dir);
    let staging = dir.path().join("junit");
    fs::create_dir_all(&staging).expect("creating the staging directory");

    for recipe in ["_test", "_test_l2", "_test_browser"] {
        let output = Command::new("just")
            .arg("--justfile")
            .arg(repo_root().join("justfile"))
            .arg("--working-directory")
            .arg(repo_root())
            .arg(recipe)
            .arg("archive-portability")
            .arg(format!("--archive-file={}", recipe_path(&archive)))
            .arg(format!("--workspace-remap={}", recipe_path(&checkout)))
            .arg(format!("--extract-to={}", recipe_path(&extract)))
            .arg("--extract-overwrite")
            // The standalone driver, because the consumer has no Cargo.
            .env("BISCUIT_NEXTEST_BIN", "cargo-nextest nextest")
            // JUnit staging asks Cargo where the target directory is when it is
            // not told; an archive consumer tells it.
            .env("BISCUIT_JUNIT_TARGET_DIR", &staging)
            .env("BISCUIT_JUNIT_WORKSPACE_ROOT", &checkout)
            .env("PATH", &path)
            .output()
            .expect("running the canonical recipe");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "just {recipe} must run the archive:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("Compiling"),
            "just {recipe} must not compile:\n{stderr}"
        );
    }
}

#[test]
fn an_archive_missing_a_declared_binary_is_refused_before_any_test_starts() {
    // The inventory is the one claim a checksum cannot make: these bytes are
    // intact, and they are an archive of fewer programs than the plan resolved.
    let dir = Scratch::new("inventory-short");
    let produced = produce_fixture(dir.path());
    let mut manifest = produced.manifest.clone();
    manifest
        .test_binaries
        .push("archive-portability::never_built".to_owned());
    manifest.test_binaries.sort();
    manifest.digest = realized_digest(&manifest);
    fs::write(
        &produced.manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .expect("rewriting the manifest");

    let (accepted, verdict) = verify_fixture(
        &produced,
        &["--workspace", &dir.path().join("workspace").to_string_lossy()],
    );
    assert!(!accepted, "{verdict:#}");
    assert_eq!(verdict["inventory_checked"], true);
    assert_eq!(
        verdict["rejections"][0]["code"],
        "build-inventory-incomplete"
    );
    assert!(
        verdict["rejections"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("never_built"),
        "{verdict:#}"
    );
}

#[test]
fn an_archive_of_more_programs_than_the_plan_resolved_is_refused() {
    let dir = Scratch::new("inventory-long");
    let produced = produce_fixture(dir.path());
    let mut manifest = produced.manifest.clone();
    let dropped = manifest
        .test_binaries
        .iter()
        .position(|id| id == "archive-portability::browser_marker")
        .expect("the browser marker is in the inventory");
    manifest.test_binaries.remove(dropped);
    manifest.digest = realized_digest(&manifest);
    fs::write(
        &produced.manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .expect("rewriting the manifest");

    let (accepted, verdict) = verify_fixture(
        &produced,
        &["--workspace", &dir.path().join("workspace").to_string_lossy()],
    );
    assert!(!accepted, "{verdict:#}");
    assert_eq!(
        verdict["rejections"][0]["code"],
        "build-inventory-unexpected"
    );
}

#[test]
fn every_archive_mode_recipe_refuses_to_run_without_a_nextest_driver() {
    // The one fallback that must never exist: `cargo test` cannot consume an
    // archive, so a missing driver has to stop the tier rather than quietly
    // recompile the program under test.
    for recipe in ["_test", "_test_l2", "_test_browser"] {
        let output = Command::new("just")
            .arg("--justfile")
            .arg(repo_root().join("justfile"))
            .arg("--working-directory")
            .arg(repo_root())
            .arg(recipe)
            .arg("archive-portability")
            .arg("--archive-file=/nonexistent/archive.tar.zst")
            .env("BISCUIT_NEXTEST_BIN", "biscuit-no-such-nextest-driver")
            .output()
            .expect("running the canonical recipe");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !output.status.success(),
            "just {recipe} must fail without a driver:\n{stderr}"
        );
        assert!(
            stderr.contains("requires cargo-nextest"),
            "just {recipe} must name the missing driver:\n{stderr}"
        );
        // The banner the recipes print when they take the `cargo test` path.
        // Naming that path in the refusal is fine; entering it is not.
        assert!(
            !stdout.contains("using") || !stdout.contains("cargo test"),
            "just {recipe} must not fall back to a recompiling runner:\n{stdout}"
        );
    }
}

/// An archive path that did not survive its caller is named, not guessed at.
///
/// Found on `build-win-native`, 2026-09-14: a tier recipe pastes `{{ args }}`
/// raw into `forwarded=({{ args }})`, so bash processes backslash escapes and
/// `C:\Users\ken\…\x.tar.zst` reaches nextest as `C:Usersken…x.tar.zst` —
/// which reports only `The system cannot find the file specified`, for a path
/// nobody typed. The recipes refuse first, and say where the spelling comes
/// from.
#[test]
fn an_archive_file_that_names_nothing_is_refused_before_the_tier_starts() {
    for recipe in ["_test", "_test_l2", "_test_browser"] {
        let output = Command::new("just")
            .arg("--justfile")
            .arg(repo_root().join("justfile"))
            .arg("--working-directory")
            .arg(repo_root())
            .arg(recipe)
            .arg("archive-portability")
            .arg("--archive-file=/nonexistent/archive.tar.zst")
            // A real driver: this is the archive path's refusal, not the
            // missing-driver one.
            .env("BISCUIT_NEXTEST_BIN", "cargo-nextest nextest")
            .output()
            .expect("running the canonical recipe");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "just {recipe} must refuse an archive it cannot read:\n{stderr}"
        );
        assert!(
            stderr.contains("--archive-file names no readable file")
                && stderr.contains("/nonexistent/archive.tar.zst"),
            "just {recipe} must name the value it could not read:\n{stderr}"
        );
        assert!(
            !stderr.contains("Compiling"),
            "just {recipe} must not compile a replacement:\n{stderr}"
        );
    }
}

/// `_native_path` answers a spelling a recipe's argument list survives.
///
/// Off Windows that is the path unchanged; on Windows it is the
/// drive-qualified, forward-slash form, with no `\\?\` verbatim prefix — the
/// one spelling both MSYS and Win32 accept. The recipe is itself backslash-safe
/// because its argument is interpolated inside single quotes, so it can be
/// handed a native spelling and asked to fix it.
#[test]
fn the_native_path_helper_answers_a_spelling_a_recipe_can_carry() {
    let dir = Scratch::new("native-path");
    let answer = recipe_path(dir.path());
    assert!(
        !answer.is_empty() && !answer.contains('\\'),
        "a recipe-safe spelling carries no backslash: {answer}"
    );
    assert!(
        !answer.starts_with("\\\\?\\") && !answer.contains("?\\"),
        "a verbatim prefix breaks nextest's own argument grammar: {answer}"
    );
    assert!(
        Path::new(&answer).is_dir(),
        "the answer must still name the same directory: {answer}"
    );
    if cfg!(windows) {
        assert!(
            answer.contains(':'),
            "on Windows the answer is drive-qualified, not an MSYS path: {answer}"
        );
    } else {
        assert_eq!(
            answer,
            dir.path().to_string_lossy(),
            "off Windows the path is already native and is echoed back"
        );
    }
}

// ---------------------------------------------------------------------------
// Owner-leg semantics (fixes/2026-09-12-single-os-compile Tasks 4.1 and 4.2)
// ---------------------------------------------------------------------------

/// The fixture plan, plus one record whose package does not exist in the
/// fixture workspace — so its `cargo nextest archive` fails and nothing else
/// about the run does.
fn plan_with_a_failing_record(dir: &Path, triple: &str) -> PathBuf {
    let path = fixture_plan(dir, triple);
    let mut plan: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("reading the fixture plan"))
            .expect("the fixture plan is JSON");
    let mut broken = plan["builds"][0].clone();
    broken["key"] = json!("dead0000dead0000");
    broken["package"] = json!("no-such-package");
    broken["artifact"] = json!("build-no-such-package-local-host-dead0000dead0000");
    broken["identity"]["package"] = json!("no-such-package");
    plan["builds"]
        .as_array_mut()
        .expect("the plan carries a builds list")
        .push(broken);
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");
    path
}

/// Run `ci-build produce` over the fixture workspace with explicit arguments.
fn produce_with(dir: &Path, plan: &Path, extra: &[&str]) -> std::process::Output {
    Command::new(crate::tests::shipped_wrapper())
        .arg("produce")
        .arg("--plan")
        .arg(plan)
        .arg("--producer")
        .arg("local-host")
        .arg("--out-dir")
        .arg(dir.join("out"))
        .arg("--workspace")
        .arg(dir.join("workspace"))
        .arg("--target-dir")
        .arg(dir.join("target"))
        .arg("--sidecars")
        .arg(fixture_sidecars(dir))
        .args(extra)
        .output()
        .expect("running ci-build produce")
}

/// A leg that fails must not take an unrelated key down with it, and must
/// leave an account of itself where the rollup reads one.
#[test]
fn a_failed_key_still_reports_itself_and_lets_an_unrelated_key_finish() {
    let dir = Scratch::new("partial-owner");
    let triple = host_triple();
    let plan = plan_with_a_failing_record(dir.path(), &triple);

    let output = produce_with(dir.path(), &plan, &[]);
    assert!(
        !output.status.success(),
        "a producer with a failed key must exit non-zero"
    );

    let out = dir.path().join("out");
    let good: Value = serde_json::from_str(
        &fs::read_to_string(
            out.join("build-archive-portability-local-host-0f1e2d3c4b5a6978.status.json"),
        )
        .expect("the successful key must leave a status"),
    )
    .expect("the status is JSON");
    assert_eq!(good["result"], "success");
    assert_eq!(good["stage"], "produce");
    assert_eq!(good["key"], "0f1e2d3c4b5a6978");
    assert_eq!(good["schema_version"], BUILD_STATUS_SCHEMA_VERSION);
    assert!(
        good["digest"].as_str().is_some_and(|value| value.len() == 16),
        "a successful key reports its realized digest: {good}"
    );
    assert!(
        good["timings"]["total_ms"].is_number(),
        "a successful key reports its timings: {good}"
    );
    assert_eq!(
        good["consumers"].as_array().map(Vec::len),
        Some(3),
        "the status carries the cells the record was resolved for"
    );
    assert!(
        out.join("build-archive-portability-local-host-0f1e2d3c4b5a6978.manifest.json").is_file(),
        "the unrelated key must have completed its archive"
    );

    let bad: Value = serde_json::from_str(
        &fs::read_to_string(
            out.join("build-no-such-package-local-host-dead0000dead0000.status.json"),
        )
        .expect("the failed key must leave a status"),
    )
    .expect("the status is JSON");
    assert_eq!(bad["result"], "failure");
    assert_eq!(bad["stage"], "compile");
    assert!(
        bad["detail"].as_str().is_some_and(|value| !value.is_empty()),
        "a failed key must say why: {bad}"
    );
    assert!(bad["digest"].is_null(), "a failed key realized no digest: {bad}");
}

/// The owner matrix expands one leg per record, so a leg produces exactly the
/// record it is named for.
#[test]
fn a_key_narrowed_leg_produces_only_its_own_record() {
    let dir = Scratch::new("one-key");
    let triple = host_triple();
    let plan = plan_with_a_failing_record(dir.path(), &triple);

    let output = produce_with(dir.path(), &plan, &["--key", "0f1e2d3c4b5a6978"]);
    assert!(
        output.status.success(),
        "the healthy key alone must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let out = dir.path().join("out");
    assert!(
        out.join("build-archive-portability-local-host-0f1e2d3c4b5a6978.manifest.json").is_file()
    );
    assert!(
        !out.join("build-no-such-package-local-host-dead0000dead0000.status.json").exists(),
        "a narrowed leg must not touch a sibling key"
    );
}

/// Rewrite one field of the fixture record's identity.
fn plan_with_identity(dir: &Path, triple: &str, field: &str, value: &str) -> PathBuf {
    let path = fixture_plan(dir, triple);
    let mut plan: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("reading the fixture plan"))
            .expect("the fixture plan is JSON");
    plan["builds"][0]["identity"][field] = json!(value);
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");
    path
}

/// The producer's own status document for the fixture record.
fn fixture_status(dir: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(
            dir.join("out")
                .join("build-archive-portability-local-host-0f1e2d3c4b5a6978.status.json"),
        )
        .expect("every attempted key leaves a status"),
    )
    .expect("the status is JSON")
}

/// Compatibility is what the toolchain says, not what the runner label implies.
///
/// A key names the compiler host it was computed for. An owner leg selected by
/// `runs-on` alone would compile a foreign-host record and upload an archive
/// that verifies — the consumer compares against the *planned* key — while the
/// binaries inside cannot run anywhere the plan promised.
#[test]
fn a_record_planned_for_another_compiler_host_is_refused_before_it_compiles() {
    let dir = Scratch::new("preflight-host");
    let triple = host_triple();
    let plan = plan_with_identity(dir.path(), &triple, "host", "s390x-unknown-linux-gnu");

    let output = produce_with(dir.path(), &plan, &[]);
    assert!(
        !output.status.success(),
        "a producer that is not the planned host must exit non-zero"
    );

    let status = fixture_status(dir.path());
    assert_eq!(status["result"], "failure");
    assert_eq!(
        status["stage"], "preflight",
        "the refusal happened before the compile, and the stage must say so: {status}"
    );
    let detail = status["detail"].as_str().unwrap_or_default();
    assert!(
        detail.contains("s390x-unknown-linux-gnu") && detail.contains(&triple),
        "the refusal must name both the planned host and the observed one: {detail}"
    );
    assert!(
        !dir.path()
            .join("out")
            .join("build-archive-portability-local-host-0f1e2d3c4b5a6978.manifest.json")
            .exists(),
        "nothing may be compiled or manifested for a record this toolchain cannot own"
    );
    assert!(
        !dir.path().join("target").exists(),
        "the compile must not have started: no target tree may appear"
    );
}

/// The same question, asked of the target rather than the host.
#[test]
fn a_record_planned_for_a_target_this_toolchain_cannot_build_is_refused() {
    let dir = Scratch::new("preflight-target");
    let triple = host_triple();
    let plan = plan_with_identity(dir.path(), &triple, "target", "nonesuch-unknown-none");

    let output = produce_with(dir.path(), &plan, &[]);
    assert!(!output.status.success());

    let status = fixture_status(dir.path());
    assert_eq!(status["result"], "failure");
    assert_eq!(status["stage"], "preflight");
    assert!(
        status["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("nonesuch-unknown-none"),
        "the refusal must name the target it could not build for: {status}"
    );
}

/// A preflight refusal is attributable to its own key, exactly as a compile
/// failure is: the owner matrix runs one leg per record, and `fail-fast: false`
/// only means something if a sibling key can still finish.
#[test]
fn a_preflight_refusal_leaves_an_unrelated_key_free_to_finish() {
    let dir = Scratch::new("preflight-sibling");
    let triple = host_triple();
    let path = plan_with_a_failing_record(dir.path(), &triple);
    let mut plan: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("reading the plan"))
            .expect("the plan is JSON");
    // The record that would otherwise fail at compile now fails at preflight,
    // leaving the healthy fixture record as the only one that may proceed.
    plan["builds"][1]["identity"]["host"] = json!("s390x-unknown-linux-gnu");
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");

    let output = produce_with(dir.path(), &path, &[]);
    assert!(!output.status.success());

    let healthy = fixture_status(dir.path());
    assert_eq!(healthy["result"], "success");
    assert_eq!(healthy["stage"], "produce");

    let refused: Value = serde_json::from_str(
        &fs::read_to_string(
            dir.path()
                .join("out")
                .join("build-no-such-package-local-host-dead0000dead0000.status.json"),
        )
        .expect("the refused key must leave a status"),
    )
    .expect("the status is JSON");
    assert_eq!(refused["stage"], "preflight");
    assert!(refused["digest"].is_null());
}

/// A key the plan does not own is a disagreement between the owner matrix and
/// the plan, and the producer refuses rather than silently building nothing.
#[test]
fn a_key_this_producer_does_not_own_is_refused() {
    let dir = Scratch::new("unknown-key");
    let triple = host_triple();
    let plan = fixture_plan(dir.path(), &triple);

    let output = produce_with(dir.path(), &plan, &["--key", "ffffffffffffffff"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ffffffffffffffff") && stderr.contains("disagree"),
        "the refusal must name the key and the disagreement: {stderr}"
    );
}

/// The producer records where it compiled, and that claim is inside the
/// realized digest — so a consumer that relocates by it cannot be handed an
/// edited path.
#[test]
fn the_manifest_records_the_workspace_the_producer_compiled_at() {
    let dir = Scratch::new("producer-workspace");
    let produced = produce_fixture(dir.path());
    let expected = canonical_path(&dir.path().join("workspace"))
        .expect("the fixture workspace resolves")
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    assert_eq!(produced.manifest.producer_workspace, expected);

    let mut edited = produced.manifest.clone();
    edited.producer_workspace = "/somewhere/else".to_owned();
    assert_ne!(
        realized_digest(&edited),
        produced.manifest.digest,
        "the recorded workspace must be covered by the realized digest"
    );
}

// ---------------------------------------------------------------------------
// Shared dependency work, without feature unification
// (fixes/2026-09-12-single-os-compile Task 6.1)
// ---------------------------------------------------------------------------
//
// `scripts/ci/fixtures/shared-deps` is a four-member workspace: two packages
// under test over two dependencies, one configured identically by both and one
// deliberately not. Producing both records into a single owner target tree is
// what the specification's "consolidate dependency work" claim reduces to, and
// the compiler-work counter is the only witness that can tell genuine reuse
// from a warm cache.

fn shared_deps_workspace() -> PathBuf {
    repo_root().join("scripts/ci/fixtures/shared-deps")
}

const ALPHA_KEY: &str = "a1a1a1a1a1a1a1a1";
const BETA_KEY: &str = "b2b2b2b2b2b2b2b2";

fn shared_deps_artifact(package: &str, key: &str) -> String {
    format!("build-{package}-local-host-{key}")
}

/// A plan naming one build record per fixture package, both owned by this host.
fn shared_deps_plan(dir: &Path, triple: &str) -> PathBuf {
    let commit = fixture_checkout(dir, &shared_deps_workspace());
    let (arch, abi, libc) = host_runtime();
    let record = |package: &str, key: &str| {
        json!({
            "key": key,
            "package": package,
            "producer": "local-host",
            "artifact": shared_deps_artifact(package, key),
            "compatible_environments": ["local-host"],
            "compatibility_reason": "the fixture's producer is its only consumer",
            "consumers": [{"environment": "local-host", "gate": "L1"}],
            "identity": {
                "source_commit": commit,
                "lockfile": "0000000000000000",
                "rust": "1.97.1",
                "nextest": "latest",
                "host": triple,
                "target": triple,
                "profile": "test",
                "rustflags": "",
                "cargo_config": [],
                "linker": if cfg!(windows) { "link.exe" } else { "cc" },
                "archive_format": "tar.zst",
                "package": package,
                "target_kinds": ["lib", "test"],
                "features": "",
                "native": [],
                "archive_includes": [],
                "sidecars": [],
            },
        })
    };
    let plan = json!({
        "head": commit,
        "environments": [{
            "name": "local-host",
            "build": {"runtime": {
                "arch": arch, "abi": abi, "libc": libc, "native_libraries": [],
            }},
        }],
        "builds": [
            record("shared-deps-alpha", ALPHA_KEY),
            record("shared-deps-beta", BETA_KEY),
        ],
    });
    let path = dir.join("shared-deps-plan.json");
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");
    path
}

/// A sidecar table with no entries: this fixture declares none, and handing the
/// producer the repository's own table would couple it to packages that have
/// nothing to do with it.
fn empty_sidecars(dir: &Path) -> PathBuf {
    let path = dir.join("shared-deps-sidecars.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": SIDECAR_SCHEMA_VERSION,
            "sidecars": {},
        }))
        .unwrap(),
    )
    .expect("writing the sidecar table");
    path
}

struct SharedDeps {
    out: PathBuf,
    counters: PathBuf,
    target: PathBuf,
}

impl SharedDeps {
    fn manifest(&self, package: &str, key: &str) -> Manifest {
        read_manifest(
            &self
                .out
                .join(format!("{}.manifest.json", shared_deps_artifact(package, key))),
        )
        .expect("the producer's own manifest must parse")
    }

    fn archive(&self, package: &str, key: &str) -> PathBuf {
        self.out
            .join(format!("{}.tar.zst", shared_deps_artifact(package, key)))
    }
}

/// Produce both fixture records, measured, into one owner target tree.
fn produce_shared_deps(dir: &Path) -> SharedDeps {
    let binary = crate::tests::shipped_wrapper();
    let plan = shared_deps_plan(dir, &host_triple());
    let out = dir.join("build");
    let counters = dir.join("build-counters");
    let target = dir.join("owner-target");

    let output = Command::new("bash")
        .arg(repo_root().join("scripts/ci/produce-owner.sh"))
        .env("CI_BUILD_BIN", &binary)
        .env("CI_BUILD_PLAN", &plan)
        .env("PRODUCER", "local-host")
        .env("RUNNER_TEMP", dir)
        .env("MEASURE", "true")
        .arg("--workspace")
        .arg(dir.join("workspace"))
        .arg("--target-dir")
        .arg(&target)
        .arg("--sidecars")
        .arg(empty_sidecars(dir))
        .arg("--json")
        .output()
        .expect("running ci-build produce");
    assert!(
        output.status.success(),
        "producing both fixture records failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    SharedDeps {
        out,
        counters,
        target,
    }
}

/// How many times rustc compiled each crate across the recorded slices.
///
/// Probes are excluded — Cargo asks a wrapper for `rustc -vV` and that is not
/// compiler work — and so are the anonymous `___` target-info probes Cargo
/// compiles from stdin, which belong to no package in the graph.
fn compiles_by_crate(counters: &Path, artifacts: &[String]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for artifact in artifacts {
        let dir = counters.join(artifact);
        assert!(
            dir.is_dir(),
            "a measured record must leave its own counter slice at {}",
            dir.display()
        );
        for event in crate::read_events(&dir).expect("reading the recorded events") {
            if event.probe {
                continue;
            }
            let Some(name) = event.crate_name.filter(|name| !name.starts_with('_')) else {
                continue;
            };
            *counts.entry(name).or_insert(0) += 1;
        }
    }
    counts
}

/// The identical dependency compiles once, the divergent one twice, and neither
/// package's tests see the other's feature graph.
#[test]
fn one_owner_tree_shares_a_dependency_compile_without_unifying_features() {
    let dir = Scratch::new("shared-deps");
    let produced = produce_shared_deps(dir.path());

    let counts = compiles_by_crate(
        &produced.counters,
        &[
            shared_deps_artifact("shared-deps-alpha", ALPHA_KEY),
            shared_deps_artifact("shared-deps-beta", BETA_KEY),
        ],
    );

    assert_eq!(
        counts.get("shared_deps_common").copied(),
        Some(1),
        "the identically configured dependency must compile exactly once across \
         both records: {counts:?}"
    );
    assert_eq!(
        counts.get("shared_deps_divergent").copied(),
        Some(2),
        "the divergently configured dependency is two units and must compile \
         twice; one compile would mean the feature graphs were unified: {counts:?}"
    );
    assert!(
        counts.contains_key("shared_deps_alpha") && counts.contains_key("shared_deps_beta"),
        "both packages under test must have been compiled: {counts:?}"
    );

    // Each record's manifest reports only its own slice, under the same
    // `{package, configuration}` label shape the tier cells publish.
    for (package, key) in [
        ("shared-deps-alpha", ALPHA_KEY),
        ("shared-deps-beta", BETA_KEY),
    ] {
        let manifest = produced.manifest(package, key);
        let work = manifest
            .compiler_work
            .as_ref()
            .unwrap_or_else(|| panic!("{package} was measured and must record counts"));
        let slices = work["slices"].as_array().expect("slices");
        assert_eq!(slices.len(), 1, "one record, one measured label: {work}");
        assert_eq!(slices[0]["package"], package);
        assert_eq!(slices[0]["configuration"], "build local-host");
        assert!(
            slices[0]["compiler_invocations"].as_u64().unwrap() > 0,
            "{package} compiled something: {work}"
        );
    }

    // The beta record compiled fewer crates than alpha did, because it reused
    // alpha's `shared-deps-common`. That is the consolidation claim, measured
    // rather than inferred from elapsed time.
    let alpha_crates = produced.manifest("shared-deps-alpha", ALPHA_KEY).compiler_work
        .as_ref()
        .and_then(|work| work["slices"][0]["distinct_crates"].as_u64())
        .expect("alpha records distinct crates");
    let beta_crates = produced.manifest("shared-deps-beta", BETA_KEY).compiler_work
        .as_ref()
        .and_then(|work| work["slices"][0]["distinct_crates"].as_u64())
        .expect("beta records distinct crates");
    assert!(
        beta_crates < alpha_crates,
        "beta must have reused a dependency alpha already compiled \
         (alpha {alpha_crates} crates, beta {beta_crates})"
    );

    // And each archive, run on its own, observes the feature graph its package
    // declared: the fixture's assertions are the observation.
    for (package, key, expected) in [
        ("shared-deps-alpha", ALPHA_KEY, 2_usize),
        ("shared-deps-beta", BETA_KEY, 2),
    ] {
        let extract = dir.path().join(format!("extract-{package}"));
        fs::create_dir_all(&extract).expect("creating the extraction directory");
        let output = Command::new("cargo-nextest")
            .arg("nextest")
            .arg("run")
            .arg("--archive-file")
            .arg(produced.archive(package, key))
            .arg("--workspace-remap")
            .arg(dir.path().join("workspace"))
            .arg("--extract-to")
            .arg(&extract)
            .arg("--extract-overwrite")
            .arg("--no-tests=fail")
            .current_dir(dir.path())
            .output()
            .expect("running the archive");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "{package}'s archive must observe its own feature graph:\n{stderr}"
        );
        assert!(
            stderr.contains(&format!("run: {expected} passed")),
            "{package} must run {expected} tests:\n{stderr}"
        );
    }

    assert!(
        produced.target.is_dir(),
        "both records compiled into one owner target tree"
    );
}

/// The producer keeps one Cargo invocation per package because a combined one
/// is *not* equivalent.
///
/// Task 6.1 permits merging the invocations only if this fixture stays green
/// under a combined build. It does not: Cargo unifies features across the
/// selected packages, `shared-deps-beta` links a `shared-deps-divergent` it
/// never asked for, and its own assertion catches it. Pinning that here is what
/// keeps a later "one invocation is faster" change from being made silently.
#[test]
fn a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart() {
    let dir = Scratch::new("shared-deps-combined");
    let output = Command::new("cargo")
        .arg("nextest")
        .arg("run")
        .arg("--manifest-path")
        .arg(shared_deps_workspace().join("Cargo.toml"))
        .arg("--workspace")
        .arg("--no-fail-fast")
        .env("CARGO_TARGET_DIR", dir.path().join("combined-target"))
        .output()
        .expect("running the fixture workspace in one invocation");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a combined invocation must not be equivalent to two separate ones:\n{stderr}"
    );
    assert!(
        stderr.contains("beta_observes_no_feature_it_did_not_ask_for"),
        "the failure must be the feature-unification assertion, not something \
         else that happens to be red:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// The shipped planner's own document
// ---------------------------------------------------------------------------

/// Every fixture above hand-writes its plan, so a field whose *type* differs
/// between `scripts/ci/schema.py` and [`Identity`] passes all of them and fails
/// on the first real run. `identity.features` did exactly that: the planner
/// writes one command-line fragment (`--features a,b`, `--all-features`, or
/// empty) and this module declared a list, so `ci-build produce --plan
/// <resolved-plan.json>` — the only way CI ever calls it — could not read a
/// single record.
///
/// This reads the planner's actual output for every package in the workspace
/// and round-trips each record, which is the one check no hand-written fixture
/// can make.
#[test]
fn every_build_record_the_shipped_planner_writes_round_trips_through_this_reader() {
    let dir = Scratch::new("planner-corpus");
    let plan_path = dir.path().join("resolved-plan.json");
    let output = Command::new(python())
        .arg(repo_root().join("scripts/ci/affected_scope.py"))
        .arg("--all")
        .arg("--resolved-plan")
        .current_dir(repo_root())
        .output()
        .expect("running the shipped planner");
    assert!(
        output.status.success(),
        "the planner must resolve a plan: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(&plan_path, &output.stdout).expect("writing the resolved plan");

    let plan = read_plan(&plan_path).expect("the producer must read the planner's own document");
    assert!(
        plan.builds.len() > 20,
        "a whole-workspace plan owns a record per package and producer, got {}",
        plan.builds.len()
    );

    // Round trip: `deny_unknown_fields` already refuses a field this reader
    // does not know, and comparing the re-serialized record to the planner's
    // own object catches the opposite — a field read as the wrong shape, or
    // dropped.
    let document: Value =
        serde_json::from_slice(&output.stdout).expect("the resolved plan is JSON");
    let written = document["builds"].as_array().expect("the plan owns builds");
    assert_eq!(written.len(), plan.builds.len());
    for (source, record) in written.iter().zip(&plan.builds) {
        assert_eq!(
            source,
            &serde_json::to_value(record).expect("a record re-serializes"),
            "build {} did not survive the round trip",
            record.key
        );
    }

    // And every producer the planner names is one this tool could actually be
    // running as — a record owned by an environment with no build contract
    // would schedule a leg that can never compile.
    for record in &plan.builds {
        assert!(
            ["ubuntu-latest", "macos-latest", "windows-latest"].contains(&record.producer.as_str()),
            "{} names producer {}",
            record.key,
            record.producer
        );
        assert!(
            plan.runtimes.contains_key(&record.producer),
            "the plan must carry {}'s runtime predicates",
            record.producer
        );
    }
}

/// `python3` where it exists, `python` where only that name is installed.
///
/// The Windows runners and `build-win-native` ship the launcher as `python`.
fn python() -> String {
    for candidate in ["python3", "python"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return candidate.to_owned();
        }
    }
    panic!("python3 is required: it runs `scripts/ci/affected_scope.py`, the scheduling authority");
}

// ---------------------------------------------------------------------------
// The declarative inventory, closed against the real workspace
// (fixes/2026-09-12-single-os-compile Task 6.4)
// ---------------------------------------------------------------------------

/// Every sidecar the table offers is declared by a real package, and every
/// binary it names is a real Cargo target.
///
/// The table is a vocabulary, and a vocabulary nothing uses is a compile the
/// producer performs for no reader — while a binary it names but no manifest
/// declares is a producer failure minutes into a run, in a leg that had
/// nothing to do with the package that mis-declared it.
#[test]
fn every_shipped_sidecar_is_declared_by_a_package_and_names_real_binaries() {
    let root = repo_root();
    let sidecars = read_sidecars(&root.join(".github/ci/sidecars.json"))
        .expect("the shipped sidecar table must parse");

    let declarations = workspace_manifests(&root);
    for (name, spec) in &sidecars {
        assert!(
            declarations
                .iter()
                .any(|manifest| manifest.declares_sidecar(name)),
            "sidecar '{name}' is in the table and no package declares it; an \
             undeclared sidecar is compiled for no reader"
        );
        let manifest = declarations
            .iter()
            .find(|manifest| manifest.package == spec.package)
            .unwrap_or_else(|| {
                panic!("sidecar '{name}' names package '{}', which is not in this workspace", spec.package)
            });
        for bin in &spec.bins {
            assert!(
                manifest.bins.contains(bin),
                "sidecar '{name}' names binary '{bin}', which package '{}' does not declare",
                spec.package
            );
        }
    }
}

/// One workspace manifest, read for the two questions this audit asks of it.
struct WorkspaceManifest {
    package: String,
    bins: BTreeSet<String>,
    sidecars: BTreeSet<String>,
}

impl WorkspaceManifest {
    fn declares_sidecar(&self, name: &str) -> bool {
        self.sidecars.contains(name)
    }
}

/// Every declared archive include and sidecar in the planner's own plan is one
/// the producer can act on.
///
/// The planner validates each package's declaration in isolation; this asks the
/// opposite question — of the records that would actually be built, does the
/// producer have everything it needs to build them.
#[test]
fn every_real_build_record_declares_only_payloads_the_producer_can_emit() {
    let dir = Scratch::new("inventory-corpus");
    let plan_path = dir.path().join("resolved-plan.json");
    let output = Command::new(python())
        .arg(repo_root().join("scripts/ci/affected_scope.py"))
        .arg("--all")
        .arg("--resolved-plan")
        .current_dir(repo_root())
        .output()
        .expect("running the shipped planner");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fs::write(&plan_path, &output.stdout).expect("writing the resolved plan");
    let plan = read_plan(&plan_path).expect("the planner's own document");
    let sidecars = read_sidecars(&repo_root().join(".github/ci/sidecars.json"))
        .expect("the shipped sidecar table");

    let mut saw_sidecar = false;
    let mut saw_include = false;
    for record in &plan.builds {
        for name in &record.identity.sidecars {
            saw_sidecar = true;
            assert!(
                sidecars.contains_key(name),
                "{} declares sidecar '{name}', which the table does not offer",
                record.package
            );
        }
        for entry in &record.identity.archive_includes {
            saw_include = true;
            let path = include_path(&record.identity, entry);
            assert!(
                !path.contains('{') && !path.contains('}'),
                "{}'s include {entry} left an unexpanded placeholder: {path}",
                record.package
            );
            assert!(
                path.starts_with(&record.identity.target),
                "{}'s include must be spelled under the producer's own triple: {path}",
                record.package
            );
        }
    }
    assert!(saw_sidecar, "the corpus must exercise the sidecar class");
    assert!(saw_include, "the corpus must exercise the archive-include class");
}

/// Every workspace manifest's text.
///
/// `cargo metadata` over the outer workspace from inside `scripts/` would be
/// both slow and a layering violation. The root `members` list is most of the
/// answer; the rest are crates that join the workspace only by being a path
/// dependency of a member — `biscuit-test-harness` is one, and it owns a
/// sidecar — so every top-level directory holding a manifest is read too.
fn workspace_manifests(root: &Path) -> Vec<WorkspaceManifest> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest");
    let mut paths: BTreeSet<PathBuf> = manifest
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split('"').next())
        .map(|member| root.join(member).join("Cargo.toml"))
        .collect();
    for entry in fs::read_dir(root).expect("reading the repository root").flatten() {
        let candidate = entry.path().join("Cargo.toml");
        if candidate.is_file() {
            paths.insert(candidate);
        }
    }
    paths
        .into_iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .filter_map(|text| text.parse::<toml::Table>().ok())
        .filter_map(|document| {
            // Parsed rather than grepped: `messenger/cli` declares a BIN named
            // `messenger`, so a line-wise `name = "messenger"` match finds the
            // wrong manifest and reports the right package's binaries missing.
            let package = document
                .get("package")?
                .get("name")?
                .as_str()?
                .to_owned();
            let bins = document
                .get("bin")
                .and_then(toml::Value::as_array)
                .map(|targets| {
                    targets
                        .iter()
                        .filter_map(|target| target.get("name")?.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let sidecars = document
                .get("package")?
                .get("metadata")
                .and_then(|value| value.get("ci"))
                .and_then(|value| value.get("tests"))
                .and_then(|value| value.get("sidecars"))
                .and_then(toml::Value::as_array)
                .map(|names| {
                    names
                        .iter()
                        .filter_map(|name| name.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            Some(WorkspaceManifest { package, bins, sidecars })
        })
        .collect()
}

/// Which declared includes name an example target the producer has to build.
///
/// Cargo has no nested example targets, so only a direct child of `examples/`
/// qualifies; anything deeper is some other build output that happens to live
/// there, and building it as an example would fail on a target that does not
/// exist.
#[test]
fn only_a_direct_examples_child_is_treated_as_a_target_to_build() {
    assert_eq!(
        example_target("examples/discovery_probe"),
        Some("discovery_probe".to_owned())
    );
    assert_eq!(
        example_target("examples/discovery_probe{EXE_SUFFIX}"),
        Some("discovery_probe".to_owned()),
        "the placeholder is expanded before the name is read"
    );
    for entry in [
        "{DLL_PREFIX}libfoo{DLL_SUFFIX}",
        "tools/probe",
        "examples/nested/probe",
        "examples/",
    ] {
        assert_eq!(example_target(entry), None, "{entry}");
    }
}

/// A declared example the package does not have is a broken declaration, and
/// the producer must say so rather than archive without it.
#[test]
fn an_example_include_naming_no_target_fails_the_record_at_compile() {
    let dir = Scratch::new("missing-example");
    let triple = host_triple();
    let path = fixture_plan(dir.path(), &triple);
    let mut plan: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("reading the fixture plan"))
            .expect("the fixture plan is JSON");
    plan["builds"][0]["identity"]["archive_includes"] = json!(["examples/no-such-example"]);
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");

    let output = produce_with(dir.path(), &path, &[]);
    assert!(!output.status.success());
    let status = fixture_status(dir.path());
    assert_eq!(status["result"], "failure");
    assert_eq!(status["stage"], "compile");
    assert!(
        status["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("no-such-example"),
        "the refusal must name the example it could not build: {status}"
    );
}

#[test]
fn tracked_source_changes_and_forged_tree_labels_are_refused_before_compilation() {
    let dir = Scratch::new("source-provenance");
    let plan = fixture_plan(dir.path(), &host_triple());
    let source = dir.path().join("workspace/crate/src/lib.rs");
    let original = fs::read_to_string(&source).unwrap();
    fs::write(&source, format!("{original}\n// tracked modification\n")).unwrap();
    let output = produce_with(dir.path(), &plan, &["--source-tree", "forged-tree"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("build-source-mismatch"));
    assert!(!dir.path().join("target").exists());
    fs::write(&source, original).unwrap();
    let output = produce_with(dir.path(), &plan, &["--source-tree", "forged-tree"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--source-tree"));
    assert!(!dir.path().join("target").exists());
}

/// The hosted pull-request boundary, built out of real commits.
///
/// GitHub checks a `pull_request` workflow out at the MERGE revision by
/// default, while the plan is labelled with the PR HEAD. The two are different
/// commits with different trees, so whichever one the plan names, only that one
/// can produce. This fixture builds that divergence and proves the refusal is
/// what decides it — which is why every checkout in the CI chain is pinned to
/// `plan.head` rather than left on the default merge ref.
#[test]
fn produce_accepts_the_planned_head_and_refuses_the_merge_revision() {
    let dir = Scratch::new("pr-merge-identity");
    let workspace = dir.path().join("workspace");
    let plan_path = fixture_plan(dir.path(), &host_triple());
    let base = fixture_git(&workspace, &["rev-parse", "HEAD"]);

    // The PR head: one commit off the base, carrying the tested change.
    fixture_git(&workspace, &["checkout", "--quiet", "-b", "feature"]);
    let source = workspace.join("crate/src/lib.rs");
    let original = fs::read_to_string(&source).expect("reading the fixture crate");
    fs::write(&source, format!("{original}\n// pull-request change\n")).unwrap();
    fixture_git(&workspace, &["commit", "--quiet", "-am", "pull-request change"]);
    let head = fixture_git(&workspace, &["rev-parse", "HEAD"]);

    // The base branch moves on, and GitHub's merge ref is the result.
    fixture_git(&workspace, &["checkout", "--quiet", "-B", "base", &base]);
    fs::write(workspace.join("BASE-MOVED.txt"), "base advanced\n").unwrap();
    fixture_git(&workspace, &["add", "-A"]);
    fixture_git(&workspace, &["commit", "--quiet", "-m", "base advanced"]);
    fixture_git(&workspace, &["merge", "--quiet", "--no-ff", "-m", "merge", "feature"]);
    let merge = fixture_git(&workspace, &["rev-parse", "HEAD"]);
    assert_ne!(merge, head, "the merge revision is not the pull-request head");

    let mut plan: Value = serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    plan["head"] = json!(head);
    plan["builds"][0]["identity"]["source_commit"] = json!(head);
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();

    // The default merge checkout: refused before anything is compiled.
    let output = produce_with(dir.path(), &plan_path, &[]);
    assert!(!output.status.success(), "the merge revision is not the planned head");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(stderr.contains("build-source-mismatch"), "{stderr}");
    assert!(stderr.contains(&head), "the refusal must name the planned commit: {stderr}");
    assert!(!dir.path().join("target").exists(), "nothing may compile before the verdict");

    // The pinned checkout: the same plan, the same producer, and it produces.
    fixture_git(&workspace, &["checkout", "--quiet", "--detach", &head]);
    let output = produce_with(dir.path(), &plan_path, &[]);
    assert!(
        output.status.success(),
        "the pinned head must produce: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(
            dir.path()
                .join("out")
                .join("build-archive-portability-local-host-0f1e2d3c4b5a6978.manifest.json"),
        )
        .expect("reading the produced manifest"),
    )
    .expect("the manifest is JSON");
    assert_eq!(manifest["source_commit"], json!(head));
}

#[test]
fn consumer_refuses_a_different_source_tree_even_with_a_recomputed_manifest_digest() {
    let dir = Scratch::new("consumer-source");
    let produced = produce_fixture(dir.path());
    let mut manifest = produced.manifest.clone();
    manifest.source_tree = "forged-tree".into();
    manifest.digest = realized_digest(&manifest);
    let mut opts = options(&produced.out, "local-host");
    opts.workspace = Some(dir.path().join("workspace"));
    let rejections = verify(&manifest, &opts, Some(&read_plan(&produced.plan).unwrap()), host_runtime());
    assert!(codes(&rejections).contains(&"build-source-mismatch"));
}

#[cfg(unix)]
#[test]
fn a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed() {
    let dir = Scratch::new("external-native");
    fixture_checkout(dir.path(), &fixture_workspace());
    let workspace = dir.path().join("workspace");
    let native = dir.path().join("native");
    fs::create_dir(&native).unwrap();
    let source = native.join("native.c");
    fs::write(&source, "int review_native(void) { return 42; }\n").unwrap();
    let library = native.join(if cfg!(target_os = "macos") { "libreview_native.dylib" } else { "libreview_native.so" });
    let mut cc = Command::new("cc");
    if cfg!(target_os = "macos") { cc.args(["-dynamiclib", "-install_name"]).arg(&library); }
    else { cc.args(["-shared", "-fPIC"]); }
    let output = cc.arg(&source).arg("-o").arg(&library).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let test = workspace.join("crate/tests/native.rs");
    fs::write(&test, r#"#[link(name = "review_native")]
unsafe extern "C" { fn review_native() -> i32; }
#[test] fn linked_native() { assert_eq!(unsafe { review_native() }, 42); }
"#).unwrap();
    fixture_git(&workspace, &["add", "-A"]);
    fixture_git(&workspace, &["commit", "--quiet", "-m", "native dependency"]);
    let plan_path = fixture_plan(dir.path(), &host_triple());
    let mut plan: Value = serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    plan["builds"][0]["identity"]["rustflags"] = json!(format!("-Lnative={} -Clink-arg=-Wl,-rpath,{}", native.display(), native.display()));
    fs::write(&plan_path, serde_json::to_string(&plan).unwrap()).unwrap();
    let output = produce_with(dir.path(), &plan_path, &[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let out = dir.path().join("out");
    let manifest_path = out.join("build-archive-portability-local-host-0f1e2d3c4b5a6978.manifest.json");
    let manifest = read_manifest(&manifest_path).unwrap();
    assert!(manifest.realized.runtime.native_libraries.iter().any(|name| name.contains("review_native")), "{:?}", manifest.realized.runtime);
    assert_ne!(manifest.realized.linker, manifest.identity.linker);
    let mut opts = options(&out, "local-host");
    opts.manifest = manifest_path;
    opts.plan = Some(plan_path);
    opts.workspace = Some(workspace);
    assert!(run_verify(&opts, &Terminal::default()).unwrap());
    fs::rename(&library, native.join("hidden-library")).unwrap();
    assert!(!run_verify(&opts, &Terminal::default()).unwrap());
}

// ---------------------------------------------------------------------------
// The real package corpus, relocated
// ---------------------------------------------------------------------------
//
// The synthetic fixture above proves the *plumbing* relocates. It cannot prove
// that the repository's own tests do: they were written against a checkout that
// was always where they were compiled, and every native consumer happens to
// receive the same layout as its producer, which hides a baked path until the
// WSL2 guest — the one consumer at a genuinely different address — runs them.
//
// So this fixture takes REAL packages through the same producer, then deletes
// the producer's checkout and its target directory outright. A test that still
// reads `env!("CARGO_MANIFEST_DIR")` has nowhere to read it from and fails here,
// on the developer's machine, instead of on a Windows runner.
// `tools/test-toolkit/tests/archive_path_guard.rs` keeps new ones from landing;
// this is the end-to-end proof that the guard's rule is the right rule.

/// Trees a relocation producer must not inherit from the live checkout.
///
/// `assets` is 440 MB of images no package under test reads, and copying it
/// twice dominates the fixture's wall clock; the rest are build or VCS state
/// that a fresh `git init` replaces.
const UNCOPIED: &[&str] = &["target", ".git", "assets", "node_modules"];

/// Copy the repository's WORKING tree — not its committed state — into
/// `destination`.
///
/// The working tree is the point: a migration that has not been committed yet is
/// exactly what this fixture has to test, so a `git worktree` or `git clone` of
/// `HEAD` would silently prove the wrong revision relocates.
fn copy_repository(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("creating the relocation checkout");
    for entry in fs::read_dir(source).expect("reading the repository") {
        let entry = entry.expect("a repository entry");
        let name = entry.file_name();
        if UNCOPIED.iter().any(|skip| *skip == name) {
            continue;
        }
        let from = entry.path();
        let to = destination.join(&name);
        // Skipped rather than dereferenced: `fs::copy` would turn `AGENTS.md`
        // into a second copy of `CLAUDE.md`, and a symlink into `target` would
        // undo the exclusion above. No package this fixture relocates reads one.
        if from.is_symlink() {
            continue;
        }
        if from.is_dir() {
            copy_repository(&from, &to);
        } else {
            fs::copy(&from, &to).expect("copying a repository file");
        }
    }
}

/// A resolved plan naming one real package, built by this host for this host.
fn real_plan(dir: &Path, workspace: &Path, package: &str, triple: &str) -> PathBuf {
    let commit = fixture_git(workspace, &["rev-parse", "HEAD"]);
    let (arch, abi, libc) = host_runtime();
    let identity = json!({
        "source_commit": commit,
        "lockfile": "0000000000000000",
        "rust": "1.97.1",
        "nextest": "latest",
        "host": triple,
        "target": triple,
        "profile": "test",
        "rustflags": "",
        "cargo_config": [],
        "linker": if cfg!(windows) { "link.exe" } else { "cc" },
        "archive_format": "tar.zst",
        "package": package,
        "target_kinds": ["lib", "test"],
        "features": "",
        "native": [],
        "archive_includes": [],
        "sidecars": [],
    });
    let plan = json!({
        "head": commit,
        "environments": [{
            "name": "local-host",
            "build": {"runtime": {
                "arch": arch, "abi": abi, "libc": libc, "native_libraries": [],
            }},
        }],
        "builds": [{
            "key": "1122334455667788",
            "package": package,
            "producer": "local-host",
            "artifact": format!("build-{package}-local-host-1122334455667788"),
            "compatible_environments": ["local-host"],
            "compatibility_reason": "this host produces and consumes its own relocation proof",
            "consumers": [{"environment": "local-host", "gate": "L1"}],
            "identity": identity,
        }],
    });
    let path = dir.join("plan.json");
    fs::write(&path, serde_json::to_string_pretty(&plan).unwrap()).expect("writing the plan");
    path
}

/// Produce every `package`'s archive from ONE throwaway checkout, then take
/// that checkout and its target directory away.
///
/// One producer serves them all deliberately. The checkout is a file-by-file
/// copy of this repository and the target directory is a cold build, so doing
/// either per package is the whole cost of this fixture; sharing them lets the
/// packages' common dependencies compile once. What may not be shared is the
/// *teardown*: every archive is produced and every consumer cloned before the
/// producer is removed, so each archive still runs with nothing of its producer
/// left, which is the property under test.
///
/// ## Returns
///
/// One archive per package, paired with the consumer checkout it must be run
/// against — at an address the producer never wrote to.
fn relocate_real_packages(dir: &Path, packages: &[&str]) -> Vec<(PathBuf, PathBuf)> {
    let binary = crate::tests::shipped_wrapper();
    let triple = host_triple();

    // `source_tree` refuses a dirty or non-git workspace, so the producer is a
    // real repository — just not this one.
    let producer = dir.join("producer");
    copy_repository(&repo_root(), &producer);
    fixture_git(&producer, &["init", "--quiet"]);
    fixture_git(&producer, &["add", "-A"]);
    fixture_git(&producer, &["commit", "--quiet", "-m", "relocation fixture"]);

    let out = dir.join("out");
    let target = dir.join("target");
    let mut produced = Vec::with_capacity(packages.len());

    for package in packages {
        let plan = real_plan(dir, &producer, package, &triple);
        let output = Command::new(&binary)
            .arg("produce")
            .arg("--plan")
            .arg(&plan)
            .arg("--producer")
            .arg("local-host")
            .arg("--out-dir")
            .arg(&out)
            .arg("--workspace")
            .arg(&producer)
            .arg("--target-dir")
            .arg(&target)
            .arg("--json")
            .output()
            .expect("running ci-build produce");
        assert!(
            output.status.success(),
            "producing {package} failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let manifest_path =
            out.join(format!("build-{package}-local-host-1122334455667788.manifest.json"));
        let manifest = read_manifest(&manifest_path).expect("the producer's own manifest must parse");
        let archive = out.join(&manifest.archive.file);

        // The consumer's checkout, at a path of its own. `git clone`, not a file
        // copy: it is what the WSL2 guest does, and a checkout without a `.git`
        // would fail tests that ask whether they are inside a repository — a
        // fixture artifact, not a relocation defect. A local clone hardlinks the
        // object store, so the producer's removal below cannot take it away.
        let consumer = dir.join("elsewhere").join(package);
        fs::create_dir_all(consumer.parent().expect("the consumer has a parent"))
            .expect("creating the consumer's directory");
        let cloned = Command::new("git")
            .args(["-c", "gc.auto=0"])
            .arg("clone")
            .arg("--quiet")
            .arg(&producer)
            .arg(&consumer)
            .output()
            .expect("git must be present to clone the consumer checkout");
        assert!(
            cloned.status.success(),
            "cloning the consumer checkout: {}",
            String::from_utf8_lossy(&cloned.stderr)
        );

        produced.push((archive, consumer));
    }

    // Both halves of the producer, gone. DELETED rather than renamed aside: a
    // baked `CARGO_MANIFEST_DIR` or `CARGO_BIN_EXE_…` must resolve to nothing,
    // not to a tree that still happens to hold what it wanted. It also returns
    // every package's dependency build to the disk before the archived suites
    // start, which is what keeps this fixture affordable.
    remove_tree(&producer);
    remove_tree(&target);
    assert!(!producer.exists(), "the producer checkout must be gone");
    assert!(!target.exists(), "the producer target directory must be gone");

    produced
}

/// Run `package`'s archived L1 suite against `consumer`, with nothing of the
/// producer left to fall back to.
fn run_relocated(dir: &Path, package: &str, archive: &Path, consumer: &Path) {
    let extract = dir.join("extracted");
    fs::create_dir_all(&extract).expect("creating the extraction directory");

    let output = Command::new("cargo-nextest")
        .arg("nextest")
        .arg("run")
        .arg("--archive-file")
        .arg(archive)
        .arg("--workspace-remap")
        .arg(consumer)
        .arg("--extract-to")
        .arg(&extract)
        .arg("--extract-overwrite")
        .arg("-E")
        .arg(tier_filter_for("L1", package))
        .arg("--no-tests=fail")
        .current_dir(dir)
        .output()
        .expect("running the archived suite");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{package}'s L1 suite must pass from a checkout its producer never saw:\n{stderr}"
    );
    assert!(
        !stderr.contains("Compiling"),
        "a relocated consumer must not compile:\n{stderr}"
    );
}

/// Two real packages, from two areas, each run from a checkout its producer
/// never saw.
///
/// `test-toolkit` reads `.config/nextest.toml`, `just/devops.just`, and
/// `.github/workflows/` out of the checkout, so nothing in its suite can pass
/// from the wrong address; it is also the cheapest package in the workspace that
/// makes that demand. `biscuit-file` is the second sample, from another area and
/// with the other fixture shape — its suite walks committed corpus
/// *directories* rather than naming single files.
///
/// One test rather than two, and each `Scratch` dropped before the next begins:
/// a relocation costs two copies of the source plus a full dependency build, and
/// two of them running concurrently under nextest exhausted the temp filesystem
/// and took an unrelated fixture down with them.
#[test]
fn slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout() {
    let packages = ["test-toolkit", "biscuit-file"];
    let dir = Scratch::new("reloc");
    for (package, (archive, consumer)) in packages
        .iter()
        .zip(relocate_real_packages(dir.path(), &packages))
    {
        run_relocated(dir.path(), package, &archive, &consumer);
    }
}
