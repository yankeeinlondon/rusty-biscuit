---
agent: "claude/"
total_phases: 4
created: 2026-07-14
phase: 1
yolo: "true"
---

# Execution Plan — More Is More: Git Context and Conflict Prediction

Derived from [`spec.md`](./spec.md). This plan adds three Git-aware runtime
context variables (`ctx.branch`, `ctx.worktree`, `ctx.merge_conflicts`) and one
read-side expression function (`predict_conflicts(branch)`), with all Git
authority living in `sniff`.

## Success Criteria (goal-backward)

The plan is done when all 16 spec Acceptance Criteria hold, specifically:

- `sniff` exposes **one** hermetic commit-pair merge helper; `WorktreeEntry::has_conflicts`
  and the new public branch API both derive from it (no parallel merge algorithm).
- `ctx.branch` / `ctx.worktree` / `ctx.merge_conflicts` capture through a single
  demand-driven `ContextGroup::Git` with independent per-field degradation.
- `predict_conflicts(branch)` merges `theirs`→`ours` in memory, read-only, anchored
  on the caller/launch area, valid in body/frontmatter/`$()` surfaces.
- Authored YAML catalogs (base schema + expression-functions) are the single source;
  `context-variables.md` catalog block is regenerated, not hand-authored.
- `just test` passes in both `darkmatter` and `sniff`; cross-platform compile checks pass.

## Cross-Cutting Rules

- **US English** for all symbols/docs. **Never run `cargo fmt`.**
- **Never `git commit`** — implementation and tests only.
- Tests use **nextest**: `just test` (L1) / `just lint` per area. No L2 added.
- Run `impact` (GitNexus) before editing `has_conflicts`, `ContextGroup`,
  `ResolutionContext`, `BINDING_GROUPS`, and the capture dispatch — these have
  known blast radius (claudine exhaustive matches, worktree consumers).
- Portable path contract: sort/dedup once **in `sniff`** over the `/`-separated
  string form; Darkmatter never re-orders or re-encodes.

---

## Phase 1 — `sniff` Hermetic Merge Primitive + Public Branch API

**Goal:** Replace the boolean `has_merge_conflicts` probe with one hermetic,
path-producing commit-pair authority, and expose a public branch-oriented
conflict-path API. This is the foundation both Darkmatter surfaces consume.

Files: `sniff/lib/src/filesystem/git/{remote_refresh.rs, api.rs, status.rs,
worktree.rs, types.rs, mod.rs}`, `sniff/docs/`.

### 1.1 — Spike: gix hermetic-boundary feasibility (BLOCKING, do first)

- [ ] Determine whether the pinned `gix` `Repository::merge_commits` facade can enforce
      **all** hermetic invariants (object-memory before merge, index/attributes derived
      from captured `ours` tree, no live-index read, rename-aware options from safe merge
      config, `TreatAsUnresolved::git()`, `fail_on_conflict` disabled, reject external
      driver/filter). Inspect the current call at `remote_refresh.rs:845`.
- [ ] If the facade cannot enforce them, identify the lower-level `gix::merge` plumbing
      path (tree merge + virtual-merge-base) required. **Do not weaken an invariant to
      keep the convenience call.**
- [ ] Record the decision inline in the helper's module doc, and if a new direct
      `gix` plumbing surface/dependency is required, note it for the `docs/dependencies.md`
      update in Phase 4.
- **Checkpoint:** written decision on facade-vs-plumbing before writing 1.2.

### 1.2 — Hermetic commit-pair helper `merge_conflicts_between`

- [ ] Implement `merge_conflicts_between(repo, ours, theirs) -> Result<Vec<PathBuf>>`
      (private, in `remote_refresh.rs` or a focused sibling module) enforcing every
      invariant from spec §"Hermetic merge boundary":
      - clone/open into a probe-local view; enable `with_object_memory()` **before** merge;
      - derive temporary index + attributes from the captured `ours` commit tree only;
      - rename-aware options from the repo's safe merge configuration; `TreatAsUnresolved::git()`;
      - `fail_on_conflict` disabled (collect **all** unresolved paths);
      - never launch a merge driver / clean-smudge/process filter / hook; when an applicable
        external driver/filter or renormalization would be required, return a dedicated
        **unsupported-merge-configuration** error naming the setting/path.
- [ ] Materialize the merge into a temporary **in-memory** index, apply conflicts with
      `TreatAsUnresolved::git()`, collect paths whose stage is not `Unconflicted`
      (D7 temporary-index authority — **not** a heuristic pick of `ours`/`theirs` change locations).
- [ ] Convert byte paths via `sniff`'s existing lossy public-path helper; sort + dedup by
      the portable `/`-separated string form; return repo-relative `PathBuf`s.
- [ ] Add a dedicated `SniffError` variant (or reuse the existing Git error taxonomy) for
      the unsupported-merge-configuration case so it is distinguishable downstream.

### 1.3 — Migrate `WorktreeEntry::has_conflicts` onto the shared helper

- [ ] Run `impact({target: "has_merge_conflicts", direction: "upstream"})` and record blast radius.
- [ ] Replace `has_merge_conflicts` body so `has_conflicts == !merge_conflicts_between(...).is_empty()`.
      Delete the old `Options::default()` boolean merge path; there must be exactly one merge algorithm.
- [ ] Accept the intended rename-aware widening (update the two call sites at
      `remote_refresh.rs:~795` and `~1198`/`~1213`). Unsafe external merge config now
      **errors** rather than silently approximating.

### 1.4 — Public branch API `merge_conflicts_with_branch_at`

- [ ] Add `merge_conflicts_with_branch_at(path: &Path, incoming_branch: &str) -> Result<Vec<PathBuf>>`
      in `filesystem/git/api.rs`, re-exported from `filesystem::git` and `filesystem` per the
      existing facade pattern.
- [ ] Implement the §"In-memory merge algorithm" resolution:
      1. trusted `gix` discovery of the repo containing `path`;
      2. require attached current local branch → snapshot+peel tip as **ours**;
      3. normalize `incoming_branch` (strip at most one `refs/heads/`, validate as complete
         local-branch name, rebuild full ref, **exact** ref lookup — no rev-parse/DWIM/prefix/tag/SHA/remote-tracking)
         → snapshot+peel tip as **theirs**;
      4. delegate to `merge_conflicts_between(repo, ours, theirs)`.
- [ ] Map the §"Errors versus clean results" conditions to errors (outside repo, unborn/detached
      HEAD, unknown/invalid branch, unrelated histories, missing/corrupt objects, trust/permission
      failure, unsupported merge config). A clean merge returns `[]`; same-branch/ancestor/already-contained
      return `[]`.

### 1.5 — `sniff` L1 tests (build fixtures with `git2` dev-dep)

- [ ] git2 index oracle parity: clean, content, add/add, modify/delete.
- [ ] Fixed expected-set fixtures (+ canonical-`git` oracle when the executable is present,
      never invoked by the library): rename/rename, directory/file, multi-path, multiple-merge-base.
- [ ] Direction-sensitive fixture: reversing ours/theirs changes reported paths (AC9).
- [ ] Temporary-index authority proof: a heuristic "one side's change locations" collection
      **fails** at least one rename/multi-path fixture (AC8/D7).
- [ ] Unsupported external merge driver/filter fixture → dedicated error **before** any command
      launches; a safe built-in text merge remains supported.
- [ ] Live-index invariance: vary/corrupt the live index incl. staged `.gitattributes` while
      holding tips fixed → prediction unchanged (AC11).
- [ ] Read-only regression snapshots: HEAD, all refs, index bytes, worktree status/files, and the
      **on-disk object-ID set** unchanged before/after — including ≥1 clean auto-merge and ≥1
      criss-cross/multiple-merge-base fixture that **synthesizes** virtual-base objects (so the
      object-memory assertion is non-vacuous, AC7/AC13).
- [ ] Error-path tests: unborn/detached HEAD, unknown branch, invalid ref syntax, unrelated histories,
      non-repository, missing/corrupt object → errors, never `[]` (AC10).

- **Validation checkpoint P1:** `just test` and `just lint` green in `sniff/`;
  `merge_conflicts_between` is the sole merge algorithm; `has_conflicts` derives from it.

---

## Phase 2 — Darkmatter Git Context Variables *(parallelizable with Phase 3 after P1)*

**Goal:** Add `ctx.branch`, `ctx.worktree`, `ctx.merge_conflicts` behind a
demand-driven `ContextGroup::Git`, wired to authored catalogs.

Files: `darkmatter/lib/src/markdown/compose/context/capture/{groups.rs, mod.rs,
snapshot.rs, git.rs (new)}`, `.../context/catalog.rs`,
`darkmatter/docs/schemas/darkmatter.yaml`, `docs/topics/context-variables.md`.

### 2.1 — Base schema descriptors (authored authority — do first)

- [ ] Add to `darkmatter.yaml` `ctx` mapping, in declaration order (= catalog display order):
      - `branch` and `worktree` **with the repository identity fields** (near `repo`/`repo_root`),
        typed `string(generated)` (optional/nullable);
      - `merge_conflicts` **with the file-change fields** (near `dirty_files`), typed
        `string[](generated; required)`.
- [ ] Descriptions must match spec §"Context Variables" YAML snippets (short local branch;
      linked-worktree basename; unresolved index-stage paths).

### 2.2 — `ContextGroup::Git` + capture module

- [ ] Run `impact({target: "ContextGroup"})` — the `all()`/`for_key`/`group_for_key` and the
      `every_owned_key_has_exactly_one_group` test are the blast radius.
- [ ] Add `ContextGroup::Git` to the enum, `all()` array (now `[Self; 10]`), and the
      `group_for_key` table with a new `git::KEYS = &["branch", "worktree", "merge_conflicts"]`.
- [ ] Create `capture/git.rs` with `KEYS` and `populate_git(&ContextCapture, &mut Map)` projecting
      the three values (branch/worktree → `Value::String`/`Null`; merge_conflicts → `string_array`,
      `[]` when absent/clean).
- [ ] Register `mod git;` in `capture/mod.rs` and dispatch `git::populate_git` under
      `if groups.contains(&ContextGroup::Git)`.

### 2.3 — Shared single-discovery capture in `ContextCapture`

- [ ] Extend `ContextCapture` with Git-group fields (e.g. `git_branch: Option<String>`,
      `git_worktree: Option<String>`, `merge_conflicts: Vec<PathBuf>`) and a `need_git_group` gate.
- [ ] Perform **one** trusted repository discovery for the Git group (share the handle across
      the three probes via handle-oriented helpers behind the existing path facades — reuse
      `GitRepo`/`try_current_branch`, `get_current_worktree_name`, `merge_conflicts_at`). Do **not**
      re-discover per key, and do **not** trigger monorepo/status/docs/OS probes.
- [ ] Partial-runtime policy (D9): discovery failure → one
      `PartialRuntimeCapture { area: "git", .. }`, project `branch`/`worktree`=`null`,
      `merge_conflicts`=`[]`. After successful discovery, probe the three fields independently;
      a single field failure records a field-named diagnostic and substitutes only that field's
      neutral value, preserving siblings. Repository **absence** is an ordinary value state, not a diagnostic.

### 2.4 — Presentation grouping (`catalog.rs`)

- [ ] Add to `CONTEXT_VARIABLE_GROUPING`: `("branch", "Repository", "Git")`,
      `("worktree", "Repository", "Git")`, `("merge_conflicts", "File Changes", "Conflicts")`.
      (Capture grouping and presentation grouping are intentionally independent.)

### 2.5 — Regenerate documentation catalog

- [ ] Regenerate the marked catalog block in `docs/topics/context-variables.md` via the existing
      `md schema about --verbose` projection (do **not** hand-author). Update the capture-group table
      to describe the new demand-driven Git group.

### 2.6 — Darkmatter Git-context tests

- [ ] Demand-driven proof (AC2): referencing only one of the three keys performs no monorepo scan,
      full status walk, doc scan, hardware/OS probe, subprocess, or network — and shares one discovery
      across the three probes. Use the existing `status_walk_count`-style probes / group-scan tests.
- [ ] `ctx.branch` (AC3): short attached branch; `null` outside repo, unborn HEAD, detached HEAD.
- [ ] `ctx.worktree` (AC4): linked-worktree basename (canonicalized first); `null` in main/bare/non-repo;
      never substitutes branch name.
- [ ] `ctx.merge_conflicts` (AC5): sorted, deduped, portable repo-relative paths; `[]` for clean/absent;
      `[]` is falsy under `is_truthy`.
- [ ] Independent field degradation preserves sibling values (D9).

- **Validation checkpoint P2:** `just test` + `just lint` green in `darkmatter/`;
  a doc composing all three keys renders correctly; single-discovery proof passes.

---

## Phase 3 — Darkmatter `predict_conflicts(branch)` Expression Function *(parallelizable with Phase 2 after P1)*

**Goal:** Register the read-side Git function, catalog-authored and caller-anchored.

Files: `darkmatter/lib/src/markdown/compose/expression/functions/{mod.rs, git.rs (new)}`,
`.../expression/resolve_ctx.rs`,
`darkmatter/docs/schemas/expression-functions.yaml`.

### 3.1 — `ResolutionContext::caller_dir()` accessor

- [ ] Run `impact({target: "ResolutionContext"})`.
- [ ] Add `pub(crate) fn caller_dir(&self) -> &Path` returning `file_ref_fallback_dir` when present,
      else `base_dir` (spec §"Caller repository anchor"). The Git handler calls this rather than
      reading the field directly.

### 3.2 — Authored catalog entry (single authority — do before runtime)

- [ ] Add the `predict_conflicts` descriptor to `expression-functions.yaml` exactly as spec
      §"Catalog entry": new **`Git`** category, **`order: 90`** (88/89 reserved by `finding-indexes`),
      one overload `branch: string -> string[] | error`, `example` with `verification: display-only`.

### 3.3 — `functions/git.rs` domain slice + registration

- [ ] Create `functions/git.rs` with
      `FunctionBinding { canonical: "predict_conflicts", aliases: &["predictconflicts"],
      evaluation: Context, handler: Some(Context(predict_conflicts_fn)) }`.
- [ ] Add `mod git;` and `git::BINDINGS` to `BINDING_GROUPS` in `functions/mod.rs`.
- [ ] Implement `predict_conflicts_fn(&[Value], &ResolutionContext) -> Result<Value, ExpressionError>`:
      - exactly one argument; `null` → `null` (language-wide null propagation); non-string/non-null → type error;
      - empty/whitespace-only → error; no trimming/rewrite except the one `refs/heads/` prefix handled in `sniff`;
      - obtain `caller_dir()`, call `sniff::filesystem::git::merge_conflicts_with_branch_at`;
      - project the already-sorted portable paths to `Value::Array` with **no** second ordering/encoding;
      - convert `sniff` failures into the expression error model **without losing** the branch name or repo anchor.
- [ ] Confirm validity across body interpolation, frontmatter interpolation, and `$()` ternary surfaces
      (no remote runtime, no network I/O).

### 3.4 — `predict_conflicts` tests

- [ ] Registry parity: `catalog_and_runtime_bindings_have_bidirectional_canonical_parity` and alias
      uniqueness still pass with the new binding (AC14).
- [ ] Direction (AC9/D2): `theirs`→`ours`; a fixture where reversing changes reported paths.
- [ ] Clean/no-op returns `[]`: same-branch, ancestor, already-contained, clean divergent (AC10).
- [ ] Error propagation (AC10): unknown branch, non-repository, detached/unborn HEAD, invalid ref
      syntax, unrelated history, missing/corrupt object, unsupported external merge config, Git access
      failure → expression errors, never `[]`.
- [ ] Committed-state boundary (AC11): staged/unstaged/untracked/already-conflicted index (incl. staged
      `.gitattributes`) ignored; missing/corrupt live index does not block; tips held fixed.
- [ ] Caller anchor (AC12): repo-A prompt invoked from repo-B evaluates against repo-B; consistent across
      body/frontmatter/`$()`.
- [ ] Read-only (AC13): snapshot HEAD/refs/index/worktree/object-ID set unchanged; no subprocess/hook/
      driver/filter/fetch invoked.

- **Validation checkpoint P3:** `just test` + `just lint` green in `darkmatter/`;
  registry parity + read-only snapshots pass.

---

## Phase 4 — DMLS Parity, Documentation, Single-Sourcing & Closure

**Goal:** Prove editor intelligence reaches DMLS through the shared catalogs (no Git at edit time),
finish single-sourced docs, and verify cross-platform.

Files: `darkmatter/dmls/` (parity tests), `darkmatter/docs/topics/darkmatter-expressions.md`,
`sniff` API/Git docs, `docs/dependencies.md` (only if 1.1 added a dep), and the
`sniff` / `rust-devops` / `darkmatter` skills.

### 4.1 — DMLS catalog-parity tests (no hard-coded name lists)

- [ ] Assert the three `ctx.*` descriptors (with exact nullable/array types) reach DMLS completion/hover
      through the schema-derived catalog.
- [ ] Assert the `predict_conflicts` descriptor reaches DMLS completion/signature/hover through the
      expression-function catalog.
- [ ] Assert DMLS never evaluates the function, discovers a repo, reads the index, or simulates a merge
      while serving requests (extend the existing `no_side_effects` posture).

### 4.2 — Documentation & single-sourcing

- [ ] `darkmatter-expressions.md`: add Git function semantics, committed-state boundary, direction,
      and the `as_unordered_list(predict_conflicts(...))` / ternary examples.
- [ ] `sniff` public API docs + Git docs: document actual (`merge_conflicts_at`) vs predicted
      (`merge_conflicts_with_branch_at`) conflict APIs and the hermetic contract.
- [ ] Update `sniff`, `rust-devops`, and `darkmatter` skills where public architecture/workflow
      descriptions need the new surface.
- [ ] Only if 1.1 required a new direct plumbing dependency: update `docs/dependencies.md` and the
      affected area document; otherwise assert no dependency-doc change is needed.
- [ ] Confirm `context-variables.md` (2.5) and expression docs contain **no** second hand-maintained catalog.

### 4.3 — Full validation & closure (AC16)

- [ ] `just build` + `just test` + `just lint` green in **both** `darkmatter/`
      and `sniff/`, plus any downstream package areas selected by impact
      analysis (including Claudine or Worktree for exhaustive consumers).
- [ ] Cross-platform compile checks for macOS, Windows, Linux (`--target` checks where the host allows;
      otherwise assert no OS-specific/`cfg` code was introduced — every Git op is pure-Rust `gix`).
- [ ] Run `detect_changes({scope: "compare", base_ref: "main"})` and confirm only expected symbols/flows changed.
- [ ] Walk all 16 Acceptance Criteria and check each off against a concrete test or artifact.

- **Validation checkpoint P4 (final):** every spec Acceptance Criterion maps to a passing
  test/artifact; both package areas green; workspace compiles cross-platform.

---

## Dependency & Parallelization Summary

```
Phase 1 (sniff foundation)
        │
        ├────────────► Phase 2 (ctx.* Git variables)   ┐
        │                                                ├─ parallel after P1
        └────────────► Phase 3 (predict_conflicts fn)   ┘
                                 │
                                 ▼
                        Phase 4 (DMLS parity + docs + closure)
```

- **Phase 1 is a hard prerequisite** for Phases 2 and 3 (both call the new `sniff` API).
- **Phases 2 and 3 are independent** and may run concurrently once Phase 1's public
  `merge_conflicts_with_branch_at` signature is landed. They touch disjoint Darkmatter
  modules (capture vs expression/functions) except for the shared authored YAML files —
  coordinate the two YAML edits (`darkmatter.yaml` in P2, `expression-functions.yaml` in P3)
  as separate files to avoid conflict.
- **Phase 4 requires both** Phase 2 and Phase 3 complete (parity tests reference both catalogs).
- Within Phase 1, **task 1.1 (spike) is blocking** — it decides facade-vs-plumbing before 1.2.

## Risk Notes

- **gix facade limitation (1.1):** highest-uncertainty item. If `merge_commits` cannot enforce
  the hermetic invariants, dropping to `gix::merge` tree plumbing enlarges Phase 1 scope and may
  add a dependency (handle in 4.2). Resolve before committing to 1.2's shape.
- **Rename-aware widening (1.3):** `has_conflicts` answers may change vs the old `Options::default()`
  probe. This is an intended correctness fix — verify existing `remote_refresh` worktree tests are
  updated to the widened expectation, not silently broken.
- **New enum variant drift (2.2):** `ContextGroup::Git` and any new `SniffError`
  variant can break exhaustive matches downstream (Claudine). The scoped
  downstream build/test/lint matrix in 4.3 catches this.
