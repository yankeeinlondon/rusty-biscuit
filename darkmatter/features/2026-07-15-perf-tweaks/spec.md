---
status: draft
reviewed: false
created: 2026-07-15
inputs:
  - ../../lib/src/markdown/schemas/mod.rs
  - ../../lib/src/markdown/compose/context/options.rs
  - ../../lib/src/markdown/compose/schema_validation.rs
  - ../../lib/src/markdown/compose/cache/hashing.rs
  - ../../lib/tests/base_schema_end_to_end.rs
  - ../../lib/benches/effective_schema_ownership.rs
  - ../../dmls/src/overlay/schema.rs
  - ../../../claudine/gen/src/inputs.rs
related:
  - ../../reviews/2026-07-12-perf
---

# Arc-Backed Schema Baseline Performance Tweaks

## Status

Draft. The read-only callsite cleanup is ready for review. The
`ComposeOptions` representation change is contingent on a saved Criterion
baseline demonstrating that the remaining built-in-baseline ownership cost is
material outside harness noise.

## Summary

Finding 29 of the Darkmatter performance review changed
`EffectiveSchema::json_schema` from `serde_json::Value` to `Arc<Value>` and
added two zero-deep-clone built-in-baseline surfaces:

- `darkmatter_base_json_schema_ref() -> &'static Value` for read-only access.
- `DarkmatterSchemas::with_darkmatter_baseline_json_schema()` for configuration.

The production schema paths already use these faster surfaces. No production
caller directly invokes the source-compatible owned accessor
`darkmatter_base_json_schema() -> Value`, and no external consumer was found
materializing an owned `Value` from `EffectiveSchema::json_schema`.

Two residual opportunities remain:

1. Five read-only tests still deep-clone the built-in JSON Schema through the
   owned accessor. They can use the borrowed accessor without changing what the
   tests prove. Intentional ownership tests and benchmark controls must retain
   the owned accessor.
2. `ComposeOptions::with_darkmatter_baseline_schema()` still clones the cached
   `SimplifiedSchema` into `ComposeOptions`, even though schema validation later
   recognizes the built-in baseline and switches to the Arc-backed compiled JSON
   Schema. The existing Criterion harness measures `DarkmatterSchemas`
   configuration, not this outer options-construction path.

This feature removes the unnecessary read-only test clones and establishes a
measurement gate for eliminating the remaining built-in `ComposeOptions` clone.

## Goals

1. Use the borrowed JSON Schema accessor at every direct read-only callsite.
2. Retain and explicitly test the independently owned, mutable accessor contract.
3. Retain the owned accessor benchmark as a compatibility and performance
   control.
4. Add a same-source Criterion baseline for built-in baseline configuration
   through `ComposeOptions`.
5. If the benchmark shows a repeatable win beyond harness noise, represent the
   built-in baseline in `ComposeOptions` without cloning `SimplifiedSchema`.
6. Preserve custom baseline behavior, cache identity, validation results, and
   public Rust signatures.

## Non-Goals

- Removing or changing `darkmatter_base_json_schema() -> Value`.
- Exposing the private cached `Arc<Value>` through another public accessor; a
  borrowed `&'static Value` is sufficient for read-only callers, and the
  built-in builder encapsulates Arc ownership for configuration.
- Converting custom caller-supplied baselines to the built-in baseline path.
- Avoiding owned values when a schema is actually merged or mutated, including
  DMLS extension overlays.
- Changing `EffectiveSchema::json_schema: Arc<Value>` again.
- Optimizing `darkmatter_base_schema()` consumers that need an owned
  `SimplifiedSchema` or mutable `SchemaShape`.
- Folding unrelated findings from the 2026-07-12 performance review into this
  feature.

## Current Contracts

### Owned compatibility accessor

```rust
pub fn darkmatter_base_json_schema() -> Value
```

Each call deep-clones the process-cached schema. This is intentional: callers
receive an independently owned value that they may mutate. The API remains
source-compatible and continues to be documented and tested.

### Borrowed read-only accessor

```rust
pub fn darkmatter_base_json_schema_ref() -> &'static Value
```

This returns a stable borrow of the process-cached Arc allocation and performs
no deep clone.

### Built-in baseline builder

```rust
DarkmatterSchemas::new().with_darkmatter_baseline_json_schema()
```

The builder Arc-clones the cached compiled schema. Compose schema validation and
the DMLS no-extension path already use it.

### Generic owned builders

`DarkmatterSchemas::with_baseline(...)`,
`DarkmatterSchemas::with_baseline_json_schema(...)`, and
`ComposeOptions::with_baseline_schema(...)` accept caller-owned custom schemas.
They remain owned paths; custom values cannot be replaced by the built-in Arc
without changing their meaning.

## Callsite Inventory

The monorepo contains nine executable calls to
`darkmatter_base_json_schema()` across seven caller functions. GitNexus reports
HIGH upstream impact because all seven call the public accessor directly, but
none participates in a production execution flow: the callers are tests and the
ownership benchmark.

### Read-only calls to migrate

| Caller | Location | Replacement |
|---|---|---|
| `base_schema_file_parses_and_converts` | `lib/tests/base_schema_end_to_end.rs` | Borrow with `darkmatter_base_json_schema_ref()` |
| `schema_document_transcludes_same_file_as_library_source` | `lib/tests/base_schema_end_to_end.rs` | Compare `&file_json` with the borrowed schema |
| `base_schema_ctx_is_darkmatter_owned_generated_context` | `lib/tests/base_schema_end_to_end.rs` | Index through the borrowed schema |
| `darkmatter_base_json_schema_validates_known_samples` | `lib/src/markdown/schemas/mod.rs` | Build the validator from the borrowed schema |
| `darkmatter_base_json_schema_allows_unknown_keys` | `lib/src/markdown/schemas/mod.rs` | Build the validator from the borrowed schema |

The setup value in `bench_default_baseline_initialization` may also borrow the
cached schema for property counting and merge inputs. This setup runs outside
the measured loop, so the change improves benchmark hygiene rather than a
reported production result.

### Owned calls to retain

| Caller | Reason |
|---|---|
| `darkmatter_base_json_schema_is_cached` | Proves the owned value can be mutated without changing the cached borrowed schema |
| `owned_accessor` benchmark case | Measures the cost of the retained owned compatibility contract |
| `bench_effective_schema_ownership` Darkmatter baseline input | Exercises the generic owned-baseline path alongside the synthetic owned control |

The public rustdoc example also remains owned because it documents the owned
contract intentionally.

### Production paths already using the faster contract

- Compose schema validation calls
  `with_darkmatter_baseline_json_schema()` for the built-in baseline.
- DMLS calls the Arc-backed builder when no extension matches, borrows the
  built-in baseline as the lower merge layer when an extension does match, and
  materializes a `Value` only because merging must produce an owned result.
- Claudine reads `EffectiveSchema::json_schema` by reference when coercing
  frontmatter.
- Darkmatter validation, coercion, rewriting, and diagnostics borrow the Arc
  contents rather than deep-cloning them.

## Phase 1 — Read-Only Callsite Cleanup

1. Import `darkmatter_base_json_schema_ref` in the integration test.
2. Replace the five read-only owned-accessor calls listed above.
3. Update test documentation that specifically names the owned accessor when
   the test is about compiled-schema behavior rather than ownership.
4. Use the borrowed accessor for untimed benchmark setup where ownership is not
   under measurement.
5. Retain the ownership-contract test and both owned benchmark controls.

### Phase 1 completion criteria

- Direct owned-accessor calls remain only where independent ownership is the
  behavior under test or measurement.
- The cached-borrow identity and independently mutable owned-value tests pass.
- The existing ownership benchmark still contains a named `owned_accessor`
  control.
- `just test`, `just test-l2`, and `just lint` pass for Darkmatter.

Phase 1 is a test and benchmark-hygiene change. It is not expected to improve
production wall time and must not be presented as a production performance win.

## Phase 2 — Measure the Outer ComposeOptions Path

The current ownership benchmark proves that
`DarkmatterSchemas::with_darkmatter_baseline_json_schema()` is effectively
zero-clone, but it does not include:

```rust
ComposeOptions::new_with_context(context)
    .with_darkmatter_baseline_schema()
```

That public builder currently obtains an owned `SimplifiedSchema` through
`darkmatter_base_schema()` and stores it in
`baseline_schema: Option<SimplifiedSchema>`. The
`baseline_is_darkmatter_default` flag later selects the Arc-backed compiled
schema during validation, but the options object and its clones still carry the
owned simplified representation. `options_hash` also converts that simplified
schema back to JSON when computing cache identity.

### Required Criterion cases

Extend `effective_schema_ownership` or add a narrowly named schema-options
benchmark with these cases:

1. Configure a cheap-context `ComposeOptions` with the built-in baseline.
2. Clone configured built-in-baseline options, modeling child-pipeline
   propagation.
3. Configure and clone an equivalently sized custom baseline as a control.
4. Compose a baseline-only document through the public options path.
5. Compose a baseline-plus-document-schema case through the same path.
6. Keep a no-baseline control so unrelated options and compose costs remain
   visible.

The benchmark must use `new_with_context` with a fixed or demand-driven empty
context so eager runtime-context capture does not mask schema ownership costs.
Save a named Criterion baseline before changing the options representation.
Record 100 samples per function, dispersion, benchmark source identity, and the
host preflight used by the existing ownership benchmark.

### Measurement ruling

Before implementation, the baseline report must declare numeric target and
regression thresholds. Phase 2 proceeds only if the built-in configuration or
clone cases show a repeatable cost beyond harness noise and the proposed change
can improve them without regressing custom-baseline or no-baseline controls.

If no repeatable opportunity remains, record that result and close Phase 2
without changing `ComposeOptions`. Measurement-based closure is not a deferral.

## Phase 3 — Conditional ComposeOptions Representation

Phase 3 runs only after Phase 2 authorizes it.

### Proposed internal model

Replace the combination of:

```rust
baseline_schema: Option<SimplifiedSchema>
baseline_is_darkmatter_default: bool
```

with one crate-private representation that distinguishes absence, the built-in
baseline, and a caller-owned custom baseline. For example:

```rust
enum BaselineSource {
    None,
    DarkmatterBuiltIn,
    Custom(SimplifiedSchema),
}
```

The exact name and layout are implementation details. The required semantics
are:

- `with_darkmatter_baseline_schema()` records a zero-clone built-in marker.
- `with_baseline_schema(schema)` stores the custom owned schema unchanged.
- `Clone` of built-in-baseline options copies only the marker.
- Schema validation maps the marker directly to
  `with_darkmatter_baseline_json_schema()`.
- Custom baseline conversion and validation remain unchanged.
- Debug output continues to distinguish configured from absent baseline without
  dumping schema contents.

### Cache identity

The built-in marker must hash the same canonical schema meaning as the current
owned built-in baseline. Do not hash only the enum discriminant: changes to the
checked-in Darkmatter baseline must still invalidate compose artifacts.

Prefer a cached canonical hash of the compiled built-in JSON Schema. A custom
baseline that is structurally equivalent to the built-in schema may share cache
identity because its validation semantics are the same. Cache identity must not
depend on Arc addresses or process-local initialization order.

### Compatibility

The public signatures of `ComposeOptions::with_baseline_schema` and
`with_darkmatter_baseline_schema` remain unchanged. Existing callers do not
observe the internal representation. The following behavior must remain
byte-for-byte and error-for-error compatible:

- Baseline-only validation.
- Document schema overriding a baseline property.
- Trigger-schema assembly with a baseline.
- Recursive transclusion inheriting the baseline.
- Run-local and persistent cache separation between semantically different
  baselines.
- `DARKMATTER_NO_BASELINE_SCHEMA` and CLI custom-baseline behavior.

### Phase 3 completion criteria

- Built-in baseline configuration and `ComposeOptions` cloning perform no deep
  `SimplifiedSchema` clone.
- The saved Criterion comparison exceeds the predeclared target threshold.
- Custom-baseline and no-baseline controls show no statistically credible
  regression.
- Cache-key regression tests distinguish different custom baselines and
  invalidate when the checked-in built-in schema changes.
- Compose, DMLS, and Claudine schema behavior remains unchanged.
- Darkmatter `just test`, `just test-l2`, and `just lint` pass.
- Relevant Claudine and DMLS package tests/checks pass.
- `cargo check --workspace` passes on the supported host, with Windows and Linux
  CI compile/test evidence recorded before completion.

## Documentation

- Keep the schema-definition documentation explicit that
  `darkmatter_base_json_schema()` returns an independent owned value and
  `darkmatter_base_json_schema_ref()` is the read-only fast path.
- Document that `with_darkmatter_baseline_schema()` uses the process-cached
  built-in schema without promising the internal enum or Arc layout.
- Record Phase 2 and, if authorized, Phase 3 Criterion results in a results
  document alongside this spec.
- If Phase 3 does not proceed, record the no-material-win ruling rather than
  leaving it silently deferred.

## Review Questions

1. Is the Phase 1 test cleanup valuable as usage guidance even though it has no
   production wall-time effect?
2. Should the Phase 2 benchmark extend `effective_schema_ownership` or use a
   separate `compose_options_schema_ownership` target?
3. Is semantic canonical-schema equality the correct cache identity when a
   custom baseline happens to equal the built-in baseline?
4. If Phase 3 proceeds, should the internal representation use an enum as shown,
   or retain the existing fields with a zero-clone built-in sentinel?
