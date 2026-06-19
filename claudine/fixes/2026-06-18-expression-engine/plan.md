---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-18
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - prompts/implement-suggestions.md
source_files_during_phase_2:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/prepare.rs
source_files_during_phase_3:
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/lifecycle.rs
docs_updated_during_phase_1: []
docs_updated_during_phase_2: []
docs_updated_during_phase_3: []
docs_created_during_phase_1: []
docs_created_during_phase_2: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_1: []
skills_files_updated_during_phase_2: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - claudine/fixes/2026-06-18-expression-engine/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/SKILL.md
source_code:
  - prompts/implement-suggestions.md
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/prepare.rs
documentation:
  - claudine/fixes/2026-06-18-expression-engine/plan.md
packages:
  - claudine
---

# Plan: Expression Engine Errors Must Not Leak Into Lifecycle Messages

Executable conversion of [`spec.md`](./spec.md). Two independent defects: a
typo-ridden bundled prompt that emits raw Darkmatter syntax, and a missing
 Claudine-side guard that lets interpolation parse failures survive into
user-visible lifecycle side effects (Discord, Slack, TTS, stderr).

The root cause lives in Darkmatter — `interpolate_text` leaves the literal
`{{ … }}` in place on a parse error when `fail_fast = false` (the default),
and the related `darkmatter/fixes/interpolation-error-handling` spec promotes
grammar errors to fatal but is not implemented yet. **This fix does not wait
for it.** Claudine adds a narrow, typed guard at the composition boundary
that scans rendered lifecycle strings for surviving `{{ … }}` spans and fails
preparation with an author-facing diagnostic. The guard uses Darkmatter's own
`ExpressionFinder::find_all_plain` for span recognition, keeping Darkmatter as
the interpolation authority.

## Dependency graph

```
Phase 1 (prompt fixture) ──┐
                           ├──> Phase 3 (regression tests) ──> Phase 4 (verification)
Phase 2 (lifecycle guard) ─┘
```

- **Phase 1 and Phase 2 are parallelizable** — disjoint files
  (`prompts/implement-suggestions.md` vs. `claudine/lib/src/composition/*`).
- Phase 3 depends on Phase 2 (tests assert the new error variant and the
  cleaned prompt composes cleanly).
- Phase 4 is the acceptance gate and must run last.

## Conventions for the implementer

- Never run `cargo fmt` in write mode (repo rule). Match surrounding style by hand.
- Use US English in any prose or symbol names touched.
- Do **not** add a second expression parser to Claudine. Use Darkmatter's
  `ExpressionFinder::find_all_plain` for span detection — it is already a
  claudine dependency (see `loop_actions.rs`, `loop_config.rs`).
- Do **not** change `ComposeOptions::fail_fast` plumbing or Darkmatter's
  interpolation behavior. That work is owned by
  `darkmatter/fixes/interpolation-error-handling`.
- Any new terminal diagnostic must render through `BlockError` + existing
  `biscuit-terminal` components (`Prose`, `StatusBlock`, `ErrorHeader`). No
  ad hoc ANSI.
- Line numbers below are anchors from the branch at planning time; verify with
  `rg` before editing since offsets drift.

---

## Phase 1 — Bundled prompt fixture correction

Goal: the `implement-suggestions.md` bundled prompt no longer authors invalid
or undefined interpolation expressions in its lifecycle frontmatter. This is
the direct user-visible defect from the bug report. Independent of Phase 2 and
parallelizable with it.

### 1A. Fix the lifecycle strings in `prompts/implement-suggestions.md`

The offending block is `prompts/implement-suggestions.md:12-20`. Confirmed
defects:

- [x] Fix the unmatched paren in `start.message`: `{{ parent_dir(review)) }}`
      → `{{ parent_dir(review) }}` (prompts/implement-suggestions.md:13).
- [x] Fix the misspelling in `start.message`: `{{interation}}` →
      `{{iteration}}` (prompts/implement-suggestions.md:13).
- [x] Fix the misspelling in `success.message`: `_implemention_` →
      `_implementation_` (prompts/implement-suggestions.md:15).
- [x] Normalize the bare `{{area}}` references to `{{ctx.area}}` in
      `start.message`, `success.message`, and `success.say`
      (prompts/implement-suggestions.md:13,15,16). The prompt body already
      uses `{{ctx.area}}` (prompts/implement-suggestions.md:22,32); the
      lifecycle strings must match so the variable resolves from context
      instead of resolving to an empty string.
- [x] Resolve the `{{iteration}}` reference. The prompt defines no
      `iteration` frontmatter key and no `ctx.iteration` exists. Either:
      - remove the `(… iteration: {{iteration}})` clause from
        `start.message` if iteration tracking is not part of this prompt's
        contract, **or**
      - add a frontmatter default such as
        `iteration: "{{ ctx.iteration || '1' }}"` if iteration is intended.
      Prefer removal unless the spec author confirms iteration is meaningful
      here — a missing variable in a lifecycle message is an operational
      status defect, not cosmetic prose.
- [x] Re-scan the full lifecycle block (`start`, `success`, `failure`) once
      the above edits land and confirm no other `{{ … }}` expression has a
      grammar defect or references an undefined bare variable. The
      `title_case(...)` and `without_date(...)` calls in `success.say` /
      `failure.message` are valid; leave them.

### Checkpoint 1 — prompt composes with no surviving delimiters

```sh
cargo run -p claudine-cli -- compose prompts/implement-suggestions.md \
  review="features/2026-06-18-composition-shell-error-diagnostics/review-1.md" \
  --dry-run 2>&1 | rg '\{\{|\}\}'
```

Expected: **zero matches**. After Phase 2 lands, the guard makes this a hard
error; before Phase 2 lands, this is a manual confirmation that the raw
delimiters are gone from the rendered lifecycle metadata.

---

## Phase 2 — Claudine lifecycle interpolation guard

Goal: Claudine fails composition preparation — before provider execution and
before any side-effect dispatch — when a rendered lifecycle string still
contains a recognized `{{ … }}` interpolation span. This is the generic,
prompt-independent defense. Independent of Phase 1 and parallelizable with it.

### 2A. New `CompositionError` variant

In `claudine/lib/src/composition/error.rs`:

- [x] Add a new variant `LifecycleInterpolationLeak` to `CompositionError`
      (after `LifecycleUnknownEffect`, error.rs:254) carrying:
      - `source_path: PathBuf` — the composed prompt file.
      - `property: String` — the dotted lifecycle key path, e.g.
        `"start.message"` or `"failure.say"`.
      - `expression: String` — the raw offending span text, e.g.
        `{{ parent_dir(review)) }}`.
      - `reason: String` — the parse/eval failure reason when Darkmatter
        recorded a warning (best-effort, extracted from the compose report's
        `warnings`); empty string when the span is unrecognized entirely.
- [x] Give it a `#[error("…")]` Display impl naming the property, expression,
      and source path, mirroring the prose style of `LifecycleInvalid`.
- [x] Add a `BlockError::status_block` arm for the new variant that renders
      through `StatusBlock::new(StatusState::Error)` +
      `ErrorHeader::new("CompositionError", "lifecycle interpolation leaked")`
      with a body identifying the file link (reuse the existing
      `render_file_link` helper, error.rs:1082), the dotted property
      (`<cyan>\`{property}\`</cyan>`), the raw expression, and a hint
      pointing the author to fix the expression grammar or define the
      referenced variable. Follow the rendering pattern of the existing
      `LifecycleInvalid` arm (error.rs:785).

### 2B. Validation helper

In `claudine/lib/src/composition/lifecycle.rs` (or a new sibling
`lifecycle_guard.rs` if `lifecycle.rs` is large — prefer keeping it in
`lifecycle.rs` next to `parse_lifecycle_config`):

- [x] Add a function `validate_no_interpolation_leaks(
        config: &LifecycleConfig,
        source_path: &Path,
        warnings: &[darkmatter::markdown::compose::ComposeWarning],
      ) -> Result<(), CompositionError>`.
- [x] Iterate every signal (`Start`, `Success`, `Blocked`, `Failure`) × every
      string field (`say`, `say_first`, `message`, `stderr`, `notify`) on the
      `LifecycleNotification`. Skip `None` and empty fields. For each
      present string, run
      `darkmatter::markdown::compose::expression::ExpressionFinder::find_all_plain(text)`.
- [x] On the first field with a non-empty span list, build the dotted key
      path as `"{signal.property_name()}.{field}"` (e.g. `"start.message"`)
      using the existing `LifecycleSignal::property_name()`
      (lifecycle.rs:432). Attempt to match the leaked span against the
      compose `warnings` to enrich the `reason`; if no warning matches, leave
      `reason` empty. Return `Err(CompositionError::LifecycleInterpolationLeak { … })`.
- [x] The field iteration order must be deterministic (iterate signals in
      `Start, Success, Blocked, Failure` order; fields in the order
      `say, say_first, message, stderr, notify`) so the first-reported leak
      is stable across runs.

### 2C. Wire the guard into preparation

In `claudine/lib/src/composition/prepare.rs`:

- [x] In `prepare_direct`, immediately after the existing
      `let lifecycle = parse_lifecycle_config(&effective_frontmatter, &source.resolved_path)?;`
      (prepare.rs:165), call
      `validate_no_interpolation_leaks(&lifecycle, &source.resolved_path, &report.warnings)?;`.
      This runs before the `Ok(PreparedComposition { … })` return, so a leak
      aborts preparation.
- [x] In `prepare_inline`, do the same after its
      `parse_lifecycle_config` call (prepare.rs:258).
- [x] Confirm `report.warnings` is in scope at both sites — it is already
      bound from the `compose_with` destructure (prepare.rs:127-130,
      prepare.rs:230-232).

### Checkpoint 2 — guard fires on malformed lifecycle, renders richly

```sh
cargo test -p claudine --lib composition::prepare \
  lifecycle_interpolation_leak
```

Expected: the focused unit test added in Phase 3A passes (the guard returns
the new variant). If Phase 3 has not landed yet, write a throwaway test in
the `prepare.rs` `mod tests` that composes a fixture with
`start: { message: "{{ parent_dir(review)) }}" }` and asserts the error is
`LifecycleInterpolationLeak` with `property == "start.message"`.

---

## Phase 3 — Regression test coverage

Goal: lock in both the prompt-specific fix and the generic guard. Depends on
Phase 2 (uses the new error variant) and Phase 1 (the cleaned prompt must
compose cleanly).

### 3A. Claudine prepare-layer unit tests

In `claudine/lib/src/composition/prepare.rs` `mod tests` (alongside the
existing `invalid_lifecycle_config_fails_preparation`,
prepare.rs:639-653):

- [x] Add `malformed_lifecycle_interpolation_fails_preparation`: a fixture
      with `start: { message: "{{ parent_dir(review)) }}" }` and a body
      string, composed via `prepare_direct`, asserts the error is
      `CompositionError::LifecycleInterpolationLeak` with
      `property == "start.message"` and `expression` containing
      `parent_dir(review))`. Provider execution never starts because
      preparation returns `Err`.
- [x] Add `lifecycle_leak_reported_for_first_field_in_deterministic_order`:
      a fixture with leaks in both `start.message` and `failure.say` asserts
      the `start.message` leak is reported (first signal, first string field)
      — proving deterministic ordering.
- [x] Add `clean_lifecycle_interpolation_passes_preparation`: a fixture
      with `start: { message: "{{ ctx.today }}" }` (a valid expression that
      resolves) asserts `prepare_direct` returns `Ok` and the rendered
      lifecycle message contains no `{{` or `}}`. This guards against the
      guard being over-eager.

### 3B. Bundled prompt regression test

In `claudine/lib/src/composition/prepare.rs` `mod tests` (or a dedicated
`tests/` integration test if file fixtures are cleaner there):

- [x] Add `implement_suggestions_prompt_composes_without_lifecycle_leak`:
      load the real `prompts/implement-suggestions.md` from the repo
      (resolve relative to `CARGO_MANIFEST_DIR` via the
      `claudine/lib` crate root — the `prompts/` dir is two levels up).
      Compose with `review` set to a fixture path. Assert `prepare_direct`
      returns `Ok` and that **no** `LifecycleNotification` string in the
      resulting `PreparedComposition.lifecycle` contains `{{` or `}}`. This
      is the direct regression for the bug report's command.
- [x] If loading the real prompt is brittle across `CARGO_MANIFEST_DIR`
      boundaries, embed the cleaned lifecycle block as a test fixture string
      and assert it composes without leaks. Prefer the real-file path so the
      test breaks if the prompt regresses.

### 3C. Messenger-zero-send guard test

In `claudine/lib/src/composition/lifecycle.rs` `mod tests` (which already
has the `RecordingEmitter` harness, lifecycle.rs:1089+):

- [x] Add `guard_prevents_message_dispatch_on_leak`: construct a
      `LifecycleConfig` whose `start.message` contains a surviving
      `{{ broken( }}` span (simulating a leak past composition), wire a
      `RecordingEmitter`, and assert that the guard's error is returned
      **before** any `emit_message` / `emit_speech` / `emit_effect` call is
      recorded. This proves the "zero sends on failure" acceptance criterion
      at the emission boundary, independent of the prepare layer.

### Checkpoint 3 — regression suite green

```sh
cargo test -p claudine --lib composition::prepare lifecycle
cargo test -p claudine --lib composition::lifecycle guard_prevents
```

Expected: all new tests pass and the existing lifecycle/prepare tests still
pass.

---

## Phase 4 — Cross-cutting verification & acceptance

Goal: prove the fix is exhaustive, no stale references remain, and the
spec's acceptance criteria are met end-to-end. Must run after Phases 1–3.

### 4A. Stale-reference sweep

- [x] Confirm the bundled prompt no longer contains the original defects:
      ```sh
      rg -n '\{\{\s*parent_dir\(review\)\)\s*\}\}|interation|implemention' \
        prompts/implement-suggestions.md
      ```
      Expected: zero matches.
- [x] Confirm the prompt's lifecycle strings no longer reference a bare,
      undefined `{{area}}`:
      ```sh
      rg -n '\{\{area\}\}' prompts/implement-suggestions.md
      ```
      Expected: zero matches (should be `{{ctx.area}}` after Phase 1A).

### 4B. Full claudine test suite

- [x] Run the canonical composition test commands:
      ```sh
      cargo test -p claudine --lib composition
      cargo test -p claudine-cli --test compose_cli
      ```
      Expected: all tests pass, including the new Phase 3 regressions and the
      existing lifecycle/prepare/sequence tests.

### 4C. Build and typecheck

- [x] Confirm both claudine crates compile cleanly (the new error variant +
      renderer + guard add surface area):
      ```sh
      cargo build -p claudine
      cargo build -p claudine-cli
      ```
- [x] Confirm claudine doctests still pass (the `LifecycleConfig` / parse
      doctest examples are sensitive to the new validation if it runs during
      `parse_lifecycle_config` — it must **not** run there, only in
      `prepare_*`):
      ```sh
      cargo test --doc -p claudine
      ```

### Checkpoint 4 — acceptance criteria met

Walk the spec's acceptance list (`spec.md:254-262`) and confirm each:

- [x] The bundled prompt no longer contains `parent_dir(review))` or
      `interation` (Phase 1A + 4A).
- [x] The spec's example command cannot send a Discord message containing
      raw `{{ parent_dir(review)) }}` — the guard aborts preparation before
      any `emit_message` call (Phase 2C + 3C).
- [x] Any lifecycle interpolation parse error fails before external side
      effects (Phase 2B/2C guard runs in `prepare_*`, before
      `LifecycleRunGuard` is constructed).
- [x] The failure diagnostic points to the bad frontmatter key and
      expression (Phase 2A renderer names `property`, `expression`,
      `source_path`).
- [x] Regression tests cover both the prompt typo (Phase 3B) and the
      lifecycle side-effect guard (Phase 3A + 3C).

---

## Risk notes

- **Overlap with `darkmatter/fixes/interpolation-error-handling`.** That fix
  promotes grammar errors to fatal inside Darkmatter itself. When it lands,
  the Claudine guard becomes a defense-in-depth second check rather than the
  primary gate. The guard must **not** be removed when the Darkmatter fix
  lands — it also catches evaluation-error leaks (unknown functions under
  non-fail-fast, missing variables) that the Darkmatter fix intentionally
  leaves lenient. Revisit the guard's scope at that time but keep it.
- **`fail_fast` is not changed.** Claudine's `prepare_*` calls use the
  default `ComposeOptions` (`fail_fast = false`). The guard operates on the
  *output* of composition, so it is robust to whatever leniency Darkmatter
  applies internally. Do not flip `fail_fast` to `true` to "fix" this — that
  would change body-text leniency and break existing prompts that rely on
  optional-variable resolution to empty string.
- **Guard runs in `prepare_*`, not `parse_lifecycle_config`.** The lifecycle
  parser is also used for raw frontmatter inspection (e.g. dry-run metadata)
  where interpolated values may legitimately be unresolved. Running the guard
  only after composition (when expressions should have resolved) avoids false
  positives on uncomposed frontmatter.
- **`{{iteration}}` decision.** Phase 1A flags whether iteration is a real
  contract variable for this prompt. If it is removed, confirm no other
  tooling or documentation references an `iteration` lifecycle variable for
  `implement-suggestions.md`. If it is kept, the frontmatter must define it
  or the guard (after the Darkmatter fix lands) will flag it.
- **Deterministic field ordering.** The guard reports the first leak in
  `Start → Success → Blocked → Failure` ×
  `say → say_first → message → stderr → notify` order. If a prompt has
  multiple leaks, the author fixes them one at a time in a stable order.
  Document this in the function's rustdoc so the contract is explicit.
