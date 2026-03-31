# Frontmatter Interpolation — Implementation Plan

This plan implements the feature specified in `spec.md` and designed in `tech-design.md`. Each phase is self-contained: it compiles, tests pass, and the codebase remains green before moving to the next phase.

---

## Phase 1: InterpolationLookup Trait and Evaluator Generalization

**Goal**: Decouple `Evaluator` from `EffectiveState` so it can be reused for frontmatter interpolation with a seed-only state.

### 1.1 Define `InterpolationLookup` trait

**File**: `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`

Add a `pub(crate)` trait before the `Evaluator` struct:

```rust
pub(crate) trait InterpolationLookup {
    fn get(&self, path: &str) -> Option<serde_json::Value>;
    fn get_string(&self, path: &str) -> String;
}
```

### 1.2 Implement `InterpolationLookup` for `EffectiveState`

**File**: `darkmatter/lib/src/markdown/compose/state.rs`

Add an impl block that delegates to the existing `get()` and `get_string()` methods:

```rust
impl crate::markdown::compose::interpolation::InterpolationLookup for EffectiveState {
    fn get(&self, path: &str) -> Option<serde_json::Value> {
        self.get(path)
    }
    fn get_string(&self, path: &str) -> String {
        self.get_string(path)
    }
}
```

> **Note**: The method names are the same, so the impl body just calls `Self::get` / `Self::get_string`. The import path for the trait needs to be visible at `pub(crate)` scope. If the circular module dependency is awkward, define the trait in a small `lookup.rs` sub-module under `interpolation/` and re-export from `interpolation/mod.rs`.

### 1.3 Make `Evaluator` generic over `InterpolationLookup`

**File**: `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`

Change:

```rust
// Before
pub struct Evaluator<'a> {
    state: &'a EffectiveState,
}

impl<'a> Evaluator<'a> {
    pub fn new(state: &'a EffectiveState) -> Self { ... }
}
```

To:

```rust
// After
pub struct Evaluator<'a, L: InterpolationLookup> {
    state: &'a L,
}

impl<'a, L: InterpolationLookup> Evaluator<'a, L> {
    pub fn new(state: &'a L) -> Self {
        Self { state }
    }
    // ... all existing methods unchanged, they already call self.state.get() / self.state.get_string()
}
```

### 1.4 Update call sites

Two call sites create `Evaluator::new(state)` where `state: &EffectiveState`:

1. **`compose/mod.rs:788`** — `run_interpolation_stage()`: `Evaluator::new(state)` where `state: &EffectiveState` — no change needed, type inference handles it.
2. **`compose/conditions.rs`** — if it constructs an `Evaluator`, same story.

Search for all `Evaluator::new` call sites and verify each compiles with the new generic signature. The compiler will guide you — existing code should just work since `EffectiveState` now implements the trait.

### 1.5 Update re-exports

**File**: `darkmatter/lib/src/markdown/compose/interpolation/mod.rs`

Add `InterpolationLookup` to the `pub use evaluator::` line:

```rust
pub use evaluator::{EvalResult, EvalValue, Evaluator, InterpolationLookup};
```

### 1.6 Tests

All existing evaluator and compose tests must still pass with no changes. Run:

```bash
just test -p darkmatter
```

No new tests are needed in this phase — the generalization is behavior-preserving.

---

## Phase 2: Shared String Rewrite Helper (`interpolate_text`)

**Goal**: Extract the scan/parse/eval/rewrite logic from `run_interpolation_stage()` into a shared helper that supports both markdown-aware and plain-text scanning modes.

### 2.1 Add `ScanMode` enum and `InterpolationRewrite` struct

**File**: `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs` (new file)

```rust
use super::{Evaluator, InterpolationLookup, ExpressionFinder, ExpressionLocation, parse, EvalResult};
use crate::markdown::compose::types::ComposeWarning;
use crate::markdown::error::MarkdownError;

/// Controls how the string rewrite scans for `{{ }}` expressions.
pub(crate) enum ScanMode {
    /// Skip expressions inside code spans and fenced code blocks.
    /// Used by body interpolation.
    MarkdownAware,
    /// Scan the entire string with no exclusions.
    /// Used by frontmatter interpolation.
    Plain,
}

/// Result of rewriting interpolation expressions in a string.
pub(crate) struct InterpolationRewrite {
    pub output: String,
    pub replacements: usize,
    pub warnings: Vec<ComposeWarning>,
}
```

### 2.2 Implement `interpolate_text`

**File**: `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`

```rust
pub(crate) fn interpolate_text<L: InterpolationLookup>(
    input: &str,
    evaluator: &Evaluator<L>,
    scan_mode: ScanMode,
    fail_fast: bool,
    warning_stage: &'static str,
) -> Result<InterpolationRewrite, MarkdownError> {
    let locations: Vec<ExpressionLocation> = match scan_mode {
        ScanMode::MarkdownAware => ExpressionFinder::new(input).find_all(),
        ScanMode::Plain => ExpressionFinder::find_all_plain(input),
    };

    if locations.is_empty() {
        return Ok(InterpolationRewrite {
            output: input.to_string(),
            replacements: 0,
            warnings: vec![],
        });
    }

    let mut output = input.to_string();
    let mut count = 0;
    let mut warnings = Vec::new();

    for loc in locations.into_iter().rev() {
        match parse(&loc.expression) {
            Ok(expr) => match evaluator.eval(&expr) {
                EvalResult::Value(replacement) => {
                    // Inherit line indentation for multiline replacements
                    let replacement = if replacement.contains('\n') {
                        let line_start = output[..loc.start]
                            .rfind('\n')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let indent: String = output[line_start..loc.start]
                            .chars()
                            .take_while(|c| c.is_whitespace())
                            .collect();
                        if indent.is_empty() {
                            replacement
                        } else {
                            replacement.replace('\n', &format!("\n{indent}"))
                        }
                    } else {
                        replacement
                    };
                    output.replace_range(loc.start..loc.end, &replacement);
                    count += 1;
                }
                EvalResult::Error { message, .. } if fail_fast => {
                    return Err(MarkdownError::Transform(format!(
                        "Interpolation evaluation failed for '{}': {}",
                        loc.expression, message
                    )));
                }
                EvalResult::Error { message, original } => {
                    warnings.push(ComposeWarning::new(
                        warning_stage,
                        format!("failed to evaluate '{}': {}", original, message),
                    ));
                }
            },
            Err(e) if fail_fast => {
                return Err(MarkdownError::Transform(format!(
                    "Interpolation parse failed for '{}': {}",
                    loc.expression, e
                )));
            }
            Err(e) => {
                warnings.push(ComposeWarning::new(
                    warning_stage,
                    format!("failed to parse '{}': {}", loc.expression, e),
                ));
            }
        }
    }

    Ok(InterpolationRewrite {
        output,
        replacements: count,
        warnings,
    })
}
```

### 2.3 Add `ExpressionFinder::find_all_plain`

**File**: `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`

Add a static method on `ExpressionFinder` that scans without building code regions:

```rust
/// Finds all `{{ }}` expressions in a plain string with no code-region exclusions.
pub fn find_all_plain(input: &str) -> Vec<ExpressionLocation> {
    let finder = Self {
        content: input,
        code_regions: vec![],  // no exclusions
    };
    finder.find_all()
}
```

### 2.4 Register the new module

**File**: `darkmatter/lib/src/markdown/compose/interpolation/mod.rs`

Add:

```rust
pub(crate) mod rewrite;
```

And re-export the key types:

```rust
pub(crate) use rewrite::{interpolate_text, InterpolationRewrite, ScanMode};
```

### 2.5 Refactor `run_interpolation_stage()` to use the shared helper

**File**: `darkmatter/lib/src/markdown/compose/mod.rs` (lines 774–844)

Replace the body of `run_interpolation_stage()` with a call to `interpolate_text`:

```rust
fn run_interpolation_stage(
    &mut self,
    state: &EffectiveState,
    options: &ComposeOptions,
) -> MarkdownResult<usize> {
    use interpolation::{Evaluator, interpolate_text, ScanMode};

    let evaluator = Evaluator::new(state);
    let result = interpolate_text(
        &self.content,
        &evaluator,
        ScanMode::MarkdownAware,
        options.fail_fast,
        "interpolation",
    )?;

    if result.replacements > 0 {
        self.content = result.output;
    }
    // Note: current body interpolation does not emit warnings to the report
    // in the non-fail-fast case (it silently preserves originals). Preserve
    // that behavior for now — warnings are dropped. Frontmatter interpolation
    // will use them.
    Ok(result.replacements)
}
```

### 2.6 Tests

1. All existing interpolation tests must still pass (behavior-preserving refactor).
2. Add unit tests for `interpolate_text` in `rewrite.rs`:
   - `ScanMode::Plain` does NOT skip code spans
   - `ScanMode::MarkdownAware` DOES skip code spans
   - multiline indentation inheritance works
   - `fail_fast: true` returns error on parse failure
   - `fail_fast: false` records warnings and preserves original

```bash
just test -p darkmatter
```

---

## Phase 3: `FrontmatterSeedState` and Detection Helpers

**Goal**: Build the seed state type and the `contains_interpolation` detection helper.

### 3.1 Create `frontmatter_interpolation.rs`

**File**: `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs` (new file)

This file will hold the entire frontmatter interpolation engine.

### 3.2 Implement `contains_interpolation`

```rust
use serde_json::Value;
use super::interpolation::ExpressionFinder;

/// Returns `true` if the JSON value tree contains any `{{ }}` interpolation expressions.
pub(crate) fn contains_interpolation(value: &Value) -> bool {
    match value {
        Value::String(s) => !ExpressionFinder::find_all_plain(s).is_empty(),
        Value::Array(arr) => arr.iter().any(contains_interpolation),
        Value::Object(obj) => obj.values().any(contains_interpolation),
        _ => false,
    }
}
```

### 3.3 Implement `FrontmatterSeedState`

```rust
use std::collections::HashMap;
use serde_json::Value;
use super::interpolation::InterpolationLookup;
use super::types::ComposeContext;

/// Lookup state for frontmatter interpolation.
///
/// Contains only non-templated (seed) top-level frontmatter values,
/// plus `ctx.*` and `env.*` from the runtime context.
pub(crate) struct FrontmatterSeedState {
    data: HashMap<String, Value>,
    context: ComposeContext,
}

impl FrontmatterSeedState {
    pub(crate) fn new(data: HashMap<String, Value>, context: ComposeContext) -> Self {
        Self { data, context }
    }
}

impl InterpolationLookup for FrontmatterSeedState {
    fn get(&self, path: &str) -> Option<Value> {
        // ctx.* prefix
        if let Some(ctx_key) = path.strip_prefix("ctx.") {
            return self.context.get(ctx_key).cloned();
        }

        // env.* prefix
        if let Some(env_key) = path.strip_prefix("env.") {
            return self.context.env().get(env_key).map(|v| Value::String(v.clone()));
        }

        // Dotted nested path in seed data
        if let Some(dot_pos) = path.find('.') {
            let root = &path[..dot_pos];
            let rest = &path[dot_pos + 1..];
            let root_val = self.data.get(root)?;
            return get_nested(root_val, rest);
        }

        // Simple key in seed data
        self.data.get(path).cloned()
    }

    fn get_string(&self, path: &str) -> String {
        match self.get(path) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(v) => v.to_string(),
        }
    }
}

/// Walks a dotted path through a JSON value.
fn get_nested(value: &Value, path: &str) -> Option<Value> {
    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}
```

### 3.4 Tests

Add unit tests in `frontmatter_interpolation.rs`:

1. **`contains_interpolation` detection**:
   - plain string: `false`
   - string with `{{ foo }}`: `true`
   - nested object where one leaf has `{{ }}`: `true`
   - array with mixed elements: `true` if any contain expression
   - number/bool/null: `false`

2. **`FrontmatterSeedState` lookup**:
   - simple key resolves from seed data
   - `ctx.today` resolves from context
   - `env.HOME` resolves from context env
   - dotted path `foo.bar` resolves nested seed value
   - missing key returns `None`
   - `get_string` returns empty string for missing keys

```bash
just test -p darkmatter
```

---

## Phase 4: Frontmatter Interpolation Engine

**Goal**: Implement the recursive rewrite algorithm and the `interpolate_frontmatter` entry point.

### 4.1 Implement `rewrite_value`

**File**: `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`

```rust
use super::interpolation::{Evaluator, InterpolationLookup, interpolate_text, ScanMode, InterpolationRewrite};
use super::types::ComposeWarning;
use crate::markdown::error::MarkdownError;

fn rewrite_value<L: InterpolationLookup>(
    value: &Value,
    evaluator: &Evaluator<L>,
    fail_fast: bool,
) -> Result<(Value, usize, Vec<ComposeWarning>), MarkdownError> {
    match value {
        Value::String(s) => {
            let result = interpolate_text(
                s,
                evaluator,
                ScanMode::Plain,
                fail_fast,
                "frontmatter-interpolation",
            )?;
            Ok((Value::String(result.output), result.replacements, result.warnings))
        }
        Value::Array(arr) => {
            let mut new_arr = Vec::with_capacity(arr.len());
            let mut total_count = 0;
            let mut all_warnings = Vec::new();
            for item in arr {
                let (new_val, count, warnings) = rewrite_value(item, evaluator, fail_fast)?;
                new_arr.push(new_val);
                total_count += count;
                all_warnings.extend(warnings);
            }
            Ok((Value::Array(new_arr), total_count, all_warnings))
        }
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::with_capacity(obj.len());
            let mut total_count = 0;
            let mut all_warnings = Vec::new();
            for (key, val) in obj {
                let (new_val, count, warnings) = rewrite_value(val, evaluator, fail_fast)?;
                new_obj.insert(key.clone(), new_val);
                total_count += count;
                all_warnings.extend(warnings);
            }
            Ok((Value::Object(new_obj), total_count, all_warnings))
        }
        // Number, Bool, Null — pass through
        other => Ok((other.clone(), 0, vec![])),
    }
}
```

### 4.2 Implement `interpolate_frontmatter`

```rust
use crate::markdown::frontmatter::Frontmatter;

/// Result of frontmatter interpolation.
pub(crate) struct FrontmatterInterpolationReport {
    pub replacements: usize,
    pub warnings: Vec<ComposeWarning>,
}

/// Interpolates templated frontmatter values using seed (non-templated) values.
///
/// Classifies top-level frontmatter entries into seed values (no `{{ }}`)
/// and templated values (contain `{{ }}`). Builds a lookup state from the
/// seed values plus the runtime context, then rewrites the templated values.
pub(crate) fn interpolate_frontmatter(
    frontmatter: &mut Frontmatter,
    context: &ComposeContext,
    fail_fast: bool,
) -> Result<FrontmatterInterpolationReport, MarkdownError> {
    let fm = frontmatter.as_map();

    // Partition: seed keys have no interpolation, templated keys have at least one.
    let mut seed_map: HashMap<String, Value> = HashMap::new();
    let mut templated_keys: Vec<String> = Vec::new();

    for (key, value) in fm.iter() {
        if contains_interpolation(value) {
            templated_keys.push(key.clone());
        } else {
            seed_map.insert(key.clone(), value.clone());
        }
    }

    if templated_keys.is_empty() {
        return Ok(FrontmatterInterpolationReport {
            replacements: 0,
            warnings: vec![],
        });
    }

    // Build seed state
    let seed_state = FrontmatterSeedState::new(seed_map, context.clone());
    let evaluator = Evaluator::new(&seed_state);

    let mut total_replacements = 0;
    let mut all_warnings = Vec::new();

    // Rewrite each templated key's value tree
    let fm_mut = frontmatter.as_map_mut();
    for key in &templated_keys {
        if let Some(value) = fm_mut.get(key).cloned() {
            let (new_value, count, mut warnings) = rewrite_value(&value, &evaluator, fail_fast)?;

            // Add key context to warnings
            for w in &mut warnings {
                w.message = format!("key '{}': {}", key, w.message);
            }

            fm_mut.insert(key.clone(), new_value);
            total_replacements += count;
            all_warnings.extend(warnings);
        }
    }

    Ok(FrontmatterInterpolationReport {
        replacements: total_replacements,
        warnings: all_warnings,
    })
}
```

### 4.3 Register the module

**File**: `darkmatter/lib/src/markdown/compose/mod.rs`

Add near the top with other module declarations:

```rust
mod frontmatter_interpolation;
```

### 4.4 Tests

Add unit tests in `frontmatter_interpolation.rs`:

1. **Spec example**: `base: "/path/to/something"`, `spec: "{{base}}/spec.md"`, `plan: "{{base}}/plan.md"` → spec and plan are resolved, base is unchanged.
2. **No templated keys**: all literals → returns zeroed report.
3. **Nested object rewrite**: `metadata: { home: "{{base}}/docs", owner: "Alice" }` → `home` is rewritten, `owner` is unchanged.
4. **Array rewrite**: `paths: ["{{base}}/a", "{{base}}/b"]` → both rewritten.
5. **Missing variable**: `spec: "{{missing}}/spec.md"` → resolves to `"/spec.md"` (empty string for missing).
6. **`ctx.*` lookup**: `date: "{{ctx.today}}"` resolves from context.
7. **`env.*` lookup**: `home: "{{env.HOME}}"` resolves from context env.
8. **Chained reference is not supported**: `spec: "{{base}}/spec.md"`, `plan: "{{spec}}.plan.md"` → `plan` gets `".plan.md"` since `spec` is itself templated and excluded from seed.
9. **fail_fast: true** returns error on parse failure in frontmatter value.
10. **fail_fast: false** records warning and preserves original expression.

```bash
just test -p darkmatter
```

---

## Phase 5: ComposeOperation Variant and ComposeReport Field

**Goal**: Register `FrontmatterInterpolation` as a first-class compose operation.

### 5.1 Add the variant

**File**: `darkmatter/lib/src/markdown/compose/types.rs`

Add to `ComposeOperation` enum (after `ShellExpansion`, before `BlockTransclusion` in the source, but the ordering in `default_order()` is what matters):

```rust
/// Resolves `{{ variable }}` expressions inside frontmatter values
/// using non-templated frontmatter values, `ctx`, and `env` as inputs.
/// Runs before the final effective state is built.
FrontmatterInterpolation,
```

### 5.2 Update `COUNT`

```rust
pub const COUNT: usize = 11;
```

### 5.3 Update `index()`

Assign the new variant a stable discriminant. Insert it with index 10 (after Normalization's 9), or renumber so it has a logical position. Simplest approach — append at the end:

```rust
Self::FrontmatterInterpolation => 10,
```

### 5.4 Update `phase()`

Map to `InlinePre`:

```rust
Self::FrontmatterInterpolation => ComposePhase::InlinePre,
```

### 5.5 Update `default_order()`

Insert at the beginning of the Inline Pre section:

```rust
pub fn default_order() -> &'static [ComposeOperation] {
    &[
        // Inline Pre (serial)
        Self::FrontmatterInterpolation,  // NEW — must run first
        Self::TextReplacement,
        Self::PageBlocks,
        Self::Interpolation,
        Self::ShellExpansion,
        // Transclusion (concurrent)
        Self::BlockTransclusion,
        Self::FrontmatterTransclusion,
        Self::CodeTransclusion,
        Self::TocLinking,
        // Inline Post (serial)
        Self::Cleanup,
        Self::Normalization,
    ]
}
```

### 5.6 Add `frontmatter_interpolations_applied` to `ComposeReport`

```rust
/// Number of frontmatter interpolation expressions resolved.
pub frontmatter_interpolations_applied: usize,
```

### 5.7 Update `has_changes()`

Add:

```rust
|| self.frontmatter_interpolations_applied > 0
```

### 5.8 Update `summary()`

Add a block (insert before the interpolations block for logical grouping):

```rust
if self.frontmatter_interpolations_applied > 0 {
    parts.push(format!(
        "{} frontmatter interpolation(s)",
        self.frontmatter_interpolations_applied
    ));
}
```

### 5.9 Update `merge()`

Add:

```rust
self.frontmatter_interpolations_applied += other.frontmatter_interpolations_applied;
```

### 5.10 Add perf metric

**File**: `darkmatter/lib/src/markdown/compose/perf.rs`

Add variant to `PerfMetricKind`:

```rust
FrontmatterInterpolation,
```

And its label:

```rust
Self::FrontmatterInterpolation => "frontmatter interpolation",
```

### 5.11 Fix existing tests

Several tests will need updating:

1. **`test_compose_operation_default_order_exact`** — update expected array to include `FrontmatterInterpolation` at position 0.
2. **`test_compose_operation_phase_mapping_is_complete`** — add entry for the new variant.
3. **Any test that asserts `COUNT == 10`** — update to 11.
4. **Any test that asserts on `ComposeOperationSet::all()`** — update to include the new variant.
5. **`test_compose_options_default_stages`** — add assertion for `FrontmatterInterpolation`.

```bash
just test -p darkmatter
```

---

## Phase 6: Pipeline Integration

**Goal**: Wire `FrontmatterInterpolation` into `run_compose_pipeline_internal()` so it runs before the final `EffectiveState` is built.

### 6.1 Modify `run_compose_pipeline_internal()`

**File**: `darkmatter/lib/src/markdown/compose/mod.rs` (lines 291–340)

Insert the frontmatter interpolation step between set-overrides application (line 315) and the `EffectiveStateBuilder` (line 319):

```rust
// Apply set overrides: unconditionally overwrite frontmatter keys.
if let Some(overrides) = options.set_overrides.as_ref().and_then(Value::as_object) {
    let fm = self.frontmatter_mut().as_map_mut();
    for (key, value) in overrides {
        fm.insert(key.clone(), value.clone());
    }
}

// === NEW: Frontmatter Interpolation ===
// Must run BEFORE EffectiveState is built because it mutates frontmatter
// inputs that drive later stages (body interpolation, transclusion targets,
// page-block conditions).
if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
    let fm_start = perf.is_enabled().then(std::time::Instant::now);
    let fm_report = frontmatter_interpolation::interpolate_frontmatter(
        self.frontmatter_mut(),
        options.context(),
        options.fail_fast,
    )?;
    report.frontmatter_interpolations_applied = fm_report.replacements;
    report.warnings.extend(fm_report.warnings);
    if let Some(start) = fm_start {
        perf.record(perf::PerfMetricKind::FrontmatterInterpolation, start.elapsed());
    }
}

// Build effective state for replacement/interpolation and condition checks.
let effective_state = EffectiveStateBuilder::new()
    // ... existing builder chain
```

### 6.2 Skip `FrontmatterInterpolation` in the operation dispatch loop

Because `FrontmatterInterpolation` has already run before the loop, the generic `run_inline_pre_operation()` dispatch must not try to run it again. Add to `run_inline_pre_operation()`:

```rust
fn run_inline_pre_operation(
    &mut self,
    operation: ComposeOperation,
    state: &EffectiveState,
    options: &ComposeOptions,
    runtime: &mut shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    match operation {
        // FrontmatterInterpolation is handled before EffectiveState build,
        // not in the generic operation loop.
        ComposeOperation::FrontmatterInterpolation => Ok(()),
        ComposeOperation::TextReplacement => { ... }
        // ... rest unchanged
    }
}
```

And add a corresponding perf match arm:

```rust
ComposeOperation::FrontmatterInterpolation => perf::PerfMetricKind::FrontmatterInterpolation,
```

### 6.3 Tests

Run full test suite:

```bash
just test -p darkmatter
```

---

## Phase 7: Compose Integration Tests

**Goal**: Add end-to-end compose tests that exercise frontmatter interpolation through the public API.

### 7.1 Test locations

**File**: `darkmatter/lib/src/markdown/compose/mod.rs` — in the `mod tests` block (after existing interpolation tests).

### 7.2 Test cases

Follow the existing test pattern (`compose_with` + `ComposeOptions::new().only(...)` or default options):

1. **Spec example** — inherited `base`, derived `spec` and `plan`, body `{{ spec }}` and `{{ plan }}`:

   ```rust
   #[test]
   fn test_frontmatter_interpolation_spec_example() {
       let content = "---\nbase: /path/to/something\nspec: \"{{base}}/spec.md\"\nplan: \"{{base}}/plan.md\"\n---\nThe spec is located at: {{spec}}\nThe plan is located at: {{plan}}";
       let md: Markdown = content.into();
       let (composed, report) = md.compose_with(
           ComposeOptions::new().only(&[
               ComposeOperation::FrontmatterInterpolation,
               ComposeOperation::Interpolation,
           ])
       ).unwrap();
       assert_eq!(report.frontmatter_interpolations_applied, 2);
       assert!(composed.content().contains("The spec is located at: /path/to/something/spec.md"));
       assert!(composed.content().contains("The plan is located at: /path/to/something/plan.md"));
   }
   ```

2. **Frontmatter interpolation with external state** — `base` comes from external state, `spec: "{{base}}/spec.md"` in document.

3. **`--set` overrides participate** — `--set base=/override` overrides a frontmatter value, and `spec: "{{base}}/spec.md"` uses the override.

4. **Arrays and objects** — frontmatter with nested structures containing `{{ }}`.

5. **Disabling `FrontmatterInterpolation`** — when disabled, frontmatter values stay literal and body interpolation sees the raw `{{ base }}` text in frontmatter values.

6. **Body interpolation still skips code spans** — regression test ensuring the refactored `interpolate_text` with `MarkdownAware` mode still works.

7. **Report counting** — verify `frontmatter_interpolations_applied` and `interpolations_applied` are counted separately.

8. **Summary formatting** — compose report summary includes "2 frontmatter interpolation(s)".

9. **Report merge** — two reports merge `frontmatter_interpolations_applied` additively.

```bash
just test -p darkmatter
```

---

## Phase 8: Documentation Updates

**Goal**: Update inline docs and compose pipeline documentation.

### 8.1 Module-level doc comment

**File**: `darkmatter/lib/src/markdown/compose/mod.rs` (lines 1–20)

Update the pipeline overview to include `FrontmatterInterpolation` as step 0 / the first Inline Pre operation:

```
//! **Inline Pre** (serial):
//! 1. **Frontmatter Interpolation** - Resolve `{{variable}}` in frontmatter values
//! 2. **Text Replacement** - Replace literal strings from frontmatter `replace` map
//! ...
```

### 8.2 Compose pipeline doc

**File**: `darkmatter/docs/darkmatter-compose-pipeline.md`

Add `FrontmatterInterpolation` to the pipeline diagram and operation list.

### 8.3 Interpolation inline doc

**File**: `darkmatter/docs/inline/fm-interpolation.md` (already exists as staged new file)

Ensure it documents:
- The seed-only semantics
- That chained frontmatter interpolation is not supported
- That `ctx.*` and `env.*` are available
- That the operation runs before `EffectiveState` is built

### 8.4 Rustdoc on new public types

Ensure `ComposeOperation::FrontmatterInterpolation`, `ComposeReport::frontmatter_interpolations_applied`, and the `frontmatter_interpolation` module have doc comments that match the project's H2-first documentation conventions.

---

## Phase Summary

| Phase | Files Modified | Files Created | Key Deliverable |
|-------|---------------|--------------|-----------------|
| 1 | `evaluator.rs`, `state.rs`, `mod.rs` (interpolation) | — | `InterpolationLookup` trait, generic `Evaluator` |
| 2 | `mod.rs` (compose), `lexer.rs` | `rewrite.rs` | Shared `interpolate_text` helper |
| 3 | — | `frontmatter_interpolation.rs` | `contains_interpolation`, `FrontmatterSeedState` |
| 4 | `frontmatter_interpolation.rs` | — | `interpolate_frontmatter` engine |
| 5 | `types.rs`, `perf.rs` | — | `ComposeOperation::FrontmatterInterpolation` variant |
| 6 | `mod.rs` (compose) | — | Pipeline wiring |
| 7 | `mod.rs` (compose, tests) | — | Integration tests |
| 8 | docs | — | Documentation |

Each phase is independently compilable and testable. The critical design decision — running frontmatter interpolation before `EffectiveState` is built — is implemented in Phase 6 but prepared by Phases 1–4.
