---
status: ready for planning and implementation
depends_on: ../2026-05-12-lifecycle/spec.md
reviewed: true
review_iterations: 3
---

# Retire the Harness Pre/Post Validation & Handler DSL

## Introduction

Claudine's `harness` module (`claudine/lib/src/harness/`) turns a Markdown-backed
prompt into a small typed job harness: **pre-run checks**, **post-run checks**,
per-page **timeouts**, **shell audit**, and a four-tier **recovery-handler**
pipeline. The [Lifecycle Formalization](../2026-05-12-lifecycle/spec.md) feature
introduces a per-event `stack` model whose `when:` guards, `Error`/`Skip`/`Proxy`
lifecycle actions, and `Retry`/`Resume`/`Requeue`/`Proxy` recovery actions are a
strict superset of what the validation/handler DSL expresses.

This feature **retires the validation and handler DSL** once the lifecycle model
ships, so authors have exactly one way to gate a run and one way to recover from
failure. It deliberately **keeps** the harness infrastructure the lifecycle model
still depends on — shell audit, timeouts, runaway guards, and agent-failure
classification.

Reader note: this spec changes the current `.claude/skills/claudine/validations-and-handlers.md`
contract on purpose. The existing validation/handler DSL is not being repaired or
documented further; it is being replaced by lifecycle stacks after the lifecycle
dependency lands. The removal must still leave compatibility diagnostics in
place so old prompt files fail clearly.

> This is a follow-up to the lifecycle feature, not a parallel one. It must not
> land until the lifecycle `stack` model (and its `initialize`/`success`/
> `finalize` events plus `Error`/`Retry`/`Resume`/`Requeue`/`Proxy` actions) is
> implemented and proven. See [Dependencies & Sequencing](#dependencies--sequencing).

## Motivation

- **Zero real adoption.** No production prompt declares `pre_checks`,
  `post_checks`, or handler frontmatter; the only test fixture that previously
  exercised the DSL (`validation_reporter/missing_file.md`) has been removed
  along with the DSL.
- **Duplicated recovery surface.** The harness recovery tiers
  (`retry` / `resume` / `redirect` / `deviate`, resolved by
  `resolve_handler`, `harness/handlers.rs:49`) are a near-exact duplicate of the
  lifecycle `Retry` / `Resume` / `Requeue` / `Proxy` actions fired from `blocked`
  and `failure`. Shipping lifecycle while keeping the handler DSL gives authors
  two ways to express the same recovery — a maintenance and documentation tax.
- **Single mental model.** Pre-flight gating, post-run verification, and
  recovery all collapse into the one lifecycle `stack` grammar. The
  `initialize`/`start`/`blocked`/`success`/`failure`/`finalize` events already
  describe every slot the harness occupied.

## What "pre-flight" means after this change

The lifecycle spec defines `start` vs `blocked` by whether **pre-flight passed**,
and states pre-flight remains "the single validation surface owned by `start`."
That statement survives — but pre-flight is **redefined** to mean only the two
surfaces that are *not* part of the removed DSL:

1. **Shell audit** — `collect_auditable_commands` + `audit_shell_commands`
   (`harness/audit.rs`), now walking the lifecycle stacks instead of `pre_checks`/
   `post_checks`/`handle`/`deviate`. (Spec already requires: *"every shell command
   in every reachable stack must pass Claudine's command whitelist during
   pre-flight."*)
2. **Schema validation** — `$schema` / `SimplifiedSchema`, owned by
   `CompositionError`, unaffected by this change.

A `blocked` outcome after this change is produced by a shell-audit denial or a
schema-validation failure — never by a removed `pre_checks` rule.

The `timeout`, `timeout_warn`, `step_timeout`, and `step_timeout_warn`
frontmatter keys are not pre-flight validation rules and must remain accepted.
Their parse-time relational checks (`timeout_warn < timeout`,
`step_timeout_warn < step_timeout`, and `step_timeout <= timeout` when both are
set) stay intact because they are part of timeout configuration, not the removed
validation DSL.

## Scope: Remove

The following are removed (or reduced to whatever the kept surfaces still need).
Exact file boundaries are an implementation concern; this is the contract.

### Frontmatter surface (author-facing)

- `pre_checks:` and `post_checks:` blocks and the entire `ValidationKind`
  catalog (file/dir/JSON/YAML/TOML existence, write-permission, clean-repo,
  `file_changed` / `file_unchanged`, `frontmatter_prop_changed`,
  `response_includes`, `response_length_at_least`, etc.).
- Declarative recovery handlers: subject-specific handlers (`handle_<check>`),
  generic handlers (`handle_timeout`, `handle_agent_failure`, …), the
  programmatic `handle:` command, and `deviate:`.

Compatibility diagnostics must reject only the removed keys. The existing
lifecycle notification keys (`start`, `blocked`, `success`, `failure`) and new
lifecycle keys from the dependency (`initialize`, `finalize`, `loop`) continue
to parse through lifecycle validation.

### Library surface (`claudine/lib/src/harness/`)

- `validate/` (`compare.rs`, `fs.rs`, `git.rs`, `mod.rs`, `render.rs`) —
  `evaluate_pre_checks`, `evaluate_post_checks`, `capture_pre_run_snapshot`,
  `PreRunSnapshot`, `check_write_permission`.
- `handlers.rs` + `parse/handlers.rs` — `resolve_handler`, `FailureContext`,
  `HandlerAction`, `execute_deviate_command`, `validate_resume`, the
  `build_*_failure_context` family.
- `parse/validations.rs` and the `ValidationRule` / `ValidationKind` /
  `HandlerTable` portions of `model.rs`.
- `failure.rs`'s validation-specific taxonomy (`ValidationEvent` ~20 variants,
  `FailurePhase::{PreCheck, PostCheck}`, `ValidationFailure`) — to the extent it
  is no longer referenced by the kept surfaces. Do not remove the non-validation
  attempt outcome and process-termination taxonomy needed by lifecycle failure
  routing.
- `resolve.rs` (document-centric path resolution for validation rules) — unless a
  kept surface still needs it.
- The validation-specific four-section reporting in `report.rs`
  (`report_check_outcomes`, `report_phase_discovery`, `Pre-validation failed` /
  `Post-validation failed` blocks).

### Orchestration surface (`claudine/cli/src/commands/wrap/`)

- All `evaluate_pre_checks` / `evaluate_post_checks` / `capture_pre_run_snapshot`
  call sites in `harness_orch/loop_control.rs` (≈ lines 264, 325, 558) and
  `composition/mod.rs:1090`.
- The `try_resolve_handler` recovery path in `resume.rs` and `loop_control.rs`
  (the agent-failure/timeout recovery branches), **replaced** by lifecycle
  `failure`-event recovery actions.
- The `validation_reporter` PTY harness bin
  (`claudine/cli/src/bin/validation_reporter_pty_harness.rs`) and its fixture
  were removed with the DSL; no re-pointing was required.
- Any CLI help, shell-completion, or frontmatter-completion metadata that
  advertises `pre_checks`, `post_checks`, `handle_*`, `handle:`, or `deviate:`.

## Scope: Keep

These live in `harness/` today but are infrastructure the lifecycle model depends
on. They stay, possibly relocated, but are **not** removed:

- **Shell audit & approval** — `shell.rs`, `audit.rs`
  (`collect_auditable_commands`, `audit_shell_commands`,
  `validate_and_approve_command*`, `ShellApprovalOptions`, the cross-step
  approval cache). Pre-flight and lifecycle shell actions both rely on these.
- **Timeouts** — `timeout.rs` (`parse_timeout`) and the wall-clock/step-timeout
  machinery; see [Timeouts](../../docs/topics/timeouts.md).
- **Runaway content guards** — unchanged; they map to `ProcessTermination::Aborted`
  independent of the validation DSL.
- **Agent-failure classification** — `runtime.rs` (`build_attempt_outcome`,
  `ProcessTermination`, `FailureEvent`, `classify_failure`). The lifecycle router
  consumes this to decide `success` vs `failure`; only the *handler-based recovery*
  on top of it is removed.
- **Lifecycle notification validation** — `LifecycleSayConflict`,
  `LifecycleUnknownEffect`, `LifecycleInvalid`, the lifecycle interpolation leak
  guard, and the lifecycle undefined-variable guard stay. These are not harness
  validations; they protect the lifecycle surface that replaces the removed DSL.
- **Speech helper** — `speech.rs` (`speak_when_able`), if still used by lifecycle
  communication actions.
- **Inline-compose frontmatter restoration** — `composition/closure.rs`
  (`apply_inline_closure`, `compare_frontmatter`, `InlineClosurePlan`). This is
  the mechanism that preserves/reverts the `prompt` (and other) frontmatter
  properties across the agentic loop. It has its own baseline and does **not**
  use the harness snapshot/diff being removed; see gap #1.

## Capability Gaps & Decisions

Three capabilities of the removed DSL have no automatic lifecycle equivalent.
Each needs an explicit decision before removal.

### 1. Pre-run snapshot / diff verification (RESOLVED — drop it)

`post_checks` such as `file_changed`, `file_unchanged`, and
`frontmatter_prop_changed` compare against a **pre-run baseline** (BLAKE3
fingerprints captured by `capture_pre_run_snapshot`). The lifecycle `current`
global exposes only the **post**-event state, so an author cannot express "did
this file change during the run?" without a baseline.

**Decision:** drop the harness diff-based verification. No consumer uses it (the
only adopter is the lone test fixture), so the capability loss is acceptable and
no replacement lifecycle primitive is added.

**Explicit carve-out — `inline-compose` frontmatter restoration is NOT this and
stays.** `inline-compose`'s guarantee that the agent did not change the `prompt`
(or any other) frontmatter property — and its reversion to the original value if
it did — is a **separate mechanism** with its own baseline, and is untouched by
this removal:

- It lives in `composition/closure.rs`, not `harness/`.
- `InlineClosurePlan` carries its own `original_document_text` +
  `original_body_hash`; `apply_inline_closure` → `compare_frontmatter` diffs
  post-run frontmatter against that text and returns `reverted_properties`
  (reported at `inline.rs:178`).
- It never calls `capture_pre_run_snapshot` / `PreRunSnapshot` /
  `evaluate_post_checks`.

Removing the harness snapshot/diff therefore cannot regress inline-compose's
prompt-preservation behavior.

### 2. Accumulate-all-failures (UX regression)

`run_checks` walks **every** rule and reports all failures at once, so an author
sees every missing prerequisite in a single pass. A lifecycle stack
**short-circuits** on the first `Error` lifecycle action, so N validation
failures expressed as N stack items surface only the first.

**Decision:** Accept the regression (it only affects the
gate-many-preconditions pattern, which has no real adopters), and note it in the
lifecycle docs. No mitigation is planned unless adoption proves it necessary.

### 3. Typed four-section failure reporting

The `RuleSource`-driven report (status header / OSC8 source line / YAML snippet /
reason, `harness/report.rs:231`) is specific to validation rules. Lifecycle
errors render through `CompositionError` / `BlockError`. **Decision:** lifecycle
reporting replaces it; the validation-specific report code is removed with the
DSL.

## Dependencies & Sequencing

- **Hard dependency:** the [lifecycle feature](../2026-05-12-lifecycle/spec.md)
  must be implemented and merged first. Specifically, `failure`/`blocked`
  recovery actions (`Retry`/`Resume`/`Requeue`/`Proxy`) must be live before the
  harness `resolve_handler` path is deleted, or agent-failure recovery regresses
  to "terminal failure, no retry."
- **Compatibility gate:** before deletion, lifecycle parsing must own typed
  diagnostics for the removed DSL keys. Removing parser support without this gate
  would downgrade authored prompts to generic unknown-field errors.
- **Proof step:** port any remaining internal prompts that still referenced the
  removed DSL to the lifecycle `stack` model and confirm equivalent behavior
  end-to-end. The only validation fixture (`validation_reporter/missing_file.md`)
  was removed with the DSL.
- **Spec edit:** tighten the lifecycle spec's `start`/`blocked`/"pre-flight"
  wording to mean *shell audit + schema validation only* (see
  [What "pre-flight" means](#what-pre-flight-means-after-this-change)).

## Migration

- This is a **breaking** change to the (unadopted) frontmatter surface. A
  document still declaring `pre_checks` / `post_checks` / handler keys after this
  change must produce a typed, actionable `CompositionError` pointing at the
  lifecycle equivalent — not a silent ignore and not an `unknown field` dump.
- Add a new removed-DSL diagnostic variant rather than overloading
  `LifecycleInvalid`. It should carry the source path, offending key, and the
  recommended replacement surface. The frontmatter excerpt renderer should
  highlight the removed key when stderr is a TTY, matching existing
  frontmatter-rooted composition errors.
- The removed-key scan must happen before generic lifecycle unknown-field
  validation, and it must include:
  - exact top-level keys: `pre_checks`, `post_checks`, `handle`, `deviate`
  - handler-prefix keys: `handle_` followed by any non-empty suffix
  This avoids accidentally rejecting lifecycle keys while still catching every
  old handler form, including `handle_inline_response_empty` and
  `handle_inline_body_unchanged`.
- Suggested diagnostic mapping:

  | Removed key | Replacement |
  |-------------|-------------|
  | `pre_checks` | `initialize` or `start` stack with `Error`, `Skip`, or `Proxy` |
  | `post_checks` | `success` or `finalize` stack with `Error` |
  | `handle_*` | `blocked` or `failure` recovery stack actions |
  | `handle` | `blocked` or `failure` stack with a shell/action bridge |
  | `deviate` | lifecycle stack shell action followed by `Retry`, `Resume`, `Requeue`, or `Proxy` |

- Provide a short mapping table in the lifecycle/composition docs:
  `pre_checks` → `initialize`/`start` stack + `Error`/`Skip`/`Proxy`;
  `post_checks` → `success`/`finalize` stack + `Error`;
  `handle_*`/`deviate` → `failure`/`blocked` recovery actions.
- Remove `harness`-validation references from the claudine skill docs
  (`validations-and-handlers.md`) and the composition topic doc, replacing them
  with lifecycle pointers.
- Update the skill catalog docs after implementation. The current
  `.claude/skills/claudine/SKILL.md` module map describes `harness` as "Typed
  pre/post validations, timeouts, handler resolution, recovery actions"; that
  summary must change to the kept harness responsibilities or the new lifecycle
  owner after code is moved.

## Acceptance Criteria

- No frontmatter key `pre_checks`, `post_checks`, `handle_*`, `handle:`, or
  `deviate:` is accepted; each yields a typed `CompositionError` naming the
  lifecycle replacement.
- Removed-key errors include the frontmatter excerpt/highlight in TTY-capable
  output and remain escape-free in non-color output.
- `evaluate_pre_checks` / `evaluate_post_checks` / `capture_pre_run_snapshot` /
  `resolve_handler` and their call sites no longer exist in the wrap orchestration.
- Shell audit, timeouts, runaway guards, schema validation, and agent-failure
  classification continue to work unchanged; a shell-audit denial still routes to
  `blocked`.
- Agent-failure recovery (previously `handle_timeout` / `handle_agent_failure`)
  is exercised through the lifecycle `failure` event and passes an equivalent
  end-to-end test.
- The harness snapshot/diff (`capture_pre_run_snapshot`, `PreRunSnapshot`,
  `file_changed`/`file_unchanged`/`frontmatter_prop_changed`) is removed, and an
  end-to-end regression confirms `inline-compose` still reverts an
  agent-modified `prompt` frontmatter property to its original value (closure
  path, unaffected).
- The `validation_reporter` bin/fixture is either removed or re-pointed at
  lifecycle behavior; the test suite is green with no dangling references.
- Skill docs, CLI reference/help/completion metadata, `frontmatter-properties.md`,
  `validations-and-handlers.md`, `pre-flight-checks.md`, and the composition
  topic doc no longer describe the validation/handler DSL as an accepted surface.

## Test Strategy

- **L1:** parser rejects the removed frontmatter keys with the typed,
  did-you-mean `CompositionError`; the diagnostic includes source path, key, and
  replacement guidance. Add focused regression coverage for `pre_checks`,
  `post_checks`, `handle`, `handle_timeout`, `handle_inline_body_unchanged`, and
  `deviate`.
- **L1:** kept surfaces retain coverage: shell audit over lifecycle stacks,
  timeout parsing and timeout relational validation, lifecycle notification
  validation, runaway/failure classification, and inline closure frontmatter
  restoration.
- **L2:** end-to-end wrap/compose run where (a) a shell-audit denial routes to
  `blocked`, (b) an agent failure recovers via a lifecycle `failure` `Retry`/
  `Resume` action, and (c) an `inline-compose` run through the CLI whose agent
  mutates the `prompt` frontmatter property still reverts it to the original
  value (proving the closure path is independent of the removed snapshot/diff).
- **Regression sweep:** `rg` confirms no remaining references to the removed
  symbols across `claudine/lib`, `claudine/cli`, and docs.
