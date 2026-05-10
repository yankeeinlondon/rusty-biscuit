---
status: draft
owner: claudine
related:
  - features/2026-04-29-leverage-dm-parser/review-2.md
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/dispatch/runner.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
---

# Unify the Hook `when` Lookup with `EventMetaExpressionLookup`

## Background

Feature `2026-04-29-leverage-dm-parser` introduced
[`EventMetaExpressionLookup`](../../../lib/src/dispatch/expression.rs) — an
`EvaluationLookup` adapter that exposes an `EventMeta` to Darkmatter's shared
expression evaluator. It is the resolution layer used by:

- Dispatch template interpolation (`dispatch::template::interpolate`)
- Event binding matchers (`dispatch::matcher::RuntimeMatcher::Expression`)
- Harness validation message rendering (`harness::validate::render_template`)

Hook action `when` clauses (`dispatch::runner::evaluate_when`) use a different
path: they serialize `EventMeta` to JSON and call Darkmatter's
`evaluate_condition_against(expr, json, work_dir)` shortcut, which constructs
its own `ShortcutLookup` over flat JSON. To make grouped paths
(`os.*`, `hardware.*`, `git.*`, `project.*`) resolve consistently against the
same surface that `EventMetaExpressionLookup` exposes, the runner manually
flattens those groups into top-level alias keys via
`flatten_event_meta_aliases` before handing the JSON to Darkmatter.

## Problem

`flatten_event_meta_aliases` is a deliberate mirror of
`EventMetaExpressionLookup::resolve_env_path`. The two layers are pinned by
unit tests today, but they remain two independent definitions of the same
path surface. Concretely:

- Adding, renaming, or removing a grouped path requires editing **both**
  `expression.rs` and `runner.rs`.
- Type fidelity is duplicated: `cores` must be a JSON `Number` (not a string)
  in **both** layers; `is_dirty` must be a JSON `Bool` in both; etc.
- Optional-field semantics (omit `git` when `None`, conditionally include
  `project` when either `primary_language` or `repo` is set) live in both
  layers and have to stay aligned by convention.
- The `tool_input.<path>` / `tool_response.<path>` / `extra.<path>` /
  `env.NAME` resolution paths are duplicated implicitly through Darkmatter's
  flat JSON-pointer lookup, which silently relies on `serde` field naming.

The first reviewer of `2026-04-29-leverage-dm-parser` flagged this as a
non-blocking suggestion (review-2.md, suggestion 2). This spec describes the
unification.

## Goals

1. **Single path surface.** Hook `when` evaluation, dispatch template
   interpolation, event binding matchers, and harness validation all resolve
   the same set of paths through one adapter.
2. **Preserve `ctx.*` support for hook `when`.** The existing flow goes
   through `evaluate_condition_against`, which provides Darkmatter's lazy
   `ctx.*` capture (e.g. `ctx.today`, `ctx.year`). That capability must
   survive the refactor. Templates/matchers/harness validation continue to
   leave `ctx.*` unresolved, matching today's behavior.
3. **No behavior change for any other surface.** Templates, matchers, and
   harness messages keep their current resolution semantics down to
   unknown-token preservation.
4. **Eliminate `flatten_event_meta_aliases`** and the `event_meta_to_json`
   wrapper that calls it. Their job moves into the shared adapter.

## Non-Goals

- Changing the hook `when` expression DSL (parse modes, operators, helper
  functions). The Darkmatter-level grammar stays exactly as-is.
- Adding new variables to the path surface. Variable additions are tracked
  separately as part of the broader template-variable roadmap.
- Reworking template/matcher/harness call sites. They already use
  `EventMetaExpressionLookup` directly and stay untouched.
- Touching reporting query filters or resource-linking filters. Both were
  intentionally deferred in `2026-04-29-leverage-dm-parser` and remain out
  of scope.

## Current State

```text
                      ┌─────────────────────────────────────────────┐
                      │ EventMetaExpressionLookup (expression.rs)   │
                      │ - env.NAME, extra.*, tool_input.*,          │
                      │   tool_response.*, os/hardware/git/project, │
                      │   top-level event fields                    │
                      │ - ctx.* deliberately unresolved (None)       │
                      └─────────────────────────────────────────────┘
                                    ▲
        ┌───────────────────────────┼─────────────────────────────┐
        │                           │                             │
templates (template.rs)    matchers (matcher.rs)     harness validate.rs

        ┌────────────────────────────────────────────────────────────┐
        │ Hook `when` (runner.rs::evaluate_when)                     │
        │   1. serde_json::to_value(meta)                            │
        │   2. flatten_event_meta_aliases(meta) injects top-level    │
        │      `os`, `hardware`, `git`, `project` aliases            │
        │   3. evaluate_condition_against(expr, json, work_dir)      │
        │      → Darkmatter's ShortcutLookup over flat JSON          │
        │      → adds `env.*` and `ctx.*` lazy capture               │
        └────────────────────────────────────────────────────────────┘
```

The duplication is the contract between `EventMetaExpressionLookup` and
`flatten_event_meta_aliases` plus the implicit reliance on serde-derived
field names for `tool_input` / `tool_response` / `extra`.

## Target State

```text
                      ┌─────────────────────────────────────────────┐
                      │ EventMetaExpressionLookup                   │
                      │ (single source of truth for event paths)    │
                      └─────────────────────────────────────────────┘
                                    ▲
        ┌───────────────────────────┼─────────────────────────────┐
        │                           │                             │
templates (template.rs)    matchers (matcher.rs)     harness validate.rs

                      ┌─────────────────────────────────────────────┐
                      │ EventMetaConditionLookup (NEW)              │
                      │ - composite EvaluationLookup                │
                      │ - ctx.* → Darkmatter ctx capture            │
                      │ - everything else → delegates to            │
                      │   EventMetaExpressionLookup                 │
                      └─────────────────────────────────────────────┘
                                    ▲
                                    │
                      Hook `when` (runner.rs::evaluate_when)
                      → parse_condition(expr) + evaluate(parsed, &lookup)
```

`flatten_event_meta_aliases` and `event_meta_to_json` are deleted. Hook
`when` evaluation no longer round-trips `EventMeta` through serde JSON.

## Design

### A. Darkmatter exposes a public `ctx.*` lookup

The blocker for a pure claudine-side change is that Darkmatter's
`ShortcutLookup` and the underlying `ContextGroup`, `for_key`, and
`capture_runtime_context_for_groups` items are all `pub(crate)`. Without one
of those exposed, claudine cannot synthesize lazy `ctx.*` resolution against
its own data.

The minimal Darkmatter change is to publish a thin **`CtxLookup`** type
under `darkmatter::markdown::compose::expression` (or a sibling module)
with the following surface:

```rust
/// Lazy-capturing `ctx.*` resolver, suitable for composing on top of any
/// `EvaluationLookup` that handles non-`ctx` paths.
pub struct CtxLookup<'a> {
    work_dir: &'a Path,
    cache: RefCell<HashMap<String, Value>>,
    captured: RefCell<HashSet<ContextGroup>>,
}

impl<'a> CtxLookup<'a> {
    pub fn new(work_dir: &'a Path) -> Self { /* ... */ }

    /// Returns `Some(value)` for `ctx.<key>` paths whose context group can
    /// be captured; `None` otherwise. Non-`ctx` paths always return `None`.
    pub fn resolve_ctx(&self, path: &str) -> Option<Value> { /* ... */ }
}

impl<'a> EvaluationLookup for CtxLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        self.resolve_ctx(path)
    }
}
```

Internally this is the body of `ShortcutLookup::get`'s existing `ctx.*`
branch, lifted into a standalone struct. `ShortcutLookup` itself can be
refactored to compose `CtxLookup` + JSON-data lookup + `env.*` resolution,
keeping the existing public `evaluate_condition_against` shortcut working
unchanged.

This is a non-breaking, purely additive change to Darkmatter.

### B. Claudine introduces a composite `EventMetaConditionLookup`

In `claudine/lib/src/dispatch/expression.rs`, add a new adapter beside
`EventMetaExpressionLookup`:

```rust
use darkmatter::markdown::compose::expression::{CtxLookup, EvaluationLookup};

/// Composite lookup used for hook `when` evaluation.
///
/// Resolves `ctx.*` via Darkmatter's lazy context capture and delegates
/// every other path to [`EventMetaExpressionLookup`]. This is the only
/// surface where `ctx.*` is honored — templates, matchers, and harness
/// validation deliberately leave `ctx.*` unresolved.
pub struct EventMetaConditionLookup<'a> {
    inner: EventMetaExpressionLookup<'a>,
    ctx: CtxLookup<'a>,
}

impl<'a> EventMetaConditionLookup<'a> {
    pub fn new(meta: &'a EventMeta, work_dir: &'a Path) -> Self {
        Self {
            inner: EventMetaExpressionLookup::new(meta),
            ctx: CtxLookup::new(work_dir),
        }
    }
}

impl<'a> EvaluationLookup for EventMetaConditionLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        if path == "ctx" || path.starts_with("ctx.") {
            return self.ctx.get(path);
        }
        self.inner.get(path)
    }
}
```

Notes:

- `EventMetaExpressionLookup::get` already returns `None` for `ctx.*`, so
  the explicit prefix check above is a small optimization, not a
  correctness requirement. Keeping it makes the routing explicit and
  matches how reviewers will read the code.
- The `work_dir` argument mirrors what `evaluate_when` already computes
  today (`meta.cwd → std::env::current_dir → "."`). The composite owns
  the `CtxLookup`, so the `work_dir` borrow only needs to live for the
  duration of one expression evaluation.

### C. Rewire `evaluate_when`

Replace the `event_meta_to_json` + `evaluate_condition_against` round-trip
with direct `parse_condition` + `evaluate` against the composite lookup:

```rust
use darkmatter::markdown::compose::conditions::parse_condition;
use darkmatter::markdown::compose::expression::evaluate;
use darkmatter::markdown::compose::expression::is_truthy; // if not pub, see §F

fn evaluate_when(when: Option<&str>, meta: &EventMeta) -> WhenOutcome {
    let Some(expr) = when else { return WhenOutcome::Run; };

    let work_dir = meta
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let parsed = match parse_condition(expr) {
        Ok(parsed) => parsed,
        Err(error) => {
            warn!(expression = expr, %error, "Hook action `when` failed to parse; skipping action");
            return WhenOutcome::SkipInvalid;
        }
    };

    let lookup = EventMetaConditionLookup::new(meta, work_dir.as_path());
    match evaluate(&parsed, &lookup) {
        Ok(value) if is_truthy(&value) => WhenOutcome::Run,
        Ok(_) => WhenOutcome::SkipFalse,
        Err(error) => {
            warn!(expression = expr, %error, "Hook action `when` failed to evaluate; skipping action");
            WhenOutcome::SkipInvalid
        }
    }
}
```

The `meta_json: &Value` parameter that `evaluate_when` accepts today is
removed; `execute_actions` no longer needs to build `meta_json` up front.

### D. Delete `flatten_event_meta_aliases` and `event_meta_to_json`

After §C, these helpers in `dispatch/runner.rs` have no callers. Both
helpers, their doc comments, and the unit tests pinning their alias
surface (`flatten_event_meta_aliases_*`) are removed. The conceptual
contract those tests were enforcing — that grouped paths resolve the same
way under hook `when` as under template/matcher evaluation — moves into
new tests on the composite lookup (see "Testing").

### E. Behavior parity invariants

The refactor must preserve the following observable behavior, all of which
is covered today by `dispatch::runner::tests::when*`:

| Invariant | Today | After |
|---|---|---|
| `tool_name == 'Bash'` resolves against `EventMeta.tool_name` | ✓ | ✓ (via `EventMetaExpressionLookup`) |
| `git.branch == 'main'` resolves against `EventMeta.env.git.branch` | ✓ (via flatten) | ✓ (via `EventMetaExpressionLookup`) |
| `hardware.cores > 8` is a numeric comparison, not string | ✓ (Number in flatten) | ✓ (`EventMetaExpressionLookup` already returns `Value::Number`) |
| `git.is_dirty` is a JSON Bool, usable in `!git.is_dirty` | ✓ | ✓ |
| `env.CI || "local"` falls back to literal when env unset | ✓ (Darkmatter parses `||`) | ✓ |
| `ctx.today` resolves via Darkmatter context capture | ✓ | ✓ (preserved by `CtxLookup`) |
| Falsy result skips the action without affecting `selected_response` | ✓ | ✓ (no logic change in `execute_actions`) |
| Invalid expression produces `warn!` + skip, never aborts dispatch | ✓ | ✓ (parse/eval errors both routed to `SkipInvalid`) |
| Skipped `Call` cannot replace a previously selected blocking response | ✓ | ✓ (no logic change in `execute_actions`) |
| `extra.<path>`, `tool_input.<path>`, `tool_response.<path>` resolve nested JSON | ✓ (via serde keys + flat pointer) | ✓ (via `EventMetaExpressionLookup`) |

The third row deserves special attention: today's flatten path inserts
`hardware.cores` as `Value::Number`, but the *non-flatten* serde path also
emits it as `Number` since `EventMeta.env.hardware.cores: u32`. The
`EventMetaExpressionLookup` returns `Value::Number(meta.env.hardware.cores.into())`
identically, so no coercion drift is possible.

### F. Darkmatter API checklist

The unification depends on the following items being publicly available
from Darkmatter. Items already exported are noted; items that need to be
exposed are marked **NEW**:

| Item | Status |
|---|---|
| `darkmatter::markdown::compose::expression::EvaluationLookup` | exported |
| `darkmatter::markdown::compose::expression::Parser` / `ParseMode` | exported |
| `darkmatter::markdown::compose::expression::evaluate` | exported |
| `darkmatter::markdown::compose::conditions::parse_condition` | check at implementation time; if private, expose it (it is the same parser as `Parser::with_mode(_, ParseMode::Condition)` so an alternative is to call the parser directly and skip the helper) |
| `darkmatter::markdown::compose::expression::is_truthy` | check at implementation time; if private, claudine can inline a small `is_truthy(&Value) -> bool` helper since the rules are stable (`Bool→self`, `Null→false`, `Number→non-zero`, `String→non-empty`, `Array→non-empty`, `Object→non-empty`) |
| `CtxLookup` (lazy `ctx.*` resolver) | **NEW** — see §A |

If exposing `parse_condition` or `is_truthy` is undesirable on the
Darkmatter side, the Claudine-side fallbacks listed above keep this work
self-contained: only `CtxLookup` is a hard dependency.

## Implementation Plan

Sequential phases. Each phase is independently mergeable.

### Phase 1 — Darkmatter: Expose `CtxLookup`

1. In `darkmatter/lib/src/markdown/compose/expression/` (or a sibling
   `ctx.rs` module), extract the `ctx.*` resolution body of
   `ShortcutLookup::get` into a standalone `pub struct CtxLookup<'a>` with
   the API sketched in §A.
2. Refactor `ShortcutLookup` to embed `CtxLookup` so the existing
   `evaluate_condition_against` shortcut and all `ShortcutLookup` tests
   pass unchanged.
3. Add unit tests for `CtxLookup` covering: cached `ctx.today`, lazy
   capture trigger on first reference, `ctx.<unknown>` returning `None`,
   non-`ctx` paths returning `None`.
4. Document `CtxLookup` in the Darkmatter compose docs.

### Phase 2 — Claudine: Introduce `EventMetaConditionLookup`

1. Add `EventMetaConditionLookup` to `claudine/lib/src/dispatch/expression.rs`
   per §B, gated behind the new Darkmatter version.
2. Update the module rustdoc to describe the `Expression` vs `Condition`
   variants and which surfaces use each.
3. Add unit tests for the composite that mirror the existing
   `EventMetaExpressionLookup` tests for non-`ctx` paths plus a `ctx.today`
   test confirming lazy capture flows through to Darkmatter.

### Phase 3 — Claudine: Rewire `evaluate_when`

1. Replace `evaluate_condition_against` with `parse_condition` + `evaluate`
   against `EventMetaConditionLookup` per §C.
2. Drop the `meta_json: &Value` parameter from `evaluate_when` and remove
   the `event_meta_to_json(meta)` call site in `execute_actions`.
3. Confirm all `dispatch::runner::tests::when*` tests still pass; update
   the parse/eval-error path test (`when_invalid_expression_skips_action_non_fatally`)
   only if the warning text changes — the outcome shape (`SkipInvalid` +
   no blocking response) is unchanged.
4. Add a regression test for `ctx.*` in `when` (e.g.
   `when: "ctx.today != ''"`) that verifies the composite preserves the
   Darkmatter ctx-capture behavior.

### Phase 4 — Claudine: Delete the flattening layer

1. Delete `flatten_event_meta_aliases`, `event_meta_to_json`, and their
   `#[cfg(test)] mod tests` blocks in `dispatch/runner.rs`.
2. Remove the `Map`/`Value` imports that become unused.
3. Update `dispatch/runner.rs`'s module rustdoc to remove references to
   the alias surface and point readers at `EventMetaConditionLookup`.

### Phase 5 — Documentation sync

1. Update `claudine/docs/topics/configuring-actions.md` if it mentions the
   alias-flattening contract (it currently describes user-facing path
   surface, not the implementation, so likely unchanged).
2. Update `.claude/skills/claudine/architecture.md` and
   `.claude/skills/claudine/hook-actions.md` to note that hook `when`
   evaluation now flows through the same `EventMetaExpressionLookup`-based
   adapter as templates and matchers, with `ctx.*` layered on top.
3. Update the SKILL.md `dispatch expression bridge` paragraph to drop the
   "JSON flattening" wording and describe the composite lookup.

## Testing

### Unit tests (Level 1)

- `EventMetaConditionLookup` parity tests in
  `claudine/lib/src/dispatch/expression.rs` mirroring every
  `EventMetaExpressionLookup` test for non-`ctx` paths.
- `EventMetaConditionLookup` ctx test: with no real captured groups, a
  reference to `ctx.today` returns a non-empty string (Darkmatter's
  built-in capture for `DateTime` group).
- All existing `dispatch::runner::tests::when*` tests pass without changes
  beyond optional warning-text adjustments.
- New `dispatch::runner::tests::when_ctx_today_resolves` regression test.

### Behavior tests

- `cargo test -p claudine dispatch::expression` — composite lookup parity.
- `cargo test -p claudine dispatch::runner::tests::when` — `when` end-to-end.
- `cargo test -p claudine dispatch::matcher` — confirm matchers untouched.
- `cargo test -p claudine harness::validate::tests::render_template` —
  confirm harness messaging untouched.

No Level 2 (terminal) or Level 3 (interactive) tests apply — this feature
is pure expression-evaluation plumbing.

## Risks and Mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| Darkmatter's `CtxLookup` extraction subtly changes capture-trigger semantics for the existing `ShortcutLookup` | Low | Keep `ShortcutLookup` as a thin composition over `CtxLookup` + JSON data + env; rerun the full Darkmatter conditions test suite. |
| Claudine consumers depend implicitly on the JSON shape produced by `event_meta_to_json` | Very low | The flattened JSON was never exported; it only existed as the input to `evaluate_condition_against`. No public API change. |
| `parse_condition` is not a public Darkmatter export and the alternative path (using `Parser::with_mode(_, ParseMode::Condition)`) returns a slightly different error type | Low | Wrap the parse error in the same `tracing::warn!` shape and `SkipInvalid` outcome regardless of source. Tests check the outcome, not the error variant. |
| `is_truthy` is not exported and Claudine inlines a copy that drifts from Darkmatter | Low | Inline copy is documented as an exact mirror; if Darkmatter later exposes the helper, swap it back. The truthiness rules for JSON Values are stable. |
| Removing `event_meta_to_json` removes the only consumer of `serde_json::to_value(meta)` in this path; if other code relied on the synthesized `os`/`hardware` aliases at the JSON level, it would break | Very low | A `cargo check -p claudine` plus a grep for `event_meta_to_json` and `flatten_event_meta_aliases` confirms no other callers. |

## Out-of-Scope Follow-ups

- Reporting query filters (`reporting/queries.rs`) and resource-linking
  filters were intentionally excluded from `2026-04-29-leverage-dm-parser`
  and remain out of scope here.
- Extending the path surface to include broader harness validation
  variables (`source_file`, `cwd`, frontmatter fields) tracked by
  review-2.md suggestion 3 is independent of this unification and may be
  scheduled separately.

## Acceptance Criteria

- `flatten_event_meta_aliases` and `event_meta_to_json` no longer exist
  in the Claudine codebase.
- Hook `when` evaluation, dispatch templates, event binding matchers, and
  harness validation all derive their non-`ctx` path resolution from
  `EventMetaExpressionLookup`.
- `ctx.*` continues to resolve under hook `when` and continues to be
  unresolved under templates/matchers/harness validation.
- All existing tests in `dispatch::runner::tests::when*`,
  `dispatch::template`, `dispatch::matcher`, and
  `harness::validate::tests::render_template` pass without behavioral
  drift.
- One new test confirms `ctx.*` survives the refactor in hook `when`.
- Skill docs (`.claude/skills/claudine/architecture.md`,
  `.claude/skills/claudine/hook-actions.md`, `SKILL.md`) describe the
  composite adapter rather than JSON flattening.
