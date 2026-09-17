# Error Taxonomy: Checked `ctx.*` Lookup

This is the Phase 1 contract for Phases 2 and 3. It covers what a checked
`ctx.*` lookup returns and which errors it raises. Names are binding unless a
later phase records a reason to change one here.

## Checked outcome (Phase 2, crate-private)

```rust
/// Classification of one `ctx.<key>` read against an authoritative snapshot.
pub(crate) enum CtxLookupOutcome {
    /// The owning group was captured and the projection holds the key. The value
    /// may be a typed `null`, `""`, `[]`, or `{}`; all of them are legitimate.
    Present(serde_json::Value),
    /// The key is cataloged, but `capture_requirements()` lacks its group.
    NotCaptured { key: String, group: ContextGroup },
    /// The group was captured, but its projection omitted a cataloged key.
    ProjectionMissing { key: String, group: ContextGroup },
    /// `ContextGroup::for_key` returned `None`. The existing
    /// unknown-context-variable path owns this case.
    Unknown,
}
```

It is classified only through `ContextGroup::for_key`,
`context_variable_descriptors()`, and `ComposeContext::capture_requirements()`.
No second key list exists. The group check runs **before** the merged
`data["ctx"]` map is read (Ruling 1). When the group *is* captured, the
existing merge and override order (user `ctx:`, then context values, then
legacy date fields) decides `Present`.

`Unknown` keeps today's lookup result: `None`, which renders as empty and
raises the unknown-variable warning. It never raises either new error
(spec §7).

## Evaluator channel

`EvaluationLookup::get` has no error channel. Add one defaulted method:

```rust
fn get_checked(&self, path: &str) -> Result<Option<Value>, ExpressionError> {
    Ok(self.get(path))
}
```

- `Expr::Variable` evaluation calls `get_checked`.
- `EffectiveState`, `ResolvingLookup`, `FrontmatterSeedState`, `LayeredLookup`,
  and `ShortcutLookup` override it for `ctx.`-prefixed paths.
- Every other implementation (the test fixtures) keeps the default.
- `get` stays as the unchecked convenience for non-compose callers. No compose
  stage may read `ctx.*` through it.

## New `ExpressionError` variants (public)

Per `expression/error.rs`, these variants are flat by design. The source
identity and authored span come from the wrapper that already carries them,
`MarkdownError::Interpolation { key, expression, source, cause }`.

```rust
/// A cataloged `ctx.*` variable whose owning group the composition request
/// never captured. This is a caller or pipeline contract violation, not a
/// fact about the host.
#[error(
    "context variable `ctx.{key}` belongs to the {group:?} capture group, \
     which this composition request did not capture; capture the document's \
     `ContextRequirements` (e.g. `ComposeContext::capture_for_document`) or \
     build options with `ComposeOptions::new()`"
)]
ContextNotCaptured { key: String, group: ContextGroup },

/// A captured group whose projection omitted one of its own cataloged keys.
/// This is a Darkmatter internal invariant failure, and the message must not
/// blame the caller.
#[error(
    "internal error: the {group:?} context capture did not project cataloged \
     variable `ctx.{key}`; this is a Darkmatter bug"
)]
ContextProjectionInvariant { key: String, group: ContextGroup },
```

- `key` holds the bare catalog key (`repo_root`). Display renders the full
  path, `ctx.repo_root`.
- Both variants are `is_authoring_fatal() == true`, so lenient mode never
  downgrades them (Ruling 2). They are added to the
  `fatality_characterization.rs` matrix for the Body and FrontmatterWholeValue
  surfaces.
- The exact caller-guidance wording is for Phase 3 to finalize. Its tests
  assert the variant, `key`, and `group`, never the prose.

## Wrappers

| Surface (see `surfaces.md`) | Today | Required |
|---|---|---|
| #1, #2, #7, #15 interpolation | `MarkdownError::Interpolation{cause}` | unchanged: the cause is the new variant |
| #5 page block, #6 transclusion `when=` | `ConditionError::Eval(String)`-style, stringly | carry `ExpressionError` as a typed `#[source]` so a caller can downcast to `ContextNotCaptured` |
| #8 `$()` ternary | `ShellExpansionError::ParseDirective` built by `format!` | same: typed `#[source]` |
| #12 standalone | `ConditionError::Eval` | same typed source; standalone only ever raises `ContextProjectionInvariant`, because it captures on demand |
| #10, #11 tolerated child failure | non-structural errors become a notice plus a warning | a `MarkdownError` whose cause chain contains either variant is **structural** and propagates |

The CLI renders these through the existing typed compose-error path and exits
nonzero. No `MarkdownError::Transform(String)` may carry them.

## Test-only malformed snapshot (Ruling 7)

Add `#[cfg(test)] pub(crate) fn ComposeContext::with_projection_key_removed(self, key: &str) -> Self`
next to `fixed_for_testing`. It keeps `captured` unchanged and removes `key`
from `values`. Because it is `cfg(test)`, integration tests in `lib/tests/`
cannot reach it, so the invariant test is a crate-internal L1 test.

## As implemented in Phase 2

- `CtxLookupOutcome` lives in `context/checked.rs`, and its single classifier
  is `CtxLookupOutcome::classify`. `ComposeContext::classify_ctx_key` and
  `CtxLookup::resolve_ctx_checked` both use it. `ContextGroup::projected_keys`
  (keys plus date/time aliases) is the one key table, and `group_for_key` now
  derives from it.
- **Staging switch:** `NotCaptured` maps to `ContextNotCaptured` only under
  `NotCapturedPolicy::Fatal`. Compose lookups apply
  `COMPOSE_NOT_CAPTURED_POLICY = Legacy`, which returns the pre-classification
  answer, until Phase 3 wires every surface. Phase 3 flips the constant to
  `Fatal`, then deletes the policy. `ProjectionMissing` is enforced now.
- Bare-name fallback (R8) goes through `into_checked_bare_name`: under `Fatal`,
  `NotCaptured` resolves to `None`.
- The fatality matrix runs both new kinds through an enforcing adapter over
  `EffectiveState::get_checked_with(path, Fatal)`.

## As implemented in Phase 3

- The staging switch is gone: `NotCapturedPolicy` and
  `COMPOSE_NOT_CAPTURED_POLICY` were deleted, and `into_checked` raises
  `ContextNotCaptured` directly.
- Wrappers:
  - `ConditionError::Eval` and `TransclusionError::ConditionEval` replaced
    `message: String` with `#[source] cause: Box<ExpressionError>`.
  - `$()` ternary interpolation and evaluation failures use the new
    `ShellExpansionError::ExpressionEvaluation { ctx, origin, message, #[source] cause }`.
    Parse failures stay `ParseDirective`.
- `ExpressionError::is_missing_runtime_context()` and
  `MarkdownError::missing_runtime_context()` (a cause-chain walk that also
  unboxes `Box<ExpressionError>`) are public.
- The tolerated-transclusion structural set includes any error whose chain
  holds either variant.
- Body interpolation errors now carry `SourceRef::OnDisk` when the document has
  a file locus. `interpolation_block` renders a dedicated
  "runtime context not captured" or "runtime context projection invariant"
  block with a "Defined in:" link.
