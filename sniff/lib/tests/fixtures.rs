use git2::Repository;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary git repo for testing
pub fn create_test_git_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let _repo = Repository::init(dir.path()).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Cargo workspace structure
pub fn create_cargo_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"pkg1\", \"pkg2\"]\n",
    )
    .unwrap();
    fs::create_dir(dir.path().join("pkg1")).unwrap();
    fs::create_dir(dir.path().join("pkg2")).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Cargo *virtual* workspace with a single member and no root
/// `[package]`. The honest predicate must report this as **not** a monorepo:
/// `[workspace]` alone does not create a package boundary at the root, so the
/// resolved set has one package, which is degenerate.
pub fn create_virtual_cargo_single_member_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"only\"]\n",
    )
    .unwrap();
    let only = dir.path().join("only");
    fs::create_dir_all(&only).unwrap();
    fs::write(
        only.join("Cargo.toml"),
        "[package]\nname = \"only\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Cargo workspace whose root also declares a `[package]` and which
/// lists a single non-root member. The honest predicate treats this as a real
/// monorepo: the root package plus the member is two package boundaries.
pub fn create_cargo_root_package_plus_one_member() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"root\"\nversion = \"0.1.0\"\n\n\
         [workspace]\nmembers = [\"child\"]\n",
    )
    .unwrap();
    let child = dir.path().join("child");
    fs::create_dir_all(&child).unwrap();
    fs::write(
        child.join("Cargo.toml"),
        "[package]\nname = \"child\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a repo whose root hosts a Cargo workspace and whose `web/` subtree
/// hosts a separate pnpm workspace.
///
/// This is the spec's canonical forest example: a Cargo workspace containing
/// a pnpm workspace several directories down. The topology must report both
/// layers — Cargo at the root and pnpm at `web/` — rather than collapsing
/// them into a single Cargo layer.
pub fn create_cargo_root_with_nested_pnpm() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Root Cargo workspace with one member.
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"server\"]\n",
    )
    .unwrap();
    let server = root.join("server");
    fs::create_dir_all(&server).unwrap();
    fs::write(
        server.join("Cargo.toml"),
        "[package]\nname = \"server\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // Nested pnpm workspace under web/.
    let web = root.join("web");
    fs::create_dir_all(&web).unwrap();
    fs::write(
        web.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(web.join("package.json"), "{}").unwrap();
    let web_app = web.join("packages/app");
    fs::create_dir_all(&web_app).unwrap();
    fs::write(web_app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let web_lib = web.join("packages/lib");
    fs::create_dir_all(&web_lib).unwrap();
    fs::write(web_lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a repo whose root hosts a pnpm workspace and whose `python/` subtree
/// has a separate uv workspace. uv's `ForbidsNested` policy is about uv's own
/// nesting, so a sibling uv workspace under a pnpm root is still reported.
pub fn create_pnpm_root_with_nested_uv() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Root pnpm workspace.
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(root.join("package.json"), "{}").unwrap();
    let app = root.join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();

    // Nested uv workspace under python/.
    let python = root.join("python");
    fs::create_dir_all(&python).unwrap();
    fs::write(
        python.join("pyproject.toml"),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n\
         [tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
    )
    .unwrap();
    let py_pkg = python.join("packages/lib");
    fs::create_dir_all(&py_pkg).unwrap();
    fs::write(
        py_pkg.join("pyproject.toml"),
        "[project]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a repo whose root hosts a pnpm workspace and whose `crates/`
/// subtree has a separate Cargo workspace.
///
/// Counterpart to [`create_cargo_root_with_nested_pnpm`] with the standards
/// flipped: Cargo's `ForbidsNested` policy only forbids nested Cargo under an
/// *ancestor Cargo* workspace, so a Cargo workspace nested under a pnpm root
/// is a valid separate layer and must appear in the topology forest.
pub fn create_pnpm_root_with_nested_cargo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Root pnpm workspace with one member.
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(root.join("package.json"), "{}").unwrap();
    let app = root.join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();

    // Nested Cargo workspace under crates/.
    let crates = root.join("crates");
    fs::create_dir_all(&crates).unwrap();
    fs::write(
        crates.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha\", \"beta\"]\n",
    )
    .unwrap();
    for name in ["alpha", "beta"] {
        let pkg = crates.join(name);
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a repo whose root has *no* workspace marker but whose `crates/`
/// subtree hosts a Cargo workspace.
///
/// Before `Cargo.toml` was added to the nested marker table, a bare root with
/// only a nested Cargo workspace was missed entirely — no ancestor standard
/// dispatched the Cargo detector at `crates/`. The marker walk now surfaces it
/// as its own layer.
pub fn create_nested_only_cargo_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Nested Cargo workspace under crates/. The root deliberately has no
    // marker of its own so only the nested walk can discover the workspace.
    let crates = root.join("crates");
    fs::create_dir_all(&crates).unwrap();
    fs::write(
        crates.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha\", \"beta\"]\n",
    )
    .unwrap();
    for name in ["alpha", "beta"] {
        let pkg = crates.join(name);
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a repo whose root hosts a Cargo workspace and whose `nested/`
/// subtree declares a *second* Cargo workspace.
///
/// Cargo's `NestingPolicy::ForbidsNested` means a nested Cargo workspace is
/// invalid Cargo — the nested detector must not produce a second Cargo layer
/// for this layout. The root workspace still resolves normally.
pub fn create_cargo_root_with_nested_forbidden_cargo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Root Cargo workspace with two members (so it is non-degenerate).
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"pkg-a\", \"pkg-b\"]\n",
    )
    .unwrap();
    for name in ["pkg-a", "pkg-b"] {
        let pkg = root.join(name);
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }

    // Nested Cargo workspace under nested/. Cargo forbids this; sniff must
    // not report it as a separate layer.
    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("Cargo.toml"),
        "[workspace]\nmembers = [\"inner\"]\n",
    )
    .unwrap();
    let inner = nested.join("inner");
    fs::create_dir_all(&inner).unwrap();
    fs::write(
        inner.join("Cargo.toml"),
        "[package]\nname = \"inner\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a repo whose root has *no* workspace marker but whose `web/`
/// subtree hosts a pnpm workspace.
///
/// Regression fixture for the nested-only full-detection panic: the root
/// itself is not a workspace, so the eager `ManifestIndex` build is skipped,
/// but nested discovery still finds `web/pnpm-workspace.yaml`. Full
/// `detect_repo` must build the index lazily rather than panicking.
pub fn create_nested_only_pnpm_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Nested pnpm workspace under web/. The root deliberately has no marker
    // of its own so the eager manifest-index build is skipped.
    let web = root.join("web");
    fs::create_dir_all(&web).unwrap();
    fs::write(
        web.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(web.join("package.json"), "{}").unwrap();
    let web_app = web.join("packages/app");
    fs::create_dir_all(&web_app).unwrap();
    fs::write(web_app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let web_lib = web.join("packages/lib");
    fs::create_dir_all(&web_lib).unwrap();
    fs::write(web_lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a pnpm workspace whose `pnpm-lock.yaml` `importers:` agree with the
/// manifest globs.
pub fn create_pnpm_workspace_with_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_pnpm_workspace();
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n\nimporters:\n  .:\n    dependencies: {}\n  packages/app:\n    dependencies: {}\n  packages/lib:\n    dependencies: {}\n",
    )
    .unwrap();
    (dir, path)
}

/// Create a pnpm workspace whose `pnpm-lock.yaml` is missing a manifest member.
pub fn create_pnpm_workspace_with_drifted_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_pnpm_workspace();
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n\nimporters:\n  .:\n    dependencies: {}\n  packages/app:\n    dependencies: {}\n",
    )
    .unwrap();
    (dir, path)
}

/// Create a pnpm workspace whose `pnpm-lock.yaml` carries an extra stale
/// importer that is no longer declared in the manifest.
///
/// Under the old subset check this lockfile was incorrectly reported as
/// matching (every manifest member appeared in the lockfile). The
/// set-equality check flags it as drift so consumers see the stale state.
pub fn create_pnpm_workspace_with_stale_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_pnpm_workspace();
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n\nimporters:\n  \\n  .:\n    dependencies: {}\n  packages/app:\n    dependencies: {}\n  packages/lib:\n    dependencies: {}\n  packages/removed:\n    dependencies: {}\n",
    )
    .unwrap();
    (dir, path)
}

/// Create a uv workspace whose `uv.lock` `workspace.members` agree with the
/// manifest globs plus the always-counted root.
pub fn create_uv_workspace_with_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_uv_workspace();
    fs::write(
        dir.path().join("uv.lock"),
        "[workspace]\nmembers = [\".\", \"packages/app\", \"packages/lib\"]\n",
    )
    .unwrap();
    (dir, path)
}

/// Create a uv workspace whose `uv.lock` carries an extra stale member.
pub fn create_uv_workspace_with_stale_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_uv_workspace();
    fs::write(
        dir.path().join("uv.lock"),
        "[workspace]\nmembers = [\".\", \"packages/app\", \"packages/lib\", \"packages/removed\"]\n",
    )
    .unwrap();
    (dir, path)
}

/// Create a directory with mixed language files
pub fn create_mixed_language_dir() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();
    fs::write(dir.path().join("index.js"), "console.log('hello')").unwrap();
    fs::write(dir.path().join("app.py"), "print('hello')").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a pnpm workspace
pub fn create_pnpm_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    // The `packages/*` glob resolves package boundaries by manifest presence, so
    // each member needs its own package.json to be a real workspace package.
    let app = dir.path().join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let lib = dir.path().join("packages/lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a pnpm workspace whose members use a deep `**` glob.
///
/// The package manifests live two directories below the glob base
/// (`members/<group>/<pkg>/package.json`), so a prefix-only expander would
/// report the intermediate group directories instead of the real packages.
pub fn create_nested_glob_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'members/**'\n",
    )
    .unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let pkg_one = dir.path().join("members/group-a/pkg-one");
    fs::create_dir_all(&pkg_one).unwrap();
    fs::write(pkg_one.join("package.json"), r#"{"name":"pkg-one"}"#).unwrap();

    let pkg_two = dir.path().join("members/group-b/pkg-two");
    fs::create_dir_all(&pkg_two).unwrap();
    fs::write(pkg_two.join("package.json"), r#"{"name":"pkg-two"}"#).unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a repo where Nx orchestrates a pnpm workspace.
///
/// Both `nx.json` and `pnpm-workspace.yaml` sit at the root. The membership
/// authority is pnpm; Nx only orchestrates.
pub fn create_nx_pnpm_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("nx.json"), "{}").unwrap();
    fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    let app = dir.path().join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let lib = dir.path().join("packages/lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Cargo workspace whose `members` array is empty (degenerate).
pub fn create_degenerate_cargo_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a `package.json` whose `workspaces` array is empty (degenerate).
pub fn create_degenerate_npm_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("package.json"), r#"{"workspaces": []}"#).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a `pnpm-workspace.yaml` whose `packages` array is empty (degenerate).
pub fn create_degenerate_pnpm_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("pnpm-workspace.yaml"), "packages: []\n").unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Bun workspace (mirrors `create_pnpm_workspace` with a `bun.lock`).
///
/// Members are declared via `package.json#workspaces`; the `bun.lock` sentinel
/// is what makes Bun — not npm — the membership authority.
pub fn create_bun_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"root","workspaces":["packages/*"]}"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();
    let app = dir.path().join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let lib = dir.path().join("packages/lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Bun repo whose `workspaces` array is empty (degenerate).
pub fn create_degenerate_bun_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"root","workspaces":[]}"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a uv workspace (root `pyproject.toml` + two member packages).
///
/// uv counts the root `[project]` as a member, so the resolved package set is
/// the root plus the two globbed children.
pub fn create_uv_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n\
         [tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
    )
    .unwrap();
    let app = dir.path().join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(
        app.join("pyproject.toml"),
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let lib = dir.path().join("packages/lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(
        lib.join("pyproject.toml"),
        "[project]\nname = \"lib\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a uv repo whose `[tool.uv.workspace] members` is empty (degenerate).
pub fn create_degenerate_uv_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = \"solo\"\nversion = \"0.1.0\"\n\n\
         [tool.uv.workspace]\nmembers = []\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Go workspace (`go.work` with two `use` directives + two modules).
pub fn create_go_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("go.work"),
        "go 1.21\n\nuse (\n\t./svc-a\n\t./svc-b\n)\n",
    )
    .unwrap();
    let svc_a = dir.path().join("svc-a");
    fs::create_dir_all(&svc_a).unwrap();
    fs::write(
        svc_a.join("go.mod"),
        "module example.com/svc-a\n\ngo 1.21\n",
    )
    .unwrap();
    let svc_b = dir.path().join("svc-b");
    fs::create_dir_all(&svc_b).unwrap();
    fs::write(
        svc_b.join("go.mod"),
        "module example.com/svc-b\n\ngo 1.21\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Go workspace whose `use` block is empty (degenerate).
pub fn create_degenerate_go_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n)\n").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Gradle multi-project build (`settings.gradle` with two `include`s).
pub fn create_gradle_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("settings.gradle"),
        "rootProject.name = 'demo'\ninclude 'core', ':app'\n",
    )
    .unwrap();
    fs::write(dir.path().join("gradlew"), "#!/bin/sh\n").unwrap();
    let core = dir.path().join("core");
    fs::create_dir_all(&core).unwrap();
    fs::write(core.join("build.gradle"), "plugins { id 'java' }\n").unwrap();
    let app = dir.path().join("app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("build.gradle"), "plugins { id 'java' }\n").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Gradle build whose `settings.gradle` declares no `include` directive.
pub fn create_degenerate_gradle_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("settings.gradle"),
        "rootProject.name = 'solo'\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Maven multi-module build (parent `pom.xml` with two `<module>`s).
pub fn create_maven_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pom.xml"),
        "<project>\n  <artifactId>parent</artifactId>\n  <packaging>pom</packaging>\n\
         <modules>\n    <module>core</module>\n    <module>web</module>\n  </modules>\n</project>\n",
    )
    .unwrap();
    fs::write(dir.path().join("mvnw"), "#!/bin/sh\n").unwrap();
    let core = dir.path().join("core");
    fs::create_dir_all(&core).unwrap();
    fs::write(
        core.join("pom.xml"),
        "<project><artifactId>core</artifactId></project>\n",
    )
    .unwrap();
    let web = dir.path().join("web");
    fs::create_dir_all(&web).unwrap();
    fs::write(
        web.join("pom.xml"),
        "<project><artifactId>web</artifactId></project>\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Maven `pom.xml` that declares no `<modules>` (degenerate).
pub fn create_degenerate_maven_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pom.xml"),
        "<project><artifactId>solo</artifactId></project>\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a .NET solution (`*.sln` listing two `.csproj` projects).
pub fn create_dotnet_solution() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Demo.sln"),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n\
         Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"App\", \"src\\App\\App.csproj\", \"{1}\"\n\
         EndProject\n\
         Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Lib\", \"src\\Lib\\Lib.csproj\", \"{2}\"\n\
         EndProject\n",
    )
    .unwrap();
    let app = dir.path().join("src/App");
    fs::create_dir_all(&app).unwrap();
    fs::write(
        app.join("App.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    )
    .unwrap();
    let lib = dir.path().join("src/Lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(
        lib.join("Lib.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a .NET solution that lists no projects (degenerate).
pub fn create_degenerate_dotnet_solution() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Empty.sln"),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a git-backed mixed workspace with nested frontend packages.
///
/// The root Cargo workspace lists `server` as its single member, and the root
/// pnpm workspace lists two frontend packages so its layer resolves
/// non-degenerately. The Cargo root is virtual (no `[package]`), so the Cargo
/// layer is degenerate on its own; the pnpm layer is what makes this repo a
/// monorepo.
pub fn create_mixed_nested_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let _repo = Repository::init(dir.path()).unwrap();

    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"server\"]\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'server/frontend'\n  - 'server/admin'\n",
    )
    .unwrap();
    fs::write(dir.path().join("package.json"), r#"{"private": true}"#).unwrap();

    let server_dir = dir.path().join("server");
    fs::create_dir_all(server_dir.join("src")).unwrap();
    fs::write(
        server_dir.join("Cargo.toml"),
        "[package]\nname = \"homelab-server\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(server_dir.join("src/main.rs"), "fn main() {}").unwrap();

    let frontend_dir = server_dir.join("frontend");
    fs::create_dir_all(frontend_dir.join("src")).unwrap();
    fs::write(
        frontend_dir.join("package.json"),
        r#"{"name":"homelab-frontend","version":"0.1.0"}"#,
    )
    .unwrap();
    fs::write(frontend_dir.join("src/index.ts"), "console.log('frontend')").unwrap();

    let admin_dir = server_dir.join("admin");
    fs::create_dir_all(&admin_dir).unwrap();
    fs::write(
        admin_dir.join("package.json"),
        r#"{"name":"homelab-admin","version":"0.1.0"}"#,
    )
    .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Bazel workspace forest: a parent `WORKSPACE` with two `BUILD`
/// packages plus a nested `WORKSPACE` whose subtree the parent must exclude.
pub fn create_bazel_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("WORKSPACE"), "").unwrap();
    fs::create_dir_all(root.join("a")).unwrap();
    fs::write(root.join("a/BUILD"), "# a\n").unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    fs::write(root.join("b/BUILD.bazel"), "# b\n").unwrap();
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("nested/WORKSPACE"), "").unwrap();
    fs::write(root.join("nested/BUILD"), "# nested\n").unwrap();
    let path = root.to_path_buf();
    (dir, path)
}

/// Create a Bazel root with no `BUILD` files (degenerate).
pub fn create_degenerate_bazel_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("WORKSPACE"), "").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Pants workspace (`pants.toml` with two leaf `BUILD.pants` files).
pub fn create_pants_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("pants.toml"), "[GLOBAL]\n").unwrap();
    fs::create_dir_all(root.join("src/app")).unwrap();
    fs::write(root.join("src/app/BUILD.pants"), "python_sources()\n").unwrap();
    fs::create_dir_all(root.join("src/lib")).unwrap();
    fs::write(root.join("src/lib/BUILD.pants"), "python_sources()\n").unwrap();
    let path = root.to_path_buf();
    (dir, path)
}

/// Create a Pants root with no leaf build files (degenerate).
pub fn create_degenerate_pants_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("pants.toml"), "[GLOBAL]\n").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Buck2 workspace (`.buckconfig` with leaf `BUCK` / `TARGETS` files).
pub fn create_buck2_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join(".buckconfig"), "[cells]\n").unwrap();
    fs::create_dir_all(root.join("app")).unwrap();
    fs::write(root.join("app/BUCK"), "# app\n").unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
    fs::write(root.join("lib/TARGETS"), "# lib\n").unwrap();
    let path = root.to_path_buf();
    (dir, path)
}

/// Create a Buck2 root with no leaf build files (degenerate).
pub fn create_degenerate_buck2_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".buckconfig"), "[cells]\n").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Rush Stack monorepo (`rush.json` listing two project folders).
pub fn create_rush_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(
        root.join("rush.json"),
        "{\n  \"rushVersion\": \"5.0.0\",\n  \"projects\": [\n\
             { \"packageName\": \"app\", \"projectFolder\": \"apps/app\" },\n\
             { \"packageName\": \"lib\", \"projectFolder\": \"libraries/lib\" }\n  ]\n}\n",
    )
    .unwrap();
    let app = root.join("apps/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let lib = root.join("libraries/lib");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();
    let path = root.to_path_buf();
    (dir, path)
}

/// Create a Rush `rush.json` that lists no projects (degenerate).
pub fn create_degenerate_rush_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("rush.json"),
        "{\n  \"rushVersion\": \"5.0.0\",\n  \"projects\": []\n}\n",
    )
    .unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a forest with two pnpm roots: a non-degenerate workspace at the
/// root (two members) and a degenerate sibling under `nested/` (one member).
///
/// pnpm's `RootMembership::Never` policy means a single-member pnpm workspace
/// is degenerate — its membership does not resolve non-degenerately. The
/// confidence for the nested (degenerate) `DetectedStandard` must stay
/// `Inferred` even though the root (non-degenerate) pnpm layer is
/// `MarkerConfirmed`. Without a per-root confidence tie-breaker the
/// degenerate sibling would inherit confirmation from its non-degenerate twin.
pub fn create_pnpm_forest_with_degenerate_sibling() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Root pnpm workspace — non-degenerate (two members).
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(root.join("package.json"), "{}").unwrap();
    for name in ["app", "lib"] {
        let pkg = root.join("packages").join(name);
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("package.json"), format!(r#"{{"name":"{name}"}}"#)).unwrap();
    }

    // Nested pnpm workspace under nested/ — degenerate (one member). pnpm
    // allows nested instances, so this produces its own layer.
    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("pnpm-workspace.yaml"),
        "packages:\n  - 'pkgs/*'\n",
    )
    .unwrap();
    let solo = nested.join("pkgs/solo");
    fs::create_dir_all(&solo).unwrap();
    fs::write(solo.join("package.json"), r#"{"name":"solo"}"#).unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a git-init'd bare-root repo with two nested standards under
/// separate subtrees: a pnpm workspace at `web/` and a .NET solution at
/// `dotnet/`.
///
/// The root itself has no workspace marker so only the nested walk can
/// surface either layer. The deep `web/packages/app/package.json` ensures
/// the second-level depth is also walked by the single-pass scan.
pub fn create_nested_pnpm_and_dotnet_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let _repo = Repository::init(dir.path()).unwrap();
    let root = dir.path();

    // Nested pnpm workspace under web/.
    let web = root.join("web");
    fs::create_dir_all(&web).unwrap();
    fs::write(
        web.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(web.join("package.json"), "{}").unwrap();
    let web_app = web.join("packages/app");
    fs::create_dir_all(&web_app).unwrap();
    fs::write(web_app.join("package.json"), r#"{"name":"app"}"#).unwrap();
    let web_lib = web.join("packages/lib");
    fs::create_dir_all(&web_lib).unwrap();
    fs::write(web_lib.join("package.json"), r#"{"name":"lib"}"#).unwrap();

    // Nested .NET solution under dotnet/. The .sln lists a real .csproj so
    // the dotnet detector resolves a non-degenerate package set.
    let dotnet = root.join("dotnet");
    fs::create_dir_all(&dotnet).unwrap();
    fs::write(
        dotnet.join("MyApp.sln"),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n\
         Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"App\", \"src\\App\\App.csproj\", \"{1}\"\n\
         EndProject\n",
    )
    .unwrap();
    let app_proj = dotnet.join("src/App");
    fs::create_dir_all(&app_proj).unwrap();
    fs::write(
        app_proj.join("App.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    )
    .unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a repo whose root has a single `package.json` and no nested
/// directories.
///
/// Exercises the `parent == root` skip in `walk_for_nested_markers`: a marker
/// directly at the repo root must not register the root itself as a nested
/// candidate (nested discovery is non-root only).
pub fn create_nested_marker_at_root_to_be_ignored() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("package.json"), r#"{"name":"root"}"#).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a repo with a non-workspace package at `app/` plus a real pnpm
/// workspace buried inside a `node_modules` subtree.
///
/// `node_modules` is in the prune list (`should_skip_directory_name`), so
/// `filter_entry` keeps the walker from descending into it. The pruned subtree
/// holds a `pnpm-workspace.yaml` that declares a real `packages/app` member: if
/// the walker descended into `node_modules`, the candidate would dispatch to
/// `detect_pnpm_workspace`, resolve the member, and produce a PnpmWorkspaces
/// layer rooted under `node_modules`. Working prune yields no such candidate
/// and no layer. The top-level `app/package.json` carries no `workspaces`
/// field, so it dispatches but self-filters to `None`, leaving the repo with
/// zero monorepo layers — making the prune the only thing that can introduce a
/// layer.
pub fn create_pruned_node_modules_with_package_json() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Real (non-workspace) package at app/.
    let app = root.join("app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();

    // Pruned subtree holding a real pnpm workspace with a resolvable member.
    let workspace = app.join("node_modules/workspace");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    let member = workspace.join("packages/app");
    fs::create_dir_all(&member).unwrap();
    fs::write(member.join("package.json"), r#"{"name":"nm-app"}"#).unwrap();

    let path = root.to_path_buf();
    (dir, path)
}

/// Create a git-init'd repo whose `nested/` subtree contains a
/// `pnpm-workspace.yaml` that is ignored via a local `nested/.gitignore`.
///
/// The ignored `pnpm-workspace.yaml` declares a real `packages/app` member, so
/// the old per-directory `Path::exists()` probe WOULD have produced a
/// PnpmWorkspaces layer here: `exists()` bypasses the walker's `git_ignore`
/// filter, the candidate at `nested/` dispatches to `detect_pnpm_workspace`,
/// and the yaml resolves the member. The new single-pass `ignore` walker honors
/// `git_ignore` and never yields the gitignored `pnpm-workspace.yaml`, so no
/// pnpm candidate and no layer. The member's own `packages/app/package.json` is
/// not gitignored but carries no `workspaces` field, so the npm/yarn/bun
/// detectors self-filter to `None`. This is the intentional behavior delta:
/// see the spec's "Intentional Behavior Change" section.
pub fn create_gitignored_nested_marker() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let _repo = Repository::init(dir.path()).unwrap();
    let root = dir.path();

    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();
    // Local gitignore rule that drops the workspace marker in this subtree.
    fs::write(nested.join(".gitignore"), "pnpm-workspace.yaml\n").unwrap();
    // The marker itself, ignored by the rule above. Under the old `exists()`
    // probe this would have produced a real PnpmWorkspaces layer.
    fs::write(
        nested.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    // A real (non-gitignored) member so the old probe resolves a layer.
    let app = nested.join("packages/app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();

    let path = root.to_path_buf();
    (dir, path)
}
