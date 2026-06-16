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

/// Create a pnpm workspace whose `pnpm-lock.yaml` `importers:` agree with the
/// manifest globs.
pub fn create_pnpm_workspace_with_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_pnpm_workspace();
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n\nimporters:\n  \\n  .:\n    dependencies: {}\n  packages/app:\n    dependencies: {}\n  packages/lib:\n    dependencies: {}\n",
    )
    .unwrap();
    (dir, path)
}

/// Create a pnpm workspace whose `pnpm-lock.yaml` is missing a manifest member.
pub fn create_pnpm_workspace_with_drifted_lockfile() -> (TempDir, PathBuf) {
    let (dir, path) = create_pnpm_workspace();
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '6.0'\n\nimporters:\n  \\n  .:\n    dependencies: {}\n  packages/app:\n    dependencies: {}\n",
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

/// Create a git-backed mixed workspace with a nested frontend package.
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
        "packages:\n  - 'server/frontend'\n",
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
