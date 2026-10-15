//! Isolated, offline builds of freshly generated client code.
//!
//! [`FreshCrate::generate`] runs the public generation entry point into a
//! temporary directory outside the checkout and writes the manifest with
//! [`generate_cargo_toml_with_paths`], pointing it at local dependency paths:
//! the checkout's `schematic-define` and `schematic-runtime` (or a doctored
//! runtime copy, [`FreshCrate::generate_with_runtime`]) and the real or a
//! stand-in `schematic-definitions`.
//! Cargo then resolves strictly offline against a copy of a committed
//! lockfile: `cargo update --workspace --offline` adds only the temporary
//! package entries, and every build runs with `--offline --locked`. There is
//! no online fallback; a host whose Cargo cache cannot satisfy the lockfile
//! fails with Cargo's own explanation and [`OFFLINE_CACHE_HINT`].
//!
//! Every child build shares one reusable target directory, a sibling of the
//! outer build's profile directories inside the configured target storage.
//! It is never the outer build's own directory, so the two never wait on each
//! other's Cargo locks. Tests that drive these builds carry the `slow_` prefix,
//! which `.config/nextest.toml` places in a one-at-a-time test group with a
//! longer timeout. When that timeout expires nextest kills the test's whole
//! process group, which bounds the child build too.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use schematic_define::RestApi;
use schematic_gen::cargo_gen::generate_cargo_toml_with_paths;
use schematic_gen::output::generate_and_write_all;
use tempfile::TempDir;

/// Where the generated crate's `schematic-definitions` dependency comes from.
pub enum Definitions<'a> {
    /// The checkout's real `schematic-definitions` crate.
    Checkout,
    /// A stand-in crate that re-exports the real one and adds one module per
    /// `(module name, Rust source)` pair. Fixture APIs need their request and
    /// response types under `schematic_definitions::<module>`, and production
    /// definitions must not carry them.
    Fixture(&'a [(&'a str, &'a str)]),
}

/// A generated client crate in a temporary directory.
pub struct FreshCrate {
    _root: TempDir,
    pub crate_dir: PathBuf,
    pub src_dir: PathBuf,
}

/// The captured result of one child Cargo invocation.
pub struct CargoRun {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub command: String,
}

/// How to fill a Cargo cache that cannot satisfy `--offline`; every guard
/// crate copies `schematic/schema/Cargo.lock`, and CI's build job runs the
/// same fetch before the guard.
pub const OFFLINE_CACHE_HINT: &str = "this host's Cargo cache lacks a crate pinned by \
     schematic/schema/Cargo.lock; run `cargo fetch --manifest-path schematic/schema/Cargo.toml` \
     from the repository root, then rerun";

/// [`OFFLINE_CACHE_HINT`] when `stderr` shows Cargo refusing a download
/// because of `--offline`.
pub fn offline_cache_hint(stderr: &str) -> Option<&'static str> {
    stderr
        .contains("but --offline was specified")
        .then_some(OFFLINE_CACHE_HINT)
}

impl CargoRun {
    /// Panics with the command, its exit, and both streams unless it succeeded.
    pub fn assert_success(&self) {
        let hint = offline_cache_hint(&self.stderr)
            .map(|hint| format!("\n\nhint: {hint}"))
            .unwrap_or_default();
        assert!(
            self.success,
            "`{}` failed{hint}\n\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.command, self.stdout, self.stderr
        );
    }
}

/// The `schematic/` package area of the checkout, resolved at run time.
pub fn schematic_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("schematic/gen has a parent")
        .to_path_buf()
}

impl FreshCrate {
    /// Generates `apis` into a new temporary crate and copies `lockfile` beside
    /// its manifest.
    pub fn generate(apis: &[&RestApi], definitions: Definitions<'_>, lockfile: &Path) -> Self {
        Self::generate_with_runtime(apis, definitions, lockfile, &schematic_root().join("runtime"))
    }

    /// [`generate`](Self::generate), with the generated crate depending on
    /// the `schematic-runtime` at `runtime` instead of the checkout's.
    pub fn generate_with_runtime(
        apis: &[&RestApi],
        definitions: Definitions<'_>,
        lockfile: &Path,
        runtime: &Path,
    ) -> Self {
        let root = TempDir::new().expect("create temp dir");
        let crate_dir = root.path().join("schema");
        let src_dir = crate_dir.join("src");

        generate_and_write_all(apis, &src_dir, false).expect("generate client code");

        let area = schematic_root();
        let define = area.join("define");
        let real_definitions = area.join("definitions");
        let definitions_path = match definitions {
            Definitions::Checkout => real_definitions,
            Definitions::Fixture(modules) => {
                let stub = root.path().join("fixture-definitions");
                write_fixture_definitions(&stub, &real_definitions, modules);
                stub
            }
        };
        let manifest = generate_cargo_toml_with_paths(
            path_str(&define),
            path_str(&definitions_path),
            path_str(runtime),
        );
        fs::write(crate_dir.join("Cargo.toml"), manifest).expect("write Cargo.toml");
        fs::copy(lockfile, crate_dir.join("Cargo.lock"))
            .unwrap_or_else(|e| panic!("copy lockfile {}: {e}", lockfile.display()));

        Self { _root: root, crate_dir, src_dir }
    }

    /// Adds `tests/<name>.rs` to the generated crate.
    pub fn add_test_file(&self, name: &str, source: &str) {
        let tests = self.crate_dir.join("tests");
        fs::create_dir_all(&tests).expect("create tests dir");
        fs::write(tests.join(format!("{name}.rs")), source).expect("write test file");
    }

    /// `cargo check --offline --locked` of the library.
    pub fn cargo_check(&self) -> CargoRun {
        self.resolve_offline();
        self.cargo(&["check", "--lib"])
    }

    /// `cargo check --offline --locked --tests --keep-going`: every test
    /// target is checked, and each one that fails reports its own errors.
    pub fn cargo_check_tests(&self) -> CargoRun {
        self.resolve_offline();
        self.cargo(&["check", "--tests", "--keep-going"])
    }

    /// Builds and runs the crate's integration tests through nextest,
    /// offline and locked, with `envs` added to the child environment.
    pub fn nextest_run(&self, envs: &[(&str, &str)]) -> CargoRun {
        self.resolve_offline();
        let mut command = self.command(&["nextest", "run", "--tests", "--no-fail-fast"]);
        for (key, value) in envs {
            command.env(key, value);
        }
        run(command)
    }

    /// Adds the temporary package entries to the copied lockfile without
    /// touching any registry version it already pins.
    fn resolve_offline(&self) {
        let mut command = cargo_command();
        command
            .args(["update", "--workspace", "--offline", "--manifest-path"])
            .arg(self.crate_dir.join("Cargo.toml"));
        run(command).assert_success();
    }

    fn cargo(&self, args: &[&str]) -> CargoRun {
        run(self.command(args))
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = cargo_command();
        command
            .args(args)
            .args(["--offline", "--locked", "--manifest-path"])
            .arg(self.crate_dir.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(child_target_dir());
        command
    }
}

fn path_str(path: &Path) -> &str {
    path.to_str()
        .unwrap_or_else(|| panic!("non-UTF-8 path {}", path.display()))
}

fn write_fixture_definitions(stub: &Path, real: &Path, modules: &[(&str, &str)]) {
    fs::create_dir_all(stub.join("src")).expect("create fixture definitions");
    // Version 0.1.1 still satisfies the generated `version = "0.1.0"`
    // requirement while keeping the stand-in distinct from the real 0.1.0 in
    // the lockfile; two path packages with one name and version collide there.
    let manifest = format!(
        "[package]\n\
         name = \"schematic-definitions\"\n\
         version = \"0.1.1\"\n\
         edition = \"2024\"\n\
         publish = false\n\n\
         [dependencies]\n\
         real = {{ package = \"schematic-definitions\", version = \"0.1.0\", path = {} }}\n\
         serde = {{ version = \"1.0\", features = [\"derive\"] }}\n\
         serde_json = \"1\"\n\
         schemars = \"1\"\n\
         schematic-define = {{ path = {} }}\n",
        toml_path(real),
        toml_path(&real.parent().unwrap().join("define"))
    );
    fs::write(stub.join("Cargo.toml"), manifest).expect("write fixture manifest");

    let mut lib = String::from("pub use real::*;\n");
    for (name, source) in modules {
        lib.push_str(&format!("\npub mod {name} {{\n{source}\n}}\n"));
    }
    fs::write(stub.join("src/lib.rs"), lib).expect("write fixture lib.rs");
}

fn toml_path(path: &Path) -> String {
    biscuit_file::toml_crate::Value::String(path_str(path).to_string()).to_string()
}

/// A Cargo command stripped of the settings the outer test run leaks into
/// its tests. Nextest's own variables would select the outer profile (`ci`
/// does not exist in the temporary crate), and an inherited target directory
/// or build target would aim the child build at the outer build's output.
fn cargo_command() -> Command {
    let program = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(program);
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        let leaked = key.starts_with("NEXTEST")
            || key.starts_with("CARGO_PKG_")
            || key.starts_with("CARGO_BIN_")
            || (key.starts_with("CARGO_TARGET_") && key.ends_with("_RUNNER"))
            || matches!(
                key,
                "CARGO_TARGET_DIR"
                    | "CARGO_BUILD_TARGET_DIR"
                    | "CARGO_BUILD_BUILD_DIR"
                    | "CARGO_BUILD_TARGET"
                    | "CARGO_MANIFEST_DIR"
                    | "CARGO_MANIFEST_PATH"
                    | "CARGO_CRATE_NAME"
                    | "CARGO_PRIMARY_PACKAGE"
                    | "CARGO_TARGET_TMPDIR"
            );
        if leaked {
            command.env_remove(key);
        }
    }
    command.env("CARGO_NET_OFFLINE", "true");
    command.env("CARGO_TERM_COLOR", "never");
    command
}

fn run(mut command: Command) -> CargoRun {
    let rendered = format!("{command:?}");
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("could not start Cargo ({rendered}): {e}"));
    CargoRun {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        command: rendered,
    }
}

/// The reusable child target directory.
///
/// The running test binary lives at `<target>/<profile>/deps/<binary>` (a
/// cross build inserts a target triple after `<target>`), so this resolves
/// inside whatever target storage the outer build was configured with.
fn child_target_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current test executable");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary sits in <profile>/deps");
    profile_dir
        .parent()
        .expect("profile dir has a parent")
        .join("schematic-fresh-output")
}

/// Copies the `schematic-runtime` crate at `runtime` into `destination` with
/// its `ABI` constant set to `abi`, for proving that generated code refuses a
/// runtime of another ABI. Only the package and its normal dependencies are
/// kept: the copy is a dependency, never a workspace member, so its tests and
/// their path dependencies have no meaning there.
pub fn doctored_runtime(runtime: &Path, destination: &Path, abi: u32) {
    use biscuit_file::toml_crate;

    let manifest = fs::read_to_string(runtime.join("Cargo.toml")).expect("read runtime manifest");
    let mut table: toml_crate::Table = toml_crate::from_str(&manifest).expect("runtime manifest is TOML");
    table.retain(|key, _| key == "package" || key == "dependencies");
    fs::create_dir_all(destination.join("src")).expect("create runtime copy");
    fs::write(destination.join("Cargo.toml"), toml_crate::to_string(&table).expect("serialize"))
        .expect("write runtime copy manifest");

    copy_tree(&runtime.join("src"), &destination.join("src"));
    let lib = destination.join("src/lib.rs");
    let source = fs::read_to_string(&lib).expect("read runtime lib.rs");
    let marker = "pub const ABI: u32 = ";
    let start = source.find(marker).expect("the runtime declares its ABI") + marker.len();
    let end = start + source[start..].find(';').expect("the ABI declaration ends");
    fs::write(&lib, format!("{}{abi}{}", &source[..start], &source[end..])).expect("write runtime lib.rs");
}

/// Copies the files below `from` into `to`, subdirectories included.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|e| panic!("create {}: {e}", to.display()));
    for entry in fs::read_dir(from).unwrap_or_else(|e| panic!("list {}: {e}", from.display())) {
        let path = entry.expect("directory entry").path();
        let target = to.join(path.file_name().expect("file name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap_or_else(|e| panic!("copy {}: {e}", path.display()));
        }
    }
}

/// Relative path to file bytes (or `None` for a directory) under one root.
pub type TreeSnapshot = BTreeMap<String, Option<Vec<u8>>>;

/// Every file and directory below `root`, keyed by `/`-separated relative
/// path. Symlinks are recorded as entries but not followed.
pub fn snapshot_tree(root: &Path) -> TreeSnapshot {
    let mut entries = TreeSnapshot::new();
    collect(root, root, &mut entries);
    entries
}

fn collect(root: &Path, dir: &Path, entries: &mut TreeSnapshot) {
    let mut children: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry"))
        .collect();
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("entry below root")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let file_type = entry.file_type().expect("file type");
        if file_type.is_dir() {
            entries.insert(relative, None);
            collect(root, &path, entries);
        } else if file_type.is_file() {
            entries.insert(relative, Some(fs::read(&path).expect("read file")));
        } else {
            entries.insert(relative, None);
        }
    }
}

/// The names of `dir`'s direct children, sorted.
pub fn list_dir(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A one-line-per-path summary of how two snapshots differ.
pub fn diff_summary(expected: &TreeSnapshot, actual: &TreeSnapshot) -> Vec<String> {
    let mut lines = Vec::new();
    for (path, bytes) in expected {
        match actual.get(path) {
            None => lines.push(format!("missing: {path}")),
            Some(other) if other != bytes => lines.push(format!("differs: {path}")),
            Some(_) => {}
        }
    }
    for path in actual.keys() {
        if !expected.contains_key(path) {
            lines.push(format!("unexpected: {path}"));
        }
    }
    lines
}
