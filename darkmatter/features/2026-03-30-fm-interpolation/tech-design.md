# Frontmatter Interpolation Tech Design

This document defines the implementation-ready technical design for the `frontmatter-interpolation` feature in Darkmatter. It is derived from:

- `darkmatter/features/2026-02-30-fm-interpolation/spec.md`
- the current compose pipeline in `darkmatter/lib/src/markdown/compose/mod.rs`
- the current effective-state model in `darkmatter/lib/src/markdown/compose/state.rs`
- the current interpolation parser/evaluator in `darkmatter/lib/src/markdown/compose/interpolation/`
- the current frontmatter container in `darkmatter/lib/src/markdown/frontmatter.rs`

The design goal is to let frontmatter values derive other frontmatter values before the normal body interpolation stage runs, without introducing a second expression language or breaking current parent-state propagation semantics.

## Purpose

Darkmatter already supports interpolation in the Markdown body:

```md
The spec is at {{ spec }}.
```

It does not yet support resolving interpolated values inside frontmatter itself:

```yaml
---
base: /tmp/project
spec: "{{base}}/spec.md"
---
```

That gap matters because frontmatter is not just metadata in Darkmatter. It also drives:

1. later body interpolation
2. frontmatter transclusion fields such as `prologue` and `epilogue`
3. child-document inherited state during transclusion
4. any other compose-time behavior that reads document state before rendering

The feature should therefore be treated as a compose-time state-shaping step, not as a formatting convenience.

## Goals

1. Resolve interpolated frontmatter values after inherited parent state, external state defaults, and `--set` overrides have been merged into the current document.
2. Reuse the existing interpolation grammar and evaluator semantics.
3. Mutate the document frontmatter before later compose stages consume it.
4. Preserve deterministic behavior across recursive transclusion.
5. Keep v1 semantics simple and explicit: only non-templated top-level frontmatter values act as interpolation inputs.
6. Expose the feature in compose reporting and operation toggling.

## Non-Goals

1. Supporting source-order-dependent chained frontmatter interpolation such as `plan: "{{spec}}.txt"` when `spec` is itself interpolated in the same pass.
2. Introducing typed expression output for frontmatter. Interpolated string leaves remain strings.
3. Interpolating frontmatter keys. Only values are interpolated.
4. Reworking text replacement to also operate on frontmatter.
5. Changing body interpolation semantics for missing variables, truthiness, fallbacks, or functions.

## Current Baseline

Today the compose flow behaves like this:

1. external state is copied into missing or null frontmatter fields
2. `--set` overrides are written into frontmatter
3. `EffectiveStateBuilder` builds the runtime lookup state
4. Inline Pre runs on `self.content` only
5. Transclusion later parses `prologue` and `epilogue` directly from `self.frontmatter()`

This means:

1. `{{spec}}` in the body can work only if `spec` is already a literal frontmatter value
2. `prologue: "{{base}}/intro.md"` does not become a usable frontmatter transclusion target
3. inherited parent values do reach child documents, but child frontmatter cannot derive new frontmatter values from them before later stages run

## Primary Recommendation

Add a first-class `ComposeOperation::FrontmatterInterpolation` that runs in Inline Pre before `PageBlocks` and before body `Interpolation`, but implement it as a state-preparation step that executes before the final `EffectiveState` is built.

This is the key design choice.

Rationale:

1. frontmatter interpolation mutates the inputs that `EffectiveState` is built from, so it must happen before the final state is frozen for later stages
2. `prologue` and `epilogue` parsing already reads `self.frontmatter()`, so mutating frontmatter early automatically enables interpolated frontmatter transclusion targets
3. page-block conditions and later body interpolation can then see resolved frontmatter values without special casing
4. parent frontmatter propagation keeps working because inherited state is merged before this pass runs

## Proposed Pipeline Order

The effective compose lifecycle should become:

1. load Markdown and parse frontmatter
2. apply external-state defaults into frontmatter
3. apply `--set` overrides into frontmatter
4. if `FrontmatterInterpolation` is enabled:
   - classify top-level frontmatter entries into seed values and templated values
   - interpolate templated values against the seed state plus runtime `ctx` and `env`
   - write resolved values back into `self.frontmatter`
5. build the final `EffectiveState`
6. run the remaining Inline Pre operations on the body:
   - `TextReplacement`
   - `PageBlocks`
   - `Interpolation`
   - `ShellExpansion`
7. run transclusion and Inline Post as today

Recommended default Inline Pre operation order:

1. `FrontmatterInterpolation`
2. `TextReplacement`
3. `PageBlocks`
4. `Interpolation`
5. `ShellExpansion`

`FrontmatterInterpolation` should be part of `ComposeOperation::default_order()` and belong to `ComposePhase::InlinePre`.

## Semantics

### Top-level seed model

The spec is explicit that only frontmatter properties whose values do not contain interpolation syntax should seed the interpolation dictionary.

For v1, treat top-level keys as the unit of classification:

1. if a top-level value tree contains no interpolation expressions anywhere, it is a seed value
2. if a top-level value tree contains one or more interpolation expressions anywhere, it is a templated value

Example:

```yaml
---
base: /tmp/project
author: Alice
spec: "{{base}}/spec.md"
metadata:
  home: "{{base}}/docs"
  owner: Alice
---
```

Classification:

1. `base` is seed
2. `author` is seed
3. `spec` is templated
4. `metadata` is templated because one nested string contains interpolation syntax

This avoids source-order dependence and stays faithful to the spec.

### Supported value shapes

Interpolation should recurse through frontmatter values:

1. `String`: interpolate with plain-text scanning
2. `Array`: recurse into each element
3. `Object`: recurse into each field value
4. `Number`, `Bool`, `Null`: unchanged

Only string leaves are rewritten. The surrounding JSON/YAML structure is preserved.

### Variable lookup scope

Frontmatter interpolation should resolve against:

1. seed frontmatter values from the current document after inherited state/defaults/overrides have been applied
2. runtime `ctx.*`
3. process `env.*`

It should not resolve against other templated top-level frontmatter values in the same pass.

This means:

1. `spec: "{{base}}/spec.md"` works when `base` is literal or inherited
2. `plan: "{{spec}}.plan.md"` is not supported when `spec` is also templated in the same document

That limitation should be documented explicitly.

### Missing variables

V1 should preserve existing interpolation semantics:

1. missing variables resolve to the empty string
2. fallbacks, ternaries, comparisons, and helper functions behave exactly as they do in body interpolation

This keeps the evaluator shared and predictable, even though it means unsupported chained frontmatter interpolation can degrade to empty-string output rather than deferred resolution.

## Internal Architecture

### 1. Add a dedicated engine module

Recommended module layout:

```txt
darkmatter/lib/src/markdown/compose/
├── frontmatter_interpolation.rs
├── interpolation/
│   ├── mod.rs
│   ├── evaluator.rs
│   ├── lexer.rs
│   ├── parser.rs
│   └── rewrite.rs
└── ...
```

Responsibilities:

1. `frontmatter_interpolation.rs`
   - classify seed vs templated top-level entries
   - walk `serde_json::Value` trees
   - update `Frontmatter`
   - report counts and warnings
2. `interpolation/rewrite.rs`
   - hold the shared string-rewrite helper currently embedded in `run_interpolation_stage`
   - support both markdown-aware scanning and plain-text scanning

### 2. Generalize evaluator lookup behind a small trait

Today `Evaluator` depends directly on `EffectiveState`. Frontmatter interpolation should not need to construct a full final effective state just to evaluate seed values.

Introduce an internal trait:

```rust
pub(crate) trait InterpolationLookup {
    fn get(&self, path: &str) -> Option<serde_json::Value>;
    fn get_string(&self, path: &str) -> String;
}
```

Implement it for:

1. `EffectiveState`
2. a new internal `FrontmatterSeedState`

`Evaluator` then becomes generic over `InterpolationLookup`.

This keeps the parser/evaluator shared while avoiding double-building the full compose state and double-emitting ctx diagnostics.

### 3. Add `FrontmatterSeedState`

Recommended internal type:

```rust
pub(crate) struct FrontmatterSeedState {
    data: std::collections::HashMap<String, serde_json::Value>,
    context: ComposeContext,
}
```

Behavior:

1. `data` contains only seed top-level values
2. `ctx.*` resolves from `ComposeContext`
3. `env.*` resolves from `ComposeContext::env()`
4. plain keys and dotted lookups resolve from `data`

This mirrors the relevant lookup behavior of `EffectiveState` without pulling in final-state concerns such as user `ctx` merge diagnostics.

## Algorithm

### Detection

Add a helper that determines whether a JSON value tree contains interpolation syntax:

```rust
fn contains_interpolation(value: &Value) -> bool
```

Rules:

1. `String`: true when plain-text interpolation scanning finds at least one `{{ ... }}`
2. `Array`: true when any element matches
3. `Object`: true when any field value matches
4. other scalar types: false

This should use the same interpolation span finder logic as body interpolation, but in plain-text mode instead of markdown-aware mode.

### Rewrite flow

Recommended engine entry point:

```rust
pub(crate) fn interpolate_frontmatter(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
) -> Result<FrontmatterInterpolationReport, MarkdownError>
```

Recommended algorithm:

1. clone the current top-level frontmatter map
2. partition entries into:
   - `seed_map`
   - `templated_keys`
3. if `templated_keys` is empty, return a zeroed report
4. build `FrontmatterSeedState` from `seed_map` and `context.clone()`
5. create `Evaluator::new(&seed_state)`
6. for each templated top-level key in insertion order:
   - recursively rewrite its value tree
   - count successful replacements
   - collect non-fatal warnings if `fail_fast` is false
   - write the updated value back into the mutable frontmatter
7. return the interpolation count and warnings

### Recursive rewrite helper

Recommended helper:

```rust
fn rewrite_value(
    value: &Value,
    evaluator: &Evaluator<impl InterpolationLookup>,
    fail_fast: bool,
) -> Result<(Value, usize, Vec<ComposeWarning>), MarkdownError>
```

Behavior:

1. `String`:
   - call shared `interpolate_text(..., ScanMode::Plain, ...)`
   - return rewritten string plus replacement count
2. `Array`:
   - recurse into each element and sum counts
3. `Object`:
   - recurse into each field value and sum counts
4. other scalars:
   - clone unchanged, zero count

## Shared String Rewrite Helper

The current `run_interpolation_stage()` in `compose/mod.rs` contains its own scan/parse/eval/rewrite loop. That logic should be factored into a shared helper so frontmatter and body interpolation do not drift.

Recommended API:

```rust
pub(crate) enum ScanMode {
    MarkdownAware,
    Plain,
}

pub(crate) struct InterpolationRewrite {
    pub output: String,
    pub replacements: usize,
    pub warnings: Vec<ComposeWarning>,
}

pub(crate) fn interpolate_text<L: InterpolationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    fail_fast: bool,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, MarkdownError>
```

Mode behavior:

1. `MarkdownAware` uses the existing code-span and fenced-code skipping behavior
2. `Plain` scans the whole string with no code-region exclusions

`run_interpolation_stage()` should call this helper with `ScanMode::MarkdownAware`.
Frontmatter interpolation should call it with `ScanMode::Plain`.

## Public Type Changes

### `ComposeOperation`

Add:

```rust
FrontmatterInterpolation,
```

Updates required:

1. increment `ComposeOperation::COUNT`
2. assign a stable discriminant index
3. include it in `ComposeOperation::default_order()`
4. map it to `ComposePhase::InlinePre`

### `ComposeReport`

Add a dedicated counter:

```rust
pub frontmatter_interpolations_applied: usize,
```

Rationale:

1. existing `interpolations_applied` currently means body interpolation
2. keeping separate counters avoids ambiguity in tests and report summaries
3. it makes the new feature visible in compose diagnostics

Required updates:

1. `has_changes()`
2. `summary()`
3. `merge()`
4. tests in `compose/types.rs`

Recommended summary wording:

- `"2 frontmatter interpolation(s)"`

## Compose Runtime Integration

### State preparation in `compose/mod.rs`

Refactor the early portion of `run_compose_pipeline_internal()` so it has two state-related steps:

1. mutable frontmatter preparation
2. final effective-state construction

Recommended flow inside `run_compose_pipeline_internal()`:

1. apply external defaults into frontmatter
2. apply set overrides into frontmatter
3. if `FrontmatterInterpolation` is enabled:
   - call `interpolate_frontmatter(self.frontmatter_mut(), options.context(), options.fail_fast)`
   - merge warnings into the compose report
   - record `frontmatter_interpolations_applied`
4. build the final `EffectiveStateBuilder` from the updated frontmatter
5. convert ctx diagnostics to compose warnings as today
6. continue with remaining operations

The frontmatter interpolation operation should not run through the existing generic `run_inline_pre_operation()` path because the final `EffectiveState` must be built after frontmatter has been mutated.

### Child transclusion behavior

No new child-state mechanism is required.

Current behavior already propagates parent state by:

1. building child `external_state` from the parent `EffectiveState`
2. applying it into child frontmatter before child state construction

Once frontmatter interpolation runs before the child’s final `EffectiveState` is built, child documents automatically gain:

1. inherited parent literals as seed inputs
2. derived local frontmatter values based on those inherited inputs
3. resolved frontmatter values for later body interpolation and transclusion

### Frontmatter transclusion behavior

No parser change is required in `compose/transclusion/parser.rs`.

`parse_frontmatter_refs()` already reads `self.frontmatter().as_map()`. After frontmatter interpolation mutates the frontmatter, values like:

```yaml
prologue: "{{base}}/intro.md"
```

will naturally be parsed as resolved strings during the transclusion phase.

## Diagnostics and Error Handling

### Warning stage

Use a dedicated warning stage name:

```rust
"frontmatter-interpolation"
```

### Failure policy

Match the current interpolation policy:

1. when `fail_fast` is `true`, parse or evaluation errors return a compose error immediately
2. when `fail_fast` is `false`, the original string leaf remains unchanged and a warning is recorded

This is slightly stricter than current body interpolation, which silently preserves parse/eval failures. That difference is acceptable here because unresolved frontmatter has broader compose-time consequences. If desired, the shared string-rewrite helper can later be adopted by body interpolation too.

### Line numbers

Frontmatter warnings do not need exact YAML line numbers in v1.

Recommended message shape:

1. include the top-level frontmatter key name
2. include the expression text when practical

Example:

```txt
frontmatter-interpolation: key 'spec' failed to evaluate '{{ base | }}': unexpected token
```

## Caching Impact

No new persistent cache dimension is required.

Reasons:

1. frontmatter interpolation only changes composed output through already-hashed inputs:
   - source document bytes
   - external state
   - set overrides
   - runtime context
   - enabled operations
2. the final composed result is still keyed by:
   - `effective_state_hash(state)`
   - `context_hash(state.context())`
   - `options_hash(options)`

The only required cache-related change is indirect:

1. adding `FrontmatterInterpolation` to the enabled operation set changes `options_hash()` automatically because operation names are already hashed

## Documentation Impact

Update these docs in the implementation change:

1. `darkmatter/docs/inline/interpolation.md`
   - explain frontmatter interpolation and its seed-only semantics
2. any compose pipeline overview doc
   - show the new operation ordering
3. public Rustdoc for `ComposeOperation` and `ComposeReport`

## Testing Plan

### Unit tests

Add focused tests for the new module:

1. string detection identifies `{{ ... }}` in plain strings
2. nested arrays and objects are classified correctly
3. seed partition excludes templated top-level keys entirely
4. recursive rewrite updates string leaves but preserves array/object structure
5. missing variables resolve to empty string
6. parse/eval failures warn or error according to `fail_fast`

### Compose integration tests

Add compose tests in `compose/mod.rs` for:

1. the spec example:
   - inherited `base`
   - local `spec` and `plan`
   - body `{{spec}}` and `{{plan}}`
2. child document frontmatter deriving from parent state
3. `prologue` and `epilogue` using interpolated frontmatter paths
4. interpolated frontmatter values visible to page-block conditions
5. `--set` overrides participating in frontmatter interpolation
6. arrays and objects in frontmatter containing interpolated strings
7. disabling `FrontmatterInterpolation` leaves frontmatter values literal
8. body interpolation still skips code spans and fenced code blocks after the refactor

### Report tests

Add or update tests for:

1. `frontmatter_interpolations_applied` counting
2. summary formatting
3. report merging

## Explicit V1 Constraint

The implementation should document and test this rule explicitly:

> Frontmatter interpolation is a single-pass transform over templated top-level keys using only non-templated top-level keys, inherited state, `ctx`, and `env` as inputs.

That constraint is intentional. It avoids hidden ordering rules, keeps the algorithm cheap, and matches the current evaluator behavior. If chained derived frontmatter becomes important later, it should be designed as a separate follow-up feature with clear cycle and missing-variable semantics rather than sneaking into this implementation.
