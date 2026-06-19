---
prompt: |-
    - Review the codebase to determine what the enum is that enumerates all the variant monorepo tech standards (cargo, pnpm workspaces, etc.).
    - Then do research on monorepo technologies in general to see if we're missing any important ones.

    Iterate over each monorepo technology and:

    - validate how we can detect:
        - when this monorepo technology is being used in a monorepo
        - what files? what properties in what files?
        - how can we test if the necessary binary for working with this monorepo is installed on the host?
    - other metadata:
        - how can we detect the packages defined in the monorepo?
        - is there a specific programming language which this technology is tied to? 
            - if there is we should capture that static relationship
            - if there is not, is there a way to determine from the monorepo standard which programming languages are used?
        - what is the associated binaries API surface?
        - how does a terminal user at the root of a monorepo run operations on a targetted package within the monorepo? What about running commands across the packages? 
    - does this monorepo technology have any variances based on the OS (windows, macOS, linux)?

    Edge Cases

    - research what some of the most promenent "edge cases" for how a monorepo is configured/setup that could cause problems in a naive implementation
    - how can one solve for these edge cases

    Gap Analysis

    - based on what we have in the source code now, how well are we capturing information about these test runners? What are the gaps that should be filled?
    - what meta data besides just the name of the monorepo and the package it contains should we be capturing?

    > **Note:** currently sniff is not detecting whether a monorepo is or is not a monorepo correctly; that's not your problem to solve but don't imagine that the code provides hints we can reliably use for detection.
last_updated: 2026-06-15
---

# Monorepo Technologies: Detection, Metadata, and Gap Analysis

## 1. The Current Enum

### The single source of truth for "which monorepo standard is this?" is **`MonorepoTool`** in `sniff/lib/src/filesystem/repo/types.rs:18`:

```rust
#[non_exhaustive]
pub enum MonorepoTool {
    CargoWorkspace,
    NpmWorkspaces,
    PnpmWorkspaces,
    YarnWorkspaces,
    Nx,
    Turborepo,
    Lerna,
    Unknown,
}
```

A parallel **`PackageEcosystem`** enum (`types.rs:41`) tags each discovered package as `Cargo | Node | Python | Go | Unknown`, and **`PackageDiscoverySource`** (`types.rs:59`) records *how* a package boundary was found (`CargoWorkspace | PnpmWorkspace | NpmWorkspace | YarnWorkspace | Nx | Turborepo | Lerna | ManifestScan`).

Detection is gated by a marker-file fast path in `detection.rs:121` (`has_workspace_marker`) and then each variant runs its own detector:

| Variant          | Detector                                        | Root marker consumed                                  |
|------------------|-------------------------------------------------|-------------------------------------------------------|
| `CargoWorkspace` | `cargo::detect_cargo_workspace` (`cargo.rs:15`) | `Cargo.toml` with a `[workspace] members` array       |
| `NpmWorkspaces`  | `npm::detect_npm_workspace` (`npm.rs:149`)      | root `package.json` with a `workspaces` field         |
| `PnpmWorkspaces` | `npm::detect_pnpm_workspace` (`npm.rs:114`)     | `pnpm-workspace.yaml` with a `packages:` sequence     |
| `YarnWorkspaces` | `npm::detect_yarn_workspace` (`npm.rs:184`)     | `yarn.lock` **plus** root `package.json` `workspaces` |
| `Nx`             | `nx_turbo::detect_nx` (`nx_turbo.rs:16`)        | `nx.json`                                             |
| `Turborepo`      | `nx_turbo::detect_turborepo` (`nx_turbo.rs:63`) | `turbo.json`                                          |
| `Lerna`          | `nx_turbo::detect_lerna` (`nx_turbo.rs:110`)    | `lerna.json`                                          |

All seven are JavaScript/Rust-centric. As the prompt notes, the **`is_monorepo` signal itself is currently unreliable**; this document does not try to fix that but treats detection of *which tool* as an independent, well-defined problem.

---

## 2. Landscape: What's Missing

The enum covers the Node.js workspace family (npm/pnpm/Yarn + the Nx/Turbo/Lerna task runners) and Cargo. It is missing every other major ecosystem's monorepo standard. Ranked by how often a real-world `sniff` user will hit them:

| Missing technology                      | Ecosystem                      | Prevalence                                                 |
|-----------------------------------------|--------------------------------|------------------------------------------------------------|
| **Bun workspaces**                      | Node                           | Rising fast; `bun install` now reads `workspaces` natively |
| **uv workspaces**                       | Python                         | de-facto modern Python monorepo (Astral)                   |
| **Go workspace mode** (`go.work`)       | Go                             | Go 1.20+ standard                                          |
| **Gradle multi-project builds**         | JVM (Java/Kotlin/Scala/Groovy) | Dominant JVM monorepo layout                               |
| **Maven multi-module**                  | JVM                            | The other half of JVM monorepos                            |
| **Bazel**                               | Polyglot                       | The "enterprise polyglot" monorepo tool                    |
| **Pants**                               | Python/polyglot                | Smaller but growing                                        |
| **Buck2**                               | Polyglot (Meta)                | Used at scale at Meta and a few others                     |
| **Swift Package Manager** multi-target  | Swift                          | iOS/server Swift monorepos                                 |
| **Rush**                                | Node                           | Microsoft's Node monorepo orchestrator                     |
| **.NET solutions** (`*.sln` + projects) | C#/F#/VB                       | The Windows monorepo standard                              |
| **SBT / Mill** multi-module             | Scala                          | Niche but real                                             |
| **Mix umbrella**                        | Elixir                         | Niche                                                      |

The three with the strongest claim to immediate inclusion are **Bun workspaces**, **uv workspaces**, and **Gradle/Maven** — the first two because they reuse manifests sniff already parses (`package.json`, `pyproject.toml`), and the JVM pair because they represent an entire ecosystem sniff currently cannot recognize as a monorepo at all.

A second axis missing from the enum entirely: **orchestrators vs. workspace standards**. Nx, Turbo, and Lerna are *task runners* that sit *on top of* an npm/pnpm/Yarn workspace. They are not mutually exclusive with the JS workspace variants — in practice `nx.json` almost always coexists with `package.json#workspaces` or `pnpm-workspace.yaml`. The enum conflates these two layers, which is why `RepoInfo.workspace_tools` (a `Vec`) exists alongside the singular `monorepo_tool`. This conflation will matter for every recommendation below.

---

## 3. Per-Technology Deep Dive

### 3.1 Cargo Workspaces (present)

**Detection — files & properties**

- Marker: `Cargo.toml` at the candidate root containing a `[workspace]` table.
- The signal that makes it a *workspace* (not a single crate) is a **non-empty `members` array** (`cargo.rs:33`). `[workspace]` with no members is a "root exists but nothing belongs" anti-pattern and sniff correctly returns `None`.
- `members` and `exclude` accept glob patterns like `sniff/*` and `crates/**`. `expand_glob_patterns_with_deps` (`detection.rs:972`) expands a subset of glob syntax — note the limitation in §5.
- A `default-members` key may also exist; it controls which crates `cargo build` targets by default but does **not** change membership.

**Binary presence check**

- Look for `cargo` on `PATH` (already modelled via `LanguagePackageManager::Cargo` in `registry.rs:87`). The `PackageManagerShape::is_available()` trait is the right hook.
- Version probing: `cargo --version`. For workspace features you sometimes care about the edition (`cargo_toml.get("workspace").get("package").get("edition")`), but that is informational.

**Enumerating packages**

- Expand `members` globs, subtract `exclude` globs. Each member dir has its own `Cargo.toml` whose `[package].name` is the canonical package name (`cargo.rs:183`).
- The workspace root itself can *also* be a package (`[package]` alongside `[workspace]`). sniff does not currently surface the root crate as a package when it is also the workspace root — worth noting as a metadata gap.
- Path dependencies inside the workspace (`path = "../foo"`) are an *alternative* monorepo signal that is weaker than workspace membership but common in older Rust repos.

**Language tie**

- Static: Rust, 100%. There is no ambiguity. `PackageEcosystem::Cargo` is correct by construction.

**Binary API surface**

- `cargo build`, `cargo check`, `cargo test`, `cargo run`, `cargo doc`, `cargo fmt`, `cargo clippy`, `cargo metadata`.
- `cargo metadata --no-deps --format-version 1` is the **authoritative** machine-readable workspace listing — the `AGENTS.md` for this very repo says so. sniff currently re-implements member expansion by hand; `cargo metadata` is the ground truth and would also report `workspace_default_members`, `workspace_root`, and per-package `features`/`manifest_path` without any globbing.

**Running on a targeted package**

- `cargo build -p <name>` / `--package <name>`.
- Multiple: `cargo build -p a -p b`.
- By dir: `cargo build --manifest-path sniff/lib/Cargo.toml`.

**Running across all packages**

- `cargo build` (uses `default-members`, or all members with `--workspace`/`--all`).
- `cargo test --workspace`.

**OS variances**

- None for the *format*. `Cargo.toml` is identical on Windows/macOS/Linux.
- Path separators in `members` globs are always `/` even on Windows — Cargo normalizes. A naive `Path::join` that produces `\` will break matching on Windows.
- `CARGO_HOME` and target dir differ per OS but are not monorepo-detection concerns.

---

### 3.2 npm workspaces (present)

**Detection — files & properties**

- Marker: root `package.json` with a top-level `workspaces` field.
- `workspaces` can be either:

    - an **array** of globs: `"workspaces": ["packages/*", "apps/*"]`, or
    - an **object** with a `packages` array: `"workspaces": { "packages": [...] }` (the Yarn-style form; `parse_package_json_workspace_patterns` at `npm.rs:242` handles both).

- npm 7+ is the first npm version that understands `workspaces`; the field pre-existed as a Yarn convention.

**Binary presence check**

- `npm` on PATH (`LanguagePackageManager::Npm`, `registry.rs:63`). `npm --version` ≥ 7 ideally.
- sniff's `resolve_js_package_manager` (`npm.rs:279`) already infers npm vs. pnpm vs. yarn vs. bun from lockfiles. **npm workspaces** specifically implies the `npm` binary, but the lockfile-resolution logic is what should drive the *binary*, while `workspaces` in `package.json` drives the *workspace standard*.

**Enumerating packages**

- Expand the globs; each matched dir must contain a `package.json`. The `name` field of that `package.json` is the package identity.

**Language tie**

- Node.js primary. A workspace package may contain TypeScript, but the *ecosystem* is Node. `PackageEcosystem::Node`.

**Binary API surface**

- `npm install` (installs the whole workspace, links internal deps), `npm run`, `npm test`, `npm exec`, `npm publish`.
- `npm ls --workspaces --all` lists workspace packages.

**Targeted / across packages**

- Targeted: `npm run build --workspace <name>` (or `-w <name>`). `--workspace` accepts a path or a name.
- Multiple: repeat `-w`, or `npm run build --workspaces` (alias `--ws`) for *all*.
- Excluding the root: `npm run build --workspaces --if-present`.

**OS variances**

- `package.json` is portable. Scripts in `"scripts"` may shell out (`&&`, `rm`) and break cross-platform, but that is a *package* concern, not a workspace-detection concern. Node's own install path differs per OS.

---

### 3.3 pnpm workspaces (present)

**Detection — files & properties**

- Marker: `pnpm-workspace.yaml` with a top-level `packages:` sequence (`npm.rs:114`, `parse_pnpm_workspace_patterns` at `npm.rs:223`).
- As of **pnpm 10+**, `pnpm-workspace.yaml` has grown: it now also hosts ` catalogs`, `onlyBuiltDependencies`, `peerDependencyRules`, `ignoredOptionalDependencies`, and most settings that used to live in `.npmrc`. **Detection of "is this a pnpm workspace" must remain keyed on the file's existence and a non-empty `packages:` list** — do not assume the file only ever contains `packages:`.
- A leading `onlyBuiltDependencies:` or `catalogs:` with no `packages:` is *possible* in v10 but rare; treat absent/empty `packages` as "not a workspace" to match current behavior.

**Binary presence check**

- `pnpm` on PATH (`LanguagePackageManager::Pnpm`). `pnpm-lock.yaml` at root is a strong secondary signal — `resolve_js_package_manager` already uses it.

**Enumerating packages**

- Expand `packages:` globs; each dir contains a `package.json`.
- pnpm also supports `workspace:` protocol in dependencies (`"foo": "workspace:*"`), `workspace:../foo` relative paths, and `workspace:foo@*` aliases — these are dependency-resolution details, not membership signals, but they are useful for building the internal-dependency graph.

**Language tie**

- Node.js. `PackageEcosystem::Node`.

**Binary API surface**

- `pnpm install`, `pnpm run`, `pnpm test`, `pnpm exec`, `pnpm publish`, `pnpm -r <cmd>` (recursive).
- `pnpm list -r --depth -1` enumerates every workspace package.

**Targeted / across packages**

- Targeted: `pnpm --filter <name> build`. `--filter` accepts name, path, glob (`./packages/*`), or selectors like `...^pkg` and `{pkg-name}`.
- Across all: `pnpm -r build` (recursive). `pnpm --filter "./packages/**" build`.
- Including the root: `pnpm -r --workspace-root build` or set `includeWorkspaceRoot: true` in config.

**OS variances**

- Same portability profile as npm. The pnpm store location (`PNPM_HOME`) differs per OS but is not a detection concern.

---

### 3.4 Yarn workspaces (present)

**Detection — files & properties**

- sniff requires **both** `yarn.lock` *and* root `package.json#workspaces` (`npm.rs:184`). This is a deliberate disambiguator from npm workspaces (which uses the same `package.json` field).
- `package.json#workspaces` may be an array or `{ packages: [...] }` (same parser as npm).
- **Yarn Classic vs. Yarn Berry (v2+):** both produce `yarn.lock`, but the file *format* differs (Classic text vs. Berry YAML-ish). Berry adds `.yarnrc.yml` and a `.yarn/` directory. sniff's `LanguagePackageManager::{YarnClassic, YarnBerry}` (`registry.rs:73`) models both but detection does not distinguish them.

**Binary presence check**

- `yarn` on PATH. `yarn --version` ≥ 2 implies Berry. sniff models `Yarn`, `YarnClassic`, `YarnBerry` (`registry.rs:71`).

**Enumerating packages**

- Same as npm: glob expansion over `workspaces`.

**Language tie**

- Node.js.

**Binary API surface**

- Classic: `yarn install`, `yarn workspace <name> run build`, `yarn workspaces run build`.
- Berry: `yarn install`, `yarn workspace <name> run build`, `yarn workspaces foreach --all run build`.

**Targeted / across packages**

- Classic targeted: `yarn workspace <pkg> build`. Across: `yarn workspaces run build`.
- Berry targeted: `yarn workspace <pkg> build`. Across: `yarn workspaces foreach --all --topological run build` (`--from`, `--no-private`, `-j` for parallelism are Berry-only).

**OS variances**

- Format portable. Berry's `enableGlobalCache` and PnP resolution interact with platform paths but are not detection concerns.

> **Note for the enum:** Yarn Classic's `yarn workspaces foreach` does not exist — that's Berry-only. If sniff ever emits "run across packages" instructions, it must branch on Yarn version.

---

### 3.5 Nx (present)

**Detection — files & properties**

- Marker: `nx.json` (`nx_turbo.rs:16`).
- `nx.json` typically contains `extends`, `targetDefaults`, `namedInputs`, and optionally `workspaceLayout` (`nx_turbo.rs:176` parses `appsDir`/`libsDir`, defaulting to `apps`/`libs`).
- **Nx is an orchestrator, not a workspace standard.** An Nx monorepo almost always *also* has `package.json#workspaces` or `pnpm-workspace.yaml`. sniff's `collect_default_workspace_patterns` (`detection.rs:271`) deliberately merges in patterns from those files so Nx package discovery works.

**Binary presence check**

- `nx` on PATH (often `npx nx`). With pnpm, `pnpm exec nx`.

**Enumerating packages**

- `nx show projects` (formerly `nx print-affected`) is authoritative. sniff falls back to pattern expansion + manifest index walking.
- **Nx "integrated" vs. "package-based":** integrated repos name projects in `nx.json#sourcePatterns` / project configs; package-based repos use the npm-workspaces glob layout. A single repo can have "standalone" projects configured via `project.json` files that are *not* in any glob.

**Language tie**

- Historically TypeScript/Node. Modern Nx supports **Gradle, .NET, Go, Rust, Python, Swift** via community plugins. There is **no static language tie**; infer from per-package manifests as sniff already does.

**Binary API surface**

- `nx run <project>:<target>`, `nx build`, `nx test`, `nx lint`, `nx serve`, `nx affected`, `nx graph`, `nx show projects`, `nx run-many`.

**Targeted / across packages**

- Targeted: `nx build my-app`, or `nx run my-app:build`.
- Across: `nx run-many --target=build --all`, `nx run-many --target=test --projects=a,b`.
- Changed-only: `nx affected --target=build`.

**OS variances**

- Nx itself is portable. The underlying language toolchains it shells out to (MSBuild on Windows, etc.) are not — relevant when sniff reports "languages used."

---

### 3.6 Turborepo (present)

**Detection — files & properties**

- Marker: `turbo.json` (`nx_turbo.rs:63`). Contains `tasks` (formerly `pipeline`), `globalDependencies`, optional `workspace` field.
- Like Nx, **Turbo is an orchestrator layered on npm/pnpm/Yarn workspaces.** A `turbo.json` without an underlying JS workspace is malformed.

**Binary presence check**

- `turbo` on PATH (often `npx turbo` or via `turborepo`).

**Enumerating packages**

- Delegated to the underlying package manager. `turbo` reads `package.json#workspaces` / `pnpm-workspace.yaml`.

**Language tie**

- JS-first but language-agnostic in principle; infer from manifests.

**Binary API surface**

- `turbo build`, `turbo test`, `turbo lint`, `turbo run <task>`, `turbo daemon`.

**Targeted / across packages**

- Targeted: `turbo build --filter=<pkg>`.
- Across: `turbo build` (runs the task across all packages that define it). `--filter=...` supports `^`, `...`, dependencies/dependents syntax.
- `--concurrency`, `--continue`, `--parallel` for execution control.

**OS variances**

- Portable.

---

### 3.7 Lerna (present)

**Detection — files & properties**

- Marker: `lerna.json` (`nx_turbo.rs:110`). Contains `packages` (glob array), `version`/`useWorkspaces`, `npmClient`.
- **Lerna is essentially deprecated in favor of Nx** (merged into Nx's `@nx/lerna` package since 2022). Lerna ≥ 6 is a thin Nx wrapper. Detection of `lerna.json` is still correct for legacy repos.
- `useWorkspaces: true` tells Lerna to defer to Yarn/Nx workspace membership rather than `lerna.json#packages`.

**Binary presence check**

- `lerna` on PATH.

**Enumerating packages**

- `lerna.json#packages` globs, or the underlying Yarn workspace config when `useWorkspaces: true`.

**Language tie**

- Node.js.

**Binary API surface**

- `lerna run <script>`, `lerna exec`, `lerna bootstrap` (deprecated in v6+), `lerna version`, `lerna publish`.

**Targeted / across packages**

- Targeted: `lerna run build --scope=<pkg>` (with `--since`, `--include-filtered-dependencies`).
- Across: `lerna run build` (all packages).

**OS variances**

- Portable.

---

### 3.8 Bun workspaces (MISSING — recommend adding `BunWorkspaces`)

**Detection — files & properties**

- Bun reads the **standard `package.json#workspaces`** field — the same as npm. There is no separate `bun-workspaces.json`.
- The distinguishing signal that this is a *Bun* workspace (rather than npm/Yarn) is the presence of **`bun.lock`** (text format, Bun ≥ 1.2) or the legacy **`bun.lockb`** (binary, pre-1.2) at the root.
- sniff's `resolve_js_package_manager` (`npm.rs:298`) already checks `bun.lock` / `bun.lockb`, so the *binary* is detected but there is **no `MonorepoTool::BunWorkspaces` variant**. Today a Bun-managed JS monorepo is mis-reported as `NpmWorkspaces`.

**Binary presence check**

- `bun` on PATH (`LanguagePackageManager::Bun`, `registry.rs:83`). `bun --version`.

**Enumerating packages**

- Identical to npm: expand `workspaces` globs.

**Language tie**

- Node.js / Bun (JavaScript/TypeScript). `PackageEcosystem::Node`.

**Binary API surface**

- `bun install`, `bun run`, `bun test`, `bun --filter`.

**Targeted / across packages**

- Targeted: `bun run --filter <pkg> build` (Bun ≥ 1.1 supports `--filter` with glob/path selectors).
- Across: `bun run build` (runs in root) — for recursive, `bun --filter '*' run build`.

**OS variances**

- Portable. Bun on Windows is stable as of 1.1.

---

### 3.9 uv workspaces (MISSING — recommend adding `UvWorkspace`)

**Detection — files & properties**

- Marker: a `pyproject.toml` containing a **`[tool.uv.workspace]`** table with a required **`members`** glob array and optional `exclude`. (Confirmed against current Astral docs — uv "is inspired by Cargo.")
- Every member directory must itself contain a `pyproject.toml`.
- A single `uv.lock` lives at the workspace root.
- **Easy to miss:** the workspace root is *also* a workspace member (its `[project]` is the root package), unlike some conventions where the root is purely administrative.

**Binary presence check**

- `uv` on PATH. `uv --version`. uv is not yet in sniff's `LanguagePackageManager` registry.

**Enumerating packages**

- Expand `members`, subtract `exclude`. Each member has `[project].name`.
- Internal dependencies are declared via `[tool.uv.sources]` with `workspace = true` (e.g. `bird-feeder = { workspace = true }`). This is the high-fidelity internal-dependency signal, equivalent to pnpm's `workspace:` protocol.

**Language tie**

- Python, 100%. `PackageEcosystem::Python`.

**Binary API surface**

- `uv sync`, `uv lock`, `uv run`, `uv add`, `uv pip`, `uv build`, `uv publish`.

**Targeted / across packages**

- Targeted: `uv run --package <name> pytest`. Acceptable from any workspace dir.
- Across: `uv run` defaults to the root; there is no "run across all members" primitive — uv operates on the workspace as a unit via the single lockfile.

**OS variances**

- Portable. `requires-python` is intersected across members at lock time.

---

### 3.10 Go workspace mode / `go.work` (MISSING — recommend adding `GoWorkspace`)

**Detection — files & properties**

- Marker: **`go.work`** at the candidate root (Go 1.20+). Contains `go <version>`, `use <module-path>` directives, optional `replace`/`exclude`.
- Each `use` points at a directory containing its own `go.mod` (a Go *module*).
- Distinguish from a single module: a lone `go.mod` is **not** a workspace; only `go.work` is.
- `go.work.sum` is the workspace checksum file.

**Binary presence check**

- `go` on PATH (`go version` ≥ 1.20 for workspace support). sniff parses `go.mod` (`go.rs`) but has no Go workspace concept.

**Enumerating packages**

- Parse `go.work` `use` directives → each is a member module. The module name lives in that module's `go.mod` `module` line (`go_module_name_from_content`).
- Go's notion of "package" is finer-grained than a module (every directory with `*.go` files is a package), but for monorepo purposes the **module** is the package boundary, matching how sniff models Cargo crates.

**Language tie**

- Go, 100%. `PackageEcosystem::Go`.

**Binary API surface**

- `go build ./...`, `go test ./...`, `go run`, `go work edit`, `go work sync`, `go list ./...`.
- `GOWORK=off` disables workspace mode.

**Targeted / across packages**

- Targeted module: `cd <module> && go test ./...`, or `go test example.com/mod/...` from root.
- Across all modules in the workspace: `go test ./...` from the `go.work` root resolves through all `use`d modules.

**OS variances**

- Format portable. `GOOS`/`GOARCH` matter for builds but not detection. Module paths use forward slashes universally.

---

### 3.11 Gradle multi-project builds (MISSING — recommend adding `GradleComposite` or `GradleMultiProject`)

**Detection — files & properties**

- Marker: **`settings.gradle`** or **`settings.gradle.kts`** at the root containing `include '...` / `include("...")` statements. This is the canonical "this is a multi-project Gradle build" signal.
- A sub-project is conventionally a directory containing a `build.gradle` / `build.gradle.kts`; its name is the string passed to `include`.
- The **Gradle Wrapper** (`gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.properties`) should be treated as the *preferred* entrypoint — sniff should report its presence.
- Distinguish from **Composite Builds**, which use `includeBuild` in `settings.gradle(.kts)` and compose *separate* Gradle builds. Both are "monorepo-shaped" but the discovery model differs.

**Binary presence check**

- `gradle` on PATH, **or** `./gradlew` (Unix) / `gradlew.bat` (Windows) at the root. The wrapper is the strongly-preferred binary.
- `LanguagePackageManager` does not currently model Gradle.

**Enumerating packages**

- Parse `include` calls in `settings.gradle(.kts)`. Each `include "a:b:c"` maps to the directory `a/b/c` relative to the **project directory** (configurable via `projectDir`).
- Authoritative runtime source: `./gradlew projects` (prints the project tree).

**Language tie**

- **No single language.** Gradle is JVM-first (Java, Kotlin, Scala, Groovy) but builds C/C++, Swift, and Android. There is no static tie — infer from per-project plugins (`java`, `kotlin`, `org.jetbrains.kotlin.jvm`, `scala`, `com.android.application`).

**Binary API surface**

- `./gradlew <task>`, `./gradlew build`, `./gradlew test`, `./gradlew projects`, `./gradlew dependencies`, `./gradlew :project:build`.

**Targeted / across packages**

- Targeted: `./gradlew :sub-project:build` (the `:` path syntax identifies the project).
- Across: `./gradlew build` runs the task in every project that defines it. `--parallel`, `--configure-on-demand` for performance.

**OS variances**

- The wrapper script `gradlew` is a POSIX shell script; on Windows the user runs `gradlew.bat`. **Detection must not treat the absence of `gradlew` as "not Gradle" on Windows** — `gradlew.bat` is the canonical Windows entry. Settings/build file names are portable.

---

### 3.12 Maven multi-module (MISSING — recommend adding `MavenMultiModule`)

**Detection — files & properties**

- Marker: a root **`pom.xml`** with a `<packaging>pom</packaging>` parent and a **`<modules>`** element listing `<module>sub-dir</module>` children.
- Each `<module>` path is relative to the parent POM and contains its own `pom.xml`.
- The parent POM usually declares `<groupId>`, `<artifactId>`, `<version>` and `<dependencyManagement>`; children reference the parent via `<parent><relativePath>`.
- **Reactor builds** = multi-module; the `<modules>` element is the signal.

**Binary presence check**

- `mvn` on PATH, **or** `mvnw` / `mvnw.cmd` (Maven Wrapper, added via `mvn -N wrapper:wrapper` since Maven 3.7).

**Enumerating packages**

- Parse `<modules>` from the root POM. Each child's `pom.xml` `<artifactId>` is its identity.
- Authoritative: `mvn validate` builds the reactor; `help:effective-pom` lists modules.

**Language tie**

- JVM (Java primary; Kotlin/Scala/Groovy via plugins). No static tie — infer from plugins (`maven-compiler-plugin`, `kotlin-maven-plugin`).

**Binary API surface**

- `mvn clean install`, `mvn test`, `mvn package`, `mvn verify`, `mvn dependency:tree`, `mvn -pl <module> ...`.

**Targeted / across packages**

- Targeted: `mvn -pl :artifactId clean install` (`-pl`/`--projects`), with `-am` (also-make dependencies) / `-amd` (also-make-dependents).
- Across: `mvn install` builds the whole reactor.

**OS variances**

- `pom.xml` is portable. The wrapper script `mvnw` (POSIX) vs `mvnw.cmd` (Windows) — same dual-script concern as Gradle.

---

### 3.13 Bazel (MISSING — recommend adding `Bazel`)

**Detection — files & properties**

- Marker: **`WORKSPACE`** or **`WORKSPACE.bazel`** at the workspace root (if both exist, `WORKSPACE.bazel` wins).
- Modern Bazel (≥ 7) is migrating to **Bzlmod**, keyed on **`MODULE.bazel`** instead. A repo may have either or both. Detection should accept either as a Bazel signal.
- **Packages** in Bazel are *not* workspace members — they are *any directory containing a `BUILD` or `BUILD.bazel` file*. This is fundamentally different from every other tool here: package boundaries are leaf-ward, not root-ward.

**Binary presence check**

- `bazel` on PATH, **or** `bazelisk` (the version-manager wrapper, widely used). `bazel --version`.

**Enumerating packages**

- Walk the tree for `BUILD` / `BUILD.bazel` files. Each containing directory is a Bazel package; targets within are named in the BUILD file (`name = "..."` attributes).
- Authoritative: `bazel query '...'` / `bazelisk query //...` enumerates all targets. `bazel info workspace` returns the root.

**Language tie**

- **Polyglot by design.** No static tie — infer from the rule families used (`cc_*`, `java_*`, `py_*`, `go_*`, `rust_*` via `rules_rust`, `ts_*`/`js_*`). The *absence* of a Cargo/package.json/pyproject.toml does not mean "no language" — the BUILD file is the manifest.

**Binary API surface**

- `bazel build //pkg:target`, `bazel test //...`, `bazel run`, `bazel query`, `bazel mod` (bzlmod).

**Targeted / across packages**

- Targeted: `bazel build //my/app:app_binary`. The label `//path:target` is the universal addressing scheme.
- Across all: `bazel build //...` (`//...` is the recursive target pattern).
- Affected: `bazel query ` with `rdeps(...)`, or use `bazel-diff`.

**OS variances**

- Format portable. Bazel's `repository_ctx` actions sometimes shell out to platform tools. MSYS2 is commonly required on Windows.

---

### 3.14 Other missing tools (briefer treatment)

**Pants** (v2): `pants.toml` at root. Packages are inferred from source files + `BUILD` files (Pants own format, also `BUILD.pants`). Polyglot (Python, JVM, Go, JS). Binary: `pants` / `pantsw`. Targeted: `pants tailor`, `pants test ::`. Across: `pants test ::` (recursive address spec).

**Buck2** (Meta): `BUCK` files (or `BUCK.bzl`), root `BUCK` for config, `.buckconfig` / `buck2.toml`. Polyglot. Binary: `buck2`. Targeted: `buck2 build //pkg:target`. Across: `buck2 build //...`.

**Swift Package Manager** (multi-target): `Package.swift` at root. Targets (`executableTarget`, `libraryTarget`) are *declared in the manifest*, not inferred from disk. Swift only. Binary: `swift build`, `swift test`. Targeted: `swift build --target <name>`. Across: `swift build`.

**Rush** (Microsoft): `rush.json` at root with `projects` array (each with `projectFolder`, `packageName`). Node.js. Binary: `rush`. Targeted: `rush build --to <pkg>` / `--from`. Across: `rush build`.

**.NET solutions**: `*.sln` / `*.slnx` at root referencing `.csproj`/`.fsproj`/`.vbproj` projects via `Project(...) =` entries. C#/F#/VB. Binary: `dotnet`, `msbuild`. Targeted: `dotnet build src/MyApp`. Across: `dotnet build MySolution.sln`.

**SBT**: `build.sbt` with `lazy val a = (project in file("a"))`. Scala. **Mill**: `build.sc` with `millSourcePath`-based modules.

**Mix umbrella** (Elixir): `mix.exs` at root listing `apps_path: "apps"`, with each app under `apps/`. Elixir. Binary: `mix`.

---

## 4. Edge Cases

The following are the prominent ways a naive implementation gets monorepo detection wrong, with mitigations.

### 4.1 Conflated layers — orchestrator without a workspace, workspace without an orchestrator

- `nx.json` / `turbo.json` / `lerna.json` can exist without `package.json#workspaces` (misconfigured, or using only `project.json` files).
- Conversely, `pnpm-workspace.yaml` exists perfectly well without Nx/Turbo.
- **Mitigation:** Keep the two layers distinct in the data model. `workspace_tools` should include *both* the workspace standard (npm/pnpm/Yarn/Bun/Cargo/uv/Go/Gradle/Maven/Bazel) *and* the orchestrator (Nx/Turbo/Lerna/Rush). Today the enum mixes them; consider splitting or adding an `Orchestrator` sibling enum.

### 4.2 Multiple JS package managers' lockfiles present

- A repo may have both `package-lock.json` *and* `pnpm-lock.yaml` committed (e.g., migration in progress). Only one is canonical.
- **Mitigation:** Detect the *workspace standard* from `package.json#workspaces` / `pnpm-workspace.yaml`, and the *binary* from the lockfile precedence sniff already implements (`resolve_js_package_manager`). Never assume `NpmWorkspaces` just because `package.json#workspaces` exists — pnpm and Yarn also consume it.

### 4.3 `package.json` with an empty or object-form `workspaces`

- `workspaces: []` (empty) and `workspaces: {}` are legal and mean "not actually a workspace." sniff handles both (`npm.rs:155` returns `None` on empty), but a future regex-based detector could miss these.
- **Mitigation:** Treat absent-or-empty `workspaces` as "not a workspace," even though the key is present.

### 4.4 Glob syntax is not uniform

- Cargo/pnpm/npm/Yarn use **minimatch-ish** globs (`packages/*`, `apps/**`). sniff's `expand_glob_patterns_with_deps` (`detection.rs:972`) only splits on `*` and reads a *prefix* — it does **not** support `**`, brace expansion `{a,b}`, or negation `!`. Real monorepos use all three.
- **Mitigation:** Either pull in the `glob` crate (or `biscuit-file`'s globbing if present), or call the package manager's own enumeration (`cargo metadata`, `pnpm list -r`, `npm ls --workspaces`). The latter is more robust.

### 4.5 The root itself is a package

- Cargo: `[workspace]` and `[package]` in the same `Cargo.toml`.
- uv: the workspace root's `[project]` is a workspace member.
- pnpm/npm: `package.json#workspaces` is *in* the root `package.json`, which is itself a package.
- **Mitigation:** Decide explicitly whether the root package appears in `packages`. Currently sniff's behavior here is inconsistent across tools (Cargo excludes the workspace root crate). Document and unify.

### 4.6 Workspace exclusions

- Cargo (`exclude`), uv (`exclude`), pnpm (negation `!` in `packages`) all support excluding paths that a positive glob matched.
- **Mitigation:** Apply exclusions *after* expansion. Mark excluded packages with `is_excluded = true` (already a field on `Package`, `types.rs:189`) rather than dropping them, so consumers can see "this exists but is excluded."

### 4.7 Workspaces nested inside workspaces

- A sub-directory may contain its own `Cargo.toml` with `[workspace]` (a "sub-workspace"). Cargo forbids overlapping workspaces but allows nested *directory* roots.
- Bazel treats a sub-directory `WORKSPACE` file as a **separate workspace** and ignores its tree.
- **Mitigation:** When walking, stop at a nested workspace-root marker for tools that forbid nesting (Bazel), or recurse for tools that allow it. The current `ManifestIndex::build` (`manifest_index.rs:97`) walks unconditionally and may surface phantom nested members.

### 4.8 Fixture / generated manifests masquerading as packages

- Test fixtures (`__fixtures__/`, `testdata/`, `tests/fixtures/`) and generated manifests (`Cargo.toml` containing `auto-generated` / `do not edit`) cause false positives.
- **Mitigation:** `is_fixture_manifest` and `is_generated_manifest` (`manifest_index.rs:258`, `:277`) already filter these. Extend the fixture heuristic per ecosystem (Maven: `src/test`, Gradle: `build/`, Go: `testdata/`).

### 4.9 Case sensitivity & path separators on Windows

- `Cargo.toml` vs `cargo.toml`: Cargo itself is case-sensitive on the marker even on Windows. JS tooling tolerates case via `NODE_OPTIONS=CASE_SENSITIVE_PLAN`. A naive `root.join("Cargo.toml").exists()` will spuriously succeed on case-insensitive filesystems for a typo.
- Glob patterns in manifests always use `/`; on Windows, `Path::join` produces `\`.
- **Mitigation:** Compare path components by exact bytes for markers; normalize separators to `/` when matching glob patterns.

### 4.10 Lockfile-as-ground-truth divergence

- `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `Cargo.lock`, `uv.lock`, `go.work.sum` all encode the *resolved* set of workspace members and their dependencies. They are more reliable than re-parsing globs because they reflect what the tool actually computed.
- **Mitigation:** When available, parse the lockfile to cross-check discovered packages and to fill the internal-dependency graph (`depends_on` / `used_by` on `Package`).

### 4.11 Symlinked / vendored packages

- pnpm symlinks internal deps into per-package `node_modules`. Cargo path dependencies can point outside the workspace root. Yarn Berry can `portal:` link external dirs.
- **Mitigation:** Use `canonicalize_path` (already present, `detection.rs:518`) when deduplicating, but keep the *declared* path in `Package.relative` so reports stay meaningful.

### 4.12 The `is_monorepo` decision is currently wrong

- Per the prompt, sniff does not reliably answer the *binary* "is this a monorepo?" question today. The per-tool detectors are correct *when their marker exists*, but the absence of all markers is treated too coarsely.
- **Mitigation (out of scope per the prompt):** a layered policy — (1) any positive workspace marker → `is_monorepo = true`; (2) else, ≥2 independent package manifests (e.g., two `Cargo.toml` in different trees, or a `Cargo.toml` + `package.json`) → `true` with `MonorepoTool::Unknown`; (3) else `false`. Capture *confidence* (marker-confirmed vs. inferred) as a field.

---

## 5. Gap Analysis vs. Current Source

Comparing §3 against the code, the concrete gaps are:

### 5.1 Enum coverage gaps

- **Missing variants:** `BunWorkspaces`, `UvWorkspace`, `GoWorkspace`, `GradleMultiProject`, `MavenMultiModule`, `Bazel`, `Pants`, `Buck2`, `SwiftPackage`, `Rush`, `DotNetSolution`. Priority order for adding: Bun, uv, Go (cheap — reuse existing manifest parsers), then Gradle, Maven, Bazel (new parsers, big ecosystems).
- **Layer confusion:** `Nx`, `Turborepo`, `Lerna` are orchestrators, not workspace standards. Consider a sibling `Orchestrator` enum and a `RepoInfo.orchestrators: Vec<Orchestrator>` field so `workspace_tools` only contains workspace standards. This resolves the "Nx detected but no npm/pnpm/yarn present" ambiguity.

### 5.2 Marker-file fast path gaps

- `has_workspace_marker` (`detection.rs:121`) hardcodes 7 filenames. Missing: `go.work`, `settings.gradle`, `settings.gradle.kts`, `pom.xml` (with `<modules>`), `WORKSPACE`, `WORKSPACE.bazel`, `MODULE.bazel`, `build.sc` (Mill), `build.sbt`, `mix.exs` (only when `apps_path:` present), `rush.json`, `*.sln`/`*.slnx`, `pants.toml`, `BUCK`, `Package.swift` (multi-target). Each missing marker means a directory tree walk gets skipped (`!has_workspace_marker` short-circuits the manifest index build) and a real monorepo is silently mis-classified as non-monorepo.
- `package.json` is in the marker list but is *not* a workspace marker on its own — a single-package Node app has `package.json`. This causes the manifest-index build to run on every Node app. Consider checking `package.json#workspaces` before counting it as a workspace marker.

### 5.3 Binary-presence detection is disconnected

- sniff has a strong package-manager registry (`registry.rs`) and an `ExecutableIndex` for the programs domain, but **nothing in the repo-detection path reports whether the workspace's binary is installed**. A monorepo detected as `PnpmWorkspaces` should be paired with "is `pnpm` available?" metadata.
- **Recommendation:** add `RepoInfo.tool_binaries: Vec<ToolBinaryStatus>` where each entry is `{ tool: MonorepoTool, binary: &str, installed: bool, version: Option<String> }`. Reuse `get_package_manager(...).is_available()` for the JS/Cargo/Bun pair.

### 5.4 Language tie is inferred at package level, not workspace level

- `PackageEcosystem` is on `Package` (`types.rs:121`). There is no `MonorepoTool → ProgrammingLanguage` static map. For tools with a 100% tie (Cargo→Rust, uv→Python, Go→Go, SwiftPM→Swift), this mapping should be a `const` table, not a runtime scan.
- **Recommendation:** add `impl MonorepoTool { pub fn primary_language(self) -> Option<ProgrammingLanguage> }` returning `Some(Rust)` for `CargoWorkspace`, `Some(Python)` for `UvWorkspace`, `Some(Go)` for `GoWorkspace`, etc., and `None` for polyglot tools (Nx, Turbo, Bazel, Pants).

### 5.5 CLI invocation metadata is not captured

- The enum knows *what* the tool is, but `RepoInfo` does not tell a caller *how to run a command* in a package. Every consumer of sniff (claudine, the commit prompt, etc.) re-derives `cargo build -p X` / `pnpm --filter X` / `mvn -pl :X` by hand.
- **Recommendation:** add per-tool invocation templates, e.g.:
  ```rust
  impl MonorepoTool {
      /// Shell argv template for running `cmd` in `package`.
      pub fn run_in_package_template(self) -> RunTemplate { ... }
      /// Shell argv template for running `cmd` across all packages.
      pub fn run_all_template(self) -> RunTemplate { ... }
  }
  ```
  
    with a small `RunTemplate { program: String, args: Vec<Token> }` where tokens are `Package`/`All`/`Task` placeholders.

### 5.6 Glob expansion is incomplete

- `expand_glob_patterns_with_deps` (`detection.rs:972`) only understands `prefix*`. Real workspaces use `**`, `{a,b}`, `!negation`. This silently drops members in many real monorepos and *also* drives §5.7.
- **Recommendation:** use the `glob` crate (or `biscuit-file`'s globber if it exposes one) for `members`/`packages` expansion, with `**` honoring `.gitignore` semantics. As a fallback, prefer the tool's own enumeration command (`cargo metadata`, `pnpm list -r --parseable`).

### 5.7 Lockfile parsing is Cargo-only

- `CargoLockVersions` (`manifest_index.rs:18`) exists solely to resolve Cargo dependency versions. The JS lockfiles (`pnpm-lock.yaml`, `package-lock.json`, `yarn.lock`), `uv.lock`, and `go.sum`/`go.work.sum` are *not* parsed for resolved versions or for the internal-dependency graph.
- **Recommendation:** `pnpm-lock.yaml` (YAML, already parseable via `serde_yaml_ng`) and `uv.lock` (TOML) would both substantially improve the accuracy of `Package.depends_on` / `used_by` for non-Cargo monorepos.

### 5.8 No "is the binary installed / which version" surface at all

- Beyond §5.3, there is no field that says "this monorepo's orchestrator (Nx) needs `nx`, which is **not** on PATH." This is the single most actionable gap for an agent trying to run commands.

---

## 6. Recommended Metadata Beyond (name, packages)

In addition to `name` and `packages`, a `RepoInfo` for a monorepo should carry:

| Field                       | Type                                            | Why                                                                                                                                          |
|-----------------------------|-------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| `workspace_standards`       | `Vec<MonorepoTool>`                             | Workspace standards detected (npm/pnpm/Yarn/Bun/Cargo/uv/Go/Gradle/Maven/Bazel…). Rename of today's overloaded `workspace_tools`.            |
| `orchestrators`             | `Vec<Orchestrator>`                             | Nx/Turbo/Lerna/Rush/Pants — distinct from workspace standards.                                                                               |
| `tool_binaries`             | `Vec<ToolBinaryStatus>`                         | Per-tool: binary name, installed?, version, found-at path. Reuses `ExecutableIndex`.                                                         |
| `lockfiles`                 | `Vec<PathBuf>`                                  | `Cargo.lock`, `pnpm-lock.yaml`, `uv.lock`, `go.work.sum`, etc. — drives resolved-version and internal-dep graph enrichment.                  |
| `root_is_package`           | `bool`                                          | Whether the workspace root is also a package (Cargo/uv/pnpm semantics differ).                                                               |
| `default_members`           | `Vec<String>`                                   | Cargo's `default-members`; the subset `build`/`test` target by default. Other tools have analogues.                                          |
| `detection_confidence`      | `enum { Marker, Glob, ManifestScan, Inferred }` | Distinguishes "we read `pnpm-workspace.yaml`" from "we guessed because two `Cargo.toml` exist." Addresses the unreliable-`is_monorepo` note. |
| `package_manager_kind`      | `enum`                                          | For JS monorepos: `Npm` / `Pnpm` / `YarnClassic` / `YarnBerry` / `Bun` — distinct from the workspace standard, derived from lockfile.        |
| `wrapper_scripts`           | `Vec<{ tool, posix_path, windows_path }>`       | `gradlew`/`gradlew.bat`, `mvnw`/`mvnw.cmd`. Consumers should prefer the wrapper over the global binary.                                      |
| `internal_dependency_graph` | already on `Package` (`depends_on`, `used_by`)  | Keep, but populate from lockfiles (§5.7) rather than name-matching.                                                                          |
| `config_files`              | `Vec<PathBuf>`                                  | `nx.json`, `turbo.json`, `.yarnrc.yml`, `.npmrc`, `bzlmod` `MODULE.bazel` — for blast-radius and editor-config tooling.                      |

### Priority for closing the gaps

1. **Split orchestrator vs. workspace standard** in the data model (unblocks correct reporting for Nx/Turbo/Lerna).
2. **Add `BunWorkspaces`, `UvWorkspace`, `GoWorkspace`** — all three reuse parsers sniff already has (`package.json`, `pyproject.toml`, `go.mod`).
3. **Extend `has_workspace_marker`** with the missing marker filenames (cheap, prevents silent non-detection).
4. **Add `tool_binaries` + `detection_confidence`** so consumers can both trust the answer and know whether they can act on it.
5. **Add Gradle + Maven + Bazel** to cover the JVM and enterprise-polyglot ecosystems entirely absent today.
6. **Replace the hand-rolled glob expander** with the `glob` crate or with each tool's enumeration command.

---

This document intentionally stops short of proposing code; per the prompt, the broken `is_monorepo` decision and the implementation of any new variant are separate pieces of work. The goal here is the inventory and the contract for what a complete `MonorepoTool` model should report.
