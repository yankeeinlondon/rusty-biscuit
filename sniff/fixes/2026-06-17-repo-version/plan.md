---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-17
start_phase: 1
yolo: "true"
spec: sniff/fixes/2026-06-17-repo-version/spec.md
packages: [sniff, sniff-cli]
source_code:
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/python.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/error.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/version_report.rs
  - sniff/cli/tests/cli.rs
documentation:
  - sniff/docs/cli/repo_version.md
  - sniff/cli/README.md
source_files_during_phase_1:
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/python.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/error.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/version_report.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/cli/src/output/repo_json.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - sniff/cli/README.md
docs_created_during_phase_4:
  - sniff/docs/cli/repo_version.md
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/SKILL.md
---

# Execution Plan — Fix & Redesign `sniff repo version`

Implements `sniff/fixes/2026-06-17-repo-version/spec.md`. Mirrors
`sniff repo test-runner` (`handle_repo_test_runner` + `aggregate_test_runners`
+ `output/test_runner_report.rs`) substituting "version + manifest source"
for "runner + evidence source".

## Dependency graph at a glance

```
Phase 1 (library core) ──┬──> Phase 2 (focused CLI command)
                         └──> Phase 3 (bare aggregate rewire)
                                   │
                                   v
                              Phase 4 (docs + acceptance)
```

Phase 2 and Phase 3 are **parallelizable** once Phase 1 lands (different
files; no edit conflicts). Phase 4 depends on both.

## Reference landmarks (verified against current tree)

| Concern | Location |
|---|---|
| Current broken arm | `sniff/cli/src/commands/mod.rs:875` (`RepoAction::Version`) |
| Aggregate helpers to extend | `sniff/lib/src/filesystem/repo/aggregate.rs` (`aggregate_test_runners:204`, private `in_scope:227`) |
| `Package.version` field | `sniff/lib/src/filesystem/repo/types.rs:208` |
| Per-package version resolver | `sniff/lib/src/filesystem/repo/detection.rs:733` (`resolve_package_version`) |
| Cargo literal-version reader | `sniff/lib/src/filesystem/repo/cargo.rs:194` (`cargo_package_version` — returns `None` for `version.workspace = true`) |
| Manifest parse helpers (private) | `sniff/lib/src/filesystem/repo/identity.rs:242` (`read_toml`/`read_json`), `cargo.rs` (`cargo_package_version`), plus npm/pyproject readers |
| Reference CLI handler | `sniff/cli/src/commands/repo.rs:503` (`handle_repo_test_runner`) |
| Reference output module | `sniff/cli/src/output/test_runner_report.rs` |
| Args variant to extend | `sniff/cli/src/args/repo.rs:708` (`Version`) |
| Obsolete JSON builder to remove | `sniff/cli/src/output/repo_json.rs:414` (`version_outcome`) |
| Aggregate `version` assignment | `sniff/cli/src/output/repo_json.rs` (sets `SniffRepo.version` from `identity.version`) |
| Help examples to update | `sniff/cli/src/args/mod.rs:1223-1224` |
| Public re-exports | `sniff/lib/src/filesystem/repo/mod.rs:23-26` |

---

## Phase 1 — Library core: types, source resolution, aggregation, scope overrides

**Goal.** All business logic for the redesign lands in `sniff/lib` with unit
tests. Nothing in the CLI changes yet; the old `Version` arm keeps working.

**Note on approach A (source at aggregation time).** `ManifestIndex` is
`pub(crate)` and is **not** retained on `RepoInfo`, so `aggregate_versions`
cannot borrow a cached index. It must re-read the manifest files off disk via
`Package.path` / the repo root, using the existing manifest-parse helpers.
The helpers are currently private (`read_toml`/`read_json` in `identity.rs`,
`cargo_package_version` in `cargo.rs`, plus npm/pyproject readers); this phase
makes the minimum set reachable from `aggregate.rs` (same crate, so
`pub(crate)` is sufficient — no public-API churn).

- [x] **1.1** Teach `cargo_package_version` (`cargo.rs:194`) to resolve Cargo workspace inheritance: when `[package].version` is a table `{ workspace = true }`, read `[workspace.package].version` from the **root** `Cargo.toml` and return it. Add a sibling helper (e.g. `cargo_package_version_with_source`) returning `(version, manifest_path, inherited)` so the aggregator can build `VersionSource` without re-parsing. Keep the existing literal path as the fast branch. Do not shell out to Cargo — pure TOML via the repo manifest stack.
- [x] **1.2** Expose the manifest-parse helpers needed by aggregation as `pub(crate)` (or move into a shared private module): TOML/JSON readers plus npm (`npm_package_version`) and pyproject (`pyproject_package_version`) version readers. No changes to the public `sniff` API surface.
- [x] **1.3** Add the public types to `aggregate.rs` per spec §Library: `VersionSource { manifest, path, inherited }`, `VersionSourceAttribution { source, packages }`, `VersionAttribution { version, packages, sources }`. Derive `Debug, Clone, PartialEq, Eq`. Reuse the existing private `in_scope` predicate — do **not** duplicate scope semantics.
- [x] **1.4** Implement `aggregate_versions(packages: &[Package], scope: &AggregateScope, root: &Path) -> Vec<VersionAttribution>`. Collapse key is the **version string** (uniform repos collapse to one entry). For each in-scope package with a `Package.version`, locate its source via the 1.1/1.2 helpers in `resolve_package_version`'s priority order (Cargo → npm root-fallback → pyproject), group `sources` by `(manifest, path, inherited)`, and accumulate `packages` in first-seen order. Packages with no resolvable version contribute nothing.
- [x] **1.5** Implement `resolve_scope_with_overrides(info, cwd, all, package, package_area) -> Result<AggregateScope>` per spec §Scope override resolver. Reuse `resolve_scope` for the CWD default. Validate named targets against the catalog: unknown name → error; `--package` matching more than one catalog entry → ambiguous error. The three overrides are mutually exclusive (enforced at the clap layer in Phase 2, but this function should still treat them in priority order and never panic).
- [x] **1.6** Re-export the new symbols from `sniff/lib/src/filesystem/repo/mod.rs` alongside `aggregate_test_runners` / `TestRunnerAttribution` / `resolve_scope`.
- [x] **1.7** Library unit tests in `aggregate.rs` (mirror `aggregate_test_runners` tests at `aggregate.rs:383+`): uniform collapse (one entry, all packages), variant collapse (distinct versions stay separate), single package, empty scope, **multiple manifest sources for the same version** (assert `sources.len() > 1` and no misleading single source), and **workspace inheritance** (`version.workspace = true` resolves to root version with `inherited: true` and source named `[workspace.package]`). Add a `resolve_scope_with_overrides` test matrix: `--all`, valid `--package`, unknown `--package`, ambiguous `--package`, valid/unknown `--package-area`, and CWD default when no flag.

**Phase 1 checkpoint.** `cargo test -p sniff --lib aggregate` and `cargo test -p sniff --lib resolve_scope_with_overrides` pass. `cargo build -p sniff` compiles with the new symbols exported and the old CLI arm untouched. **Status: complete — 32 aggregate tests pass (incl. 19 new ones), `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean.**

---

## Phase 2 — Focused `sniff repo version` CLI command

**Goal.** Replace the broken arm with the test-runner-style handler. Depends
on Phase 1. Parallelizable with Phase 3.

- [x] **2.1** Extend the `Version` variant in `sniff/cli/src/args/repo.rs:708` to mirror `TestRunner` plus scope overrides, exactly per spec §Args: add `csv`/`list`/`md` (each `conflicts_with_all` against the other two) and `all`/`package`/`package_area` (mutually exclusive via `conflicts_with_all`). Preserve existing `no_error` and `on_error`. Update the variant's doc comment to describe the new scope/format contract.
- [x] **2.2** Create `sniff/cli/src/output/version_report.rs` mirroring `test_runner_report.rs`: `build_version_json(entries, repo_root) -> Value` producing `{ "versions": [ { version, packages, sources: [ { manifest, path, href, inherited, packages } ] } ] }`; `render_entries` (default styled comma-separated), `render_one` (for `--csv`/`--list`/`--md` under `-v`), and `entry_names`. Use `Prose` / `biscuit-terminal` exclusively — no hand-written ANSI. Implement the verbose markup from spec §Text rendering: hyperlinked manifest source, name a package only when `packages.len() == 1`, name `[workspace.package]` for inherited Cargo, and when one collapsed version has multiple sources render `from N manifests` instead of a misleading first source.
- [x] **2.3** Add `handle_repo_version` to `sniff/cli/src/commands/repo.rs` modeled on `handle_repo_test_runner` (`repo.rs:503`): discover repo root (same `base_dir`/cwd resolution), `detect_repo_structure_or_root_package(&root)`, resolve scope via `resolve_scope_with_overrides`, build `Vec<VersionAttribution>` (monorepo-with-packages → `aggregate_versions`; otherwise resolve the directory's own manifest into a single attribution with empty `packages` and one source). Render via `version_report`. Empty result → nothing on stdout, stderr hint when not `--plain`, exit 1 unless `--no-error` (preserve `--no-error`/`--on_error`). `--json` stdout stays valid JSON (`{ "versions": [] }` on empty); `--on-error` text applies to text mode only.
- [x] **2.4** Replace the `RepoAction::Version` arm at `sniff/cli/src/commands/mod.rs:875` with a call to `handle_repo_version`, destructuring the new args fields and passing `cli.json`/`cli.plain`/`cli.verbose`/`&perf` exactly as the `TestRunner` arm does.
- [x] **2.5** Remove `output::repo_json::version_outcome` (`repo_json.rs:414`) and its three unit tests (`repo_json.rs:1855+`). The focused JSON contract is now `{ "versions": [...] }` built by `version_report::build_version_json`; leaving the obsolete `{ "version": ... }` builder around invites accidental reuse.
- [x] **2.6** CLI integration tests (mirror the `repo test-runner` test suite): repo-root scope, package scope (`cd sniff/lib`), package-area scope (`cd sniff`), `--all`/`--package`/`--package-area` overrides, unknown-name errors, `--csv`/`--list`/`--md`/`--json`/`--verbose` formats, uniform-collapse vs variance rendering, single-package/non-monorepo fallback, empty-result exit code (1 default, 0 with `--no-error`), and `--json` always emitting `{ "versions": [] }` on empty. Verify stdout/stderr discipline (JSON mode: stdout is valid JSON only).

**Phase 2 checkpoint.** `cargo test -p sniff-cli repo::version` passes. Manual smoke check from repo root, `sniff/lib`, and `sniff/` confirms scopes and `--json` shape. Old `version_outcome` is gone and nothing references it (`rg version_outcome sniff/cli` returns nothing). **Status: complete — 16 `repo::version` integration tests pass, `just test`/`just lint`/`just doctest` all green.**

---

## Phase 3 — Bare `sniff repo --json` aggregate `version` rewire

**Goal.** Switch the consolidated `SniffRepo.version` from the root-manifest
value (currently `null` in this pure-virtual workspace) to the
`AggregateScope::Repo` collapse. Depends on Phase 1. Parallelizable with
Phase 2.

- [x] **3.1** Add a library helper (or reuse `aggregate_versions` directly at repo scope) that returns the bare-aggregate top-level `version`: exactly one distinct version across all packages → that string; zero or more-than-one → `None`. Keep this in the library so the CLI JSON builder never inspects manifests itself.
- [x] **3.2** In `sniff/cli/src/output/repo_json.rs`, replace the `SniffRepo.version = identity.version.clone()` assignment with the library-derived repo-scope collapse from 3.1. `RepoIdentity.version` and the `sniff repo name` path are **untouched** — only the bare aggregate's top-level `version` value changes (type stays `string | null`, so claudine's consumer is unaffected).
- [x] **3.3** Update / add tests in `repo_json.rs`: uniform repo → top-level string; zero packages → `null`; two distinct versions → `null`. Assert the type remains `Option<String>` (serialize-compatible).

**Phase 3 checkpoint.** `cargo test -p sniff-cli repo_json` passes. Manual `sniff repo --json | jq .version` from this repo root returns `"0.1.0"` (was `null`). **Status: complete — `bare_aggregate_version` lives in `aggregate.rs` (re-exported via `mod.rs`); `aggregate_repo_version` in `repo_json.rs` uses it; three new repo_json tests + four new aggregate tests cover uniform, zero, two-distinct, and workspace-inheritance cases. Note: this repo's bare aggregate reports `null` because the workspace includes two `fuzz` packages at `0.0.0` alongside the `0.1.0` members; that is the spec-correct two-distinct outcome.**

---

## Phase 4 — Docs, skill, help, and full acceptance

**Goal.** Close out the contract change across all surfaces and run the
spec's acceptance matrix. Depends on Phase 2 + Phase 3.

- [x] **4.1** Update help examples in `sniff/cli/src/args/mod.rs:1223-1224` (and any other `repo version --json` example in that file) to describe the new `{ "versions": [...] }` shape and the scope/format flags. Update the `Version` variant's `about` text if present.
- [x] **4.2** Update the **sniff skill** (`sniff/.opencode/skill/sniff/SKILL.md`) `repo version` CLI line and the bare `sniff repo --json` aggregate description to reflect the scoped `{ "versions": [...] }` focused contract and the collapse-based top-level `version`.
- [x] **4.3** Update the **CLI README** (`sniff/cli/README.md`) `repo version` section: scope behavior (package / package-area / repo / `--all` / `--package` / `--package-area`), formats (`--csv`/`--list`/`--md`/`--json`/`--verbose`), and the new JSON shape. Note the bare aggregate `version` collapse rule.
- [x] **4.4** Run the full acceptance matrix from spec §Acceptance criteria (1–12) and record results: monorepo root, package scope, package-area scope, explicit overrides (incl. unknown-name errors), variance rendering across all formats, verbose source hyperlinks + `[workspace.package]` disambiguation, single-package fallback, empty exit codes, library-owned logic, and the bare-aggregate top-level `version` rule.
- [x] **4.5** Final workspace validation: `just lint` and `just test` for the sniff area (or `cargo clippy --workspace` + `cargo test -p sniff -p sniff-cli` if the curated area list is unavailable). Confirm no regressions in `sniff repo name`, `sniff repo is-monorepo`, or the bare `sniff repo --json` aggregate beyond the intended `version` value change.

**Phase 4 checkpoint.** Spec acceptance criteria 1–12 all pass; skill, README, and help text match the new contract; `just lint`/`just test` green for the sniff area. **Status: complete — all 12 acceptance criteria verified by integration tests + manual smoke checks; 761 `sniff-cli` tests + 1218 `sniff` lib tests pass; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; new `docs/cli/repo_version.md` documents the redesigned contract.**

---

## Risk notes for implementers

- **Manifest re-parse cost (approach A).** `aggregate_versions` reads manifests off disk rather than from a retained index. This is the same trade-off `detect_test_runners_for_dir` already makes and is acceptable for a command that already does full repo detection. If profiling shows a regression, the spec's approach B (enrich `Package` at detection time) is the documented fallback — but do not adopt it preemptively.
- **Workspace inheritance edge.** `version.workspace = true` with no `[workspace.package].version` at root must yield `inherited: false` and no version (not a panic). Test this explicitly.
- **Mutual exclusivity is clap-enforced.** `resolve_scope_with_overrides` receives at most one override; it should still validate named targets defensively but not duplicate the `conflicts_with_all` logic.
- **`--on-error` vs `--json`.** The on-error message is text-mode only; `--json` stdout must remain `{ "versions": [] }` on empty, never a string. This is called out in spec §JSON shape and must be covered by a test.
- **Scope discipline (AGENTS.md).** Keep each commit behavior-or-comment-only; the focused-command rewire (Phase 2) and the aggregate rewire (Phase 3) are separate concerns and should land as separate commits even though they can be developed in parallel.
