---
total_phases: 4
created: 2026-10-18
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/detailed_resolution.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/searched_tests.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_1:
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_2:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
docs_updated_during_phase_3:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter
    - biscuit-file
    - dmls
---

# Plan: DMLS cache optimization follow-ups

## Summary and definition of done

A trigger payload that resolves through an ordered list of candidate locations
(for example `Fields@shared/types.yaml`: authoring folder, then repository
root) can go stale when a *nearer* candidate that was previously absent is
created. Two independent reuse decisions both miss it today:

1. **Library**: `file_payload_memo::inputs_fingerprint`
   (`lib/src/markdown/schemas/triggers/assemble.rs`) hashes dependency bytes,
   schema-root listings, and the parents of *selected* dependencies only.
2. **DMLS**: `Dependencies` / `scope_affected` / `watch_folders`
   (`dmls/src/overlay/registry.rs`) and `schema_watch_registration`
   (`dmls/src/workspace/watch.rs`) track the same narrow set, so a ready
   registry never learns of the new file and nothing watches the location.

The work is: record every candidate location actually probed during
resolution (library), check them before a memo hit (library), then feed them
to editor invalidation and watching (`dmls`), and document it.

**Done when** (spec acceptance 1-6):

- [x] Concrete example (`schemas/payload.yaml` importing `Fields@shared/types.yaml`):
      unchanged repeat is a hit returning the same shared schema; creating
      `schemas/shared/types.yaml` forces a miss and selects the new fields;
      deleting it falls back.
- [x] Same precedence change covered for payload path, transitive import,
      example file; bare-name root-shadowing test kept as control; explicit
      `./` / `../` references gain no fallback.
- [x] A ready DMLS registry detects the new file through a delivered watch
      event (no test-only invalidation), including removal, save notification,
      and fallback-mode rescan; change replay during an in-flight preparation
      prevents a stale install; a malformed nearer file follows last-good
      policy and recovers after repair.
- [x] Unchanged/unrelated inputs keep hits; ready lookups do no reads, hashes,
      or payload resolution; existing isolation, capacity (32), LRU, and
      embedded tests stay green.
- [ ] `just test` and `just lint` pass in `darkmatter/`; docs, skill, and
      comments updated. Terminal state: *implementation complete, ready for
      review* (the author, not the agent, moves the fix to `_completed`).

No performance spike is scheduled: the spec states none is required and the
added observations are bounded by the candidates resolution already considered.
No Input Robustness Matrix: this work adds no file-format or configuration
reader (it adds filesystem *observations*), so that section does not apply.

## Phase 1: Record search dependencies in the library resolver

Goal: `ResolvedSchema` → `ResolvedPayload` → `TriggerRegistry` carry the
lexical, sorted, deduplicated list of candidate locations attempted, separate
from the content-hashed `dependencies` list.

### Necessary Rules

Rulings the implementer must treat as decided (author may override):

- **R1 Recursive `%` references are deferred** (spec's recommended option).
  Recursive references record nothing new (their `SearchRoot` dispositions are
  not treated as file probes). `docs/lsp/indexing.md` states the limitation
  explicitly (Phase 4). Do not claim recursive shadowing is solved.
- **R2 Representation.** Add one new field to `ResolvedSchema`
  (`searched: Vec<PathBuf>`, lexical, sorted, deduplicated), mirrored on
  `ResolvedPayload` and exposed from `TriggerRegistry` as an iterator beside
  `dependencies()`. The existing `dependencies` list is unchanged: its
  consumers read and hash contents, and absent paths would break them.
- **R3 What is recorded.** Every candidate path probed in order up to and
  including the selected one (an earlier absent candidate, and the selected
  file itself so a file→directory swap is seen). Absent paths are stored
  lexically (via `biscuit-file` path identity, no canonicalization); existing
  dependency files keep today's simplified canonical form.
- **R4 Failure semantics unchanged.** A probe error that stops resolution today
  still returns `Err`; nothing is recorded for, or cached from, a failed
  resolution. Recovery in DMLS (Phase 3) therefore must not depend on the
  failed resolution's output (see R7).
- **R5 One candidate algorithm.** Obtain candidates from `FileReference`
  (`resolve_detailed(...).candidates()` or `candidate_plan`); do not write a
  second parser/ordering. The bare-name schema-root loop in
  `try_bare_name_in_roots` (explicit `./name` per root) records each attempted
  root's `root/name` itself.
- **R6 Reuse inside a resolution.** `namespace_cache` (`CachedNamespace`,
  `resolve.rs` ~1182) returns imports without re-probing, so each cache entry
  must store the searched set produced when it was first resolved and re-merge
  it on every hit. `embedded_cache` contributes nothing (no disk).
- **R7 (DMLS, decided here so Phase 3 can rely on it).** When a preparation
  fails and last-good is retained, the new scope keeps the previous scope's
  search/watch dependencies (union with what the failed scan could record),
  because the failing nearer file is itself a previously searched candidate.
- **R8 Watching an absent candidate parent** registers the nearest existing
  ancestor as base with patterns for both the candidate folder itself and its
  children, so a folder-plus-file creation and a bare directory event are both
  seen (a client may report only the directory). A directory-path event is
  treated as affecting any scope whose searched candidate lies at or below it.

Spikes: none (spec's figures answer the cost question; see above).

### Wave 1.1 — resolver instrumentation (sequential; one file, `resolve.rs`)

- [x] **Search field** — add `searched` to `ResolvedSchema` and all
  constructors (~lines 725, 979, 1076, 1097) with doc comments distinguishing it
  from `imports`/`examples`/`referenced_files`.
- [x] **Single recording choke point** — route
  `resolve_file_reference_in_context` through `resolve_detailed` on the
  `for_cwd(base_dir)` context and return both the result and the candidate
  paths; keep error/NoMatch mapping identical. Record in
  `try_bare_name_in_roots` per root.
- [x] **Thread to all four sites** — `resolve_reference_in_context`
  (top-level + delegated), `named_type_file_path` (imports),
  `resolve_example_path` (examples), root-union arms. Merge nested results
  where `referenced_files` is merged today (`accumulate_referenced_file`
  analogue, ~517/562/611) and in the root-level collection (~317, ~406).
- [x] **Cache-carry (R6)** — store searched set in `CachedNamespace`; re-merge
  on hit.
- [x] **Finalize** — sort/dedup after resolution only, never reordering
  evaluation.

### Wave 1.2 — propagation (after 1.1; parallel with 1.3)

- [x] **Payload propagation** — in `resolve_file_payload` (`assemble.rs` ~265)
  copy `resolved.searched` into `ResolvedPayload.searched` (embedded payload at
  ~653: empty).
- [x] **Registry accessor** — `TriggerRegistry::searched_locations()` in
  `discovery.rs` next to `dependencies()`; embedded triggers contribute none.

### Wave 1.3 — resolver tests (after 1.1; parallel with 1.2)

- [x] Unit tests in `resolve.rs`: concrete example records
  `schemas/shared/types.yaml` (absent) and `shared/types.yaml` (selected);
  explicit `./`/`../` records a single location; bare-name records each root
  tried; nested delegation/import/example merge; second import of the same file
  (namespace-cache hit) still reports its searched set; failed probe records
  nothing and still errors.

### Checkpoint 1

- [x] `just test` and `just lint` in `darkmatter/` pass; no change to existing
  `dependencies` assertions in `payload_memo_tests.rs`.

## Phase 2: Check searched locations before a payload-cache hit

Goal: `file_payload_memo` misses when any recorded candidate changes.

### Wave 2.1 — fingerprint (sequential)

- [x] **Extend `inputs_fingerprint`** (`assemble.rs` ~430) with the searched
  candidates and each candidate's parent-folder observation: sorted, deduplicated
  folders; entry names for readable folders; a distinct marker for a missing
  folder versus an empty readable one; a per-candidate kind observation
  (absent / regular file / directory / other) so a swap that leaves entry names
  unchanged still misses. Keep `biscuit-hash`, do the I/O outside the mutex.
- [x] **Failed observation never validates a hit** — an unreadable directory
  (other than expected "not found") forces a normal re-resolve so typed errors
  surface; avoid permission-bit tests (use existing seams).
- [x] **Store searched with the entry** — `Entry` fingerprint computed from
  `resolved.searched`; shared `Arc` identity on unchanged hits preserved;
  capacity 32/LRU/key unchanged.
- [x] **Comments** — replace the claim that root and selected-folder listings
  cover every searched directory with the actual coverage.

### Wave 2.2 — library tests (after 2.1; extend `payload_memo_tests.rs`; tests parallelizable by author)

- [x] Acceptance 1: concrete example (hit, same object, create → miss + new
  fields + new dependency, delete → fallback).
- [x] Acceptance 2: payload path, transitive import, example variants; keep the
  existing bare-name test as control; explicit relative refs get no fallback.
- [x] Acceptance 3: existing candidate parent; directory→regular file swap with
  same entry name; failed observation is not a hit.
- [x] Acceptance 6 (library half): unrelated edit keeps hit; embedded,
  context-isolation, capacity and eviction tests green. Use tally counters, no
  sleeps or global cache clears.

### Checkpoint 2

- [x] `just test` and `just lint` in `darkmatter/` pass. The library regression
  is fixed on its own; DMLS still stale until Phase 3.

## Phase 3: Editor invalidation and watching (`dmls`)

Goal: a ready registry notices the new location through a watch event and
re-prepares; watch coverage follows candidates.

### Wave 3.1 — dependencies and invalidation (`registry.rs`) ∥ watch registration (`watch.rs`)

These touch separate files and can be implemented concurrently by two
subagents against an agreed interface: `Dependencies.searched: Vec<PathBuf>`
plus `watch_folders()` returning a richer type (folder + "absent" marker) or
a second set of absent-candidate folders.

- [x] **Record searched in `Preparation::run`** — populate `Dependencies` from
  `registry.searched_locations()` (lexical, normalized like `files`), no extra
  reads.
- [x] **`scope_affected`** — also affected when the event path equals a searched
  candidate, equals/contains-below a searched candidate's ancestor (directory
  events, R8), or its parent is a searched candidate's parent. Preserve the
  byte comparison for known dependency files; unrelated file events must not
  invalidate every registry.
- [x] **Failed preparation (R7)** — on error, union the previous scope's
  searched/folders/absent into the retained scope's `Dependencies`; verify
  last-good policy and the root-failure rule (never serve last-good on root
  failure) are untouched.
- [x] **`watch_folders` / registration** — include searched candidate parents;
  for absent parents register the nearest existing ancestor base with patterns
  for the folder and its children (R8) in `schema_watch_registration`; bounded
  by recorded paths only, including external roots.
- [x] **Coverage advance** — after the event, the re-preparation recomputes
  watch folders; confirm `router.rs` (~1652) re-registers when the set changes.

### Wave 3.2 — replay and fallback (after 3.1)

- [x] Confirm `Change` replay at `install` applies the new rules (event during
  in-flight preparation leaves result stale or triggers follow-up).
- [x] Fallback watch mode: server rescan rebuilds from the new candidate
  state; a supported save notification for a candidate invalidates it. No
  promise for external changes without client events.

### Wave 3.3 — protocol/L1 tests (after 3.1/3.2; parallel authors OK)

Under `dmls/tests/l1` using the existing preparation test gate and work
counters.

- [x] Acceptance 4: ready registry → create nearer folder+file → deliver
  watched event → watch coverage asserted and schema behavior changes, no
  explicit invalidation; removal; save notification; fallback rescan.
- [x] Acceptance 5: gate held, creation event delivered, replay prevents a
  stale response; malformed nearer file → last-good/error, recovers after
  repair (exercises R7).
- [x] Acceptance 6 (editor half): unrelated file events do not invalidate;
  ready lookups perform no reads/hashes/payload resolution.
- [x] Watch unit tests in `watch.rs`: registration for an absent nested
  candidate parent (nearest existing ancestor, folder + children patterns).

### Checkpoint 3

- [x] `just test` and `just lint` in `darkmatter/` pass; verify tests use
  portable temp paths (path separators, case, `\\?\` simplification) so they
  compile on Windows/WSL2 — load the `os` skill before touching path
  comparisons.

## Phase 4: Documentation, skill, comments, final verification

Goal: current-state docs match behavior; no drift.

- [ ] **`docs/lsp/indexing.md`** — update payload-sharing and registry-change
  sections: covered search locations, absent-folder handling and watch
  advance, fallback-mode limits, and the explicit recursive `%` limitation
  (R1). Developer-audience wording with a compact example and a Mermaid diagram
  of probe → fingerprint → watch → re-prepare. No link to this fix or its spec.
- [ ] **Skill** — update `.claude/skills/darkmatter/` (schema.md/dmls.md) if
  dependency propagation guidance changes (mention the separate searched list).
- [ ] **Comment sweep** — `ResolvedSchema`, `ResolvedPayload`, `file_payload_memo`
  doc, `Dependencies`, module header at `registry.rs` ~39, watch docs; fix
  drifted claims (treat code as correct) and note any drift found.
- [ ] **Final gates** — `just test`, `just lint` in `darkmatter/`; check
  consumers of changed `pub` items (`ResolvedSchema` field) with
  Sniff/GitNexus (Claudine) and fix any struct-literal breakages.
- [ ] Update the implementation log; leave the fix in place (no move to
  `_completed`, no `just complete`); do not commit unless told.

### Checkpoint 4

- [ ] All "Done when" boxes above checked; report "implementation complete,
  ready for review".

## Open items for the author

- Spec frontmatter is `finalized-spec` and the unscheduled-status open
  question in the spec is moot for the plan; confirm R1 (defer recursive) and
  the R7/R8 rulings before Phase 3 begins.
