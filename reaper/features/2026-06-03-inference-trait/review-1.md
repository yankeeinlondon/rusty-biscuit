---
agent: claude
model: claude-opus-4-8[1m]
ready: true
iteration: 1
reviewed: 2026-06-11
---

# Review — Inference Trait Contract (`biscuit-contract`)

> Iteration #1

## Verdict

**Ready for production.** The `biscuit-contract` crate is a small, pure in-process
Rust contract that faithfully implements the normative shape in the spec. Every
public type, derive, and trait bound matches the specification; all
spec-mandated L1 tests are present and pass; `cargo clippy -D warnings` is clean;
and the package area is fully wired into the workspace, root dependency docs,
root `justfile` curated area list, and the local skill catalog.

The findings below are quality and coverage improvements, none of which block
release. They should be folded into this crate or picked up by the first
follow-up that touches it.

## Verification summary

| Check | Result |
|-------|--------|
| `cargo test -p biscuit-contract` | 7 unit tests + 1 doctest, all green |
| `cargo clippy -p biscuit-contract -- -D warnings` | clean |
| `cargo metadata` lists `biscuit-contract` | yes (`biscuit-contract/lib`) |
| Root `Cargo.toml` workspace member | yes |
| Root `docs/dependencies.md` entry | yes |
| Root `justfile` curated `areas` list | yes |
| `.claude/skills/biscuit-contract/SKILL.md` | present |
| Forbidden deps (tokio prod / HTTP / schema engine / consumer crate) | none; tokio is dev-only |

## Test-rigor classification (L1 / L2 / L3)

This crate exposes **no user-observable terminal, rendering, input, or UX
behavior**. It is a provider-neutral, in-process Rust API: traits, owned data
structs, and an error type. Per the spec ("All contract-crate tests are L1 and
deterministic"), **every requirement is correctly verified at Level 1**, which is
the appropriate level for each one. There is no requirement of the form "when the
user does X, Y renders," so no L2 (real-terminal capture) or L3 (OS keyboard
injection) verification is owed.

The `justfile` correctly stubs `test-l2`, `test-l3`, `test-browser`, and
`test-real` as explicitly *not applicable* rather than silently omitting them —
this is the right call and leaves no level mismatch. Real-provider conformance
(structured output validated against a real JSON Schema engine) is correctly
deferred to the Claudine / Unchained AI follow-up specs under the `real_` tier,
as the spec dictates.

**No level mismatch found.** No requirement's strongest test sits at the wrong level.

## Spec-to-implementation coverage

Every item in the normative `Contract` block is implemented exactly:

- `InferenceAdapter` — `#[async_trait]`, `Send + Sync` supertrait, single
  `infer(&self, InferenceRequest) -> Result<InferenceResponse, InferenceError>`.
  Default (Send) `async_trait` bounds, not `?Send`. ✓
- Object safety is proven **twice** — a hidden `_assert_*` fn taking
  `Arc<dyn InferenceAdapter>` and a `no_run` doctest. ✓
- All enums/structs (`InferenceRequest`, `InferenceOutput`, `InferenceProfile`,
  `InferencePriority`, `ReasoningEffort`, `InferenceResponse`, `InferenceData`,
  `InferenceMetadata`, `InferenceErrorKind`, `InferenceError`) match the spec's
  fields, variants, defaults (`Balanced`, `Medium`), and derive sets. ✓
- No `Serialize`/`Deserialize` derives on request/response types. ✓
- `InferenceError` uses `thiserror` `#[error("{message}")]`, implementing
  `Error` + `Display`; `retry_after: Option<Duration>` present. ✓
- Dependency discipline (`async-trait`, `serde_json`, `thiserror` only; tokio
  dev-only) matches `Dependency Direction`. ✓

Spec-mandated tests, all present and passing:
`Arc<dyn>` storage/call; prose round-trip; structured round-trip; profile
defaults; construct + exhaustively match every `InferenceErrorKind`; variant
mismatch → `InvalidResponse`; schema violation → `InvalidResponse`.

## Findings

### F1 — `with_retry_after` / `retry_after` path has zero test coverage (Medium)

`InferenceError::with_retry_after` and the populated `retry_after` field are
never exercised by any test. The spec calls out `retry_after` semantics
explicitly ("populated only when the provider supplies a meaningful delay,
normally for `RateLimited` or `Unavailable`"), and the builder is `#[must_use]`,
yet nothing verifies that `with_retry_after(d)` actually sets `Some(d)` or that
`new(..)` leaves it `None`. This is the only public method in the crate with no
coverage.

**Recommendation:** add a small synchronous test, e.g.

```rust
#[test]
fn retry_after_builder_sets_and_defaults() {
    let base = InferenceError::new(InferenceErrorKind::RateLimited, "slow down");
    assert_eq!(base.retry_after, None);
    let with = base.clone().with_retry_after(Duration::from_secs(2));
    assert_eq!(with.retry_after, Some(Duration::from_secs(2)));
}
```

### F2 — `InferenceError` Display / `std::error::Error` impl is asserted only indirectly (Medium)

The spec mandates "`InferenceError` must implement `std::error::Error` and
`Display`." No test pins the observable behavior of that contract — that
`err.to_string()` yields `message`, or that the value coerces to
`Box<dyn std::error::Error>`. The existing tests only check `!err.message.is_empty()`,
which would still pass if the `#[error(...)]` format string drifted (e.g. to
`"{kind:?}: {message}"`).

**Recommendation:** assert the Display contract directly:

```rust
#[test]
fn error_displays_message_and_is_std_error() {
    let err = InferenceError::new(InferenceErrorKind::Provider, "boom");
    assert_eq!(err.to_string(), "boom");
    let _boxed: Box<dyn std::error::Error> = Box::new(err);
}
```

### F3 — README usage example is not a compiled doctest (Low)

The `## Usage` block in `biscuit-contract/README.md` is a `rust` fenced block but
the README is not pulled into the crate (`#[doc = include_str!("../README.md")]`
is not used), so the example is never compiled and can silently drift from the
API. The in-crate doctest on `InferenceAdapter` *is* compiled and covers the
object-safety claim, so this is low severity.

**Recommendation (optional):** either wire the README into the lib docs via
`#![doc = include_str!("../../README.md")]` so the example is doctested, or accept
the drift risk and rely on the existing in-crate doctest. Given the crate's
stability, accepting the risk is reasonable.

### F4 — Ergonomic opportunity: no convenience constructors (Low / informational)

Consumers must build `InferenceRequest` and `InferenceResponse` with full struct
literals every call site, and must hand-match `InferenceData` to extract a payload.
The spec explicitly permits "constructors, and convenience conversions [that]
follow local style." Adding a few thin helpers would reduce boilerplate in the
four downstream consumers without widening the contract:

- `InferenceRequest::prose(prompt)` / `InferenceRequest::structured(prompt, schema)`
  (each defaulting `profile`).
- `InferenceData::as_prose(&self) -> Option<&str>` /
  `as_structured(&self) -> Option<&Value>`.

These are additive and non-breaking, so they can land here or in the first
consumer follow-up. Not a defect.

### F5 — Test helper `is_object_schema` is narrower than the contract (informational)

`FakeStructuredAdapter`'s `is_object_schema` rejects any schema that is not
`type: "object"`, while the spec states "The schema may describe any valid JSON
value, not only an object." This is harmless because it is a *fake* adapter, not
the contract, but a reader could mistake it for a contract rule. A one-line
comment already hedges this; no change required.

## What was checked and found correct

- Default `#[async_trait]` (Send) bounds on both the trait and every impl — no
  `?Send` leak.
- `Eq` derivability holds across all types (`Duration`, `String`, enums all `Eq`).
- No async runtime, HTTP client, or JSON Schema engine in `[dependencies]`; tokio
  confined to `[dev-dependencies]` with only `macros` + `rt`.
- Stray `*-timing.jsonl` artifacts in the package dir are git-ignored — they will
  not be committed.
- Edition 2024, version `0.1.0`, matching the Compatibility section.

## Bottom line

The contract is implemented to spec, deterministically tested at the correct
level, lint-clean, and fully integrated into the repo's tooling and docs. F1 and
F2 are worth a few lines of test before or shortly after merge; F3–F5 are
optional polish. None gate production.
