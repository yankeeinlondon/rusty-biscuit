---
ready: false
agent: codex/default
created: 2026-06-27T21:10:35
implemented: true
---

# Review 3: Path Resolution

## Verdict

Not ready for production.

The review-2 blockers are addressed: lifecycle shell preflight now carries the launch-area fallback, sequence template preflight now builds `ComposeOptions` with the same fallback, and the targeted L1 regressions pass.

One production blocker remains in Darkmatter's compose cache. The new `file_ref_fallback_dir` option changes interpolation and schema-validation results, but it is not part of the compose options cache key. A cached compose result can therefore be reused across different launch-area anchors and return stale `file_exists`, `frontmatter`, or `file` schema outcomes.

## Findings

### 1. [High] Compose cache keys ignore `file_ref_fallback_dir`

**Evidence:** `ComposeOptions::file_ref_fallback_dir` is the new launch-area anchor and is threaded into expression resolution (`darkmatter/lib/src/markdown/compose/context/options.rs:320`, `darkmatter/lib/src/markdown/compose/context/options.rs:856`, `darkmatter/lib/src/markdown/compose/context/options.rs:882`) and schema validation (`darkmatter/lib/src/markdown/compose/schema_validation.rs:75`). Claudine now sets it on production compose paths (`claudine/lib/src/composition/prepare.rs:182`, `claudine/cli/src/commands/compose/prep.rs:263`). However, `darkmatter/lib/src/markdown/compose/cache/hashing.rs:135` builds `options_hash` from operation flags, set overrides, magic paths, baseline schema, etc., and never includes `options.file_ref_fallback_dir` before hashing at `darkmatter/lib/src/markdown/compose/cache/hashing.rs:202`.

**Why this matters:** The spec requires resolution to be independent of the post-launch `chdir` and to use the captured launch area as an explicit fallback. With the persistent compose cache enabled, two invocations of the same prompt with the same `spec` string but different launch areas can share the same source hash and options hash even though `{{ file_exists(spec) }}`, `{{ frontmatter(spec, ...) }}`, and `format: darkmatter-file` validation may produce different results. The second invocation can reuse the first invocation's cached composed output instead of resolving against its own launch area, reintroducing exactly the kind of timing/anchor disagreement this fix is meant to remove.

**Test rigor:** Level 1 is the right level for this deterministic cache-key behavior. Existing L1 tests prove the fallback works when composition executes, but I found no L1 cache regression that composes the same prompt twice with distinct `file_ref_fallback_dir` values and asserts the second run does not reuse the first run's result. There is also no `options_hash` unit test analogous to the existing baseline-schema test proving the hash is sensitive to the fallback path.

**Required fix:** Include a stable representation of `options.file_ref_fallback_dir` in `options_hash`. Add a focused unit test for `options_hash` sensitivity and an end-to-end compose-cache regression where run 1 resolves a launch-area-only file as present, run 2 uses a different launch area where the same relative path is absent (or has different frontmatter), and run 2 must not return run 1's cached output.

## Verification-Level Assessment

All requirements in this fix are deterministic filesystem/path-resolution behavior. Level 1 coverage is appropriate; no Level 2 or Level 3 terminal/keyboard tests are required.

Covered at Level 1:

- expression-level document-first / launch-area-fallback resolution;
- lifecycle event-time `file_exists` and `frontmatter` fallback;
- lifecycle shell preflight uses the launch-area fallback;
- sequence template preflight uses the launch-area fallback;
- body `::file` transclusion remains document-relative;
- `$schema` references and root-union string arms remain document-relative;
- schema `file` property values use document-first / launch-area-fallback resolution;
- `pre_validate_schema`, `drop_invalid_optionals`, and schema status reports use the fallback;
- sequence step schema pre-validation uses the fallback.

Missing or insufficient at Level 1:

- Darkmatter compose cache identity is sensitive to `file_ref_fallback_dir`.

## Verification Run

Ran:

```bash
cargo nextest run --color=never -p claudine -p darkmatter -E 'test(/lifecycle_shell_read_side/) + test(/file_ref_fallback/) + test(/launch_area_fallback/) + test(/pre_validate_schema_uses_launch_area_fallback_not_cwd/) + test(/drop_invalid_optionals_keeps_file_under_launch_area_fallback/) + test(/sequence_step_pre_validation_uses_launch_area_fallback/)'
```

Result: 12 passed.

Ran:

```bash
cargo nextest run --color=never -p claudine-cli -E 'test(/template_preflight_.*fallback/)'
```

Result: 2 passed.
