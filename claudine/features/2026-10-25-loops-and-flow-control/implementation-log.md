---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-25-loops-and-flow-control/spec.md
plan: claudine/features/2026-10-25-loops-and-flow-control/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/features/2026-10-25-loops-and-flow-control/spike/loop_flow_spike.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/dependency-audit.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/path-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/coverage-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/matrix-contracts.md
    - claudine/features/2026-10-25-loops-and-flow-control/spike/README.md
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
human_review: true
human_review_items:
    - |-
      **Should `loop.on_rate_limit` stay, and where?** (decide before Phase 2)

      Today a looping *document* can say what to do when the AI provider reports it is being rate-limited: `pause` until the limit resets (the default), `abort` with exit code 75, or `continue`. The `--on-rate-limit` flag sets the same thing. This feature lets groups and sequences loop too, but the new loop grammar in the spec never mentions this key. Phase 2 rewrites the loop parser, so it has to know whether to keep the key.

      - **Option A: keep it for documents only; reject it on group and sequence loops.** *Recommended.*
        - Pros: nothing that works today breaks. The key only means something where a provider stream exists, and a group or sequence has none. A container that names the key gets a clear error instead of a setting that silently does nothing.
        - Cons: the loop grammar is slightly different for documents than for containers.
        - Detail: the pause is applied only after the loop has decided to run another pass, which replaces today's guess about the last pass.
      - **Option B: keep it everywhere; on a container, pass it to the documents inside.**
        - Pros: the grammar is the same everywhere.
        - Cons: this is new behavior the spec does not describe. It is unclear which document's rate limit should pause a whole group.
      - **Option C: remove it.**
        - Pros: the simplest grammar.
        - Cons: removes working rate-limit recovery. Its tests and its documentation in five pages would go too. No shipped prompt uses it, so nothing in the repository breaks.

      I recommend **A**, because it keeps current behavior and needs no new behavior to be designed.
    - |-
      **Should `file_exists()` report errors instead of answering `false`?** (decide before Phase 2's path work)

      `file_exists(path)` is a Darkmatter expression function. Today it answers `false` whenever it cannot look. That includes a permission-denied folder, a malformed path, a missing home or vault setting, or a failed download. The spec says such errors must never be turned into "the file is missing". The function's own documentation, however, promises it "never errors", and three existing tests check that.

      - **Option A: errors become errors; a genuinely absent file is still `false`.** *Recommended.*
        - Pros: it matches the spec. A permissions problem can no longer make a loop guard or the review loop's "review file already exists?" check give the wrong answer silently.
        - Cons: it changes a documented contract. Three tests and the docs change, and any prompt that relied on `false` for a bad path now fails loudly.
      - **Option B: keep `false` for everything.**
        - Pros: no change.
        - Cons: it contradicts the spec, and a permission error can still look like "missing".

      I recommend **A**, because the review loop depends on this check to refuse a stale or pre-existing review file.
    - |-
      **The review loop: one agent or two?** (decide before Phase 5; it does not block Phases 2–4)

      The live `prompts/review-loop.md` takes two inputs, `review_agent` and `repair_agent`, so reviews and repairs can use different AI agents. The spec's rewritten version takes a single `agent` for every step.

      - **Option A: follow the spec (one `agent`).**
        - Pros: simpler, and it matches the finalized spec.
        - Cons: you can no longer use, say, one agent to review and another to repair.
      - **Option B: keep both inputs in the migrated prompt.** *Recommended.*
        - Pros: keeps a capability that exists today, and the change is small.
        - Cons: a departure from the spec's example, which would be recorded in the implementation log.

      I recommend **B**, because it keeps the separate review and repair agents that exist today. If you prefer the spec's single input, choose A.
message_to_agent: |-
  Phase 1 delivered inventories, rulings, a spike, and a contract matrix; read them before coding. They are in `claudine/features/2026-10-25-loops-and-flow-control/phase-1/` and `spike/README.md`:
  - `dependency-audit.md` §3 (seams), §6 (every loose loop verb's signature), and §7 (owner per shared file).
  - `path-inventory.md` §5–6 (path fixture table and the exact change sites for `absolute`/`dirname`).
  - `coverage-inventory.md` §2 (test registration: `autotests = false`, add `mod` lines) and §6 (acceptance criterion → test → task).
  - `matrix-contracts.md`: every robustness cell, marked covered, Phase N, or N/A.

  Key facts:
  1. The worktree carries **uncommitted** sequence-containers review fixes (about 20 claudine files and 3 untracked L1 tests). Build on them; never stash or reset them.
  2. `on_rate_limit` is awaiting the author's ruling (human review). Preserve its current behavior until it is decided.
  3. `claudine/cli/tests/l1/loop_config_matrix.rs` is the YAML-frontmatter matrix traversal: a `matrix!` macro with one test per cell, each a one-edit change to the `CONTROL` fixture. When Phase 2 lands a change, add its pending cells from `matrix-contracts.md` as rows. The largest group is 11 `LoopInvalid` errors that do not name the document; the check requires a rejection's stderr to contain `count.md`. Write the other formats' traversals (standalone YAML, JSON, JSON5, JSONL/NDJSON) the same way once container and task grammar exist. JSON-family duplicate keys are last-wins today and must become errors.
  4. Spike contracts:
     - A break settles a container with `guard_mut().defuse()` and then one `finalize`.
     - `prep` is a returning action through an injected runner beside `shell_runner`, not a `StackControl`.
     - `RuntimeState` needs a pass-checkpoint restore of mutations and outputs that keeps `proxy_chain`.
     - Call ancestry reuses `enter_container_proxy`'s RAII guard.
     - No deadline carrier exists; add an absolute-instant `CallDeadline`.
     - `LoopAmbient.last_output` must become a `Value`.
  5. `launch_recorder` is Unix-only, so prompt-recording tests are `cfg(unix)`.
  6. Stale comment for the executor owner: `StackExecutionContext::scope` (`lifecycle/executor.rs:579`) still mentions task setup/teardown.
---

# Implementation Log for 2026-10-25-loops-and-flow-control (6 phases)

## Phase 1

Phase 1 started 2026-10-16. All four dependency specs report
`implemented: true` (2026-10-20-lifecycle-grammar,
2026-10-21-claudine-schemas, 2026-10-23-shell-template-approval,
2026-10-24-sequence-containers); 2026-10-26-task-inputs-and-providers is not
implemented, so task inputs stay `params:`.

### Wave 1: inventories

Three read-only inventories ran in parallel. Their reports are the Phase 1
deliverables, and later phases read them:

- `phase-1/dependency-audit.md`: delivered behavior with file:line evidence,
  the seams, the cause of the looping-document defect, the `on_rate_limit`
  inventory, every loose loop verb's signature, and a proposed owner for each
  shared file (§7).
- `phase-1/path-inventory.md`: every Darkmatter path function's return type,
  missing-target and URL behavior, and errors; the Phase 2 path fixture table
  (§5); and the smallest change sites (§6).
- `phase-1/coverage-inventory.md`: existing tests, test registration rules,
  where the prompt tests live, how route drift derives its hash, which formats
  each entry point accepts, and the acceptance-criterion → test → task map
  (§6).

**Dependency gate: PASS.** Lifecycle grammar (node tree, last-item rule,
planned `break`/`prep` in the schemas and the `PLANNED` parity list), shell
template approval (exact versus template, `ShellPreApprovals` on
`PreflightGraph`), and sequence containers (`ContainerRunner`, `cancelled`,
group `fail_fast`, `seq`/`group`/`loop` roots) are all in code. Task
`setup:`/`teardown:` is rejected grammar; only dead plumbing remains.
`break` and `prep` are parsed nowhere. The things the plan says this feature
removes are still present, as expected: loop `fail_fast`, predictive
`is_last` (`looping/engine.rs:284-306`), and the action applied after the
last pass (`engine.rs:496-522`).

Baseline caveat: the sequence-containers review fixes are uncommitted in this
worktree (20 modified claudine files and 3 untracked L1 tests). Phase 2 must
start from that tree and must not stash or reset it.

The defect in spec §3 is a routing gap. The loop engine is reached only from
`prepare_and_run_active_document` → `execute_loop_or_single`. A `prompt:`
task goes `WrapperPromptRunner::run` (`cli/wrap/sequence/task_run.rs:620`) →
`execute_composition_request_inner`, which runs a single attempt and never
reads `loop:`. A document reached by `proxy` inside a step does loop. The
shipped `review-loop.md` → `implement-plan.md` route hits the defect today.

### Compatibility ruling: `on_rate_limit` (plan rule 8)

Inventory: `loop.on_rate_limit` (`pause`/`abort`/`continue`, default
`pause`) and `--on-rate-limit` act only in the document loop, between
iterations, when the provider stream reports throttling. `abort` exits 75.
No shipped prompt or frozen fixture uses it. A container has no provider
stream, so the key cannot mean anything on a group or sequence loop.

The author has not ruled; this session is non-interactive. Recommendation,
recorded as a human review item: keep it for documents only, reject it with a
typed error on container `loop:` blocks, and apply the policy only after the
condition and cap decide another pass will run. That replaces today's
predicted-last-pass skip. **Until the author rules, current behavior is
preserved**, and Phase 2 must not remove or extend the key.

### Other rulings and findings

- **Empty collections** (plan rule 9): `gate: []` and `action: []` are
  explicit no-ops. `gate: null`, `gate: {}`, `action: ""`, and `loop: {}`
  are errors. `prep.with: {}` is an explicit empty overlay; `with: null` is
  an error, as for `proxy.with`. Today `gate: null` and `gate: {}` parse as an
  empty gate, so Phase 2 tightens them.
- **Duplicate keys**: YAML frontmatter and standalone YAML already reject
  them. JSON, JSON5, JSONL, and NDJSON are last-wins today. Phase 2 adds a
  duplicate-detecting read for executable task sources, before the
  `serde_json::Value` conversion.
- **Unclosed frontmatter fence** reads as empty frontmatter (Darkmatter). It
  is kept: it is Markdown grammar, which this feature does not change.
- **`implemented` guard**: characterized with `md compose`
  (`phase-1/matrix-contracts.md`, review metadata). A quoted `'true'` counts
  as implemented under `!= true`. The spec's semantics are kept.
- **`review_iterations` presence**: `frontmatter()` returns null for absent
  and for explicit null. The migrated prompt uses the existing
  `has_key(frontmatter(spec), 'review_iterations')` to tell them apart; no
  new read is needed.
- **`file_exists` swallows errors**: permission probe errors, invalid
  references, missing home or vault, and failed fetches all read `false`
  (`darkmatter/.../functions/mod.rs:1619-1622`). Spec §10 says such errors
  never become missing-file success. Recorded as a human review item for
  Phase 2.
- **`dirname` expectation changes**: preserving the absolute anchor reverses
  outputs pinned by `darkmatter/lib/src/markdown/compose/tests/schema.rs:565,
  795, 874`, `claudine/cli/tests/l1/compose_caller_file_provenance.rs:86`, and
  possibly `darkmatter/cli/tests/l1/compose_schema_file_rewrite.rs:146`.
  The spec requires the change; Phase 2 updates these on purpose and logs it.
- **`review-loop.md` agents**: the live prompt takes `review_agent` and
  `repair_agent`; the spec's version takes one `agent`. Recorded for the
  author before Phase 5.
- **`commit.md`** already ends `initialize` with `skip`, so spec §9's edit
  has landed. Only its acceptance test (AC12) remains.
- **Windows recorder gap**: `launch_recorder` is Unix-only, and no portable
  prompt recorder exists. New prompt-recording CLI tests are `cfg(unix)`, as
  existing loop tests are, until a portable recorder lands.
- **Stale comment noticed, left as is** (no behavior change in this phase):
  `StackExecutionContext::scope` (`lifecycle/executor.rs:579`) still
  mentions task `setup:`/`teardown:` stacks. The owner of that file fixes it
  in Phase 2.

### Wave 2: control-flow spike

Ran once and passed 6/6. It drove the real `ContainerRunner` and
`LifecycleRunGuard` with fake bodies, as a temporary in-crate test that was
removed afterwards. The archive and the interface contracts are in
`spike/README.md` and `spike/loop_flow_spike.rs`. Summary:

1. A break settles a container through the existing guard (`defuse()`, then
   `finalize` once). No new guard state is needed, and Drop synthesizes no
   terminal event.
2. A failed `finalize` (`Finished { failed: true }` with a primary error)
   discards the pending break.
3. Group pass settlement is a pure declaration-order fold, run after every
   leg settles. A handled leg failure keeps the break; `fail_fast` failure
   wins.
4. `prep` is a returning **action** through an injected runner, like
   `shell_runner`, not a `StackControl`. The executor already continues the
   list and after a conditional branch.
5. **New seam needed:** `RuntimeState` cannot restore its mutation layer.
   `replace_sequence_scope` restores outputs only. Add a pass-checkpoint
   restore that keeps `proxy_chain`.
6. Call ancestry can reuse `enter_container_proxy`'s RAII guard: repeated
   calls are legal, active-ancestor calls are refused, and branches are
   isolated. Cancellation uses `current().child()`. **No deadline carrier
   exists**; add an absolute-instant `CallDeadline` that a child can only
   tighten.

### Matrix contracts

`phase-1/matrix-contracts.md` expands every column: each loose verb from its
real signature, each format's duplicate-key and trailing-content behavior,
and the measured review-metadata decisions. Each cell is marked covered,
Phase N, or N/A.

Implemented the parameterized traversal for the one format that carries a
loop field today, YAML frontmatter on a document, as
`claudine/cli/tests/l1/loop_config_matrix.rs`. It is registered `cfg(unix)`
in `cli/tests/l1/main.rs` because it uses the Unix-only `launch_recorder`.
It has a `matrix!` macro with one `#[test]` per cell: 36 cells, each one
textual edit of the counting control, asserting exit status, launch count,
prompt text, that a rejection names the document, and that no audio spool
exists. All 36 pass, the slowest at about 1.6 s.

Deviation: the plan asks for one traversal per supported format. The other
formats (standalone YAML, JSON, JSON5, JSONL/NDJSON) carry only container
and task fields that do not exist until the Phase 2 grammar, so their
traversals are written in Phase 2 against the contract table. The probe run
measured 18 cells that differ from the contract today. They are not
registered, so `just test` stays green, and each is listed with its current
result in the contract:

- **11 unlocated errors**: `LoopInvalid` messages carry no document path.
  The cells are `loop: null`, `loop: {}`, a missing condition, null
  `until`/`while`/`max`/`action`/`actions`, both conditions together, both
  aliases together, and `action: ""`. Phase 2 makes the loop error located.
- **Accepted when they should be rejected**: `fail_fast: true`,
  `gate: null`, and `gate: {}`.
- **Rejected when it should resolve**: templated `max: "{{ cap }}"`.
- **Rejected but unlocated**: `fail_fast: null` and `on_rate_limit: null`.
- **Wrong exit status**: `max: 2` with the condition unmet exits 0, because
  the action runs after the last pass. The loop engine fixes this in Phase 3.

### Acceptance mapping

Every criterion (1–14) is mapped to an observable assertion, a proposed test
file, and its owning plan task in `phase-1/coverage-inventory.md` §6. That
table is the Phase 1 acceptance mapping.

### Verification (macOS)

- `just test` in `claudine/`: 9892 passed, 0 failed, 9 skipped, 27 slow. All
  slow tests are pre-existing. No `loop_config_matrix` test is slow; the
  longest takes about 2.7 s under full-suite load.
- `just lint` in `claudine/`: first run failed on `clippy::unnecessary_to_owned`
  in the new matrix macro's control row. Fixed, then clean.
- `cargo nextest run -p claudine-cli --test l1 -E 'test(loop_config_matrix)'`
  after the fix: 36/36 passed.
- `just check-tier-coverage claudine`: nothing stranded. The new tests carry
  no tier marker and run in L1.
- Not run: Darkmatter tests (no Darkmatter code changed) and other OS rigs.
  The only new test is `cfg(unix)` subprocess composition with no path or
  OS-specific logic, so Linux and WSL CI cover it. The temporary spike module
  was removed and `lifecycle/tests/mod.rs` is back to its prior content.

Requirement-to-test mapping for this phase: Phase 1 changes no behavior. Its
one test deliverable is the robustness-matrix traversal: 36 cells, each
named in `phase-1/matrix-contracts.md`. The 18 cells that differ from the
contract today are listed there with their measured current result and the
phase that changes them. The spike tests were a one-time experiment and are
archived, not registered.

Phase 1 status: complete, ready for review. Phase 2 is blocked on the
`on_rate_limit` ruling and, for the path work, on the `file_exists` ruling
(see `human_review_items`).
