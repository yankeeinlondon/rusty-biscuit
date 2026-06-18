---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-18
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - claudine/lib/src/composition/loop_config.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/compose.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/context.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/loop_actions.rs
  - claudine/lib/src/composition/select.rs
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/composition/loop_engine.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - claudine/lib/src/composition/loop_config.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/loop_actions.rs
  - claudine/lib/src/composition/select.rs
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
documentation: []
packages:
  - claudine
  - darkmatter
---

# Loop State Sequencing — Execution Plan

Converts [`spec.md`](spec.md) into a high-confidence, dependency-ordered
implementation plan. Every task is observable through a test, a compile, or a
behavioral checkpoint.

## Context (one-paragraph recap)

A `loop:` composition whose control variables (`phase`, `total_phases`) are
defined as frontmatter **expression templates** (`"{{ frontmatter(plan, ...)
|| 1 }}"`) crashes on the first `increment(phase)` because the loop engine
seeds its mutable state from the **raw** document frontmatter — template
strings and all — and never from the **resolved** `effective_frontmatter`
that `prepare_direct_with_schema` already produces each iteration. The
defect is architectural: the loop mutates and tests unresolved state while
the body renders from resolved state. The fix gives the loop ownership of a
**typed, resolved, control-variable-only** subset of state, while leaving
**derived/presentation** variables (e.g. `pass_icon`) to re-resolve every
iteration from the document.

## Resolved design decisions (binds the 5 open questions in the spec)

These are committed up front so implementers do not re-litigate them.

1. **Control-variable set = action targets ∪ condition identifiers ∪
   action-value-template identifiers.** Action targets come from every
   `LoopAction` variant's `prop`. Condition identifiers come from a recursive
   walk of the Darkmatter `Expr` AST (`Expr::Variable` leaves), reusing the
   existing `parse_condition` parser — never a regex. Action-value templates
   (`set(next, "{{ phase + 1 }}")`) are scanned via
   `ExpressionFinder::find_all_plain` + `parse`, and their identifiers are
   included so any variable the loop *reads* is resolved. (Resolves open
   question 1: **include** action-value identifiers.)

2. **Control variables are top-level keys.** `increment`/`set`/etc. target
   top-level keys today (`fm.get(prop)`), and `resolve_frontmatter` treats the
   head segment of a dotted condition path as the lookup key. The seed lifter
   uses the same rule. A dotted action target like `increment(state.phase)`
   is treated as the literal top-level key `"state.phase"` (preserving
   today's behavior). No new dotted-path support is added; a follow-up can
   revisit if a real document needs it. (Resolves open question 2: **defer**
   dotted control variables.)

3. **Accept the seed-pass cost.** Seeding adds exactly one extra
   `prepare_direct` compose pass before iteration 1 so the condition sees
   resolved values from the first check. Reusing the seed composition as
   iteration 1's prepare output is an optimization tracked as a follow-up
   (open question 3), **not** in scope here — correctness first. (Resolves
   open question 3.)

4. **Owned-keys model (the spec's proposed model), not re-sync.** The loop
   owns a resolved, control-variable-only map and mutates it; it never
   re-reads `effective_frontmatter` for control keys between iterations.
   This makes ownership explicit and avoids the re-pin hazard the spec calls
   out. (Resolves open question 4.)

5. **Read-only control variables (like `total_phases`) are handled by the
   same rule.** They are resolved once at seed and carried in the control
   map; no action mutates them, so they stay constant. This is harmless and
   keeps a single rule. (Resolves open question 5.)

### The surgical insight that shapes this plan

`execute_loop_with_config` — the engine function the CLI actually calls —
**does not change signature or behavior**. It already works correctly on
resolved values; today it simply receives raw ones. The entire fix is
upstream of it: **build a better `initial_frontmatter`** that contains only
( a ) resolved control variables and ( b ) CLI `key=value` setters, and
deliberately omits document-derived keys so they re-resolve each iteration.
This keeps every existing `loop_engine` / `loop_actions` / `loop_expression`
test passing unchanged.

Verified facts driving the plan:

- `execute_loop` (the high-level library entrypoint) has **zero production
  callers** — the CLI bypasses it for `execute_loop_with_config` via
  `run_loop_with_overrides` (`claudine/cli/src/commands/compose.rs:1430`).
  Both entrypoints independently build `initial_frontmatter` from
  `source.markdown.frontmatter().as_map()` (the raw seed).
- `LoopIterationContext::as_set_overrides`
  (`claudine/lib/src/composition/loop_engine.rs:73`) returns
  `frontmatter ∪ ambients`. Because Darkmatter's `set_overrides`
  **unconditionally overwrites** frontmatter keys at compose time, anything
  in this map is pinned into the rendered body. Today the raw seed pins
  template strings; after the fix, the control map pins only resolved
  control values, leaving derived keys (`pass_icon`) to re-resolve.
- `prepare_direct` already produces
  `PreparedComposition.effective_frontmatter`
  (`claudine/lib/src/composition/prepare.rs:141`) — the resolved, typed JSON
  the seed must read from.
- Darkmatter's `Expr` AST
  (`darkmatter/.../compose/expression/ast.rs`) exposes `Variable`,
  `Binary`, `Index`, `MemberAccess`, `Fallback`, `Ternary`,
  `FunctionCall`, `UnaryNot`, `UnaryMinus` — a complete visitor target for
  identifier collection.

---

## Phase 1 — Library seed + control-variable identification

**Goal:** the library becomes the single source of truth for building a
resolved, control-variable-only loop seed. After this phase, a library
caller can produce a correct `initial_frontmatter` for
`execute_loop_with_config`, and `execute_loop` uses it too.

**Files touched (expected):**

- `claudine/lib/src/composition/loop_config.rs` — new
  `extract_control_variables` + identifier walker.
- `claudine/lib/src/composition/loop_engine.rs` — new `build_loop_seed`;
  `execute_loop` wired through it.
- `claudine/lib/src/composition/mod.rs` — re-export new helpers.
- New unit tests co-located with the above.

### Tasks

- [x] **1.1 Add `extract_control_variables(config: &LoopConfig) -> Vec<String>`.**
  Place it in `loop_config.rs` alongside `resolve_loop_config`. It must
  return a deduplicated, deterministic list (use `BTreeSet<String>`
  internally so output is stable for diffs). Sources:
  (a) every `LoopAction` target — `Increment(p)`/`Decrement(p)` yield `p`;
  `Set`/`Append`/`Prepend`/`Merge` yield `prop`;
  (b) condition identifiers — parse `config.condition` source with the
  existing `darkmatter::markdown::compose::expression::parse_condition`, then
  walk the resulting `Expr` collecting `Expr::Variable` head segments;
  (c) action-value template identifiers — for `Set`/`Append`/`Prepend`/
  `Merge` values that are `Value::String` containing `{{ }}`, run
  `ExpressionFinder::find_all_plain` + `parse` on each span and collect
  identifiers. **Exclude reserved namespaces** from the condition/action walk:
  `true`, `false`, `doc`, `env`, and any identifier starting with `_loop_`
  (ambients are supplied separately and are never seed-resolved).

- [x] **1.2 Add the recursive `Expr` identifier visitor.**
  A private `collect_identifiers(&Expr, &mut BTreeSet<String>)` helper in
  `loop_config.rs`. Must recurse through every AST node that can carry a
  variable reference: `Variable`, `UnaryNot`, `UnaryMinus`, `Binary`,
  `Index` (both `base` and `index`), `MemberAccess` (`base` only — the
  member `name` is not a variable), `Fallback`, `Ternary` (all three arms),
  and `FunctionCall` (`args` only — `name` is a function, not a variable).
  Literals (`StringLiteral`, `NumberLiteral`, `BoolLiteral`) terminate.

- [x] **1.3 Add `build_loop_seed` to the library.**
  New function in `loop_engine.rs` (it needs `prepare_direct`, which lives in
  the same crate). Signature:

  ```rust
  pub fn build_loop_seed(
      source: &ResolvedCompositionSource,
      config: &LoopConfig,
      prepare_options: PrepareOptions,
  ) -> Result<Map<String, Value>, CompositionError>
  ```

  Contract: run `prepare_direct(source, prepare_options)` once, read
  `effective_frontmatter`, and build a map containing **only**:
  1. CLI setters — every key from `prepare_options.set_overrides` (if
     `Some(Value::Object)`), carried verbatim so the body sees them every
     iteration;
  2. Control variables — for each name from `extract_control_variables`,
     lift its resolved value from `effective_frontmatter` (top-level
     `get(name)`). Control values **win over** CLI setters for overlapping
     keys (insert CLI setters first, then control values).
  Document-derived keys (`pass_icon`, lifecycle `message`, etc.) are
  intentionally **not** lifted — they re-resolve each iteration. If a control
  variable is absent from `effective_frontmatter`, omit it (the action will
  create it, matching today's `None → 1` increment semantics).

- [x] **1.4 Wire `execute_loop` through `build_loop_seed`.**
  `execute_loop` (`loop_engine.rs:224`) currently builds `initial_frontmatter`
  from `source.markdown.frontmatter().as_map()`. Change it to accept a
  `prepare_options: PrepareOptions` parameter (add it after `options`) and
  call `build_loop_seed(source, &config, prepare_options)` to produce
  `initial_frontmatter`, then delegate to `execute_loop_with_config`
  unchanged. This is a signature change to a function with **zero production
  callers** (only doc-referenced), so blast radius is the library's public
  API + any library tests that call `execute_loop` directly (audit with a
  grep — expected: none besides its own doctests).

- [x] **1.5 Re-export the new helpers from `composition/mod.rs`.**
  Add `build_loop_seed` and `extract_control_variables` to the
  `pub use loop_engine::{...}` / `pub use loop_config::{...}` blocks so the
  CLI can reach them as `claudine::composition::build_loop_seed`.

### Phase 1 validation checkpoints

- [x] **VC-1.1 `extract_control_variables` unit tests (in `loop_config.rs`).**
  Cases that must pass:
  - repro shape: `until: "phase > total_phases"`, `action: "increment(phase)"`
    → returns `["phase", "total_phases"]` (order-stable);
  - action-value template: `set(next, "{{ phase + 1 }}")` plus
    `until: "phase < max"` → returns `["max", "next", "phase"]` — note `next`
    is the *target* (action prop), `phase` comes from both the condition and
    the value template, `max` from the condition;
  - reserved-namespace exclusion: `while: "_loop_count < 3 && env.DEBUG"`
    with no actions → returns `[]`;
  - dotted condition path: `until: "state.done"` → returns `["state"]`
    (head segment only);
  - empty/identity: no actions, `while: "true"` → returns `[]`.
- [x] **VC-1.2 `build_loop_seed` unit tests (in `loop_engine.rs`).**
  Build a `ResolvedCompositionSource` from an inline `Markdown` whose
  frontmatter has `phase: "{{ start || 1 }}"`, `total_phases: "{{ 6 }}"`,
  plus a derived `pass_icon: "{{ _loop_is_last ? '✅' : '🧑‍💻' }}"`. Use a
  `LoopConfig` matching the repro. Call `build_loop_seed` with
  `PrepareOptions { set_overrides: Some(json!({"start": 1})), .. }`. Assert:
  - returned map contains `phase == json!(1)` (number, not string);
  - returned map contains `total_phases == json!(6)`;
  - returned map **does not** contain `pass_icon` (derived, not lifted);
  - returned map contains `start == json!(1)` (CLI setter carried).
  > Note: the test fixture uses `initial_phase` as the CLI setter key instead
  > of `start` because `start` is a reserved lifecycle frontmatter property;
  > the assertion semantics are unchanged.
- [x] **VC-1.3 Regression: existing `loop_engine` / `loop_actions` /
  `loop_expression` tests pass unchanged.** Run the library test suite for
  these three modules; zero modifications expected because
  `execute_loop_with_config` is untouched. This is the proof that the change
  is surgical.
- [x] **VC-1.4 `cargo build -p claudine` (library) compiles.** Then
  `cargo build -p claudine-cli` — the plan expected a compile error at
  `run_loop_with_overrides`, but the CLI uses `execute_loop_with_config`
  directly and therefore still compiles. The raw-seed behavior in
  `run_loop_with_overrides` remains and will be replaced in Phase 2.

---

## Phase 2 — CLI wiring

**Goal:** `run_loop_with_overrides` stops re-implementing seeding and routes
through the shared library helper. Per-iteration overrides carry the typed
control state so iteration *N*'s body renders `phase = N`.

**Dependency:** Phase 1 (`build_loop_seed` must exist and compile).
**Parallelizable with:** Phase 3 (error-message work touches different
functions in `loop_actions.rs` / `error.rs`).

**Files touched (expected):**

- `claudine/cli/src/commands/compose.rs` — `run_loop_with_overrides`
  (lines ~1338–1447) and its two call sites (~592, ~1080).

### Tasks

- [x] **2.1 Replace the raw-seed block in `run_loop_with_overrides` with
  `build_loop_seed`.**
  Delete the block at `compose.rs:1359–1371` that builds
  `initial_frontmatter` from `source.markdown.frontmatter().as_map()` +
  merges `set_overrides`. In its place, construct a `PrepareOptions`
  matching what iteration 1's executor builds today (env overrides, shell
  working directory, source repo root — these are already available in the
  caller scope at both call sites) and call
  `claudine::composition::build_loop_seed(source, &config, prepare_options)`.
  The returned map becomes `initial_frontmatter` for
  `execute_loop_with_config`. Preserve the existing `launch_cwd` /
  `launch_pwd` capture and the `wrapped_executor` interrupt/CWD-restoration
  logic untouched — that machinery is orthogonal to seeding.

- [x] **2.2 Confirm per-iteration overrides already carry control state.**
  No code change expected here — verify, don't assume. The executor closure
  at `compose.rs:601–613` builds `set_overrides: Some(ctx.as_set_overrides())`.
  Because `ctx.frontmatter` is now the control-only seed (mutated by actions
  each pass), `as_set_overrides()` already returns the live control value
  (`phase = N`) plus CLI setters plus ambients. Add an inline assertion in
  the executor (or a debug-logging probe during development) that
  `ctx.frontmatter.get("phase") == Some(json!(N))` on iteration N, then
  remove the probe once the integration test (Phase 4) covers it.

- [x] **2.3 Apply the same wiring to the second `run_loop_with_overrides`
  call site (inline-compose path, ~1080).**
  Both call sites share the same `run_loop_with_overrides` function, so this
  is automatically covered by 2.1 — but **verify** the `PrepareOptions`
  constructed at each call site carries the same env/cwd/repo-root fields
  iteration 1 uses, so the seed matches iteration 1's compose. If the two
  call sites build `PrepareOptions` differently, align them.

### Phase 2 validation checkpoints

- [x] **VC-2.1 The repro command compiles and runs to completion.** Execute
  the exact repro from the spec against a stub `plan.md` fixture (or the real
  `features/2026-06-09-improved-descriptions/plan.md` if available):
  ```
  claudine compose prompts/implement-plan.md \
    plan=<path/to/plan.md> -y --opencode --model kimi-for-coding/k2p7
  ```
  under a fake executor (or `--dry-run` if the loop is skipped — see note).
  Expect: no `InvalidIncrementType`; `phase` advances; loop halts. (If a real
  provider run is impractical in CI, Phase 4's library integration test is
  the authoritative substitute; this checkpoint is the manual smoke test.)
- [x] **VC-2.2 `cargo test -p claudine-cli` passes.** No new CLI tests
  required in this phase (Phase 4 covers the loop end-to-end), but existing
  CLI tests must not regress.
- [x] **VC-2.3 No behavioral drift between library and CLI entrypoints.**
  Confirm by grep that **neither** `execute_loop` **nor**
  `run_loop_with_overrides` references
  `source.markdown.frontmatter().as_map()` for seeding after the change —
  both route through `build_loop_seed`. This is the spec's explicit
  "share one seeding path" success criterion.

---

## Phase 3 — Honest error when coercion genuinely fails

**Goal:** when an increment/decrement target holds a value that cannot be
coerced, the error names the offending value and explains the resolution
stage — never the bare "has type string".

**Dependency:** Phase 1 (the resolved-state path must exist so an unresolved
template at action time is a meaningful signal). **Parallelizable with:**
Phase 2.

**Files touched (expected):**

- `claudine/lib/src/composition/error.rs` — `InvalidIncrementType` /
  `InvalidDecrementType` variants (lines ~360–392) and their `#[error]`
  format strings.
- `claudine/lib/src/composition/loop_actions.rs` —
  `apply_increment_with_context` (line ~251) /
  `apply_decrement_with_context` (line ~272) construction sites.

### Tasks

- [x] **3.1 Extend the two error variants with an offending-value field.**
  Add `value_excerpt: String` to both `InvalidIncrementType` and
  `InvalidDecrementType` in `error.rs`. Update the `#[error(...)]` format to
  read, e.g.:
  `invalid increment at iteration {iteration}, action {action_index} of {total_actions}: property '{property}' has type {found} (value: {value_excerpt})`.
  Keep the existing fields and their names stable so downstream matchers
  keep working; only **add** the new field.

- [x] **3.2 Populate the excerpt at the two construction sites.**
  In `apply_increment_with_context` / `apply_decrement_with_context`
  (`loop_actions.rs`), build the excerpt from the offending `value`:
  - if the value is a `Value::String` that matches the unresolved-template
    shape (contains `{{` and `}}`), set the excerpt to the raw string and
    append a stage note, e.g. `"{{ frontmatter(plan, 'start_phase') || 1 }}" (unresolved template — the loop seed failed to resolve this property)`.
    This is defense-in-depth: reaching it after Phases 1–2 indicates a
    seed-resolution gap, not user error.
  - otherwise, truncate the JSON representation of the value to a sane
    width (e.g. 80 chars) and store it as the excerpt, e.g.
    `"area-51"` for a resolved-but-non-numeric string.
  Add a small private helper (`value_error_excerpt(&Value) -> String`) in
  `loop_actions.rs` to keep the two sites identical.

- [x] **3.3 Route through the existing composition-error formatting.**
  No new renderer code. These variants already flow through the claudine
  composition-error formatting contract (styled, deduplicated, no raw
  chains). Confirm by checking the error renders via the existing
  `as_block_error` / CLI error-render path — do **not** bypass it with a
  bespoke `eprintln!`.

### Phase 3 validation checkpoints

- [x] **VC-3.1 Unresolved-template error message unit test.** Drive
  `apply_increment` on a map containing
  `phase: "{{ frontmatter(plan, 'start_phase') || 1 }}"` (the literal repro
  value). Assert the resulting `CompositionError::InvalidIncrementType`
  carries `found == "string"` **and** `value_excerpt` contains both
  `{{ frontmatter(plan, 'start_phase')` and the phrase
  `unresolved template`. This is the message that would have saved the
  original debugging session.
- [x] **VC-3.2 Resolved-non-numeric error message unit test.** Drive
  `apply_increment` on `area: "claudine-cli"` (a resolved but non-numeric
  string). Assert `found == "string"` and `value_excerpt` contains
  `claudine-cli` (truncated if longer than the cap). This covers the
  genuine-user-error path.
- [x] **VC-3.3 Decrement parity test.** Same two cases against
  `apply_decrement`, asserting `InvalidDecrementType` with the same excerpt
  semantics.
- [x] **VC-3.4 Existing error tests still match.** The tests at
  `loop_actions.rs:711` (`increment_rejects_non_numeric_strings`),
  `:935` (`increment_rejects_boolean`), `:951` (`decrement_rejects_boolean`),
  `:967` (`decrement_rejects_non_numeric_string`) use structural
  `matches!` patterns that ignore the new `value_excerpt` field — confirm
  they still compile and pass. If any test binds fields exhaustively
  (unlikely — they use `..` or partial patterns), update them.

---

## Phase 4 — Integration test

**Goal:** an end-to-end library test proves the four success criteria that
unit tests cannot: increment advances across iterations, the body reflects
`phase = N`, a derived variable stays live (flips on the last pass), and the
loop stops at the right iteration.

**Dependency:** Phases 1, 2, and 3 (needs the seeded path, the CLI wiring,
and the honest errors).

**Files touched (expected):**

- `claudine/lib/src/composition/loop_engine.rs` — new test module section
  (preferred: co-located with existing engine tests so it shares helpers),
  **or** a new `claudine/lib/tests/loop_seed_integration.rs` if the fixture
  wants to live as a file. Recommendation: inline the fixture document via
  `Markdown::new` to avoid fixture-file management, matching the style of
  the existing `until_file_exists_resolves_against_prompt_parent` test.

### Tasks

- [x] **4.1 Author the repro fixture document inline.**
  Build a `ResolvedCompositionSource` from a `Markdown` whose frontmatter
  mirrors `implement-plan.md`:
  ```yaml
  phase: "{{ start || 1 }}"
  total_phases: 6
  pass_icon: "{{ _loop_is_last ? '✅' : '🧑‍💻' }}"
  loop:
    until: "phase > total_phases"
    action: "increment(phase)"
  ```
  and whose body is `Implement Phase {{ phase }} of {{ total_phases }}`.
  No external file I/O — keep it self-contained in the test.

- [x] **4.2 Wire the seeded engine with a real-prepare executor.**
  Call `build_loop_seed` → `execute_loop_with_config` with an executor that
  invokes `prepare_direct(source, PrepareOptions { set_overrides:
  Some(ctx.as_set_overrides()), .. })` and captures both the rendered body
  and the resolved `pass_icon` per iteration into a `RefCell<Vec<...>>`.
  Use a no-op provider (return `LoopIterationOutput::success(body)`).

- [x] **4.3 Assert the full success-criteria matrix.** The test must
  observe, across the run:
  - **Stop count:** exactly 6 iterations execute; the loop halts without
    `InvalidIncrementType` and without `LoopLimitExceeded`.
  - **Increment advances:** the seed control state goes `phase = 1, 2, …, 7`
    (6 iterations render 1–6, then the post-iteration-6 increment to 7
    trips `phase > total_phases`). Assert via the captured per-iteration
    `ctx.frontmatter["phase"]`.
  - **Body reflects `phase = N`:** iteration N's captured body equals
    `Implement Phase N of 6` for N in 1..=6. This is the spec's "rendered
    body of iteration N reflects the mutated control value" criterion.
  - **Derived variable stays live:** `pass_icon` is `🧑‍💻` for iterations
    1..=5 and `✅` for iteration 6 (the final pass). This proves the
    derived/presentation class was never frozen at its seed value.

- [x] **4.4 Assert the honest-error path end-to-end (negative fixture).**
  A second fixture where `increment(area)` targets a resolved non-numeric
  string (`area: "claudine"`). Seed resolves it (to the string), the first
  increment fails, and the resulting `InvalidIncrementType` carries the
  `value_excerpt` from Phase 3. This ties Phases 1 + 3 together at the
  engine level.

### Phase 4 validation checkpoints

- [x] **VC-4.1 The integration test passes locally on macOS** (the host OS).
- [x] **VC-4.2 `just test` (or `cargo test -p claudine`) is green**, including
  the new test and all pre-existing loop tests.
- [x] **VC-4.3 `cargo fmt --check` is clean** for the touched files (read-only
  diagnostic — do **not** run `cargo fmt` in write mode per the repo's
  formatting policy in `AGENTS.md`; hand-match surrounding style instead).

---

## Validation matrix (maps spec success criteria → checkpoints)

| Spec success criterion | Proven by |
|---|---|
| Repro runs to completion; `phase` advances 1 → 7; no `InvalidIncrementType` | VC-4.3 (asserts advance + no error), VC-2.1 (manual smoke) |
| Iteration N's body shows `Implement Phase N of 6` | VC-4.3 (body capture per iteration) |
| `pass_icon` flips on the final pass (derived stays live) | VC-4.3 (`pass_icon` capture), VC-1.2 (seed omits `pass_icon`) |
| Non-coercible control var → error names value + stage | VC-3.1, VC-3.2, VC-4.4 |
| Library and CLI share one seeding path | VC-2.3 (grep audit), VC-1.3 (engine unchanged) |
| All existing loop tests pass; new tests cover seed + derived/control split | VC-1.3, VC-4.2 |

## Dependency graph & parallelization

```
Phase 1 (sequential internally: 1.1 → 1.2 → 1.3 → 1.4 → 1.5)
   │
   ├─→ Phase 2 (CLI wiring)  ─┐
   │                          │   ← Phase 2 and Phase 3 are independent
   └─→ Phase 3 (errors)       ┤     and MAY run in parallel after Phase 1.
                              │
                              └─→ Phase 4 (integration test)
```

- **Critical path:** Phase 1 → Phase 2 → Phase 4.
- **Parallel lane:** Phase 3 can start the moment Phase 1 lands; it touches
  `error.rs` + `loop_actions.rs` construction sites, which Phase 2 does not
  touch.
- **Phase 1 internal order is fixed:** control-variable extraction (1.1, 1.2)
  must exist before `build_loop_seed` (1.3) can lift the right keys, which
  must exist before `execute_loop` (1.4) can call it.

## Risks & notes

- **PrepareOptions divergence between seed and iteration 1** is the most
  likely silent bug. The seed pass and iteration 1's executor must build
  `PrepareOptions` from the same env/cwd/repo-root source. Task 2.3
  explicitly audits both call sites for this. If they diverge, the seed's
  resolved values will disagree with iteration 1's compose and the body will
  render the wrong `phase` on the first pass.
- **`execute_loop` signature change** (1.4) is a public-API break, but the
  function has zero production callers (verified by grep). The CLI uses
  `execute_loop_with_config` directly. If an external consumer of the
  `claudine` library calls `execute_loop`, they must supply
  `PrepareOptions::default()` — document this in the function's rustdoc.
- **Seed-pass cost** (one extra compose before iteration 1) is accepted for
  correctness (open question 3). A follow-up issue should evaluate reusing
  the seed composition as iteration 1's prepare output. Do not attempt that
  optimization inside this feature — it changes failure semantics (a seed
  error would become an iteration-1 error).
- **No `cargo fmt` write mode** (repo policy, `AGENTS.md`). All file edits
  hand-match surrounding style; `cargo fmt --check` is a read-only gate only.
- **Cross-platform:** no OS-specific code is introduced (no filesystem, no
  process, no env-var mutation beyond what the existing launch-CWD
  restoration already does). The fix is pure data-flow and works identically
  on macOS, Windows, and Linux.
