---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-14
start_phase: 1
yolo: true
spec: 2026-05-27-scope-complete-json/spec.md
status: phase 4 complete
source_files_during_phase_1:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/commands/mod.rs
source_files_during_phase_2:
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/tests/cli.rs
source_files_during_phase_3:
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/tests/cli.rs
source_files_during_phase_4: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
docs_updated_during_phase_4:
  - sniff/docs/cli/repo.md
  - sniff/docs/topics/json-output.md
  - sniff/docs/cli/repo_structure.md
  - sniff/cli/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .opencode/skill/sniff/SKILL.md
source_code:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/tests/cli.rs
documentation:
  - sniff/docs/cli/repo.md
  - sniff/docs/topics/json-output.md
  - sniff/docs/cli/repo_structure.md
  - sniff/cli/README.md
packages:
  - sniff-cli
---

# Execution Plan: Scope-Complete JSON for `sniff repo`

Implements the three concrete changes from the spec:

1. **Change #1 (HIGH)** — Make `sniff repo --json` aggregate its children's scopes
2. **Change #2 (HIGH)** — Add `is-monorepo`, `package-count`, `version` leaves
3. **Change #3 (HIGH)** — Fix the `repo name -v` terminal-subset leak

## Key design decisions (from spec, verified against working tree)

- **Bare-vs-explicit distinction** is the foundational seam. Currently
  `Commands::to_repo_action()` maps bare `repo` to `RepoAction::Name`
  (`cli/src/args/mod.rs:686`), so `sniff repo` and `sniff repo name` are
  indistinguishable after normalization. All three changes require splitting
  them. The spec permits either a distinct `RepoAction` variant or a
  `repo_subcommand.is_none()` check at the dispatch site.
- **No new detection.** `RepoIdentity` (`lib/src/filesystem/repo/identity.rs:29`)
  already carries `name`, `version`, `language`, `is_monorepo`,
  `package_count`. The new leaves and the aggregate consume these as-is.
- **Aggregate is offline.** `remote`, `pr` (network-primary) and `hash`
  (parameterized) are excluded. No `--refresh-remotes` / `--latest-versions`
  supplemental fields appear.
- **Keying rule.** Keys are kebab-case matching subcommand names. Single-key
  leaves contribute their unwrapped value; multi-field children contribute
  their whole scope object.

## Dependency graph

```
Phase 1 (dispatch seam + new leaves)
   ├──> Phase 2 (aggregate JSON builder)     ──┐
   └──> Phase 3 (terminal subset fix)         ──┤
                                                  └──> Phase 4 (docs + validation)
```

Phase 2 and Phase 3 are **parallelizable** after Phase 1, provided Phase 1
establishes clearly separate `cli.json` vs. text branches in the bare-`repo`
dispatch block. They touch different arms of that block.

---

## Phase 1 — Foundation: dispatch seam + three new identity leaves

**Goal:** Establish the bare-`repo` vs. explicit-`repo name` dispatch
distinction that Changes #1 and #3 depend on, and ship the three new identity
leaves (Change #2) that the aggregate will draw from.

**Validation checkpoint:** `cargo nextest run -p sniff-cli --lib` passes;
`sniff repo is-monorepo`, `sniff repo package-count`, `sniff repo version`
are invokable and produce correct text + JSON output; `sniff repo name --json`
is unchanged.

### Tasks

- [x] **1.1 — Add the bare-vs-explicit dispatch distinction.** Introduce a way
  to distinguish `sniff repo` (no subcommand) from `sniff repo name` after
  `to_repo_action()` normalization. Two options from the spec — pick one:
  (a) add a distinct `RepoAction` variant (e.g. `RepoAction::Default` or
  `RepoAction::Aggregate`) for the bare case, or
  (b) check `repo_subcommand.is_none()` at the dispatch site in
  `commands/mod.rs`.
  The `to_repo_action()` return for bare `repo` must no longer collapse to
  `RepoAction::Name`.
  Files: `cli/src/args/mod.rs:683-686`, `cli/src/args/repo.rs:35`,
  `cli/src/commands/mod.rs:462`.

- [x] **1.2 — Update the `to_repo_action_none_is_name` test.** The existing
  test (`cli/src/args/mod.rs:2221`) asserts bare `repo` → `RepoAction::Name`.
  Update it to assert the new bare-`repo` behavior (new variant or `None`).
  Add a companion test asserting explicit `repo name` still → `RepoAction::Name`.

- [x] **1.3 — Add `is-monorepo`, `package-count`, `version` to `RepoSubcommand`.**
  Add three new unit variants (or struct variants with optional flags — see
  task 1.5) to the clap `RepoSubcommand` enum in `cli/src/args/repo.rs`,
  following the style of existing identity leaves like `Name` (line 619) and
  `Language` (line 587). Use kebab-case `#[command(name = "...")]` where the
  Rust identifier differs.
  Files: `cli/src/args/repo.rs`.

- [x] **1.4 — Add `IsMonorepo`, `PackageCount`, `Version` to `RepoAction`.**
  Add the three corresponding variants to the normalized `RepoAction` enum
  (`cli/src/args/repo.rs:35`). `Version` should carry `no_error: bool` /
  `on_error: Option<String>` (see task 1.5); `IsMonorepo` and `PackageCount`
  are unit variants since their values are always known when identity
  detection succeeds.
  Files: `cli/src/args/repo.rs:35`.

- [x] **1.5 — Wire `to_repo_action()` mappings for the three new leaves.**
  Map each new `RepoSubcommand` variant to its `RepoAction` counterpart in
  `to_repo_action()` (`cli/src/args/mod.rs:683`). For `version`, include
  `no_error` / `on_error` fields matching the pattern used by `Worktree`
  (`cli/src/args/repo.rs:594`) and `Package` (`cli/src/args/repo.rs:405`),
  since `version` may be absent. `is-monorepo` and `package-count` do not
  need these flags (spec: "always known when repo identity detection succeeds").
  Files: `cli/src/args/mod.rs:683-710`.

- [x] **1.6 — Add JSON outcome builders for the three new leaves.** In
  `cli/src/output/repo_json.rs`, add focused builders following the existing
  `name_outcome` (line 317) and `worktree_outcome` (line 343) patterns:
  - `is_monorepo_outcome(value: bool)` → `{ "is-monorepo": bool }`, exit 0.
  - `package_count_outcome(count: usize)` → `{ "package-count": N }`, exit 0.
  - `version_outcome(version: Option<&str>, no_error: bool)` →
    `{ "version": "..." | null }`, exit 1 on `None` unless `no_error`.
  Keys are kebab-case to match the subcommand name. Add unit tests in the
  existing `tests` module mirroring the `name_outcome_wraps_string` /
  `name_outcome_empty_sets_exit_code_one` pattern (line 1181).

- [x] **1.7 — Add command handlers for the three new leaves.** In the
  early-return dispatch block (`cli/src/commands/mod.rs:465-883`), add arms
  for `RepoAction::IsMonorepo`, `RepoAction::PackageCount`, and
  `RepoAction::Version`. Each should call `detect_repo_identity` (same as the
  existing `Name` arm at line 681), then route to the new JSON outcome
  builder under `--json`, or print plain text otherwise. Text form follows
  existing single-value leaf conventions (`yes`/`no` for is-monorepo; the
  count as plain text for package-count; the version string or blank for
  version). Respect stdout/stderr discipline from the sniff best practices
  (data on stdout, diagnostics on stderr).

- [x] **1.8 — Update `REPO_AFTER_HELP`.** Add the three new leaves to the
  "Identity" section of the after-help string (`cli/src/args/mod.rs:1105`),
  and fix the `sniff repo name -v` line to reflect that the rich one-liner is
  moving to `sniff repo -v` (this is a forward-looking edit; Phase 3 completes
  the actual terminal behavior change). Add a brief note about the JSON
  modes.

- [x] **1.9 — Add parser unit tests for the three new leaves.** In
  `cli/src/args/mod.rs` tests module (near line 2197), add tests asserting
  `to_repo_action()` correctly maps each new `RepoSubcommand` variant to its
  `RepoAction`.

- [x] **1.10 — Validate Phase 1.** Run `cargo nextest run -p sniff-cli --lib`
  and `cargo nextest run -p sniff-cli --test cli` (filtered to repo tests).
  Manually verify: `sniff repo is-monorepo --json`, `sniff repo package-count
  --json`, `sniff repo version --json` each return a single-key object;
  `sniff repo name --json` still returns `{ "name": "..." }` (regression
  guard).

---

## Phase 2 — `sniff repo --json` aggregate builder (Change #1)

**Goal:** Make `sniff repo --json` return the aggregate of its participating
children's scopes, keyed by kebab-case subcommand name. Depends on Phase 1
(new leaves + dispatch seam).

**Parallelizable with Phase 3** after Phase 1, since this phase touches only
the `cli.json` branch of the bare-`repo` dispatch.

**Validation checkpoint:** `sniff repo --json` returns a flat JSON object
whose keys are exactly the participating children; `sniff repo <key>` is
invokable for each key; `remote`, `pr`, `hash` keys are absent; no network
call is made.

### Tasks

- [x] **2.1 — Factor file-list JSON value construction into reusable helpers.**
  The file-list commands (`staged-files`, `unstaged-files`, `untracked-files`,
  `dirty-source-code`, `staged-source-code`, `unstaged-source-code`,
  `dirty-files`) currently route through `handle_file_list_command` which
  early-exits on "no results". Extract a value-only builder (e.g.
  `file_list_value(scope, kind, paths) -> Value`) that returns the stable
  JSON shape (`{ "scope": "...", "kind": "...", "paths": [] }`) without
  exiting. The aggregate calls this helper; direct leaf invocation preserves
  its existing exit-code behavior.
  Files: `cli/src/output/repo_json.rs`, `cli/src/commands/mod.rs` (file-list
  handler).

- [x] **2.2 — Factor locator and boolean value construction for aggregate use.**
  Verify the existing builders in `repo_json.rs` (`locator_root_outcome`,
  `name_outcome`, `has_merge_conflict_outcome`, boolean outcomes) can be
  called in value-only mode from the aggregate. The `BuildOutcome` pattern
  already separates value from exit code — extract `.value` where the
  aggregate calls them. If any builder performs detection inline (e.g.
  `IsCurrentPackageAreaDirty` at `repo_json.rs:221`), ensure the aggregate
  can supply the pre-computed detection result instead of re-detecting.
  Files: `cli/src/output/repo_json.rs`.

- [x] **2.3 — Factor commit-family JSON value construction for aggregate use.**
  `handle_recent_commits_command` (`cli/src/output/recent_commits.rs:13`)
  early-exits via `handle_no_results` when `commit_set.commits.is_empty()`.
  Extract a value-only builder that returns the focused JSON shape
  (`commits`, `period_label`, `repo_root`, plus `packages`/`filter`) with
  empty arrays instead of exiting, so the aggregate can include
  `recent-commits`, `source-code-changes`, and `documentation-changes` with
  their stable empty shape.
  Files: `cli/src/output/recent_commits.rs`.

- [x] **2.4 — Build the aggregate assembler.** Create a new function (e.g.
  `build_aggregate_value(result: &SniffResult, base_dir, identity: &RepoIdentity)
  -> Value`) in `cli/src/output/repo_json.rs` that assembles a single
  `serde_json::Map` by invoking each participating child's value builder and
  inserting under its kebab-case key. Apply the keying rule:
  - Single-key leaves (`name`, `version`, `language`, `is-monorepo`,
    `package-count`, `worktree`, `package`, `package-area`, `area`,
    `package-root`, `package-area-root`, `root`) contribute their **unwrapped
    value** under that key.
  - Multi-field children (`structure`, `packages`, `package-areas`, `deps`,
    `git-status`, `worktrees`, file-list leaves, package-family leaves,
    boolean leaves, commit-family leaves) contribute their **whole scope
    object** under their subcommand key.
  - **Excluded:** `hash` (parameterized), `remote` and `pr` (network-primary).
  Reuse existing focused builders; do not hand-roll new serialization.

- [x] **2.5 — Implement the aggregate error policy.** The aggregate must not
  silently omit participating keys. A child's "no value" state that already
  has a stable JSON shape remains in the aggregate as that stable value
  (`null`, `""`, `[]`, `{ ... }`, or `false`). If required local detection
  fails in a way that prevents a scope-complete aggregate, the parent command
  fails — in `--json` mode, stdout must contain either the valid aggregate or
  nothing; diagnostics go to stderr. Add a `Result` return to the assembler
  and propagate detection errors.

- [x] **2.6 — Wire the aggregate into bare-`repo --json` dispatch.** In the
  bare-`repo` dispatch path established in task 1.1, route the `cli.json ==
  true` case to the new aggregate builder. The builder needs a `SniffResult`
  from the full detection pass (the path that currently falls through to
  `build_with_outcome` at `cli/src/commands/mod.rs:878+`), so the aggregate
  dispatch must run after detection completes, not as an early return. Print
  via `output::print_json_value`.

- [x] **2.7 — Add aggregate unit tests.** In `cli/src/output/repo_json.rs`
  tests module, add tests using existing `SniffResult` fixtures (e.g.
  `fixture_with_git_and_repo` at line 575):
  - Aggregate keys are exactly the participating children set.
  - `remote`, `pr`, `hash` keys are absent.
  - Single-key leaves are unwrapped (e.g. `"name": "..."` not
    `"name": { "name": "..." }`).
  - Empty-value children still present with stable shape (`"worktree": null`,
    `"version": null`, `"package": ""`, `"staged-files": { ..., "paths": [] }`).

- [x] **2.8 — Add aggregate integration tests.** In `cli/tests/cli.rs`, add
  tests (following the `repo_name_json_is_leaf_only` pattern at line 257):
  - `repo_aggregate_json_keys_round_trip`: top-level keys match invokable
    subcommands; walk each key and assert `sniff repo <key>` is accepted.
  - `repo_aggregate_json_excludes_network_and_parameterized`: assert `remote`,
    `pr`, `hash` keys absent.
  - `repo_aggregate_json_is_offline`: assert no `--refresh-remotes` /
    `--latest-versions`-only fields in `structure` / `git-status` sub-objects.
  - `repo_aggregate_json_not_partial`: assert all participating keys present
    even when some have empty values.

- [x] **2.9 — Validate Phase 2.** Run
  `cargo nextest run -p sniff-cli --lib` and the aggregate integration tests.
  Manually verify `sniff repo --json` output matches the spec's target shape
  (spec lines 86-125). Verify `sniff repo name --json` is still leaf-only
  (regression guard from the shipped `git-identity-request` work).

---

## Phase 3 — Terminal subset fix for `repo name -v` (Change #3)

**Goal:** Make `repo name`'s terminal output match its `{ name }` JSON scope
at all verbosities, and move the rich one-liner to the `repo` parent's
default-dispatch text form where every field it shows is in the aggregate
scope.

**Parallelizable with Phase 2** after Phase 1, since this phase touches only
the text branch of the bare-`repo` dispatch.

**Validation checkpoint:** `sniff repo name -v` prints only the name;
`sniff repo -v` prints the rich one-liner (`**name** vX [<n> package
monorepo]` / `[<language>]`); `sniff repo` (no flags) prints the bare name.

### Tasks

- [x] **3.1 — Make `render_repo_name` leaf-only.** Modify
  `render_repo_name` (`cli/src/output/filesystem/repo.rs:22`) so it emits only
  the bare name at all verbosity levels. Verbose may add contextual styling
  (e.g. bold) but **no foreign data fields** (no version, language,
  is_monorepo, package_count). This brings the terminal output into lockstep
  with the `{ name }` JSON scope.
  Files: `cli/src/output/filesystem/repo.rs:22-53`.

- [x] **3.2 — Create a parent-level rich renderer.** Extract the rich
  one-liner logic (version suffix, monorepo package-count suffix, language
  suffix) into a new function (e.g. `render_repo_default_verbose(identity:
  &RepoIdentity) -> String`) that draws from the full `RepoIdentity`. This
  renderer is used by the bare-`repo` parent text dispatch at `-v`. Every
  field it shows (`name`, `version`, `package_count`, `is_monorepo`,
  `language`) is in the parent aggregate scope, so this is compliant.
  Files: `cli/src/output/filesystem/repo.rs`.

- [x] **3.3 — Wire the parent text dispatch for bare `repo`.** In the
  bare-`repo` dispatch path (task 1.1), route the text (`cli.json == false`)
  case:
  - v0: bare name (a subset of the aggregate — compliant, unchanged).
  - `-v`: the rich one-liner via the new `render_repo_default_verbose`.
  Both paths use `detect_repo_identity` (cheap, same as today's `Name` arm).
  Continue using `biscuit-terminal` renderable components (`Prose`).
  Files: `cli/src/commands/mod.rs` (bare-`repo` dispatch block).

- [x] **3.4 — Add terminal-subset tests.** Add tests asserting:
  - `sniff repo name -v` prints only the name (no version/language/monorepo
    data) — use `assert_cmd` with `NO_COLOR=1` and assert on stdout content.
  - `sniff repo -v` still prints the rich one-liner (assert it contains
    version or package-count suffix when present).
  - `sniff repo` (no flags) prints the bare name.
  Follow the existing integration test style in `cli/tests/cli.rs`.
  Files: `cli/tests/cli.rs`.

- [x] **3.5 — Validate Phase 3.** Run `cargo nextest run -p sniff-cli --test cli`
  (filtered to repo/name tests). Manually verify the three terminal forms
  (`repo`, `repo -v`, `repo name`, `repo name -v`).

---

## Phase 4 — Documentation, drift cleanup & full-suite validation

**Goal:** Bring all documentation into compliance with the shipped behavior
(per the repo's `CLAUDE.md` Drift Maintenance rule — docs must be updated in
the same change as the code), fix pre-existing drift, and run the full
validation suite.

**Validation checkpoint:** `just lint`, `just test` (sniff area), and
`just doctest` (sniff area) all pass; all doc targets verified against
implemented behavior.

### Tasks

- [x] **4.1 — Update `sniff/docs/cli/repo.md`.** Three edits:
  - **Fix pre-existing drift (line 3):** correct "alias for `sniff repo
    structure`" to "alias for `sniff repo name`" — code defaults to
    `RepoAction::Name` (`args/mod.rs:686`), and `json-output.md:52` already
    says `name`. This work rewrites that default-dispatch behavior, so the
    correction belongs here.
  - **`## Subcommands` section:** add `is-monorepo`, `package-count`,
    `version` (from Phase 1 / Change #2) to the appropriate category tables.
  - **`## JSON Output` section:** document the `repo --json` aggregate
    (from Phase 2 / Change #1), including the keying rule and the
    offline/network-exclusion policy. Note that `remote`, `pr`, and `hash`
    are excluded.
  Files: `sniff/docs/cli/repo.md`.

- [x] **4.2 — Update `docs/topics/json-output.md`.** Verify the aggregate
  example (lines 43-50) matches the implemented key set after Phase 2 lands.
  Update if the example's key list or shape drifted from the actual output.
  Files: `sniff/docs/topics/json-output.md`.

- [x] **4.3 — Update `.opencode/skill/sniff/SKILL.md`.** Document the
  aggregate, the three new leaves, and the offline-aggregate network policy.
  Add the new leaves to the CLI examples section and note the aggregate
  behavior under `sniff repo --json`.
  Files: `.opencode/skill/sniff/SKILL.md`.

- [x] **4.4 — Update `sniff/cli/README.md`.** If the output-modes section
  enumerates repo subcommands, add the three new leaves and note the
  aggregate JSON mode.
  Files: `sniff/cli/README.md`.

- [x] **4.5 — Verify cross-references in `repo_structure.md`.** Touch
  `sniff/docs/cli/repo_structure.md` only if the default-subcommand
  correction (task 4.1) changes any cross-reference pointing at it as the
  default. If no references are affected, skip.
  Files: `sniff/docs/cli/repo_structure.md`.

- [x] **4.6 — Do NOT modify `docs/topics/terminal-output.md`.** The spec
  applies the stricter verbose-subset rule only to `repo name`; it should
  not silently rewrite the broader terminal policy. Only update this file if
  the team explicitly decides to make the behavior a repo-wide standard
  (out of scope per spec).

- [x] **4.7 — Run clippy + fmt.** Execute `cargo clippy --color=never -- -D
  warnings` and `cargo fmt --check` for the sniff-cli and sniff-lib crates.
  Fix any issues introduced by this work.
  Command: `just lint` (sniff area) or direct cargo invocations.

- [x] **4.8 — Run the full sniff test suite.** Execute `just test` (sniff
  area) or `cargo nextest run -p sniff-cli -p sniff-lib`. Ensure all L1
  tests pass, including the new aggregate, leaf, and terminal-subset tests.

- [x] **4.9 — Run doctests.** Execute `cargo test --doc -p sniff-cli -p
  sniff-lib` or `just doctest` (sniff area). Ensure the `detect_repo_identity`
  doctest and any new doc examples pass.

- [x] **4.10 — Final manual smoke test.** Run the following commands and
  verify output matches the spec's target shape and the scope-complete
  principle:
  - `sniff repo --json` — aggregate with all participating children.
  - `sniff repo name --json` — leaf-only `{ "name": "..." }`.
  - `sniff repo is-monorepo --json` — `{ "is-monorepo": true }`.
  - `sniff repo package-count --json` — `{ "package-count": N }`.
  - `sniff repo version --json` — `{ "version": "..." | null }`.
  - `sniff repo name -v` — name only (no foreign fields).
  - `sniff repo -v` — rich one-liner.
  - `sniff repo` — bare name.

---

## Risk notes

- **Dispatch-site contention.** Phases 2 and 3 both modify the bare-`repo`
  dispatch block in `commands/mod.rs`. If parallelized, coordinate so Phase 2
  owns the `cli.json` branch and Phase 3 owns the text branch. Phase 1
  (task 1.1) should establish this split skeleton before either phase begins.
- **Factoring early-exit behavior.** Several existing leaf handlers
  (`handle_file_list_command`, `handle_recent_commits_command`,
  `handle_no_results`) call `std::process::exit` on the no-results path. The
  aggregate must not trigger those exits. Task 2.1-2.3 extracts value-only
  builders; direct leaf invocation must preserve its existing exit codes.
- **`version` no-result semantics.** The spec permits `version` to support
  `--no-error` / `--on-error` "if that can be done without widening global
  CLI semantics." The existing `Worktree` / `Package` / `Area` leaves already
  use this pattern, so task 1.5 follows it. If the implementation reveals a
  conflict, fall back to exit-1-on-null without the flags.
- **Aggregate size.** The aggregate invokes every participating child's
  builder. For large monorepos this may be slower than today's single-leaf
  emit. The spec does not set a performance budget, but the implementation
  should reuse the single detection pass's `SniffResult` rather than
  re-detecting per child.
