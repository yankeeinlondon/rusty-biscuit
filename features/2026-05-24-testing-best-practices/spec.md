---
status: ready for planning and implementation
reviewed: true
---

# Testing Best Practices Specification

**Status:** Reviewed and ready for planning and implementation. Each topic below presents the original problem, candidate approaches, and pros/cons that informed the choice. **The authoritative outcome for every question is in the [Decisions](#decisions) section at the bottom (D1-D16).** Sub-sections labeled "Open questions for Topic N" record the path; they are all resolved in Decisions.

**Review note:** This inline review resolves a few remaining specification gaps:

- Tier enforcement should extend the existing `tools/test-toolkit` crate instead of creating a second overlapping `biscuit-test` helper crate.
- The canonical justfile migration applies to the curated package-area list from the root `areas` variable, not every Cargo workspace member discovered by `cargo metadata`.
- Sanity tests exclude doctests and lint, and the `just all` order is `sanity -> lint -> doctest -> test -> test-l2 -> test-browser`.
- Runtime level helpers do not rename tests, so this spec adds a separate naming/filter contract for nextest and recipe selection.

---

## Context: What we have today

A consolidated picture of the testing surface across the monorepo (see research below for full citations).

| Area                          | State                                                                                                          |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **biscuit-test-harness**      | Mature. 4 backends (WezTerm, Kitty, tmux, Apple Terminal) + `cliclick` for L3. Used by 4 packages.             |
| **L1/L2/L3 taxonomy**         | Defined in `.claude/skills/cli/cli-best-practices.md` and `prompts/snippets/test-rigor.md`. Not enforced.      |
| **L2/L3 enforcement**         | Fragmented: each test file re-implements its own `level2_required()` / `level3_enabled()` env-var check.       |
| **Browser testing**           | Inline in `darkmatter/lib/tests/browser_render.rs` using `chromiumoxide`. No shared abstraction.               |
| **Just recipes for testing**  | Shared `_test` in `just/devops.just`. Tiered recipes (`test-l2`, `test-server`, `test-integrations-real-*`) are per-package and inconsistently named. Root `areas` is curated and does not represent all Cargo workspace members. |
| **Sanity / smoke tier**       | Does not exist as a named concept.                                                                              |
| **Criterion benchmarks**      | 4 packages: darkmatter (6 benches), claudine (1), biscuit-terminal (2), sniff (8-module modular). No `just bench` orchestrator. |
| **Property tests (proptest)** | 7 packages with embedded or tests/ proptest usage. No fuzz targets, no `cargo-fuzz`, no `arbitrary`.            |
| **Doctests**                  | ~2,979 examples across 332 files. Not measured or invoked separately by every package.                          |
| **Coverage**                  | `just/coverage.just` exists (cargo-llvm-cov, LCOV output). Not wired into per-package workflows or CI gates.    |
| **Shared test helpers**       | `tools/test-toolkit` already provides cross-suite helpers such as `EnvGuard` and tracing phases; tier policy should live there to avoid a parallel test-helper crate. |

**Key gap themes:**

1. **Naming drift** — `test-l2`, `test-real-base`, `test-integrations-real-destructive`, etc. — same idea ("slow, requires resources"), different names.
2. **No machine-readable tier** — you cannot run "all L1 tests in the workspace" with a single filter. Each package has to define its own recipe.
3. **No sanity tier** — agents (and humans) lack a fast "this won't catch everything, but if it's red we have a problem" command.
4. **Browser testing waiting on a second consumer** — by design, but if biscuit-terminal grows browser-target tests, we'll need the abstraction anyway.
5. **Bench is opt-in by accident** — no convention says "performance-critical libraries should have a bench file"; the four that exist were added ad-hoc.
6. **Fuzz is absent** despite obvious candidates (biscuit-file PDF/JSON5/YAML/TOML, darkmatter markdown, tree-hugger queries, claudine hook JSON).

---

## Topic 1 — Test tier taxonomy and enforcement

### Problem

The repo has a documented L1/L2/L3 vocabulary but no binding to the test runner. Each test file re-implements env-gate logic. Agents cannot reliably answer "did the L2 suite actually run, or did it skip cleanly?"

### Approach 1A — Filename convention only (status quo, formalized)

Keep `level2_*.rs` / `level3_*.rs` filenames. Each package's `justfile` defines `test`, `test-l2`, `test-l3` recipes that map to `cargo test --test level2_*`. No new code.

- **Pros:** Zero new dependencies. Already partially adopted. Easy to grep.
- **Cons:** Per-package boilerplate. No way to mix tiers in one file. Skip-clean still hand-rolled. Cannot filter "all L1 across the workspace" without iterating packages.

### Approach 1B — Extend `test-toolkit` with tier helpers ▶ **Selected**

Extend the existing `test-toolkit` workspace crate with tier helpers:

- A standard `require_level!(2, harness_check)` macro that emits `skipping: ...` and returns cleanly, or panics if a uniform env var (`BISCUIT_TEST_LEVEL_REQUIRED=2`) is set.
- A `BISCUIT_TEST_LEVEL=1|2|3` runtime gate that lets `cargo test` skip tests above the requested level without rebuilding.
- Optional nextest filter integration via test naming + `nextest.toml` filterset aliases (`set:level2 = test(level2_)`, etc.).

- **Pros:** One place to change skip/enforce policy. Agent-friendly (`BISCUIT_TEST_LEVEL=1 just test` is documented and uniform). Removes ~50 lines of duplicated env-check boilerplate per package. Reuses the existing shared test-helper crate instead of adding a competing helper surface.
- **Cons:** `test-toolkit` becomes responsible for both fixtures and tier policy. The macro is slightly more verbose than an attribute macro because it deliberately avoids a proc-macro crate.

### Approach 1C — Pure nextest filtersets (no Rust code)

Define filtersets in `.config/nextest.toml` keyed on test name regex, and standardize that all tests start with `level1_`, `level2_`, or `level3_`. Run `cargo nextest run -E 'set:level1'`.

- **Pros:** No new crate. Native to nextest. Composable with package filters.
- **Cons:** Requires renaming every existing test (mass churn). Doesn't help skip-clean inside the test body. Still need per-test harness-availability check. Tests that mix tiers in one file get awkward.

### Naming and filter contract

Runtime helpers decide whether a test body should execute, but they do not give nextest a machine-readable way to select tests. To keep nextest filtering deterministic without proc macros, tests that belong to a non-default tier must use one of these stable identifiers in the test function name or enclosing module name:

| Identifier | Meaning |
| ---------- | ------- |
| `level2_`  | Real terminal, PTY, or other local harness tests. |
| `level3_`  | OS keyboard/mouse injection tests. |
| `browser_` | Headless browser tests. |
| `real_`    | Real external resources such as devices, network services, or provider APIs. |
| `slow_`    | Slow but otherwise ordinary tests that must not run in `sanity`. |

`.config/nextest.toml` gains explicit filtersets for these identifiers. The implementation must update existing tests and just recipes incrementally as package areas migrate; do not bulk-rename unrelated tests outside the packages being migrated in the current phase.

### Open questions for Topic 1

- **Q1.1** — Do you want a proc-macro (`#[level2]`) or a runtime helper (`level!(2); require_harness!(WezTerm);`)? Proc-macros are slicker but add compile time and a new crate boundary.
- **Q1.2** — Should `BISCUIT_TEST_LEVEL=1` cause higher-level tests to *skip cleanly* or *not compile in*? Skip-clean is simpler; cfg-gating is faster but less flexible.

---

## Topic 2 — The "sanity test" tier

### Problem

We need a tier above unit-test granularity that agents and devs can trust as "quick confidence, not a full pass." Today there's no such concept; the closest is "run `just test` in one package."

### What "sanity" should mean (proposed definition)

A **sanity test** for a package area must (per D1 + D12):

1. Run in **≤ 15 seconds wall-clock** on a typical dev machine (excluding compile time).
2. Cover every **public API surface** at least at a smoke level (call it once, assert it doesn't panic / returns the expected shape).
3. **Never require** WezTerm, Kitty, browsers, network, devices, or `RUN_LEVEL3`.
4. **Exclude doctests** (they live in the separate `doctest` recipe — compile cost blows the budget).
5. Be invoked by a uniform recipe: **`just sanity`** at root and in every package area.

### Approach 2A — `sanity` recipe maps to `cargo nextest run --lib --bins -E '!set:slow'` ▶ **Selected**

Each package's `justfile` defines `sanity` that runs library and binary tests, excluding anything tagged "slow" via nextest filterset (`set:slow = test(level2_) + test(level3_) + test(real_) + test(browser_) + test(slow_)`). Doctests are excluded from `sanity` and run through the separate `doctest` recipe.

- **Pros:** Builds on nextest. One filter expression defines "what's excluded from sanity." Sanity is automatically inclusive of every new unit test.
- **Cons:** Requires nextest (already the standard via `_test`). The filter expression must be kept current as new "slow" test prefixes appear.

### Approach 2B — Explicit sanity test files (`tests/sanity.rs`)

Each package maintains a hand-picked `tests/sanity.rs` listing what to run. Root `just sanity` iterates packages.

- **Pros:** Crystal-clear what's in the sanity tier. Curated for speed.
- **Cons:** Drift: new public APIs added but not added to sanity. Maintenance burden. Encourages sanity tests to diverge from "real" tests.

### Approach 2C — Hybrid: filter-based + curated additions

Default to 2A's filter, but allow a package to `include!` extra sanity scenarios from `tests/sanity_extra.rs`.

- **Pros:** Most flexible.
- **Cons:** Two ways to do it. Probably overengineered for the current need.

### Approach 2D — Tag a `#[sanity]` opt-in attribute

Tests opt INTO sanity rather than out of slow. Default is "not sanity."

- **Pros:** Sanity grows only intentionally; stays fast.
- **Cons:** Inverts the discoverability problem (you have to remember to tag); new public-API tests don't auto-protect against regressions.

### Open questions for Topic 2

- **Q2.1** — Is 15 s the right ceiling? Per package or total across the monorepo (which would be ~50 × 15s = 12 min for the whole repo)?
- **Q2.2** — Should `sanity` include doctests? They're slow to compile. Maybe `sanity` excludes doctests and `sanity-full` includes them.
- **Q2.3** — Should `sanity` also run `cargo clippy` (a fast lint pass) or only tests?

### Review clarification for Topic 2

The "cover every public API surface" requirement is a smoke-coverage target, not a demand that `sanity` duplicate integration-test suites. Each migrated package must ensure that its public entry points have at least one fast library, binary, or smoke integration test that remains outside `set:slow`. When the only meaningful coverage of a public API requires a real terminal, browser, device, or network service, the package must document that exception in `docs/testing-strategy.md` rather than forcing it into `sanity`.

---

## Topic 3 — Terminal testing foundation (biscuit-test-harness)

### Problem

The harness is solid for the four packages that use it, but:

- No standard "shared harness with atexit cleanup" helper — every consumer reimplements the `Mutex<Option<Harness>>` + `libc::atexit` pattern.
- No Linux equivalent for `cliclick` (xdotool) — L3 only works on macOS.
- No published convention for *which* backend to default to (tmux is most portable, but most consumers reach for WezTerm).

### Approach 3A — Add `SharedHarness<T>` utility and default-backend guidance ▶ **Selected, with xdotool deferred**

Extend `biscuit-test-harness` with:

1. `harness::shared::SharedHarness<T>` — wraps the `Mutex<Option<T>>` + atexit pattern as a single line of test setup.
2. Document the recommended default in the README: **"Prefer tmux for portability unless you need graphics/SGR-rich features."**
3. Defer `xdotool.rs` until a Linux L3 CI runner or concrete Linux L3 user exists.

- **Pros:** Removes ~30 lines of cleanup boilerplate per consumer. Reduces decision-fatigue on backend choice. Avoids adding an X11-only backend before there is a CI/user requirement for it.
- **Cons:** L3 remains macOS-only for now. `SharedHarness<T>` adds a small generic abstraction that's only useful if 3+ packages use it (we have 3+).

### Approach 3B — Leave the harness alone; document patterns externally

Add a `biscuit-test-harness/docs/patterns.md` with copy-paste snippets for shared-harness and L3 setup. No code changes.

- **Pros:** Zero risk to existing tests.
- **Cons:** Boilerplate stays. L3 still macOS-only.

### Open questions for Topic 3

- **Q3.1** — Is xdotool worth adding now, or wait until we actually have a Linux CI runner?
- **Q3.2** — Do we standardize on tmux as the default L2 backend (most portable), or WezTerm (richest)?

---

## Topic 4 — Browser testing strategy

### Problem

`darkmatter/lib/tests/browser_render.rs` has a working chromiumoxide setup (Chrome auto-discovery, `serial(browser)`, computed-style helpers) but it's inline. If `biscuit-terminal` (or any other package targeting the Browser render target) needs browser tests, they'll either copy-paste it or do something different.

### Approach 4A — Create `biscuit-browser-harness` crate now ▶ **Selected**

New crate exposes:

- `BrowserHarness` trait with `spawn()`, `render_html(fragment)`, `computed_style(selector, property)`, `screenshot(selector) -> Vec<u8>`.
- `ChromeHarness` impl (extracts current darkmatter code).
- `available()` probe with `BISCUIT_BROWSER_REQUIRED=1` enforcement convention (mirrors `BISCUIT_TEST_HARNESS` patterns).
- `#[serial(browser)]` documented as the standard gate.
- Migrate `darkmatter/lib/tests/browser_render.rs` to consume it. (One existing consumer is the bar for extraction.)

- **Pros:** Consistent with the L2-terminal pattern (`biscuit-test-harness`). Ready for biscuit-terminal's eventual `BrowserRenderable` tests. Centralizes Chrome-discovery logic that no individual package wants to own.
- **Cons:** Premature if biscuit-terminal won't need it for many months. Adds a new workspace member to maintain.

### Approach 4B — Wait for the second consumer; extract then

Keep darkmatter's inline code. When biscuit-terminal needs browser tests, *then* extract.

- **Pros:** YAGNI-respecting. Smaller workspace today.
- **Cons:** When the second consumer arrives, there's pressure to ship features, not refactor harnesses. Extraction tends to slip.

### Approach 4C — Snapshot/golden-HTML alternative

Skip the headless browser entirely. Render to HTML, snapshot the output, compare to a golden file. Use a browser only for occasional manual review.

- **Pros:** Zero Chrome dependency. Fast. Deterministic.
- **Cons:** Doesn't verify *computed* CSS (e.g., "background color matches what a browser actually paints"). Doesn't catch CSS bugs that depend on browser layout. Snapshot churn on every styling tweak.

### Open questions for Topic 4

- **Q4.1** — Does biscuit-terminal's BrowserRenderable need browser tests within the next quarter, or is that hypothetical?
- **Q4.2** — Even with a real-browser harness, do we want a snapshot tier (4C) for cheap regression catches?

---

## Topic 5 — Fuzz testing strategy

### Problem

We have obvious untrusted-input parsers (PDF, JSON5, YAML, TOML, markdown, tree-sitter queries, hook JSON) and zero fuzz coverage. Proptest is in use in 7 places but tends to test round-trips, not adversarial input.

### What qualifies as a good fuzz candidate

A parser/decoder is a good fuzz candidate if **all** are true:

1. It accepts data from outside the process boundary (file, network, user input, another tool's output).
2. A crash, hang, or OOM in it is a real defect (not "well, garbage in, garbage out").
3. It has a stable surface area worth investing structured fuzzers in.

Concrete candidates in priority order:

| Candidate                                         | Why                                                                |
| ------------------------------------------------- | ------------------------------------------------------------------ |
| `biscuit-file` PDF extraction (pdf-extract/lopdf) | PDF is hostile by design; native deps; OOM/crash risk highest.    |
| `biscuit-file` JSON5/YAML/TOML round-trip         | Format converters touch user files daily.                          |
| `darkmatter` markdown parser (pulldown-cmark)     | Stable upstream but our composition layer adds risk.               |
| `claudine` hook JSON and provider stream lines    | Provider APIs evolve; defensive parsing already partially proptested. |
| `tree-hugger` tree-sitter query strings           | Queries come from skill files / user input.                        |
| `schematic` schema definitions                    | Code-generation input.                                             |

### Approach 5A — `cargo-fuzz` targets in a `fuzz/` directory per high-risk crate ▶ **Selected for top 3 candidates**

Add `cargo-fuzz` targets for the top three (`biscuit-file` PDF, `biscuit-file` JSON5/YAML/TOML, `darkmatter` markdown). Run nightly via CI. Treat any new crash as a `R0` regression.

- **Pros:** Industry-standard. Excellent coverage on parsers. Catches real bugs (most libc and serde crates have shipped fuzzer-found fixes).
- **Cons:** Requires nightly Rust for `cargo-fuzz`. Each target is a small crate inside `fuzz/`. Needs corpus storage (git LFS or external).

### Approach 5B — Property tests with `proptest`/`arbitrary` only

Use `proptest` with structured input generators. Don't run AFL/libFuzzer.

- **Pros:** Stable Rust. No corpus management.
- **Cons:** Less effective at finding deep parser bugs. Tests are bounded by your imagination of input shapes.

### Approach 5C — Hybrid: proptest for round-trips, cargo-fuzz for adversarial input

Both. Proptest validates "what we wrote, we can read." Cargo-fuzz validates "we don't crash on garbage."

- **Pros:** Each tool used for what it's good at.
- **Cons:** Two test infrastructures to maintain per package.

### Open questions for Topic 5

- **Q5.1** — Are we OK requiring nightly Rust for fuzz targets (toolchain-pinned in `fuzz/rust-toolchain.toml`)?
- **Q5.2** — Where do we store fuzz corpora — in-repo (small, public) or external (git LFS, S3)?
- **Q5.3** — Is fuzzing part of the "sanity" tier (no), CI nightly (probably yes), or PR gate (probably no)?

---

## Topic 6 — Criterion benchmark standardization

### Problem

Four packages have benches, all written independently. There's no `just bench` orchestrator, no baseline tracking convention, no documented "every library should/shouldn't have benches" rule.

### Proposed rule: which packages must have benches

A library **must** have a criterion bench file if **any** are true:

1. It implements a parser, encoder, hasher, or renderer.
2. Its public API is called in a hot loop by a consumer.
3. It has measurable performance characteristics worth defending (e.g., "hashing must stay under 100ns/KB").

A library **may opt out** by adding to its `Cargo.toml`:
```toml
[package.metadata.benchmarks]
required = false
reason = "Pure data-types crate; no measurable hot paths."
```
This is the **exception clause** the user asked for, and the metadata is grep-able.

Under this rule, packages that should add benches:

- `biscuit-file` (parsers, converters)
- `biscuit-hash` (the *literal* job is performance)
- `tree-hugger` (query execution)
- `renderable` (tree folding)
- `biscuit-terminal` (already has 2, could expand)

### Approach 6A — Shared `_bench` recipe + per-package `bench` recipe + `just bench` root iterator ▶ **Selected**

Add to `just/devops.just`:
```just
_bench pkg *args:
    @cargo bench -p {{ pkg }} {{ args }}
```

Root `justfile` gains `bench` that iterates the same `areas` list as `test`. Each package's `justfile` gets `bench` that calls `_bench` for each crate.

- **Pros:** Mirrors the `_test` pattern exactly. One invocation pattern. Easy to add baselines later via `--save-baseline`/`--baseline`.
- **Cons:** Some packages have nothing to bench; their `bench` recipe becomes a no-op or skip.

### Approach 6B — Centralized `benches/` workspace crate

One `workspace-benches` crate consumes all libraries and benchmarks them together.

- **Pros:** One place to run, one report.
- **Cons:** Couples unrelated packages. Single `cargo bench` invocation becomes huge. Hard to iterate on one package's benches.

### Open questions for Topic 6

- **Q6.1** — Should `just bench` be parallel or sequential across packages? Parallel is faster but noisier (benches need quiet CPU).
- **Q6.2** — Do we want a baseline-tracking convention now (commit `target/criterion/baseline-main/` snapshots to git) or defer?
- **Q6.3** — Is the `[package.metadata.benchmarks]` opt-out convention worth the static-check tooling, or just a doc comment?

---

## Topic 7 — Just-driven consistency

### Problem

Per-package justfiles diverge in naming (`test-l2`, `test-server`, `test-integrations-real-base`). Agents struggle to know which recipe runs what.

### Proposed canonical recipe set (every package area should define these)

| Recipe          | Meaning                                                                                                       | Required? |
| --------------- | ------------------------------------------------------------------------------------------------------------- | --------- |
| `sanity`        | Fast confidence check. ≤15s. No external resources. (See Topic 2.)                                            | **Yes**   |
| `test`          | Full L1 suite for this area (default `cargo test` / `nextest`).                                               | **Yes**   |
| `test-l2`       | Real-terminal tests. Skip-cleanly if harness unavailable; hard-fail under `BISCUIT_TEST_LEVEL_REQUIRED=2`.    | If applicable |
| `test-l3`       | OS-keyboard-injection tests. Always skip unless `RUN_LEVEL3=1`.                                                | If applicable |
| `test-browser`  | Headless-browser tests. Skip-cleanly if Chrome absent.                                                          | If applicable |
| `test-real`     | Tests against real external resources (devices, network, APIs). Always `--ignored` unless explicitly opted in. | If applicable |
| `lint`          | Clippy + fmt check.                                                                                            | **Yes**   |
| `bench`         | Criterion. Skip if opted out via `[package.metadata.benchmarks] required = false`.                            | **Yes** (may be no-op) |
| `coverage`      | LLVM coverage for this area.                                                                                   | **Yes**   |
| `doctest`       | Doctests for this area's crates.                                                                                | **Yes**   |
| `fuzz`          | Cargo-fuzz suites (short run; nightly toolchain). Skip if no targets defined.                                  | If applicable |
| `all`           | Run sanity → lint → doctest → test → test-l2 → test-browser in sequence.                                       | **Yes**   |

### Approach 7A — Codify the canonical set in `just/lifecycle.just` + a `_check_canonical` recipe ▶ **Selected**

Move the boilerplate into `just/lifecycle.just`. Each package's `justfile` becomes ~10 lines of `@just _test <pkg>` / `@just _sanity <pkg>` calls. Add a `just _check_canonical` recipe that any CI job can use to assert "this package's justfile defines all required recipes."

- **Pros:** Discoverable. Agents can always type `just sanity` regardless of package. Drift detectable.
- **Cons:** Some packages don't need (and can't sensibly implement) some tiers — `_check_canonical` needs to permit "explicit no-op" recipes.

### Scope clarification for Topic 7

The canonical set applies to the root `justfile`'s curated `areas` list, not every member from `cargo metadata`. This matches the existing repo rule that root just coverage is curated. If a workspace member is intentionally outside the root area list, it does not need a package-area justfile migration as part of this initiative. If a new area is added to the root list later, it must expose the canonical recipes before landing.

### Approach 7B — Document the convention in CLAUDE.md and trust contributors

No tooling. Just a written convention.

- **Pros:** Zero infrastructure.
- **Cons:** Drift is the current state; documentation alone won't fix it.

### Approach 7C — Generate per-package justfiles from a template

A `just _scaffold-justfile <pkg>` recipe writes a baseline `justfile` for a new package area.

- **Pros:** New packages start consistent.
- **Cons:** Doesn't fix existing drift. Templates rot.

---

## Topic 8 — Coverage

### Problem

`just/coverage.just` runs whole-workspace coverage. No per-package, no CI integration, no gate.

### Approach 8A — Per-package `coverage` recipe + root aggregator ▶ **Selected**

Add `_coverage <pkg>` to `just/coverage.just` (per-package LCOV). Root `just coverage` aggregates with `cargo llvm-cov --workspace`. CI uploads to a coverage service (Codecov?) but does **not** gate on a coverage percentage — only reports.

- **Pros:** Per-package iteration. No flaky gate. Reports are useful for review.
- **Cons:** Without a gate, coverage may slowly degrade. Acceptable trade-off (gates are noisy).

### Approach 8B — Workspace-only coverage, run on demand

Keep current behavior. Document as "ad-hoc."

- **Pros:** Simplest.
- **Cons:** No per-package data. Hard to know where to invest in tests.

### Open questions for Topic 8

- **Q8.1** — Coverage gate (e.g., "PR cannot drop coverage by more than 1%") or report-only?
- **Q8.2** — Coverage service: Codecov, Coveralls, self-hosted, or just artifact upload?

---

## Topic 9 — Documentation deliverables

A new testing strategy needs three documents to land effectively:

1. **`.claude/skills/rust-testing/SKILL.md`** — agent-facing summary (<200 lines): the tier definitions, the canonical justfile recipes, when to use each tool. This is what an agent loads on "I'm about to write a test."
2. **`docs/testing-strategy.md`** — human-facing reference (the long version): rationale, examples, fuzz playbook, browser-harness API, decision log.
3. **`prompts/snippets/test-rigor.md`** — already exists; update to reference the new skill and recipes.

### Open questions for Topic 9

- **Q9.1** — Do we also want a `CONTRIBUTING.md` section at repo root, or is `.claude/skills/` + `docs/` enough?
- **Q9.2** — Should the skill include a decision tree ("you're writing a test for X — start at Level N because...")?

---

## Cross-cutting summary — confirmed deliverables

Based on D1-D16, this initiative produces:

### New or expanded workspace test crates

- **`test-toolkit` expansion** (D2) — exposes `require_level!(Level, available_check)` macro, `Level` enum (`L1`/`L2`/`L3`), and helpers wrapping the `BISCUIT_TEST_LEVEL` / `BISCUIT_TEST_LEVEL_REQUIRED` / `RUN_LEVEL3` env contract. Runtime macro only — no proc-macro crate. Existing `EnvGuard` and tracing helpers remain in this crate.
- **`biscuit-browser-harness`** (D3) — `BrowserHarness` trait + `ChromeHarness` impl extracted from `darkmatter/lib/tests/browser_render.rs`. `BISCUIT_BROWSER_REQUIRED=1` enforcement convention. First consumer: darkmatter (migrated as part of this initiative). This is a dev/test infrastructure crate, not a production rendering API.

### Additions to `biscuit-test-harness`

- `SharedHarness<T>` utility (Topic 3) — wraps the `Mutex<Option<T>>` + `libc::atexit` pattern as a single-line setup.
- Default-backend documentation note: prefer tmux for portability (D5).
- xdotool (Linux L3) **deferred** to a follow-up spec (D13).

### Nextest filter contract (D15)

- Add `.config/nextest.toml` filtersets for `level2_`, `level3_`, `browser_`, `real_`, and `slow_`.
- Update migrated package tests so non-default tests include the stable identifier in either the test function name or module path.
- `sanity` uses the shared `set:slow` filter; `test-l2`, `test-l3`, `test-browser`, and `test-real` use their corresponding filtersets where possible.
- Runtime `test-toolkit::require_level!` remains the source of skip-vs-fail behavior when a filtered test is selected but its required resource is unavailable.

### Fuzz infrastructure (D4 + D10)

- `biscuit-file/fuzz/` with two targets: PDF extraction, JSON5/YAML/TOML round-trip.
- `darkmatter/fuzz/` with one target: markdown parser.
- Each `fuzz/` has `rust-toolchain.toml` pinning nightly, a `corpus-seed/` directory, and a `crashes/` directory for committed regression fixtures.
- Nightly CI workflow runs short fuzz cycles per target.

### Criterion bench additions (Topic 6)

- New benches in: `biscuit-file`, `biscuit-hash`, `tree-hugger`, `renderable`. Existing four (darkmatter, claudine, biscuit-terminal, sniff) retained as-is.
- `[package.metadata.benchmarks] required = false` convention for opt-out (D14).
- Bencher.dev SaaS integration in CI: darkmatter wired as proof-of-concept (D9). Others incremental.

### Shared just recipes (in `just/lifecycle.just` or new `just/test.just`)

`_sanity`, `_test`, `_test_l2`, `_test_l3`, `_test_browser`, `_test_real`, `_lint`, `_bench`, `_coverage`, `_doctest`, `_fuzz`, `_all`, plus `_check_canonical` validator.

### Per-package justfile migration

All package areas in the root `justfile`'s curated `areas` list updated to expose the canonical 12-recipe set (D6) with no-op recipes where not applicable, in the `just all` order from D7. This does not require every Cargo workspace member to grow its own justfile.

### CI workflows (in `.github/workflows/`)

- `sanity.yml` — runs `just sanity` on every PR, ≤5 min wall-clock budget.
- `test.yml` — runs `just all` on every PR (tier-skipped where harnesses absent).
- `fuzz-nightly.yml` — runs `just fuzz` against all targets nightly; opens an issue on new crash.
- `bench-nightly.yml` — runs `cargo bench -p darkmatter`, pushes to Bencher.dev.
- `coverage.yml` — runs `just coverage` per package + workspace aggregator, uploads LCOV artifact. Report-only (D8).

### Documentation (D11)

- `.claude/skills/rust-testing/SKILL.md` — agent-facing, ≤200 lines, includes decision tree.
- `docs/testing-strategy.md` — human-facing deep dive.
- `prompts/snippets/test-rigor.md` — updated to reference new skill + `require_level!`.
- `CLAUDE.md` — one-line pointer added to Best Practices section.

---

## Phased implementation order

Suggested phases for the implementation plan (subject to refinement during planning):

1. **Phase 1 — Foundation crates.** Extend `test-toolkit` with the `require_level!` macro and level model. Add `biscuit-browser-harness` extracted from darkmatter. Add `SharedHarness<T>` to `biscuit-test-harness`. Migrate the three current harness consumers (darkmatter, biscuit-terminal, biscuit-tui) plus claudine's PTY tests to use the new helpers. Unblocks everything else.
2. **Phase 2 — Shared just recipes + sanity tier.** Add the 12-recipe canonical set to `just/`. Implement `_sanity` filterset and nextest filtersets. Migrate the most-used packages (claudine, darkmatter, biscuit-terminal, biscuit-file) first to validate the contract.
3. **Phase 3 — Remaining package migration.** Roll the canonical set out to all packages in `areas`. Add `_check_canonical` validator + CI workflow.
4. **Phase 4 — Bench standardization + Bencher.dev wiring.** Add new criterion benches (`biscuit-file`, `biscuit-hash`, `tree-hugger`, `renderable`). Wire darkmatter to Bencher.dev as proof.
5. **Phase 5 — Fuzz infrastructure.** Add `fuzz/` directories for `biscuit-file` and `darkmatter`. Nightly CI workflow.
6. **Phase 6 — Documentation + CI workflows.** Skill, deep dive, snippet update. Sanity / test / coverage CI workflows.

---

## Decisions

- **D1 (Q2.1) — Sanity tier time budget.** ≤15 s per package, targeting ~5 min for the whole monorepo (sequential) or ~1 min (parallel). This makes sanity usable as a frequent pre-commit / pre-push check and forces it to remain genuinely fast.
- **D2 (Q1.1 + Q1.2) — Tier enforcement via runtime helper macro in `test-toolkit`.** Extend the existing `tools/test-toolkit` crate with `require_level!(Level::L2, <available-check>)`. No proc-macro crate and no new overlapping `biscuit-test` crate. Skip-clean by default; panic when `BISCUIT_TEST_LEVEL_REQUIRED=N` is set and the harness for level `N` is unavailable. Standardized env contract:
    - `BISCUIT_TEST_LEVEL=1|2|3` — runtime gate; tests above this level skip cleanly.
    - `BISCUIT_TEST_LEVEL_REQUIRED=2` — CI use; missing harness causes a panic instead of a skip.
    - `RUN_LEVEL3=1` — explicit opt-in for OS-keyboard-injection tests.
    - Per-package `*_LEVEL2_REQUIRED` flags (e.g. `DARKMATTER_LEVEL2_REQUIRED`) are deprecated in favor of the unified variable.
- **D3 (Q4.1) — Extract `biscuit-browser-harness` now.** New workspace crate that wraps chromiumoxide with `BrowserHarness` trait (`spawn`, `render_html`, `computed_style`, `screenshot`), Chrome auto-discovery, `available()` probe, and a `BISCUIT_BROWSER_REQUIRED=1` enforcement convention mirroring `BISCUIT_TEST_LEVEL_REQUIRED`. Migrate `darkmatter/lib/tests/browser_render.rs` to consume it as the first user; future `biscuit-terminal` browser-target tests adopt it.
- **D4 (Q5.1 + Q5.3) — Fuzz via cargo-fuzz on nightly, executed in nightly CI only.** Add `fuzz/` directories with targets for the top three parser candidates: `biscuit-file` PDF extraction, `biscuit-file` JSON5/YAML/TOML, and `darkmatter` markdown. Pin nightly Rust via `fuzz/rust-toolchain.toml`. CI runs a short fuzz cycle nightly with corpus persistence. Crashes are treated as R0 regressions. Fuzz is **not** part of `sanity`, `test`, or any PR-blocking gate. Future expansion to `claudine`, `tree-hugger`, `schematic` per the candidate ranking in Topic 5.
- **D5 (Q3.2) — Default L2 backend is tmux.** Headless, portable, runs on any CI runner without a GUI. Verifies plain text, SGR, OSC8. Tests requiring graphics protocols (Kitty graphics, inline images, WezTerm-specific behaviors) explicitly opt up to the appropriate backend. The `pick_default_l2_backend()` helper proposed in Topic 3 is **not** added — explicit backend choice is preferred over runtime auto-selection.
- **D6 (Topic 7) — Canonical 12-recipe set in every curated package-area justfile, with no-op allowed.** Each package area in the root `areas` list defines: `sanity`, `test`, `test-l2`, `test-l3`, `test-browser`, `test-real`, `lint`, `bench`, `coverage`, `doctest`, `fuzz`, `all`. Recipes that don't apply are explicit no-ops with a doc comment explaining why (e.g. `# not applicable: pure data crate`). A `just _check_canonical` recipe (added to `just/lifecycle.just` or a new `just/test.just`) validates that every required recipe is present in a package area's `justfile`. CI runs `_check_canonical` against the full root `areas` list.
- **D7 (Topic 7) — `just all` execution order.** `sanity → lint → doctest → test → test-l2 → test-browser`. Fast-fail order: cheapest signals run first so the failure surfaces quickly. `test-l3`, `test-real`, `fuzz`, and `bench` are **excluded** from `all` — they require explicit opt-in (devices, OS keyboard focus, nightly toolchain, quiet CPU). Recipes that skip cleanly (e.g. `test-l2` when no harness available) do not fail the `all` run.
- **D8 (Q8.1) — Coverage is report-only.** CI generates per-package LCOV plus a workspace-aggregated report and uploads as an artifact (and to a coverage service if/when we wire one up). No PR gating on coverage percentage. Rationale: gates create perverse incentives and false alarms on legitimate refactors. Per-package `coverage` recipe is part of the canonical set; aggregator at the root.
- **D9 (Q6.2 refined) — Bench baselines via Bencher.dev SaaS, proof-of-concept scope.** Use Bencher.dev (free OSS tier) to track criterion results over time. This initiative covers:
    - Bencher.dev account + project setup + auth token in CI secrets.
    - CI workflow that runs `cargo bench` for **darkmatter** (most benches), emits criterion JSON, and pushes to Bencher.
    - Documentation of the onboarding procedure so other bench-having packages (claudine, biscuit-terminal, sniff) can be added incrementally in follow-up work.
    - No in-repo baseline snapshots.
- **D10 (Q5.2) — Fuzz corpus: in-repo seed only.** Each `fuzz/` target has a small hand-curated `corpus-seed/` directory committed to the repo. The nightly CI fuzz run grows an ephemeral corpus that is discarded between runs. Only **minimized crash inputs** are committed back (to `fuzz/crashes/<target>/`) as regression fixtures. Avoids Git LFS dependency and external storage ops overhead.
- **D11 (Q9.1 + Q9.2) — Documentation deliverables.**
    - `.claude/skills/rust-testing/SKILL.md` — agent-facing summary (≤200 lines) with a decision tree ("you're writing a test for X — start at Level N because…").
    - `docs/testing-strategy.md` — human-facing reference (long version): rationale, examples, fuzz playbook, browser-harness API, decision log.
    - `prompts/snippets/test-rigor.md` — updated to reference the new skill, canonical recipes, and `require_level!` macro.
    - `CLAUDE.md` — one-line pointer to the testing skill in the existing "Best Practices" section.
- **D12 (Q2.2 + Q2.3) — Sanity is tests only.** `sanity` runs `cargo nextest run --lib --bins -E '!set:slow'`. Doctests live in the separate `doctest` recipe (compile cost would blow the 15s budget). Lint lives in `lint`. `just all` chains them in the order from D7.
- **D13 (Q3.1 + Q6.1) — Defer xdotool; benches run sequentially at the root.** No Linux L3 backend in this initiative; revisit when a Linux CI runner needs L3. Root `just bench` iterates packages sequentially (criterion needs a quiet CPU for stable numbers). Per-package `bench` recipes may use criterion's internal parallelism but the root orchestrator does not parallelize across packages.
- **D14 (Q6.3) — Bench opt-out by documented convention only.** `[package.metadata.benchmarks] required = false` with a `reason = "..."` field is the convention, documented in the testing skill and deep dive. No `_check_bench` static checker — reviewers enforce. The field is grep-able if we ever want to add tooling later.
- **D15 (Topic 1 + Topic 2) — Nextest filtersets use stable test/module identifiers.** Because the selected tier helper is runtime-only, the implementation must add nextest filtersets based on `level2_`, `level3_`, `browser_`, `real_`, and `slow_` identifiers in test names or module paths. Migrated package tests must adopt those identifiers for non-default tiers. This makes `sanity`, `test-l2`, `test-l3`, `test-browser`, and `test-real` select predictable test sets without proc macros.
- **D16 (Drift maintenance) — Dependency and skill documentation updates are in scope.** Adding or moving dev/test crates still updates root and per-area `docs/dependencies.md` where applicable, plus `.claude/skills/rust-testing/SKILL.md`, `.claude/skills/cli/cli-best-practices.md`, and package README/testing docs that currently mention old env flags. The deprecated per-package `*_LEVEL2_REQUIRED` variables remain accepted during migration only when wrapped by the new `test-toolkit` helper.

---

## Open question summary

All brainstorming questions and inline-review gaps are resolved (D1-D16 above). No open questions remain at the spec level. Implementation-phase questions (e.g. exact Bencher.dev project name, GitHub Actions runner sizing, specific corpus seeds) will be handled during plan writing and execution.
