# Test Runner Strategy

Mini-design for the two test-runner surfaces introduced by the *More Repo* feature:

| Surface | Question it answers | Detection mechanism | Spec ref |
|---------|--------------------|--------------------|----------|
| `sniff software test-runners` | "Is this test runner **installed on the host**?" | host PATH / runtime probe (9th software category) | `## Sniff Software` |
| `sniff repo test-runner` | "Which test runner does this **repo/package use**?" | repo key-file + manifest inspection, aggregated across the monorepo | `### sniff repo test-runner` |

These are different questions with different evidence, and conflating them is the main risk. This doc proposes a single source-of-truth `TestRunner` enum that feeds **both** surfaces, plus the per-surface detection logic.

---

## 1. The central problem: test runners don't fit the host-PATH model

The 8 existing software categories (editors, utilities, …) all assume **one program == one global binary on `PATH`** (with macOS-bundle / Windows-registry fallbacks). Test runners violate that assumption in four distinct ways. Every runner we care about falls into exactly one **invocation class**:

| Class | Meaning | Host detection | Examples |
|-------|---------|----------------|----------|
| **A — Path binary** | A real standalone executable discoverable on `PATH` | `which` (standard `ExecutableIndex`) ✅ | `cargo-nextest`, `gotestsum`, `ginkgo`, `rspec`, `pytest`, `nose2`, `tox`, `nox`, `atoum`/`phpunit` PHAR |
| **B — Ecosystem subcommand** | A subcommand built into a parent tool; no binary of its own | parent binary on `PATH` | `cargo test`→`cargo`, `go test`→`go`, `mvn test`→`mvn`, `gradle test`→`gradle`, `dotnet test`→`dotnet`, `mix test`→`mix` |
| **C — Runtime module** | Invoked through a language runtime; no binary, no install | runtime binary on `PATH` | `python -m unittest`→`python`, `node --test`→`node`, Minitest/test-unit→`ruby`/`rake` |
| **D — Project-local** | A vendored binary under the project tree, not on `PATH` | search project bin dirs (cwd-relative) ✅ | `vitest`/`jest`/`mocha`/`ava`/`jasmine`/`tap`/`uvu` (`node_modules/.bin`), `phpunit`/`pest`/`codecept`/`behat` (`vendor/bin`) |

**Consequence for `sniff software test-runners`:** a Class-D runner is **not** on `PATH`, so a bare `which` misses it — but it *is* installed and runnable, just under the project tree (`node_modules/.bin`, `vendor/bin`). We must **capture these locally-installed variants** by searching the ecosystem's project bin dirs relative to the cwd (see §3). The same caveat applies to several Class-A Python runners: `pytest`/`tox`/`nox`/`nose2` very often live only in a project virtualenv (`.venv/bin/pytest`), never on the global `PATH`. So "local install detection" is a cross-class concern, not a Class-D special case.

**Consequence for `sniff repo test-runner`:** host installation is irrelevant here. Repo usage is determined by **manifest dependency keys + config files**, which every runner has and which are reliable regardless of invocation class. This is the stronger, primary signal (see §4).

---

## 2. Single source of truth: the `TestRunner` enum

Define **one** `TestRunner` enum in `sniff/lib` that carries metadata for *both* surfaces. This avoids two divergent runner lists drifting apart.

```rust
// sniff/lib/src/programs/enums/categories.rs  (new variant set)
pub enum TestRunner {
    // Rust
    CargoTest, Nextest,
    // Go
    GoTest, Gotestsum, Ginkgo,
    // JS/TS
    Vitest, Jest, Mocha, Ava, NodeTest, Jasmine, NodeTap, Uvu,
    // Python
    Pytest, Unittest, Nose2, Tox, Nox,
    // PHP
    PhpUnit, Pest, Codeception, Behat, Atoum,
    // Ruby
    RSpec, Minitest, TestUnit,
    // JVM
    JUnit5, JUnit4, TestNg,
    // .NET
    XUnit, NUnit, MsTest,
    // Elixir
    ExUnit, ESpec,
}
```

Each variant needs metadata beyond the standard `ProgramInfo`. Two design options:

- **Option 1 (recommended) — parallel metadata table.** Keep `ProgramInfo` (for the §3 host surface, so `TestRunner` can `impl ProgramMetadata`/`CategoryEnum` like the other 8 categories) and add a sibling static `TestRunnerSpec` table indexed by the same variant ordinal, carrying the §4 repo-detection signals:

  ```rust
  pub struct TestRunnerSpec {
      pub ecosystem: PackageEcosystem,     // reuse existing enum (Cargo/Node/Python/Go/Unknown) + extend
      pub invocation: InvocationClass,     // A/B/C/D from §1
      pub kind: RunnerKind,                // Runner | Orchestrator | Bdd
      pub parent_binary: Option<&'static str>, // for class B/C host detection: "cargo","go","node",...
      pub manifest_dep_keys: &'static [&'static str], // exact dep names in the manifest
      pub config_globs: &'static [&'static str],      // exact config filenames/globs in the package dir
      pub is_ecosystem_default: bool,      // true => implicitly available (cargo test, go test, unittest, …)
  }
  ```

- **Option 2 — one fat struct.** Fold the repo signals into `ProgramInfo`. Rejected: pollutes the shared `ProgramInfo` shape used by the other 8 categories with fields meaningless to them.

`RunnerKind::Orchestrator` flags `tox`/`nox` (they build envs and *delegate* to pytest/unittest); `RunnerKind::Bdd` flags `behat`/`espec`/`ginkgo`-style suites. Reporting can group/annotate by kind.

---

## 3. Surface 1 — `sniff software test-runners` (host)

Follow the established 9th-category pattern (the `Editor`/`Utility` blueprint):

1. `TestRunner` enum + `TEST_RUNNER_INFO: &[ProgramInfo]` table → `impl ProgramMetadata`.
2. `impl CategoryEnum for TestRunner` (`category_name() = "test-runners"`, `serde_key`, `variant_index`).
3. `pub type InstalledTestRunners = CategoryDetector<TestRunner>;`
4. Add `test_runners` field to `ProgramsInfo`; detect in parallel via the shared `Arc<ExecutableIndex>` in `ProgramsInfo::detect()`.
5. CLI: add `SoftwareSubcommand::TestRunners` (a **report-only leaf** — no `install` / `install-plan` action, unlike the other eight categories) and `OutputFilter::TestRunners`, render via `render_programs_markdown` / `build_programs_json`. Test runners do not fit the host-install model (class B/C are parent-tool subcommands; class D are vendored per-project), so the `define_program_action!(TestRunnerAction, …)` install machinery is intentionally **not** wired for this category.

### Local-install detection (capturing project-local variants)

The standard `ExecutableIndex` only scans global `PATH` (+ macOS bundles / Windows registry). To capture **locally-installed variants** we add an ecosystem-aware **project bin search** that probes well-known per-project bin directories relative to the cwd. Each ecosystem has its own roots, searched closest-first:

| Ecosystem | Project bin roots (searched relative to cwd) | Walk behavior |
|-----------|----------------------------------------------|---------------|
| Node | `node_modules/.bin/<bin>` (`<bin>.cmd`/`.ps1` on Windows) | **walk up** cwd→repo-root (mirrors node resolution; catches hoisted/workspace-root installs) |
| PHP | `vendor/bin/<bin>` | cwd, then repo root |
| Python | `.venv/bin/<bin>`, `venv/bin/<bin>`, `env/bin/<bin>` (`Scripts/` on Windows); honor `$VIRTUAL_ENV` if set | cwd, then repo root |
| Ruby | `bin/<bin>` binstub (Bundler), `$(bundle exec)` resolution | cwd |

A second, optional tier covers **package-manager global bins that aren't on `PATH`**: npm/pnpm/yarn global prefix, Composer global (`~/.composer/vendor/bin`, `~/.config/composer/vendor/bin`), `gem` user dir, pipx (`~/.local/bin`). Recommend deferring this tier to a follow-up unless it proves necessary — the project-local tier covers the common case.

This logic belongs in the **library** (a `LocalBinIndex` or an extension of `ExecutableIndex` taking extra search roots), keeping the CLI a pure reporter. Note this makes `sniff software test-runners` **cwd-sensitive** — like the spec's group-B context queries — which is intentional and should be documented: the command answers "what's installed and runnable *from here*," complementing `sniff repo test-runner`'s "what the repo *declares*."

**Per-class host-detection rule (the new business logic — lives in the library).** Search roots are tried in priority order; the first hit wins and records *where* it was found:

```text
resolve(runner):
    1. project bin dirs   (ecosystem-specific, cwd-relative)  -> Availability::Local { path, root }
    2. global PATH index  (ExecutableIndex)                    -> Availability::Installed { path }
    3. parent binary      (class B/C: cargo/go/node/python/…)  -> Availability::ViaParent { parent }
    4. none of the above                                       -> Availability::NotFound
```

Class A/D both search steps 1→2 (most JS/PHP land at step 1, most global CLIs at step 2). Class B/C resolve at step 3 via `parent_binary`. Version is probed from the resolved path via `version_from_path`, so a venv/`node_modules` runner reports its real local version.

So the JSON entry replaces a bare `installed: bool` with an `availability` discriminator that distinguishes global from local installs:

```jsonc
{ "name": "nextest",    "availability": "installed", "path": "~/.cargo/bin/cargo-nextest" }            // A global
{ "name": "pytest",     "availability": "local",     "path": ".venv/bin/pytest",        "root": ".venv" }       // A in venv
{ "name": "vitest",     "availability": "local",     "path": "node_modules/.bin/vitest", "root": "node_modules" }// D project-local
{ "name": "phpunit",    "availability": "local",     "path": "vendor/bin/phpunit",       "root": "vendor" }      // D vendored
{ "name": "cargo-test", "availability": "via_parent","parent": "cargo" }                                         // B
{ "name": "unittest",   "availability": "via_parent","parent": "python3" }                                       // C
{ "name": "ava",        "availability": "not_found" }                                                            // absent
```

`ExecutableSource` (`contract.rs:20`) gains a `ProjectLocal { root }` variant (and optionally `PackageManagerGlobal`) so the existing source-tracking carries the new locations.

---

## 4. Surface 2 — `sniff repo test-runner` (repo)

This mirrors `detect_package_managers()` exactly (`detection.rs:602`), which already returns `Vec<String>` per `Package`. Add a sibling:

```rust
// sniff/lib/src/filesystem/repo/detection.rs  — called from create_package()
fn detect_test_runners(pkg_dir: &Path, cache: &ManifestCache) -> Vec<TestRunnerUsage>;
```

storing the result on the `Package` struct (`types.rs:108`) as a new field:

```rust
pub test_runners: Vec<TestRunnerUsage>,  // alongside `package_managers`
```

`TestRunnerUsage` records the runner **and the evidence**, so reporting can explain *why*:

```rust
pub struct TestRunnerUsage {
    pub runner: TestRunner,
    pub source: TestRunnerSource,   // Manifest("vitest") | Config("vitest.config.ts") | EcosystemDefault | Convention
}
```

### Detection algorithm (per package)

For the package's ecosystem, evaluate signals in priority order:

1. **Config file present** in the package dir → strongest, disambiguates (e.g. `tests/Pest.php` distinguishes Pest from bare PHPUnit even though both carry `phpunit/phpunit`). A few runners keep a single config at the **workspace/repo root** that governs every member rather than one config per package dir — nextest's `.config/nextest.toml` is the canonical case. For those (`root_scoped_config` in `test_runner_usage.rs`), the config search extends from the package dir up to the repo root, so scanning a member crate still surfaces the runner instead of reporting only the `cargo test` ecosystem default.
2. **Manifest dependency key** present (dev-deps preferred) → strong.
3. **Ecosystem default** (`is_ecosystem_default`) → fallback when no explicit runner found but the ecosystem always ships one (`cargo test`, `go test`, `mix test`, `unittest`, `node --test`).
4. **Convention only** (`tests/` + `*_test.*` naming, no config/dep) → weakest; emit for stdlib runners (unittest, Minitest) that have no dedicated marker.

**Prioritization (single answer).** The result is collapsed to the *strongest tier present*: a configured (`Config`) or declared (`Manifest`) runner **supersedes** the ecosystem default and convention fallbacks, so a package that configures nextest reports `nextest` alone — not `nextest` *and* `cargo test`. The ecosystem default survives only when it is the sole signal (a package with no explicit runner). This gives callers a single answer wherever one exists; a package still yields more than one runner only when two markers of the same top tier coexist (e.g. pytest + tox). See `prioritize` in `test_runner_usage.rs`. *(Supersedes the earlier "never empty — always report the default tagged" rule in D2 below.)*

### Manifest parsing reuse

The `ManifestCache` (`detection.rs:44`) already parses `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`. Extend it (or read alongside) for the manifests test detection needs that aren't cached yet: `composer.json` (PHP), `*.csproj` (.NET), `pom.xml`/`build.gradle[.kts]` (JVM), `mix.exs` (Elixir), `Gemfile`/`*.gemspec` (Ruby), `requirements*.txt` (Python). Config-file globs are a cheap `pkg_dir.join(name).exists()` check — no new walk.

### Aggregation (single-vs-list — identical to `package-manager`)

Reuse the same collapse logic the spec mandates for `sniff repo package-manager`:

```text
package        -> singular runner set for that package
package-area   -> union across contained packages; if uniform -> singular, else unique list
repo root      -> union across all packages; if uniform -> singular, else unique list
```

Output formats per spec: default styled, plus `--csv` / `--list` / `--md` and `--json`. A factored helper `aggregate_distinct<T>(scope, |pkg| pkg.field)` should serve **both** `package-manager` and `test-runner` so the two commands share one collapse implementation.

---

## 5. Runner catalog (reference appendix)

Verified June 2026. `dep key` = exact manifest dependency name; `config` = exact config filename(s); class per §1. `default` marks ecosystem built-ins always implicitly available.

### Rust
| Runner | Class | Host binary | Version | Repo signal |
|--------|-------|-------------|---------|-------------|
| cargo test | B *(default)* | `cargo` | `cargo --version` | `Cargo.toml` present; no config |
| nextest | A | `cargo-nextest` | `cargo nextest --version` | config `.config/nextest.toml` (dep is dev-only, manifest looks identical to cargo test) |

### Go
| Runner | Class | Host binary | Version | Repo signal |
|--------|-------|-------------|---------|-------------|
| go test | B *(default)* | `go` | `go version` | `go.mod` present; `*_test.go` |
| gotestsum | A | `gotestsum` | `gotestsum --version` | `go.mod` dep `gotest.tools/gotestsum`; CI/Makefile invocation |
| ginkgo | A (Bdd) | `ginkgo` | `ginkgo version` | `go.mod` dep `github.com/onsi/ginkgo/v2`; `*_suite_test.go` |

### JS/TS — all Class D (project-local; `node --test` is Class C)
| Runner | Host bin (`node_modules/.bin`) | Version | `package.json` dep key | Config files |
|--------|-------------------------------|---------|------------------------|--------------|
| Vitest | `vitest` | `vitest --version` | `vitest` | `vitest.config.{ts,js,mjs,mts,cts,cjs}`, or `test` key in `vite.config.*` |
| Jest | `jest` | `jest --version` | `jest` (`@jest/core`) | `jest.config.{js,ts,mjs,cjs,json}`, **`"jest"` key in `package.json`** |
| Mocha | `mocha` | `mocha --version` | `mocha` | `.mocharc.{js,cjs,mjs,yaml,yml,json,jsonc}`, **`"mocha"` key in `package.json`** |
| AVA | `ava` (local-only) | `ava --version` | `ava` | **`"ava"` key in `package.json`**, `ava.config.{js,cjs,mjs}` |
| Node Test | *(none — `node --test`)* | `node --version` | *(none, built in)* | none (filename convention `*.test.*`, `test/**`) |
| Jasmine | `jasmine` | `jasmine --version` | `jasmine` (`jasmine-core`) | `spec/support/jasmine.json` |
| node-tap | `tap` | `tap --version` | **`tap`** (not `node-tap`) | `.taprc`, **`"tap"` key in `package.json`** |
| uvu | `uvu` | — | `uvu` | none (CLI args only) |

> Naming trap: `tap` (node-tap) ≠ `tape`. Distinct packages, distinct `package.json` keys.

### Python
| Runner | Class | Host binary | Version | Repo signal |
|--------|-------|-------------|---------|-------------|
| pytest | A | `pytest`, `py.test` | `pytest --version` | `pytest.ini`; `pyproject.toml [tool.pytest.ini_options]`; `tox.ini [pytest]`; `setup.cfg [tool:pytest]`; `conftest.py`; dep `pytest` |
| unittest | C *(default)* | *(none — `python -m unittest`)* | `python --version` | convention only: `tests/`, `test*.py`, `TestCase` subclasses |
| nose2 | A | `nose2` | `nose2 --version` | `unittest.cfg`/`nose2.cfg` `[unittest]`; `setup.cfg [unittest]`; dep `nose2` |
| tox | A (Orchestrator) | `tox` | `tox --version` | `tox.ini [tox]`; `tox.toml`; `pyproject.toml [tool.tox]` |
| nox | A (Orchestrator) | `nox` | `nox --version` | `noxfile.py` |

### PHP — all Class D (`vendor/bin`), strongest signal = `composer.json` `require-dev`
| Runner | Vendored binary | Version | `require-dev` key | Config |
|--------|-----------------|---------|-------------------|--------|
| PHPUnit | `vendor/bin/phpunit` | `--version` | `phpunit/phpunit` | `phpunit.xml` / `phpunit.xml.dist` |
| Pest | `vendor/bin/pest` | `--version` | `pestphp/pest` | `tests/Pest.php` (+ `phpunit.xml`) |
| Codeception | `vendor/bin/codecept` | `--version` | `codeception/codeception` | `codeception.yml` / `*.suite.yml` |
| Behat | `vendor/bin/behat` | `-V` | `behat/behat` | `behat.yml[.dist]`, `behat.dist.yml`, `behat.php`; `features/*.feature` |
| atoum | `vendor/bin/atoum` | `--version` | `atoum/atoum` | `.atoum.php` |

### Ruby
| Runner | Class | Host binary | Version | Repo signal |
|--------|-------|-------------|---------|-------------|
| RSpec | A | `rspec` | `rspec --version` | `.rspec`; `spec/spec_helper.rb`; `spec/**/*_spec.rb`; Gemfile/gemspec `rspec` |
| Minitest | C *(default, stdlib)* | *(none — via `ruby`/`rake`)* | `gem list minitest` | Gemfile `minitest`; `test/**/*_test.rb`; `Rake::TestTask` |
| test-unit | C | *(none — `testrb` removed)* | `gem list test-unit` | Gemfile `test-unit`; `test/**/*_test.rb` |

### JVM (Java/Kotlin) — all Class B (`mvn test` / `gradle test`)
| Runner | Manifest signal |
|--------|-----------------|
| JUnit 5 | Maven `org.junit.jupiter:junit-jupiter`; Gradle `testImplementation("org.junit.jupiter:junit-jupiter")` + `useJUnitPlatform()` |
| JUnit 4 | Maven `junit:junit:4.x`; Gradle `useJUnit()`; vintage bridge `org.junit.vintage` |
| TestNG | Maven `org.testng:testng`; Gradle `useTestNG()`; `testng.xml` |

### .NET — all Class B (`dotnet test`), signal = `.csproj` `<PackageReference>`
| Runner | PackageReference keys |
|--------|----------------------|
| xUnit | `xunit` + `xunit.runner.visualstudio` (+ `Microsoft.NET.Test.Sdk`) |
| NUnit | `NUnit` + `NUnit3TestAdapter` (+ `Microsoft.NET.Test.Sdk`) |
| MSTest | `MSTest` (or `MSTest.TestFramework` + `MSTest.TestAdapter`) (+ `Microsoft.NET.Test.Sdk`) |

> `Microsoft.NET.Test.Sdk` is the generic "this is a test project" anchor; the framework package disambiguates. Under the newer Microsoft.Testing.Platform, `Microsoft.NET.Test.Sdk` may be absent — treat the framework packages as primary.

### Elixir — Class B (`mix test`)
| Runner | Manifest signal |
|--------|-----------------|
| ExUnit | *(default, stdlib — no dep)*; `mix.exs` + `test/**/*_test.exs` + `test/test_helper.exs` |
| ESpec | `mix.exs` deps `{:espec, …}`; `spec/**/*_spec.exs` |

---

## 6. Open decisions for the user

- **D1 (host Class-D handling) — RESOLVED:** capture locally-installed variants. `sniff software test-runners` searches project bin dirs (`node_modules/.bin`, `vendor/bin`, `.venv/bin`, …) in addition to global `PATH`, and reports `availability: local` with the resolved path + root (§3). *Remaining sub-decision (D1a):* second-tier package-manager global bins (npm -g, composer global, …) — include now or defer? *(Recommend: defer.)*
- **D2 (ecosystem default reporting)** — when a package has no explicit runner config, report the implicit built-in (`cargo test`, `go test`, `unittest`, …) with `source: EcosystemDefault`, or report "none configured"? *(Recommend: report the default, tagged so it's distinguishable.)*
- **D3 (orchestrators)** — do `tox`/`nox` count as the package's "test runner," or as a wrapper around one? *(Recommend: report with `kind: orchestrator`; let the consumer decide.)*
- **D4 (typed vs string)** — `package_managers` is `Vec<String>` today; should the new `test_runners` field be a typed `Vec<TestRunnerUsage>` (richer, evidence-carrying) even though it diverges from the string convention? *(Recommend: typed — the evidence/source is worth it, and `package-manager` could later adopt the same shape.)*
- **D5 (scope of v1 catalog)** — ship all ~30 runners above, or start with the spec's explicit list (Rust/JS/Python/PHP) and add Go/Ruby/JVM/.NET/Elixir in a follow-up? *(Recommend: land the enum + detection plumbing with the full catalog metadata, since the per-variant cost is just table rows.)*

---

## 7. Implementation checklist (anchored to current code)

**Library — host surface (§3):**
- `programs/enums/categories.rs` — add `TestRunner` enum + `InstalledTestRunners` alias.
- `programs/enums/metadata.rs` — add `TEST_RUNNER_INFO` table; `impl ProgramMetadata for TestRunner`.
- `programs/contract.rs` — `impl CategoryEnum for TestRunner`.
- `programs/mod.rs` — add `test_runners` to `ProgramsInfo` + parallel detect in `detect()`.
- New: `InvocationClass` host rule (A/B/C/D) resolving the §3 search order (project bin → PATH → parent).
- New: `LocalBinIndex` (or `ExecutableIndex` extra-roots mode) implementing the ecosystem project-bin search (`node_modules/.bin` walk-up, `vendor/bin`, `.venv/bin`/`Scripts`), platform-aware.
- `programs/contract.rs` — add `ExecutableSource::ProjectLocal { root }`.

**Library — repo surface (§4):**
- `filesystem/repo/types.rs:108` — add `test_runners: Vec<TestRunnerUsage>` to `Package`.
- `filesystem/repo/detection.rs:602` — add `detect_test_runners()` peer to `detect_package_managers()`, called from `create_package()` (`:1029`).
- `filesystem/repo/manifest_index.rs` / `ManifestCache` — extend manifest coverage (composer.json, *.csproj, pom.xml, build.gradle[.kts], mix.exs, Gemfile).
- New `TestRunnerSpec` table keyed by `TestRunner` ordinal (the §4 signals).

**CLI:**
- `args/mod.rs` — `SoftwareSubcommand::TestRunners` (report-only leaf, no `define_program_action!`), `OutputFilter::TestRunners`.
- `args/repo.rs` — `RepoSubcommand::TestRunner` + `RepoAction::TestRunner`.
- `args/mod.rs:683` — map in `to_repo_action()`.
- `commands/repo.rs` — `handle_repo_test_runner()` modeled on `handle_repo_packages()`, using the shared `aggregate_distinct` helper (also back-fits `package-manager`).
- `output/programs.rs` — render the `availability` discriminator (`installed` / `local` / `via_parent` / `not_found`) plus path+root for local installs, instead of a bare bool.

**Business-logic stays in the library** (invocation-class rules, project-bin search roots, manifest/config signal tables, aggregation collapse). The CLI only reports. stdout carries the runner data; any "searched from cwd; see `sniff repo test-runner` for declared usage" hint goes to **stderr** and is suppressed under `--json`.
