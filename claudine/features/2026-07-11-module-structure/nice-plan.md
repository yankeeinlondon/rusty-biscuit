---
agent: open_code/zai-coding-plan/glm-5.2
total_phases: 6
created: 2026-07-11
start_phase: 1
prerequisite: critical-plan.md (C1–C6) AND strong-plan.md (S1–S9) fully landed
source_code:
  # Phase 1 — N1 adapters rename
  - claudine/lib/src/lib.rs
  - claudine/lib/src/adapters/mod.rs
  - claudine/lib/src/adapters/claude.rs
  - claudine/lib/src/adapters/codex.rs
  - claudine/lib/src/adapters/gemini.rs
  - claudine/lib/src/adapters/goose.rs
  - claudine/lib/src/adapters/kimicode.rs
  - claudine/lib/src/adapters/opencode.rs
  - claudine/lib/src/adapters/qwen.rs
  - claudine/lib/src/adapters/pi.rs
  - claudine/lib/src/adapters/antigravity.rs
  - claudine/lib/src/hook_adapters/mod.rs
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/error.rs
  - claudine/lib/src/provider/behavior.rs
  - claudine/lib/src/provider/claude/behavior.rs
  - claudine/lib/src/provider/codex/behavior.rs
  - claudine/lib/src/provider/gemini/behavior.rs
  - claudine/lib/src/provider/goose/behavior.rs
  - claudine/lib/src/provider/kimi/behavior.rs
  - claudine/lib/src/provider/kilo/behavior.rs
  - claudine/lib/src/provider/opencode/behavior.rs
  - claudine/lib/src/provider/pi/behavior.rs
  - claudine/lib/src/provider/qwen/behavior.rs
  - claudine/lib/src/provider/antigravity/behavior.rs
  # Phase 2 — N4 prepare hints carve-out
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/hints.rs
  - claudine/lib/src/composition/mod.rs
  # Phase 3 — N5 loop types/seeds hive-off (post-S4: composition/looping/)
  - claudine/lib/src/composition/looping/engine.rs
  - claudine/lib/src/composition/looping/types.rs
  - claudine/lib/src/composition/looping/seed.rs
  - claudine/lib/src/composition/mod.rs
  # Phase 4 — N2 mod.rs hygiene
  - claudine/lib/src/linking/compatibility/mod.rs
  - claudine/lib/src/linking/compatibility/classify.rs
  - claudine/lib/src/linking/compatibility/frontmatter_io.rs
  - claudine/lib/src/linking/compatibility/properties.rs
  - claudine/lib/src/render/event_renderer/mod.rs
  - claudine/lib/src/render/event_renderer/helpers.rs
  # Phase 5 — N3 CLI reporting/completion splits
  - claudine/cli/src/commands/context.rs
  - claudine/cli/src/commands/context/expressions.rs
  - claudine/cli/src/commands/context/effects.rs
  - claudine/cli/src/commands/context/format.rs
  - claudine/cli/src/completion/schema_completion.rs
  - claudine/cli/src/completion/schema_completion/keys.rs
  - claudine/cli/src/completion/schema_completion/candidates.rs
  - claudine/cli/src/commands/schema_interactive.rs
  - claudine/cli/src/commands/schema_interactive/status.rs
  - claudine/cli/src/completion/engine.rs
  - claudine/cli/src/completion/engine/tokens.rs
  # Phase 6 — N6 relocate protocol fixture-replay tests
  - claudine/lib/src/stream/protocol/kimi.rs
  - claudine/lib/src/stream/protocol/codex.rs
  - claudine/lib/tests/protocol_fixture_replay.rs
documentation:
  - .opencode/skill/claudine/architecture.md
packages:
  - claudine
  - claudine-cli
source_files_during_phase_1:
  - claudine/lib/src/lib.rs
  - claudine/lib/src/hook_adapters/mod.rs
  - claudine/lib/src/hook_adapters/claude.rs
  - claudine/lib/src/hook_adapters/codex.rs
  - claudine/lib/src/hook_adapters/gemini.rs
  - claudine/lib/src/hook_adapters/goose.rs
  - claudine/lib/src/hook_adapters/kimicode.rs
  - claudine/lib/src/hook_adapters/opencode.rs
  - claudine/lib/src/hook_adapters/qwen.rs
  - claudine/lib/src/hook_adapters/pi.rs
  - claudine/lib/src/hook_adapters/antigravity.rs
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/error.rs
  - claudine/lib/src/provider/behavior.rs
  - claudine/lib/src/provider/claude/behavior.rs
  - claudine/lib/src/provider/codex/behavior.rs
  - claudine/lib/src/provider/gemini/behavior.rs
  - claudine/lib/src/provider/goose/behavior.rs
  - claudine/lib/src/provider/kimi/behavior.rs
  - claudine/lib/src/provider/kilo/behavior.rs
  - claudine/lib/src/provider/opencode/behavior.rs
  - claudine/lib/src/provider/pi/behavior.rs
  - claudine/lib/src/provider/qwen/behavior.rs
  - claudine/lib/src/provider/antigravity/behavior.rs
  - claudine/lib/tests/diagnostic_detail_conformance.rs
  - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_1:
  - claudine/lib/README.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/SKILL.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_2:
  - claudine/lib/src/composition/hints.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/looping/types.rs
  - claudine/lib/src/composition/looping/seed.rs
  - claudine/lib/src/composition/looping/engine.rs
  - claudine/lib/src/composition/looping/engine/tests.rs
  - claudine/lib/src/composition/looping/mod.rs
docs_updated_during_phase_3:
  - .claude/skills/claudine/architecture.md
  - .opencode/skill/claudine/architecture.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/architecture.md
  - .opencode/skill/claudine/architecture.md
source_files_during_phase_4:
  - claudine/lib/src/linking/compatibility/mod.rs
  - claudine/lib/src/linking/compatibility/classify.rs
  - claudine/lib/src/linking/compatibility/frontmatter_io.rs
  - claudine/lib/src/linking/compatibility/properties.rs
  - claudine/lib/src/render/event_renderer/mod.rs
  - claudine/lib/src/render/event_renderer/helpers.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/context/mod.rs
  - claudine/cli/src/commands/context/format.rs
  - claudine/cli/src/commands/context/expressions.rs
  - claudine/cli/src/commands/context/effects.rs
  - claudine/cli/src/commands/context/tests.rs
  - claudine/cli/src/completion/schema_completion/mod.rs
  - claudine/cli/src/completion/schema_completion/keys.rs
  - claudine/cli/src/completion/schema_completion/candidates.rs
  - claudine/cli/src/completion/schema_completion/tests.rs
  - claudine/cli/src/commands/schema_interactive/mod.rs
  - claudine/cli/src/commands/schema_interactive/status.rs
  - claudine/cli/src/completion/engine/mod.rs
  - claudine/cli/src/completion/engine/tokens.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/lib/src/stream/protocol/kimi.rs
  - claudine/lib/tests/protocol_fixture_replay.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine
packages:
  - claudine
  - claudine-cli
---

# Nice-to-Have Module-Structure Refactor — Execution Plan

Converts the six **Nice to Have** findings (N1–N6) from
[`review.md`](review.md) into a dependency-ordered, high-confidence plan that
executes **after** [`critical-plan.md`](critical-plan.md) (C1–C6) **and**
[`strong-plan.md`](strong-plan.md) (S1–S9) are fully landed.

These are the lowest-urgency items. None is correctness-critical; each is a
hygiene improvement that pays off as the area grows. Every phase is observable
through a compile, a `just test` run, or a `just lint` pass, and no phase
changes runtime behavior.

## Scope notes

- **N1** is a pure rename — the most mechanical, broadest-blast-radius item, so
  it goes first to settle naming before later phases touch the same files.
- **N5** is unblocked only because **S4** (the `looping/` directory) is landed;
  it operates on `composition/looping/engine.rs` (the post-S4 location of
  `loop_engine.rs`).
- **N4** operates on `composition/prepare.rs`, which S7 already touched only to
  dedupe `json_type_name` — the hint-parsing cluster is intact and separable.
- **N6** carries the review's own caveat ("reasonable to co-locate"). It is the
  one phase where the move is a judgment call, not a clear win; the phase
  front-loads a go/no-go decision.

## Dependency graph

```
Phase 1 (N1) ── independent (rename, lib-wide)
Phase 2 (N4) ── independent (composition/prepare.rs)
Phase 3 (N5) ── depends on S4 landed (looping/ directory exists)
Phase 4 (N2) ── independent (linking/ + render/)
Phase 5 (N3) ── independent (CLI commands + completion)
Phase 6 (N6) ── independent (stream/protocol tests)
```

All six phases are mutually independent once the two prerequisite plans are
landed. The ordering is by ascending blast radius / risk: rename → single-file
carve-outs → module hygiene → CLI splits → test relocation. Any phase may be
deferred or dropped independently without affecting the others.

## Verified structural facts driving this plan

1. **`adapters/` has 13 external importers, all mechanical.** Verified files
   outside `adapters/` that reference `crate::adapters`: `dispatch/mod.rs`,
   `error.rs`, and the eleven `provider/{claude,codem,gemini,goose,kimi,kilo,
   opencode,pi,qwen,antigravity}/behavior.rs` plus `provider/behavior.rs`. The
   public surface is exactly three items (`adapters/mod.rs:20` `AdapterError`
   enum, `:53` `ProviderAdapter` trait, `:123` `adapter_for` fn) plus the static
   adapter consts (`GOOSE_ADAPTER`, `OPENCODE_ADAPTER`, etc.). A rename is a
   directory move + find/replace `crate::adapters` → `crate::hook_adapters` +
   `pub mod` re-declaration in `lib.rs:2`.

2. **`adapters/` and `stream/providers/` have no responsibility overlap.** The
   review verified this. `adapters/` parses native hook request/response
   payloads; `stream/providers/` parses stdout NDJSON. The name collision is
   purely cosmetic but it misleads — docs and the skill text sometimes call
   stream parsers "adapters." Renaming to `hook_adapters/` makes the split
   unambiguous. (`hooks/` is avoided: it collides conceptually with the
   `claudine hooks` command surface and the `hooks/` research docs.)

3. **`linking/compatibility/mod.rs` is 1,186 lines with one submodule**
   (`pub mod table;` at 12) and a `mod tests` at 648 — so ~640 production lines
   in the module root. The logic falls into three clean clusters:
   - **Classification**: `classify_canonical_candidate` (30),
     `classify_target_reference` (85), `canonical_definition` (129),
     `canonical_file_path` (138), `inferred_name` (433), `valid_slug` (452).
   - **Frontmatter YAML I/O**: `parse_frontmatter_mapping` (191),
     `parse_frontmatter_lines` (213), `frontmatter_bounds` (270),
     `yaml_indentation_has_tabs` (318), `normalize_yaml_indentation_tabs` (328),
     `write_markdown_document` (366), `FrontmatterBounds` (22).
   - **Property satisfaction**: `property_alias_groups` (477),
     `missing_required_for_all_providers` (506), `property_is_satisfied` (529),
     `mapping_to_string_map` (544), `yaml_value_to_string` (555),
     `get_frontmatter_value` (574), `frontmatter_has_value` (581),
     `yaml_value_has_data` (587), `hash_frontmatter` (598), `normalize_key`
     (609), `apply_alias_duplication` (383), `apply_name_derivation` (411).

4. **`render/event_renderer/mod.rs` is 486 lines and ALREADY has three
   submodules** (`error_block`, `provider_extension`, `tool_use` at 18–20). The
   root holds `RenderUnit`/`Disposition` types (41, 57), the `EventRenderer`
   struct + impl (94–435), one free helper `subagent_description` (436), and
   `mod tests` (463). This file is the **borderline** case the review
   flags: the `EventRenderer` impl is a cohesive state machine, not a grab-bag.
   The realistic carve is limited to extracting the self-contained free helpers
   and any large leaf render methods — modest upside. `provider/mod.rs` and
   `reporting/mod.rs` are the clean-root templates.

5. **`commands/context.rs` (1,286) has a `#[cfg(test)]` at line 29** (on a
   test-support item, not a test module) **and a second `#[cfg(test)]` test
   module later** — the review flags two separate test blocks to merge. The
   function clusters are confirmed: value/formatting helpers (112–204:
   `display_property`, `format_context_value_type`, `format_value`,
   `format_array_element`, `format_safety`, `format_example`), the nine
   `render_expressions_*` fns (460–717), and the side-effects report (`render_
   side_effects_report` 717+).

6. **`prepare.rs` hint parsing is a clean cluster at 467–620.** Confirmed:
   `parse_selection_hints_from_frontmatter` (480), `ParsedAgentHint` (508),
   `parse_agent_hint_full`, `parse_model_hint`, `parse_interactive_hint`, and
   the `to_agent_hint` conversion. It is consumed at two prepare call sites
   (213, 370) plus the public entry point. Distinct from prompt preparation.

7. **`loop_engine.rs` type/seed block is 29–370 (post-S4: `looping/engine.rs`).**
   Confirmed: `LoopExecutionOptions` (29), `LoopIterationContext` (64),
   `LoopIterationOutput` (88), `LoopExecutionResult` (189), `build_loop_seed`
   (276), `LoopSeed` (294), `build_loop_seed_with_lifecycle` (317). The engine
   proper (`execute_loop*`, `route_init_failure*`, `run_loop_gate`) starts at
   371. Clean seam at line 370.

8. **Protocol fixture-replay tests are dense and fixture-backed.** Confirmed:
   `protocol/kimi.rs` is 1,981 lines with `mod tests` at 1,031 (~48% tests);
   `protocol/codex.rs` is 1,146 lines (~37% tests). Fixtures live in
   `stream/protocol/fixtures/{kimi,codex}/*.jsonl` (verified:
   `wire-auth-expired.jsonl`, `wire-greet.jsonl`, `wire-protocol-110.jsonl`,
   etc.). The tests are deserialization-fidelity checks over that corpus.

---

## Phase 1 — N1: Rename `adapters/` → `hook_adapters/`

**Goal:** Eliminate the name collision between hook-request adapters
(`adapters/`) and stream parsers (`stream/providers/`) by renaming the
directory to `hook_adapters/`. Pure naming change — no behavior, no API shape.

**Risk:** Low. Broad but mechanical: 13 importers + the `lib.rs` declaration +
the directory itself. No public-API rename (the crate does not re-export the
`adapters` module path as a public surface — consumers use the `ProviderAdapter`
trait via `provider::behavior`).

### Step 1.1 — Move the directory and fix the module declaration

**Files:**
- Move: `claudine/lib/src/adapters/` → `claudine/lib/src/hook_adapters/`
- Modify: `claudine/lib/src/lib.rs`

- [x] `git mv claudine/lib/src/adapters claudine/lib/src/hook_adapters`
- [x] In `lib.rs:2` change `pub mod adapters;` → `pub mod hook_adapters;`
- [x] Verify the crate still compiles (errors will point at the 13 importers)

### Step 1.2 — Update the 13 external importers

**Files (all `crate::adapters` → `crate::hook_adapters`):**
- `claudine/lib/src/dispatch/mod.rs`
- `claudine/lib/src/error.rs`
- `claudine/lib/src/provider/behavior.rs`
- `claudine/lib/src/provider/{claude,codex,gemini,goose,kimi,kilo,opencode,pi,
  qwen,antigravity}/behavior.rs`

- [x] Find/replace `crate::adapters` → `crate::hook_adapters` in each
- [x] Update any doc comments / `///` references that spell out the
      `adapters::` path
- [x] Verify: `just test` + `just lint`

### Step 1.3 — Update internal cross-references and docs

**Files:**
- Within `hook_adapters/mod.rs`: confirm no internal `use crate::adapters`
      self-references remain (the module uses `use crate::…` for its
      dependencies, which are unaffected)
- `.opencode/skill/claudine/architecture.md`: the module map lists `adapters`
      as "Provider-specific event parsers" — rename to `hook_adapters` and
      sharpen the description to "native hook request/response adapters"
- Any topic doc that calls stream parsers "adapters" — leave for a separate
      doc pass (out of scope for a code move), but add a one-line note in the
      architecture doc that `hook_adapters/` ≠ `stream/providers/`

- [x] Verify: `just test` + `just lint`

### Phase 1 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] No `crate::adapters` reference remains (`rg "crate::adapters"` returns
      nothing outside `hook_adapters/` itself)
- [x] `architecture.md` reflects the new name

---

## Phase 2 — N4: Carve Hint Parsing out of `prepare.rs`

**Goal:** Move the selection-hint parsing cluster (lines ~467–620) from
`composition/prepare.rs` into `composition/hints.rs`. Prompt preparation and
hint parsing are distinct concerns.

**Risk:** Low. The hint parsers are called at two prepare sites (213, 370) and
exposed via one public entry point (`parse_selection_hints_from_frontmatter`).
A `pub mod hints;` declaration + re-export keeps the public surface identical.

### Step 2.1 — Extract the hint cluster into `composition/hints.rs`

**Files:**
- Modify: `claudine/lib/src/composition/prepare.rs`
- Create: `claudine/lib/src/composition/hints.rs`
- Modify: `claudine/lib/src/composition/mod.rs`

Move from `prepare.rs`:
- `parse_selection_hints_from_frontmatter` (480)
- `ParsedAgentHint` struct + impl (508+) and its `to_agent_hint` conversion
- `parse_agent_hint_full`, `parse_model_hint`, `parse_interactive_hint`
- Any hint-specific error helpers and the `EffectiveSelectionHints` assembly
  if it is local to the hint cluster (if `EffectiveSelectionHints` is defined
  elsewhere and only assembled here, keep the assembly call site in `prepare.rs`
  and move only the parsing fns)

- [x] Create `hints.rs` with `use super::*;` (or targeted imports for
      `CompositionError`, the `AgentHint`/`ModelHint`/`InteractiveHint` types,
      and the frontmatter `Map` accessors)
- [x] In `prepare.rs`, replace the two inline call sites (213, 370) with calls
      into `super::hints::…` (or keep the calls identical if the items are
      re-exported)
- [x] Declare `pub mod hints;` in `composition/mod.rs`; re-export
      `parse_selection_hints_from_frontmatter` if it is currently public via
      the `prepare` barrel
- [x] Verify the public barrel (`composition/mod.rs:82` `pub use prepare::{…}`)
      still exports the same names
- [x] Verify: `just test` + `just lint`

### Phase 2 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] `prepare.rs` no longer contains hint-parsing logic
- [x] `hints.rs` holds the hint cluster
- [x] No public-API name changed

---

## Phase 3 — N5: Hive Off Loop Types and Seeds

**Goal:** Split the result/option/context types (lines 29–276) and seed
building (276–370) out of the loop engine proper into `looping/types.rs` and
`looping/seed.rs`. The engine (`execute_loop*`, routing, gate) is the only
thing that should remain in `engine.rs`.

**Risk:** Low. Pure structural move within the already-grouped `looping/`
directory (S4 landed). The types are consumed through the barrel.

**Prerequisite:** S4 complete (`composition/looping/` exists; the file is
`looping/engine.rs`).

### Step 3.1 — Carve types into `looping/types.rs`

**Files:**
- Modify: `claudine/lib/src/composition/looping/engine.rs`
- Create: `claudine/lib/src/composition/looping/types.rs`
- Modify: `claudine/lib/src/composition/looping/mod.rs`

Move from `engine.rs` (lines 29–188):
- `LoopExecutionOptions` (29)
- `LoopIterationContext` (64) + impl (73)
- `LoopIterationOutput` (88) + impl (124)
- `LoopExecutionResult` (189) + impl (207)

- [x] `types.rs` starts with `use super::*;` for shared imports; make any
      cross-references (`LoopAmbient`, `LoopAction`, etc.) explicit
- [x] `engine.rs` declares `use super::types::*;` (or `mod types;` in
      `looping/mod.rs` and import from there)
- [x] `looping/mod.rs` re-exports the types so the `composition` barrel
      (`pub use looping::{…}`) is unchanged
- [x] Verify: `just test` + `just lint`

### Step 3.2 — Carve seed building into `looping/seed.rs`

**Files:**
- Modify: `claudine/lib/src/composition/looping/engine.rs`
- Create: `claudine/lib/src/composition/looping/seed.rs`

Move from `engine.rs` (lines 276–370):
- `build_loop_seed` (276)
- `LoopSeed` (294)
- `build_loop_seed_with_lifecycle` (317)

- [x] `seed.rs` imports the types from `super::types`; the engine calls
      `super::seed::build_loop_seed(…)`
- [x] Re-export through `looping/mod.rs` so the barrel is unchanged
- [x] Verify: `just test` + `just lint`

### Step 3.3 — Confirm `engine.rs` is the engine only

- [x] `engine.rs` now starts at the `execute_loop` family (line ~371
      pre-carve) and holds only the execution/routing/gate logic
- [x] Update `.opencode/skill/claudine/architecture.md` module map to note
      `looping/{engine,types,seed,config,dsl,actions,expression}.rs`
- [x] Verify: `just test` + `just lint`

### Phase 3 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] `engine.rs` contains only execution/routing/gate logic
- [x] `types.rs` and `seed.rs` hold the carved blocks
- [x] No public-API name changed (barrel unchanged)

---

## Phase 4 — N2: `mod.rs` Hygiene for `linking/compatibility/` and `render/event_renderer/`

**Goal:** Move computation out of two logic-in-root `mod.rs` files into named
submodules, leaving declaration/re-export roots — matching the
`provider/mod.rs` and `reporting/mod.rs` templates.

**Risk:** Low for `linking/compatibility/` (clean three-way seams). **Lower
upside** for `render/event_renderer/` (already has three submodules; the root
`EventRenderer` impl is cohesive) — the step is scoped to the self-contained
helpers only.

### Step 4.1 — Split `linking/compatibility/mod.rs` into three submodules

**Files:**
- Modify: `claudine/lib/src/linking/compatibility/mod.rs`
- Create: `claudine/lib/src/linking/compatibility/classify.rs`
- Create: `claudine/lib/src/linking/compatibility/frontmatter_io.rs`
- Create: `claudine/lib/src/linking/compatibility/properties.rs`

Keep in `mod.rs`: the `pub mod table;` declaration (12), the new `mod`/`pub use`
declarations, and the `FrontmatterBounds` struct if it is shared across
clusters (otherwise move it to `frontmatter_io.rs`).

Move:
- → `classify.rs`: `classify_canonical_candidate` (30),
      `classify_target_reference` (85), `canonical_definition` (129),
      `canonical_file_path` (138), `inferred_name` (433), `valid_slug` (452)
- → `frontmatter_io.rs`: `parse_frontmatter_mapping` (191),
      `parse_frontmatter_lines` (213), `frontmatter_bounds` (270),
      `yaml_indentation_has_tabs` (318), `normalize_yaml_indentation_tabs`
      (328), `write_markdown_document` (366)
- → `properties.rs`: `property_alias_groups` (477),
      `missing_required_for_all_providers` (506), `property_is_satisfied`
      (529), `apply_alias_duplication` (383), `apply_name_derivation` (411),
      and the YAML value/string helpers (`mapping_to_string_map`,
      `yaml_value_to_string`, `get_frontmatter_value`, `frontmatter_has_value`,
      `yaml_value_has_data`, `hash_frontmatter`, `normalize_key`)

- [x] Each submodule starts with `use super::*;`; cross-cluster references go
      through `pub(super)`
- [x] `mod.rs` re-exports the public entry points (`classify_canonical_candidate`,
      `classify_target_reference`) so consumers of `linking::compatibility::…`
      are unchanged
- [x] The `mod tests` (648) stays in `mod.rs` or splits per submodule —
      prefer leaving it in `mod.rs` (it likely exercises the public entry
      points end-to-end); if a test is unit-scoped to a helper, move it with
      the helper
- [x] Verify: `just test` + `just lint`

### Step 4.2 — Modest carve for `render/event_renderer/mod.rs`

**Files:**
- Modify: `claudine/lib/src/render/event_renderer/mod.rs`
- Create: `claudine/lib/src/render/event_renderer/helpers.rs`

This file already has three submodules (`error_block`, `provider_extension`,
`tool_use`). The realistic move is limited:

- [x] Move the free helper `subagent_description` (436) and any other
      self-contained free render helpers into `helpers.rs`
- [x] **Do not** force-split the `EventRenderer` impl (94–435) — it is a
      cohesive state machine; splitting it would create artificial coupling,
      the opposite of the hygiene goal
- [x] If, after moving the free helpers, `mod.rs` is still > ~400 logic lines,
      consider extracting the largest self-contained render *methods* (e.g. a
      tool-use rendering block) into an existing submodule — but only if the
      extraction is clean. If not, stop: the file already meets the "named
      submodules" bar better than `compatibility/mod.rs` did
- [x] Verify: `just test` + `just lint`

### Phase 4 Exit Criteria

- [x] `just test` + `just lint` pass
- [x] `linking/compatibility/mod.rs` is a declaration/re-export root (~40
      lines of declarations/re-exports + co-located test module, down from
      1,186)
- [x] `render/event_renderer/mod.rs` free helpers are extracted; the impl is
      left cohesive
- [x] No public-API name changed

---

## Phase 5 — N3: CLI Reporting/Completion Splits

**Goal:** Split four CLI files that each mix separable rendering/reporting
concerns. The largest is `commands/context.rs`.

**Risk:** Low-medium. CLI-internal; no public API. Each split is along
existing function clusters. The `context.rs` test-block merge is the one
non-mechanical step.

### Step 5.1 — Split `commands/context.rs` into a `context/` directory

**Files:**
- Modify: `claudine/cli/src/commands/context.rs` → `context/mod.rs`
- Create: `claudine/cli/src/commands/context/expressions.rs`
- Create: `claudine/cli/src/commands/context/effects.rs`
- Create: `claudine/cli/src/commands/context/format.rs`

Move from `context.rs` (1,286 lines):
- → `expressions.rs`: the nine `render_expressions_*` fns (460–717) plus
      `render_expression_table` (415) and `show_examples` (403)
- → `effects.rs`: `render_side_effects_report` (717+) and
      `group_effect_descriptors` (94)
- → `format.rs`: the value/formatting helpers (112–204:
      `display_property`, `format_context_value_type`,
      `context_value_type_markup`, `format_value`, `format_array_element`,
      `format_safety`, `format_example`, `format_effect_example`)

Keep in `mod.rs`: `ContextArgs` (36), `render_default_report` (204),
`render_values_report` / `render_values_report_with` (259/269),
`render_expressions_report` (351) entry point, `group_context_descriptors`
(54), `group_expression_descriptors` (71), and the `mod`/`pub use`
declarations.

- [x] **Merge the two `#[cfg(test)]` blocks** (review: lines 29 and 869).
      The line-29 `#[cfg(test)]` is on a test-support item; fold it into the
      single test module (move to `context/tests.rs` per the C1 convention, or
      consolidate at the end of `mod.rs`)
- [x] Each submodule uses `use super::*;` for shared types (`Terminal`,
      `ContextValueType`, etc.)
- [x] Verify: `just test` + `just lint`

### Step 5.2 — Split `completion/schema_completion.rs`

**Files:**
- Modify: `claudine/cli/src/completion/schema_completion.rs` →
      `schema_completion/mod.rs`
- Create: `claudine/cli/src/completion/schema_completion/keys.rs`
- Create: `claudine/cli/src/completion/schema_completion/candidates.rs`

Split the two separable clusters (1,438 lines):
- → `keys.rs`: schema-key extraction (walking the Darkmatter schema descriptor
      to enumerate completable keys)
- → `candidates.rs`: file-candidate matching (resolving `@`-style file/path
      candidates)

- [x] Keep the public completion entry point and the dispatch glue in `mod.rs`
- [x] Verify: `just test` + `just lint`

### Step 5.3 — Split `commands/schema_interactive.rs`

**Files:**
- Modify: `claudine/cli/src/commands/schema_interactive.rs` →
      `schema_interactive/mod.rs`
- Create: `claudine/cli/src/commands/schema_interactive/status.rs`

Split (1,135 lines):
- → `status.rs`: status rendering (57–154)
- Keep in `mod.rs`: the interactive collection loop (463+) and its entry point

- [x] If the interactive-collection cluster is itself large (> ~500 lines),
      consider a second `collect.rs` submodule; otherwise leave it in `mod.rs`
      (~350-line collection cluster is below the threshold — left in `mod.rs`)
- [x] Verify: `just test` + `just lint`

### Step 5.4 — Carve token predicates out of `completion/engine.rs`

**Files:**
- Modify: `claudine/cli/src/completion/engine.rs` → `engine/mod.rs`
- Create: `claudine/cli/src/completion/engine/tokens.rs`

- [x] Move the token-predicate cluster (561–611) into `tokens.rs`
- [x] The review frames this as conditional ("if it grows further") — at 1,211
      lines the file is borderline; do the carve only if the predicates are a
      clean, self-contained cluster. If they are tangled with the engine core,
      defer this step and leave a note
      (the six token-shape predicates — `split_setter`, `is_setter_name_partial`,
      `is_flag_token`, `is_value_bearing_flag`, `is_setter_shaped`,
      `is_global_bool_flag` — are pure `&str` classifiers with no engine-state
      coupling: a clean cluster, carved into `engine/tokens.rs`)
- [x] Verify: `just test` + `just lint`

### Phase 5 Exit Criteria

- [x] `just test` + `just lint` pass for claudine-cli
- [x] `context.rs` is a `context/` directory; its two test blocks are merged
- [x] `schema_completion.rs`, `schema_interactive.rs`, and `engine.rs` each
      delegate a separable cluster to a named submodule
- [x] No CLI command behavior changed

---

## Phase 6 — N6: Relocate Protocol Fixture-Replay Tests

**Goal:** Move the deserialization-fidelity tests from `stream/protocol/kimi.rs`
(~48% tests) and `stream/protocol/codex.rs` (~37% tests) into an integration
test so the protocol model compile units shrink.

**Risk:** Low. **This is the judgment-call phase** — the review itself notes
"reasonable to co-locate." The models are legitimately large and hand-written;
moving tests is a compile-unit optimization, not a correctness fix. Front-load
a go/no-go decision.

### Step 6.0 — Go/no-go decision

- [x] Confirm the goal is still worth it: the protocol modules are compiled
      twice today (once as a lib unit, once as a test unit). Moving the
      fixture-replay tests to `claudine/lib/tests/` (a separate integration
      test binary) shrinks the lib's test build. **If** the compile-time win
      is negligible on this machine, **skip this phase** and leave the tests
      co-located — the review explicitly allows this.
      **Decision: GO for kimi, no-op for codex.** `kimi.rs` carries 9 genuine
      fixture-corpus replay tests (runtime `classify_lines` reads over
      `fixtures/kimi/*.jsonl`, ~250 test lines + 3 helpers) that move cleanly
      into a shared integration binary reusing the already-compiled lib rlib —
      every referenced type/field/method is already `pub`, so no API widening.
      `codex.rs` has **no fixture corpus** (verified: no `fixtures/codex/`
      directory); all its `mod tests` are inline JSON-literal deserialization
      unit tests, which the plan itself says to keep co-located — so `codex.rs`
      is left unchanged.

### Step 6.1 — Move fixture-replay tests to an integration test

**Files:**
- Modify: `claudine/lib/src/stream/protocol/kimi.rs`
- Modify: `claudine/lib/src/stream/protocol/codex.rs`
- Create: `claudine/lib/tests/protocol_fixture_replay.rs`

- [x] Identify the fixture-replay tests in each `mod tests` block (the
      deserialization-fidelity cases reading `fixtures/{kimi,codex}/*.jsonl`)
      vs. the genuinely unit-scoped helper tests. Move **only** the
      fixture-replay cases; keep helper unit tests co-located
      (kimi: 9 `classify_lines` corpus tests moved; kimi's 2 single-line
      `include_str!` decode tests + all inline-literal tests kept; codex: no
      corpus, nothing to move)
- [x] Create `claudine/lib/tests/protocol_fixture_replay.rs` as an integration
      test that `use claudine::stream::protocol::{kimi, codex};` and replays
      the same corpus with the same assertions
      (kimi only — glob-imports `claudine::stream::protocol::kimi::*`; codex has
      no corpus so is not imported)
- [x] The fixture path must resolve from the integration-test working dir
      (`CARGO_MANIFEST_DIR`-relative) — adjust the path helper so it points at
      `src/stream/protocol/fixtures/`
- [x] Verify: `just test` (the new integration test passes; the protocol
      modules' test count drops accordingly)

### Phase 6 Exit Criteria

- [x] `just test` passes (including the new integration test —
      `protocol_fixture_replay`, 9 tests)
- [x] `protocol/kimi.rs` and `protocol/codex.rs` hold only their models +
      co-located unit helper tests (kimi's corpus replay moved out; kimi keeps
      its inline-literal + single-line `include_str!` unit tests; codex was
      already models + inline-literal tests, left unchanged)
- [x] The fixture corpus is exercised exactly once at the typed-model layer,
      from the integration test (the semantic-parser-layer replay in
      `kimi_wire.rs` is a distinct layer and is unaffected)
- [ ] **Or:** the phase was skipped at Step 6.0 with a recorded rationale

---

## Global Exit Criteria

After all 6 phases:

- [x] `just test claudine` passes (verified via `just test` in the claudine
      area — same curated package set: claudine, claudine-cli, claudine-contract,
      catalog-types, claudine-gen, all green)
- [x] `just lint` passes in the claudine area
- [x] `adapters/` is renamed to `hook_adapters/`; no stale `crate::adapters`
      reference remains (only historical `_completed` spec + this plan's prose
      mention the old path)
- [x] `composition/prepare.rs` no longer holds hint parsing; `composition/
      hints.rs` does
- [x] `composition/looping/engine.rs` holds only engine logic; types and seeds
      are in `looping/types.rs` / `looping/seed.rs`
- [x] `linking/compatibility/mod.rs` is a declaration root; `render/
      event_renderer/mod.rs` free helpers are extracted
- [x] The four CLI files (`context.rs`, `schema_completion.rs`,
      `schema_interactive.rs`, `engine.rs`) each delegate a separable cluster
      to a named submodule
- [x] `.opencode/skill/claudine/architecture.md` reflects every rename/new
      directory

## Coordination hazards

All six phases are independent and may be cherry-picked individually. Phase 1
(rename) is the only one with broad blast radius — land it when no active
branch touches `provider/*/behavior.rs`. Phases 2–6 each touch a disjoint file
set and can proceed in parallel. Because these are nice-to-haves with no
correctness pressure, each is safely deferrable: if a phase conflicts with
in-flight feature work, skip it and revisit later — the cost is zero (the
current code works).
