---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-13
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/filesystem/packages.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/bin/render_git_status_fixture.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/tests/snapshots/snapshots__os_json_summary.snap
docs_updated_during_phase_1:
  - sniff/lib/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/request.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/bin/render_git_status_fixture.rs
  - sniff/lib/tests/git_parity.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_3:
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - sniff/lib/src/request.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/benches/cases/git_ops.rs
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/benches/ci-bench-ids.txt
docs_updated_during_phase_4:
  - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - sniff
  - sniff-cli
source_code:
  - sniff/cli/src/bin/render_git_status_fixture.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/filesystem/packages.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/snapshots/snapshots__os_json_summary.snap
  - sniff/lib/benches/cases/git_ops.rs
  - sniff/lib/benches/ci-bench-ids.txt
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/request.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
documentation:
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
---

# Execution Plan: A Status-Free Git Identity Request Level

Source spec: [`spec.md`](./spec.md). Every citation below was re-verified against
the working tree on 2026-06-13 and matches the spec's "Current State" section.

## Verified Foundation (planning assumptions)

These facts were confirmed by reading the source and drive the plan's ordering:

- `GitRequest` has **9 fields** and 4 presets (`sniff/lib/src/request.rs:279-380`).
  `is_minimal()` (`:409`) is true for `minimal()`/`summary()`; `wants_repo_metadata()`
  (`:427`) is false for both. There is **no** status-free branch today.
- `GitRepo::detect_with_request` (`sniff/lib/src/filesystem/git/types.rs:798`) is the
  single funnel. Its first statement is `let current_branch = self.try_current_branch()?;`
  (`:805`). The status block (`:824-856`) has three branches, **all** of which walk the
  tree. This is exactly where the identity early-return must slot in.
- The three status entry points to gate (success-criterion #1's proof points) are
  `get_repo_status_with_changes` (`status.rs:58`), `is_repo_dirty` (`status.rs:866`),
  and `get_repo_status_counts_detailed` (`status.rs:903`).
- Zero-status getters already exist on `GitRepo`: `repo_root()` (`:587`),
  `head_id()` (`:603`), `try_current_branch()` (`:638`), `in_worktree()` (`:653`),
  `base_repo_root()` (`:658`), `org_and_repo()` (`:672`, reads remote **URLs** only —
  no network, no ref walks).
- `GitInfo.status` is a non-optional `RepoStatus` (`types.rs:987`). The struct is
  constructed in one place (`detect_with_request`, `:928-943`) plus CLI fixtures.
- The filesystem stage discovers the handle exactly once and threads it into the git
  stage (`sniff/lib/src/filesystem/mod.rs:101-129`). The identity level needs only to
  return early from `detect_with_request`; no plan-level plumbing changes are required
  for correctness — only validation.
- `detect_repo_identity` (`repo/identity.rs:71`) calls `filesystem::repo_root` →
  `trusted_discover`, i.e. a **second** discovery independent of the plan path. It is
  reached by `sniff repo name`, **not** by `detect_with_plan`, so the L9 dedup (spec §3)
  is secondary and off the critical path.

### Two decisions the plan commits to (per spec open questions)

1. **Discriminator field.** `identity()` cannot be told apart from `minimal()` by flags
   alone (both are all-false today; `summary()` is byte-identical to `minimal()`). The
   cleanest, most honest discriminator is a new `#[serde(default)] pub identity_only: bool`
   field on `GitRequest`, set true only by `identity()`. `is_identity_only()` returns
   that field. Rejected alternative: reusing an existing flag as a sentinel (ambiguous,
   fragile). This is an additive public-API change in service of the feature and is in
   scope (the "missing setters" L6 item in Out-of-Scope is unrelated).

2. **CLI surface deferred.** Per spec §4 and Open Question 2's recommendation, this plan
   is **library-only**. No `sniff` subcommand routes through `identity()`, and
   `sniff repo git-status` is explicitly untouched.

### Proof strategy for "no status walk" (success criterion #1)

`cfg(test)` statics in a library are visible to that library's **unit** tests, and
**nextest** (the monorepo's default runner, per `just/devops.just`) runs each test in its
own process — so a `#[cfg(test)] static STATUS_WALK_COUNT: AtomicUsize` in `status.rs`,
incremented at the entry of all three status functions, gives a contamination-free
proof when asserted from a unit test in the git module. This is stronger than a
dirty-fixture `status: None` assertion (which the spec explicitly calls insufficient on
its own).

---

## Phase 1 — Make `GitInfo.status` optional (type migration, no behavior change)

**Goal:** `status: RepoStatus` → `status: Option<RepoStatus>`. Every existing preset
wraps its status in `Some(...)`. All current tests pass with no behavioral change.
This is the prerequisite for Phase 2: identity mode must be able to yield `None`, which
the current non-optional field cannot represent.

**Why first:** the identity level depends on an honest "no status" representation.
Doing the mechanical migration in isolation — with zero behavior change — produces a
clean review boundary and keeps the feature logic (Phase 2) uncluttered by churn.

- [x] Change the `GitInfo.status` field to `Option<RepoStatus>` and add
      `#[serde(skip_serializing_if = "Option::is_none")]` so identity JSON can omit it
      (`sniff/lib/src/filesystem/git/types.rs:987`). Keep the field non-`Option`-default
      at construction sites by wrapping in `Some` everywhere it is built.
- [x] Update the single production construction site — `detect_with_request`'s return
      literal — to emit `status: Some(status)` (`types.rs:928-943`). `detect_full`
      (`:777`) routes through this, so no separate change is needed there.
- [x] Update the `GitInfo` doc example that reads `info.status.is_dirty` to unwrap/match
      the optional, and explain that `None` means "status not requested," not "clean"
      (`types.rs:954-963`).
- [x] Migrate every CLI **reader/mutator** of `git.status` to handle `Option` explicitly.
      These are all status-oriented paths reached only via `full()`/`deep()` (which yield
      `Some`), so they may assume presence but must do so loudly:
      - `sniff/cli/src/commands/mod.rs:1133-1245` (package/branch/worktree filtering
        mutates `git.status.*`) — use `git.status.as_mut().expect("git-status command always computes status")` or an equivalent explicit match.
      - `sniff/cli/src/output/filesystem/repo.rs:706-781` (text status renderer).
      - `sniff/cli/src/output/filesystem/mod.rs:866` and the JSON renderers at `:1184-1264`.
      - `sniff/cli/src/output/filesystem/packages.rs:143-148`.
      - `sniff/cli/src/output/repo_json.rs` (renderer + fixtures at `:564`, `:804`, `:1033`).
      - `sniff/cli/src/bin/render_git_status_fixture.rs:57`.
- [x] Update lib + CLI **test fixtures and assertions** that build `GitInfo` or read
      `.status` directly: wrap construction in `Some(...)` (e.g.
      `cli/src/output/filesystem/mod.rs:1407`, `cli/src/output/repo_json.rs:564`,
      `cli/src/bin/render_git_status_fixture.rs:57`) and adjust assertions
      (`lib/tests/git_parity.rs` ~13 sites, `lib/tests/integration.rs:945-1030`) to
      `.as_ref().unwrap()` / `.expect(...)` or match.
- [x] **Validation checkpoint (Phase 1):**
      - `just build` (`cargo build -p sniff -p sniff-cli`) compiles clean.
      - `just test` green with **no behavior change** — existing presets still serialize
        a top-level `status` object.
      - `just lint` green.
      - `just doctest` green.
      - Golden spot-check: `cargo run -q -p sniff-cli -- repo git-status --json`
        includes a `"status"` object (proves existing JSON shape is preserved —
        success criterion #6).

> **Parallelizable:** the CLI reader updates are mechanically independent across files
> (`commands/mod.rs`, `output/filesystem/*`, `output/repo_json.rs`, fixture bin) and can
> be divided across implementers, but they must converge to a single compiling tree
> before the checkpoint. Treat it as one compile-until-green loop with parallel edits.

---

## Phase 2 — The `identity()` request level and status-walk proof

**Goal:** Add the status-free floor. `GitRequest::identity()` returns repo root, branch,
HEAD id, worktree flag, base repo root, and cheap `org`/`repo` — and provably never
walks the working tree. Depends on Phase 1's optional `status`.

- [x] Add the discriminator field `#[serde(default)] pub identity_only: bool` to
      `GitRequest` (`request.rs:279-303`). Add a builder setter is **not** required
      (presets drive it); document that manual toggling yields identity semantics.
      Verify `DetectionPlan` serialization roundtrip still passes (the `#[serde(default)]`
      keeps old payloads deserializing to `false`).
- [x] Add the `GitRequest::identity()` preset (`request.rs`, below `minimal()`). It sets
      every existing flag to the all-off floor **and** `identity_only: true`. It is the
      new floor below `minimal()`/`summary()`.
- [x] Add `pub fn is_identity_only(&self) -> bool { self.identity_only }`
      (`request.rs`). Confirm `is_minimal()` (`:409`) stays false for `identity()` — do
      **not** fold identity into minimal (spec §1). Confirm `wants_repo_metadata()`
      (`:427`) stays false for `identity()`.
- [x] Add the private `GitRepo::identity_only_info(&self, current_branch: Option<String>)`
      helper (`types.rs`, impl `GitRepo`). It fills `repo_root`, `current_branch`,
      `head_id` (`self.head_id()`), `in_worktree`, `base_repo_root`, `org`/`repo`
      (`self.org_and_repo()`), and leaves **all collections empty** with
      `status: None`. It must **not** touch `remotes`, branches, tracking, worktrees,
      config, or anything requiring network/ref-walks.
- [x] Add the identity early-return as the **first arm** of `detect_with_request`,
      immediately after the existing `try_current_branch()?` line (`types.rs:805`), and
      **before** the status block (`:824`):
      ```rust
      if request.is_identity_only() {
          return Ok(self.identity_only_info(current_branch));
      }
      ```
      This preserves the existing HEAD error policy: malformed/unreadable HEAD already
      errors at `try_current_branch()?`; detached/unborn HEAD yields `current_branch:
      None`; `head_id` is `None` for unborn HEAD.
- [x] Add the `#[cfg(test)] static STATUS_WALK_COUNT: AtomicUsize` instrumentation in
      `status.rs`, incremented at the entry of `is_repo_dirty`, the entry of
      `get_repo_status_with_changes`, and the entry of `get_repo_status_counts_detailed`.
      Add a `#[cfg(test)] pub(crate) fn reset_status_walk_counter()` / reader helper so
      unit tests can reset and read it. (No effect on non-test builds.)
- [x] **Unit test — status-walk proof** (in the git module's `#[cfg(test)]` block): on a
      dirty temp repo, reset the counter, call
      `repo.detect_with_request(&GitRequest::identity())`, assert
      `STATUS_WALK_COUNT.load(Ordering::SeqCst) == 0`, and assert the returned `GitInfo`
      has `status: None`. Then call `GitRequest::summary()` on the same repo and assert
      the counter incremented (proves the gate is real, not vacuous). Note the nextest
      per-process isolation assumption in a comment.
- [x] **Unit tests — identity field correctness across HEAD states** (success
      criterion #7), all using `git2`-init temp fixtures (already a dev-dep):
      - main worktree on a branch: `current_branch` set, `in_worktree == false`,
        `head_id.is_some()`, `status == None`.
      - linked worktree: `in_worktree == true`, `base_repo_root.is_some()`.
      - detached HEAD: `current_branch == None`, `head_id.is_some()`.
      - unborn HEAD (fresh repo, no commits): `current_branch == None`,
        `head_id == None`.
      All assertions use host-independent path checks; no network.
- [x] **Serde test — identity JSON omits `status`** (success criterion #6): serialize a
      `GitInfo` built from `identity()` and assert the JSON string contains no `"status"`
      key. Mirror test: a `summary()` `GitInfo` still serializes a `"status"` object.
- [x] Update the `request.rs` preset tests: assert `identity()` has `identity_only: true`,
      `is_identity_only() == true`, `is_minimal() == false`, `wants_repo_metadata() ==
      false`; and that the other four presets keep `is_identity_only() == false`.
- [x] **Validation checkpoint (Phase 2):**
      - `just test` green, including the new counter proof and HEAD-state tests.
      - The status-walk counter assertion is `0` for identity and `>0` for `summary()`.
      - `just lint` and `just doctest` green.
      - `sniff repo git-status --json` still includes `"status"` (unchanged).

> **Parallelizable:** the HEAD-state fixtures (main / linked-worktree / detached /
> unborn) are independent test bodies and can be written concurrently once
> `identity_only_info` exists.

---

## Phase 3 — Plan-level integration, end-to-end proof, and the secondary L9 dedup

**Goal:** Prove the motivating path — `DetectionPlan → FilesystemRequest →
GitRequest::identity()` — returns identity with no status walk end-to-end (success
criterion #2). Separately, opportunistically remove `detect_repo_identity`'s duplicate
discovery **only** if it stays additive (spec §3 / Open Question 1).

- [x] **Integration test — end-to-end identity plan** (success criterion #2): build a
      plan that expresses "git identity only, repo structure only, nothing else" —
      `DetectionPlan::new().without_os().without_hardware().without_network().filesystem(
      FilesystemRequest::new().git(GitRequest::identity()).repo(RepoRequest::structure())
      .without_file_inventory().without_formatting().without_docs())` — and run it via
      `detect_with_plan` against a dirty temp repo. Assert the returned `SniffResult`
      carries `git.status == None`, correct `repo_root`/`current_branch`, and
      `STATUS_WALK_COUNT == 0` across the whole plan. (Add a public test-only reset/read
      surface or reuse the Phase 2 helper; if the integration test lives outside the
      `sniff` lib crate, expose the counter behind a `test-instrumentation` feature
      enabled in `[dev-dependencies]` rather than `cfg(test)`.)
- [x] **Regression test — presets unchanged** (success criterion #4): assert
      `minimal()`, `summary()`, `full()`, `deep()` each yield `status.is_some()` and
      (for `summary`/`minimal`) `is_dirty` reflects a dirty fixture. This locks the
      no-behavior-change contract for existing presets.
- [x] **Optional / deferrable — L9 handle-sharing dedup** (spec §3): add an additive
      `detect_repo_identity_with_repo(&GitRepo) -> Result<RepoIdentity>` in
      `repo/identity.rs` that skips the internal `trusted_discover` by reusing the passed
      handle. Keep the public `detect_repo_identity(&Path)` signature intact (delegate to
      `GitRepo::discover` then call the new helper). **Skip this task** — and split it
      into its own change — if it forces any public signature churn beyond the new
      helper. Note in the commit/PR that `detect_repo_identity` is off the plan path
      (reached only by `sniff repo name`), so this is a secondary optimization that does
      not affect the motivating consumer.
- [x] **Validation checkpoint (Phase 3):**
      - End-to-end identity plan test green; counter is `0`.
      - Preset-regression test green (all four existing presets unchanged).
      - `just test` + `just lint` green.
      - If the L9 helper was added: `sniff repo name` still resolves correctly on a real
        repo; if skipped, document that one known duplicate discovery remains (per spec).

> **Parallelizable:** the end-to-end integration test and the preset-regression test are
> independent and can be authored together. The optional L9 helper is fully independent
> of both.

---

## Phase 4 — Documentation, skill cheat sheet, and cost re-measurement

**Goal:** Close out success criteria #3 and #5. Correct the docs that the feature makes
false, and record the before/after cost for the motivating claudine path. CLI surface
remains untouched (deferred per spec §4).

- [x] Update `sniff/lib/src/request.rs` module docs (`:1-21`) and the `GitRequest` field
      doc block to introduce `identity()` as the new floor below `minimal()`/`summary()`,
      and document `is_identity_only()` and the `identity_only` field.
- [x] Update `sniff/docs/sniff-library-architecture.md`:
      - GitRequest preset table (`:123-124`): add an `identity()` row and note it is the
        status-free floor.
      - "Git Status Layers" section (`:185-193`): add the identity path as a fourth,
        zero-walk arm, and **correct the now-false sentence** "every path runs a
        working-tree status walk" (`:193`) to state that `identity()` is the exception.
- [x] Update the sniff skill cheat sheet (`.opencode/skill/sniff/SKILL.md`) which
      currently says "**Every** preset … runs a working-tree status walk" — correct it to
      document `identity()` and its status-free contract (success criterion #5).
      (Both `.opencode/skill/sniff/SKILL.md` and `.claude/skills/sniff/SKILL.md` already
      carried the corrected wording from Phase 2; no further edits were required.)
- [x] **Cost re-measurement** (success criterion #3): confirm the exact claudine
      compose-prep call site (claudine lives in a **separate package area**; this plan
      touches only `sniff/lib`, never claudine). Measure identity vs. `summary()` latency
      on the rusty-biscuit tree (the audit cited ~40 ms for `summary()`). Record the
      before/after figures in the plan's completion notes or the PR description. Do **not**
      modify claudine as part of this change; if a follow-up is warranted, file it
      against the claudine area.

      Measured on the rusty-biscuit tree (current working directory) using the new
      `git_ops/request_levels` Criterion benchmarks on 2026-06-13:

      - `GitRequest::identity()`: **~415 µs** (median 414.85 µs, 95% CI 413.05–417.27 µs)
      - `GitRequest::summary()`: **~9.6 ms** (median 9.5865 ms, 95% CI 9.5278–9.6437 ms)

      The audit's previous ~40 ms `summary()` figure is now ~9.6 ms on this hardware,
      but the identity path is still roughly **23× faster** than `summary()` and avoids
      the working-tree status walk entirely.
- [x] **Final validation checkpoint (Phase 4 + whole feature):**
      - `just test` + `just lint` + `just doctest` all green for `sniff` and `sniff-cli`.
      - Grep the repo for stale claims: `rg "every (path|preset).*status walk"` returns
        only corrected/qualified statements.
      - `sniff repo git-status --json` still emits a top-level `status` object; an
        identity-only `GitInfo` JSON omits `status`.
      - All four existing presets remain behaviorally identical (Phase 3 regression test).

> **Parallelizable:** the three doc updates (`request.rs`, architecture doc, skill cheat
> sheet) are independent and can be edited concurrently. The cost re-measurement is
> independent of all of them.

---

## Out of Scope (carried verbatim from the spec)

- L7 preset-naming unification; L10/L1 caller-directed parallel runner; generalizing the
  `GitRepo` handle pattern; folding `programs`/`services`/`package` into the plan
  vocabulary; adding the missing `GitRequest` setters (L6) unless a planned change
  already edits those lines.
- Any CLI command routed through `identity()` (spec §4 — library-only for the first
  implementation).

## Risk Notes

- **Test-runner assumption.** The status-walk counter proof relies on nextest's
  per-process isolation (the monorepo default). If run under stock `cargo test`, parallel
  unit tests sharing one process could contaminate the counter; the counter test resets
  it at start, but the cleanest guarantee is `cargo nextest run` (what `just test` uses).
- **`identity_only` field is public.** A caller setting it on a non-identity preset
  yields identity semantics (early return). This is an acceptable, documented footgun and
  avoids fragile sentinel-reuse.
- **CLI reader migration (Phase 1) is the widest task.** It is pure mechanical
  `Option`-handling with no behavior change; isolate it in its own review boundary so the
  feature logic in Phase 2 stays focused.
