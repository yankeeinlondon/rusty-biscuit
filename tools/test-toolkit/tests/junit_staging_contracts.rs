//! Contract tests for the JUnit report staging that `just/devops.just` performs
//! around every nextest invocation.
//!
//! `.config/nextest.toml` points the `ci` profile's JUnit writer at a single
//! `<target>/nextest/ci/test-results.xml`, so a multi-package area recipe used to
//! keep only whichever package ran last, and `just`'s abort-on-first-failure meant
//! a red package silently cancelled every package after it. The staging contract
//! these tests pin fixes both:
//!
//! ```text
//! $STAGE/<tier>/<package>.xml   one verbatim report per nextest invocation
//! $STAGE/manifest.jsonl         one record per invocation, appended
//! ```
//!
//! The CI rollup (plan phase 0.2) consumes both paths verbatim, so the layout,
//! the manifest key set, and the key order are all load-bearing.
//!
//! Each test drives a real `just` invocation against a throwaway two-crate cargo
//! workspace whose `justfile` imports the repo's real `just/devops.just`. Nothing
//! is mocked: the recipes, `cargo nextest`, and the JUnit writer all run. The
//! fixture crates have no dependencies, so a cold run costs about a second.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// The manifest keys, in the order the rollup expects to find them.
const MANIFEST_KEYS: [&str; 9] = [
    "tier",
    "package",
    "xml",
    "exit_code",
    "area",
    "environment",
    "shard",
    "duration_s",
    "report_present",
];

const ALPHA: &str = "stage-fixture-alpha";
const BETA: &str = "stage-fixture-beta-cli";

/// Walk up from this crate until the directory holding the shared `just` recipes
/// is found. Works from a worktree or a relocated checkout.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join("just").join("devops.just").is_file())
        .map(Path::to_path_buf)
        .expect("could not locate the repo root from CARGO_MANIFEST_DIR")
}

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("create fixture workspace");
        let root = dir.path();

        write(
            &root.join("Cargo.toml"),
            &format!(
                "[workspace]\nresolver = \"2\"\nmembers = [\"{ALPHA}\", \"{BETA}\"]\n\
                 [profile.dev]\ndebug = 0\n"
            ),
        );

        // The staging helper reads the report from
        // `<target>/nextest/<profile>/test-results.xml`, so the fixture has to
        // configure JUnit the same way the repo's own `ci` profile does.
        write(
            &root.join(".config").join("nextest.toml"),
            "[profile.ci]\nretries = 0\njunit = { path = \"test-results.xml\" }\n",
        );

        // `just` accepts `/`-separated import paths on every platform.
        let devops = repo_root()
            .join("just")
            .join("devops.just")
            .to_string_lossy()
            .replace('\\', "/");
        write(
            &root.join("justfile"),
            &format!(
                "import '{devops}'\n\n\
                 test *args=\"\":\n    @just _test_all \"{ALPHA} {BETA}\" {{{{ args }}}}\n\n\
                 test-per-package-flags *args=\"\":\n    @just _test_all \"{ALPHA} --features extra; {BETA}\" {{{{ args }}}}\n"
            ),
        );

        crate_at(root, ALPHA, "ALPHA");
        crate_at(root, BETA, "BETA");

        Self { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn stage(&self) -> PathBuf {
        self.root()
            .join("target")
            .join("nextest")
            .join("ci-reports")
    }

    /// Run the fixture area's `test` recipe with a scrubbed environment.
    fn run_test(&self, env: &[(&str, &str)], extra_args: &[&str]) -> Output {
        self.run_recipe("test", env, extra_args)
    }

    fn run_recipe(&self, recipe: &str, env: &[(&str, &str)], extra_args: &[&str]) -> Output {
        let mut cmd = Command::new("just");
        cmd.arg(recipe).args(extra_args).current_dir(self.root());

        // Cargo and nextest export state describing *this* test run. Leaking it
        // into the nested build would redirect the fixture's target directory
        // (and therefore its report) somewhere shared.
        for (key, _) in std::env::vars() {
            let leaks = key.starts_with("NEXTEST")
                || key.starts_with("BISCUIT_")
                || key.starts_with("CARGO_PKG_")
                || key.starts_with("CARGO_BIN_")
                || key.starts_with("CARGO_MANIFEST")
                || key.starts_with("STAGE_FIXTURE_")
                || key == "CARGO_TARGET_DIR"
                || key == "CARGO_CRATE_NAME"
                || key == "CARGO_PRIMARY_PACKAGE"
                || key == "CARGO_MAKEFLAGS"
                || key == "RUST_TEST_THREADS";
            if leaks {
                cmd.env_remove(key);
            }
        }
        cmd.env("NEXTEST_PROFILE", "ci");
        for (key, value) in env {
            cmd.env(key, value);
        }

        cmd.output().unwrap_or_else(|err| {
            panic!("failed to run `just test` in the fixture workspace: {err}")
        })
    }

    fn manifest_records(&self) -> Vec<Record> {
        let path = self.stage().join("manifest.jsonl");
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("no manifest at {}: {err}", path.display()));
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(Record::parse)
            .collect()
    }

    fn staged_xml(&self, tier: &str, package: &str) -> String {
        let path = self.stage().join(tier).join(format!("{package}.xml"));
        fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("no staged report at {}: {err}", path.display()))
    }
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture directory");
    }
    fs::write(path, contents).expect("write fixture file");
}

/// A dependency-free crate with two tests, the first of which fails when
/// `STAGE_FIXTURE_<tag>_MUST_FAIL` is set. One build serves the pass and the
/// fail scenario.
/// The `extra` feature exists so a test can prove that per-package extra args
/// reach only their own package — the mechanism `sniff` (`--features remote`)
/// and `messenger` (`--features desktop`) rely on.
fn crate_at(root: &Path, name: &str, tag: &str) {
    write(
        &root.join(name).join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
             [features]\nextra = []\n"
        ),
    );
    write(
        &root.join(name).join("src").join("lib.rs"),
        &format!(
            "#[cfg(test)]\nmod tests {{\n\
             \x20   #[test]\n\
             \x20   fn case_one() {{\n\
             \x20       assert!(\n\
             \x20           std::env::var(\"STAGE_FIXTURE_{tag}_MUST_FAIL\").is_err(),\n\
             \x20           \"fixture failing on request\"\n\
             \x20       );\n\
             \x20   }}\n\n\
             \x20   #[test]\n\
             \x20   fn case_two() {{}}\n\n\
             \x20   #[cfg(feature = \"extra\")]\n\
             \x20   #[test]\n\
             \x20   fn case_behind_extra_feature() {{}}\n\
             }}\n"
        ),
    );
}

/// One `manifest.jsonl` line, kept as raw JSON text so the tests can assert on
/// the literal encoding as well as the values.
struct Record {
    line: String,
    keys: Vec<String>,
    values: BTreeMap<String, String>,
}

impl Record {
    /// The records are emitted by `jq -c` over flat scalar values, so a
    /// depth-1 scan is sufficient and keeps this crate free of a JSON
    /// dependency.
    fn parse(line: &str) -> Self {
        let body = line
            .trim()
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
            .unwrap_or_else(|| panic!("manifest line is not a JSON object: {line}"));

        let mut keys = Vec::new();
        let mut values = BTreeMap::new();
        for field in split_top_level(body) {
            let (key, value) = field
                .split_once(':')
                .unwrap_or_else(|| panic!("manifest field has no `:`: {field}"));
            let key = key.trim().trim_matches('"').to_string();
            keys.push(key.clone());
            values.insert(key, value.trim().to_string());
        }

        Self {
            line: line.to_string(),
            keys,
            values,
        }
    }

    /// Raw JSON text of a field — `"L1"` for a string, `0` for a number.
    fn raw(&self, key: &str) -> &str {
        self.values
            .get(key)
            .unwrap_or_else(|| panic!("manifest record has no `{key}`: {}", self.line))
    }

    fn string(&self, key: &str) -> String {
        self.raw(key).trim_matches('"').to_string()
    }
}

/// Split a flat JSON object body on commas that are not inside a string.
fn split_top_level(body: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in body.chars() {
        match ch {
            _ if escaped => {
                escaped = false;
                current.push(ch);
            }
            '\\' if in_string => {
                escaped = true;
                current.push(ch);
            }
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            ',' if !in_string => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        fields.push(current);
    }
    fields
}

fn assert_schema(record: &Record) {
    assert_eq!(
        record.keys, MANIFEST_KEYS,
        "manifest key set/order is a fixed contract; got {}",
        record.line
    );
}

// --- contracts -------------------------------------------------------------

/// Two passing packages leave two distinct reports and two records. Before
/// staging, the second nextest invocation overwrote the first one's XML.
#[test]
fn two_package_success_stages_one_report_per_package() {
    let fixture = Fixture::new();
    let output = fixture.run_test(&[], &[]);
    assert!(
        output.status.success(),
        "fixture area `test` should pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let records = fixture.manifest_records();
    assert_eq!(records.len(), 2, "one record per nextest invocation");
    for record in &records {
        assert_schema(record);
        assert_eq!(record.string("tier"), "L1");
        assert_eq!(record.raw("exit_code"), "0");
        assert_eq!(record.raw("report_present"), "true");
    }
    assert_eq!(records[0].string("package"), ALPHA);
    assert_eq!(records[1].string("package"), BETA);

    // Each staged document must be the report for its OWN package — the
    // overwrite bug left both names pointing at one package's results.
    let alpha = fixture.staged_xml("L1", ALPHA);
    let beta = fixture.staged_xml("L1", BETA);
    assert!(
        alpha.contains(&format!("<testsuite name=\"{ALPHA}\"")),
        "staged alpha report does not describe alpha:\n{alpha}"
    );
    assert!(
        beta.contains(&format!("<testsuite name=\"{BETA}\"")),
        "staged beta report does not describe beta:\n{beta}"
    );
    assert!(
        alpha.starts_with("<?xml"),
        "staged report must stay a valid document"
    );
}

/// A red first package must not cancel the packages after it, and both reports
/// must survive. This is the defect that left `biscuit-file-cli` untested on
/// Windows with no evidence of the gap.
#[test]
fn first_package_failure_still_runs_and_stages_the_rest() {
    let fixture = Fixture::new();
    let output = fixture.run_test(&[("STAGE_FIXTURE_ALPHA_MUST_FAIL", "1")], &[]);
    assert!(
        !output.status.success(),
        "a failing package must make the area recipe fail"
    );

    let records = fixture.manifest_records();
    assert_eq!(
        records.len(),
        2,
        "the second package must still run:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    for record in &records {
        assert_schema(record);
        assert_eq!(record.raw("report_present"), "true");
    }

    assert_eq!(records[0].string("package"), ALPHA);
    assert_ne!(
        records[0].raw("exit_code"),
        "0",
        "the failing package's non-zero exit belongs in the manifest"
    );
    assert_eq!(records[1].string("package"), BETA);
    assert_eq!(records[1].raw("exit_code"), "0");

    // The failing package's own report is staged, not discarded…
    let alpha = fixture.staged_xml("L1", ALPHA);
    assert!(
        alpha.contains("<failure") || alpha.contains("failures=\"1\""),
        "the failing package's report should record its failure:\n{alpha}"
    );
    // …and the later package really executed its tests.
    let beta = fixture.staged_xml("L1", BETA);
    assert!(
        beta.contains("tests=\"2\""),
        "the package after the failure should have run its tests:\n{beta}"
    );
}

/// Sharded runs are separate jobs writing separate staging trees, so shard
/// identity has to travel in the manifest rather than in the file name.
#[test]
fn sharded_run_records_shard_identity() {
    let fixture = Fixture::new();
    let output = fixture.run_test(
        &[
            ("BISCUIT_CI_AREA", "stage-fixture"),
            ("BISCUIT_CI_ENVIRONMENT", "ubuntu-latest"),
            ("BISCUIT_CI_SHARD", "1/2"),
        ],
        &["--partition", "count:1/2"],
    );
    assert!(
        output.status.success(),
        "sharded fixture run should pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let records = fixture.manifest_records();
    assert_eq!(records.len(), 2);
    for record in &records {
        assert_schema(record);
        assert_eq!(record.string("shard"), "1/2");
        assert_eq!(record.string("area"), "stage-fixture");
        assert_eq!(record.string("environment"), "ubuntu-latest");
        assert_eq!(record.raw("report_present"), "true");
    }
}

/// The CI identity fields are reported, never invented: unset means `""`.
#[test]
fn absent_ci_identity_is_empty_not_invented() {
    let fixture = Fixture::new();
    let output = fixture.run_test(&[], &[]);
    assert!(output.status.success());

    for record in &fixture.manifest_records() {
        assert_eq!(record.raw("area"), "\"\"");
        assert_eq!(record.raw("environment"), "\"\"");
        assert_eq!(record.raw("shard"), "\"\"");
    }
}

/// Hyphenated package names are the norm in this workspace. They must survive
/// into both the staged file name and the JSON encoding.
#[test]
fn hyphenated_package_names_round_trip() {
    let fixture = Fixture::new();
    let output = fixture.run_test(&[], &[]);
    assert!(output.status.success());

    let records = fixture.manifest_records();
    let beta = records
        .iter()
        .find(|record| record.string("package") == BETA)
        .unwrap_or_else(|| panic!("no record for {BETA}"));

    assert!(
        beta.line.contains(&format!("\"package\":\"{BETA}\"")),
        "package name should be plain JSON text: {}",
        beta.line
    );
    assert_eq!(beta.string("xml"), format!("L1/{BETA}.xml"));
    assert!(
        fixture
            .stage()
            .join("L1")
            .join(format!("{BETA}.xml"))
            .is_file(),
        "the `xml` field must name a file that exists"
    );
}

/// `BISCUIT_JUNIT_STAGE_DIR` relocates the staging root; the layout under it is
/// unchanged. The CI rollup uses this to collect reports outside `target/`.
#[test]
fn stage_dir_override_is_honored() {
    let fixture = Fixture::new();
    let elsewhere = fixture.root().join("collected-reports");
    let output = fixture.run_test(
        &[("BISCUIT_JUNIT_STAGE_DIR", &elsewhere.to_string_lossy())],
        &[],
    );
    assert!(
        output.status.success(),
        "override run should pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(elsewhere.join("manifest.jsonl").is_file());
    assert!(elsewhere.join("L1").join(format!("{ALPHA}.xml")).is_file());
    assert!(elsewhere.join("L1").join(format!("{BETA}.xml")).is_file());
    assert!(
        !fixture.stage().exists(),
        "the default staging root should be unused when overridden"
    );
}

/// A `;`-separated spec gives one package extra flags without giving them to
/// the others. `sniff` loses whole Wiremock suites without `--features remote`,
/// and `messenger`'s contract needs `--features desktop`.
#[test]
fn per_package_flags_reach_only_their_own_package() {
    let fixture = Fixture::new();
    let output = fixture.run_recipe("test-per-package-flags", &[], &[]);
    assert!(
        output.status.success(),
        "per-package-flag run should pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let alpha = fixture.staged_xml("L1", ALPHA);
    assert!(
        alpha.contains("case_behind_extra_feature"),
        "`--features extra` should have reached {ALPHA}:\n{alpha}"
    );
    let beta = fixture.staged_xml("L1", BETA);
    assert!(
        !beta.contains("case_behind_extra_feature"),
        "{BETA} was given another package's flags:\n{beta}"
    );

    // The manifest keys on the bare package name, not the invocation's flags.
    let records = fixture.manifest_records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].string("package"), ALPHA);
    assert_eq!(records[1].string("package"), BETA);
}

/// The staging helper reads `<target>/nextest/<profile>/test-results.xml`. That
/// path is only correct while the repo's `ci` profile keeps writing there.
#[test]
fn repo_ci_profile_writes_the_report_path_staging_expects() {
    let config = fs::read_to_string(repo_root().join(".config").join("nextest.toml"))
        .expect("read .config/nextest.toml");
    let ci_profile = config
        .split_once("[profile.ci]")
        .expect("`.config/nextest.toml` must define a `ci` profile")
        .1;
    assert!(
        ci_profile.contains("junit = { path = \"test-results.xml\" }"),
        "`_stage_junit` copies from `<target>/nextest/<profile>/test-results.xml`; \
         update it alongside any change to the `ci` profile's junit path"
    );
}
