---
agent: open_code/zai-coding-plan/glm-5.2
phases: 5
created: 2026-06-14
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/select.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/tests/level2_prompt_reporting_capture.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .opencode/skill/claudine/cli-reference.md
  - .opencode/skill/claudine/architecture.md
source_files_during_phase_3:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/cli/tests/level2_schema_prompt_pty.rs
  - claudine/cli/tests/compose_schema_cli.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/frontmatter-properties.md
  - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .opencode/skill/claudine/cli-reference.md
  - .opencode/skill/claudine/architecture.md
source_code:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/select.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/tests/level2_prompt_reporting_capture.rs
  - claudine/cli/src/commands/sequence.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/wrap_commands.rs
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/cli/tests/level2_schema_prompt_pty.rs
  - claudine/cli/tests/compose_schema_cli.rs
documentation:
  - claudine/docs/topics/frontmatter-properties.md
  - claudine/docs/topics/composition.md
packages:
  - claudine
  - claudine-cli
---

# Execution Plan — Frontmatter-Driven Interactive Sessions & Session-Independent Schema Collection

Source spec: [`spec.md`](spec.md)

## Resolved Open Questions (binding for implementation)

- **OQ1 (Reporting):** Follow the spec recommendation — emit **tracing/dry-run
  metadata now**; defer JSONL schema expansion until a reporting feature needs
  it. No `reporting` migration in this feature.
- **OQ2 (config default):** Follow the spec recommendation — **no** global
  user-config "default interactive" knob. Per-document + per-invocation control
  only.

## Dependency Summary

```
Phase 1 (lib: hint/error/resolution types)
   └── Phase 2 (cli: flag + resolver + 5 wiring sites)
          └── Phase 3 (command-specific behavior — sub-tasks parallelizable)
                 ├── Phase 4 (schema-independence invariant tests)
                 └── Phase 5 (documentation)
```

Phases 4 and 5 are parallelizable with each other once Phase 3 lands.

---

## Phase 1 — Library Foundations: Hint Parsing, Error & Resolution Types

All work in this phase is in `claudine/lib`. No CLI changes. Everything downstream
depends on these types existing.

- [x] Add `CompositionError::InteractiveHintWrongType(String)` to
  `claudine/lib/src/composition/error.rs`, mirroring `AgentHintWrongType`
  (`error.rs:142`). Message: `` `interactive` must be a boolean (true/false), got {0} ``.
  Rendering falls through to the default `BlockError::status_block` `_ =>` arm —
  verify it renders cleanly.

- [x] Add `interactive: Option<bool>` to `EffectiveSelectionHints` in
  `claudine/lib/src/composition/types.rs:429`. Default (`#[derive(Default)]`) already
  yields `None`, preserving today's behavior.

- [x] Add `parse_interactive_hint` in `claudine/lib/src/composition/prepare.rs`,
  alongside `parse_model_hint` (`prepare.rs:401`). Accepts `Bool(b)` → `Some(b)`,
  `Null` → `None` (absent), anything else → `Err(InteractiveHintWrongType)` via the
  existing `json_type_name` helper (`prepare.rs:429`).

- [x] Wire `interactive` into the **three** `EffectiveSelectionHints` construction
  sites so the field is populated everywhere `agent`/`model` are:
  1. `parse_selection_hints_from_frontmatter` (`prepare.rs:308`) — raw frontmatter path
  2. `prepare_direct` (`prepare.rs:153`) — composed (direct) path
  3. `prepare_inline` (`prepare.rs:239`) — composed (inline) path

- [x] Add `SessionInteractivitySource` enum and `ResolvedSessionInteractivity`
  struct to `claudine/lib/src/composition/types.rs`, exactly as specified in the
  spec (`NoInteractiveFlag`, `InteractiveFlag`, `Frontmatter`, `Default`; struct
  holds `value: bool` + `source`). Derive `Debug, Clone, Copy, PartialEq, Eq`.
  Add a `Display` impl for `SessionInteractivitySource` (used by diagnostics).
  Re-export from `composition` mod.

- [x] Add `session_interactive_source: SessionInteractivitySource` field to
  `CompositionExecutionRequest` (`types.rs:590`, next to `session_interactive`).
  This lets the executor and dry-run renderer attribute the resolved mode without
  re-deriving it.

- [x] **Unit tests** (inline `#[cfg(test)]` in `prepare.rs`): `parse_interactive_hint`
  accepts `true`/`false`/absent; treats `null` as absent; rejects string/number/array/
  object with `InteractiveHintWrongType` carrying the correct JSON type name.

**Validation checkpoint:**
```sh
cargo check -p claudine
cargo test  -p claudine composition::prepare
```

---

## Phase 2 — CLI Flag, Resolver & Wiring

Adds `--no-interactive`, the resolution helper, and routes all five request
construction sites through it.

- [x] Add `--no-interactive` flag to `SharedComposeArgs`
  (`claudine/cli/src/commands/compose.rs:102`, next to `interactive`). Mark
  `#[arg(long, conflicts_with = "interactive")]`. clap now rejects `-i` +
  `--no-interactive` at parse time.

- [x] Add resolver method on `SharedComposeArgs`:
  ```rust
  pub(crate) fn resolve_session_interactivity(
      &self,
      frontmatter_interactive: Option<bool>,
  ) -> ResolvedSessionInteractivity
  ```
  Precedence (highest → lowest): `--no-interactive` → `Some(false)`/`NoInteractiveFlag`;
  `-i`/`--interactive` → `Some(true)`/`InteractiveFlag`; frontmatter value →
  `Frontmatter`; else `Default`/`false`. Mirrors the spec pseudocode.

- [x] Wire the resolver into the **four** `CompositionExecutionRequest` sites in
  `compose.rs`, replacing `session_interactive: shared.interactive` with the
  resolved value **and** populating `session_interactive_source`:
  1. compose loop path (`compose.rs:594`)
  2. compose single path (`compose.rs:703`)
  3. inline-compose loop path (`compose.rs:1070`)
  4. inline-compose single path (`compose.rs:1178`)

  Each site reads `prepared.selection_hints.interactive` (effective frontmatter)
  for the frontmatter input. For the loop sites, re-resolve per iteration from the
  freshly-prepared composition.

- [x] Wire the resolver into the **one** sequence-step site in
  `claudine/cli/src/commands/wrap/sequence.rs:562`, populating both fields. For
  sequence, the frontmatter input is `None` when Phase 3b's rejection already
  guarantees no authored `interactive: true` survives — but resolve defensively
  from the prepared hints regardless.

- [x] **Unit tests** (CLI test module or `wrap/profile/mod.rs` probe pattern):
  full precedence table —
  `--no-interactive` wins over `-i` and frontmatter `true`; `-i` wins over
  frontmatter `true`; frontmatter `true` beats default `false`; absent frontmatter
  → `Default`. Mutual-exclusion conflict is exercised via clap parse failure.

**Validation checkpoint:**
```sh
cargo check -p claudine-cli
cargo test  -p claudine-cli --lib
```

---

## Phase 3 — Command-Specific Resolution Behavior

These four sub-tasks touch disjoint files/sections and are **parallelizable**
once Phase 2 lands. Each carries its own tests.

### 3a. Timeout conflict against the resolved session mode

- [x] Enhance the executor timeout-conflict diagnostic at
  `claudine/cli/src/commands/wrap/composition/mod.rs:1519` so the message names
  the resolved source when the conflict fires (e.g. "interactive mode
  (from frontmatter `interactive: true`) cannot be used with --timeout"). Read
  `request.session_interactive_source`.

  Rationale: `request.session_interactive` now carries the **resolved** value, so
  the existing `request.timeout.is_some() && request.session_interactive` guard
  already catches frontmatter-driven conflicts for free. The only new work is the
  source-attributed message. The `--no-interactive` + frontmatter `true` +
  `--timeout` case resolves to non-interactive and is correctly allowed.

- [x] Confirm the **early** entry-point syntax checks (`compose.rs:359`,
  `compose.rs:743`, `sequence.rs:66`) remain as a fast path for the raw
  `-i --timeout` clash. No behavior change required — they are additive to the
  executor check, not the only check.

- [x] **Unit test:** resolved-mode timeout conflict fires for frontmatter
  `interactive: true` + `--timeout`; is suppressed by `--no-interactive`; message
  names the source.

### 3b. Sequence frontmatter rejection

- [x] Add `CompositionError::SequenceInteractiveRejected { source_path: PathBuf }`
  to `claudine/lib/src/composition/error.rs` with a bespoke `BlockError::status_block`
  arm. Diagnostic must: name `interactive`; state a sequence is serial automation;
  point to `compose`/`inline-compose` for dialog-shaped prompts; note that the
  existing `--interactive` CLI flag remains the only explicit override. Exit via the
  default top-level walker.

- [x] In `claudine/cli/src/commands/sequence.rs` `run_sequence_inner`, **after**
  source load (`sequence.rs:86`) and **before** sequence-plan resolution
  (`sequence.rs:95`): parse the raw authored frontmatter `interactive` value
  (reuse `parse_interactive_hint` on `source.markdown.frontmatter()`); hard-error
  with `SequenceInteractiveRejected` when it is `Some(true)`. `false`, `null`, and
  absent are all no-ops.

- [x] **Unit test:** `sequence` on a doc with `interactive: true` → the new
  diagnostic; `interactive: false` / `null` / absent → proceed normally.

### 3c. Inline-compose source-aware guard

- [x] The existing `InlineInteractiveUnsupported` guard
  (`wrap/composition/mod.rs:1102`) already fires on the resolved
  `request.session_interactive`. Enhance its diagnostic to name the resolved
  source (`request.session_interactive_source`) so the remediation distinguishes
  frontmatter-driven from flag-driven interactivity. Update the `error.rs`
  `InlineInteractiveUnsupported` rendering or its call-site message accordingly.

- [x] **Unit test:** frontmatter `interactive: true` on an unsupported inline
  provider surfaces the guard naming `interactive: true` (frontmatter) as the
  source; `-i` flag names the flag.

### 3d. Dry-run metadata — session mode & source

- [x] Add `session_interactive: bool` and `session_source: SessionInteractivitySource`
  to `DryRunRender` (`claudine/cli/src/commands/wrap/composition/dry_run.rs:39`) and
  populate them in `from_request` (`dry_run.rs:72`) from the request.

- [x] Add a **"Session"** row to `render_metadata_table` (`dry_run.rs:208`):
  value shows `interactive` / `non-interactive` with the source in parentheses
  (e.g. `interactive (frontmatter)`, `non-interactive (default)`). Place it after
  the YOLO row, before the conditional Area row.

- [x] **Unit tests** (`dry_run.rs` test module): the row renders for each source
  variant; `--no-interactive` over frontmatter `true` shows `non-interactive
  (--no-interactive)`.

**Validation checkpoint (after 3a–3d):**
```sh
cargo test -p claudine -p claudine-cli
```

---

## Phase 4 — Schema Collection Independence Invariant (Feature 2)

Feature 2 is "mostly already true" — `pre_validate_with_interactive_collection`
already runs before session launch and ignores `session_interactive`. This phase
makes it a **documented, tested invariant** that holds once a document can
self-select interactive mode.

- [x] Add a `///` doc comment to `InteractiveSchemaOptions::allowed()`
  (`claudine/lib/src/composition/schema_validation.rs:87`) asserting the
  invariant: the decision to prompt depends **only** on the four documented
  signals and **must not** depend on the resolved `session_interactive` value.
  Note the ordering guarantee (collection completes before provider spawn).

- [x] **L2 PTY test #1** — `compose -i` with a missing required `string`
  property: the biscuit-tui prompt appears and is collected **before** the
  provider stub launches. Pattern: `claudine/cli/tests/level2_schema_prompt_pty.rs`
  (`require_level!(Level::L2, pty_available(), …)`).

- [x] **L2 PTY test #2** — `compose` on a doc with `interactive: true`
  frontmatter (no `-i`): same assertion — prompt precedes session start.

- [x] **L2 PTY test #3** — `--no-interactive` on an `interactive: true` doc:
  the session is non-interactive, **but** the schema prompt still appears under a
  TTY (proving the gate is mode-independent).

- [x] **L1 non-TTY test** — an `interactive: true` doc with a missing required
  property, piped stdin/stderr: emits the typed `MissingProperties` report (no
  hang, no prompt). Pattern: `claudine/cli/tests/compose_schema_cli.rs`.

**Validation checkpoint:**
```sh
just test-l2
cargo test -p claudine-cli --test compose_schema_cli
```

---

## Phase 5 — Documentation

Parallelizable with Phase 4 once Phase 3 is complete.

- [x] Add an `interactive` row to the **Composition Core** table in
  `claudine/docs/topics/frontmatter-properties.md`: boolean, default `false`,
  sets the default session mode for `compose`/`inline-compose`; hard-rejected for
  `sequence`. Link the new symbols (`SessionInteractivitySource`,
  `ResolvedSessionInteractivity`, `EffectiveSelectionHints.interactive`,
  `parse_interactive_hint`, `CompositionError::InteractiveHintWrongType`).

- [x] Update `claudine/docs/topics/composition.md`:
  - **`--interactive` section** — document the four-tier precedence, the new
    `--no-interactive` escape hatch, and that `-i`/`--no-interactive` are mutually
    exclusive.
  - **Schema Validation section** — state the independence invariant: collection
    precedes session launch regardless of resolved session mode.
  - **Dry Run section** — note the new Session metadata row and what its source
    values mean.

- [x] Update the **claudine skill** (`.opencode/skill/claudine/`): add the
  `--no-interactive` flag and `interactive` frontmatter property to
  `cli-reference.md` and `architecture.md`; note the sequence rejection.

- [x] Update `AGENTS.md` only if a repo-wide convention changed (none expected
  here — this is feature-scoped).

**Validation checkpoint:**
```sh
just doctest
```

---

## Cross-Cutting Validation (run before declaring complete)

- [x] `just test` passes (curated area list).
- [x] `just lint` passes (clippy clean, no new warnings).
- [x] Backward-compat spot checks: a document **without** `interactive` behaves
  identically to before (default `false`); `-i` keeps its exact prior meaning.
- [x] `--dry-run` on an `interactive: true` doc shows the resolved mode/source
  in the metadata table and launches no provider.
