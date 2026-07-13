---
agent: open_code/zai-coding-plan/glm-5.2
phases: 6
created: 2026-07-11
start_phase: 1
source_code:
  # Phase 1 — C1 test extraction targets
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/lifecycle/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/stream/logs/opencode/reasoning/tests.rs
  - claudine/lib/src/composition/lifecycle_executor.rs
  - claudine/lib/src/composition/lifecycle_executor/tests.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/composition/loop_engine/tests.rs
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/lib/src/composition/schema_validation/tests.rs
  - claudine/rendezvous/daemon/src/session_log.rs
  - claudine/rendezvous/daemon/src/session_log/tests.rs
  # Phase 2 — C6 error.rs render split
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/error/render.rs
  - claudine/lib/src/composition/mod.rs
  # Phase 3 — C2 lifecycle split
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/lifecycle/parse.rs
  - claudine/lib/src/composition/lifecycle/action_shape.rs
  - claudine/lib/src/composition/lifecycle/validate.rs
  - claudine/lib/src/composition/lifecycle/audio.rs
  - claudine/lib/src/composition/lifecycle/actions.rs
  - claudine/lib/src/composition/lifecycle/signatures.rs
  - claudine/lib/src/composition/lifecycle/context.rs
  - claudine/lib/src/composition/lifecycle/control.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/mod.rs
  # Phase 4 — C3 run_harness_loop decomposition
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/lifecycle_events.rs
  - claudine/cli/src/commands/wrap/harness_orch/error_routing.rs
  - claudine/cli/src/commands/wrap/harness_orch/control_dispatch.rs
  - claudine/cli/src/commands/wrap/harness_orch/proxy.rs
  - claudine/cli/src/commands/wrap/harness_orch/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/mod.rs
  # Phase 5 — C4 lib promotion
  - claudine/lib/src/composition/lifecycle/runtime.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/lib/src/composition/loop_engine.rs
  # Phase 6 — C5 stream ParserShared
  - claudine/lib/src/stream/providers/common.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/antigravity.rs
  - claudine/lib/src/stream/providers/mod.rs
documentation:
  - .opencode/skill/claudine/architecture.md
  - claudine/docs/topics/composition.md
packages:
  - claudine
source_files_during_phase_1:
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/lifecycle/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/stream/logs/opencode/reasoning/tests.rs
  - claudine/lib/src/composition/lifecycle_executor.rs
  - claudine/lib/src/composition/lifecycle_executor/tests.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/composition/loop_engine/tests.rs
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/lib/src/composition/schema_validation/tests.rs
  - claudine/rendezvous/daemon/src/session_log.rs
  - claudine/rendezvous/daemon/src/session_log/tests.rs
docs_updated_during_phase_1:
  - CLAUDE.md
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/features/2026-07-11-module-structure/critical-plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_2:
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render.rs
docs_updated_during_phase_2:
  - claudine/features/2026-07-11-module-structure/critical-plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/lifecycle/parse.rs
  - claudine/lib/src/composition/lifecycle/action_shape.rs
  - claudine/lib/src/composition/lifecycle/validate.rs
  - claudine/lib/src/composition/lifecycle/audio.rs
  - claudine/lib/src/composition/lifecycle/actions.rs
  - claudine/lib/src/composition/lifecycle/signatures.rs
  - claudine/lib/src/composition/lifecycle/context.rs
  - claudine/lib/src/composition/lifecycle/control.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/tests/boundary_lint.rs
docs_updated_during_phase_3:
  - claudine/features/2026-07-11-module-structure/critical-plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs
docs_updated_during_phase_4:
  - claudine/features/2026-07-11-module-structure/critical-plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/lib/src/composition/lifecycle/runtime.rs
  - claudine/lib/src/composition/lifecycle/mod.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_5:
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/features/2026-07-11-module-structure/critical-plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/architecture.md
source_files_during_phase_6:
  - claudine/lib/src/stream/providers/common.rs
  - claudine/lib/src/stream/providers/mod.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/pi.rs
  - claudine/lib/src/stream/providers/antigravity.rs
docs_updated_during_phase_6:
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/features/2026-07-11-module-structure/critical-plan.md
  - claudine/features/2026-07-11-module-structure/review.md
docs_created_during_phase_6:
  - claudine/features/2026-07-11-module-structure/phase6-discovery.md
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/architecture.md
---

# Critical Module-Structure Refactor — Execution Plan

Converts the six **Critical / Must Fix** findings (C1–C6) from
[`review.md`](review.md) into a dependency-ordered, high-confidence plan.

Every phase is observable through a compile, a `just test` run, or a `just
lint` pass. No phase changes runtime behavior — all six are pure structural
moves. The barrel re-export in `composition/mod.rs` (lines 39–112) is the key
enabler: most moves are invisible to the ~38 consuming files.

## Context (recap)

Roughly half of every god-file is inline test code. After mechanical test
extraction the worst offenders halve immediately. The remaining structural
work is a layering violation (lifecycle runtime logic in the CLI), a
1,001-sloc god-function (`run_harness_loop`), an 8-way stream-parser
copy-paste, and two files (`lifecycle.rs`, `error.rs`) that each mix four+
unrelated concerns.

## Dependency graph

```
Phase 1 (C1) ────────────────────────────────────────────┐
                                                         │
Phase 2 (C6) ──────────────────────────────────────┐     │
                                                   │     │
Phase 3 (C2) ── depends on Phase 1 for lifecycle ─┘     │
                                                         │
Phase 4 (C3) ── depends on Phase 1 for loop_control ─────┘
        │
        ▼
Phase 5 (C4) ── depends on Phase 4 (needs extracted pure routing)
        
Phase 6 (C5) ── independent of 3/4/5; depends only on Phase 1 for test extraction
```

Phase 2 and Phase 6 are independent of each other and of Phases 3–5; they can
run in parallel once Phase 1 is complete. Phase 5 must follow Phase 4 because
it promotes the routing logic that Phase 4 carves out of `run_harness_loop`.

## Verified structural facts driving this plan

1. **The sibling `tests.rs` pattern is already established** in 9 places:
   `lib/src/provider/mod.rs:52`, `cli/…/wrap/composition/mod.rs:2037`,
   `cli/…/wrap/mod.rs:695`, `cli/…/wrap/sequence/mod.rs:487`,
   `cli/…/wrap/env/mod.rs:427`, `cli/…/compose/mod.rs:616`,
   `cli/…/wrap/exec/wiring/mod.rs:80`, `contract/src/lib.rs:52`,
   `cli/…/config_tui/tabs/messenger/mod.rs:60`. Each uses `#[cfg(test)] mod
   tests;` at the bottom of the parent file, with a sibling `tests.rs`
   starting with `use super::*;`.

2. **`composition/mod.rs` lines 39–112 is a pure `pub use` barrel.** Only
   three deep-path import sites exist outside it: `wrap/inline.rs` →
   `closure`, `output/error_walker.rs` + `harness_orch` →
   `lifecycle_context`, `harness_orch` → `lifecycle_executor`. Facades in
   `mod.rs` keep all three compiling.

3. **`loop_control.rs` has two separate inline test modules**: `mod tests`
   (line 2,697) and `mod requeue_fallback_tests` (line 5,910). Both move to
   sibling files.

4. **`reasoning.rs` has a `#[cfg(test)]` at line 526 on a single accessor
   method** (`generation_count_since_progress`), not a test module. It stays
   in the main file; only the `mod tests` at line 1,744 extracts.

5. **The terminal-recovery sequence in `run_harness_loop`** is copy-pasted
   three times: failure path (2,321–2,458), inline-closure path (2,460–2,581),
   success path (2,583–2,693). Each follows the same five steps:
   execute/classify → `handle_terminal_evaluation_error` →
   `dispatch_terminal_control` → `run_finalize_with_recovery` → return.

6. **The stream parser duplication is verified across 8 provider files**.
   The `SemanticStreamParser` trait (`parser.rs:19`) has two methods:
   `feed_line` and `finish`. Each provider re-declares ~12 shared fields and
   re-implements `emit_provider_extension`, `emit_malformed_warning`,
   `base_extra`, the `finish` summary builder, and `classify_error`.

7. **`error.rs` rendering layer** spans `impl BlockError` (1,828–2,582), six
   `render_*` free functions (2,589–2,876), and `impl Diagnostic` (2,878 end
   of file minus tests). The barrel exports the enum and aux types (line 41),
   never the render functions — zero external blast radius.

---

## Phase 1 — C1: Extract Inline Test Modules + Written Convention

**Goal:** Move every `#[cfg(test)] mod tests { … }` block from files
> ~1,500 lines to a sibling `tests.rs` file. This is the single
highest-leverage, lowest-risk change: it halves the god-files and makes every
subsequent phase easier.

**Risk:** Zero — pure mechanical move. No production line changes.

**Verification:** `just test` + `just lint` in the claudine area after each
file. The test binary links the same symbols regardless of whether the module
is inline or in a sibling file.

### Step 1.1 — Document the test-placement convention

Before touching any files, document the rule so the area does not drift back.

- [x] Add a "Test Placement" section to `.opencode/skill/claudine/architecture.md`
  stating:

  > **Inline tests** (`#[cfg(test)] mod tests { … }`) are the default for
  > small files. Once a file exceeds **~800 production lines** or its test
  > module exceeds **~300 lines**, move tests to a sibling file declared via
  > `#[cfg(test)] mod tests;` at the bottom of the parent. This pattern is
  > already established in `lib/src/provider/`, `cli/…/wrap/composition/`,
  > and `cli/…/wrap/exec/wiring/`.

- [x] Add a one-line cross-reference in `AGENTS.md` under the Testing section:
  `# Test placement: see claudine skill architecture.md → Test Placement`

### Step 1.2 — Extract tests from `lifecycle.rs`

**Files:**
- Modify: `claudine/lib/src/composition/lifecycle.rs`
- Create: `claudine/lib/src/composition/lifecycle/tests.rs`

- [x] Cut lines 3,613–end (the `#[cfg(test)] mod tests { … }` block) from
      `lifecycle.rs`
- [x] Paste into `claudine/lib/src/composition/lifecycle/tests.rs`, prefixed
      with `use super::*;` and the module doc-comment
- [x] At the bottom of `lifecycle.rs`, replace the cut block with:
      `#[cfg(test)] mod tests;`
- [x] Verify: `just test` (claudine lib)
- [x] Confirm `lifecycle.rs` is now ~3,600 lines (down from 7,549)

**Note:** `lifecycle.rs` currently declares `lifecycle` as a flat module in
`composition/mod.rs:21` (`pub mod lifecycle;`). Since we are converting to a
directory in Phase 3, for now the sibling file is
`lifecycle/tests.rs` — but wait: a file `lifecycle.rs` with `mod tests;`
creates `lifecycle/tests.rs` automatically. This works because Rust resolves
`mod tests;` in `lifecycle.rs` to `lifecycle/tests.rs` (the `lifecycle.rs`
file is the crate root of the `lifecycle` module). No `composition/mod.rs`
change needed.

### Step 1.3 — Extract tests from `loop_control.rs`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
- Create: `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests.rs`

- [x] Cut lines 2,697–5,908 (the `mod tests { … }` block)
- [x] Cut lines 5,910–end (the `mod requeue_fallback_tests { … }` block)
- [x] Paste both into `loop_control/tests.rs` as two separate modules:
      `pub(crate) mod tests { use super::*; … }` and
      `pub(crate) mod requeue_fallback_tests { use super::*; … }`
- [x] At the bottom of `loop_control.rs`, replace with:
      ```rust
      #[cfg(test)]
      mod tests;

      #[cfg(test)]
      mod requeue_fallback_tests;
      ```
      Wait — `mod tests;` resolves to `loop_control/tests.rs`. For two
      modules, declare:
      ```rust
      #[cfg(test)]
      mod tests;
      ```
      and inside `loop_control/tests.rs`:
      ```rust
      mod tests_inner { use super::*; … }
      mod requeue_fallback_tests { use super::*; … }
      ```
      Simpler approach: create ONE sibling file `loop_control/tests.rs`
      containing both inline modules with `use super::*;` in each. Declare
      `#[cfg(test)] mod tests;` at the bottom of `loop_control.rs`.

- [x] Verify: `just test` (claudine-cli)
- [x] Confirm `loop_control.rs` is now ~2,700 lines (down from 6,064)

### Step 1.4 — Extract tests from `error.rs`

**Files:**
- Modify: `claudine/lib/src/composition/error.rs`
- Create: `claudine/lib/src/composition/error/tests.rs`

- [x] Cut lines 3,120–end (the `#[cfg(test)] mod tests { … }` block)
- [x] Paste into `error/tests.rs` with `use super::*;`
- [x] At the bottom of `error.rs`, add: `#[cfg(test)] mod tests;`
- [x] Verify: `just test` (claudine lib)
- [x] Confirm `error.rs` is now ~3,100 lines (down from 4,519)

### Step 1.5 — Extract tests from `reasoning.rs`

**Files:**
- Modify: `claudine/lib/src/stream/logs/opencode/reasoning.rs`
- Create: `claudine/lib/src/stream/logs/opencode/reasoning/tests.rs`

- [x] Cut lines 1,744–end (the `#[cfg(test)] mod tests { … }` block)
- [x] Paste into `reasoning/tests.rs` with `use super::*;`
- [x] At the bottom of `reasoning.rs`, add: `#[cfg(test)] mod tests;`
- [x] **Do NOT move** the `#[cfg(test)]` accessor at line 526
      (`generation_count_since_progress`) — it is a test-only method on the
      impl, not a test module. It stays.
- [x] Verify: `just test` (claudine lib)
- [x] Confirm `reasoning.rs` is now ~1,750 lines (down from 4,024)

### Step 1.6 — Extract tests from `lifecycle_executor.rs`

**Files:**
- Modify: `claudine/lib/src/composition/lifecycle_executor.rs`
- Create: `claudine/lib/src/composition/lifecycle_executor/tests.rs`

- [x] Cut lines 1,294–end (the `#[cfg(test)] mod tests { … }` block)
- [x] Paste into `lifecycle_executor/tests.rs` with `use super::*;`
- [x] At the bottom, add: `#[cfg(test)] mod tests;`
- [x] Verify: `just test`
- [x] Confirm ~1,300 lines (down from 3,634)

### Step 1.7 — Extract tests from `loop_engine.rs`

**Files:**
- Modify: `claudine/lib/src/composition/loop_engine.rs`
- Create: `claudine/lib/src/composition/loop_engine/tests.rs`

- [x] Cut lines 1,532–end (the `#[cfg(test)] mod tests { … }` block)
- [x] Paste into `loop_engine/tests.rs` with `use super::*;`
- [x] At the bottom, add: `#[cfg(test)] mod tests;`
- [x] Verify: `just test`
- [x] Confirm ~1,550 lines (down from 3,476)

### Step 1.8 — Extract tests from `schema_validation.rs`

**Files:**
- Modify: `claudine/lib/src/composition/schema_validation.rs`
- Create: `claudine/lib/src/composition/schema_validation/tests.rs`

- [x] Cut lines 1,665–end (the `#[cfg(test)] mod tests { … }` block)
- [x] Paste into `schema_validation/tests.rs` with `use super::*;`
- [x] At the bottom, add: `#[cfg(test)] mod tests;`
- [x] Verify: `just test`
- [x] Confirm ~1,700 lines (down from 3,398)

### Step 1.9 — Extract tests from `rendezvous/daemon/src/session_log.rs`

**Files:**
- Modify: `claudine/rendezvous/daemon/src/session_log.rs`
- Create: `claudine/rendezvous/daemon/src/session_log/tests.rs`

- [x] Cut lines 1,345–end (the `#[cfg(test)] mod tests { … }` block)
- [x] Paste into `session_log/tests.rs` with `use super::*;`
- [x] At the bottom, add: `#[cfg(test)] mod tests;`
- [x] Verify: `cargo nextest run -p claudine-rendezvous-daemon` (or `just test`
      from the rendezvous/daemon area)
- [x] Confirm ~1,350 lines (down from 3,154)

### Phase 1 Exit Criteria

- [x] `just test` passes for claudine, claudine-cli, and rendezvous/daemon
- [x] `just lint` passes for claudine area
- [x] The convention is documented in `architecture.md`
- [x] All 8 files in the C1 table are at their post-extraction size

---

## Phase 2 — C6: Split `composition/error.rs` into Data Model vs Rendering

**Goal:** Move the rendering layer (`impl BlockError`, six `render_*` free
functions, `impl Diagnostic`) into `composition/error/render.rs`. This is the
best "first split" because it has **zero external blast radius** — the barrel
exports types, not render functions — and establishes the submodule-split
pattern used in Phases 3 and 4.

**Risk:** Very low. The rendering code is private (not exported through the
barrel). The split is a `git mv` + path fixup.

**Prerequisite:** Phase 1 complete (error.rs tests already extracted to
`error/tests.rs`).

### Step 2.1 — Convert `error.rs` to `error/mod.rs`

**Files:**
- Move: `claudine/lib/src/composition/error.rs` → `claudine/lib/src/composition/error/mod.rs`
- Create: `claudine/lib/src/composition/error/render.rs`

Since `error/tests.rs` already exists from Phase 1 (Step 1.4), converting
`error.rs` to `error/mod.rs` is required so the directory is coherent.

- [x] `git mv claudine/lib/src/composition/error.rs claudine/lib/src/composition/error/mod.rs`
- [x] The existing `error/tests.rs` sibling stays in place
- [x] Verify: `just test` — tests should pass unchanged (the `mod tests;`
      declaration in `mod.rs` now resolves to `error/tests.rs` in the
      directory)

### Step 2.2 — Carve rendering into `error/render.rs`

**Files:**
- Modify: `claudine/lib/src/composition/error/mod.rs`
- Create: `claudine/lib/src/composition/error/render.rs`

Move these sections from `mod.rs` to `render.rs`:

| Section | Current lines (post-test-extraction) | Destination |
|---------|--------------------------------------|-------------|
| `impl BlockError for CompositionError` | ~1,828–2,582 | `render.rs` |
| `fn pointer_to_dotted` | ~2,589–2,620 | `render.rs` |
| `fn render_file_link` | ~2,622–2,641 | `render.rs` |
| `fn render_inline_sequence_mismatch_block` | ~2,643–2,677 | `render.rs` |
| `fn render_agent_resolution_failed_body` | ~2,679–2,711 | `render.rs` |
| `fn render_sequence_missing_properties_block` | ~2,713–2,766 | `render.rs` |
| `fn render_missing_properties_block` | ~2,768–2,828 | `render.rs` |
| `fn compose_failed_code` | ~2,830–2,876 | `render.rs` |
| `impl Diagnostic for CompositionError` | ~2,878–end | `render.rs` |

What stays in `mod.rs`:
- The `CompositionError` enum and all aux types (lines 1–1,557)
- `impl CompositionError` constructors + frontmatter enrichment (1,558–1,809)
- The `FrontmatterHighlightTarget` enum (if rendering needs it, move it to
  `render.rs` and re-export from `mod.rs` if any non-render code uses it)

- [x] Create `error/render.rs` with `use super::*;` and the `BlockError` impl
- [x] In `error/mod.rs`, declare `#[allow(missing_docs)] mod render;` (or
      `pub(super) mod render;` depending on visibility)
- [x] Ensure `render.rs` has access to the private enum variants and aux
      types it references — `use super::*;` in `render.rs` provides this
      since it is a child of the `error` module
- [x] If any helper used by `render.rs` is currently `fn` (private) in
      `mod.rs`, either move it to `render.rs` or make it `pub(super)`
- [x] Verify: `just test` + `just lint`

### Step 2.3 — Confirm barrel is unchanged

**Files:**
- Verify: `claudine/lib/src/composition/mod.rs` (no changes needed)

- [x] Confirm `composition/mod.rs:17` still says `mod error;` (now resolves
      to `error/mod.rs` — no change needed)
- [x] Confirm lines 41–45 still export `CompositionError`,
      `DroppedOptional`, etc. — these are defined in `error/mod.rs`,
      unchanged
- [x] Confirm no external file imports a `render_*` function or the
      `BlockError` impl directly (grep for `render_file_link`,
      `render_missing_properties_block`, etc.)

### Phase 2 Exit Criteria

- [x] `just test` passes for claudine lib
- [x] `just lint` passes
- [x] `error/mod.rs` is ~1,800 lines (enum + constructors)
- [x] `error/render.rs` is ~1,300 lines (BlockError + Diagnostic + render fns)
- [x] No external file needed modification

---

## Phase 3 — C2: Split `composition/lifecycle.rs` into a `lifecycle/` Subdirectory

**Goal:** After test extraction (~3,600 lines), split the file by its six
existing clusters into named submodules. Also fold the four lifecycle-sibling
files (`lifecycle_actions.rs`, `lifecycle_context.rs`, `lifecycle_control.rs`,
`lifecycle_executor.rs`) into the same directory so the lifecycle family lives
in one place.

**Risk:** Low. The barrel in `composition/mod.rs` keeps facades alive. Only
three deep-path importers need to compile.

**Prerequisite:** Phase 1 complete (lifecycle.rs tests extracted to
`lifecycle/tests.rs`).

### Step 3.1 — Convert `lifecycle.rs` to `lifecycle/mod.rs` and carve clusters

**Files:**
- Move: `lifecycle.rs` → `lifecycle/mod.rs` (it already has `lifecycle/tests.rs`
  from Phase 1)
- Create: `lifecycle/parse.rs`, `lifecycle/action_shape.rs`,
  `lifecycle/validate.rs`, `lifecycle/audio.rs`

The six clusters and their destinations:

| New module | Content (current lines in lifecycle.rs, post-extraction) |
|------------|----------------------------------------------------------|
| `lifecycle/mod.rs` | Config/type model: `LifecycleConfig` (187), `LifecycleStacks` (219), `LifecycleSignal` (244), `LifecycleRuntimeState` (296), `DefaultLifecycleEmitter` (387), `LifecycleRunGuard` (452–792), `LifecycleRuntimeContext` (811–957), `LifecycleNotification` (127). Plus `pub mod` declarations + `pub use` facades. |
| `lifecycle/parse.rs` | `parse_lifecycle_config` (1161), `parse_event_block` (1268), `parse_lifecycle_stack` (1335), `annotate_stack_error` (1352), `parse_lifecycle_stack_item` (1440), `parse_stack_item_action_object` (1665), `parse_scalar_action` (1715), `parse_bare_verb_string` (1749), `parse_long_form_action_object` (1789), `scan_removed_validation_keys` (1084), `parse_serde_unknown_field` (3341). |
| `lifecycle/action_shape.rs` | `parse_positional_action` (1886), `classify_positional_value` (1910), `validate_positional_arity_and_build` (1954), `build_non_control_positional_action` (2023), `check_exact_positional_arity` (2086), `check_optional_positional_arity` (2112), `check_positional_signature` (2140), `did_you_mean_verb` (2182), `build_action_from_params` (2259), `parse_lifecycle_control_long` (2381), `collect_named_signature_args` (2454), `reject_extra_params` (2487). |
| `lifecycle/validate.rs` | `validate_no_interpolation_leaks` (2585), `find_matching_warning_reason` (2646), `iter_stack_expression_surfaces` (2691), `iter_action_expressions` (2720), `visit_string_literals` (2849), `validate_no_undefined_lifecycle_variables` (2927), `find_undefined_top_level_variable` (2997), `find_undefined_stack_variable` (3036), `first_undefined_stack_variable` (3076), `resolves_outside_frontmatter` (3090), `undefined_bare_variable` (3109), `undefined_stack_variable` (3129), `validate_no_err_in_no_error_events` (3161), `surface_references_err` (3207), `literal_spans_reference_err` (3222), `references_bare_err` (3236), `collect_lifecycle_shell_commands` (3291), `expr_as_string_literal` (3309). |
| `lifecycle/audio.rs` | `audio_phases` (856), TTS/sound blocking emission, `run_blocking_with_timeout`, `emit_lifecycle_signal` (884), `AudioPhase` enum, `play_effect_blocking`, `normalize_empty_string` (3320). |

- [x] `git mv lifecycle.rs lifecycle/mod.rs`
- [x] Cut each cluster into its named file with `use super::*;` + necessary
      `use` imports
- [x] In `lifecycle/mod.rs`, declare:
      ```rust
      mod parse;
      mod action_shape;
      mod validate;
      mod audio;
      ```
- [x] Make any cross-module references work via `pub(crate)` or `pub(super)`
      on the items each submodule needs from `mod.rs` (e.g. `parse.rs` needs
      `LifecycleConfig`, `LifecycleStacks`, `LifecycleNotification`)
- [x] Verify: `just test` + `just lint`

### Step 3.2 — Fold sibling files into `lifecycle/`

**Files:**
- Move: `lifecycle_actions.rs` → `lifecycle/actions.rs`
  - Carve signature registry (lines 344–628) → `lifecycle/signatures.rs`
- Move: `lifecycle_context.rs` → `lifecycle/context.rs`
- Move: `lifecycle_control.rs` → `lifecycle/control.rs`
- Move: `lifecycle_executor.rs` → `lifecycle/executor.rs`
  - Note: `lifecycle_executor/tests.rs` from Phase 1 moves too (as
    `lifecycle/executor/tests.rs` — the `mod tests;` declaration travels with
    the file)

- [x] For each move: `git mv` the file, update any `use super::…` paths that
      referenced the old parent
- [x] In `lifecycle/actions.rs`, carve `signatures.rs` for the signature
      registry; declare `mod signatures;` in `actions.rs`
- [x] In `lifecycle/mod.rs`, declare:
      ```rust
      pub mod actions;
      pub mod context;
      pub mod control;
      pub mod executor;
      ```

### Step 3.3 — Preserve facades in `composition/mod.rs`

**Files:**
- Modify: `claudine/lib/src/composition/mod.rs`

The barrel (lines 14–37) currently declares `pub mod lifecycle;` (line 21),
`mod lifecycle_actions;` (line 22), `pub mod lifecycle_context;` (line 23),
`pub mod lifecycle_control;` (line 24), `pub mod lifecycle_executor;` (line
25). After the fold, these four siblings are submodules of `lifecycle/`, so
the old declarations are removed. But the `pub use` re-exports (lines 50–69)
and the three deep-path importers need them.

- [x] Replace the five module declarations with:
      ```rust
      pub mod lifecycle;
      ```
- [x] Add facade re-exports so the old module paths still resolve:
      ```rust
      pub use self::lifecycle::actions as lifecycle_actions;
      pub use self::lifecycle::context as lifecycle_context;
      pub use self::lifecycle::control as lifecycle_control;
      pub use self::lifecycle::executor as lifecycle_executor;
      ```
- [x] Verify the three deep-path importers still compile:
      1. `cli/…/wrap/inline.rs` → `closure` (resolves through barrel)
      2. `cli/…/output/error_walker.rs` → `lifecycle_context` (now facade)
      3. `cli/…/harness_orch/loop_control.rs` → `lifecycle_executor` (now facade)
- [x] Verify: `just test` + `just lint`

### Step 3.4 — Update skill architecture doc

**Files:**
- Modify: `.opencode/skill/claudine/architecture.md`

- [x] Update the module map to reflect the `lifecycle/` directory structure

### Phase 3 Exit Criteria

- [x] `just test` passes for claudine lib + claudine-cli
- [x] `just lint` passes
- [x] `lifecycle/mod.rs` is ~960 lines (config/types only)
- [x] No deep-path importer needed code changes (only facades in mod.rs)
- [x] The lifecycle family is unified under `composition/lifecycle/`

---

## Phase 4 — C3: Decompose `run_harness_loop` + Terminal-Recovery De-duplication

**Goal:** Break up the 1,001-sloc `run_harness_loop` function
(`loop_control.rs:1494–2695`) by (a) collapsing the three-way copy-pasted
terminal-recovery sequence into one helper, (b) introducing a context struct
and `LoopStep` enum to eliminate the `too_many_arguments` plague, and (c)
splitting `loop_control.rs`'s six mixed concerns into named sibling files.

**Risk:** Medium — this is the one phase with real logic restructuring.
However, the terminal-recovery de-duplication is mechanical (the three copies
are near-verbatim) and the context-struct extraction is mechanical (the
arguments don't change, just get bundled). Test coverage from Phase 1 is the
safety net.

**Prerequisite:** Phase 1 complete (loop_control.rs tests extracted).

> **Status truthing (2026-07-11, second pass):** Step 4.1's `LoopStep` and
> `HarnessLoopCtx` landed (loop_control.rs:33/39), but the ctx is
> immediately destructured back into ~28 locals at the top of
> `run_harness_loop_inner` — the phase helpers do not consume it, and the
> 15 `#[allow(clippy::too_many_arguments)]` remain. Steps 4.2 and 4.3 did
> **not** land (no `drive_terminal_recovery`; the three terminal-recovery
> sequences are still copy-pasted at ~2470/~2637/~2748; the sibling split
> files do not exist). A previously ticked 4.3 verify checkbox has been
> reset to match reality.

### Step 4.1 — Introduce `LoopStep` enum and `HarnessLoopCtx`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`

- [x] Define `enum LoopStep`:
      ```rust
      enum LoopStep {
          NextAttempt,
          Return(i32, Option<AgentExecutionPerf>, Option<IterationSummarySignals>),
          Abort { reason: String, code: i32 },
      }
      ```
- [x] Define `struct HarnessLoopCtx<'a>` bundling the ~10 arguments that
      every phase threads: `provider`, `binary_path`, `child_cwd`,
      `base_args`, `base_env`, `term`, `lifecycle_guard`, `effect_engine`,
      `harness_context`, `env_context`, etc. (not the mutable prompt state —
      that stays separate or as a field)
- [ ] Refactor each of the 13 sequential phases inside `run_harness_loop`
      into a method or free function taking `&mut HarnessLoopCtx` and
      returning `LoopStep`
      > **Deferred (2026-07-11 second pass):** after 4.2's dedup the loop
      > body is ~960 lines of *once-each* sequential phases with no
      > remaining duplication. Converting them to phase methods requires
      > moving ~20 loop-mutable locals into a state struct — judged not
      > worth the behavior-drift risk in this pass. Revisit only if the
      > loop grows again.
- [ ] The loop body becomes: call phase → match on `LoopStep` → continue or
      return *(deferred with the bullet above)*
- [ ] This eliminates the 15 `#[allow(clippy::too_many_arguments)]` on the
      phase functions (they now take one `&mut HarnessLoopCtx`)
      *(deferred — the allows sit on genuinely multi-param helpers; they
      disappear only with the state-struct conversion above)*
- [x] Verify: `just test` (the behavior is identical — this is a pure
      extraction)

### Step 4.2 — Collapse terminal-recovery into `drive_terminal_recovery`

**Files:**
- Modify: `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`

The three copy-pasted sequences (failure 2,321–2,458; inline-closure
2,460–2,581; success 2,583–2,693) each do:

1. Execute/classify terminal event (`execute_terminal_event` at line 62)
2. `handle_terminal_evaluation_error` (line 1,307)
3. `dispatch_terminal_control` (line 1,080)
4. `run_finalize_with_recovery` (line 1,429)
5. Return

The Abort arms at 2,396–2,422 / 2,527–2,553 / 2,638–2,664 are near-verbatim.

- [x] Extract one helper — landed as
      `drive_terminal_recovery(lifecycle_guard, signal, event_err, …) ->
      Result<TerminalRecovery>` (parameter style matches its sibling
      `run_finalize_with_recovery`; `Err` = halt, `NextAttempt`/`Completed`
      = loop control). It runs the whole 5-step sequence including
      `execute_terminal_event`, unifying the per-path finalize `err` as
      `event_err.or(downgrade_err)` — verified exact for all three paths.
- [x] Replace each of the three inline sequences with a call to
      `drive_terminal_recovery`
- [x] Verify: `just test` — 54/54 `loop_control` tests + 774 wrap-scoped
      tests pass
- [x] Confirm ~400 lines collapsed (three ~110-line sequences → three
      ~24-line call sites + one shared helper)

### Step 4.3 — Split `loop_control.rs` into sibling files

**Files:**
- Create: `harness_orch/lifecycle_events.rs` (terminal-event execution +
      stack-context assembly: `execute_terminal_event`, `TerminalEventOutcome`)
- Create: `harness_orch/error_routing.rs` (the `emit_*_with_err` /
      `handle_*_evaluation_error` family: `emit_blocked_finalize_with_err`,
      `emit_failure_finalize_with_err`, `surface_catch_evaluation_error`,
      `handle_terminal_evaluation_error`)
- Create: `harness_orch/control_dispatch.rs` (`dispatch_terminal_control`,
      `ControlBudgets`, `TerminalControlAction`, `run_finalize_with_recovery`)
- Create: `harness_orch/proxy.rs` (`ProxyTracking`,
      `run_target_initialize`)
- Create: `harness_orch/requeue.rs` (the dead-code `RequeueEnqueueError`,
      `write_requeue_fallback`, `enqueue_requeue_entry*`,
      `try_enqueue_via_daemon` — lines 876–1,048 — currently
      `#[allow(dead_code)]`, retained for the future rendezvous backend)

- [x] After splitting, `loop_control.rs` contains only: `run_harness_loop`
      (+ `run_harness_loop_inner`), `HarnessLoopCtx`, and `LoopStep`
      (1,126 lines, from 2,743)
      > **Placement deviation:** the five modules landed as **children of
      > `loop_control/`** (beside the existing `tests.rs`) rather than
      > `harness_orch/` siblings, with `use super::*;` in each child and
      > glob re-imports in the parent. This keeps the parent's import set
      > flowing into the children and preserves the test suite's
      > `use super::*;` contract with zero test edits. Items are
      > `pub(super)`; `drive_terminal_recovery`/`TerminalRecovery` live in
      > `control_dispatch.rs` with the dispatch machinery they wrap.
- [x] Update the module declarations for the new children (in
      `loop_control.rs`; `harness_orch/mod.rs` unchanged — it still exposes
      only `run_harness_loop`)
- [x] Verify: `just test` + `just lint`

### Phase 4 Exit Criteria

- [x] `just test` passes for claudine-cli
- [x] `just lint` passes
- [ ] `run_harness_loop` is < 300 lines (from 1,001) — *not met by design*:
      after the dedup, `run_harness_loop_inner` is ~960 lines of once-each
      sequential phases; the phase-method conversion is deferred (see 4.1)
- [x] `drive_terminal_recovery` replaces three copy-pasted sequences
- [ ] `loop_control.rs` is < 1,000 lines total — 1,126; the miss is
      entirely the undecomposed loop function above
- [ ] Zero `#[allow(clippy::too_many_arguments)]` on the phase functions —
      deferred with the state-struct conversion (16 remain, one per
      multi-param helper across the split modules)

---

## Phase 5 — C4: Promote the Lifecycle Runtime into the Lib Crate (Layering Fix)

**Goal:** The blocked/failure/finalize routing algorithm exists in three
places: `cli/…/loop_control.rs`, `cli/…/composition/mod.rs`, and
`lib/…/loop_engine.rs`. This logic is provider-agnostic and pure. Promote it
into a lib module so both the harness loop and the compose preflight consume
one implementation. Also move `IterationSummarySignals` from the CLI to the
lib.

**Risk:** Medium. The three copies already drift in return types
(`CompositionError` vs loop-step). The consolidation is a semantic
unification, not just a move. Must be carefully tested.

**Prerequisite:** Phase 4 complete (the pure routing must already be carved
out of `run_harness_loop` as `drive_terminal_recovery` and friends).

> **Addendum (2026-07-11, second pass):** Phase 5 landed *without* Phase 4's
> `drive_terminal_recovery` — the lib routers (`route_blocked_finalize`,
> `route_failure_finalize`, `route_loop_gate`) are consumed directly by the
> three still-copy-pasted terminal-recovery sites in `loop_control.rs` (via
> the adapters at :482/:582/:1332), the compose preflight
> (`wrap/composition/mod.rs:204/:327`), and lib `loop_engine`
> (:1101/:1197/:1210). **No Phase 5 rework is needed**: completing Step 4.2
> collapses the three CLI call sites into one helper, which becomes the
> single harness-side consumer of the routing layer — the shape this phase
> originally intended. `lifecycle/runtime.rs` itself should need at most
> trivial signature adjustments.

### Step 5.1 — Create `lib/src/composition/lifecycle/runtime.rs`

**Files:**
- Create: `claudine/lib/src/composition/lifecycle/runtime.rs`
- Modify: `claudine/lib/src/composition/lifecycle/mod.rs`

The shared routing functions to consolidate:

| Function | Current CLI location | Current lib location |
|----------|---------------------|---------------------|
| `emit_blocked_finalize_with_err` | `loop_control.rs:329` | — |
| `emit_failure_finalize_with_err` | `loop_control.rs:467` | — |
| `surface_catch_evaluation_error` | `loop_control.rs:1265` | — |
| `run_finalize_with_recovery` | `loop_control.rs:1429` | — |
| `emit_preflight_blocked_and_finalize` | `composition/mod.rs:226` (CLI) | — |
| `route_init_failure*` | — | `loop_engine.rs:1052–1152` |
| `run_loop_gate` | — | `loop_engine.rs:1154` |

- [x] In `runtime.rs`, define a provider-agnostic routing API that returns
      pure decision types (not CLI-specific `LoopStep` or
      `CompositionError`):
      ```rust
      pub struct TerminalRoutingDecision { ... }
      pub fn route_blocked_finalize(...) -> TerminalRoutingDecision { ... }
      pub fn route_failure_finalize(...) -> TerminalRoutingDecision { ... }
      pub fn route_loop_gate(...) -> TerminalRoutingDecision { ... }
      ```
- [x] The CLI callers adapt: `drive_terminal_recovery` (Phase 4) calls the
      lib `route_*` functions and maps the decision to `LoopStep`
      (the Phase 4 helper is not present in this worktree, so the existing
      terminal-recovery adapters call the shared router directly)
- [x] The compose preflight (`cli/…/composition/mod.rs:226`) calls the same
      lib `route_*` functions instead of its own mirror
- [x] Declare `pub mod runtime;` in `lifecycle/mod.rs`
- [x] Export from `composition/mod.rs` barrel if needed

### Step 5.2 — Move `IterationSummarySignals` to lib

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition/mod.rs` (remove the
  struct at line 408)
- Create/modify: `claudine/lib/src/composition/lifecycle/runtime.rs` (add
  the struct here)

`IterationSummarySignals` is defined in the CLI
(`wrap/composition/mod.rs:408`) but is conceptually the sibling of lib
`loop_engine`'s `LoopIterationOutput` (`loop_engine.rs:126–140`) and flows
back into lib. It belongs in lib.

- [x] Move the struct definition to `lifecycle/runtime.rs`
- [x] Update the CLI import in `loop_control.rs:19`:
      `use super::super::composition::IterationSummarySignals;` →
      `use claudine::composition::lifecycle::runtime::IterationSummarySignals;`
      (or export through the barrel)
- [x] Verify: `just test` + `just lint`

### Step 5.3 — Remove the CLI `composition/mod.rs` mirror

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition/mod.rs`

- [x] Replace `emit_preflight_blocked_and_finalize` (line 226, documented at
      line 194 as a mirror of the harness-loop version) with a call to the
      lib `route_*` function
- [x] Remove the duplicated routing logic
- [x] Verify: `just test` — the 6 tests in
      `composition/tests.rs:1029–1543` that test
      `emit_preflight_blocked_and_finalize_*` must still pass
- [x] Confirm the "Mirrors `harness_orch::loop_control::…`" doc comment at
      line 194 is updated or removed (the mirror is gone)

### Phase 5 Exit Criteria

- [x] `just test` passes for claudine + claudine-cli
- [x] `just lint` passes
- [x] The routing algorithm has exactly one implementation in
      `lib/…/lifecycle/runtime.rs`
- [x] `IterationSummarySignals` lives in the lib crate
- [x] The `composition/mod.rs:194` "Mirrors" comment is gone

---

## Phase 6 — C5: Collapse the 8-Way Parser Copy-Paste (Re-scoped, Incremental)

> **Re-scope rationale (2026-07-11):** the first execution attempt was
> reverted. Its report was half right: the original Step 6.1 made unifying
> parser *state* (`ParserShared` with "~12 compatible fields") the entry
> gate, and listed **antigravity.rs** — the buffered-JSON outlier with no
> line loop — as a `feed_line`-driver target. When the structs turned out to
> differ materially, the whole phase wedged with nothing landed. The
> verified duplication (7 byte-identical `emit_provider_extension` copies,
> `emit_malformed_warning`, `base_extra`, the `finish` assembly idiom, the
> `classify_error` cascade) never depended on state unification. This
> rewrite extracts those as **independently-landable increments** — free
> functions / builders, zero changes to any parser's own state struct — and
> demotes the speculative generic driver to an evidence-gated decision at
> the end. Each increment is revert-safe on its own.
>
> **Execution constraints:** run increments serially in this worktree (the
> aborted attempt also hit >60s cargo-lock contention from a concurrent
> build; wait out locks or use an isolated `CARGO_TARGET_DIR`, never abort
> the design work over the lock). Antigravity participates in 6a–6c only —
> never in a line driver.

**Goal:** every provider file shrinks toward its typed dispatch arms — the
only part that actually differs — so the next provider on the ladder costs
one dispatch match, not a 1,000-line copy.

**Risk:** Low per increment. Parser logic is well-tested; each increment
converts one concern across the parsers with `just test` between parsers.

**Prerequisite:** Phase 1 complete. Independent of Phases 2–5.

### Step 6.0 — Discovery: ground-truth diff of the 8 parser structs

Before any extraction, produce a field-by-field comparison of the 8 parser
state structs and their `finish()` implementations:

- [x] The intersection: fields identical in name/type/semantics across all
      8 (and across the 7 line-oriented parsers)
- [x] Fields shared by 4+ parsers but not all
- [x] Genuinely provider-specific state, per provider
- [x] Which `StreamExecutionSummary` fields each `finish()` populates, and
      any provider-specific computation (Kimi wire flushes, Antigravity
      buffered-object finish; the "OpenCode stderr-state merge" turned out
      to live in `OpenCodeLogBridge`, not in `finish()`)
- [x] The per-provider `classify_error` keyword deltas as a table

The findings gate what 6a–6c share and decide 6d. Recorded in
[`phase6-discovery.md`](phase6-discovery.md). Headline: only 8 of ~15
fields are identical across all 8 structs (`token_usage`/`cost_usd`/
`num_turns` split Option-vs-plain), confirming the delegation design over
state unification.

### Step 6a — Shared emit helpers (no state changes)

**Files:**
- Create: `claudine/lib/src/stream/providers/common.rs`
- Modify: `claudine/lib/src/stream/providers/mod.rs` (`pub(crate) mod common;`)
- Modify: the 8 provider files, one at a time

Extract the byte-identical-modulo-`Provider` helpers as free functions over
explicit parameters — **no `ParserShared` struct, no field moves**:

```rust
pub(crate) fn base_extra(provider: Provider, line_num: usize, raw_kind: &str) -> Map<String, Value>;
pub(crate) fn emit_provider_extension(sink: &mut dyn SemanticEventSink, provider: Provider, line_num: usize, kind: &str, payload: Value);
pub(crate) fn emit_malformed_warning(sink: &mut dyn SemanticEventSink, provider: Provider, line_num: usize, err: &str);
```

(Exact signatures follow 6.0's findings — if a provider's copy has a real
variation, either parameterize it or leave that provider's copy local with a
comment naming the delta.)

- [x] Create `common.rs` with the three helpers (`base_extra_parts`,
      `base_extra`, `emit_provider_extension`, `emit_malformed_warning`)
- [x] Convert one parser at a time; stream suite (767 tests) after the batch
- [x] Kill the 7 `emit_provider_extension` copies (7 line-oriented files
      delegate; Antigravity never had the helpers — no line surface)

### Step 6b — Shared summary assembly

- [x] Landed simpler than planned: `StreamExecutionSummary` already
      implements `Default`, so no seed/builder struct was needed — each
      parser's literal keeps only its populated fields plus
      `..Default::default()`, and `common::finish_summary(provider,
      summary)` stamps `provider` + `derive_badges` (provider can no longer
      drift from the badge derivation)
- [x] Providers keep their own counters/state; provider-specific pre-steps
      (kimi flushes, gemini `flush_pending_text`, antigravity buffered
      re-emit + error synthesis, `trace_parser_finish` calls) stay local
- [x] Converted all 8; stream suite green (767 tests)

### Step 6c — Parameterized `classify_error` cascade

- [x] Added `common::ErrorKeywords` (**ordered** bucket tables — order is
      the behavior contract, encoding gemini's config-first kind branch,
      the late-ApiRemote pass, antigravity's auth-first message branch, and
      pi/antigravity's `"abort"` vs the others' `"aborted"`) +
      `classify_error_by_keywords`; `pi.rs`/`antigravity.rs` route through
      the same cascade with empty kind tables
- [x] Kimi's JSON-RPC numeric-code `match` stays local and falls through to
      its message table
- [x] Verified: all `classify_error_*` tests pass unchanged (784-test
      stream+classify run)

### Step 6d — Decision point: generic `feed_line` driver (optional)

Only after 6a–6c, and only for the **7 line-oriented parsers** (Antigravity
excluded by design):

- [x] Re-measured: post-6a each `feed_line` is the 6-line prologue + a
      pure-delegation fallback arm + genuinely provider-specific typed
      dispatch (kimi's envelope classifier and gemini's pre-dispatch flush
      deviate structurally). A driver would save ~14 lines × 6 parsers at
      the cost of generic machinery + per-provider hooks. **C5 closed at
      6c**; driver demoted to Nice-to-Have in `review.md` (decision +
      evidence in `phase6-discovery.md`)
- [x] Not implemented (per the re-measure above)

### Phase 6 Exit Criteria

- [x] `just test` passes for the whole claudine area (3,379 lib + 1,900
      cli + 90 gen + 47 contract + 21 catalog-types)
- [x] `just lint` passes
- [x] Zero copies of `emit_provider_extension` / `emit_malformed_warning` /
      `base_extra` bodies outside `common.rs` (parsers keep 1–3-line
      delegating methods so call sites did not churn)
- [x] One shared summary-assembly helper (`finish_summary`); one shared
      `classify_error` cascade (`classify_error_by_keywords`)
- [x] The 6d decision is recorded in `phase6-discovery.md`
- [x] The dispatch inventory test passes (regenerated via
      `CLAUDINE_UPDATE_INVENTORY=1` — the diff is line-number churn from
      the moves plus 6 new `direct-ref` sites from the keyword-table
      constants; no new decentralized `match Provider` dispatch)

---

## Global Exit Criteria

After all 6 phases:

- [ ] `just test claudine` passes at the repo root
- [ ] `just lint` passes in the claudine area
- [ ] No file in `claudine/lib/src/` or `claudine/cli/src/` exceeds ~1,500
      production lines (excluding generated files:
      `lib/src/signals/generated.rs` and `provider/*/data.rs`)
- [ ] The lifecycle runtime logic has exactly one home in the lib crate
- [ ] The stream parser skeleton is shared, not copy-pasted
- [ ] The test-placement convention is documented and followed
- [ ] `.opencode/skill/claudine/architecture.md` reflects the new module
      structure

## Coordination hazards

The main coordination hazard is in-flight branches touching `lifecycle.rs`
or `loop_control.rs`. Land the directory moves (Phase 3) and the
`run_harness_loop` decomposition (Phase 4) at a quiet point with no active
feature branches on these files. The monorepo has no installed user base yet
(per repo rules, refactoring cost is at its lifetime low).
