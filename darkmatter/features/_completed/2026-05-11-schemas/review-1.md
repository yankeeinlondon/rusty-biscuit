---
ready: false
agent: open_code
model: ""
---

# Review: Schemas in Darkmatter (Iteration #1)

**Scope**: Full review of the schemas feature as defined in `spec.md` against the implementation across `darkmatter/lib/src/markdown/schemas/`, `darkmatter/cli/src/commands/schema/`, external test files, and benchmarks.

## Summary

The implementation is architecturally sound and covers the majority of the specification. All six phases from the execution plan are marked complete. The grammar parser, SimplifiedSchema-to-JSON-Schema converter, resolution pipeline, validation engine, detection algorithm, CLI surface, shell-completion integration, and error rendering are all present and compile cleanly. 155 unit tests pass locally, plus snapshot, table-driven, and CLI integration tests.

However, there are several gaps — some functional, some test-coverage — that prevent a "ready for production" assessment at this iteration.

---

## Findings

### F1. `arm_index` never populated on root-union validation failures [HIGH]

**File**: `darkmatter/lib/src/markdown/schemas/validate.rs:172-183`

`collect_problems` always sets `arm_index: None`. The spec (§Schema Validation, §Root-level Unions) requires:

> When validation fails against a root union, the CLI reports the closest-matching arm (the one producing the fewest problems) by default and prefixes problems with the arm index (e.g. `arm[1]: title is required`).

The `arm_index` field exists on `ValidationProblem` but is dead code. The `anyOf` error from `jsonschema` does not decompose per-arm results in a way that trivially maps; this needs a post-processing step to partition errors by arm, count per arm, and annotate each problem.

**Severity**: HIGH — spec requirement not met; root-union error reporting is user-facing.

### F2. Source line/column always `None` [HIGH]

**File**: `darkmatter/lib/src/markdown/schemas/validate.rs:176-177`

`ValidationProblem::line` and `ValidationProblem::column` are always `None`. The spec (§Output Shape) shows:

```
• title is required
    at line 2, column 1 of frontmatter
```

The CLI `emit_pretty` function does render `format_location`, but since the values are never populated, the location lines never appear. Implementing this requires a frontmatter source map (byte-offset → line/column) from the `Markdown` type, which would need to be threaded through `effective_for` → `validate` → `collect_problems`.

**Severity**: HIGH — user-facing output is incomplete.

### F3. Missing `PatternOptions::regex()` in validator builder [MEDIUM]

**File**: `darkmatter/lib/src/markdown/schemas/validate.rs:154-166`

The spec (§Validator Construction & Caching, §Dependencies) calls for:

```rust
.with_pattern_options(PatternOptions::regex())
```

This configures the `regex` crate's linear-time engine instead of the default (which may be susceptible to ReDoS on adversarial patterns). The current code omits this call.

**Severity**: MEDIUM — security/performance best practice.

### F4. Validation table-driven fixtures are thin [MEDIUM]

**Directory**: `darkmatter/lib/tests/fixtures/validate/` — 8 cases

The spec (§Testing Strategy) calls for coverage of many more scenarios. Missing fixture cases:

| Missing case | Why it matters |
|---|---|
| `file` type with `match()` globs | Core custom format + keyword |
| `url` type with `scheme()` | Custom keyword validation |
| `email` format assertion | Format assertion enabled |
| `date`/`datetime`/`time` format assertion | Format assertions enabled |
| `not-empty` pattern constraint | Regex pattern correctness |
| `pattern` constraint | Regex pattern correctness |
| `unique` array constraint | Array-level validation |
| `numberlike` / `boolish` anyOf | Multi-type validation |
| Property-level union validation | anyOf hoisting + arm-local constraints |
| Baseline merge in validation | Baseline + document interaction |
| Root union with file-ref arms | End-to-end resolution + union |
| Multiple files, mixed valid/invalid | Aggregate exit codes |

The 8 existing cases (simple_required_present, missing_required, inline_schema, range_violation, json_schema_ref, root_union, root_union_none_match, enum_member_invalid) are a good skeleton but only scratch the surface of the constraint vocabulary.

**Severity**: MEDIUM — low confidence that many constraint types work end-to-end.

### F5. `SchemaError::Baseline` lost `#[source]` chain [LOW]

**File**: `darkmatter/lib/src/markdown/schemas/errors.rs:76`

The spec defines:

```rust
Baseline { #[source] source: Box<SchemaError> },
```

The implementation uses `Baseline { message: String }`. This means the causal chain is flattened to a string and callers (or error reporters) cannot programmatically inspect the underlying cause. The `BlockError` rendering compensates visually, but programmatic consumers (e.g. `--format json` output) lose the structured cause.

**Severity**: LOW — cosmetic / ergonomics; rendering is fine.

### F6. File format tests mutate process CWD [MEDIUM]

**File**: `darkmatter/lib/src/markdown/schemas/format.rs:267-385`

The `CWD_LOCK` mutex serialises format tests, but CWD mutation is still fragile:
- If any test in the process panics between `set_current_dir` and the restore, the CWD is left in a temp directory.
- Parallel test runners (e.g. `nextest`) run each test in its own process, which mitigates this, but `cargo test` does not.

Consider using `resolve_from(base_dir)` with an explicit base directory instead of relying on the ambient CWD, or use `std::env::set_current_dir` in a `scopeguard`/`Drop` guard.

**Severity**: MEDIUM — test reliability.

### F7. Detection `file` type resolves from document dir, not CWD [LOW]

**File**: `darkmatter/lib/src/markdown/schemas/detect.rs:224-234`

`resolves_to_existing_file` resolves from `base_dir` (document parent), but the spec says `file` property validation resolves from **CWD**. Detection and validation are different operations, so this isn't a bug per se, but the asymmetry should be documented — a detected `file` type might not validate if the document is run from a different working directory.

**Severity**: LOW — documentation/UX clarity.

### F8. `schema_to_yaml` for root unions emits potentially invalid YAML [LOW]

**File**: `darkmatter/lib/src/markdown/schemas/detect.rs:430-431`

```rust
SchemaArm::Inline(shape) => {
    out.push_str("  - \n");
    write_shape(&mut out, shape, 2);
}
```

This emits `  - \n    key: value\n` — a list item with an empty scalar followed by an indented mapping. Some YAML parsers accept this, but strictly it should be `  - key: value\n` (mapping directly after the `-`). This is a serialisation-only concern (detection never produces root unions today), but the output won't round-trip through `parse_yaml_schema`.

**Severity**: LOW — latent bug in an uncommon path.

### F9. Criterion benchmark exists but not in CI [INFO]

**File**: `darkmatter/lib/benches/schema_validation.rs`

The spec mentions running in CI for trend tracking. The benchmark is well-structured (1000-doc corpus, warm/cold cache variants) but the `justfile` / CI pipeline integration is not visible. If trend tracking is desired, this needs a CI step.

**Severity**: INFO — no functional impact.

### F10. No `--quiet` warning when `$schema` absent and no baseline [MEDIUM]

**File**: `darkmatter/cli/src/commands/schema/validate.rs:149-160`

The spec says:

> If no baseline and no `$schema`, validation succeeds vacuously and emits a warning line in `pretty` mode (suppressed by `--quiet`).

The implementation emits `<dim>(no schema; vacuously valid)</dim>` even without `--quiet`, which satisfies the "emit a warning" requirement. However, the output uses the **success** format with a dim note rather than an explicit warning. This is a minor UX deviation — the spec's intent is clear but the implementation's interpretation is reasonable.

**Severity**: LOW — acceptable interpretation of spec.

---

## Positive Observations

1. **Grammar parser is thorough** — hand-written lexer + parser covers all spec'd syntax forms including enum member quoting, array-level constraints, and the `->` description suffix. Proptest round-trip tests (256 cases) provide strong confidence.

2. **Snapshot tests are comprehensive** — `schemas_convert_snapshots.rs` covers every row of the spec's mapping table plus unions, descriptions, defaults, and a full end-to-end document.

3. **Error rendering is excellent** — `BlockError` implementations on every `SchemaError` variant produce rich, hint-containing status blocks with proper Prose markup.

4. **Validator cache is well-designed** — LRU eviction, SHA-256 keying, configurable capacity via env var, `Arc<Validator>` sharing. The serialise-outside-lock pattern minimises contention.

5. **Baseline merge handles root unions per-arm** — correctly recurses into each `anyOf` arm independently, matching the spec.

6. **Shell completions module is clean** — read-only consumer, pure functions over resolved state, good test coverage for the completable property types.

7. **CLI exit codes match spec** — 0/1/2/3 correctly mapped; `--quiet`, `--format json`, and `BASELINE_SCHEMA` env var all work.

---

## Test Rigor Assessment

| Requirement | Verification Level | Adequate? |
|---|---|---|
| Grammar parser correctness | Level 1 (unit + proptest) | Yes |
| SimplifiedSchema → JSON Schema conversion | Level 1 (unit + snapshots) | Yes |
| Schema resolution (inline, file, JSON) | Level 1 (unit with tempdir) | Yes |
| Validation against schemas | Level 1 (table-driven) | Partial — only 8 fixture cases |
| Custom format validators (file, url, scheme) | Level 1 (unit with CWD mutation) | Yes, but fragile (F6) |
| CLI `md schema validate` | Level 1 (assert_cmd) | Partial — no exit-64 test |
| CLI `md schema detect` | Level 1 (assert_cmd) | Yes |
| Root-union arm-index reporting | **None** | **Gap (F1)** |
| Source line/column in problems | **None** | **Gap (F2)** |
| Baseline merge + validation | Level 1 (unit in resolve.rs) | Partial — no end-to-end fixture |
| Performance (1000-file corpus) | Level 1 (criterion) | Yes (smoke) |
| Error rendering (BlockError) | Level 1 (unit) | Yes |

No Level 2 or Level 3 tests are present, but for a non-TUI, non-interactive feature like schema validation, Level 1 is sufficient. The primary gaps are Level 1 omissions (F1, F2, F4).

---

## Verdict

**Not ready for production.**

Three findings block production readiness:

1. **F1** (root-union arm_index never populated) — a user-facing spec requirement is unimplemented.
2. **F2** (source line/column always None) — the pretty-format output shape cannot match the spec without this.
3. **F4** (thin validation fixtures) — many constraint types (file, url, email, date/time, not-empty, pattern, unique, numberlike, boolish) lack any table-driven end-to-end test, leaving low confidence in real-world correctness.

Resolving F1, F2, and F4 would bring this feature to a shippable state. F3 (PatternOptions) should also be addressed before release as a security best practice.
